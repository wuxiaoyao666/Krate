use aead::{
    Payload,
    generic_array::GenericArray,
    stream::{DecryptorBE32, EncryptorBE32},
};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;
use tauri::command;

// 文件头布局:
// 1. MAGIC_HEADER: 标识这是 Krate 归档
// 2. FORMAT_MARKER: 标识当前归档格式版本
// 3. flags/compression/可选加密参数
// 4. payload: gzip(tar(...)) 或 encrypted(gzip(tar(...)))
const MAGIC_HEADER: &[u8; 9] = b"KRATE_PKG";
const FORMAT_MARKER: &[u8; 4] = b"V002";

const FLAG_ENCRYPTED: u8 = 0b0000_0001;
const COMPRESSION_GZIP: u8 = 1;

// 默认压缩级别选 6，是压缩率和速度之间更均衡的取值。
const DEFAULT_GZIP_LEVEL: u32 = 6;
// Argon2 参数默认偏保守，目标是明显提升抗暴力破解能力，
// 同时避免桌面端打包/解包时出现过于夸张的等待。
const DEFAULT_ARGON2_MEMORY_KIB: u32 = 64 * 1024;
const DEFAULT_ARGON2_ITERATIONS: u32 = 2;
const MAX_ARGON2_LANES: u32 = 4;

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
// STREAM-BE32 会从 XChaCha20 的 24 字节 nonce 中占用 5 字节做计数器和 last 标记，
// 因此我们只需要持久化剩余的 19 字节“基础 nonce”。
const STREAM_NONCE_LEN: usize = 19;
// 分块大小决定了性能和额外开销:
// - 块越大，认证 tag 和分块头占比越低
// - 块越小，流式处理更细，但开销更高
// 这里取 256 KiB，足够大，且不会让内存占用失控。
const CHUNK_PLAINTEXT_SIZE: usize = 256 * 1024;
const CHUNK_LENGTH_BYTES: usize = 4;
const AEAD_TAG_LEN: usize = 16;
const CHUNK_FLAG_NEXT: u8 = 0;
const CHUNK_FLAG_LAST: u8 = 1;

type ArchiveCipher = XChaCha20Poly1305;
type ArchiveEncryptor = EncryptorBE32<ArchiveCipher>;
type ArchiveDecryptor = DecryptorBE32<ArchiveCipher>;

// 加密参数直接写入归档头，保证解包时可以独立恢复密钥派生上下文。
// 其中 salt 和 stream_nonce 每个归档随机生成一次。
#[derive(Clone, Debug)]
struct EncryptionMetadata {
    memory_kib: u32,
    iterations: u32,
    lanes: u32,
    salt: [u8; SALT_LEN],
    stream_nonce: [u8; STREAM_NONCE_LEN],
}

#[derive(Clone, Debug)]
struct ArchiveHeader {
    flags: u8,
    compression: u8,
    encryption: Option<EncryptionMetadata>,
}

impl ArchiveHeader {
    // 无密码时只做 tar + gzip，不启用加密。
    fn new_plain() -> Self {
        Self {
            flags: 0,
            compression: COMPRESSION_GZIP,
            encryption: None,
        }
    }

    // 有密码时，归档头中写入随机 salt 和基础 nonce。
    // 真正的对称密钥不落盘，只能由密码重新派生。
    fn new_encrypted() -> Result<Self, String> {
        Ok(Self {
            flags: FLAG_ENCRYPTED,
            compression: COMPRESSION_GZIP,
            encryption: Some(EncryptionMetadata {
                memory_kib: DEFAULT_ARGON2_MEMORY_KIB,
                iterations: DEFAULT_ARGON2_ITERATIONS,
                lanes: default_argon2_lanes(),
                salt: random_bytes()?,
                stream_nonce: random_bytes()?,
            }),
        })
    }

