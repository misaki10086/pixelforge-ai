//! 动画管理命令：CRUD、帧操作、Sprite Sheet 构建、GIF 导出、Sprite Sheet 拆分。
//!
//! 动画文件夹结构（逻辑清晰、相互隔离）：
//!   animations/{anim_id}/
//!     frames/frame_01.png ...     当前帧
//!     history/frame_01_*.png      单帧重生成的历史备份
//!     source_sheet.png            AI 生成的原始 Sprite Sheet
//!     sheet.png / sheet.json      构建产物（可导出）

use crate::database::{AnimationDetail, AnimationView, NewAnimation};
use crate::errors::{AppError, AppResult};
use crate::export::ExportResult;
use crate::filesystem;
use crate::providers::animation_logic;
use crate::AppState;
use serde::Serialize;
use tauri::State;

#[tauri::command]
pub fn list_animations(state: State<AppState>, project_id: String) -> AppResult<Vec<AnimationView>> {
    state.db.list_animations(&project_id)
}

#[tauri::command]
pub fn get_animation_detail(state: State<AppState>, animation_id: String) -> AppResult<AnimationDetail> {
    state.db.get_animation_detail(&animation_id)
}

#[tauri::command]
pub fn create_animation(
    state: State<AppState>,
    project_id: String,
    name: String,
    anim_type: String,
    fps: i64,
    is_loop: bool,
) -> AppResult<AnimationView> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::user("动画名称不能为空。"));
    }
    animation_logic::validate_anim_type(&anim_type)?;
    let id = state.db.create_animation(&NewAnimation {
        project_id,
        name,
        anim_type,
        fps: fps.max(1),
        is_loop,
        source_asset_id: None,
        prompt: String::new(),
        resolution: 0,
    })?;
    state.db.get_animation(&id)
}

#[tauri::command]
pub fn update_animation(
    state: State<AppState>,
    animation_id: String,
    name: String,
    fps: i64,
    is_loop: bool,
) -> AppResult<AnimationDetail> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::user("动画名称不能为空。"));
    }
    state
        .db
        .update_animation_meta(&animation_id, &name, fps.max(1), is_loop)?;
    state.db.get_animation_detail(&animation_id)
}

#[tauri::command]
pub fn delete_animation(state: State<AppState>, animation_id: String) -> AppResult<()> {
    let anim = state.db.get_animation(&animation_id)?;
    let files = state.db.delete_animation(&animation_id)?;
    for f in files {
        let _ = std::fs::remove_file(&f);
    }
    // 删除整个动画目录（frames / history / sheet 产物）
    let project = state.db.get_project(&anim.project_id)?;
    let dir = std::path::Path::new(&project.path)
        .join("animations")
        .join(&anim.id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .map_err(|e| AppError::user(format!("删除动画目录失败：{e}")))?;
    }
    Ok(())
}

#[tauri::command]
pub fn move_frame(
    state: State<AppState>,
    animation_id: String,
    from_index: i64,
    to_index: i64,
) -> AppResult<AnimationDetail> {
    state.db.move_frame(&animation_id, from_index, to_index)?;
    state.db.get_animation_detail(&animation_id)
}

#[tauri::command]
pub fn delete_frame(state: State<AppState>, frame_id: String) -> AppResult<AnimationDetail> {
    let frame = state.db.get_frame(&frame_id)?;
    state.db.delete_frame(&frame_id)?;
    if let Ok(p) = std::path::Path::new(&frame.image_path).canonicalize() {
        let _ = std::fs::remove_file(p);
    }
    state.db.normalize_frame_indexes(&frame.animation_id)?;
    state.db.get_animation_detail(&frame.animation_id)
}

#[tauri::command]
pub fn duplicate_frame(state: State<AppState>, frame_id: String) -> AppResult<AnimationDetail> {
    let frame = state.db.get_frame(&frame_id)?;
    let src = std::path::Path::new(&frame.image_path);
    if !src.exists() {
        return Err(AppError::user("帧图片文件不存在，无法复制。"));
    }
    let dest = src.with_file_name(format!(
        "frame_{:02}_{}.png",
        frame.frame_index + 1,
        crate::database::new_id()
    ));
    std::fs::copy(src, &dest).map_err(|e| AppError::user(format!("复制帧文件失败：{e}")))?;
    state.db.insert_frame_at(
        &frame.animation_id,
        frame.frame_index + 1,
        &dest.to_string_lossy(),
        frame.duration_ms,
        frame.width,
        frame.height,
    )?;
    state.db.normalize_frame_indexes(&frame.animation_id)?;
    state.db.get_animation_detail(&frame.animation_id)
}

