//! 异步生成任务执行器。
//! 任务类型（kind）：
//! - generate_image     文生图资产（Character Generator 等）
//! - generate_animation 基于角色参考或文字描述生成动画 Sprite Sheet 并自动切帧
//! - regenerate_frame   动画单帧 AI 重生成（只重画选中帧）
//! 生命周期：queued → running → success / failed / cancelled。
//! 每次状态变化都会向前端 emit `task-updated` 事件。

use crate::database::{
    new_id, now_iso, AnimationFrame, Db, GenerationParams, GenerationTask, NewAnimation, NewAsset,
    ProviderConfig,
};
use crate::errors::{AppError, AppResult};
use crate::filesystem;
use crate::providers::{self, animation_logic, prompt};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

pub type TaskHandles = Mutex<HashMap<String, tauri::async_runtime::JoinHandle<()>>>;

// ---------------------------------------------------------------------------
// 任务参数
// ---------------------------------------------------------------------------

/// 动画生成请求（前端提交）
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationGenParams {
    pub project_id: String,
    pub name: String,
    pub anim_type: String,
    pub frame_count: u32,
    pub fps: i64,
    pub resolution: u32,
    pub provider_id: String,
    /// 参考角色资产（已生成的角色 → 动画）
    pub source_asset_id: Option<String>,
    /// 无参考时的角色描述；有参考时作为附加描述
    pub description: String,
    pub extra_instructions: String,
    pub remove_background: bool,
    /// 按透明间隙智能切帧（AI 输出帧距不均时更可靠），false 为均匀切分
    #[serde(default)]
    pub auto_split: bool,
}

/// 单帧重生成请求
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameRegenParams {
    pub animation_id: String,
    pub frame_id: String,
    pub provider_id: String,
    pub extra_instructions: String,
}

fn new_task_row(
    project_id: Option<String>,
    provider_id: &str,
    model: &str,
    kind: &str,
    params: serde_json::Value,
) -> GenerationTask {
    GenerationTask {
        id: new_id(),
        project_id,
        provider_id: provider_id.to_string(),
        model: model.to_string(),
        kind: kind.to_string(),
        status: "queued".into(),
        progress: 0.0,
        prompt: String::new(),
        params,
        error: None,
        asset_id: None,
        animation_id: None,
        created_at: now_iso(),
        started_at: None,
        finished_at: None,
        elapsed_ms: None,
    }
}

fn provider_image_model(db: &Db, provider_id: &str) -> AppResult<String> {
    let cfg = providers::load_provider_config(db, provider_id)?;
    Ok(cfg.image_model.clone())
}

// ---------------------------------------------------------------------------
// 进度上报辅助
// ---------------------------------------------------------------------------

struct Progress<'a> {
    app: &'a AppHandle,
    db: &'a Db,
    task_id: &'a str,
}

impl<'a> Progress<'a> {
    fn set(&self, p: f64) -> AppResult<()> {
        self.db.update_task_progress(self.task_id, "running", p)?;
        emit_task(self.app, self.db, self.task_id);
        Ok(())
    }
}

