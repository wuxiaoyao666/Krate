use native_tls::{HandshakeError, TlsConnector, TlsStream};
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{command, State};

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStartRequest {
    listen_host: String,
    listen_port: u16,
    routes: Vec<ProxyRouteInput>,
}

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

struct ProxyRuntime {
    stop_flag: Arc<AtomicBool>,
    handle: thread::JoinHandle<()>,
}

struct ProxySnapshot {
    running: bool,
    listen_host: Option<String>,
    listen_port: Option<u16>,
    route_count: usize,
    started_at: Option<u64>,
    last_error: Option<String>,
    message: String,
}

pub struct ProxyState {
    runtime: Mutex<Option<ProxyRuntime>>,
    snapshot: Mutex<ProxySnapshot>,
    total_requests: Arc<AtomicU64>,
}

impl ProxyState {
    pub fn new() -> Self {
        Self {
            runtime: Mutex::new(None),
            snapshot: Mutex::new(ProxySnapshot {
                running: false,
                listen_host: None,
                listen_port: None,
                route_count: 0,
                started_at: None,
                last_error: None,
                message: "代理服务未启动".to_string(),
            }),
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum TargetScheme {
    Http,
    Https,
}

#[derive(Clone)]
struct ProxyRoute {
    host: Option<String>,
    path_prefix: String,
    target_scheme: TargetScheme,
    target_host: String,
    target_port: u16,
    strip_prefix: bool,
    allow_insecure_tls: bool,
}

struct HttpRequest {
    method: String,
    uri: String,
    version: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    host: String,
    path: String,
    query: Option<String>,
}

enum UpstreamStream {
    Tcp(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl Read for UpstreamStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            UpstreamStream::Tcp(stream) => stream.read(buf),
            UpstreamStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for UpstreamStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            UpstreamStream::Tcp(stream) => stream.write(buf),
            UpstreamStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            UpstreamStream::Tcp(stream) => stream.flush(),
            UpstreamStream::Tls(stream) => stream.flush(),
        }
    }
}

#[command]
pub fn proxy_get_status(state: State<ProxyState>) -> ProxyStatus {
    state.status()
}

#[command]
pub fn proxy_start(
    state: State<ProxyState>,
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

    let mut runtime_guard = state
        .runtime
        .lock()
        .map_err(|_| "代理状态锁异常".to_string())?;
    if runtime_guard.is_some() {
        return Err("代理服务已经在运行，请先停止再启动".to_string());
    }

    let bind_addr = format!("{}:{}", listen_host, config.listen_port);
    let listener =
        TcpListener::bind(&bind_addr).map_err(|err| format!("监听失败 {}: {}", bind_addr, err))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("设置监听模式失败: {}", err))?;

    state.total_requests.store(0, Ordering::Relaxed);

    let routes = Arc::new(routes);
    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_for_thread = stop_flag.clone();
    let routes_for_thread = routes.clone();
    let total_requests = state.total_requests.clone();

    let handle = thread::Builder::new()
        .name("krate-proxy".to_string())
        .spawn(move || {
            proxy_accept_loop(
                listener,
                stop_flag_for_thread,
                routes_for_thread,
                total_requests,
            )
        })
        .map_err(|err| format!("启动代理线程失败: {}", err))?;

    *runtime_guard = Some(ProxyRuntime { stop_flag, handle });
    drop(runtime_guard);

    let mut snapshot = state
        .snapshot
        .lock()
        .map_err(|_| "代理状态锁异常".to_string())?;
    snapshot.running = true;
    snapshot.listen_host = Some(listen_host);
    snapshot.listen_port = Some(config.listen_port);
    snapshot.route_count = routes.len();
    snapshot.started_at = Some(current_timestamp());
    snapshot.last_error = None;
    snapshot.message = format!("代理服务运行中，共 {} 条路由", routes.len());
    drop(snapshot);

    Ok(state.status())
}

#[command]
pub fn proxy_stop(state: State<ProxyState>) -> Result<ProxyStatus, String> {
    let runtime = {
        let mut guard = state
            .runtime
            .lock()
            .map_err(|_| "代理状态锁异常".to_string())?;
        guard.take()
    };

    if let Some(runtime) = runtime {
        runtime.stop_flag.store(true, Ordering::Relaxed);
        let _ = runtime.handle.join();
    }

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
    drop(snapshot);

    Ok(state.status())
}

fn proxy_accept_loop(
    listener: TcpListener,
    stop_flag: Arc<AtomicBool>,
    routes: Arc<Vec<ProxyRoute>>,
    total_requests: Arc<AtomicU64>,
) {
    while !stop_flag.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let routes = routes.clone();
                let total_requests = total_requests.clone();
                thread::spawn(move || {
                    let _ = handle_client(stream, peer, routes, total_requests);
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(30));
            }
            Err(_) => {
                thread::sleep(Duration::from_millis(120));
            }
        }
    }
}

