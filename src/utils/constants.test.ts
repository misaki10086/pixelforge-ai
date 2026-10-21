import { describe, expect, it } from "vitest";
import { buildPromptPreview } from "./promptPreview";
import { ASSET_TYPES, GENERATOR_READY_TYPES, assetTypeLabel, FPS_OPTIONS } from "./constants";
import type { GenerationParams } from "@/types";

function params(): GenerationParams {
  return {
    projectId: "p1",
    assetType: "character",
    assetName: "Knight",
    providerId: "prov",
    description: "a brave knight with flaming sword",
    style: "16bit",
    resolution: 64,
    perspective: "side",
    direction: "right",
    palette: "pico8",
    outline: "1px",
    animation: "walk",
    frameCount: 4,
    extraInstructions: "silver armor",
    autoResize: true,
    removeBackground: true,
  };
}

describe("buildPromptPreview", () => {
  it("contains every dimension of the form", () => {
    const prompt = buildPromptPreview(params());
    expect(prompt).toContain("16-bit SNES style");
    expect(prompt).toContain("game character sprite: a brave knight with flaming sword");
    expect(prompt).toContain("facing right");
    expect(prompt).toContain("64x64 pixel grid");
    expect(prompt).toContain("PICO-8 16-color palette");
    expect(prompt).toContain("1px dark outline");
    expect(prompt).toContain("4-frame horizontal sprite sheet");
    expect(prompt).toContain("silver armor");
  });

  it("omits sprite sheet for single frame", () => {
    const p = params();
    p.animation = "none";
    p.frameCount = 1;
    const prompt = buildPromptPreview(p);
    expect(prompt).not.toContain("sprite sheet");
    expect(prompt).toContain("single sprite");
  });

  it("skips empty palette", () => {
    const p = params();
    p.palette = "none";
    expect(buildPromptPreview(p)).not.toContain("palette");
  });
});

describe("constants", () => {
  it("asset types have unique values and labels", () => {
    const values = ASSET_TYPES.map((t) => t.value);
    expect(new Set(values).size).toBe(values.length);
    for (const t of ASSET_TYPES) {
      expect(assetTypeLabel(t.value)).toBe(t.label);
    }
  });

  it("character generator is ready in phase 1", () => {
    expect(GENERATOR_READY_TYPES).toContain("character");
  });

  it("fps options are sorted", () => {
    const sorted = [...FPS_OPTIONS].sort((a, b) => a - b);
    expect(sorted).toEqual([...FPS_OPTIONS]);
  });
});
