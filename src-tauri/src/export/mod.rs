//! 导出器接口与实现。
//! GenericExporter 第一阶段完整支持（PNG + JSON 元数据）；
//! Godot / Unity 导出器建立架构与基础实现，后续阶段补全。

use crate::database::{Asset, AssetVersion};
use crate::errors::{AppError, AppResult};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub written_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExportContext<'a> {
    pub asset: &'a Asset,
    pub version: &'a AssetVersion,
    pub dest_dir: &'a Path,
    /// 导出文件名（不含扩展名），已做合法性处理
    pub file_stem: &'a str,
}

/// 通用导出器接口：所有引擎导出器都实现此 trait
pub trait AssetExporter {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn export(&self, ctx: &ExportContext) -> AppResult<ExportResult>;
}

fn copy_png(src: &Path, dest: &Path) -> AppResult<()> {
    if !src.exists() {
        return Err(AppError::user(format!(
            "源图片文件不存在：{}（可能已被移动或删除）",
            src.display()
        )));
    }
    std::fs::copy(src, dest).map_err(|e| AppError::user(format!("导出 PNG 失败：{e}")))?;
    Ok(())
}

fn frame_json(asset: &Asset, version: &AssetVersion, stem: &str) -> serde_json::Value {
    serde_json::json!({
        "meta": {
            "app": "PixelForge AI",
            "version": env!("CARGO_PKG_VERSION"),
            "image": format!("{stem}.png"),
            "size": { "w": version.width, "h": version.height },
        },
        "asset": {
            "id": asset.id,
            "name": asset.name,
            "type": asset.asset_type,
            "createdAt": asset.created_at,
            "providerId": asset.provider_id,
            "model": asset.model,
            "prompt": asset.prompt,
            "tags": asset.tags,
        },
        "frame": {
            "name": stem,
            "x": 0,
            "y": 0,
            "width": version.width,
            "height": version.height,
            "duration": 100,
        }
    })
}

/// 第一阶段：PNG + JSON 元数据
pub struct GenericExporter;

impl AssetExporter for GenericExporter {
    fn name(&self) -> &'static str {
        "GenericExporter"
    }

    fn export(&self, ctx: &ExportContext) -> AppResult<ExportResult> {
        std::fs::create_dir_all(ctx.dest_dir)
            .map_err(|e| AppError::user(format!("无法创建导出目录：{e}")))?;
        let png_path = ctx.dest_dir.join(format!("{}.png", ctx.file_stem));
        copy_png(Path::new(&ctx.version.image_path), &png_path)?;

        let json_path = ctx.dest_dir.join(format!("{}.json", ctx.file_stem));
        std::fs::write(
            &json_path,
            serde_json::to_vec_pretty(&frame_json(ctx.asset, ctx.version, ctx.file_stem))?,
        )
        .map_err(|e| AppError::user(format!("导出 JSON 元数据失败：{e}")))?;

        Ok(ExportResult {
            written_files: vec![
                png_path.to_string_lossy().to_string(),
                json_path.to_string_lossy().to_string(),
            ],
        })
    }
}

/// Godot 基础导出：PNG + project 内可引用的 .tres 纹理描述（后续阶段接入 SpriteFrames）
pub struct GodotExporter;

impl AssetExporter for GodotExporter {
    fn name(&self) -> &'static str {
        "GodotExporter"
    }

    fn export(&self, ctx: &ExportContext) -> AppResult<ExportResult> {
        std::fs::create_dir_all(ctx.dest_dir)
            .map_err(|e| AppError::user(format!("无法创建导出目录：{e}")))?;
        let png_path = ctx.dest_dir.join(format!("{}.png", ctx.file_stem));
        copy_png(Path::new(&ctx.version.image_path), &png_path)?;

        // Godot 4.x CompressedTexture2D 资源描述
        let tres = format!(
            "[gd_resource type=\"CompressedTexture2D\" load_steps=1 format=3]\n\n[resource]\ncomment = \"Exported by PixelForge AI ({asset}: {name})\"\n",
            asset = ctx.asset.id,
            name = ctx.asset.name,
        );
        let tres_path = ctx.dest_dir.join(format!("{}.tres", ctx.file_stem));
        std::fs::write(&tres_path, tres)
            .map_err(|e| AppError::user(format!("写入 Godot 资源描述失败：{e}")))?;

        Ok(ExportResult {
            written_files: vec![png_path.to_string_lossy().to_string()],
        })
    }
}

/// Unity 基础导出：PNG + .meta 侧车文件（后续阶段接入 Sprite Sheet 切片配置）
pub struct UnityExporter;