fn handle_client(
    mut client: TcpStream,
    peer: SocketAddr,
    routes: Arc<Vec<ProxyRoute>>,
    total_requests: Arc<AtomicU64>,
) -> Result<(), String> {
    client
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|err| err.to_string())?;
    client
        .set_write_timeout(Some(Duration::from_secs(15)))
        .map_err(|err| err.to_string())?;

    let request = match read_http_request(&mut client) {
        Ok(req) => req,
        Err(err) => {
            write_error_response(&mut client, "400 Bad Request", &err);
            return Ok(());
        }
    };

    let request_host = normalize_host_value(&request.host);
    let route = match select_route(&routes, request_host.as_deref(), &request.path) {
        Some(route) => route,
        None => {
            write_error_response(&mut client, "404 Not Found", "未匹配到可用的反向代理路由");
            return Ok(());
        }
    };

    if let Err(err) = forward_request(&mut client, &request, route, peer, &total_requests) {
        write_error_response(
            &mut client,
            "502 Bad Gateway",
            &format!("上游服务不可用: {}", err),
        );
    }

    Ok(())
}

fn forward_request(
    client: &mut TcpStream,
    request: &HttpRequest,
    route: &ProxyRoute,
    peer: SocketAddr,
    total_requests: &Arc<AtomicU64>,
) -> Result<(), String> {
    if is_websocket_upgrade(request) {
        forward_websocket(client, request, route, peer, total_requests)?;
        total_requests.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    let mut upstream = connect_upstream(route)?;
    let outbound = build_outbound_request(request, route, peer, false);

    upstream
        .write_all(&outbound)
        .map_err(|err| format!("转发请求失败: {}", err))?;
    upstream.flush().map_err(|err| err.to_string())?;

    proxy_response_to_client(client, &mut upstream)?;
    total_requests.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

fn forward_websocket(
    client: &mut TcpStream,
    request: &HttpRequest,
    route: &ProxyRoute,
    peer: SocketAddr,
    _total_requests: &Arc<AtomicU64>,
) -> Result<(), String> {
    let mut upstream = connect_upstream(route)?;
    let outbound = build_outbound_request(request, route, peer, true);
    upstream
        .write_all(&outbound)
        .map_err(|err| format!("转发 WebSocket 握手失败: {}", err))?;
    upstream.flush().map_err(|err| err.to_string())?;

    let (head, tail, status_code) = read_http_response_head(&mut upstream)?;
    client
        .write_all(&head)
        .map_err(|err| format!("写回握手响应失败: {}", err))?;
    if !tail.is_empty() {
        client
            .write_all(&tail)
            .map_err(|err| format!("写回握手剩余数据失败: {}", err))?;
    }

    if status_code != Some(101) {
        proxy_response_to_client(client, &mut upstream)?;
        return Ok(());
    }

    tunnel_upgraded_connection(client, &mut upstream)?;
    Ok(())
}

fn connect_upstream(route: &ProxyRoute) -> Result<UpstreamStream, String> {
    let upstream_addr = format!("{}:{}", route.target_host, route.target_port);
    let tcp = TcpStream::connect(&upstream_addr)
        .map_err(|err| format!("连接上游失败 {}: {}", upstream_addr, err))?;

    set_stream_timeouts(&tcp, 20)?;

    if route.target_scheme == TargetScheme::Http {
        return Ok(UpstreamStream::Tcp(tcp));
    }

    let mut builder = TlsConnector::builder();
    builder.danger_accept_invalid_certs(route.allow_insecure_tls);
    builder.danger_accept_invalid_hostnames(route.allow_insecure_tls);
    let connector = builder
        .build()
        .map_err(|err| format!("初始化 TLS 连接器失败: {}", err))?;

    let tls = connector
        .connect(&route.target_host, tcp)
        .map_err(|err| match err {
            HandshakeError::Failure(inner) => format!("TLS 握手失败: {}", inner),
            HandshakeError::WouldBlock(_) => "TLS 握手超时".to_string(),
        })?;

    set_stream_timeouts(tls.get_ref(), 20)?;
    Ok(UpstreamStream::Tls(tls))
}

fn set_stream_timeouts(stream: &TcpStream, seconds: u64) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(seconds)))
        .map_err(|err| err.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(seconds)))
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn proxy_response_to_client(
    client: &mut TcpStream,
    upstream: &mut UpstreamStream,
) -> Result<(), String> {
    let mut buffer = [0u8; 8192];
    loop {
        let size = upstream
            .read(&mut buffer)
            .map_err(|err| format!("读取上游响应失败: {}", err))?;
        if size == 0 {
            break;
        }
        client
            .write_all(&buffer[..size])
            .map_err(|err| format!("写回客户端失败: {}", err))?;
    }
    Ok(())
}

