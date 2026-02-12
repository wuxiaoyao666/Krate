//! 反向代理命令模块。
//!
//! 设计目标：
//! - 作为 Tauri 后端命令，提供“启动/停止/状态查询”能力；
//! - 支持 HTTP/HTTPS 反向代理与 WebSocket 透传；
//! - 提供按 Host + 路径前缀匹配的路由能力；
//! - 提供按路由粒度控制的“不安全 TLS 校验”开关（仅调试场景建议开启）。

use bytes::Bytes;
use http::header::{self, HeaderName};
use http::{HeaderMap, HeaderValue, StatusCode, Uri};
use http_body_util::{Either, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{verify_tls12_signature, verify_tls13_signature, CryptoProvider};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, Error as TlsError, SignatureScheme};
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{command, State};
use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::time::{sleep, Duration};

type ProxyResponse = Response<Either<Incoming, Full<Bytes>>>;
type HttpsConnector = hyper_rustls::HttpsConnector<HttpConnector>;
type HttpsClient = Client<HttpsConnector, Incoming>;

/// 一组可复用的上游客户端：
/// - `secure`: 标准证书校验
/// - `insecure`: 跳过证书链与主机名校验（调试用途）
#[derive(Clone)]
struct ProxyClients {
    secure: HttpsClient,
    insecure: HttpsClient,
}

/// 不安全 TLS 校验器。
///
/// 安全提示：该校验器会放过服务端证书有效性检查，仅保留握手签名算法验证，
/// 适合开发调试，不适合生产环境。
#[derive(Debug)]
struct InsecureTlsVerifier {
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for InsecureTlsVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, TlsError> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, TlsError> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStartRequest {
    /// 监听地址（例如 `127.0.0.1` 或 `0.0.0.0`）。
    listen_host: String,
    /// 监听端口。
    listen_port: u16,
    /// 路由配置列表。
    routes: Vec<ProxyRouteInput>,
}

/// 前端传入的单条路由配置。
#[derive(Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyRouteInput {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    host: String,
    #[serde(default)]
    path_prefix: String,
    target: String,
    #[serde(default)]
    strip_prefix: bool,
    #[serde(default)]
    allow_insecure_tls: bool,
}

/// 代理运行状态（返回给前端）。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    running: bool,
    listen_host: Option<String>,
    listen_port: Option<u16>,
    route_count: usize,
    total_requests: u64,
    started_at: Option<u64>,
    last_error: Option<String>,
    message: String,
}

/// 持有运行中的代理任务句柄和停止信号。
struct ProxyRuntime {
    stop_sender: Option<oneshot::Sender<()>>,
    handle: tauri::async_runtime::JoinHandle<()>,
}

/// 代理状态快照（受互斥锁保护）。
struct ProxySnapshot {
    running: bool,
    listen_host: Option<String>,
    listen_port: Option<u16>,
    route_count: usize,
    started_at: Option<u64>,
    last_error: Option<String>,
    message: String,
}

