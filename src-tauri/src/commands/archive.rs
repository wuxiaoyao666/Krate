use serde::Serialize;
use tauri::{command, Emitter};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use rand::rngs::OsRng;
use rand::TryRngCore;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use walkdir::WalkDir;

use argon2::Argon2;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use zeroize::Zeroize;

use aead::generic_array::typenum::Unsigned;
use aead::stream::{DecryptorBE32, EncryptorBE32, Nonce, StreamBE32};

const MAGIC_HEADER: &[u8; 10] = b"KRATE_PKG\0";
const VERSION_V1: u8 = 1;

const FLAG_ENCRYPTED: u8 = 0b0000_0001;
const FLAG_COMPRESSED: u8 = 0b0000_0010;

// 分块写入大小（明文块）
// 越大：更快/更省 header；越小：更细粒度的校验/更平滑的进度（但开销更大）
const PLAIN_CHUNK: usize = 64 * 1024;

// 我们自己的帧格式：
// u32 header（最高位=是否 last，低 31 位=密文长度） + ciphertext bytes
fn pack_chunk_header(is_last: bool, len: usize) -> [u8; 4] {
    let mut v = len as u32;
    if is_last {
        v |= 0x8000_0000;
    }
    v.to_be_bytes()
}

fn unpack_chunk_header(b: [u8; 4]) -> (bool, usize) {
    let v = u32::from_be_bytes(b);
    let is_last = (v & 0x8000_0000) != 0;
    let len = (v & 0x7FFF_FFFF) as usize;
    (is_last, len)
}

/// 给前端的进度事件
#[derive(Debug, Clone, Serialize)]
pub struct KrateProgress {
    pub phase: String, // "scan" | "pack" | "unpack"
    pub current: u64,
    pub total: u64,
    pub message: String,
}

fn emit_progress(window: &tauri::Window, payload: KrateProgress) {
    // 前端 listen('krate://progress', ...)
    let _ = window.emit("krate://progress", payload);
}

/// 展开 inputs：把文件夹递归成文件列表，用于进度统计和打包
fn collect_entries(inputs: &[String]) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    // (src_path, archive_rel_path)
    let mut out = Vec::new();

    // 为了避免多个目录同名冲突：做个简单的去重命名
    let mut used_top = std::collections::HashMap::<String, u32>::new();

    for p in inputs {
        let src = PathBuf::from(p);
        if !src.exists() {
            return Err(format!("路径不存在: {}", p));
        }

        let base_name = src
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("input")
            .to_string();

        let n = used_top.entry(base_name.clone()).or_insert(0);
        *n += 1;

        let top = if *n == 1 {
            base_name.clone()
        } else {
            format!("{}-{}", base_name, n)
        };

        if src.is_dir() {
            for e in WalkDir::new(&src).follow_links(false) {
                let e = e.map_err(|e| e.to_string())?;
                let path = e.path().to_path_buf();
                if path.is_file() {
                    let rel = path.strip_prefix(&src).map_err(|e| e.to_string())?;
                    let arc = PathBuf::from(&top).join(rel);
                    out.push((path, arc));
                }
            }
        } else {
            out.push((src.clone(), PathBuf::from(&top)));
        }
    }

    Ok(out)
}

/// ========= Stream Encrypt Writer =========
/// tar/gzip 会连续 write 明文到这个 writer；
/// 我们把明文凑够 64KB 就 encrypt_next，最后剩余部分 encrypt_last。
struct StreamEncryptWriter<W: Write> {
    inner: W,
    enc: Option<EncryptorBE32<ChaCha20Poly1305>>,
    buf: Vec<u8>,
}

impl<W: Write> StreamEncryptWriter<W> {
    fn new(inner: W, enc: EncryptorBE32<ChaCha20Poly1305>) -> Self {
        Self {
            inner,
            enc: Some(enc),
            buf: Vec::with_capacity(PLAIN_CHUNK * 2),
        }
    }

    fn flush_chunks(&mut self) -> io::Result<()> {
        while self.buf.len() >= PLAIN_CHUNK {
            let chunk: Vec<u8> = self.buf.drain(..PLAIN_CHUNK).collect();
            let enc = self
                .enc
                .as_mut()
                .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "encryptor consumed"))?;

