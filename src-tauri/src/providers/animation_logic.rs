//! 动画逻辑提示词（skill 模板）：为每种动画类型提供逐帧姿态描述，
//! 让 AI 生成的 Sprite Sheet 在动作逻辑上成立（接触/过渡/发力/回收等关键帧），
//! 并强制角色在各帧之间保持一致。

use crate::errors::{AppError, AppResult};

/// 动作关键帧逻辑：返回该动画类型在给定帧数下的逐帧描述（英文提示词片段）。
/// 4 帧为核心关键帧；更多帧时补充过渡帧描述。
pub fn animation_frame_logic(anim_type: &str, frames: u32) -> String {
    let f = frames.max(1);
    let core = core_poses(anim_type);
    let mut lines: Vec<String> = Vec::new();
    let core_len = core.len() as u32;
    for i in 1..=f {
        let pose = if i <= core_len {
            core[(i - 1) as usize].to_string()
        } else {
            // 超出核心关键帧：在相邻关键帧之间插值过渡
            let a = core[((i - 2) % core_len) as usize];
            let b = core[(i % core_len) as usize];
            format!("smooth in-between transitioning from \"{a}\" toward \"{b}\"")
        };
        lines.push(format!("frame {i}: {pose}"));
    }
    lines.join("; ")
}

fn core_poses(anim_type: &str) -> Vec<&'static str> {
    match anim_type {
        "idle" => vec![
            "neutral standing pose, arms relaxed at sides",
            "breathing in, chest slightly raised, head up by one pixel",
            "neutral standing pose",
            "breathing out, chest slightly lowered, head down by one pixel",
        ],
        "walk" => vec![
            "contact pose, right leg forward heel down, left arm swinging forward",
            "passing pose, legs crossing under the body, body at its highest point",
            "contact pose, left leg forward heel down, right arm swinging forward",
            "passing pose, legs crossing, body at its lowest point",
        ],
        "run" => vec![
            "forward lean contact pose, right leg extended far forward, arms bent and pumping",
            "airborne passing pose, both feet off the ground, body leaning forward",
            "forward lean contact pose, left leg extended far forward",
            "airborne passing pose, both feet off the ground, arms driving",
        ],
        "attack" => vec![
            "windup pose, weapon pulled back behind the body, weight on back foot",
            "strike pose, weapon swung forward across the target, motion arc trail",
            "impact follow-through pose, weapon fully extended, body rotated into the hit",
            "recovery pose, weapon returning to guard stance",
        ],
        "hurt" => vec![
            "recoil pose, body knocked backward, arms flailing up",
            "flinch pose, head thrown back, knees buckling",
            "stagger pose, body bent forward, off balance",
            "recovering pose, returning to guard stance",
        ],
        "death" => vec![
            "stagger pose, weapon dropped, body leaning back",
            "collapsing pose, falling to knees, head down",
            "falling pose, body crumpling to the ground",
            "final lying pose, motionless on the ground",
        ],
        "jump" => vec![
            "crouch anticipation pose, knees deeply bent, arms swung back",
            "launch pose, body fully extended upward, feet leaving the ground",
            "apex pose, legs tucked, body at highest point",
            "landing pose, knees bent absorbing impact, arms forward",
        ],
        "skill" => vec![
            "charging pose, both hands gathering glowing energy in front of the chest",
            "cast pose, energy released forward with a bright burst effect",
            "sustain pose, beam / effect at full power, body braced",
            "finish pose, effect dissipating into sparks, returning to stance",
        ],
        _ => vec![
            "starting pose of the action",
            "active middle pose of the action",
            "follow-through pose of the action",
            "recovery pose returning to neutral stance",
        ],
    }
}

/// 校验动画类型
pub fn validate_anim_type(anim_type: &str) -> AppResult<()> {
    const OK: &[&str] = &[
        "idle", "walk", "run", "attack", "hurt", "death", "jump", "skill", "custom",
    ];
    if OK.contains(&anim_type) {
        Ok(())
    } else {
        Err(AppError::user(format!(
            "不支持的动画类型「{anim_type}」，可选：{}",
            OK.join(" / ")
        )))
    }
}

