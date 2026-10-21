import { ANIMATIONS } from "./constants";
import type { GenerationParams } from "@/types";

/**
 * Prompt 预览构建：镜像 Rust 端 providers/prompt.rs 的组装逻辑，
 * 仅用于生成页实时预览；实际请求以 Rust 端构建结果为准。
 */

const STYLE_TEXT: Record<string, string> = {
  "retro-8bit": "8-bit retro NES style, chunky pixels, very limited colors",
  "16bit": "16-bit SNES style, detailed pixel art, rich shading",
  gba: "GBA era style, soft pixel shading, vibrant colors",
  "modern-pixel": "modern high-fidelity pixel art, crisp pixels, subtle dithering",
  "fantasy-rpg": "fantasy RPG pixel art style, classic JRPG aesthetic",
  "sci-fi": "sci-fi pixel art style, neon accents, futuristic",
  "cute-chibi": "cute chibi pixel art style, big head, small body",
  "dark-fantasy": "dark fantasy pixel art style, moody lighting, grim atmosphere",
};

const TYPE_TEXT: Record<string, string> = {
  character: "game character sprite",
  enemy: "game enemy monster sprite",
  npc: "game NPC villager sprite",
  weapon: "game weapon item sprite",
  item: "game item icon sprite",
  environment: "game environment scene art",
  building: "game building sprite",
  background: "game background landscape art",
  tile: "seamless game tile texture",
  vfx: "game VFX effect frames",
  ui: "game UI element",
  animation: "game animation sprite",
};

const DIRECTION_TEXT: Record<string, string> = {
  front: "facing the camera (front view)",
  back: "facing away from the camera (back view)",
  left: "facing left",
  right: "facing right",
};

const PERSPECTIVE_TEXT: Record<string, string> = {
  side: "side-view, side-scroller perspective",
  topdown: "top-down view",
  iso: "isometric perspective",
  "three-quarter": "3/4 view perspective",
  "front-facing": "front facing portrait view",
};

const PALETTE_TEXT: Record<string, string> = {
  none: "",
  gameboy: "4-tone Game Boy green palette",
  nes: "NES 8-color limited palette",
  pico8: "PICO-8 16-color palette",
  cga: "CGA 16-color palette",
  warm: "warm color palette, orange and amber tones",
  cool: "cool color palette, blue and teal tones",
  mono: "monochrome palette",
  pastel: "soft pastel palette",
  "high-contrast": "high contrast palette",
};

function animName(value: string): string {
  const found = ANIMATIONS.find((a) => a.value === value);
  if (!found) return value;
  const en = found.label.split(" ")[0];
  return en ? en.toLowerCase() : value;
}

export function buildPromptPreview(p: GenerationParams): string {
  const parts: string[] = [];
  parts.push(STYLE_TEXT[p.style] ?? STYLE_TEXT["16bit"]);
  parts.push(`${TYPE_TEXT[p.assetType] ?? "game asset sprite"}: ${p.description.trim()}`);

  const perspective = PERSPECTIVE_TEXT[p.perspective];
  if (perspective) parts.push(perspective);

  const direction = DIRECTION_TEXT[p.direction];
  if (direction) parts.push(direction);

  if (p.resolution > 0) parts.push(`designed on a ${p.resolution}x${p.resolution} pixel grid`);

  const palette = PALETTE_TEXT[p.palette];
  if (palette) parts.push(palette);

  if (p.outline === "1px") parts.push("1px dark outline");
  else if (p.outline === "2px") parts.push("2px dark outline");
  else parts.push("no outline");

  if (p.animation !== "none" && p.animation) {
    const name = animName(p.animation);
    if (p.frameCount > 1) {
      parts.push(
        `${name} animation, ${p.frameCount}-frame horizontal sprite sheet, frames evenly spaced left to right, same character consistent across all frames`
      );
    } else {
      parts.push(`${name} animation`);
    }
  } else {
    parts.push("single sprite, centered, full body visible");
  }

  parts.push(
    "clean solid background, crisp hard pixels, no anti-aliasing, no blur, high quality game asset"
  );

  if (p.extraInstructions.trim()) parts.push(p.extraInstructions.trim());

  return parts.filter(Boolean).join(", ");
}