/// 代理全局状态（Tauri `State`）。
///
/// - `runtime`：运行时句柄（用于停止）
/// - `snapshot`：状态文本与错误等可观测信息
/// - `total_requests`：累计转发请求数
pub struct ProxyState {
    runtime: Mutex<Option<ProxyRuntime>>,
    snapshot: Arc<Mutex<ProxySnapshot>>,
    total_requests: Arc<AtomicU64>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            runtime: Mutex::new(None),
            snapshot: Arc::new(Mutex::new(ProxySnapshot {
                running: false,
                listen_host: None,
                listen_port: None,
                route_count: 0,
                started_at: None,
                last_error: None,
                message: "代理服务未启动".to_string(),
            })),
            total_requests: Arc::new(AtomicU64::new(0)),
        }
    }

    fn status(&self) -> ProxyStatus {
        let snapshot = self.snapshot.lock().unwrap();
        ProxyStatus {
            running: snapshot.running,
            listen_host: snapshot.listen_host.clone(),
            listen_port: snapshot.listen_port,
            route_count: snapshot.route_count,
            total_requests: self.total_requests.load(Ordering::Relaxed),
            started_at: snapshot.started_at,
            last_error: snapshot.last_error.clone(),
            message: snapshot.message.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetScheme {
    Http,
    Https,
}

impl TargetScheme {
    fn as_str(self) -> &'static str {
        match self {
            TargetScheme::Http => "http",
            TargetScheme::Https => "https",
        }
    }

    fn default_port(self) -> u16 {
        match self {
            TargetScheme::Http => 80,
            TargetScheme::Https => 443,
        }
    }
}

#[derive(Clone)]
struct ProxyRoute {
    /// Host 条件；`None` 表示通配。
    host: Option<String>,
    /// 前缀匹配路径（已归一化，形如 `/api`）。
    path_prefix: String,
    target_scheme: TargetScheme,
    target_host: String,
    target_port: u16,
    /// 是否剥离匹配前缀。
    strip_prefix: bool,
    /// 是否允许跳过 TLS 证书校验（仅 HTTPS/WSS 有意义）。
    allow_insecure_tls: bool,
}

impl ProxyRoute {
    fn target_authority(&self) -> String {
        if self.target_port == self.target_scheme.default_port() {
            self.target_host.clone()
        } else {
            format!("{}:{}", self.target_host, self.target_port)
        }
    }
}

#[command]
pub fn proxy_get_status(state: State<ProxyState>) -> ProxyStatus {
    state.status()
}

/// 启动反向代理服务。
///
/// 启动流程：
/// 1. 校验监听参数和路由配置；
/// 2. 绑定监听端口并初始化上游客户端；
/// 3. 启动 accept 循环并写入运行时句柄；
/// 4. 更新快照状态返回前端。
#[command]
pub async fn proxy_start(
    state: State<'_, ProxyState>,
    config: ProxyStartRequest,
) -> Result<ProxyStatus, String> {
    let listen_host = config.listen_host.trim().to_string();
    if listen_host.is_empty() {
        return Err("监听地址不能为空".to_string());
    }
    if config.listen_port == 0 {
        return Err("监听端口非法".to_string());
    }

    let routes = build_routes(&config.routes)?;
    if routes.is_empty() {
        return Err("至少需要一条启用的路由规则".to_string());
    }

    {
        let runtime_guard = state
            .runtime
            .lock()
            .map_err(|_| "代理状态锁异常".to_string())?;
        if runtime_guard.is_some() {
            return Err("代理服务已经在运行，请先停止再启动".to_string());
        }
    }

    let bind_addr = format!("{}:{}", listen_host, config.listen_port);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .map_err(|err| format!("监听失败 {}: {}", bind_addr, err))?;

    state.total_requests.store(0, Ordering::Relaxed);

    let clients = Arc::new(create_https_clients()?);
    let routes = Arc::new(routes);
    let total_requests = state.total_requests.clone();
    let snapshot = state.snapshot.clone();
    let (stop_sender, stop_receiver) = oneshot::channel::<()>();

    let handle = tauri::async_runtime::spawn(run_proxy_server(
        listener,
        routes.clone(),
        clients,
        total_requests,
        snapshot.clone(),
        stop_receiver,
    ));
    let mut stop_sender = Some(stop_sender);

    let mut runtime_guard = state
        .runtime
        .lock()
        .map_err(|_| "代理状态锁异常".to_string())?;
    let already_running = runtime_guard.is_some();
    if already_running {
        // 并发启动时，已经有其他请求先成功注册了运行时。
        // 这里需要主动清理当前任务，避免形成无法停止的孤儿代理。
        if let Some(sender) = stop_sender.take() {
            let _ = sender.send(());
        }
        handle.abort();
        return Err("代理服务已经在运行，请先停止再启动".to_string());
    }

    *runtime_guard = Some(ProxyRuntime {
        stop_sender,
        handle,
    });
    drop(runtime_guard);

    {
        let mut snap = snapshot.lock().map_err(|_| "代理状态锁异常".to_string())?;
        snap.running = true;
        snap.listen_host = Some(listen_host);
        snap.listen_port = Some(config.listen_port);
        snap.route_count = routes.len();
        snap.started_at = Some(current_timestamp());
        snap.last_error = None;
        snap.message = format!("代理服务运行中，共 {} 条路由", routes.len());
    }

    Ok(state.status())
}

/// 停止反向代理服务。
#[command]
pub async fn proxy_stop(state: State<'_, ProxyState>) -> Result<ProxyStatus, String> {
    let runtime = {
        let mut guard = state
            .runtime
            .lock()
            .map_err(|_| "代理状态锁异常".to_string())?;
        guard.take()
    };

    if let Some(mut runtime) = runtime {
        if let Some(stop_sender) = runtime.stop_sender.take() {
            let _ = stop_sender.send(());
        }
        let _ = runtime.handle.await;
    }

    {
        let mut snapshot = state
            .snapshot
            .lock()
            .map_err(|_| "代理状态锁异常".to_string())?;
        snapshot.running = false;
        snapshot.listen_host = None;
        snapshot.listen_port = None;
        snapshot.started_at = None;
        snapshot.route_count = 0;
        snapshot.message = "代理服务已停止".to_string();
    }

    Ok(state.status())
}

/// 代理主循环：接收入站连接，并为每个连接创建 HTTP/1 服务任务。
async fn run_proxy_server(
    listener: TcpListener,
    routes: Arc<Vec<ProxyRoute>>,
    clients: Arc<ProxyClients>,
    total_requests: Arc<AtomicU64>,
    snapshot: Arc<Mutex<ProxySnapshot>>,
    mut stop_receiver: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut stop_receiver => {
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, peer)) => {
                        let routes = routes.clone();
                        let clients = clients.clone();
                        let total_requests = total_requests.clone();
                        let snapshot = snapshot.clone();

                        tauri::async_runtime::spawn(async move {
                            let io = TokioIo::new(stream);
                            let snapshot_for_service = snapshot.clone();
                            let service = service_fn(move |request| {
                                handle_proxy_request(
                                    request,
                                    peer,
                                    routes.clone(),
                                    clients.clone(),
                                    total_requests.clone(),
                                    snapshot_for_service.clone(),
                                )
                            });

                            if let Err(err) = http1::Builder::new()
                                .preserve_header_case(true)
                                .title_case_headers(true)
                                .serve_connection(io, service)
                                .with_upgrades()
                                .await
                            {
                                set_runtime_error(&snapshot, format!("连接处理失败: {}", err));
                            }
                        });
                    }
                    Err(err) => {
                        set_runtime_error(&snapshot, format!("监听 accept 失败: {}", err));
                        sleep(Duration::from_millis(80)).await;
                    }
                }
            }
        }
    }
}

