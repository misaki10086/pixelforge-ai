//! 统一错误类型：所有面向用户的错误消息均使用中文，
//! 禁止把异常堆栈直接抛给前端。

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 已翻译好的、可直接展示给用户的错误
    #[error("{0}")]
    User(String),
    #[error("文件读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("数据库操作失败：{0}")]
    Db(#[from] rusqlite::Error),
    #[error("图像处理失败：{0}")]
    Image(String),
    #[error("凭据存取失败：{0}")]
    Keyring(String),
    #[error("网络请求失败：{0}")]
    Http(#[from] reqwest::Error),
    #[error("数据序列化失败：{0}")]
    Json(#[from] serde_json::Error),
}

/// 手动实现 Serialize：Tauri 命令错误只回传 message 字符串
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn user(msg: impl Into<String>) -> Self {
        AppError::User(msg.into())
    }

    /// 将 HTTP 状态码翻译为用户可理解的中文错误
    pub fn from_http_status(status: u16, body: &str) -> Self {
        let snippet: String = body.chars().take(400).collect();
        let msg = match status {
            400 | 422 => format!("请求参数被 AI 服务拒绝（{status}）：{snippet}"),
            401 | 403 => "API Key 无效或没有访问权限，请在「设置 → Providers」中检查 API Key。".to_string(),
            404 => "接口不存在（404）：请检查 Base URL 与模型名称是否正确。".to_string(),
            408 => "AI 服务响应超时，请稍后重试。".to_string(),
            429 => "请求频率超过限制（Rate Limit），请稍后再试，或在服务商控制台提升配额。".to_string(),
            500..=599 => format!("AI 服务暂时不可用（{status}），请稍后重试。{snippet}"),
            _ => format!("请求失败（HTTP {status}）：{snippet}"),
        };
        AppError::user(msg)
    }
}
