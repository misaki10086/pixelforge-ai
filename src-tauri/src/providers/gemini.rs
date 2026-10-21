//! Google Gemini 适配器（generativelanguage REST API）。
//! 图像生成：`{model}:generateContent` + responseModalities IMAGE，
//! 兼容 gemini-2.0-flash-exp-image-generation / gemini-2.5-flash-image 等模型。

use super::{decode_b64, ensure_key, http_client, AIProvider};
use crate::database::ProviderConfig;
use crate::errors::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::header::HeaderMap;
use serde_json::json;

pub struct GeminiProvider {
    cfg: ProviderConfig,
}

impl GeminiProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        GeminiProvider { cfg }
    }

    fn base(&self) -> String {
        let b = self.cfg.base_url.trim().trim_end_matches('/');
        if b.is_empty() {
            super::DEFAULT_GEMINI_BASE.to_string()
        } else {
            b.to_string()
        }
    }

    fn image_model(&self) -> String {
        let m = self.cfg.image_model.trim();
        if m.is_empty() {
            "gemini-2.5-flash-image".to_string()
        } else {
            m.to_string()
        }
    }

    fn headers(&self) -> AppResult<HeaderMap> {
        let mut map = HeaderMap::new();
        let key = self.cfg.api_key.trim();
        map.insert(
            "x-goog-api-key",
            reqwest::header::HeaderValue::from_str(key)
                .map_err(|_| AppError::user("API Key 含有非法字符，无法构造请求头。"))?,
        );
        for (k, v) in &self.cfg.headers {
            let name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|_| AppError::user(format!("自定义 Header 名称非法：{k}")))?;
            let value = reqwest::header::HeaderValue::from_str(v)
                .map_err(|_| AppError::user(format!("自定义 Header 值非法：{k}")))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    /// 从 generateContent 响应提取图片（inlineData / inline_data 均兼容）
    fn extract_image(resp: &serde_json::Value) -> AppResult<Vec<u8>> {
        let parts = resp
            .pointer("/candidates/0/content/parts")
            .and_then(|p| p.as_array())
            .ok_or_else(|| {
                let msg = resp
                    .pointer("/error/message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("响应中没有 candidates 字段");
                AppError::user(format!("AI 服务返回异常：{msg}"))
            })?;

        for part in parts {
            let inline = part.get("inlineData").or_else(|| part.get("inline_data"));
            if let Some(inline) = inline {
                if let Some(data) = inline.get("data").and_then(|d| d.as_str()) {
                    return decode_b64(data);
                }
            }
        }
        Err(AppError::user(
            "AI 服务未返回图片数据：请确认所选模型支持图像生成（如 gemini-2.5-flash-image）。",
        ))
    }

    async fn generate_content(&self, parts: serde_json::Value) -> AppResult<Vec<u8>> {
        ensure_key(&self.cfg)?;
        let model = self.image_model();
        let body = json!({
            "contents": [{ "role": "user", "parts": parts }],
            "generationConfig": { "responseModalities": ["TEXT", "IMAGE"] }
        });
        let client = http_client()?;
        let url = format!("{}/models/{model}:generateContent", self.base());
        let resp = client
            .post(url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    AppError::user("请求超时：AI 服务响应时间过长，请稍后重试。")
                } else if e.is_connect() {
                    AppError::user("网络连接失败：请检查网络与 Base URL 是否正确。")
                } else {
                    AppError::user(format!("网络请求失败：{e}"))
                }
            })?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::from_http_status(status.as_u16(), &text));
        }
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AppError::user(format!("AI 服务响应解析失败：{e}")))?;
        Self::extract_image(&parsed)
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn adapter_name(&self) -> &'static str {
        "GeminiProvider"
    }

    fn config(&self) -> &ProviderConfig {
        &self.cfg
    }

    async fn generate_image(&self, prompt: &str, _size: &str) -> AppResult<Vec<u8>> {
        self.generate_content(json!([{ "text": prompt }])).await
    }

    async fn edit_image(&self, prompt: &str, reference_png: &[u8], _size: &str) -> AppResult<Vec<u8>> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(reference_png);
        self.generate_content(json!([
            { "text": prompt },
            { "inline_data": { "mime_type": "image/png", "data": b64 } }
        ]))
        .await
    }

    async fn health_check(&self) -> AppResult<()> {
        ensure_key(&self.cfg)?;
        let client = http_client()?;
        let resp = client
            .get(format!("{}/models", self.base()))
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|e| AppError::user(format!("网络请求失败：{e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::from_http_status(status.as_u16(), &text));
        }
        Ok(())
    }

    async fn list_models(&self) -> AppResult<Vec<String>> {
        ensure_key(&self.cfg)?;
        let client = http_client()?;
        let resp = client
            .get(format!("{}/models?pageSize=200", self.base()))
            .headers(self.headers()?)
            .send()
            .await
            .map_err(|e| AppError::user(format!("网络请求失败：{e}")))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::from_http_status(status.as_u16(), &text));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AppError::user(format!("模型列表解析失败：{e}")))?;
        let mut models: Vec<String> = v
            .get("models")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|m| {
                        let methods = m
                            .get("supportedGenerationMethods")
                            .and_then(|s| s.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.as_str())
                                    .any(|x| x == "generateContent")
                            })
                            .unwrap_or(false);
                        methods
                    })
                    .filter_map(|m| m.get("name").and_then(|n| n.as_str()))
                    .map(|n| n.trim_start_matches("models/").to_string())
                    .collect()
            })
            .unwrap_or_default();
        models.sort();
        Ok(models)
    }
}
