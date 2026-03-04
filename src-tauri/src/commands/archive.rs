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
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tauri::{Emitter, Window, command};

const MAGIC_HEADER: &[u8; 9] = b"KRATE_PKG";
const FORMAT_MARKER: &[u8; 4] = b"V002";

const FLAG_ENCRYPTED: u8 = 0b0000_0001;
const COMPRESSION_GZIP: u8 = 1;

const DEFAULT_GZIP_LEVEL: u32 = 6;
const DEFAULT_ARGON2_MEMORY_KIB: u32 = 64 * 1024;
const DEFAULT_ARGON2_ITERATIONS: u32 = 2;
const MAX_ARGON2_LANES: u32 = 4;

const KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const STREAM_NONCE_LEN: usize = 19;
const CHUNK_PLAINTEXT_SIZE: usize = 256 * 1024;
const CHUNK_LENGTH_BYTES: usize = 4;
const AEAD_TAG_LEN: usize = 16;
const CHUNK_FLAG_NEXT: u8 = 0;
const CHUNK_FLAG_LAST: u8 = 1;

const ARCHIVE_PROGRESS_EVENT: &str = "archive://progress";

type ArchiveCipher = XChaCha20Poly1305;
type ArchiveEncryptor = EncryptorBE32<ArchiveCipher>;
type ArchiveDecryptor = DecryptorBE32<ArchiveCipher>;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveProgressPayload {
    operation: String,
    stage: String,
    message: String,
    progress: f64,
    current_path: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
struct InputStats {
    total_bytes: u64,
    total_files: u64,
}

struct ArchiveProgressTracker {
    operation: &'static str,
    stage: &'static str,
    total_bytes: u64,
    processed_bytes: u64,
    last_emitted_progress: i16,
    current_path: Option<String>,
}

struct ProgressReader<'a, R: Read> {
    inner: R,
    tracker: &'a mut ArchiveProgressTracker,
    window: Option<&'a Window>,
    message: &'static str,
}

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

impl ArchiveProgressTracker {
    fn new(operation: &'static str, stage: &'static str, total_bytes: u64) -> Self {
        Self {
            operation,
            stage,
            total_bytes,
            processed_bytes: 0,
            last_emitted_progress: -1,
            current_path: None,
        }
    }

    fn set_stage(&mut self, window: Option<&Window>, stage: &'static str, message: &'static str) {
        self.stage = stage;
        self.emit(window, message, true);
    }

    fn set_current_path(
        &mut self,
        window: Option<&Window>,
        current_path: Option<String>,
        message: &'static str,
    ) {
        self.current_path = current_path;
        self.emit(window, message, true);
    }

    fn advance_bytes(&mut self, window: Option<&Window>, bytes: u64, message: &'static str) {
        self.processed_bytes = self.processed_bytes.saturating_add(bytes);
        if self.total_bytes > 0 {
            self.processed_bytes = self.processed_bytes.min(self.total_bytes);
        }
        self.emit(window, message, false);
    }

    fn finish(&mut self, window: Option<&Window>, stage: &'static str, message: &'static str) {
        self.stage = stage;
        self.processed_bytes = self.total_bytes;
        self.current_path = None;
        self.emit(window, message, true);
    }

    fn emit(&mut self, window: Option<&Window>, message: &'static str, force: bool) {
        let progress = if self.total_bytes == 0 {
            if force { 100.0 } else { 0.0 }
        } else {
            (self.processed_bytes as f64 / self.total_bytes as f64 * 100.0).clamp(0.0, 100.0)
        };
        let rounded = progress.floor() as i16;

        if !force && rounded == self.last_emitted_progress {
            return;
        }

        self.last_emitted_progress = rounded;
        emit_archive_progress(
            window,
            ArchiveProgressPayload {
                operation: self.operation.to_string(),
                stage: self.stage.to_string(),
                message: message.to_string(),
                progress,
                current_path: self.current_path.clone(),
            },
        );
    }
}

impl<'a, R: Read> ProgressReader<'a, R> {
    fn new(
        inner: R,
        tracker: &'a mut ArchiveProgressTracker,
        window: Option<&'a Window>,
        message: &'static str,
    ) -> Self {
        Self {
            inner,
            tracker,
            window,
            message,
        }
    }
}

