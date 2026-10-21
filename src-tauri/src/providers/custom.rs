//! Custom Provider：用户自配 Base URL / Headers / 请求格式。
//! request_format = "openai" → 复用 OpenAI 适配器；"gemini" → 复用 Gemini 适配器。

use super::{openai::OpenAIProvider, gemini::GeminiProvider, AIProvider};
use crate::database::ProviderConfig;
use crate::errors::{AppError, AppResult};
use async_trait::async_trait;

pub enum InnerAdapter {
    OpenAi(OpenAIProvider),
    Gemini(GeminiProvider),
}

pub struct CustomProvider {
    inner: InnerAdapter,
}

impl CustomProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        let format = cfg.request_format.trim().to_lowercase();
        let inner = if format == "gemini" {
            InnerAdapter::Gemini(GeminiProvider::new(cfg))
        } else {
            InnerAdapter::OpenAi(OpenAIProvider::new(cfg))
        };
        CustomProvider { inner }
    }
}

#[async_trait]
impl AIProvider for CustomProvider {
    fn adapter_name(&self) -> &'static str {
        match &self.inner {
            InnerAdapter::OpenAi(p) => p.adapter_name(),
            InnerAdapter::Gemini(p) => p.adapter_name(),
        }
    }

    fn config(&self) -> &ProviderConfig {
        match &self.inner {
            InnerAdapter::OpenAi(p) => p.config(),
            InnerAdapter::Gemini(p) => p.config(),
        }
    }

    async fn generate_image(&self, prompt: &str, size: &str) -> AppResult<Vec<u8>> {
        match &self.inner {
            InnerAdapter::OpenAi(p) => p.generate_image(prompt, size).await,
            InnerAdapter::Gemini(p) => p.generate_image(prompt, size).await,
        }
    }

    async fn edit_image(&self, prompt: &str, reference_png: &[u8], size: &str) -> AppResult<Vec<u8>> {
        match &self.inner {
            InnerAdapter::OpenAi(p) => p.edit_image(prompt, reference_png, size).await,
            InnerAdapter::Gemini(p) => p.edit_image(prompt, reference_png, size).await,
        }
    }

    async fn health_check(&self) -> AppResult<()> {
        match &self.inner {
            InnerAdapter::OpenAi(p) => p.health_check().await,
            InnerAdapter::Gemini(p) => p.health_check().await,
        }
    }

    async fn list_models(&self) -> AppResult<Vec<String>> {
        match &self.inner {
            InnerAdapter::OpenAi(p) => p.list_models().await,
            InnerAdapter::Gemini(p) => p.list_models().await,
        }
    }
}

/// 校验自定义 Provider 必填字段
pub fn validate_custom(cfg: &ProviderConfig) -> AppResult<()> {
    if cfg.base_url.trim().is_empty() {
        return Err(AppError::user("Custom Provider 必须填写 Base URL。"));
    }
    let fmt = cfg.request_format.trim().to_lowercase();
    if fmt != "openai" && fmt != "gemini" {
        return Err(AppError::user(
            "请求格式仅支持 openai（OpenAI 兼容）或 gemini（Google 兼容）。",
        ));
    }
    Ok(())
}
