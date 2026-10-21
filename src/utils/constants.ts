import type { AssetType, ThemeMode } from "@/types";

/** 资产类型定义：label 为中文名，folder 与 Rust 端 filesystem::folder_for_asset_type 对应 */
export interface AssetTypeDef {
  value: AssetType;
  label: string;
  folder: string;
}

export const ASSET_TYPES: AssetTypeDef[] = [
  { value: "character", label: "角色", folder: "characters" },
  { value: "enemy", label: "敌人", folder: "enemies" },
  { value: "npc", label: "NPC", folder: "npc" },
  { value: "weapon", label: "武器", folder: "weapons" },
  { value: "item", label: "道具", folder: "items" },
  { value: "environment", label: "场景", folder: "environment" },
  { value: "building", label: "建筑", folder: "environment" },
  { value: "background", label: "背景", folder: "environment" },
  { value: "tile", label: "地块", folder: "tiles" },
  { value: "vfx", label: "特效", folder: "vfx" },
  { value: "ui", label: "UI", folder: "ui" },
  { value: "animation", label: "动画", folder: "animations" },
];

export function assetTypeLabel(type: string): string {
  return ASSET_TYPES.find((t) => t.value === type)?.label ?? type;
}

/** Generate 页第一阶段开放的类型 */
export const GENERATOR_READY_TYPES: AssetType[] = ["character"];

export const RESOLUTIONS = [16, 32, 48, 64, 96, 128] as const;

export const DIRECTIONS = [
  { value: "front", label: "正面" },
  { value: "back", label: "背面" },
  { value: "left", label: "朝左" },
  { value: "right", label: "朝右" },
] as const;

export const PERSPECTIVES = [
  { value: "side", label: "侧视（横版过关）" },
  { value: "topdown", label: "俯视（俯视角）" },
  { value: "iso", label: "等距（2.5D）" },
  { value: "three-quarter", label: "3/4 视角" },
  { value: "front-facing", label: "正面肖像" },
] as const;

export const PALETTES = [
  { value: "none", label: "不限" },
  { value: "gameboy", label: "Game Boy 4 色" },
  { value: "nes", label: "NES 8 色" },
  { value: "pico8", label: "PICO-8 16 色" },
  { value: "cga", label: "CGA 16 色" },
  { value: "warm", label: "暖色调" },
  { value: "cool", label: "冷色调" },
  { value: "mono", label: "单色" },
  { value: "pastel", label: "柔和浅色" },
  { value: "high-contrast", label: "高对比" },
] as const;

export const OUTLINES = [
  { value: "none", label: "无描边" },
  { value: "1px", label: "1px 描边" },
  { value: "2px", label: "2px 描边" },
] as const;

export const ANIMATIONS = [
  { value: "none", label: "单帧（无动画）" },
  { value: "idle", label: "待机（Idle）" },
  { value: "walk", label: "行走（Walk）" },
  { value: "run", label: "奔跑（Run）" },
  { value: "attack", label: "攻击（Attack）" },
  { value: "hurt", label: "受击（Hurt）" },
  { value: "death", label: "死亡（Death）" },
  { value: "jump", label: "跳跃（Jump）" },
  { value: "skill", label: "技能（Skill）" },
] as const;

export const FRAME_COUNTS = [1, 4, 6, 8, 12, 16] as const;

export const FPS_OPTIONS = [1, 4, 6, 8, 12, 15, 24, 30, 60] as const;

/** 动画编辑器使用的动画类型（与 Rust 端 animation_logic 对齐） */
export const ANIM_TYPE_OPTIONS = [
  { value: "idle", label: "待机（Idle）" },
  { value: "walk", label: "行走（Walk）" },
  { value: "run", label: "奔跑（Run）" },
  { value: "attack", label: "攻击（Attack）" },
  { value: "hurt", label: "受击（Hurt）" },
  { value: "death", label: "死亡（Death）" },
  { value: "jump", label: "跳跃（Jump）" },
  { value: "skill", label: "技能（Skill）" },
  { value: "custom", label: "自定义（Custom）" },
] as const;

/** 角色类资产：可作为动画生成的参考 */
export const CHARACTER_LIKE_TYPES: string[] = ["character", "enemy", "npc"];

/**
 * 像素画社区经典调色板（参考 Aseprite / Lospec 常用预设）。
 * 调色板版权归各自作者：
 * - PICO-8 © Lexaloffle Games
 * - Sweetie 16 © GrafxKid
 * - DB16 (DawnBringer 16) © DawnBringer
 * - EDG32 (Endesga 32) © Endesga
 * - Game Boy DMG / CGA 为硬件复刻调色板
 */