fn set_upstream_poll_timeout(
    stream: &mut UpstreamStream,
    read_timeout_ms: u64,
) -> Result<(), String> {
    let read_timeout = Some(Duration::from_millis(read_timeout_ms));
    let write_timeout = Some(Duration::from_secs(30));

    match stream {
        UpstreamStream::Tcp(inner) => {
            inner
                .set_read_timeout(read_timeout)
                .map_err(|err| err.to_string())?;
            inner
                .set_write_timeout(write_timeout)
                .map_err(|err| err.to_string())?;
        }
        UpstreamStream::Tls(inner) => {
            inner
                .get_mut()
                .set_read_timeout(read_timeout)
                .map_err(|err| err.to_string())?;
            inner
                .get_mut()
                .set_write_timeout(write_timeout)
                .map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

fn tunnel_upgraded_connection(
    client: &mut TcpStream,
    upstream: &mut UpstreamStream,
) -> Result<(), String> {
    client
        .set_read_timeout(Some(Duration::from_millis(80)))
        .map_err(|err| err.to_string())?;
    client
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| err.to_string())?;
    set_upstream_poll_timeout(upstream, 80)?;

    let mut client_closed = false;
    let mut upstream_closed = false;
    let mut idle_cycles: u32 = 0;
    let mut c_buf = [0u8; 8192];
    let mut u_buf = [0u8; 8192];

    loop {
        let mut progressed = false;

        if !client_closed {
            match client.read(&mut c_buf) {
                Ok(0) => client_closed = true,
                Ok(size) => {
                    upstream
                        .write_all(&c_buf[..size])
                        .map_err(|err| format!("写入上游 WebSocket 数据失败: {}", err))?;
                    upstream.flush().map_err(|err| err.to_string())?;
                    progressed = true;
                }
                Err(err) if is_retryable_io_error(&err) => {}
                Err(err) if is_disconnected_io_error(&err) => client_closed = true,
                Err(err) => return Err(format!("读取客户端 WebSocket 数据失败: {}", err)),
            }
        }

        if !upstream_closed {
            match upstream.read(&mut u_buf) {
                Ok(0) => upstream_closed = true,
                Ok(size) => {
                    client
                        .write_all(&u_buf[..size])
                        .map_err(|err| format!("写回客户端 WebSocket 数据失败: {}", err))?;
                    progressed = true;
                }
                Err(err) if is_retryable_io_error(&err) => {}
                Err(err) if is_disconnected_io_error(&err) => upstream_closed = true,
                Err(err) => return Err(format!("读取上游 WebSocket 数据失败: {}", err)),
            }
        }

        if client_closed && upstream_closed {
            break;
        }

        if progressed {
            idle_cycles = 0;
            continue;
        }

        idle_cycles += 1;
        if idle_cycles > 1200 && (client_closed || upstream_closed) {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let _ = client.shutdown(Shutdown::Both);
    match upstream {
        UpstreamStream::Tcp(inner) => {
            let _ = inner.shutdown(Shutdown::Both);
        }
        UpstreamStream::Tls(inner) => {
            let _ = inner.get_mut().shutdown(Shutdown::Both);
        }
    }
    Ok(())
}

fn build_outbound_request(
    request: &HttpRequest,
    route: &ProxyRoute,
    peer: SocketAddr,
    websocket_mode: bool,
) -> Vec<u8> {
    let outbound_uri = rewrite_uri(&request.uri, &request.path, request.query.as_deref(), route);
    let mut outbound = Vec::<u8>::new();
    outbound.extend_from_slice(
        format!(
            "{} {} {}\r\n",
            request.method, outbound_uri, request.version
        )
        .as_bytes(),
    );

    let mut has_content_length = false;
    for (name, value) in &request.headers {
        let lower = name.to_ascii_lowercase();
        if websocket_mode && lower == "upgrade" {
            continue;
        }
        if matches!(
            lower.as_str(),
            "host" | "connection" | "proxy-connection" | "keep-alive" | "transfer-encoding"
        ) {
            continue;
        }
        if lower == "content-length" {
            has_content_length = true;
        }
        outbound.extend_from_slice(format!("{}: {}\r\n", name, value).as_bytes());
    }

    if (route.target_scheme == TargetScheme::Http && route.target_port == 80)
        || (route.target_scheme == TargetScheme::Https && route.target_port == 443)
    {
        outbound.extend_from_slice(format!("Host: {}\r\n", route.target_host).as_bytes());
    } else {
        outbound.extend_from_slice(
            format!("Host: {}:{}\r\n", route.target_host, route.target_port).as_bytes(),
        );
    }

    if websocket_mode {
        let upgrade = get_header_value(request, "upgrade").unwrap_or("websocket");
        outbound.extend_from_slice(format!("Upgrade: {}\r\n", upgrade).as_bytes());
        outbound.extend_from_slice(b"Connection: Upgrade\r\n");
    } else {
        outbound.extend_from_slice(b"Connection: close\r\n");
    }

    outbound.extend_from_slice(format!("X-Forwarded-For: {}\r\n", peer.ip()).as_bytes());
    if !request.host.is_empty() {
        outbound.extend_from_slice(format!("X-Forwarded-Host: {}\r\n", request.host).as_bytes());
    }
    outbound.extend_from_slice(b"X-Forwarded-Proto: http\r\n");

    if !request.body.is_empty() && !has_content_length {
        outbound
            .extend_from_slice(format!("Content-Length: {}\r\n", request.body.len()).as_bytes());
    }

    outbound.extend_from_slice(b"\r\n");
    if !request.body.is_empty() {
        outbound.extend_from_slice(&request.body);
    }

    outbound
}

fn is_retryable_io_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    )
}

fn is_disconnected_io_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::UnexpectedEof
    )
}