    // 归档头的序列化结果同时承担两件事:
    // 1. 告诉解包端如何处理 payload
    // 2. 作为 AEAD 的 AAD，绑定头部和密文，防止头部被静默篡改
    fn encoded_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            FORMAT_MARKER.len()
                + 2
                + self
                    .encryption
                    .as_ref()
                    .map(|_| 12 + SALT_LEN + STREAM_NONCE_LEN)
                    .unwrap_or(0),
        );
        bytes.extend_from_slice(FORMAT_MARKER);
        bytes.push(self.flags);
        bytes.push(self.compression);

        if let Some(meta) = &self.encryption {
            bytes.extend_from_slice(&meta.memory_kib.to_le_bytes());
            bytes.extend_from_slice(&meta.iterations.to_le_bytes());
            bytes.extend_from_slice(&meta.lanes.to_le_bytes());
            bytes.extend_from_slice(&meta.salt);
            bytes.extend_from_slice(&meta.stream_nonce);
        }

        bytes
    }

    // AAD 使用 “magic + header bytes”，这样只要头部任何字段被改动，
    // 后续解密认证就会失败。
    fn aad_bytes(&self) -> Vec<u8> {
        let mut aad = Vec::with_capacity(MAGIC_HEADER.len() + self.encoded_bytes().len());
        aad.extend_from_slice(MAGIC_HEADER);
        aad.extend_from_slice(&self.encoded_bytes());
        aad
    }
}

// gzip 输出的是连续字节流，这个 writer 负责把它切成固定大小分块，
// 再逐块喂给 STREAM AEAD 做认证加密。
struct EncryptedPayloadWriter<W: Write> {
    inner: W,
    encryptor: Option<ArchiveEncryptor>,
    aad: Vec<u8>,
    buffer: Vec<u8>,
    finished: bool,
}

impl<W: Write> EncryptedPayloadWriter<W> {
    fn new(inner: W, key_bytes: [u8; KEY_LEN], nonce_bytes: [u8; STREAM_NONCE_LEN], aad: Vec<u8>) -> Self {
        let key = GenericArray::clone_from_slice(&key_bytes);
        let nonce = GenericArray::clone_from_slice(&nonce_bytes);
        let cipher = ArchiveCipher::new(&key);

        Self {
            inner,
            encryptor: Some(ArchiveEncryptor::from_aead(cipher, &nonce)),
            aad,
            buffer: Vec::with_capacity(CHUNK_PLAINTEXT_SIZE),
            finished: false,
        }
    }

    // 每个块的落盘格式:
    // [1 byte flag][4 bytes ciphertext_len][ciphertext_with_tag]
    //
    // flag 区分普通块和最后一块，避免解包端无法识别流尾。
    fn write_chunk(&mut self, is_last: bool, plaintext: &[u8]) -> io::Result<()> {
        let payload = Payload {
            msg: plaintext,
            aad: &self.aad,
        };

        let ciphertext = if is_last {
            self.encryptor
                .take()
                .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "archive writer already finalized"))?
                .encrypt_last(payload)
        } else {
            self.encryptor
                .as_mut()
                .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "archive writer already finalized"))?
                .encrypt_next(payload)
        }
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "archive encryption failed"))?;

        let chunk_len = u32::try_from(ciphertext.len()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "encrypted chunk is too large to fit in the archive format",
            )
        })?;

        self.inner.write_all(&[if is_last { CHUNK_FLAG_LAST } else { CHUNK_FLAG_NEXT }])?;
        self.inner.write_all(&chunk_len.to_le_bytes())?;
        self.inner.write_all(&ciphertext)?;
        Ok(())
    }

    // 将满块 buffer 取出并重建一个新 buffer，避免重复分配大块内存。
    fn take_buffer(&mut self) -> Vec<u8> {
        std::mem::replace(&mut self.buffer, Vec::with_capacity(CHUNK_PLAINTEXT_SIZE))
    }

    // 流结束时必须显式写出最后一块，否则 STREAM 无法生成 final block 标记。
    fn finish(mut self) -> io::Result<W> {
        if !self.finished {
            self.finished = true;
            let final_chunk = self.take_buffer();
            self.write_chunk(true, &final_chunk)?;
            self.inner.flush()?;
        }
        Ok(self.inner)
    }
}

