//! OpenAI（及一切 OpenAI Images 兼容接口）适配器。
//! 支持 dall-e-2/3 与 gpt-image-1；返回 b64_json 或 url 均可处理。

use super::{
    decode_b64, ensure_key, http_client, AIProvider,
};
use crate::database::ProviderConfig;
use crate::errors::{AppError, AppResult};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde_json::json;

pub struct OpenAIProvider {
    cfg: ProviderConfig,
}

impl OpenAIProvider {
    pub fn new(cfg: ProviderConfig) -> Self {
        OpenAIProvider { cfg }
    }

    fn base(&self) -> String {
        let b = self.cfg.base_url.trim().trim_end_matches('/');
        if b.is_empty() {
            super::DEFAULT_OPENAI_BASE.to_string()
        } else {
            b.to_string()
        }
    }

    fn image_model(&self) -> String {
        let m = self.cfg.image_model.trim();
        if m.is_empty() {
            "gpt-image-1".to_string()
        } else {
            m.to_string()
        }
    }

    fn headers(&self) -> AppResult<HeaderMap> {
        let mut map = HeaderMap::new();
        let key = self.cfg.api_key.trim();
        map.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {key}"))
                .map_err(|_| AppError::user("API Key 含有非法字符，无法构造请求头。"))?,
        );
        for (k, v) in &self.cfg.headers {
            let name = reqwest::header::HeaderName::from_bytes(k.as_bytes())
                .map_err(|_| AppError::user(format!("自定义 Header 名称非法：{k}")))?;
            let value = HeaderValue::from_str(v)
                .map_err(|_| AppError::user(format!("自定义 Header 值非法：{k}")))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    /// 从 OpenAI Images 响应中提取图片字节（b64_json 优先，url 兜底下载）
    async fn extract_image(&self, resp: serde_json::Value) -> AppResult<Vec<u8>> {
        let data = resp
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .ok_or_else(|| {
                let msg = resp
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("响应中没有 data 字段");
                AppError::user(format!("AI 服务返回异常：{msg}"))
            })?;

        if let Some(b64) = data.get("b64_json").and_then(|b| b.as_str()) {
            return decode_b64(b64);
        }
        if let Some(url) = data.get("url").and_then(|u| u.as_str()) {
            let client = http_client()?;
            let img = client
                .get(url)
                .send()
                .await
                .map_err(|e| AppError::user(format!("下载生成图片失败：{e}")))?;
            if !img.status().is_success() {
                return Err(AppError::from_http_status(
                    img.status().as_u16(),
                    "图片下载失败",
                ));
            }
            return Ok(img.bytes().await?.to_vec());
        }
        Err(AppError::user("AI 服务未返回图片数据（既无 b64_json 也无 url）。"))
    }

    async fn post_json(&self, url: &str, body: serde_json::Value) -> AppResult<serde_json::Value> {
        let client = http_client()?;
        let resp = client
            .post(url)
            .headers(self.headers()?)
            .json(&body)
            .send()
            .await
            .map_err(|e| map_request_error(e))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::from_http_status(status.as_u16(), &text));
        }
        serde_json::from_str(&text)
            .map_err(|e| AppError::user(format!("AI 服务响应解析失败：{e}")))
    }
}

fn map_request_error(e: reqwest::Error) -> AppError {
    if e.is_timeout() {
        AppError::user("请求超时：AI 服务响应时间过长，请稍后重试。")
    } else if e.is_connect() {
        AppError::user("网络连接失败：请检查网络与 Base URL 是否正确。")
    } else {
        AppError::user(format!("网络请求失败：{e}"))
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn adapter_name(&self) -> &'static str {
        "OpenAIProvider"
    }

    fn config(&self) -> &ProviderConfig {
        &self.cfg
    }

    async fn generate_image(&self, prompt: &str, size: &str) -> AppResult<Vec<u8>> {
        ensure_key(&self.cfg)?;
        let model = self.image_model();
        let mut body = json!({
            "model": model,
            "prompt": prompt,
            "size": size,
            "n": 1,
        });
        // gpt-image-1 不接受 response_format 参数（恒为 b64_json）
        if model.starts_with("dall-e") {
            body["response_format"] = json!("b64_json");
        }
        let url = format!("{}/images/generations", self.base());
        let resp = self.post_json(&url, body).await?;
        self.extract_image(resp).await
    }

    async fn edit_image(&self, prompt: &str, reference_png: &[u8], size: &str) -> AppResult<Vec<u8>> {
        ensure_key(&self.cfg)?;
        let client = http_client()?;
        let form = reqwest::multipart::Form::new()
            .text("model", self.image_model())
            .text("prompt", prompt.to_string())
            .text("n", "1")
            .text("size", size.to_string())
            .part(
                "image",
                reqwest::multipart::Part::bytes(reference_png.to_vec())
                    .file_name("reference.png")
                    .mime_str("image/png")?,
            );
        let resp = client
            .post(format!("{}/images/edits", self.base()))
            .headers(self.headers()?)
            .multipart(form)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::from_http_status(status.as_u16(), &text));
        }
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AppError::user(format!("AI 服务响应解析失败：{e}")))?;
        self.extract_image(parsed).await
    }

    async fn health_check(&self) -> AppResult<()> {
        ensure_key(&self.cfg)?;
        let client = http_client()?;
        let resp = client
            .get(format!("{}/models", self.base()))
            .headers(self.headers()?)
            .send()
            .await
            .map_err(map_request_error)?;
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
            .get(format!("{}/models", self.base()))
            .headers(self.headers()?)
            .send()
            .await
            .map_err(map_request_error)?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(AppError::from_http_status(status.as_u16(), &text));
        }
        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| AppError::user(format!("模型列表解析失败：{e}")))?;
        let mut models: Vec<String> = v
            .get("data")
            .and_then(|d| d.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|i| i.as_str()).map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        models.sort();
        Ok(models)
    }
}