fn read_http_response_head<R: Read>(
    reader: &mut R,
) -> Result<(Vec<u8>, Vec<u8>, Option<u16>), String> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 2048];

    loop {
        let size = reader
            .read(&mut temp)
            .map_err(|err| format!("读取上游响应失败: {}", err))?;
        if size == 0 {
            return Err("上游在响应头返回前断开连接".to_string());
        }

        buffer.extend_from_slice(&temp[..size]);
        if let Some(pos) = find_header_end(&buffer) {
            let head_len = pos + 4;
            let status_code = parse_status_code(&buffer[..head_len]);
            let head = buffer[..head_len].to_vec();
            let tail = buffer[head_len..].to_vec();
            return Ok((head, tail, status_code));
        }

        if buffer.len() > 1024 * 1024 {
            return Err("上游响应头过大".to_string());
        }
    }
}

fn parse_status_code(raw_head: &[u8]) -> Option<u16> {
    let text = String::from_utf8_lossy(raw_head);
    let line = text.lines().next()?;
    let mut parts = line.split_whitespace();
    let _http_version = parts.next()?;
    let code = parts.next()?;
    code.parse::<u16>().ok()
}

fn is_websocket_upgrade(request: &HttpRequest) -> bool {
    let upgrade = get_header_value(request, "upgrade")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let connection = get_header_value(request, "connection")
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();

    upgrade == "websocket" && connection.contains("upgrade")
}

fn get_header_value<'a>(request: &'a HttpRequest, key: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, value)| value.as_str())
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut temp = [0u8; 2048];
    let header_end;

    loop {
        let size = stream.read(&mut temp).map_err(|err| err.to_string())?;
        if size == 0 {
            return Err("客户端已断开连接".to_string());
        }
        buffer.extend_from_slice(&temp[..size]);

        if let Some(pos) = find_header_end(&buffer) {
            header_end = pos;
            break;
        }

        if buffer.len() > 1024 * 1024 {
            return Err("请求头过大".to_string());
        }
    }

    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| "请求行为空".to_string())?;

    let mut first = request_line.split_whitespace();
    let method = first
        .next()
        .ok_or_else(|| "请求行 method 缺失".to_string())?
        .to_string();
    let uri = first
        .next()
        .ok_or_else(|| "请求行 uri 缺失".to_string())?
        .to_string();
    let version = first.next().unwrap_or("HTTP/1.1").to_string();

    let mut headers = Vec::new();
    let mut content_length = 0usize;
    let mut host = String::new();
    let mut chunked = false;

    for line in lines {
        if line.is_empty() {
            continue;
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("非法请求头: {}", line))?;
        let key = name.trim().to_string();
        let val = value.trim().to_string();
        let lower = key.to_ascii_lowercase();

        if lower == "content-length" {
            content_length = val
                .parse::<usize>()
                .map_err(|_| "非法 Content-Length".to_string())?;
        }
        if lower == "host" {
            host = val.clone();
        }
        if lower == "transfer-encoding" && val.to_ascii_lowercase().contains("chunked") {
            chunked = true;
        }

        headers.push((key, val));
    }

    if chunked {
        return Err("暂不支持 chunked 请求体".to_string());
    }

    let mut body = buffer[(header_end + 4)..].to_vec();
    if content_length > 0 && body.len() < content_length {
        let mut missing = vec![0u8; content_length - body.len()];
        stream
            .read_exact(&mut missing)
            .map_err(|err| format!("读取请求体失败: {}", err))?;
        body.extend_from_slice(&missing);
    } else if content_length > 0 && body.len() > content_length {
        body.truncate(content_length);
    } else if content_length == 0 {
        body.clear();
    }

    let (path, query) = split_uri(&uri);

    Ok(HttpRequest {
        method,
        uri,
        version,
        headers,
        body,
        host,
        path,
        query,
    })
}