            // 注意：这里要传 slice，不能传 &Vec，否则你会遇到 Payload 的 E0277
            let ct = enc
                .encrypt_next(&chunk[..])
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "encrypt_next failed"))?;

            let hdr = pack_chunk_header(false, ct.len());
            self.inner.write_all(&hdr)?;
            self.inner.write_all(&ct)?;
        }
        Ok(())
    }

    fn finish(mut self) -> io::Result<W> {
        // flush 中间块
        self.flush_chunks()?;

        // last chunk
        let last_plain = self.buf;
        let enc = self
            .enc
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "encryptor consumed"))?;

        let ct = enc
            .encrypt_last(&last_plain[..])
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "encrypt_last failed"))?;

        let hdr = pack_chunk_header(true, ct.len());
        self.inner.write_all(&hdr)?;
        self.inner.write_all(&ct)?;
        self.inner.flush()?;

        Ok(self.inner)
    }
}

impl<W: Write> Write for StreamEncryptWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        self.flush_chunks()?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// ========= Stream Decrypt Reader =========
/// 读取帧：u32 header + ciphertext，按 is_last 决定 decrypt_next/decrypt_last
struct StreamDecryptReader<R: Read> {
    inner: R,
    dec: Option<DecryptorBE32<ChaCha20Poly1305>>,
    done: bool,
    out_buf: Vec<u8>,
    out_pos: usize,
}

impl<R: Read> StreamDecryptReader<R> {
    fn new(inner: R, dec: DecryptorBE32<ChaCha20Poly1305>) -> Self {
        Self {
            inner,
            dec: Some(dec),
            done: false,
            out_buf: Vec::new(),
            out_pos: 0,
        }
    }

    fn refill(&mut self) -> io::Result<bool> {
        if self.done {
            return Ok(false);
        }

        // 读 header（4 bytes）
        let mut hdr = [0u8; 4];
        match self.inner.read_exact(&mut hdr) {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                // 没有更多数据
                self.done = true;
                return Ok(false);
            }
            Err(e) => return Err(e),
        }

        let (is_last, ct_len) = unpack_chunk_header(hdr);
        if ct_len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid chunk length",
            ));
        }

        let mut ct = vec![0u8; ct_len];
        self.inner.read_exact(&mut ct)?;

        let dec = self
            .dec
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "decryptor consumed"))?;

        let pt = if is_last {
            // decrypt_last 会“消耗” decryptor
            let pt = dec
                .decrypt_last(&ct[..])
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "decrypt_last failed"))?;
            self.done = true;
            pt
        } else {
            let mut dec2 = dec;
            let pt = dec2
                .decrypt_next(&ct[..])
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "decrypt_next failed"))?;
            self.dec = Some(dec2);
            pt
        };

        self.out_buf = pt;
        self.out_pos = 0;
        Ok(true)
    }
}

impl<R: Read> Read for StreamDecryptReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.out_pos >= self.out_buf.len() {
            self.out_buf.clear();
            self.out_pos = 0;

            let ok = self.refill()?;
            if !ok {
                return Ok(0);
            }
        }

        let n = std::cmp::min(buf.len(), self.out_buf.len() - self.out_pos);
        buf[..n].copy_from_slice(&self.out_buf[self.out_pos..self.out_pos + n]);
        self.out_pos += n;
        Ok(n)
    }
}