impl AssetExporter for UnityExporter {
    fn name(&self) -> &'static str {
        "UnityExporter"
    }

    fn export(&self, ctx: &ExportContext) -> AppResult<ExportResult> {
        std::fs::create_dir_all(ctx.dest_dir)
            .map_err(|e| AppError::user(format!("无法创建导出目录：{e}")))?;
        let png_path = ctx.dest_dir.join(format!("{}.png", ctx.file_stem));
        copy_png(Path::new(&ctx.version.image_path), &png_path)?;

        let meta = format!(
            "fileFormatVersion: 2\nguid: {guid}\nTextureImporter:\n  textureType: 8 # Sprite\n  spritePixelsPerUnit: 16\ncomment: Exported by PixelForge AI\n",
            guid = crate::database::new_id()
        );
        let meta_path = ctx.dest_dir.join(format!("{}.png.meta", ctx.file_stem));
        std::fs::write(&meta_path, meta)
            .map_err(|e| AppError::user(format!("写入 Unity meta 失败：{e}")))?;

        Ok(ExportResult {
            written_files: vec![png_path.to_string_lossy().to_string()],
        })
    }
}

pub fn exporter_by_engine(engine: &str) -> Box<dyn AssetExporter> {
    match engine {
        "godot" => Box::new(GodotExporter),
        "unity" => Box::new(UnityExporter),
        _ => Box::new(GenericExporter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_png(path: &Path) {
        let img = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 0, 0, 255]));
        img.save(path).unwrap();
    }

    #[test]
    fn generic_exporter_writes_png_and_json() {
        let tmp = std::env::temp_dir().join(format!("pf-export-{}", crate::database::new_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("src.png");
        write_test_png(&src);

        let asset = Asset {
            id: "a1".into(),
            project_id: "p1".into(),
            name: "Knight".into(),
            asset_type: "character".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            image_path: src.to_string_lossy().to_string(),
            thumbnail_path: None,
            width: 8,
            height: 8,
            style_id: None,
            provider_id: None,
            model: None,
            prompt: "knight".into(),
            tags: vec!["hero".into()],
            current_version: 1,
        };
        let version = AssetVersion {
            id: "v1".into(),
            asset_id: "a1".into(),
            version: 1,
            image_path: src.to_string_lossy().to_string(),
            thumbnail_path: None,
            width: 8,
            height: 8,
            prompt: "knight".into(),
            provider_id: None,
            model: None,
            note: String::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let out_dir = tmp.join("out");
        let result = GenericExporter
            .export(&ExportContext {
                asset: &asset,
                version: &version,
                dest_dir: &out_dir,
                file_stem: "knight_v1",
            })
            .unwrap();

        assert!(out_dir.join("knight_v1.png").exists());
        assert!(out_dir.join("knight_v1.json").exists());
        assert_eq!(result.written_files.len(), 2);

        let json: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(out_dir.join("knight_v1.json")).unwrap())
                .unwrap();
        assert_eq!(json["frame"]["width"], 8);
        assert_eq!(json["asset"]["name"], "Knight");

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn godot_unity_exporters_write_sidecars() {
        let tmp = std::env::temp_dir().join(format!("pf-export-{}", crate::database::new_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let src = tmp.join("src.png");
        write_test_png(&src);

        let asset = Asset {
            id: "a1".into(),
            project_id: "p1".into(),
            name: "Knight".into(),
            asset_type: "character".into(),
            created_at: String::new(),
            updated_at: String::new(),
            image_path: src.to_string_lossy().to_string(),
            thumbnail_path: None,
            width: 8,
            height: 8,
            style_id: None,
            provider_id: None,
            model: None,
            prompt: String::new(),
            tags: vec![],
            current_version: 1,
        };
        let version = AssetVersion {
            id: "v1".into(),
            asset_id: "a1".into(),
            version: 1,
            image_path: src.to_string_lossy().to_string(),
            thumbnail_path: None,
            width: 8,
            height: 8,
            prompt: String::new(),
            provider_id: None,
            model: None,
            note: String::new(),
            created_at: String::new(),
        };
        let godot_dir = tmp.join("godot");
        GodotExporter
            .export(&ExportContext {
                asset: &asset,
                version: &version,
                dest_dir: &godot_dir,
                file_stem: "knight",
            })
            .unwrap();
        assert!(godot_dir.join("knight.png").exists());
        assert!(godot_dir.join("knight.tres").exists());

        let unity_dir = tmp.join("unity");
        UnityExporter
            .export(&ExportContext {
                asset: &asset,
                version: &version,
                dest_dir: &unity_dir,
                file_stem: "knight",
            })
            .unwrap();
        assert!(unity_dir.join("knight.png").exists());
        assert!(unity_dir.join("knight.png.meta").exists());

        std::fs::remove_dir_all(&tmp).ok();
    }
}
