// 全局共享类型：与 Rust 端 serde camelCase 序列化一一对应

export type ViewId =
  | "dashboard"
  | "generate"
  | "assets"
  | "animation"
  | "editor"
  | "projects"
  | "styles"
  | "settings";

export type AssetType =
  | "character"
  | "enemy"
  | "npc"
  | "weapon"
  | "item"
  | "environment"
  | "building"
  | "background"
  | "tile"
  | "vfx"
  | "ui"
  | "animation";

export type TaskStatus = "queued" | "running" | "success" | "failed" | "cancelled";

export interface Project {
  id: string;
  name: string;
  description: string;
  path: string;
  createdAt: string;
  updatedAt: string;
  lastOpenedAt: string | null;
  assetCount: number;
}

export interface Asset {
  id: string;
  projectId: string;
  name: string;
  type: AssetType;
  createdAt: string;
  updatedAt: string;
  imagePath: string;
  thumbnailPath: string | null;
  width: number;
  height: number;
  styleId: string | null;
  providerId: string | null;
  model: string | null;
  prompt: string;
  tags: string[];
  currentVersion: number;
}

export interface AssetVersion {
  id: string;
  assetId: string;
  version: number;
  imagePath: string;
  thumbnailPath: string | null;
  width: number | null;
  height: number | null;
  prompt: string;
  providerId: string | null;
  model: string | null;
  note: string;
  createdAt: string;
}

export interface AssetDetail {
  asset: Asset;
  versions: AssetVersion[];
}

export type ProviderKind = "openai" | "gemini" | "custom";

export interface HeaderPair {
  key: string;
  value: string;
}

export interface ProviderView {
  id: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  model: string;
  imageModel: string;
  requestFormat: string;
  headers: HeaderPair[];
  isDefault: boolean;
  createdAt: string;
  updatedAt: string;
  hasKey: boolean;
}

export interface ProviderInput {
  id?: string;
  name: string;
  kind: ProviderKind;
  baseUrl: string;
  model: string;
  imageModel: string;
  requestFormat: string;
  headers: HeaderPair[];
  apiKey: string;
  isDefault: boolean;
}

export interface ProviderTestResult {
  ok: boolean;
  message: string;
  latencyMs: number;
  models: string[];
}

export interface GenerationTask {
  id: string;
  projectId: string | null;
  providerId: string;
  model: string;
  kind: string;
  status: TaskStatus;
  progress: number;
  prompt: string;
  /** 任务参数 JSON（三种任务结构不同，按需读取字段） */
  params: Record<string, unknown> | null;
  error: string | null;
  assetId: string | null;
  animationId: string | null;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  elapsedMs: number | null;
}

export interface GenerationParams {
  projectId: string;
  assetType: AssetType;
  assetName: string;
  providerId: string;
  description: string;
  style: string;
  resolution: number;
  perspective: string;
  direction: string;
  palette: string;
  outline: string;
  animation: string;
  frameCount: number;
  extraInstructions: string;
  autoResize: boolean;
  removeBackground: boolean;
}

export interface DashboardStats {
  totalAssets: number;
  byType: [string, number][];
  totalAnimations: number;
  totalTasks: number;
  failedTasks: number;
  recentAssets: Asset[];
  recentTasks: GenerationTask[];
}

export interface ExportResult {
  writtenFiles: string[];
}

export type ThemeMode = "dark" | "light";

// ---------------------------------------------------------------------------
// 动画系统（第二阶段）
// ---------------------------------------------------------------------------

export type AnimationType =
  | "idle"
  | "walk"
  | "run"
  | "attack"
  | "hurt"
  | "death"
  | "jump"
  | "skill"
  | "custom";

export interface AnimationView {
  id: string;
  projectId: string;
  name: string;
  type: AnimationType;
  fps: number;
  loop: boolean;
  sourceAssetId: string | null;
  prompt: string;
  resolution: number;
  createdAt: string;
  updatedAt: string;
  frameCount: number;
}

export interface AnimationFrame {
  id: string;
  animationId: string;
  frameIndex: number;
  imagePath: string;
  durationMs: number;
  width: number;
  height: number;
  createdAt: string;
}

export interface AnimationDetail {
  animation: AnimationView;
  frames: AnimationFrame[];
}

export interface AnimationGenParams {
  projectId: string;
  name: string;
  animType: AnimationType;
  frameCount: number;
  fps: number;
  resolution: number;
  providerId: string;
  sourceAssetId: string | null;
  description: string;
  extraInstructions: string;
  removeBackground: boolean;
  /** 按透明间隙智能切帧（Retro Diffusion / sprite slicer 思路） */
  autoSplit: boolean;
}

export interface FrameRegenParams {
  animationId: string;
  frameId: string;
  providerId: string;
  extraInstructions: string;
}

export interface SpriteSheetFrame {
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  duration: number;
}

export interface SpriteSheetResult {
  pngPath: string;
  jsonPath: string;
  width: number;
  height: number;
  columns: number;
  rows: number;
  cellWidth: number;
  cellHeight: number;
  frames: SpriteSheetFrame[];
}