impl<R: Read> Read for ProgressReader<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        if read > 0 {
            self.tracker
                .advance_bytes(self.window, read as u64, self.message);
        }
        Ok(read)
    }
}

impl ArchiveHeader {
    fn new_plain() -> Self {
        Self {
            flags: 0,
            compression: COMPRESSION_GZIP,
            encryption: None,
        }
    }

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

    // 头部既负责描述归档格式，也作为 AEAD 的 AAD，
    // 这样一旦有人篡改加密参数或压缩标记，认证阶段会直接失败。
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

    fn aad_bytes(&self) -> Vec<u8> {
        let mut aad = Vec::with_capacity(MAGIC_HEADER.len() + self.encoded_bytes().len());
        aad.extend_from_slice(MAGIC_HEADER);
        aad.extend_from_slice(&self.encoded_bytes());
        aad
    }
}

// STREAM AEAD 基于流式分块工作：gzip 连续明文流写入这里后，
// 会被切成固定大小的块并逐块认证加密，避免整包驻留内存。
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

    fn take_buffer(&mut self) -> Vec<u8> {
        std::mem::replace(&mut self.buffer, Vec::with_capacity(CHUNK_PLAINTEXT_SIZE))
    }

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

        while buf.len() >= CHUNK_PLAINTEXT_SIZE {
            let (chunk, rest) = buf.split_at(CHUNK_PLAINTEXT_SIZE);
            self.write_chunk(false, chunk)?;
            buf = rest;
        }

        if !buf.is_empty() {
            self.buffer.extend_from_slice(buf);
        }

        Ok(original_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

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
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "归档解密失败，密码错误或文件已损坏"))?;

        self.offset = 0;
        Ok(())
    }

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

fn emit_archive_progress(window: Option<&Window>, payload: ArchiveProgressPayload) {
    if let Some(window) = window {
        let _ = window.emit(ARCHIVE_PROGRESS_EVENT, payload);
    }
}

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

fn default_argon2_lanes() -> u32 {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get() as u32)
        .unwrap_or(1)
        .clamp(1, MAX_ARGON2_LANES)
}

fn random_bytes<const N: usize>() -> Result<[u8; N], String> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|err| format!("生成归档随机参数失败: {}", err))?;
    Ok(bytes)
}

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

fn collect_input_stats(inputs: &[String]) -> Result<InputStats, String> {
    let mut stats = InputStats::default();

    for path_str in inputs {
        collect_path_stats(Path::new(path_str), &mut stats)?;
    }

    Ok(stats)
}

fn collect_path_stats(path: &Path, stats: &mut InputStats) -> Result<(), String> {
    let metadata = fs::metadata(path).map_err(|err| err.to_string())?;

    if metadata.is_file() {
        stats.total_bytes = stats.total_bytes.saturating_add(metadata.len());
        stats.total_files = stats.total_files.saturating_add(1);
        return Ok(());
    }

    if metadata.is_dir() {
        for child in sorted_children(path)? {
            collect_path_stats(&child, stats)?;
        }
        return Ok(());
    }

    Err(format!("不支持归档的路径类型: {}", path.display()))
}

