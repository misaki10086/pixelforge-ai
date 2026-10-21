//! 资产管理命令

use crate::database::{now_iso, Asset, AssetDetail, DashboardStats};
use crate::errors::{AppError, AppResult};
use crate::filesystem;
use crate::imageproc;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_assets(state: State<AppState>, project_id: String) -> AppResult<Vec<Asset>> {
    state.db.list_assets(&project_id)
}

#[tauri::command]
pub fn get_asset_detail(state: State<AppState>, asset_id: String) -> AppResult<AssetDetail> {
    state.db.get_asset_detail(&asset_id)
}

#[tauri::command]
pub fn update_asset(
    state: State<AppState>,
    asset_id: String,
    name: String,
    tags: Vec<String>,
) -> AppResult<AssetDetail> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::user("资产名称不能为空。"));
    }
    state.db.update_asset_meta(&asset_id, &name, &tags)?;
    state.db.get_asset_detail(&asset_id)
}

#[tauri::command]
pub fn delete_asset(state: State<AppState>, asset_id: String) -> AppResult<()> {
    // 先收集磁盘文件路径，再删除记录（删除后无法再查询）
    let asset = state.db.get_asset(&asset_id)?;
    let mut to_delete = state.db.list_asset_files(&asset_id).unwrap_or_default();
    to_delete.push(asset.image_path.clone());
    if let Some(t) = &asset.thumbnail_path {
        to_delete.push(t.clone());
    }
    state.db.delete_asset(&asset_id)?;
    for f in to_delete {
        let p = std::path::Path::new(&f);
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
    }
    Ok(())
}

/// 导入本地图片为资产（拖拽 / 选择文件）
#[tauri::command]
pub fn import_asset(
    state: State<AppState>,
    project_id: String,
    source_path: String,
    asset_type: String,
) -> AppResult<Asset> {
    let project = state.db.get_project(&project_id)?;
    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(AppError::user(format!("文件不存在：{source_path}")));
    }
    let ext_ok = matches!(
        src.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp")
    );
    if !ext_ok {
        return Err(AppError::user("仅支持导入 PNG / JPG / GIF / BMP / WebP 图片。"));
    }

    let img = image::open(src)
        .map_err(|e| AppError::user(format!("无法解析图片文件：{e}")))?;
    let rgba = img.to_rgba8();

    let asset_id = crate::database::new_id();
    let dir = std::path::Path::new(&project.path).join(filesystem::folder_for_asset_type(&asset_type));
    filesystem::ensure_dir_exists(&dir)?;
    let image_path = dir.join(format!("{asset_id}.png"));
    rgba.save(&image_path).map_err(|e| AppError::user(format!("复制图片失败：{e}")))?;

    let thumb_dir = std::path::Path::new(&project.path).join("thumbnails");
    filesystem::ensure_dir_exists(&thumb_dir)?;
    let thumb_path = thumb_dir.join(format!("{asset_id}.png"));
    imageproc::make_thumbnail(&rgba, 256)
        .save(&thumb_path)
        .map_err(|e| AppError::user(format!("保存缩略图失败：{e}")))?;

    let name = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("imported")
        .to_string();

    let id = state.db.insert_asset_with_version(&crate::database::NewAsset {
        project_id,
        name,
        asset_type,
        image_path: image_path.to_string_lossy().to_string(),
        thumbnail_path: Some(thumb_path.to_string_lossy().to_string()),
        width: rgba.width() as i64,
        height: rgba.height() as i64,
        provider_id: None,
        model: None,
        prompt: String::new(),
        version_note: "本地导入".into(),
    })?;
    state.db.get_asset(&id)
}

/// 版本还原：复制历史版本文件为新版本（vN+1），不覆盖任何历史
#[tauri::command]
pub fn restore_asset_version(state: State<AppState>, version_id: String) -> AppResult<AssetDetail> {
    let version = state.db.get_asset_version(&version_id)?;

    let src = std::path::Path::new(&version.image_path);
    if !src.exists() {
        return Err(AppError::user("版本图片文件已丢失，无法还原。"));
    }
    let new_file_id = crate::database::new_id();
    let dest = src.with_file_name(format!("{new_file_id}.png"));
    std::fs::copy(src, &dest).map_err(|e| AppError::user(format!("复制版本文件失败：{e}")))?;

    let thumbnail_path = match &version.thumbnail_path {
        Some(tp) if std::path::Path::new(tp).exists() => {
            let thumb_dest = src.with_file_name(format!("{new_file_id}-thumb.png"));
            std::fs::copy(tp, &thumb_dest).ok();
            Some(thumb_dest.to_string_lossy().to_string())
        }
        _ => None,
    };

    let restored = crate::database::AssetVersion {
        id: String::new(),
        asset_id: version.asset_id.clone(),
        version: 0, // restore_version 内部自动递增
        image_path: dest.to_string_lossy().to_string(),
        thumbnail_path,
        width: version.width,
        height: version.height,
        prompt: version.prompt.clone(),
        provider_id: version.provider_id.clone(),
        model: version.model.clone(),
        note: format!("还原自 v{}", version.version),
        created_at: now_iso(),
    };
    state.db.restore_version(&restored, &restored.note)
}

#[tauri::command]
pub fn delete_asset_version(state: State<AppState>, version_id: String) -> AppResult<AssetDetail> {
    let version = state.db.get_asset_version(&version_id)?;
    let detail = state.db.delete_asset_version(&version_id)?;
    // 该版本文件不再被任何版本引用时删除磁盘文件
    let still_referenced = detail
        .versions
        .iter()
        .any(|v| v.image_path == version.image_path)
        || detail.asset.image_path == version.image_path;
    if !still_referenced {
        let _ = std::fs::remove_file(&version.image_path);
    }
    Ok(detail)
}

#[tauri::command]
pub fn get_dashboard_stats(
    state: State<AppState>,
    project_id: Option<String>,
) -> AppResult<DashboardStats> {
    state.db.dashboard_stats(project_id.as_deref())
}