/// 基于已有角色资产生成动画的完整提示词（走 edit_image，图生图）
pub fn build_animation_from_character_prompt(
    anim_type: &str,
    frames: u32,
    extra: &str,
) -> String {
    format!(
        "Using the provided character sprite as a STRICT reference, generate a {frames}-frame horizontal sprite sheet of the SAME character performing a {anim_type} animation. \
Frame-by-frame logic: {logic}. \
Consistency rules: identical character design, colors, outfit, proportions and outline style in every frame; same scale; feet aligned to the same baseline; frames evenly spaced left to right with no drifting; {frames} frames total. \
Clean solid background, crisp hard pixels, no anti-aliasing. {extra}",
        frames = frames,
        anim_type = anim_type,
        logic = animation_frame_logic(anim_type, frames),
        extra = extra.trim()
    )
}

/// 无参考时直接生成动画角色的提示词（走 generate_image）
pub fn build_animation_from_text_prompt(
    description: &str,
    anim_type: &str,
    frames: u32,
    extra: &str,
) -> String {
    format!(
        "pixel art game character sprite performing a {anim_type} animation: {description}. \
Generate a {frames}-frame horizontal sprite sheet. \
Frame-by-frame logic: {logic}. \
The SAME character in every frame, identical colors and proportions, same scale, feet on the same baseline, frames evenly spaced left to right. \
Clean solid background, crisp hard pixels, no anti-aliasing, high quality game asset. {extra}",
        anim_type = anim_type,
        description = description.trim(),
        frames = frames,
        logic = animation_frame_logic(anim_type, frames),
        extra = extra.trim()
    )
}

/// 单帧重生成提示词：保持角色不变，只重画该帧的姿态
pub fn build_frame_regen_prompt(
    anim_type: &str,
    frame_index: i64,
    frames: u32,
    extra: &str,
) -> String {
    let logic = animation_frame_logic(anim_type, frames);
    let one_line = single_frame_pose(anim_type, frame_index, frames);
    format!(
        "Keep the EXACT same character (identical colors, outfit, proportions, outline, style and background) and redraw ONLY this single frame. \
New pose: {one_line}. \
This is frame {idx} of a {total}-frame {anim_type} animation; overall logic: {logic}. \
Single sprite only, centered, no sprite sheet, no text. {extra}",
        one_line = one_line,
        idx = frame_index + 1,
        total = frames,
        anim_type = anim_type,
        logic = logic,
        extra = extra.trim()
    )
}

/// 单帧姿态描述（按索引在核心关键帧序列中取模）
fn single_frame_pose(anim_type: &str, frame_index: i64, frames: u32) -> String {
    let core = core_poses(anim_type);
    let core_len = core.len() as i64;
    if frame_index < core_len {
        core[frame_index as usize].to_string()
    } else {
        let a = core[((frame_index - 1) % core_len) as usize];
        let b = core[(frame_index % core_len) as usize];
        format!("smooth in-between between \"{a}\" and \"{b}\" (frame {frame_index} of {frames})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_logic_has_contact_and_passing() {
        let logic = animation_frame_logic("walk", 4);
        assert!(logic.contains("frame 1: contact pose"));
        assert!(logic.contains("passing pose"));
        assert!(logic.contains("frame 4:"));
    }

    #[test]
    fn more_frames_get_inbetweens() {
        let logic = animation_frame_logic("walk", 6);
        assert!(logic.contains("frame 5: smooth in-between"));
        assert!(logic.contains("frame 6: smooth in-between"));
    }

    #[test]
    fn all_builtin_types_valid() {
        for t in ["idle", "walk", "run", "attack", "hurt", "death", "jump", "skill", "custom"] {
            assert!(validate_anim_type(t).is_ok());
        }
        assert!(validate_anim_type("dance").is_err());
    }

    #[test]
    fn character_prompt_keeps_reference_strict() {
        let p = build_animation_from_character_prompt("attack", 4, "fiery sword");
        assert!(p.contains("STRICT reference"));
        assert!(p.contains("4-frame horizontal sprite sheet"));
        assert!(p.contains("windup pose"));
        assert!(p.contains("fiery sword"));
        assert!(p.contains("feet aligned to the same baseline"));
    }

    #[test]
    fn regen_prompt_targets_single_frame() {
        let p = build_frame_regen_prompt("walk", 2, 4, "");
        assert!(p.contains("redraw ONLY this single frame"));
        assert!(p.contains("frame 3 of a 4-frame walk"));
        assert!(p.contains("Single sprite only"));
    }
}
