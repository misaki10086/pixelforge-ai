// Service 层：所有 Tauri invoke 的唯一出口。
// 组件与 store 只允许通过本目录访问后端，禁止在 UI 里直接操作 FS / SQLite / Provider。

import { invoke } from "@tauri-apps/api/core";
import type {
  Asset,
  AssetDetail,
  AnimationDetail,
  AnimationGenParams,
  AnimationView,
  DashboardStats,
  ExportResult,
  FrameRegenParams,
  GenerationParams,
  GenerationTask,
  Project,
  ProviderInput,
  ProviderTestResult,
  ProviderView,
  SpriteSheetResult,
} from "@/types";

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    // Rust 端 AppError 序列化为字符串，直接以中文错误向上抛
    throw new Error(typeof e === "string" ? e : String(e));
  }
}

export const projectApi = {
  list: () => call<Project[]>("list_projects"),
  create: (name: string, description: string, baseDir?: string) =>
    call<Project>("create_project", { name, description, baseDir: baseDir ?? null }),
  open: (projectId: string) => call<Project>("open_project", { projectId }),
  rename: (projectId: string, name: string) =>
    call<Project>("rename_project", { projectId, name }),
  duplicate: (projectId: string) => call<Project>("duplicate_project", { projectId }),
  delete: (projectId: string, deleteFiles: boolean) =>
    call<void>("delete_project", { projectId, deleteFiles }),
  persistJson: (projectId: string) => call<void>("persist_project_json", { projectId }),
};

export const assetApi = {
  list: (projectId: string) => call<Asset[]>("list_assets", { projectId }),
  detail: (assetId: string) => call<AssetDetail>("get_asset_detail", { assetId }),
  update: (assetId: string, name: string, tags: string[]) =>
    call<AssetDetail>("update_asset", { assetId, name, tags }),
  delete: (assetId: string) => call<void>("delete_asset", { assetId }),
  import: (projectId: string, sourcePath: string, assetType: string) =>
    call<Asset>("import_asset", { projectId, sourcePath, assetType }),
  restoreVersion: (versionId: string) =>
    call<AssetDetail>("restore_asset_version", { versionId }),
  deleteVersion: (versionId: string) =>
    call<AssetDetail>("delete_asset_version", { versionId }),
  dashboardStats: (projectId: string | null) =>
    call<DashboardStats>("get_dashboard_stats", { projectId }),
  export: (assetId: string, destDir: string, engine: string) =>
    call<ExportResult>("export_asset", { assetId, destDir, engine }),
};

export const providerApi = {
  list: () => call<ProviderView[]>("list_providers"),
  save: (input: ProviderInput) => call<ProviderView>("save_provider", { input }),
  delete: (providerId: string) => call<void>("delete_provider", { providerId }),
  test: (input: ProviderInput) => call<ProviderTestResult>("test_provider", { input }),
  listModels: (providerId: string) =>
    call<string[]>("list_provider_models", { providerId }),
};

export const generationApi = {
  start: (params: GenerationParams) => call<GenerationTask>("start_generation", { params }),
  startAnimation: (params: AnimationGenParams) =>
    call<GenerationTask>("start_animation_generation", { params }),
  regenerateFrame: (params: FrameRegenParams) =>
    call<GenerationTask>("regenerate_frame", { params }),
  listTasks: (limit?: number) => call<GenerationTask[]>("list_tasks", { limit: limit ?? null }),
  cancel: (taskId: string) => call<GenerationTask>("cancel_task", { taskId }),
  retry: (taskId: string) => call<GenerationTask>("retry_task", { taskId }),
  clearFinished: () => call<void>("clear_finished_tasks"),
};

export const animationApi = {
  list: (projectId: string) => call<AnimationView[]>("list_animations", { projectId }),
  detail: (animationId: string) => call<AnimationDetail>("get_animation_detail", { animationId }),
  create: (projectId: string, name: string, animType: string, fps: number, isLoop: boolean) =>
    call<AnimationView>("create_animation", { projectId, name, animType, fps, isLoop }),
  update: (animationId: string, name: string, fps: number, isLoop: boolean) =>
    call<AnimationDetail>("update_animation", { animationId, name, fps, isLoop }),
  delete: (animationId: string) => call<void>("delete_animation", { animationId }),
  moveFrame: (animationId: string, fromIndex: number, toIndex: number) =>
    call<AnimationDetail>("move_frame", { animationId, fromIndex, toIndex }),
  deleteFrame: (frameId: string) => call<AnimationDetail>("delete_frame", { frameId }),
  duplicateFrame: (frameId: string) => call<AnimationDetail>("duplicate_frame", { frameId }),
  updateFrameDuration: (frameId: string, durationMs: number) =>
    call<AnimationDetail>("update_frame_duration", { frameId, durationMs }),
  addFrameFromFile: (animationId: string, sourcePath: string) =>
    call<AnimationDetail>("add_frame_from_file", { animationId, sourcePath }),
  splitSpriteSheet: (assetId: string, columns: number, rows: number, fps: number, auto: boolean) =>
    call<AnimationView>("split_sprite_sheet", { assetId, columns, rows, fps, auto }),
  buildSheet: (animationId: string, columns: number, rows: number, padding: number, margin: number) =>
    call<SpriteSheetResult>("build_sprite_sheet", { animationId, columns, rows, padding, margin }),
  exportSheet: (
    animationId: string,
    columns: number,
    rows: number,
    padding: number,
    margin: number,
    destDir: string
  ) =>
    call<ExportResult>("export_animation_sheet", {
      animationId,
      columns,
      rows,
      padding,
      margin,
      destDir,
    }),
  exportGif: (animationId: string, destDir: string) =>
    call<ExportResult>("export_animation_gif", { animationId, destDir }),
  exportGodot: (animationId: string, destDir: string) =>
    call<ExportResult>("export_animation_godot", { animationId, destDir }),
};

export const imageApi = {
  extractPalette: (assetId: string, count: number) =>
    call<string[]>("extract_asset_palette", { assetId, count }),
  applyPalette: (assetId: string, colors: string[], dither: boolean) =>
    call<AssetDetail>("apply_asset_palette", { assetId, colors, dither }),
  pixelize: (assetId: string, resolution: number, paletteCount: number, dither: boolean) =>
    call<AssetDetail>("process_asset_pixelize", { assetId, resolution, paletteCount, dither }),
};

export const settingsApi = {
  all: () => call<Array<[string, string]>>("get_settings"),
  set: (key: string, value: string) => call<void>("set_setting", { key, value }),
  reveal: (path: string) => call<void>("reveal_path", { path }),
};