/// 处理单个 HTTP 请求：
/// - 路由匹配
/// - 构造上游 URI
/// - 代理头处理
/// - HTTP 或 WebSocket 转发
async fn handle_proxy_request(
    mut request: Request<Incoming>,
    peer: std::net::SocketAddr,
    routes: Arc<Vec<ProxyRoute>>,
    clients: Arc<ProxyClients>,
    total_requests: Arc<AtomicU64>,
    snapshot: Arc<Mutex<ProxySnapshot>>,
) -> Result<ProxyResponse, Infallible> {
    let request_host = extract_request_host(&request);
    let request_path = request.uri().path().to_string();

    let route = match select_route(&routes, request_host.as_deref(), &request_path) {
        Some(route) => route.clone(),
        None => {
            return Ok(plain_response(
                StatusCode::NOT_FOUND,
                "未匹配到可用的反向代理路由",
            ));
        }
    };

    let upstream_uri = match build_upstream_uri(request.uri(), &route) {
        Ok(uri) => uri,
        Err(err) => return Ok(plain_response(StatusCode::BAD_REQUEST, &err)),
    };

    let original_host = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    let websocket_upgrade = is_websocket_upgrade(&request);
    *request.uri_mut() = upstream_uri;

    if let Err(err) = apply_proxy_headers(
        request.headers_mut(),
        &route,
        peer,
        &original_host,
        websocket_upgrade,
    ) {
        return Ok(plain_response(StatusCode::BAD_REQUEST, &err));
    }

    let client = select_upstream_client(&route, &clients);

    if websocket_upgrade {
        let response = forward_websocket(request, client, total_requests, snapshot).await;
        return Ok(response);
    }

    match client.request(request).await {
        Ok(response) => {
            total_requests.fetch_add(1, Ordering::Relaxed);
            Ok(response.map(Either::Left))
        }
        Err(err) => {
            set_runtime_error(&snapshot, format!("转发请求失败: {}", err));
            Ok(plain_response(
                StatusCode::BAD_GATEWAY,
                &format!("上游服务不可用: {}", err),
            ))
        }
    }
}