fn sorted_children(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut children = fs::read_dir(path)
        .map_err(|err| err.to_string())?
        .map(|entry| entry.map(|child| child.path()).map_err(|err| err.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    children.sort();
    Ok(children)
}

// 打包进度按已读取的源文件字节数推进，同时在切换文件时发出当前路径，
// 这样前端能展示“正在处理哪个文件”，而不是只有百分比。
fn append_inputs_to_tar<W: Write>(
    tar: &mut tar::Builder<W>,
    inputs: &[String],
    tracker: &mut ArchiveProgressTracker,
    window: Option<&Window>,
    progress_message: &'static str,
) -> Result<(), String> {
    for path_str in inputs {
        let path = Path::new(path_str);
        let archive_root = path
            .file_name()
            .map(PathBuf::from)
            .ok_or_else(|| format!("无效的归档输入路径: {}", path.display()))?;
        append_path_to_tar(tar, path, &archive_root, tracker, window, progress_message)?;
    }

    Ok(())
}

fn append_path_to_tar<W: Write>(
    tar: &mut tar::Builder<W>,
    source_path: &Path,
    archive_path: &Path,
    tracker: &mut ArchiveProgressTracker,
    window: Option<&Window>,
    progress_message: &'static str,
) -> Result<(), String> {
    let metadata = fs::metadata(source_path).map_err(|err| err.to_string())?;

    if metadata.is_dir() {
        tar.append_dir(archive_path, source_path)
            .map_err(|err| err.to_string())?;

        for child in sorted_children(source_path)? {
            let child_name = child
                .file_name()
                .map(PathBuf::from)
                .ok_or_else(|| format!("无效的归档路径: {}", child.display()))?;
            append_path_to_tar(
                tar,
                &child,
                &archive_path.join(child_name),
                tracker,
                window,
                progress_message,
            )?;
        }

        return Ok(());
    }

    if metadata.is_file() {
        tracker.set_current_path(
            window,
            Some(source_path.display().to_string()),
            progress_message,
        );

        let mut header = tar::Header::new_gnu();
        header.set_metadata(&metadata);
        header.set_cksum();

        let file = File::open(source_path).map_err(|err| err.to_string())?;
        let mut reader = ProgressReader::new(BufReader::new(file), tracker, window, progress_message);
        tar.append_data(&mut header, archive_path, &mut reader)
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    Err(format!("不支持归档的路径类型: {}", source_path.display()))
}

fn write_archive_header<W: Write>(writer: &mut W, header: &ArchiveHeader) -> Result<Vec<u8>, String> {
    writer.write_all(MAGIC_HEADER).map_err(|err| err.to_string())?;
    let encoded_header = header.encoded_bytes();
    writer
        .write_all(&encoded_header)
        .map_err(|err| err.to_string())?;
    Ok(header.aad_bytes())
}

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

fn extract_archive_contents<R: Read>(reader: R, output_dir: &str) -> Result<(), String> {
    let decompressor = GzDecoder::new(reader);
    let mut archive = tar::Archive::new(decompressor);
    archive.set_preserve_permissions(true);
    archive.set_unpack_xattrs(false);
    archive.unpack(output_dir).map_err(|err| err.to_string())
}

async fn create_archive_impl(
    window: Option<&Window>,
    inputs: Vec<String>,
    output_path: String,
    password: Option<String>,
    gzip_level: Option<u32>,
) -> Result<(), String> {
    if inputs.is_empty() {
        return Err("请至少选择一个文件或文件夹".to_string());
    }

    let stats = collect_input_stats(&inputs)?;
    let mut tracker = ArchiveProgressTracker::new("pack", "准备归档", stats.total_bytes);
    tracker.set_stage(window, "准备归档", "正在准备归档");

    let normalized_password = normalized_password(password);
    let header = if normalized_password.is_some() {
        ArchiveHeader::new_encrypted()?
    } else {
        ArchiveHeader::new_plain()
    };
    let level = gzip_level.unwrap_or(DEFAULT_GZIP_LEVEL);
    let progress_message = if normalized_password.is_some() {
        "正在压缩并加密"
    } else {
        "正在压缩打包"
    };

    let file = File::create(&output_path).map_err(|err| err.to_string())?;
    let mut writer = BufWriter::new(file);
    let aad = write_archive_header(&mut writer, &header)?;

    tracker.set_stage(window, progress_message, progress_message);

    if let (Some(password), Some(metadata)) = (normalized_password.as_deref(), header.encryption.as_ref()) {
        let key = derive_archive_key(password, metadata)?;
        let payload_writer = EncryptedPayloadWriter::new(writer, key, metadata.stream_nonce, aad);
        let compressor = GzEncoder::new(payload_writer, Compression::new(level));
        let mut tar = tar::Builder::new(compressor);

        append_inputs_to_tar(&mut tar, &inputs, &mut tracker, window, progress_message)?;

        let compressor = tar
            .into_inner()
            .map_err(|err| format!("Tar finish failed: {}", err))?;
        let payload_writer = compressor
            .finish()
            .map_err(|err| format!("Gzip finish failed: {}", err))?;
        let mut writer = payload_writer.finish().map_err(|err| err.to_string())?;
        writer.flush().map_err(|err| err.to_string())?;
        tracker.finish(window, "归档完成", "归档完成");
        return Ok(());
    }

    let compressor = GzEncoder::new(writer, Compression::new(level));
    let mut tar = tar::Builder::new(compressor);

    append_inputs_to_tar(&mut tar, &inputs, &mut tracker, window, progress_message)?;

    let compressor = tar
        .into_inner()
        .map_err(|err| format!("Tar finish failed: {}", err))?;
    let mut writer = compressor
        .finish()
        .map_err(|err| format!("Gzip finish failed: {}", err))?;
    writer.flush().map_err(|err| err.to_string())?;

    tracker.finish(window, "归档完成", "归档完成");
    Ok(())
}

async fn extract_archive_impl(
    window: Option<&Window>,
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    let normalized_password = normalized_password(password);
    let total_bytes = fs::metadata(&archive_path)
        .map_err(|err| err.to_string())?
        .len();
    let mut tracker = ArchiveProgressTracker::new("extract", "读取归档头", total_bytes);
    tracker.set_stage(window, "读取归档头", "正在读取归档头");

    let file = File::open(&archive_path).map_err(|err| err.to_string())?;
    let buffered = BufReader::new(file);
    let mut progress_reader = ProgressReader::new(buffered, &mut tracker, window, "正在读取归档头");

    let mut magic = [0u8; MAGIC_HEADER.len()];
    if progress_reader.read_exact(&mut magic).is_err() || magic != *MAGIC_HEADER {
        return Err("文件损坏或格式不正确：无法识别的 Krate 包".to_string());
    }

    let mut marker = [0u8; FORMAT_MARKER.len()];
    progress_reader
        .read_exact(&mut marker)
        .map_err(|err| err.to_string())?;

    if marker != *FORMAT_MARKER {
        return Err("不支持的 .krate 版本，请使用当前版本重新生成归档".to_string());
    }

    let header = read_archive_header(&mut progress_reader)?;

    if let Some(metadata) = header.encryption.as_ref() {
        let password = normalized_password
            .as_deref()
            .ok_or("该 .krate 归档已加密，请输入密码后再解压".to_string())?;
        let key = derive_archive_key(password, metadata)?;
        progress_reader.message = "正在校验密码并解压";
        progress_reader
            .tracker
            .set_stage(progress_reader.window, "正在校验密码并解压", "正在校验密码并解压");

        let mut payload_reader = EncryptedPayloadReader::new(
            progress_reader,
            key,
            metadata.stream_nonce,
            header.aad_bytes(),
        );
        payload_reader.prime().map_err(|err| err.to_string())?;
        extract_archive_contents(payload_reader, &output_dir)?;
    } else {
        progress_reader.message = "正在解压归档";
        progress_reader
            .tracker
            .set_stage(progress_reader.window, "正在解压归档", "正在解压归档");
        extract_archive_contents(progress_reader, &output_dir)?;
    }

    tracker.finish(window, "解压完成", "解压完成");
    Ok(())
}

#[command]
pub async fn create_archive(
    window: Window,
    inputs: Vec<String>,
    output_path: String,
    password: Option<String>,
    gzip_level: Option<u32>,
) -> Result<(), String> {
    create_archive_impl(Some(&window), inputs, output_path, password, gzip_level).await
}

#[command]
pub async fn extract_archive(
    window: Window,
    archive_path: String,
    output_dir: String,
    password: Option<String>,
) -> Result<(), String> {
    extract_archive_impl(Some(&window), archive_path, output_dir, password).await
}

#[cfg(test)]
mod tests {
    use super::*;
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

        create_archive_impl(
            None,
            vec![input_file.to_string_lossy().to_string()],
            archive_file.to_string_lossy().to_string(),
            None,
            Some(1),
        )
        .await
        .unwrap();

        extract_archive_impl(
            None,
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

        create_archive_impl(
            None,
            vec![input_file.to_string_lossy().to_string()],
            archive_file.to_string_lossy().to_string(),
            Some(password.to_string()),
            Some(1),
        )
        .await
        .unwrap();

        extract_archive_impl(
            None,
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

        create_archive_impl(
            None,
            vec![input_file.to_string_lossy().to_string()],
            archive_file.to_string_lossy().to_string(),
            Some("right-password".to_string()),
            Some(1),
        )
        .await
        .unwrap();

        let error = extract_archive_impl(
            None,
            archive_file.to_string_lossy().to_string(),
            output_dir.to_string_lossy().to_string(),
            Some("wrong-password".to_string()),
        )
        .await
        .unwrap_err();

        assert!(error.contains("归档解密失败"));

        let _ = fs::remove_dir_all(root);
    }
}