impl<W: Write> Write for EncryptedPayloadWriter<W> {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        if self.finished {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "cannot write to a finalized archive writer",
            ));
        }

        let original_len = buf.len();

        // 先尝试填满上一次残留的半块。
        if !self.buffer.is_empty() {
            let needed = CHUNK_PLAINTEXT_SIZE - self.buffer.len();
            let take = needed.min(buf.len());
            self.buffer.extend_from_slice(&buf[..take]);
            buf = &buf[take..];

            if self.buffer.len() == CHUNK_PLAINTEXT_SIZE {
                let chunk = self.take_buffer();
                self.write_chunk(false, &chunk)?;
            }
        }

        // 对后续完整块直接加密写出，避免不必要的中间复制。
        while buf.len() >= CHUNK_PLAINTEXT_SIZE {
            let (chunk, rest) = buf.split_at(CHUNK_PLAINTEXT_SIZE);
            self.write_chunk(false, chunk)?;
            buf = rest;
        }

        // 剩余不足一块的内容暂存，等待后续 write 或 finish。
        if !buf.is_empty() {
            self.buffer.extend_from_slice(buf);
        }

        Ok(original_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

// 解包时与 EncryptedPayloadWriter 反向对称:
// 从磁盘读出块头和密文，逐块认证解密，再按 Read 接口连续吐出明文。
struct EncryptedPayloadReader<R: Read> {
    inner: R,
    decryptor: Option<ArchiveDecryptor>,
    aad: Vec<u8>,
    buffer: Vec<u8>,
    offset: usize,
    finished: bool,
}

impl<R: Read> EncryptedPayloadReader<R> {
    fn new(inner: R, key_bytes: [u8; KEY_LEN], nonce_bytes: [u8; STREAM_NONCE_LEN], aad: Vec<u8>) -> Self {
        let key = GenericArray::clone_from_slice(&key_bytes);
        let nonce = GenericArray::clone_from_slice(&nonce_bytes);
        let cipher = ArchiveCipher::new(&key);

        Self {
            inner,
            decryptor: Some(ArchiveDecryptor::from_aead(cipher, &nonce)),
            aad,
            buffer: Vec::new(),
            offset: 0,
            finished: false,
        }
    }

    // 一次加载一个完整密文块并认证解密。
    // 任何密码错误、头部篡改、密文改动，都会在这里失败。
    fn load_next_chunk(&mut self) -> io::Result<()> {
        let mut flag = [0u8; 1];
        self.inner.read_exact(&mut flag).map_err(|err| {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                io::Error::new(io::ErrorKind::InvalidData, "encrypted archive ended unexpectedly")
            } else {
                err
            }
        })?;

        let mut len_bytes = [0u8; CHUNK_LENGTH_BYTES];
        self.inner.read_exact(&mut len_bytes)?;
        let chunk_len = u32::from_le_bytes(len_bytes) as usize;

        if chunk_len < AEAD_TAG_LEN || chunk_len > CHUNK_PLAINTEXT_SIZE + AEAD_TAG_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "encrypted archive chunk length is invalid",
            ));
        }

        let mut ciphertext = vec![0u8; chunk_len];
        self.inner.read_exact(&mut ciphertext)?;

        let payload = Payload {
            msg: ciphertext.as_slice(),
            aad: &self.aad,
        };

        self.buffer = match flag[0] {
            CHUNK_FLAG_NEXT => self
                .decryptor
                .as_mut()
                .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "archive reader already finalized"))?
                .decrypt_next(payload),
            CHUNK_FLAG_LAST => {
                self.finished = true;
                self.decryptor
                    .take()
                    .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "archive reader already finalized"))?
                    .decrypt_last(payload)
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "encrypted archive chunk flag is invalid",
                ))
            }
        }
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "归档解密失败，密码错误或文件已损坏",
            )
        })?;

        self.offset = 0;
        Ok(())
    }

    // 解包前先探测第一块，尽早把“密码错误/文件损坏”暴露出来，
    // 避免错误被 tar/gzip 包装成不直观的上层报错。
    fn prime(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() && !self.finished {
            self.load_next_chunk()?;
        }
        Ok(())
    }
}