/// 按路由选择上游客户端：
/// - HTTPS/WSS + `allow_insecure_tls=true` 使用不安全客户端；
/// - 其余情况使用默认安全客户端。
fn select_upstream_client(route: &ProxyRoute, clients: &ProxyClients) -> HttpsClient {
    if route.allow_insecure_tls && route.target_scheme == TargetScheme::Https {
        clients.insecure.clone()
    } else {
        clients.secure.clone()
    }
}

/// 处理 WebSocket 握手与双向流量透传。
async fn forward_websocket(
    mut request: Request<Incoming>,
    client: HttpsClient,
    total_requests: Arc<AtomicU64>,
    snapshot: Arc<Mutex<ProxySnapshot>>,
) -> ProxyResponse {
    let on_client_upgrade = hyper::upgrade::on(&mut request);

    match client.request(request).await {
        Ok(mut response) => {
            let on_upstream_upgrade = if response.status() == StatusCode::SWITCHING_PROTOCOLS {
                Some(hyper::upgrade::on(&mut response))
            } else {
                None
            };

            let response_to_client = response.map(Either::Left);
            total_requests.fetch_add(1, Ordering::Relaxed);

            if let Some(on_upstream_upgrade) = on_upstream_upgrade {
                tauri::async_runtime::spawn(async move {
                    match tokio::try_join!(on_client_upgrade, on_upstream_upgrade) {
                        Ok((client_upgraded, upstream_upgraded)) => {
                            let mut client_io = TokioIo::new(client_upgraded);
                            let mut upstream_io = TokioIo::new(upstream_upgraded);
                            let _ = copy_bidirectional(&mut client_io, &mut upstream_io).await;
                        }
                        Err(err) => {
                            set_runtime_error(&snapshot, format!("WebSocket 升级失败: {}", err));
                        }
                    }
                });
            }

            response_to_client
        }
        Err(err) => {
            set_runtime_error(&snapshot, format!("WebSocket 握手转发失败: {}", err));
            plain_response(
                StatusCode::BAD_GATEWAY,
                &format!("WebSocket 上游连接失败: {}", err),
            )
        }
    }
}

/// 创建上游客户端集合。
fn create_https_clients() -> Result<ProxyClients, String> {
    let secure = create_secure_https_client()?;
    let insecure = create_insecure_https_client()?;
    Ok(ProxyClients { secure, insecure })
}

/// 创建默认安全客户端（使用系统信任根证书）。
fn create_secure_https_client() -> Result<HttpsClient, String> {
    let https_connector = HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|err| format!("加载系统证书失败: {}", err))?
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Client::builder(TokioExecutor::new()).build(https_connector))
}

/// 创建“不安全 TLS”客户端。
///
/// 说明：这里先调用一次 `ClientConfig::builder()`，用于确保 rustls 的默认
/// crypto provider 已初始化，然后再读取 provider 构建自定义 verifier。
fn create_insecure_https_client() -> Result<HttpsClient, String> {
    let _ = ClientConfig::builder();
    let provider = CryptoProvider::get_default()
        .cloned()
        .ok_or_else(|| "TLS 加密提供方初始化失败".to_string())?;

    let tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureTlsVerifier { provider }))
        .with_no_client_auth();

    let https_connector = HttpsConnectorBuilder::new()
        .with_tls_config(tls_config)
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Client::builder(TokioExecutor::new()).build(https_connector))
}

/// 写入代理转发相关请求头。
///
/// 关键行为：
/// - 清理 hop-by-hop 头；
/// - 重写 `Host` 为目标上游；
/// - 追加 `X-Forwarded-For`；
/// - 写入 `X-Forwarded-Host` 与 `X-Forwarded-Proto`。
fn apply_proxy_headers(
    headers: &mut HeaderMap<HeaderValue>,
    route: &ProxyRoute,
    peer: std::net::SocketAddr,
    original_host: &str,
    keep_upgrade: bool,
) -> Result<(), String> {
    sanitize_hop_headers(headers, keep_upgrade);

    let target_host_header = route.target_authority();
    headers.insert(
        header::HOST,
        HeaderValue::from_str(&target_host_header)
            .map_err(|_| "目标主机格式非法，无法写入 Host 头".to_string())?,
    );

    append_x_forwarded_for(headers, peer)?;

    if !original_host.is_empty() {
        headers.insert(
            HeaderName::from_static("x-forwarded-host"),
            HeaderValue::from_str(original_host)
                .map_err(|_| "原始 Host 非法，无法写入 X-Forwarded-Host".to_string())?,
        );
    }

    headers.insert(
        HeaderName::from_static("x-forwarded-proto"),
        HeaderValue::from_static("http"),
    );

    Ok(())
}