/// 更新单帧时长（逐帧变速）
#[tauri::command]
pub fn update_frame_duration(
    state: State<AppState>,
    frame_id: String,
    duration_ms: i64,
) -> AppResult<AnimationDetail> {
    let frame = state.db.get_frame(&frame_id)?;
    state.db.update_frame_duration(&frame_id, duration_ms)?;
    state.db.get_animation_detail(&frame.animation_id)
}

/// 从本地图片添加新帧（追加到末尾）
#[tauri::command]
pub fn add_frame_from_file(
    state: State<AppState>,
    animation_id: String,
    source_path: String,
) -> AppResult<AnimationDetail> {
    let anim = state.db.get_animation(&animation_id)?;
    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(AppError::user(format!("文件不存在：{source_path}")));
    }
    let img = image::open(src).map_err(|e| AppError::user(format!("无法解析图片：{e}")))?;
    let rgba = img.to_rgba8();

    // 帧图片统一放入动画 frames 目录
    let project = state.db.get_project(&anim.project_id)?;
    let frames_dir = std::path::Path::new(&project.path)
        .join("animations")
        .join(&anim.id)
        .join("frames");
    std::fs::create_dir_all(&frames_dir)?;
    let dest = frames_dir.join(format!("frame_add_{}.png", crate::database::new_id()));
    rgba.save(&dest).map_err(|e| AppError::user(format!("保存帧失败：{e}")))?;

    state.db.append_frame(
        &animation_id,
        &dest.to_string_lossy(),
        (1000.0 / anim.fps.max(1) as f64).round() as i64,
        rgba.width() as i64,
        rgba.height() as i64,
    )?;
    state.db.get_animation_detail(&animation_id)
}

/// 把已有资产（如横向 Sprite Sheet）拆分为动画。
/// auto = true 时按透明间隙智能检测帧边界（检测失败回退均分）。
#[tauri::command]
pub fn split_sprite_sheet(
    state: State<AppState>,
    asset_id: String,
    columns: u32,
    rows: u32,
    fps: i64,
    auto: bool,
) -> AppResult<AnimationView> {
    let asset = state.db.get_asset(&asset_id)?;
    let img = image::open(&asset.image_path)
        .map_err(|e| AppError::user(format!("无法解析资产图片：{e}")))?;
    let rgba = img.to_rgba8();

    let (cells, desc) = if auto {
        let cells = crate::imageproc::split_image_auto(&rgba, columns.max(1));
        (
            cells,
            format!("自动检测帧边界（期望 {} 帧）", columns.max(1)),
        )
    } else {
        let cols = columns.max(1);
        let rows = rows.max(1);
        (
            crate::imageproc::split_image_cells(&rgba, cols, rows),
            format!("均匀网格 {}×{}", cols, rows),
        )
    };

    let anim_id = state.db.create_animation(&NewAnimation {
        project_id: asset.project_id.clone(),
        name: format!("{} 拆分", asset.name),
        anim_type: "custom".into(),
        fps: fps.max(1),
        is_loop: true,
        source_asset_id: Some(asset.id.clone()),
        prompt: format!("从资产「{}」拆分：{}", asset.name, desc),
        resolution: 0,
    })?;

    // 创建动画帧目录并落盘
    let project = state.db.get_project(&asset.project_id)?;
    let frames_dir = std::path::Path::new(&project.path)
        .join("animations")
        .join(&anim_id)
        .join("frames");
    std::fs::create_dir_all(&frames_dir)?;
    let duration_ms = (1000.0 / fps.max(1) as f64).round() as i64;
    for (i, cell) in cells.iter().enumerate() {
        let path = frames_dir.join(format!("frame_{:02}.png", i + 1));
        cell.save(&path)
            .map_err(|e| AppError::user(format!("保存第 {} 帧失败：{e}", i + 1)))?;
        state.db.append_frame(
            &anim_id,
            &path.to_string_lossy(),
            duration_ms,
            cell.width() as i64,
            cell.height() as i64,
        )?;
    }
    state.db.get_animation(&anim_id)
}

