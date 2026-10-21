//! 生成任务命令

use crate::database::{GenerationParams, GenerationTask};
use crate::errors::AppResult;
use crate::tasks::{self, AnimationGenParams, FrameRegenParams};
use crate::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn start_generation(
    app: AppHandle,
    _state: State<AppState>,
    params: GenerationParams,
) -> AppResult<GenerationTask> {
    tasks::spawn_generation(app, params)
}

/// 基于角色参考 / 文字描述生成动画（自动切帧建档）
#[tauri::command]
pub fn start_animation_generation(
    app: AppHandle,
    _state: State<AppState>,
    params: AnimationGenParams,
) -> AppResult<GenerationTask> {
    tasks::spawn_animation_generation(app, params)
}

/// 动画单帧 AI 重生成（只重画选中帧）
#[tauri::command]
pub fn regenerate_frame(
    app: AppHandle,
    _state: State<AppState>,
    params: FrameRegenParams,
) -> AppResult<GenerationTask> {
    tasks::spawn_frame_regeneration(app, params)
}

#[tauri::command]
pub fn list_tasks(state: State<AppState>, limit: Option<i64>) -> AppResult<Vec<GenerationTask>> {
    state.db.list_tasks(limit.unwrap_or(50))
}

#[tauri::command]
pub fn cancel_task(app: AppHandle, task_id: String) -> AppResult<GenerationTask> {
    tasks::cancel_task(&app, &task_id)
}

#[tauri::command]
pub fn retry_task(app: AppHandle, task_id: String) -> AppResult<GenerationTask> {
    tasks::retry_task(app, &task_id)
}

#[tauri::command]
pub fn clear_finished_tasks(state: State<AppState>) -> AppResult<()> {
    state.db.clear_finished_tasks()
}