fn split_uri(uri: &str) -> (String, Option<String>) {
    let path_and_query = if uri.starts_with("http://") || uri.starts_with("https://") {
        let rest = if uri.starts_with("http://") {
            uri.trim_start_matches("http://")
        } else {
            uri.trim_start_matches("https://")
        };
        let slash_index = rest.find('/').unwrap_or(rest.len());
        if slash_index == rest.len() {
            "/".to_string()
        } else {
            rest[slash_index..].to_string()
        }
    } else if uri.starts_with('/') {
        uri.to_string()
    } else {
        format!("/{}", uri)
    };

    match path_and_query.split_once('?') {
        Some((path, query)) => (path.to_string(), Some(query.to_string())),
        None => (path_and_query, None),
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    if buffer.len() < 4 {
        return None;
    }
    buffer.windows(4).position(|w| w == b"\r\n\r\n")
}

fn build_routes(inputs: &[ProxyRouteInput]) -> Result<Vec<ProxyRoute>, String> {
    let mut routes = Vec::new();

    for route in inputs.iter().filter(|item| item.enabled) {
        let path_prefix = normalize_path_prefix(&route.path_prefix);
        let host = normalize_host_value(&route.host);
        let (target_scheme, target_host, target_port) = parse_target(&route.target)?;

        let _ = (&route.id, &route.name);
        routes.push(ProxyRoute {
            host,
            path_prefix,
            target_scheme,
            target_host,
            target_port,
            strip_prefix: route.strip_prefix,
            allow_insecure_tls: route.allow_insecure_tls,
        });
    }

    routes.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));
    Ok(routes)
}

fn parse_target(raw: &str) -> Result<(TargetScheme, String, u16), String> {
    let normalized = raw.trim().trim_end_matches('/').to_string();
    if normalized.is_empty() {
        return Err("目标地址不能为空".to_string());
    }

    let (scheme, rest) = if let Some(next) = normalized.strip_prefix("http://") {
        (TargetScheme::Http, next)
    } else if let Some(next) = normalized.strip_prefix("https://") {
        (TargetScheme::Https, next)
    } else {
        return Err("目标地址必须以 http:// 或 https:// 开头".to_string());
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

    let default_port = if scheme == TargetScheme::Https {
        443
    } else {
        80
    };

    if let Some((host, port_text)) = rest.rsplit_once(':') {
        let host = host.trim();
        let port = port_text
            .trim()
            .parse::<u16>()
            .map_err(|_| "目标端口非法".to_string())?;

        if host.is_empty() {
            return Err("目标主机不能为空".to_string());
        }

        return Ok((scheme, host.to_string(), port));
    }

    Ok((scheme, rest.to_string(), default_port))
}

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

fn rewrite_uri(uri: &str, path: &str, query: Option<&str>, route: &ProxyRoute) -> String {
    let mut next_path = path.to_string();

    if route.strip_prefix && route.path_prefix != "/" {
        if let Some(rest) = path.strip_prefix(&route.path_prefix) {
            next_path = if rest.is_empty() {
                "/".to_string()
            } else if rest.starts_with('/') {
                rest.to_string()
            } else {
                format!("/{}", rest)
            };
        }
    }

    match query {
        Some(query) if !query.is_empty() => format!("{}?{}", next_path, query),
        _ => {
            if uri.contains('?') && next_path == "/" {
                "/".to_string()
            } else {
                next_path
            }
        }
    }
}

fn write_error_response(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