// ---------------------------------------------------------------------------
// Sprite Sheet 构建
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSheetResult {
    pub png_path: String,
    pub json_path: String,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub frames: Vec<SpriteSheetFrame>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSheetFrame {
    pub name: String,
    pub x: i64,
    pub y: i64,
    pub width: i64,
    pub height: i64,
    pub duration: i64,
}

#[tauri::command]
pub fn build_sprite_sheet(
    state: State<AppState>,
    animation_id: String,
    columns: u32,
    rows: u32,
    padding: u32,
    margin: u32,
) -> AppResult<SpriteSheetResult> {
    build_sheet_inner(&state, &animation_id, columns, rows, padding, margin).map(|(r, _)| r)
}

/// 构建逻辑复用入口：返回构建结果 + 动画目录路径
fn build_sheet_inner(
    state: &AppState,
    animation_id: &str,
    columns: u32,
    rows: u32,
    padding: u32,
    margin: u32,
) -> AppResult<(SpriteSheetResult, std::path::PathBuf)> {
    let detail = state.db.get_animation_detail(animation_id)?;
    let anim = &detail.animation;
    if detail.frames.is_empty() {
        return Err(AppError::user("动画没有任何帧，无法构建 Sprite Sheet。"));
    }
    let cols = columns.max(1);
    let rows = rows.max(1);
    if (cols as usize) * (rows as usize) < detail.frames.len() {
        return Err(AppError::user(format!(
            "网格 {}×{} 不足以容纳 {} 帧。",
            cols,
            rows,
            detail.frames.len()
        )));
    }

    // 单元尺寸取最大帧尺寸
    let cell_w = detail.frames.iter().map(|f| f.width).max().unwrap_or(1).max(1) as u32;
    let cell_h = detail.frames.iter().map(|f| f.height).max().unwrap_or(1).max(1) as u32;
    let sheet_w = margin * 2 + cols * cell_w + (cols - 1) * padding;
    let sheet_h = margin * 2 + rows * cell_h + (rows - 1) * padding;

    let mut canvas = image::RgbaImage::new(sheet_w, sheet_h);
    let mut frames_json = vec![];
    for (i, frame) in detail.frames.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = margin + col * (cell_w + padding);
        let y = margin + row * (cell_h + padding);
        let img = image::open(&frame.image_path)
            .map_err(|e| AppError::user(format!("读取帧 {} 失败：{}", i + 1, e)))?
            .to_rgba8();
        image::imageops::overlay(&mut canvas, &img, x as i64, y as i64);
        frames_json.push(SpriteSheetFrame {
            name: format!("{}_{}", filesystem::sanitize_name(&anim.name), i + 1),
            x: x as i64,
            y: y as i64,
            width: frame.width,
            height: frame.height,
            duration: frame.duration_ms,
        });
    }

    // 保存到动画目录
    let project = state.db.get_project(&anim.project_id)?;
    let anim_dir = std::path::Path::new(&project.path).join("animations").join(&anim.id);
    std::fs::create_dir_all(&anim_dir)?;
    let png_path = anim_dir.join("sheet.png");
    let json_path = anim_dir.join("sheet.json");
    canvas.save(&png_path).map_err(|e| AppError::user(format!("保存 Sprite Sheet 失败：{e}")))?;

    let meta = serde_json::json!({
        "meta": {
            "app": "PixelForge AI",
            "version": env!("CARGO_PKG_VERSION"),
            "image": "sheet.png",
            "size": { "w": sheet_w, "h": sheet_h },
            "animation": {
                "name": anim.name,
                "type": anim.anim_type,
                "fps": anim.fps,
                "loop": anim.is_loop,
                "frameCount": detail.frames.len(),
            }
        },
        "frames": frames_json,
    });
    std::fs::write(&json_path, serde_json::to_vec_pretty(&meta)?)
        .map_err(|e| AppError::user(format!("保存 Sprite Sheet JSON 失败：{e}")))?;

    Ok((
        SpriteSheetResult {
            png_path: png_path.to_string_lossy().to_string(),
            json_path: json_path.to_string_lossy().to_string(),
            width: sheet_w,
            height: sheet_h,
            columns: cols,
            rows,
            cell_width: cell_w,
            cell_height: cell_h,
            frames: frames_json,
        },
        anim_dir,
    ))
}

/// 把构建好的 sheet.png + sheet.json 拷贝到用户选择的目录
#[tauri::command]
pub fn export_animation_sheet(
    state: State<AppState>,
    animation_id: String,
    columns: u32,
    rows: u32,
    padding: u32,
    margin: u32,
    dest_dir: String,
) -> AppResult<ExportResult> {
    let sheet = build_sprite_sheet(state, animation_id, columns, rows, padding, margin)?;
    let dest = std::path::Path::new(&dest_dir);
    std::fs::create_dir_all(dest).map_err(|e| AppError::user(format!("无法创建导出目录：{e}")))?;
    let mut written = vec![];
    for src in [&sheet.png_path, &sheet.json_path] {
        let src_path = std::path::Path::new(src);
        let dest_path = dest.join(src_path.file_name().unwrap_or_default());
        std::fs::copy(src_path, &dest_path)
            .map_err(|e| AppError::user(format!("导出失败：{e}")))?;
        written.push(dest_path.to_string_lossy().to_string());
    }
    Ok(ExportResult { written_files: written })
}

