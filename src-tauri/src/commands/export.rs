//! 导出命令

use crate::database::AssetDetail;
use crate::errors::{AppError, AppResult};
use crate::export::{exporter_by_engine, ExportResult};
use crate::AppState;
use tauri::State;

/// 导出资产当前版本（engine: generic / godot / unity）
#[tauri::command]
pub fn export_asset(
    state: State<AppState>,
    asset_id: String,
    dest_dir: String,
    engine: String,
) -> AppResult<ExportResult> {
    let detail: AssetDetail = state.db.get_asset_detail(&asset_id)?;
    let current = detail
        .versions
        .iter()
        .find(|v| v.version == detail.asset.current_version)
        .ok_or_else(|| AppError::user("资产当前版本数据异常。"))?;

    let dest = std::path::Path::new(&dest_dir);
    let stem = crate::filesystem::sanitize_name(&format!(
        "{}_v{}",
        detail.asset.name, detail.asset.current_version
    ));

    let exporter = exporter_by_engine(&engine);
    exporter.export(&crate::export::ExportContext {
        asset: &detail.asset,
        version: current,
        dest_dir: dest,
        file_stem: &stem,
    })
}