fn emit_task(app: &AppHandle, db: &Db, task_id: &str) {
    if let Ok(task) = db.get_task(task_id) {
        let _ = app.emit("task-updated", &task);
    }
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------

pub fn spawn_generation(app: AppHandle, params: GenerationParams) -> AppResult<GenerationTask> {
    prompt::validate_params(&params)?;
    let state = app.state::<crate::AppState>();
    let model = provider_image_model(&state.db, &params.provider_id)?;
    let task = new_task_row(
        Some(params.project_id.clone()),
        &params.provider_id,
        &model,
        "generate_image",
        serde_json::to_value(&params)?,
    );
    state.db.insert_task(&task)?;
    let handle = tauri::async_runtime::spawn(run_task(app.clone(), task.id.clone()));
    state.task_handles.lock().insert(task.id.clone(), handle);
    Ok(task)
}

pub fn spawn_animation_generation(
    app: AppHandle,
    params: AnimationGenParams,
) -> AppResult<GenerationTask> {
    validate_anim_params(&params)?;
    let state = app.state::<crate::AppState>();
    let model = provider_image_model(&state.db, &params.provider_id)?;
    let task = new_task_row(
        Some(params.project_id.clone()),
        &params.provider_id,
        &model,
        "generate_animation",
        serde_json::to_value(&params)?,
    );
    state.db.insert_task(&task)?;
    let handle = tauri::async_runtime::spawn(run_task(app.clone(), task.id.clone()));
    state.task_handles.lock().insert(task.id.clone(), handle);
    Ok(task)
}

pub fn spawn_frame_regeneration(
    app: AppHandle,
    params: FrameRegenParams,
) -> AppResult<GenerationTask> {
    let state = app.state::<crate::AppState>();
    let frame = state.db.get_frame(&params.frame_id)?;
    if frame.animation_id != params.animation_id {
        return Err(AppError::user("帧与动画不匹配，请刷新后重试。"));
    }
    let anim = state.db.get_animation(&params.animation_id)?;
    let model = provider_image_model(&state.db, &params.provider_id)?;
    let task = new_task_row(
        Some(anim.project_id.clone()),
        &params.provider_id,
        &model,
        "regenerate_frame",
        serde_json::to_value(&params)?,
    );
    state.db.insert_task(&task)?;
    let handle = tauri::async_runtime::spawn(run_task(app.clone(), task.id.clone()));
    state.task_handles.lock().insert(task.id.clone(), handle);
    Ok(task)
}

fn validate_anim_params(p: &AnimationGenParams) -> AppResult<()> {
    if p.project_id.trim().is_empty() {
        return Err(AppError::user("请先选择或创建一个项目。"));
    }
    if p.provider_id.trim().is_empty() {
        return Err(AppError::user("请先在「设置 → Providers」中配置并选择一个 AI Provider。"));
    }
    if p.name.trim().is_empty() {
        return Err(AppError::user("请填写动画名称。"));
    }
    animation_logic::validate_anim_type(&p.anim_type)?;
    if p.frame_count == 0 {
        return Err(AppError::user("帧数不能为 0。"));
    }
    if p.source_asset_id.is_none() && p.description.trim().is_empty() {
        return Err(AppError::user(
            "请选择一个已有角色资产作为参考，或填写角色描述。",
        ));
    }
    Ok(())
}

pub fn cancel_task(app: &AppHandle, task_id: &str) -> AppResult<GenerationTask> {
    let state = app.state::<crate::AppState>();
    if let Some(handle) = state.task_handles.lock().remove(task_id) {
        handle.abort();
    }
    state.db.with(|conn| {
        conn.execute(
            "UPDATE generation_tasks SET status = 'cancelled', finished_at = ?2 WHERE id = ?1 AND status IN ('queued','running')",
            rusqlite::params![task_id, now_iso()],
        )
        .map_err(AppError::from)?;
        Ok(())
    })?;
    let task = state.db.get_task(task_id)?;
    let _ = app.emit("task-updated", &task);
    Ok(task)
}

/// 重试：按原任务类型与参数重新入队
pub fn retry_task(app: AppHandle, task_id: &str) -> AppResult<GenerationTask> {
    let state = app.state::<crate::AppState>();
    let old = state.db.get_task(task_id)?;
    if old.status != "failed" && old.status != "cancelled" {
        return Err(AppError::user("只能重试失败或已取消的任务。"));
    }
    match old.kind.as_str() {
        "generate_image" => {
            let params: GenerationParams = serde_json::from_value(old.params.clone())
                .map_err(|e| AppError::user(format!("任务参数已损坏，无法重试：{e}")))?;
            spawn_generation(app, params)
        }
        "generate_animation" => {
            let params: AnimationGenParams = serde_json::from_value(old.params.clone())
                .map_err(|e| AppError::user(format!("任务参数已损坏，无法重试：{e}")))?;
            spawn_animation_generation(app, params)
        }
        "regenerate_frame" => {
            let params: FrameRegenParams = serde_json::from_value(old.params.clone())
                .map_err(|e| AppError::user(format!("任务参数已损坏，无法重试：{e}")))?;
            spawn_frame_regeneration(app, params)
        }
        other => Err(AppError::user(format!("未知任务类型「{other}」，无法重试。"))),
    }
}

// ---------------------------------------------------------------------------
// 任务主体
// ---------------------------------------------------------------------------

async fn run_task(app: AppHandle, task_id: String) {
    let state = app.state::<crate::AppState>();
    let (kind, params_json) = match state.db.get_task(&task_id) {
        Ok(t) => (t.kind.clone(), t.params.clone()),
        Err(err) => {
            // 任务行已不存在（异常情况）：直接返回
            eprintln!("task {task_id} missing: {err}");
            return;
        }
    };

    let result = match kind.as_str() {
        "generate_image" => {
            match serde_json::from_value::<GenerationParams>(params_json) {
                Ok(p) => execute_generate_image(&app, &task_id, &p).await,
                Err(e) => Err(AppError::user(format!("任务参数解析失败：{e}"))),
            }
        }
        "generate_animation" => {
            match serde_json::from_value::<AnimationGenParams>(params_json) {
                Ok(p) => execute_generate_animation(&app, &task_id, &p).await,
                Err(e) => Err(AppError::user(format!("任务参数解析失败：{e}"))),
            }
        }
        "regenerate_frame" => {
            match serde_json::from_value::<FrameRegenParams>(params_json) {
                Ok(p) => execute_regenerate_frame(&app, &task_id, &p).await,
                Err(e) => Err(AppError::user(format!("任务参数解析失败：{e}"))),
            }
        }
        other => Err(AppError::user(format!("未知任务类型：{other}"))),
    };

    let state = app.state::<crate::AppState>();
    match result {
        Ok(TaskOutcome::Asset(asset_id)) => {
            let _ = state.db.finish_task(&task_id, "success", None, Some(&asset_id), None);
            let _ = app.emit("assets-changed", true);
        }
        Ok(TaskOutcome::Animation(animation_id)) => {
            let _ = state.db.finish_task(&task_id, "success", None, None, Some(&animation_id));
            let _ = app.emit("animations-changed", true);
        }
        Ok(TaskOutcome::Frame(animation_id)) => {
            let _ = state.db.finish_task(&task_id, "success", None, None, None);
            let _ = app.emit("animations-changed", animation_id);
        }
        Err(err) => {
            let _ = state.db.finish_task(&task_id, "failed", Some(&err.to_string()), None, None);
        }
    }
    emit_task(&app, &state.db, &task_id);
    state.task_handles.lock().remove(&task_id);
}

enum TaskOutcome {
    Asset(String),
    Animation(String),
    /// 单帧重生成：返回动画 id 便于定向刷新
    Frame(String),
}

// ---------------------------------------------------------------------------
// generate_image
// ---------------------------------------------------------------------------

async fn execute_generate_image(
    app: &AppHandle,
    task_id: &str,
    params: &GenerationParams,
) -> AppResult<TaskOutcome> {
    let state = app.state::<crate::AppState>();
    let db = &state.db;
    let progress = Progress { app, db, task_id };

    progress.set(0.05)?;
    let prompt_text = prompt::build_generation_prompt(params);
    db.set_task_prompt(task_id, &prompt_text)?;

    let cfg: ProviderConfig = providers::load_provider_config(db, &params.provider_id)?;
    let provider = providers::create_provider(cfg);
    progress.set(0.15)?;

    let png_bytes = provider.generate_image(&prompt_text, "1024x1024").await?;
    progress.set(0.65)?;

    let project = db.get_project(&params.project_id)?;
    let root = PathBuf::from(&project.path);
    filesystem::ensure_dir_exists(&root)?;
    let saved = tokio::task::spawn_blocking({
        let params = params.clone();
        move || filesystem::save_generated_asset(&root, &params, &png_bytes)
    })
    .await
    .map_err(|e| AppError::user(format!("任务线程异常终止：{e}")))??;
    progress.set(0.9)?;

    let asset_id = db.insert_asset_with_version(&NewAsset {
        project_id: params.project_id.clone(),
        name: params.asset_name.trim().to_string(),
        asset_type: params.asset_type.clone(),
        image_path: saved.image_path.to_string_lossy().to_string(),
        thumbnail_path: saved
            .thumbnail_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        width: saved.width,
        height: saved.height,
        provider_id: Some(provider.config().id.clone()),
        model: Some(provider.display_model()),
        prompt: prompt_text,
        version_note: "AI 生成".into(),
    })?;

    progress.set(1.0)?;
    Ok(TaskOutcome::Asset(asset_id))
}

// ---------------------------------------------------------------------------
// generate_animation
// ---------------------------------------------------------------------------

async fn execute_generate_animation(
    app: &AppHandle,
    task_id: &str,
    params: &AnimationGenParams,
) -> AppResult<TaskOutcome> {
    let state = app.state::<crate::AppState>();
    let db = &state.db;
    let progress = Progress { app, db, task_id };

    progress.set(0.05)?;

    let cfg = providers::load_provider_config(db, &params.provider_id)?;
    let provider = providers::create_provider(cfg);
    let frames = params.frame_count.max(1);
    let anim_type = params.anim_type.as_str();
    let extra = params.extra_instructions.trim();

    // 1. 组装提示词：有角色参考走图生图（edit_image），否则走文生图
    let png_bytes = if let Some(source_id) = &params.source_asset_id {
        let detail = db.get_asset_detail(source_id)?;
        let current = detail
            .versions
            .iter()
            .find(|v| v.version == detail.asset.current_version)
            .ok_or_else(|| AppError::user("参考资产版本数据异常。"))?;
        let reference = std::fs::read(&current.image_path)
            .map_err(|e| AppError::user(format!("读取参考角色图片失败：{e}")))?;
        let prompt_text = if params.description.trim().is_empty() {
            animation_logic::build_animation_from_character_prompt(anim_type, frames, extra)
        } else {
            animation_logic::build_animation_from_character_prompt(
                anim_type,
                frames,
                &format!("{}, {}", params.description.trim(), extra),
            )
        };
        db.set_task_prompt(task_id, &prompt_text)?;
        progress.set(0.15)?;
        provider.edit_image(&prompt_text, &reference, "1024x1024").await?
    } else {
        let prompt_text = animation_logic::build_animation_from_text_prompt(
            &params.description,
            anim_type,
            frames,
            extra,
        );
        db.set_task_prompt(task_id, &prompt_text)?;
        progress.set(0.15)?;
        provider.generate_image(&prompt_text, "1024x1024").await?
    };
    progress.set(0.6)?;

    // 2. 解码 + 管线（背景移除在整图上执行）
    let mut img = image::load_from_memory(&png_bytes)
        .map_err(|e| AppError::Image(format!("AI 返回的内容不是有效图片：{e}")))?
        .to_rgba8();
    if params.remove_background {
        img = crate::imageproc::remove_background(&img, 28);
        img = crate::imageproc::cleanup_alpha(&img, 128);
    }
    progress.set(0.7)?;

    // 3. 建档（先创建动画记录，得到 id 用于目录命名）
    let project = db.get_project(&params.project_id)?;
    let anim_id = db.create_animation(&NewAnimation {
        project_id: params.project_id.clone(),
        name: params.name.trim().to_string(),
        anim_type: params.anim_type.clone(),
        fps: params.fps.max(1),
        is_loop: true,
        source_asset_id: params.source_asset_id.clone(),
        prompt: db.get_task(task_id)?.prompt,
        resolution: params.resolution as i64,
    })?;

    let root = PathBuf::from(&project.path);
    let anim_dir = root.join("animations").join(&anim_id);
    let frames_dir = anim_dir.join("frames");
    let history_dir = anim_dir.join("history");
    std::fs::create_dir_all(&frames_dir)
        .map_err(|e| AppError::user(format!("创建动画目录失败：{e}")))?;
    std::fs::create_dir_all(&history_dir)
        .map_err(|e| AppError::user(format!("创建动画目录失败：{e}")))?;

    // 原始 Sheet 留档
    img.save(anim_dir.join("source_sheet.png"))
        .map_err(|e| AppError::user(format!("保存动画源图失败：{e}")))?;

    // 4. 切帧 + 逐帧缩放到目标分辨率
    let cells = if params.auto_split {
        crate::imageproc::split_image_auto(&img, frames)
    } else {
        crate::imageproc::split_image_cells(&img, frames, 1)
    };
    if cells.is_empty() {
        return Err(AppError::user("切帧结果为空，请尝试关闭智能切帧或调整帧数。"));
    }
    let duration_ms = (1000.0 / params.fps.max(1) as f64).round() as i64;
    let mut saved_paths: Vec<(PathBuf, u32, u32)> = Vec::new();
    for (i, cell) in cells.iter().enumerate() {
        let mut frame_img = cell.clone();
        if params.resolution > 0 {
            frame_img = crate::imageproc::resize_nearest(&frame_img, params.resolution);
        }
        let (w, h) = frame_img.dimensions();
        let path = frames_dir.join(format!("frame_{:02}.png", i + 1));
        frame_img
            .save(&path)
            .map_err(|e| AppError::user(format!("保存第 {} 帧失败：{e}", i + 1)))?;
        saved_paths.push((path, w, h));
    }
    progress.set(0.9)?;

    for (path, w, h) in &saved_paths {
        db.append_frame(
            &anim_id,
            &path.to_string_lossy(),
            duration_ms,
            *w as i64,
            *h as i64,
        )?;
    }

    progress.set(1.0)?;
    Ok(TaskOutcome::Animation(anim_id))
}

// ---------------------------------------------------------------------------
// regenerate_frame
// ---------------------------------------------------------------------------

async fn execute_regenerate_frame(
    app: &AppHandle,
    task_id: &str,
    params: &FrameRegenParams,
) -> AppResult<TaskOutcome> {
    let state = app.state::<crate::AppState>();
    let db = &state.db;
    let progress = Progress { app, db, task_id };

    progress.set(0.05)?;

    let frame: AnimationFrame = db.get_frame(&params.frame_id)?;
    let anim = db.get_animation(&frame.animation_id)?;
    let frames_total = anim.frame_count.max(1) as u32;

    let prompt_text = animation_logic::build_frame_regen_prompt(
        &anim.anim_type,
        frame.frame_index,
        frames_total,
        &params.extra_instructions,
    );
    db.set_task_prompt(task_id, &prompt_text)?;

    let cfg = providers::load_provider_config(db, &params.provider_id)?;
    let provider = providers::create_provider(cfg);
    let reference = std::fs::read(&frame.image_path)
        .map_err(|e| AppError::user(format!("读取当前帧失败：{e}")))?;
    progress.set(0.2)?;

    let png_bytes = provider
        .edit_image(&prompt_text, &reference, "1024x1024")
        .await?;
    progress.set(0.75)?;

    let mut new_img = image::load_from_memory(&png_bytes)
        .map_err(|e| AppError::Image(format!("AI 返回的内容不是有效图片：{e}")))?
        .to_rgba8();

    // 缩放回该帧原始尺寸
    if frame.width > 0 && frame.height > 0 {
        new_img = image::imageops::resize(
            &new_img,
            frame.width as u32,
            frame.height as u32,
            image::imageops::FilterType::Nearest,
        );
    }

    // 备份旧帧到 history/，再覆盖写入
    let frame_path = PathBuf::from(&frame.image_path);
    if frame_path.exists() {
        let history_dir = frame_path
            .parent()
            .map(|p| p.parent().map(|pp| pp.join("history")).unwrap_or_else(|| p.to_path_buf()))
            .unwrap_or_else(|| frame_path.clone());
        std::fs::create_dir_all(&history_dir)?;
        let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup = history_dir.join(format!(
            "frame_{:02}_{stamp}.png",
            frame.frame_index + 1
        ));
        let _ = std::fs::copy(&frame_path, &backup);
    }
    new_img
        .save(&frame_path)
        .map_err(|e| AppError::user(format!("写入新帧失败：{e}")))?;

    progress.set(1.0)?;
    Ok(TaskOutcome::Frame(anim.id))
}
