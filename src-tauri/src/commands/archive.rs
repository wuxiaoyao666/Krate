use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use tauri::command;

// === 配置区 ===
const MAGIC_HEADER: &[u8; 9] = b"KRATE_PKG";
const INTERNAL_KEY: &[u8] = b"Krate_Internal_Secret_Key_2026_Performance_First";

// === 自定义流式加密写入器 ===
struct EncryptWriter<W: Write> {
    inner: W,
    key: Vec<u8>,
    position: usize,
    buffer: Vec<u8>,
}

impl<W: Write> EncryptWriter<W> {
    fn new(inner: W, key: Vec<u8>) -> Self {
        Self {
            inner,
            key,
            position: 0,
            buffer: Vec::with_capacity(8 * 1024), // 预分配 8KB 空间
        }
    }
}

impl<W: Write> Write for EncryptWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let key_len = self.key.len();
        if key_len == 0 {
            return self.inner.write(buf);
        }

        // 重用内存，避免每次 write 都进行堆分配
        self.buffer.clear();

        // 确保容量足够，不够时会自动扩容
        self.buffer.reserve(buf.len());

        for &b in buf {
            let k = self.key[self.position % key_len];
            self.position = self.position.wrapping_add(1);
            self.buffer.push(b ^ k);
        }

        self.inner.write(&self.buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

// === 自定义流式解密读取器 ===
struct DecryptReader<R: Read> {
    inner: R,
    key: Vec<u8>,
    position: usize,
}

impl<R: Read> DecryptReader<R> {
    fn new(inner: R, key: Vec<u8>) -> Self {
        Self {
            inner,
            key,
            position: 0,
        }
    }
}

impl<R: Read> Read for DecryptReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        let key_len = self.key.len();
        if key_len == 0 {
            return Ok(n);
        }

        for i in 0..n {
            let k = self.key[self.position % key_len];
            buf[i] ^= k;
            self.position = self.position.wrapping_add(1);
        }

        Ok(n)
    }
}

// 辅助：获取密钥
fn get_key(password: Option<String>) -> Vec<u8> {
    match password {
        Some(pwd) if !pwd.is_empty() => pwd.into_bytes(),
        _ => INTERNAL_KEY.to_vec(),
    }
}

// === 核心功能实现 ===

#[command]
pub async fn create_archive(
    inputs: Vec<String>,
    output_path: String,
    password: Option<String>,
    gzip_level: Option<u32>,
) -> Result<(), String> {
    let key = get_key(password);
    let level = gzip_level.unwrap_or(9); // 默认最高压缩

    let file = File::create(&output_path).map_err(|e| e.to_string())?;
    let mut writer = BufWriter::new(file);

    // 写入魔法头
    writer.write_all(MAGIC_HEADER).map_err(|e| e.to_string())?;

    // 构建管道: File <- BufWriter <- Encrypt <- Gzip <- Tar
    let encryptor = EncryptWriter::new(writer, key);
    let compressor = GzEncoder::new(encryptor, Compression::new(level));
    let mut tar = tar::Builder::new(compressor);

    // 遍历添加文件
    for path_str in inputs {
        let path = Path::new(&path_str);
        let name = path.file_name().ok_or("无效的文件路径")?;

        if path.is_dir() {
            tar.append_dir_all(name, path).map_err(|e| e.to_string())?;
        } else {
            let mut f = File::open(path).map_err(|e| e.to_string())?;
            tar.append_file(name, &mut f).map_err(|e| e.to_string())?;
        }
    }

    // 结束 tar，获取内部的 GzEncoder
    let compressor = tar
        .into_inner()
        .map_err(|e| format!("Tar finish failed: {}", e))?;

    // 结束 gzip，写入 footer 并获取内部的 EncryptWriter
    let mut encryptor = compressor
        .finish()
        .map_err(|e| format!("Gzip finish failed: {}", e))?;

    // 刷新加密流，确保所有字节写入磁盘
    encryptor.flush().map_err(|e| e.to_string())?;

    Ok(())
}

#[command]
pub async fn extract_archive(
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    let key = get_key(password);

    let file = File::open(&archive_path).map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(file);

    // 验证魔法头
    let mut header = [0u8; 9];
    if reader.read_exact(&mut header).is_err() || &header != MAGIC_HEADER {
        return Err("文件损坏或格式不正确：无法识别的 Krate 包".to_string());
    }

    // 构建管道: File -> BufReader -> Decrypt -> Gzip -> Tar
    let decryptor = DecryptReader::new(reader, key);
    let decompressor = GzDecoder::new(decryptor);
    let mut archive = tar::Archive::new(decompressor);

    // 解压配置
    archive.set_preserve_permissions(true);
    archive.set_unpack_xattrs(false);

    // 执行解压
    archive.unpack(&output_dir).map_err(|e| e.to_string())?;

    Ok(())
}