/// 清理 hop-by-hop 头，避免这些头被错误地转发到上游。
fn sanitize_hop_headers(headers: &mut HeaderMap<HeaderValue>, keep_upgrade: bool) {
    let connection_tokens = headers
        .get(header::CONNECTION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .unwrap_or_default();

    for token in connection_tokens.split(',') {
        let item = token.trim().to_ascii_lowercase();
        if item.is_empty() || (keep_upgrade && item == "upgrade") {
            continue;
        }
        if let Ok(name) = HeaderName::from_bytes(item.as_bytes()) {
            headers.remove(name);
        }
    }

    for name in [
        header::CONNECTION,
        header::PROXY_AUTHENTICATE,
        header::PROXY_AUTHORIZATION,
        header::TE,
        header::TRAILER,
        header::TRANSFER_ENCODING,
    ] {
        headers.remove(name);
    }

    headers.remove(HeaderName::from_static("proxy-connection"));
    headers.remove(HeaderName::from_static("keep-alive"));

    if keep_upgrade {
        headers.insert(header::CONNECTION, HeaderValue::from_static("upgrade"));
    } else {
        headers.remove(header::UPGRADE);
        headers.insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
}

/// 追加 `X-Forwarded-For` 链路信息。
fn append_x_forwarded_for(
    headers: &mut HeaderMap<HeaderValue>,
    peer: std::net::SocketAddr,
) -> Result<(), String> {
    let xff_name = HeaderName::from_static("x-forwarded-for");
    let current = headers
        .get(&xff_name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .trim();

    let next = if current.is_empty() {
        peer.ip().to_string()
    } else {
        format!("{}, {}", current, peer.ip())
    };

    headers.insert(
        xff_name,
        HeaderValue::from_str(&next)
            .map_err(|_| "X-Forwarded-For 构建失败，IP 字段非法".to_string())?,
    );

    Ok(())
}

/// 判断请求是否为 WebSocket Upgrade。
fn is_websocket_upgrade(request: &Request<Incoming>) -> bool {
    let upgrade = request
        .headers()
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    let connection = request
        .headers()
        .get(header::CONNECTION)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    upgrade == "websocket" && connection.contains("upgrade")
}

/// 提取请求 Host（优先 `Host` 头，其次 URI authority）。
fn extract_request_host(request: &Request<Incoming>) -> Option<String> {
    if let Some(host) = request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    {
        return normalize_host_value(host);
    }

    request
        .uri()
        .authority()
        .and_then(|authority| normalize_host_value(authority.as_str()))
}

/// 构建转发后的上游 URI（包含路径和 query）。
fn build_upstream_uri(original_uri: &Uri, route: &ProxyRoute) -> Result<Uri, String> {
    let path_and_query = rewrite_path_and_query(original_uri, route);
    let uri_text = format!(
        "{}://{}{}",
        route.target_scheme.as_str(),
        route.target_authority(),
        path_and_query
    );

    uri_text
        .parse::<Uri>()
        .map_err(|err| format!("构建上游地址失败: {}", err))
}

/// 基于路由策略重写 path 和 query。
fn rewrite_path_and_query(uri: &Uri, route: &ProxyRoute) -> String {
    let path = uri.path();
    let mut rewritten_path = path.to_string();

    if route.strip_prefix && route.path_prefix != "/" {
        if let Some(rest) = path.strip_prefix(&route.path_prefix) {
            rewritten_path = if rest.is_empty() {
                "/".to_string()
            } else if rest.starts_with('/') {
                rest.to_string()
            } else {
                format!("/{}", rest)
            };
        }
    }

    if let Some(query) = uri.query() {
        if !query.is_empty() {
            return format!("{}?{}", rewritten_path, query);
        }
    }

    rewritten_path
}

/// 构建并排序启用路由。
///
/// 排序策略：
/// 1. 路径前缀长度降序（最长前缀优先）；
/// 2. 前缀相同则 Host 精确匹配优先于通配。
fn build_routes(inputs: &[ProxyRouteInput]) -> Result<Vec<ProxyRoute>, String> {
    let mut routes = Vec::new();

    for item in inputs.iter().filter(|route| route.enabled) {
        let path_prefix = normalize_path_prefix(&item.path_prefix);
        let host = normalize_host_value(&item.host);
        let (scheme, target_host, target_port) = parse_target(&item.target)?;

        let _ = (&item.id, &item.name);

        routes.push(ProxyRoute {
            host,
            path_prefix,
            target_scheme: scheme,
            target_host,
            target_port,
            strip_prefix: item.strip_prefix,
            allow_insecure_tls: item.allow_insecure_tls,
        });
    }

    routes.sort_by(|left, right| {
        right
            .path_prefix
            .len()
            .cmp(&left.path_prefix.len())
            .then_with(|| right.host.is_some().cmp(&left.host.is_some()))
    });
    Ok(routes)
}

/// 解析目标地址（支持 `http://`、`https://`、`ws://`、`wss://`）。
///
/// 返回 `(scheme, host, port)`，其中 ws/wss 会映射为 http/https 传输语义。
fn parse_target(raw: &str) -> Result<(TargetScheme, String, u16), String> {
    let normalized = raw.trim().trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Err("目标地址不能为空".to_string());
    }

    let normalized_lower = normalized.to_ascii_lowercase();
    let (scheme, rest) = if normalized_lower.starts_with("http://") {
        (TargetScheme::Http, &normalized[7..])
    } else if normalized_lower.starts_with("https://") {
        (TargetScheme::Https, &normalized[8..])
    } else if normalized_lower.starts_with("ws://") {
        (TargetScheme::Http, &normalized[5..])
    } else if normalized_lower.starts_with("wss://") {
        (TargetScheme::Https, &normalized[6..])
    } else {
        return Err("目标地址必须以 http://、https://、ws:// 或 wss:// 开头".to_string());
    };

    if rest.is_empty() {
        return Err("目标地址不能为空".to_string());
    }

    if rest.contains('/') {
        return Err("目标地址暂不支持路径，请只填写主机和端口".to_string());
    }

    if rest.matches(':').count() > 1 {
        return Err("当前版本暂不支持 IPv6 地址".to_string());
    }

    let default_port = scheme.default_port();

    if let Some((host, port_text)) = rest.rsplit_once(':') {
        let host = host.trim();
        if host.is_empty() {
            return Err("目标主机不能为空".to_string());
        }

        let port = port_text
            .trim()
            .parse::<u16>()
            .map_err(|_| "目标端口非法".to_string())?;

        return Ok((scheme, host.to_string(), port));
    }

    Ok((scheme, rest.to_string(), default_port))
}

/// 归一化路径前缀，确保以 `/` 开头并去除尾部多余 `/`。
fn normalize_path_prefix(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "/".to_string();
    }

    let mut prefix = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    };

    if prefix.len() > 1 {
        prefix = prefix.trim_end_matches('/').to_string();
    }

    prefix
}