impl<R: Read> Read for EncryptedPayloadReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }

        // 当前明文块已经消费完时，再去加载下一块。
        if self.offset >= self.buffer.len() {
            if self.finished {
                return Ok(0);
            }

            self.load_next_chunk()?;
            if self.offset >= self.buffer.len() && self.finished {
                return Ok(0);
            }
        }

        let remaining = &self.buffer[self.offset..];
        let count = remaining.len().min(out.len());
        out[..count].copy_from_slice(&remaining[..count]);
        self.offset += count;
        Ok(count)
    }
}

// 前端传空串时统一视为“没有密码”，避免不同层重复判断空白字符。
fn normalized_password(password: Option<String>) -> Option<String> {
    password.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

// Argon2 并行度最多跟 CPU 走，但限制在 4 以内，避免桌面端开销过高。
fn default_argon2_lanes() -> u32 {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() as u32)
        .unwrap_or(1)
        .clamp(1, MAX_ARGON2_LANES)
}

// 所有随机数据都来自系统 RNG，用于 salt / nonce，不能复用或硬编码。
fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|err| format!("生成归档随机参数失败: {}", err))?;
    Ok(bytes)
}

// 使用 Argon2id 从用户密码派生对称密钥。
// 这样密码本身不直接参与 AEAD，且能显著提高离线暴力破解成本。
fn derive_archive_key(password: &str, metadata: &EncryptionMetadata) -> Result<[u8; KEY_LEN], String> {
    let params = Params::new(
        metadata.memory_kib,
        metadata.iterations,
        metadata.lanes,
        Some(KEY_LEN),
    )
    .map_err(|err| format!("归档加密参数无效: {}", err))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; KEY_LEN];

    argon2
        .hash_password_into(password.as_bytes(), &metadata.salt, &mut key)
        .map_err(|err| format!("归档密钥派生失败: {}", err))?;

    Ok(key)
}

// 将输入文件/目录追加到 tar 中。
// 当前仍保留原有行为: 目录递归打包，单文件按文件名写入。
fn append_inputs_to_tar<W: Write>(tar: &mut tar::Builder<W>, inputs: Vec<String>) -> Result<(), String> {
    for path_str in inputs {
        let path = Path::new(&path_str);
        let name = path.file_name().ok_or("无效的文件路径")?;

        if path.is_dir() {
            tar.append_dir_all(name, path).map_err(|err| err.to_string())?;
        } else {
            let mut file = File::open(path).map_err(|err| err.to_string())?;
            tar.append_file(name, &mut file).map_err(|err| err.to_string())?;
        }
    }

    Ok(())
}

// 写入 Krate 头部，并返回对应的 AAD。
fn write_archive_header<W: Write>(writer: &mut W, header: &ArchiveHeader) -> Result<Vec<u8>, String> {
    writer.write_all(MAGIC_HEADER).map_err(|err| err.to_string())?;

    let encoded_header = header.encoded_bytes();
    writer
        .write_all(&encoded_header)
        .map_err(|err| err.to_string())?;

    Ok(header.aad_bytes())
}

