//! 生成提示词构建：把表单参数翻译成高质量像素画提示词。
//! 每个维度（风格 / 视角 / 方向 / 调色板 / 描边 / 动画）都是独立的映射函数。

use crate::database::GenerationParams;
use crate::errors::{AppError, AppResult};

/// 内置风格预设（Style Library 将在后续阶段扩展为用户自定义 Profile）
pub const STYLE_PRESETS: &[(&str, &str)] = &[
    ("retro-8bit", "8-bit retro NES style, chunky pixels, very limited colors"),
    ("16bit", "16-bit SNES style, detailed pixel art, rich shading"),
    ("gba", "GBA era style, soft pixel shading, vibrant colors"),
    ("modern-pixel", "modern high-fidelity pixel art, crisp pixels, subtle dithering"),
    ("fantasy-rpg", "fantasy RPG pixel art style, classic JRPG aesthetic"),
    ("sci-fi", "sci-fi pixel art style, neon accents, futuristic"),
    ("cute-chibi", "cute chibi pixel art style, big head, small body"),
    ("dark-fantasy", "dark fantasy pixel art style, moody lighting, grim atmosphere"),
];

pub const PALETTES: &[(&str, &str)] = &[
    ("none", ""),
    ("gameboy", "4-tone Game Boy green palette"),
    ("nes", "NES 8-color limited palette"),
    ("pico8", "PICO-8 16-color palette"),
    ("cga", "CGA 16-color palette"),
    ("warm", "warm color palette, orange and amber tones"),
    ("cool", "cool color palette, blue and teal tones"),
    ("mono", "monochrome palette"),
    ("pastel", "soft pastel palette"),
    ("high-contrast", "high contrast palette"),
];

pub const PERSPECTIVES: &[(&str, &str)] = &[
    ("side", "side-view, side-scroller perspective"),
    ("topdown", "top-down view"),
    ("iso", "isometric perspective"),
    ("three-quarter", "3/4 view perspective"),
    ("front-facing", "front facing portrait view"),
];

pub const DIRECTIONS: &[(&str, &str)] = &[
    ("front", "facing the camera (front view)"),
    ("back", "facing away from the camera (back view)"),
    ("left", "facing left"),
    ("right", "facing right"),
];

pub const ANIMATIONS: &[(&str, &str)] = &[
    ("idle", "idle breathing animation"),
    ("walk", "walk cycle animation"),
    ("run", "run cycle animation"),
    ("attack", "attack animation, swinging weapon"),
    ("hurt", "hurt / damage animation"),
    ("death", "death animation"),
    ("jump", "jump animation"),
    ("skill", "special skill cast animation"),
];

fn lookup(table: &[(&str, &str)], key: &str) -> String {
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
        .unwrap_or_default()
}

pub fn build_generation_prompt(p: &GenerationParams) -> String {
    let type_desc = match p.asset_type.as_str() {
        "character" => "game character sprite",
        "enemy" => "game enemy monster sprite",
        "npc" => "game NPC villager sprite",
        "weapon" => "game weapon item sprite",
        "item" => "game item icon sprite",
        "environment" => "game environment scene art",
        "building" => "game building sprite",
        "background" => "game background landscape art",
        "tile" => "seamless game tile texture",
        "vfx" => "game VFX effect frames",
        "ui" => "game UI element",
        "animation" => "game animation sprite",
        _ => "game asset sprite",
    };

    let mut parts: Vec<String> = Vec::new();

    parts.push(lookup(STYLE_PRESETS, &p.style));
    parts.push(format!("{type_desc}: {}", p.description.trim()));

    let perspective = lookup(PERSPECTIVES, &p.perspective);
    if !perspective.is_empty() {
        parts.push(perspective);
    }
    let direction = lookup(DIRECTIONS, &p.direction);
    if !direction.is_empty() {
        parts.push(direction);
    }

    // 分辨率提示：AI 实际输出 1024px，但要求画面按目标网格设计
    if p.resolution > 0 {
        parts.push(format!(
            "designed on a {}x{} pixel grid",
            p.resolution, p.resolution
        ));
    }

    let palette = lookup(PALETTES, &p.palette);
    if !palette.is_empty() {
        parts.push(palette);
    }

    match p.outline.as_str() {
        "1px" => parts.push("1px dark outline".into()),
        "2px" => parts.push("2px dark outline".into()),
        _ => parts.push("no outline".into()),
    }

    if p.animation != "none" && !p.animation.is_empty() {
        let anim = lookup(ANIMATIONS, &p.animation);
        if p.frame_count > 1 {
            parts.push(format!(
                "{anim}, {}-frame horizontal sprite sheet, frames evenly spaced left to right, same character consistent across all frames",
                p.frame_count
            ));
        } else {
            parts.push(anim);
        }
    } else {
        parts.push("single sprite, centered, full body visible".into());
    }

    parts.push(
        "clean solid background, crisp hard pixels, no anti-aliasing, no blur, high quality game asset"
            .into(),
    );

    let extra = p.extra_instructions.trim();
    if !extra.is_empty() {
        parts.push(extra.to_string());
    }

    parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// 校验生成参数
pub fn validate_params(p: &GenerationParams) -> AppResult<()> {
    if p.project_id.trim().is_empty() {
        return Err(AppError::user("请先选择或创建一个项目。"));
    }
    if p.provider_id.trim().is_empty() {
        return Err(AppError::user("请先在「设置 → Providers」中配置并选择一个 AI Provider。"));
    }
    if p.description.trim().is_empty() {
        return Err(AppError::user("请填写角色描述（Description）。"));
    }
    if p.asset_name.trim().is_empty() {
        return Err(AppError::user("请填写资产名称（Asset Name）。"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> GenerationParams {
        GenerationParams {
            project_id: "p1".into(),
            asset_type: "character".into(),
            asset_name: "Knight".into(),
            provider_id: "prov1".into(),
            description: "a brave knight with flaming sword".into(),
            style: "16bit".into(),
            resolution: 64,
            perspective: "side".into(),
            direction: "right".into(),
            palette: "pico8".into(),
            outline: "1px".into(),
            animation: "walk".into(),
            frame_count: 4,
            extra_instructions: "silver armor".into(),
            auto_resize: true,
            remove_background: true,
        }
    }

    #[test]
    fn prompt_contains_all_dimensions() {
        let prompt = build_generation_prompt(&params());
        assert!(prompt.contains("16-bit SNES style"));
        assert!(prompt.contains("game character sprite"));
        assert!(prompt.contains("a brave knight with flaming sword"));
        assert!(prompt.contains("facing right"));
        assert!(prompt.contains("64x64 pixel grid"));
        assert!(prompt.contains("PICO-8"));
        assert!(prompt.contains("1px dark outline"));
        assert!(prompt.contains("4-frame horizontal sprite sheet"));
        assert!(prompt.contains("silver armor"));
    }

    #[test]
    fn prompt_no_animation_when_none() {
        let mut p = params();
        p.animation = "none".into();
        p.frame_count = 1;
        let prompt = build_generation_prompt(&p);
        assert!(!prompt.contains("sprite sheet"));
        assert!(prompt.contains("single sprite"));
    }

    #[test]
    fn validate_rejects_empty_description() {
        let mut p = params();
        p.description = "  ".into();
        assert!(validate_params(&p).is_err());
        p.description = "knight".into();
        p.provider_id = String::new();
        assert!(validate_params(&p).is_err());
    }
}