/// 归一化 Host；空字符串或 `*` 视为通配（返回 `None`）。
fn normalize_host_value(raw: &str) -> Option<String> {
    let value = raw.trim().to_ascii_lowercase();
    if value.is_empty() || value == "*" {
        return None;
    }

    let host = value.split(':').next().unwrap_or("").trim().to_string();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// 从路由列表中选出第一条匹配规则。
///
/// 注意：路由在进入该函数前已经按“优先级”排序。
fn select_route<'a>(
    routes: &'a [ProxyRoute],
    request_host: Option<&str>,
    request_path: &str,
) -> Option<&'a ProxyRoute> {
    routes.iter().find(|route| {
        let host_match = match (&route.host, request_host) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(expect), Some(actual)) => expect == actual,
        };

        host_match && path_match(&route.path_prefix, request_path)
    })
}

/// 判断路径是否命中前缀。
fn path_match(prefix: &str, path: &str) -> bool {
    if prefix == "/" {
        return true;
    }

    if path == prefix {
        return true;
    }

    match path.strip_prefix(prefix) {
        Some(rest) => rest.starts_with('/'),
        None => false,
    }
}

/// 构建纯文本响应。
fn plain_response(status: StatusCode, message: &str) -> ProxyResponse {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Either::Right(Full::new(Bytes::from(message.to_string()))))
        .unwrap_or_else(|_| {
            Response::new(Either::Right(Full::new(Bytes::from_static(
                b"internal response build error",
            ))))
        })
}

