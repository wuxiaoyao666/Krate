use std::fs::{metadata, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use serde::Serialize;
use tauri::command;

const MIN_CHUNK_BYTES: usize = 8 * 1024;
const MAX_CHUNK_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_CHUNK_BYTES: usize = 512 * 1024;
const MIN_TAIL_BYTES: usize = 16 * 1024;
const MAX_TAIL_BYTES: usize = 8 * 1024 * 1024;
const SAMPLE_BYTES: usize = 4096;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    path: String,
    size: u64,
    is_binary: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogChunk {
    start_offset: u64,
    end_offset: u64,
    eof: bool,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSearchMatch {
    offset: u64,
    preview: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSearchResponse {
    matches: Vec<LogSearchMatch>,
    next_offset: u64,
    done: bool,
}

fn to_err(context: &str, err: impl std::fmt::Display) -> String {
    format!("{context}: {err}")
}

fn truncate_with_ellipsis(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}...")
    } else {
        head
    }
}

fn clamp_chunk_bytes(bytes: Option<usize>) -> usize {
    bytes
        .unwrap_or(DEFAULT_CHUNK_BYTES)
        .clamp(MIN_CHUNK_BYTES, MAX_CHUNK_BYTES)
}

fn detect_binary_file(path: &str, size: u64) -> Result<bool, String> {
    if size == 0 {
        return Ok(false);
    }

    let mut file = File::open(path).map_err(|e| to_err("打开文件失败", e))?;

    let mut positions = vec![0_u64];
    if size > SAMPLE_BYTES as u64 {
        positions.push(size / 2);
        positions.push(size.saturating_sub(SAMPLE_BYTES as u64));
    }
    positions.sort_unstable();
    positions.dedup();

    let mut buf = vec![0_u8; SAMPLE_BYTES];
    for pos in positions {
        file.seek(SeekFrom::Start(pos))
            .map_err(|e| to_err("读取文件失败", e))?;
        let read = file.read(&mut buf).map_err(|e| to_err("读取文件失败", e))?;
        if read == 0 {
            continue;
        }

        let sample = &buf[..read];
        let nul_count = sample.iter().filter(|&&b| b == 0).count();
        if nul_count > 0 {
            return Ok(true);
        }

        let control_count = sample
            .iter()
            .filter(|&&b| (b < 0x09) || (b > 0x0D && b < 0x20))
            .count();
        let ratio = control_count as f64 / read as f64;
        if ratio > 0.3 {
            return Ok(true);
        }
    }

    Ok(false)
}

fn read_log_chunk_inner(path: &str, offset: u64, max_bytes: usize) -> Result<LogChunk, String> {
    let meta = metadata(path).map_err(|e| to_err("读取文件信息失败", e))?;
    if !meta.is_file() {
        return Err("目标不是文件".to_string());
    }

    let size = meta.len();
    let start = offset.min(size);
    if start >= size {
        return Ok(LogChunk {
            start_offset: size,
            end_offset: size,
            eof: true,
            text: String::new(),
        });
    }

    let mut file = File::open(path).map_err(|e| to_err("打开文件失败", e))?;
    file.seek(SeekFrom::Start(start))
        .map_err(|e| to_err("定位文件失败", e))?;

    let mut buf = vec![0_u8; max_bytes];
    let read = file.read(&mut buf).map_err(|e| to_err("读取文件失败", e))?;
    buf.truncate(read);

    let end = start + read as u64;
    Ok(LogChunk {
        start_offset: start,
        end_offset: end,
        eof: end >= size,
        text: String::from_utf8_lossy(&buf).into_owned(),
    })
}

#[command]
pub fn inspect_log_file(path: String) -> Result<LogFileInfo, String> {
    let meta = metadata(&path).map_err(|e| to_err("读取文件信息失败", e))?;
    if !meta.is_file() {
        return Err("目标不是文件".to_string());
    }

    let size = meta.len();
    let is_binary = detect_binary_file(&path, size)?;
    Ok(LogFileInfo {
        path,
        size,
        is_binary,
    })
}

#[command]
pub fn read_log_chunk(path: String, offset: u64, max_bytes: Option<usize>) -> Result<LogChunk, String> {
    let chunk_bytes = clamp_chunk_bytes(max_bytes);
    read_log_chunk_inner(&path, offset, chunk_bytes)
}

#[command]
pub fn read_log_tail(path: String, window_bytes: Option<usize>) -> Result<LogChunk, String> {
    let meta = metadata(&path).map_err(|e| to_err("读取文件信息失败", e))?;
    if !meta.is_file() {
        return Err("目标不是文件".to_string());
    }

    let size = meta.len();
    let window = window_bytes
        .unwrap_or(DEFAULT_CHUNK_BYTES)
        .clamp(MIN_TAIL_BYTES, MAX_TAIL_BYTES) as u64;
    let start = size.saturating_sub(window);
    read_log_chunk_inner(&path, start, window as usize)
}

#[command]
pub fn search_log_in_file(
    path: String,
    keyword: String,
    start_offset: u64,
    limit: Option<usize>,
) -> Result<LogSearchResponse, String> {
    let term = keyword.trim();
    if term.is_empty() {
        return Err("搜索关键词不能为空".to_string());
    }
    let term_lower = term.to_lowercase();

    let meta = metadata(&path).map_err(|e| to_err("读取文件信息失败", e))?;
    if !meta.is_file() {
        return Err("目标不是文件".to_string());
    }

    let size = meta.len();
    let mut offset = start_offset.min(size);
    let max_matches = limit.unwrap_or(50).clamp(1, 200);

    let mut file = File::open(&path).map_err(|e| to_err("打开文件失败", e))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|e| to_err("定位文件失败", e))?;

    let mut reader = BufReader::with_capacity(64 * 1024, file);
    if offset > 0 {
        // 从中间偏移开始搜索时，先丢弃首行残缺部分，避免返回不完整预览。
        let mut skip_buf = Vec::new();
        let skipped = reader
            .read_until(b'\n', &mut skip_buf)
            .map_err(|e| to_err("搜索读取失败", e))?;
        offset += skipped as u64;
    }

    let mut matches = Vec::new();
    let mut line_buf = Vec::new();

    while matches.len() < max_matches {
        line_buf.clear();
        let bytes = reader
            .read_until(b'\n', &mut line_buf)
            .map_err(|e| to_err("搜索读取失败", e))?;
        if bytes == 0 {
            break;
        }

        let line_start = offset;
        offset += bytes as u64;

        let line = String::from_utf8_lossy(&line_buf);
        let normalized = line.trim_end_matches(['\r', '\n']);
        if normalized.to_lowercase().contains(&term_lower) {
            let preview = truncate_with_ellipsis(normalized, 240);
            matches.push(LogSearchMatch {
                offset: line_start,
                preview,
            });
        }
    }

    Ok(LogSearchResponse {
        matches,
        next_offset: offset,
        done: offset >= size,
    })
}
