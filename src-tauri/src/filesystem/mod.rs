//! 本地文件系统：项目目录结构、生成图片落盘、项目复制。
//! 数据库只存元数据，所有大文件都在项目目录内。

use crate::database::{GenerationParams, Project};
use crate::errors::{AppError, AppResult};
use crate::imageproc;
use std::path::{Path, PathBuf};

/// 项目固定目录结构
pub const PROJECT_FOLDERS: &[&str] = &[
    "assets/characters",
    "assets/enemies",
    "assets/npc",
    "assets/items",
    "assets/weapons",
    "assets/environment",
    "assets/tiles",
    "assets/vfx",
    "assets/ui",
    "assets/raw",
    "animations",
    "exports",
    "references",
    "thumbnails",
];

/// Asset Type → 项目内子目录
pub fn folder_for_asset_type(asset_type: &str) -> &'static str {
    match asset_type {
        "character" => "assets/characters",
        "enemy" => "assets/enemies",
        "npc" => "assets/npc",
        "item" => "assets/items",
        "weapon" => "assets/weapons",
        "building" | "environment" | "background" => "assets/environment",
        "tile" => "assets/tiles",
        "vfx" => "assets/vfx",
        "ui" => "assets/ui",
        "animation" => "animations",
        _ => "assets/raw",
    }
}

/// Windows 文件名合法化：非法字符替换为空格后按空白分组拼接连字符
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c => c,
        })
        .collect();
    let joined = cleaned.split_whitespace().collect::<Vec<_>>().join("-");
    if joined.is_empty() {
        "untitled".into()
    } else {
        joined
    }
}

pub fn ensure_project_structure(root: &Path) -> AppResult<()> {
    for dir in PROJECT_FOLDERS {
        std::fs::create_dir_all(root.join(dir))
            .map_err(|e| AppError::Io(std::io::Error::new(e.kind(), format!("创建目录失败 {}: {e}", root.join(dir).display()))))?;
    }
    Ok(())
}

pub fn write_project_json(project: &Project) -> AppResult<()> {
    let doc = serde_json::json!({
        "id": project.id,
        "name": project.name,
        "description": project.description,
        "createdAt": project.created_at,
        "updatedAt": project.updated_at,
        "pixelforgeVersion": env!("CARGO_PKG_VERSION"),
    });
    let path = Path::new(&project.path).join("project.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&doc)?)
        .map_err(|e| AppError::user(format!("写入 project.json 失败：{e}")))?;
    Ok(())
}

/// 在 base_dir 下创建唯一的项目目录（重名自动追加 -2 / -3 ...）
pub fn create_unique_dir(base_dir: &Path, name: &str) -> AppResult<PathBuf> {
    let base = base_dir.join(sanitize_name(name));
    let mut candidate = base.clone();
    let mut n = 2;
    while candidate.exists() {
        candidate = base_dir.join(format!("{}-{}", sanitize_name(name), n));
        n += 1;
    }
    std::fs::create_dir_all(&candidate)
        .map_err(|e| AppError::user(format!("无法创建项目目录 {}：{e}", candidate.display())))?;
    Ok(candidate)
}

/// 目录不存在时按原样返回（用户可能自行移动了目录）
pub fn ensure_dir_exists(path: &Path) -> AppResult<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

pub struct SavedImage {
    pub image_path: PathBuf,
    pub thumbnail_path: Option<PathBuf>,
    pub width: i64,
    pub height: i64,
}

/// 把 PNG 字节保存为资产主图 + 缩略图（按参数执行处理管线）
pub fn save_generated_asset(
    project_root: &Path,
    params: &GenerationParams,
    png_bytes: &[u8],
) -> AppResult<SavedImage> {
    let asset_id = crate::database::new_id();
    let folder = folder_for_asset_type(&params.asset_type);
    let dir = project_root.join(folder);
    ensure_dir_exists(&dir)?;

    let img = image::load_from_memory(png_bytes)
        .map_err(|e| AppError::Image(format!("AI 返回的内容不是有效图片：{e}")))?;
    let mut rgba = img.to_rgba8();

    // 保留未经处理的原始输出，便于重新走管线
    let raw_dir = project_root.join("assets/raw");
    ensure_dir_exists(&raw_dir)?;
    let raw_path = raw_dir.join(format!("{asset_id}-original.png"));
    rgba.save(&raw_path).map_err(|e| AppError::user(format!("保存原始图片失败：{e}")))?;

    // 管线：背景移除 → 透明清理 → 分辨率归一化
    if params.remove_background {
        rgba = imageproc::remove_background(&rgba, 28);
        rgba = imageproc::cleanup_alpha(&rgba, 128);
    }
    if params.auto_resize && params.resolution > 0 && params.resolution != rgba.width() {
        rgba = imageproc::resize_nearest(&rgba, params.resolution);
    }

    let image_path = dir.join(format!("{asset_id}.png"));
    rgba.save(&image_path).map_err(|e| AppError::user(format!("保存资产图片失败：{e}")))?;

    let thumb_dir = project_root.join("thumbnails");
    ensure_dir_exists(&thumb_dir)?;
    let thumb_path = thumb_dir.join(format!("{asset_id}.png"));
    let thumbnail = imageproc::make_thumbnail(&rgba, 256);
    thumbnail.save(&thumb_path).map_err(|e| AppError::user(format!("保存缩略图失败：{e}")))?;

    Ok(SavedImage {
        image_path,
        thumbnail_path: Some(thumb_path),
        width: rgba.width() as i64,
        height: rgba.height() as i64,
    })
}

/// 递归复制目录（Duplicate Project 使用）
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_windows_filenames() {
        assert_eq!(sanitize_name("Knight: the <Brave>?"), "Knight-the-Brave");
        assert_eq!(sanitize_name("  "), "untitled");
        assert_eq!(sanitize_name("法師/魔導士*1"), "法師-魔導士-1");
    }

    #[test]
    fn folder_mapping_covers_all_types() {
        assert_eq!(folder_for_asset_type("character"), "assets/characters");
        assert_eq!(folder_for_asset_type("tile"), "assets/tiles");
        assert_eq!(folder_for_asset_type("building"), "assets/environment");
        assert_eq!(folder_for_asset_type("unknown"), "assets/raw");
    }

    #[test]
    fn unique_dir_appends_suffix() {
        let tmp = std::env::temp_dir().join(format!("pf-test-{}", crate::database::new_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let d1 = create_unique_dir(&tmp, "MyRPG").unwrap();
        assert!(d1.ends_with("MyRPG"));
        let d2 = create_unique_dir(&tmp, "MyRPG").unwrap();
        assert!(d2.ends_with("MyRPG-2"));
        std::fs::remove_dir_all(&tmp).ok();
    }
}