// 读取 V002 格式的头部参数。旧格式不走这里。
fn read_archive_header<R: Read>(reader: &mut R) -> Result<ArchiveHeader, String> {
    let mut flags = [0u8; 1];
    reader.read_exact(&mut flags).map_err(|err| err.to_string())?;

    let mut compression = [0u8; 1];
    reader
        .read_exact(&mut compression)
        .map_err(|err| err.to_string())?;

    if compression[0] != COMPRESSION_GZIP {
        return Err("不支持的 .krate 压缩格式".to_string());
    }

    let encryption = if flags[0] & FLAG_ENCRYPTED != 0 {
        let mut memory_kib = [0u8; 4];
        let mut iterations = [0u8; 4];
        let mut lanes = [0u8; 4];
        let mut salt = [0u8; SALT_LEN];
        let mut stream_nonce = [0u8; STREAM_NONCE_LEN];

        reader
            .read_exact(&mut memory_kib)
            .and_then(|_| reader.read_exact(&mut iterations))
            .and_then(|_| reader.read_exact(&mut lanes))
            .and_then(|_| reader.read_exact(&mut salt))
            .and_then(|_| reader.read_exact(&mut stream_nonce))
            .map_err(|err| err.to_string())?;

        Some(EncryptionMetadata {
            memory_kib: u32::from_le_bytes(memory_kib),
            iterations: u32::from_le_bytes(iterations),
            lanes: u32::from_le_bytes(lanes),
            salt,
            stream_nonce,
        })
    } else {
        None
    };

    Ok(ArchiveHeader {
        flags: flags[0],
        compression: compression[0],
        encryption,
    })
}

// tar 解包只关心拿到一个明文字节流，至于明文来自“直接 gzip”还是“先解密再 gunzip”，
// 上层已经处理完毕，这里统一收口。
fn extract_archive_contents<R: Read>(reader: R, output_dir: &str) -> Result<(), String> {
    let decompressor = GzDecoder::new(reader);
    let mut archive = tar::Archive::new(decompressor);
    archive.set_preserve_permissions(true);
    archive.set_unpack_xattrs(false);
    archive.unpack(output_dir).map_err(|err| err.to_string())
}

#[command]
pub async fn create_archive(
    inputs: Vec<String>,
    output_path: String,
    password: Option<String>,
    gzip_level: Option<u32>,
) -> Result<(), String> {
    let normalized_password = normalized_password(password);
    let header = if normalized_password.is_some() {
        ArchiveHeader::new_encrypted()?
    } else {
        ArchiveHeader::new_plain()
    };
    let level = gzip_level.unwrap_or(DEFAULT_GZIP_LEVEL);

    let file = File::create(&output_path).map_err(|err| err.to_string())?;
    let mut writer = BufWriter::new(file);
    let aad = write_archive_header(&mut writer, &header)?;

    // 有密码:
    //   tar -> gzip -> encrypted chunk writer -> file
    // 无密码:
    //   tar -> gzip -> file
    if let (Some(password), Some(metadata)) = (normalized_password.as_deref(), header.encryption.as_ref()) {
        let key = derive_archive_key(password, metadata)?;
        let payload_writer = EncryptedPayloadWriter::new(writer, key, metadata.stream_nonce, aad);
        let compressor = GzEncoder::new(payload_writer, Compression::new(level));
        let mut tar = tar::Builder::new(compressor);

        append_inputs_to_tar(&mut tar, inputs)?;

        let compressor = tar
            .into_inner()
            .map_err(|err| format!("Tar finish failed: {}", err))?;
        let payload_writer = compressor
            .finish()
            .map_err(|err| format!("Gzip finish failed: {}", err))?;
        let mut writer = payload_writer.finish().map_err(|err| err.to_string())?;
        writer.flush().map_err(|err| err.to_string())?;
        return Ok(());
    }

    let compressor = GzEncoder::new(writer, Compression::new(level));
    let mut tar = tar::Builder::new(compressor);
    append_inputs_to_tar(&mut tar, inputs)?;

    let compressor = tar
        .into_inner()
        .map_err(|err| format!("Tar finish failed: {}", err))?;
    let mut writer = compressor
        .finish()
        .map_err(|err| format!("Gzip finish failed: {}", err))?;
    writer.flush().map_err(|err| err.to_string())?;

    Ok(())
}

