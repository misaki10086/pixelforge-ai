//! 颜色 / 风格处理命令：调色板提取、调色板量化（应用为新版本）、Pixelize 处理。
//! 所有处理都会创建新的资产版本，不覆盖历史。

use crate::database::AssetDetail;
use crate::errors::{AppError, AppResult};
use crate::imageproc::{self, DitherMode};
use crate::AppState;
use tauri::State;

fn load_current_image(state: &AppState, asset_id: &str) -> AppResult<(AssetDetail, image::RgbaImage)> {
    let detail = state.db.get_asset_detail(asset_id)?;
    let current = detail
        .versions
        .iter()
        .find(|v| v.version == detail.asset.current_version)
        .ok_or_else(|| AppError::user("资产版本数据异常。"))?;
    let img = image::open(&current.image_path)
        .map_err(|e| AppError::user(format!("无法读取资产图片：{e}")))?
        .to_rgba8();
    Ok((detail, img))
}

/// 把处理后的图像保存为新版本文件（写入资产所在目录），返回新版本详情
fn save_as_new_version(
    state: &AppState,
    detail: &AssetDetail,
    processed: image::RgbaImage,
    note: &str,
) -> AppResult<AssetDetail> {
    let asset = &detail.asset;
    let src = std::path::Path::new(&asset.image_path);
    let dir = src.parent().ok_or_else(|| AppError::user("资产路径异常。"))?;

    let file_id = crate::database::new_id();
    let image_path = dir.join(format!("{file_id}.png"));
    processed
        .save(&image_path)
        .map_err(|e| AppError::user(format!("保存处理结果失败：{e}")))?;

    let thumb_dir = dir
        .parent()
        .map(|p| p.join("thumbnails"))
        .unwrap_or_else(|| dir.to_path_buf());
    std::fs::create_dir_all(&thumb_dir)?;
    let thumb_path = thumb_dir.join(format!("{file_id}.png"));
    imageproc::make_thumbnail(&processed, 256)
        .save(&thumb_path)
        .map_err(|e| AppError::user(format!("保存缩略图失败：{e}")))?;

    let restored = crate::database::AssetVersion {
        id: String::new(),
        asset_id: asset.id.clone(),
        version: 0, // restore_version 内部自动递增
        image_path: image_path.to_string_lossy().to_string(),
        thumbnail_path: Some(thumb_path.to_string_lossy().to_string()),
        width: processed.width() as i64,
        height: processed.height() as i64,
        prompt: asset.prompt.clone(),
        provider_id: asset.provider_id.clone(),
        model: asset.model.clone(),
        note: note.to_string(),
        created_at: crate::database::now_iso(),
    };
    state.db.restore_version(&restored, note)
}

fn parse_hex_color(s: &str) -> AppResult<[u8; 3]> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return Err(AppError::user(format!("颜色格式应为 #RRGGBB，收到「{s}」")));
    }
    let r = u8::from_str_radix(&s[0..2], 16).map_err(|_| AppError::user("颜色解析失败。"))?;
    let g = u8::from_str_radix(&s[2..4], 16).map_err(|_| AppError::user("颜色解析失败。"))?;
    let b = u8::from_str_radix(&s[4..6], 16).map_err(|_| AppError::user("颜色解析失败。"))?;
    Ok([r, g, b])
}

fn to_hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

/// 提取资产当前版本的主色调色板
#[tauri::command]
pub fn extract_asset_palette(
    state: State<AppState>,
    asset_id: String,
    count: u32,
) -> AppResult<Vec<String>> {
    let (_, img) = load_current_image(&state, &asset_id)?;
    let k = count.clamp(2, 64) as usize;
    Ok(imageproc::extract_palette(&img, k)
        .into_iter()
        .map(to_hex)
        .collect())
}

/// 用给定调色板量化资产，生成新版本（风格统一 / 调色）
#[tauri::command]
pub fn apply_asset_palette(
    state: State<AppState>,
    asset_id: String,
    colors: Vec<String>,
    dither: bool,
) -> AppResult<AssetDetail> {
    if colors.is_empty() {
        return Err(AppError::user("调色板为空。"));
    }
    let mut palette = Vec::with_capacity(colors.len());
    for c in &colors {
        palette.push(parse_hex_color(c)?);
    }
    let (detail, img) = load_current_image(&state, &asset_id)?;
    let processed = imageproc::quantize_to_palette(&img, &palette, if dither { DitherMode::Bayer4 } else { DitherMode::None });
    save_as_new_version(&state, &detail, processed, &format!("调色板量化（{} 色）", palette.len()))
}

/// Pixelize：分辨率归一化 + 调色板缩减 + 可选抖动，一步生成新版本
#[tauri::command]
pub fn process_asset_pixelize(
    state: State<AppState>,
    asset_id: String,
    resolution: u32,
    palette_count: u32,
    dither: bool,
) -> AppResult<AssetDetail> {
    let (detail, img) = load_current_image(&state, &asset_id)?;
    let mut processed = img;
    if resolution > 0 {
        processed = imageproc::resize_nearest(&processed, resolution);
    }
    if palette_count > 0 {
        let mode = if dither { DitherMode::Bayer4 } else { DitherMode::None };
        processed = imageproc::quantize_colors(&processed, palette_count.clamp(2, 64) as usize, mode);
    }
    if resolution == 0 && palette_count == 0 {
        return Err(AppError::user("请至少选择分辨率或调色板数量中的一项。"));
    }
    let note = format!(
        "Pixelize（{}，{}）",
        if resolution > 0 { format!("{resolution}×{resolution}") } else { "保持分辨率".into() },
        if palette_count > 0 { format!("{} 色", palette_count) } else { "保持颜色".into() }
    );
    save_as_new_version(&state, &detail, processed, &note)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_color_roundtrip() {
        assert_eq!(parse_hex_color("#FF8000").unwrap(), [255, 128, 0]);
        assert_eq!(parse_hex_color("ff8000").unwrap(), [255, 128, 0]);
        assert_eq!(to_hex([255, 128, 0]), "#FF8000");
        assert!(parse_hex_color("#FFF").is_err());
        assert!(parse_hex_color("#GGGGGG").is_err());
    }
}
