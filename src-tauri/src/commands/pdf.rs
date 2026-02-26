use lopdf::{
    encryption::crypt_filters::{Aes128CryptFilter, CryptFilter},
    Document, EncryptionState, EncryptionVersion, Permissions,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::command;

fn build_encryption_state(document: &Document, password: &str) -> Result<EncryptionState, String> {
    let crypt_filter: Arc<dyn CryptFilter> = Arc::new(Aes128CryptFilter);
    let permissions =
        Permissions::all() & !(Permissions::COPYABLE | Permissions::COPYABLE_FOR_ACCESSIBILITY);

    let version = EncryptionVersion::V4 {
        document,
        encrypt_metadata: true,
        crypt_filters: BTreeMap::from([(b"StdCF".to_vec(), crypt_filter)]),
        stream_filter: b"StdCF".to_vec(),
        string_filter: b"StdCF".to_vec(),
        owner_password: password,
        user_password: password,
        permissions,
    };

    EncryptionState::try_from(version).map_err(|err| format!("构建加密配置失败: {}", err))
}

#[command]
pub fn encrypt_pdf(input: String, output: String, password: String) -> Result<(), String> {
    if password.trim().is_empty() {
        return Err("密码不能为空".to_string());
    }

    let mut document = Document::load(&input).map_err(|err| format!("读取 PDF 失败: {}", err))?;
    if document.is_encrypted() {
        return Err("PDF 已加密，请先解密后再加密".to_string());
    }

    let state = build_encryption_state(&document, &password)?;
    document
        .encrypt(&state)
        .map_err(|err| format!("加密失败: {}", err))?;

    document
        .save(&output)
        .map(|_| ())
        .map_err(|err| format!("保存 PDF 失败: {}", err))
}

#[command]
pub fn decrypt_pdf(input: String, output: String, password: String) -> Result<(), String> {
    if password.trim().is_empty() {
        return Err("密码不能为空".to_string());
    }

    let mut document =
        Document::load_with_password(&input, &password).map_err(|err| format!("读取或解密 PDF 失败: {}", err))?;

    // load_with_password 会在读取阶段直接完成解密:
    // - was_encrypted=false: 原文件本就未加密
    // - is_encrypted=true: 仍处于加密态，视为解密未完成
    if !document.was_encrypted() {
        return Err("该 PDF 未加密".to_string());
    }
    if document.is_encrypted() {
        return Err("解密未完成，请检查密码或文件加密方式".to_string());
    }

    document
        .save(&output)
        .map(|_| ())
        .map_err(|err| format!("保存 PDF 失败: {}", err))
}