/// 更新运行时错误快照（用于前端展示最近错误）。
fn set_runtime_error(snapshot: &Arc<Mutex<ProxySnapshot>>, message: String) {
    if let Ok(mut snap) = snapshot.lock() {
        snap.last_error = Some(message);
    }
}

/// 获取当前 UNIX 秒级时间戳。
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_route(host: &str, path_prefix: &str, target: &str) -> ProxyRouteInput {
        ProxyRouteInput {
            id: String::new(),
            name: String::new(),
            enabled: true,
            host: host.to_string(),
            path_prefix: path_prefix.to_string(),
            target: target.to_string(),
            strip_prefix: false,
            allow_insecure_tls: false,
        }
    }

    #[test]
    fn parse_target_supports_http_https_ws_wss_and_case_insensitive_scheme() {
        let (scheme_http, host_http, port_http) = parse_target("HTTP://example.com").unwrap();
        assert_eq!(scheme_http, TargetScheme::Http);
        assert_eq!(host_http, "example.com");
        assert_eq!(port_http, 80);

        let (scheme_https, host_https, port_https) =
            parse_target("Https://api.example.com:8443").unwrap();
        assert_eq!(scheme_https, TargetScheme::Https);
        assert_eq!(host_https, "api.example.com");
        assert_eq!(port_https, 8443);

        let (scheme_ws, host_ws, port_ws) = parse_target("ws://socket.local").unwrap();
        assert_eq!(scheme_ws, TargetScheme::Http);
        assert_eq!(host_ws, "socket.local");
        assert_eq!(port_ws, 80);

        let (scheme_wss, host_wss, port_wss) = parse_target("WSS://socket.secure.local").unwrap();
        assert_eq!(scheme_wss, TargetScheme::Https);
        assert_eq!(host_wss, "socket.secure.local");
        assert_eq!(port_wss, 443);
    }

    #[test]
    fn parse_target_rejects_path() {
        let err = parse_target("https://example.com/api").unwrap_err();
        assert!(err.contains("暂不支持路径"));
    }

    #[test]
    fn rewrite_path_and_query_respects_strip_prefix() {
        let route = ProxyRoute {
            host: None,
            path_prefix: "/api".to_string(),
            target_scheme: TargetScheme::Http,
            target_host: "127.0.0.1".to_string(),
            target_port: 3000,
            strip_prefix: true,
            allow_insecure_tls: false,
        };

        let uri: Uri = "/api/user/list?page=1".parse().unwrap();
        assert_eq!(rewrite_path_and_query(&uri, &route), "/user/list?page=1");
    }

    #[test]
    fn build_routes_prefers_more_specific_host_when_prefix_equal() {
        let routes = build_routes(&[
            enabled_route("", "/api", "http://127.0.0.1:3001"),
            enabled_route("api.example.com", "/api", "http://127.0.0.1:3002"),
        ])
        .unwrap();

        let selected = select_route(&routes, Some("api.example.com"), "/api/users").unwrap();
        assert_eq!(selected.host.as_deref(), Some("api.example.com"));
        assert_eq!(selected.target_port, 3002);
    }

    #[test]
    fn build_routes_prefers_longest_path_prefix() {
        let routes = build_routes(&[
            enabled_route("", "/api", "http://127.0.0.1:3001"),
            enabled_route("", "/api/admin", "http://127.0.0.1:3002"),
        ])
        .unwrap();

        let selected = select_route(&routes, None, "/api/admin/users").unwrap();
        assert_eq!(selected.path_prefix, "/api/admin");
        assert_eq!(selected.target_port, 3002);
    }

    #[test]
    fn path_match_handles_boundary_correctly() {
        assert!(path_match("/", "/anything"));
        assert!(path_match("/api", "/api"));
        assert!(path_match("/api", "/api/user"));
        assert!(!path_match("/api", "/apix"));
    }
}
