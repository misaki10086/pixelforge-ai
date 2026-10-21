//! AI Provider 抽象层。
//!
//! 架构约束：UI → Service(Tauri Command) → ProviderManager → Provider Adapter。
//! 任何 Provider 的 HTTP 细节都不允许泄漏到前端。

pub mod animation_logic;
pub mod custom;
pub mod gemini;
pub mod openai;
pub mod prompt;

use crate::database::{keyring, Db, ProviderConfig, ProviderRow, ProviderTestResult};
use crate::errors::{AppError, AppResult};
use async_trait::async_trait;

#[async_trait]
pub trait AIProvider: Send + Sync {
    /// 适配器名称（诊断与测试用）
    #[allow(dead_code)]
    fn adapter_name(&self) -> &'static str;
    /// 文生图，返回 PNG 字节
    async fn generate_image(&self, prompt: &str, size: &str) -> AppResult<Vec<u8>>;
    /// 图生图 / 局部编辑（支持参考图），返回 PNG 字节（第三阶段 AI Edit 使用）
    #[allow(dead_code)]
    async fn edit_image(&self, prompt: &str, reference_png: &[u8], size: &str) -> AppResult<Vec<u8>>;
    /// 连通性检查
    async fn health_check(&self) -> AppResult<()>;
    /// 列出可用模型
    async fn list_models(&self) -> AppResult<Vec<String>>;

    fn config(&self) -> &ProviderConfig;
    fn display_model(&self) -> String {
        self.config().image_model.clone()
    }
}

pub const DEFAULT_OPENAI_BASE: &str = "https://api.openai.com/v1";
pub const DEFAULT_GEMINI_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// 根据数据库行 + 凭据管理器构建可用的 Provider 实例
pub fn load_provider_config(db: &Db, provider_id: &str) -> AppResult<ProviderConfig> {
    let row = db.get_provider_row(provider_id)?;
    Ok(row_to_config(&row))
}

pub fn row_to_config(row: &ProviderRow) -> ProviderConfig {
    let api_key = keyring::get_key(&row.id).unwrap_or_default();
    ProviderConfig {
        id: row.id.clone(),
        name: row.name.clone(),
        kind: row.kind.clone(),
        base_url: row.base_url.clone(),
        model: row.model.clone(),
        image_model: row.image_model.clone(),
        request_format: row.request_format.clone(),
        headers: row.headers.iter().cloned().collect(),
        api_key,
    }
}

/// ProviderManager 工厂：按 kind 分发到对应适配器
pub fn create_provider(cfg: ProviderConfig) -> Box<dyn AIProvider> {
    match cfg.kind.as_str() {
        "gemini" => Box::new(gemini::GeminiProvider::new(cfg)),
        "custom" => Box::new(custom::CustomProvider::new(cfg)),
        _ => Box::new(openai::OpenAIProvider::new(cfg)),
    }
}

/// 公共 HTTP 客户端：图像生成可能耗时较长，整体超时放宽
pub fn http_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| AppError::user(format!("无法创建网络客户端：{e}")))
}

/// 测试连通性（供「测试连接」按钮使用）
pub async fn test_provider(cfg: ProviderConfig) -> ProviderTestResult {
    let started = std::time::Instant::now();
    let provider = create_provider(cfg);
    let result = provider.health_check().await;
    let latency = started.elapsed().as_millis();
    match result {
        Ok(()) => {
            let models = provider.list_models().await.unwrap_or_default();
            ProviderTestResult {
                ok: true,
                message: format!("连接成功（{} ms）", latency),
                latency_ms: latency,
                models,
            }
        }
        Err(e) => ProviderTestResult {
            ok: false,
            message: e.to_string(),
            latency_ms: latency,
            models: vec![],
        },
    }
}

/// 统一校验：必须有 API Key（除非 Gemini 显式留空走免费额度——这里统一要求填写）
pub fn ensure_key(cfg: &ProviderConfig) -> AppResult<()> {
    if cfg.api_key.trim().is_empty() {
        return Err(AppError::user(format!(
            "Provider「{}」尚未配置 API Key，请在「设置 → Providers」中填写。",
            cfg.name
        )));
    }
    Ok(())
}

/// 解码 base64 图片数据
pub fn decode_b64(data: &str) -> AppResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(data.trim())
        .map_err(|e| AppError::user(format!("AI 返回的图片数据解码失败：{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_routes_by_kind() {
        let mut cfg = ProviderConfig {
            id: "1".into(),
            name: "t".into(),
            kind: "openai".into(),
            base_url: DEFAULT_OPENAI_BASE.into(),
            model: "gpt-4o-mini".into(),
            image_model: "gpt-image-1".into(),
            request_format: "openai".into(),
            headers: std::collections::HashMap::new(),
            api_key: "sk-test".into(),
        };
        assert_eq!(create_provider(cfg.clone()).adapter_name(), "OpenAIProvider");

        cfg.kind = "gemini".into();
        assert_eq!(create_provider(cfg.clone()).adapter_name(), "GeminiProvider");

        cfg.kind = "custom".into();
        cfg.request_format = "gemini".into();
        assert_eq!(create_provider(cfg.clone()).adapter_name(), "GeminiProvider");
        cfg.request_format = "openai".into();
        assert_eq!(create_provider(cfg).adapter_name(), "OpenAIProvider");
    }

    #[tokio::test]
    async fn missing_key_fails_fast() {
        let cfg = ProviderConfig {
            id: "1".into(),
            name: "NoKey".into(),
            kind: "openai".into(),
            base_url: DEFAULT_OPENAI_BASE.into(),
            model: String::new(),
            image_model: "gpt-image-1".into(),
            request_format: "openai".into(),
            headers: std::collections::HashMap::new(),
            api_key: String::new(),
        };
        let provider = create_provider(cfg);
        let err = provider.generate_image("test", "1024x1024").await.unwrap_err();
        assert!(err.to_string().contains("API Key"));
    }
}