/// 由密码派生 key（32 bytes）
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut out = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut out)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// ========== 新版：创建归档（可加密） ==========
#[command]
pub async fn create_archive(
    window: tauri::Window,
    inputs: Vec<String>,
    output_path: String,
    password: Option<String>,
    gzip_level: Option<u32>, // 0-9
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        create_archive_blocking(window, inputs, output_path, password, gzip_level)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn create_archive_blocking(
    window: tauri::Window,
    inputs: Vec<String>,
    output_path: String,
    password: Option<String>,
    gzip_level: Option<u32>,
) -> Result<(), String> {
    emit_progress(
        &window,
        KrateProgress {
            phase: "scan".into(),
            current: 0,
            total: 0,
            message: "正在扫描文件...".into(),
        },
    );

    let entries = collect_entries(&inputs)?;
    let total = entries.len() as u64;

    if total == 0 {
        return Err("没有可打包的文件".into());
    }

    let file = File::create(&output_path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    // 写魔法头
    writer.write_all(MAGIC_HEADER).map_err(|e| e.to_string())?;

    // 写版本与 flags
    let mut flags = FLAG_COMPRESSED;
    let encrypted = password.as_ref().map(|s| !s.is_empty()).unwrap_or(false);
    if encrypted {
        flags |= FLAG_ENCRYPTED;
    }

    writer.write_all(&[VERSION_V1]).map_err(|e| e.to_string())?;
    writer.write_all(&[flags]).map_err(|e| e.to_string())?;

    // gzip level
    let lvl = gzip_level.unwrap_or(9).min(9) as u8;
    writer.write_all(&[lvl]).map_err(|e| e.to_string())?;

    // 如果加密：写 salt + nonce
    let mut key_bytes_opt: Option<[u8; 32]> = None;

    // stream nonce size: NonceSize<ChaCha20Poly1305, StreamBE32<ChaCha20Poly1305>>::USIZE
    let nonce_len = <aead::stream::NonceSize<ChaCha20Poly1305, StreamBE32<ChaCha20Poly1305>> as Unsigned>::USIZE;

    let mut salt = [0u8; 16];
    let mut nonce_bytes = vec![0u8; nonce_len];

    if encrypted {
        OsRng
            .try_fill_bytes(&mut salt)
            .map_err(|e| format!("生成 salt 失败: {e}"))?;

        OsRng
            .try_fill_bytes(&mut nonce_bytes)
            .map_err(|e| format!("生成 nonce 失败: {e}"))?;

        writer
            .write_all(&[salt.len() as u8])
            .map_err(|e| e.to_string())?;
        writer.write_all(&salt).map_err(|e| e.to_string())?;

        writer
            .write_all(&[nonce_bytes.len() as u8])
            .map_err(|e| e.to_string())?;
        writer.write_all(&nonce_bytes).map_err(|e| e.to_string())?;

        let pw = password.as_ref().unwrap();
        let key_bytes = derive_key(pw, &salt)?;
        key_bytes_opt = Some(key_bytes);
    } else {
        // 未加密：salt_len = 0, nonce_len = 0
        writer.write_all(&[0u8]).map_err(|e| e.to_string())?;
        writer.write_all(&[0u8]).map_err(|e| e.to_string())?;
    }

    // 构造 payload writer：加密 or 直写
    // 这里保持“先压缩再加密”：tar -> gzip -> (encrypt writer) -> file
    let compression = Compression::new(lvl.into());

    if encrypted {
        let mut key_bytes = key_bytes_opt.unwrap();
        let cipher = ChaCha20Poly1305::new((&key_bytes).into());

        let nonce =
            Nonce::<ChaCha20Poly1305, StreamBE32<ChaCha20Poly1305>>::from_slice(&nonce_bytes);
        let enc = EncryptorBE32::from_aead(cipher, nonce);

        // encrypt writer
        let enc_writer = StreamEncryptWriter::new(writer, enc);

        // gzip -> tar
        let gz = GzEncoder::new(enc_writer, compression);
        let mut tar_builder = tar::Builder::new(gz);

        emit_progress(
            &window,
            KrateProgress {
                phase: "pack".into(),
                current: 0,
                total,
                message: "正在打包...".into(),
            },
        );

        for (i, (src, arc)) in entries.iter().enumerate() {
            tar_builder
                .append_path_with_name(src, arc)
                .map_err(|e| e.to_string())?;

            emit_progress(
                &window,
                KrateProgress {
                    phase: "pack".into(),
                    current: (i as u64) + 1,
                    total,
                    message: format!("已打包: {}", arc.display()),
                },
            );
        }

        // finish tar -> finish gz -> finish enc writer
        let gz = tar_builder.into_inner().map_err(|e| e.to_string())?;
        let enc_writer = gz.finish().map_err(|e| e.to_string())?;
        let _writer = enc_writer.finish().map_err(|e| e.to_string())?;

        key_bytes.zeroize();
        nonce_bytes.zeroize();
        salt.zeroize();
    } else {
        // 未加密：tar -> gzip -> file
        let gz = GzEncoder::new(writer, compression);
        let mut tar_builder = tar::Builder::new(gz);

        emit_progress(
            &window,
            KrateProgress {
                phase: "pack".into(),
                current: 0,
                total,
                message: "正在打包...".into(),
            },
        );

        for (i, (src, arc)) in entries.iter().enumerate() {
            tar_builder
                .append_path_with_name(src, arc)
                .map_err(|e| e.to_string())?;

            emit_progress(
                &window,
                KrateProgress {
                    phase: "pack".into(),
                    current: (i as u64) + 1,
                    total,
                    message: format!("已打包: {}", arc.display()),
                },
            );
        }

        let mut gz = tar_builder.into_inner().map_err(|e| e.to_string())?;
        gz.flush().map_err(|e| e.to_string())?;
        let _w = gz.finish().map_err(|e| e.to_string())?;
    }

    emit_progress(
        &window,
        KrateProgress {
            phase: "pack".into(),
            current: total,
            total,
            message: "打包完成".into(),
        },
    );

    Ok(())
}

/// ========== 新版：解包（可解密） ==========
#[command]
pub async fn extract_archive(
    window: tauri::Window,
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        extract_archive_blocking(window, archive_path, output_dir, password)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn extract_archive_blocking(
    window: tauri::Window,
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    let file = File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);

    // 验证 magic header
    let mut magic = [0u8; 10];
    reader
        .read_exact(&mut magic)
        .map_err(|_| "无法读取文件头".to_string())?;
    if &magic != MAGIC_HEADER {
        return Err("无法识别的文件格式（magic 不匹配）".into());
    }

    // 尝试读版本；如果下一字节是 gzip 头 0x1F，则认为是“旧格式”
    let mut b = [0u8; 1];
    reader.read_exact(&mut b).map_err(|e| e.to_string())?;
    let first = b[0];

    if first == 0x1F {
        // 旧格式：把 0x1F 放回去（用一个 Chain）
        let chained = io::Read::chain(&b[..], reader);
        let gz = GzDecoder::new(chained);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(&output_dir).map_err(|e| e.to_string())?;
        return Ok(());
    }

    let version = first;
    if version != VERSION_V1 {
        return Err(format!("不支持的版本: {}", version));
    }

    // flags + gzip_level
    let mut flags = [0u8; 1];
    reader.read_exact(&mut flags).map_err(|e| e.to_string())?;
    let flags = flags[0];

    let mut lvl = [0u8; 1];
    reader.read_exact(&mut lvl).map_err(|e| e.to_string())?;
    let _gzip_level = lvl[0];

    let encrypted = (flags & FLAG_ENCRYPTED) != 0;

    // salt/nonce
    let mut salt_len = [0u8; 1];
    reader
        .read_exact(&mut salt_len)
        .map_err(|e| e.to_string())?;
    let salt_len = salt_len[0] as usize;

    let mut salt = vec![0u8; salt_len];
    if salt_len > 0 {
        reader.read_exact(&mut salt).map_err(|e| e.to_string())?;
    }

    let mut nonce_len = [0u8; 1];
    reader
        .read_exact(&mut nonce_len)
        .map_err(|e| e.to_string())?;
    let nonce_len = nonce_len[0] as usize;

    let mut nonce_bytes = vec![0u8; nonce_len];
    if nonce_len > 0 {
        reader
            .read_exact(&mut nonce_bytes)
            .map_err(|e| e.to_string())?;
    }

    emit_progress(
        &window,
        KrateProgress {
            phase: "unpack".into(),
            current: 0,
            total: 0,
            message: "正在解包...".into(),
        },
    );

    if encrypted {
        let pw = password.unwrap_or_default();
        if pw.is_empty() {
            return Err("该 krate 包已加密：请输入密码".into());
        }

        let mut key_bytes = derive_key(&pw, &salt)?;
        let cipher = ChaCha20Poly1305::new((&key_bytes).into());

        let nonce =
            Nonce::<ChaCha20Poly1305, StreamBE32<ChaCha20Poly1305>>::from_slice(&nonce_bytes);
        let dec = DecryptorBE32::from_aead(cipher, nonce);

        let dec_reader = StreamDecryptReader::new(reader, dec);
        let gz = GzDecoder::new(dec_reader);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(&output_dir).map_err(|e| e.to_string())?;

        key_bytes.zeroize();
        nonce_bytes.zeroize();
        salt.zeroize();
    } else {
        let gz = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(gz);
        archive.unpack(&output_dir).map_err(|e| e.to_string())?;
    }

    emit_progress(
        &window,
        KrateProgress {
            phase: "unpack".into(),
            current: 1,
            total: 1,
            message: "解包完成".into(),
        },
    );

    Ok(())
}
