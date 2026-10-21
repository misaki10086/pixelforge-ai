//! API Key 安全存储：使用操作系统凭据管理器（Windows Credential Manager）。
//! Key 绝不写入 SQLite、不写入前端、不写入任何配置文件。

use crate::errors::{AppError, AppResult};

const SERVICE: &str = "PixelForge AI";

fn entry(provider_id: &str) -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, provider_id)
        .map_err(|e| AppError::Keyring(format!("无法创建凭据条目：{e}")))
}

pub fn set_key(provider_id: &str, key: &str) -> AppResult<()> {
    entry(provider_id)?
        .set_password(key)
        .map_err(|e| AppError::Keyring(format!("保存 API Key 失败：{e}")))
}

pub fn get_key(provider_id: &str) -> Option<String> {
    entry(provider_id).ok()?.get_password().ok()
}

pub fn delete_key(provider_id: &str) {
    if let Ok(e) = entry(provider_id) {
        let _ = e.delete_credential();
    }
}