#[command]
pub async fn extract_archive(
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    let normalized_password = normalized_password(password);

    let file = File::open(&archive_path).map_err(|err| err.to_string())?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; MAGIC_HEADER.len()];
    if reader.read_exact(&mut magic).is_err() || magic != *MAGIC_HEADER {
        return Err("文件损坏或格式不正确：无法识别的 Krate 包".to_string());
    }

    let mut marker = [0u8; FORMAT_MARKER.len()];
    reader.read_exact(&mut marker).map_err(|err| err.to_string())?;

    if marker != *FORMAT_MARKER {
        return Err("不支持的 .krate 版本，请使用当前版本重新生成归档".to_string());
    }

    let header = read_archive_header(&mut reader)?;

    // V002 有密码时必须先完成首块认证，再进入 tar/gzip 解包流程。
    if let Some(metadata) = header.encryption.as_ref() {
        let password = normalized_password
            .as_deref()
            .ok_or("该 .krate 归档已加密，请输入密码后再解压".to_string())?;
        let key = derive_archive_key(password, metadata)?;
        let mut payload_reader =
            EncryptedPayloadReader::new(reader, key, metadata.stream_nonce, header.aad_bytes());
        payload_reader.prime().map_err(|err| err.to_string())?;
        return extract_archive_contents(payload_reader, &output_dir);
    }

    extract_archive_contents(reader, &output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_case_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("krate-archive-{name}-{}-{nanos}", std::process::id()));
        path
    }

    fn write_text_file(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    #[tokio::test]
    async fn plain_archive_roundtrip_preserves_contents() {
        let root = temp_case_dir("plain");
        let input_file = root.join("input").join("notes.txt");
        let archive_file = root.join("plain.krate");
        let output_dir = root.join("output");
        let expected = "plain archive roundtrip";

        write_text_file(&input_file, expected);

        create_archive(
            vec![input_file.to_string_lossy().to_string()],
            archive_file.to_string_lossy().to_string(),
            None,
            Some(1),
        )
        .await
        .unwrap();

        extract_archive(
            archive_file.to_string_lossy().to_string(),
            output_dir.to_string_lossy().to_string(),
            None,
        )
        .await
        .unwrap();

        let extracted = fs::read_to_string(output_dir.join("notes.txt")).unwrap();
        assert_eq!(extracted, expected);

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn encrypted_archive_roundtrip_requires_password() {
        let root = temp_case_dir("encrypted");
        let input_file = root.join("input").join("secret.txt");
        let archive_file = root.join("secret.krate");
        let output_dir = root.join("output");
        let password = "correct horse battery staple";
        let expected = "encrypted archive roundtrip";

        write_text_file(&input_file, expected);

        create_archive(
            vec![input_file.to_string_lossy().to_string()],
            archive_file.to_string_lossy().to_string(),
            Some(password.to_string()),
            Some(1),
        )
        .await
        .unwrap();

        extract_archive(
            archive_file.to_string_lossy().to_string(),
            output_dir.to_string_lossy().to_string(),
            Some(password.to_string()),
        )
        .await
        .unwrap();

        let extracted = fs::read_to_string(output_dir.join("secret.txt")).unwrap();
        assert_eq!(extracted, expected);

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn encrypted_archive_rejects_wrong_password() {
        let root = temp_case_dir("wrong-password");
        let input_file = root.join("input").join("secret.txt");
        let archive_file = root.join("secret.krate");
        let output_dir = root.join("output");

        write_text_file(&input_file, "do not leak");

        create_archive(
            vec![input_file.to_string_lossy().to_string()],
            archive_file.to_string_lossy().to_string(),
            Some("right-password".to_string()),
            Some(1),
        )
        .await
        .unwrap();

        let error = extract_archive(
            archive_file.to_string_lossy().to_string(),
            output_dir.to_string_lossy().to_string(),
            Some("wrong-password".to_string()),
        )
        .await
        .unwrap_err();

        assert!(error.contains("解密失败"));

        let _ = fs::remove_dir_all(root);
    }
}