export interface PresetPalette {
  id: string;
  label: string;
  colors: string[];
}

export const PRESET_PALETTES: PresetPalette[] = [
  {
    id: "pico8",
    label: "PICO-8 (16)",
    colors: [
      "#000000", "#1D2B53", "#7E2553", "#008751", "#AB5236", "#5F574F",
      "#C2C3C7", "#FFF1E8", "#FF004D", "#FFA300", "#FFEC27", "#00E436",
      "#29ADFF", "#83769C", "#FF77A8", "#FFCCAA",
    ],
  },
  {
    id: "sweetie16",
    label: "Sweetie 16",
    colors: [
      "#1a1c2c", "#5d275d", "#b13e53", "#ef7d57", "#ffcd75", "#a7f070",
      "#38b764", "#257179", "#29366f", "#3b5dc9", "#41a6f6", "#73eff7",
      "#f4f4f4", "#94b0c2", "#566c86", "#333c57",
    ],
  },
  {
    id: "db16",
    label: "DawnBringer 16",
    colors: [
      "#140c1c", "#442434", "#30346d", "#4e4a4e", "#854c30", "#346524",
      "#d04648", "#757161", "#597dce", "#d27d2c", "#8595a1", "#6daa2c",
      "#d2aa99", "#6dc2ca", "#dad45e", "#deeed6",
    ],
  },
  {
    id: "edg32",
    label: "Endesga 32",
    colors: [
      "#be4a2f", "#d77643", "#ead4aa", "#e4a672", "#b86f50", "#733e39",
      "#3e2731", "#a22633", "#e43b44", "#f77622", "#feae34", "#fee761",
      "#63c74d", "#3e8948", "#265c42", "#193c3e", "#124e89", "#0099db",
      "#2ce8f5", "#ffffff", "#c0cbdc", "#8b9bb4", "#5a6988", "#3a4466",
      "#262b44", "#181425", "#ff0044", "#68386c", "#b55088", "#f6757a",
      "#e8b796", "#c28569",
    ],
  },
  {
    id: "gb",
    label: "Game Boy DMG (4)",
    colors: ["#0F380F", "#306230", "#8BAC0F", "#9BBC0F"],
  },
  {
    id: "cga",
    label: "CGA (16)",
    colors: [
      "#000000", "#0000AA", "#00AA00", "#00AAAA", "#AA0000", "#AA00AA",
      "#AA5500", "#AAAAAA", "#555555", "#5555FF", "#55FF55", "#55FFFF",
      "#FF5555", "#FF55FF", "#FFFF55", "#FFFFFF",
    ],
  },
];

/** 画布缩放档位（fit = 自适应容器） */
export const CANVAS_ZOOMS = [1, 2, 4, 8] as const;

/** 播放模式（参考 Aseprite / Pixelorama） */
export const PLAY_MODES = [
  { value: "loop", label: "循环" },
  { value: "pingpong", label: "往返" },
  { value: "once", label: "单次" },
] as const;

/** 与 Rust 端 providers/prompt.rs STYLE_PRESETS 对齐（用于下拉展示） */
export const STYLE_PRESETS = [
  { value: "retro-8bit", label: "8-bit 复古" },
  { value: "16bit", label: "16-bit 经典" },
  { value: "gba", label: "GBA 时代" },
  { value: "modern-pixel", label: "现代像素" },
  { value: "fantasy-rpg", label: "奇幻 RPG" },
  { value: "sci-fi", label: "科幻" },
  { value: "cute-chibi", label: "Q 版" },
  { value: "dark-fantasy", label: "暗黑奇幻" },
] as const;

export const PROVIDER_KIND_LABELS: Record<string, string> = {
  openai: "OpenAI",
  gemini: "Gemini",
  custom: "自定义",
};

export const DEFAULT_BASE_URLS: Record<string, string> = {
  openai: "https://api.openai.com/v1",
  gemini: "https://generativelanguage.googleapis.com/v1beta",
  custom: "",
};

export const DEFAULT_IMAGE_MODELS: Record<string, string> = {
  openai: "gpt-image-1",
  gemini: "gemini-2.5-flash-image",
  custom: "",
};

export const IMAGE_EXT_FILTER = ["png", "jpg", "jpeg", "gif", "bmp", "webp"];

export const THEME_DEFAULT: ThemeMode = "dark";