/// 导出 Godot 4 SpriteFrames 资源（sheet.png + .tres，可直接挂到 AnimatedSprite2D）。
/// .tres 结构参考 Godot 官方 SpriteFrames / AtlasTexture 格式。
#[tauri::command]
pub fn export_animation_godot(
    state: State<AppState>,
    animation_id: String,
    dest_dir: String,
) -> AppResult<ExportResult> {
    let frame_count = state.db.get_animation_detail(&animation_id)?.frames.len();
    if frame_count == 0 {
        return Err(AppError::user("动画没有任何帧，无法导出。"));
    }
    let (sheet, _anim_dir) = build_sheet_inner(
        &state,
        &animation_id,
        frame_count as u32,
        1,
        0,
        0,
    )?;
    let anim_name = state.db.get_animation(&animation_id)?.name;

    let dest = std::path::Path::new(&dest_dir);
    std::fs::create_dir_all(dest).map_err(|e| AppError::user(format!("无法创建导出目录：{e}")))?;

    let stem = filesystem::sanitize_name(&anim_name);
    let dest_png = dest.join(format!("{stem}.png"));
    std::fs::copy(&sheet.png_path, &dest_png)
        .map_err(|e| AppError::user(format!("复制 sheet.png 失败：{e}")))?;

    // 生成 AtlasTexture 子资源
    let mut subs = String::new();
    let mut frame_refs = String::new();
    for (i, f) in sheet.frames.iter().enumerate() {
        subs.push_str(&format!(
            "[sub_resource type=\"AtlasTexture\" id=\"AtlasTexture_{i}\"]\natlas = ExtResource(\"1_sheet\")\nregion = Rect2({}, {}, {}, {})\n\n",
            f.x, f.y, f.width, f.height
        ));
        frame_refs.push_str(&format!(
            "{{\"duration\": 1.0, \"texture\": SubResource(\"AtlasTexture_{i}\")}}, "
        ));
    }
    let load_steps = sheet.frames.len() + 2;
    let tres = format!(
        "[gd_resource type=\"SpriteFrames\" load_steps={load_steps} format=3]\n\n\
[ext_resource type=\"Texture2D\" path=\"res://{stem}.png\" id=\"1_sheet\"]\n\n\
{subs}\
[resource]\n\
animations = [{{\n\
\"frames\": [{frame_refs}],\n\
\"loop\": {},\n\
\"name\": &\"default\",\n\
\"speed\": {}.0\n\
}}]\n",
        state.db.get_animation(&animation_id)?.is_loop,
        state.db.get_animation(&animation_id)?.fps,
    );
    let tres_path = dest.join(format!("{stem}.tres"));
    std::fs::write(&tres_path, tres)
        .map_err(|e| AppError::user(format!("写入 Godot SpriteFrames 失败：{e}")))?;

    Ok(ExportResult {
        written_files: vec![
            dest_png.to_string_lossy().to_string(),
            tres_path.to_string_lossy().to_string(),
        ],
    })
}

/// 导出 GIF（帧序列 + FPS → 动画 GIF）
#[tauri::command]
pub fn export_animation_gif(
    state: State<AppState>,
    animation_id: String,
    dest_dir: String,
) -> AppResult<ExportResult> {
    let detail = state.db.get_animation_detail(&animation_id)?;
    if detail.frames.is_empty() {
        return Err(AppError::user("动画没有任何帧，无法导出 GIF。"));
    }
    let anim = &detail.animation;
    let dest = std::path::Path::new(&dest_dir);
    std::fs::create_dir_all(dest).map_err(|e| AppError::user(format!("无法创建导出目录：{e}")))?;

    let name = filesystem::sanitize_name(&anim.name);
    let out_path = dest.join(format!("{name}.gif"));
    let file = std::fs::File::create(&out_path)
        .map_err(|e| AppError::user(format!("创建 GIF 文件失败：{e}")))?;

    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    encoder
        .set_repeat(image::codecs::gif::Repeat::Infinite)
        .map_err(|e| AppError::user(format!("GIF 编码失败：{e}")))?;

    for frame in &detail.frames {
        let img = image::open(&frame.image_path)
            .map_err(|e| AppError::user(format!("读取帧失败：{e}")))?
            .to_rgba8();
        let delay = image::Delay::from_numer_denom_ms(frame.duration_ms.max(20) as u32, 1000);
        let gif_frame = image::Frame::from_parts(img, 0, 0, delay);
        encoder
            .encode_frame(gif_frame)
            .map_err(|e| AppError::user(format!("GIF 编码失败：{e}")))?;
    }

    Ok(ExportResult {
        written_files: vec![out_path.to_string_lossy().to_string()],
    })
}
