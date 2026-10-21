//! Provider 管理命令：CRUD、测试连接、获取模型列表。
//! API Key 只进系统凭据管理器，永远不返回给前端。

use crate::database::{
    keyring, new_id, now_iso, ProviderConfig, ProviderInput, ProviderRow, ProviderTestResult,
    ProviderView,
};
use crate::errors::{AppError, AppResult};
use crate::providers::{self, custom};
use crate::AppState;
use tauri::State;

fn view_of(row: ProviderRow) -> ProviderView {
    let has_key = keyring::get_key(&row.id)
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false);
    ProviderView { row, has_key: has_key }
}

fn row_from_input(id: &str, input: &ProviderInput) -> ProviderRow {
    ProviderRow {
        id: id.to_string(),
        name: input.name.trim().to_string(),
        kind: input.kind.trim().to_string(),
        base_url: input.base_url.trim().to_string(),
        model: input.model.trim().to_string(),
        image_model: input.image_model.trim().to_string(),
        request_format: input.request_format.trim().to_lowercase(),
        headers: input
            .headers
            .iter()
            .filter(|h| !h.key.trim().is_empty())
            .map(|h| (h.key.trim().to_string(), h.value.clone()))
            .collect(),
        is_default: input.is_default,
        created_at: now_iso(),
        updated_at: now_iso(),
    }
}

/// 从表单输入合成运行时配置；api_key 为空且已有 id 时回退到凭据管理器
fn config_from_input(input: &ProviderInput) -> ProviderConfig {
    let id = input.id.clone().unwrap_or_else(new_id);
    let api_key = if input.api_key.trim().is_empty() {
        keyring::get_key(&id).unwrap_or_default()
    } else {
        input.api_key.clone()
    };
    let row = row_from_input(&id, input);
    let mut cfg = providers::row_to_config(&row);
    cfg.api_key = api_key;
    cfg
}

fn validate_input(input: &ProviderInput) -> AppResult<()> {
    if input.name.trim().is_empty() {
        return Err(AppError::user("Provider 名称不能为空。"));
    }
    if !matches!(input.kind.as_str(), "openai" | "gemini" | "custom") {
        return Err(AppError::user("Provider 类型不合法。"));
    }
    if input.kind == "custom" {
        let cfg = config_from_input(input);
        custom::validate_custom(&cfg)?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_providers(state: State<AppState>) -> AppResult<Vec<ProviderView>> {
    Ok(state
        .db
        .list_providers()?
        .into_iter()
        .map(view_of)
        .collect())
}

#[tauri::command]
pub fn save_provider(state: State<AppState>, input: ProviderInput) -> AppResult<ProviderView> {
    validate_input(&input)?;
    let id = input.id.clone().unwrap_or_else(new_id);
    let mut row = row_from_input(&id, &input);
    if input.id.is_some() {
        // 保留原始创建时间
        if let Ok(existing) = state.db.get_provider_row(&id) {
            row.created_at = existing.created_at;
        }
    }
    if !input.api_key.trim().is_empty() {
        keyring::set_key(&id, input.api_key.trim())?;
    }
    state.db.upsert_provider(&row)?;
    Ok(view_of(row))
}

#[tauri::command]
pub fn delete_provider(state: State<AppState>, provider_id: String) -> AppResult<()> {
    keyring::delete_key(&provider_id);
    state.db.delete_provider(&provider_id)
}

#[tauri::command]
pub async fn test_provider(input: ProviderInput) -> AppResult<ProviderTestResult> {
    let cfg: ProviderConfig = config_from_input(&input);
    if cfg.api_key.trim().is_empty() {
        return Err(AppError::user("请先填写 API Key 再测试连接。"));
    }
    Ok(providers::test_provider(cfg).await)
}

#[tauri::command]
pub async fn list_provider_models(
    state: State<'_, AppState>,
    provider_id: String,
) -> AppResult<Vec<String>> {
    let cfg = providers::load_provider_config(&state.db, &provider_id)?;
    if cfg.api_key.trim().is_empty() {
        return Err(AppError::user(format!(
            "Provider「{}」尚未配置 API Key。",
            cfg.name
        )));
    }
    let provider = providers::create_provider(cfg);
    provider.list_models().await
}
