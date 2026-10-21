//! PixelForge AI — AI 游戏资产工作台
//!
//! 架构：UI(React) → Tauri Command(Service) → database / filesystem / providers / tasks。
//! 前端永远不直接触碰文件系统、SQLite 或 AI Provider。

mod commands;
mod database;
mod errors;
mod export;
mod filesystem;
mod imageproc;
mod providers;
mod tasks;

use database::Db;
use tauri::Manager;

/// 全局共享状态
pub struct AppState {
    pub db: Db,
    pub task_handles: tasks::TaskHandles,
}

fn setup_directories(app: &tauri::App) -> Result<(std::path::PathBuf, std::path::PathBuf), Box<dyn std::error::Error>> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法定位应用数据目录：{e}"))?;
    let cache_dir = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| data_dir.join("cache"));
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&cache_dir)?;
    Ok((data_dir, cache_dir))
}

fn default_project_dir() -> String {
    dirs::document_dir()
        .map(|d| d.join("PixelForge").to_string_lossy().to_string())
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("PixelForge").to_string_lossy().to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let (data_dir, cache_dir) = setup_directories(app)?;
            let db = Db::open(&data_dir.join("pixelforge.db"))?;
            database::migrate(&db)?;

            // 首次运行：初始化默认设置
            let defaults = [
                ("project_dir", default_project_dir()),
                ("cache_dir", cache_dir.to_string_lossy().to_string()),
                ("theme", "dark".to_string()),
            ];
            for (k, v) in defaults {
                if db.get_setting(k)?.is_none() {
                    db.set_setting(k, &v)?;
                }
            }

            app.manage(AppState {
                db,
                task_handles: tasks::TaskHandles::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // projects
            commands::projects::list_projects,
            commands::projects::create_project,
            commands::projects::open_project,
            commands::projects::rename_project,
            commands::projects::duplicate_project,
            commands::projects::delete_project,
            // assets
            commands::assets::list_assets,
            commands::assets::get_asset_detail,
            commands::assets::update_asset,
            commands::assets::delete_asset,
            commands::assets::import_asset,
            commands::assets::restore_asset_version,
            commands::assets::delete_asset_version,
            commands::assets::get_dashboard_stats,
            // providers
            commands::providers::list_providers,
            commands::providers::save_provider,
            commands::providers::delete_provider,
            commands::providers::test_provider,
            commands::providers::list_provider_models,
            // generation
            commands::generation::start_generation,
            commands::generation::start_animation_generation,
            commands::generation::regenerate_frame,
            commands::generation::list_tasks,
            commands::generation::cancel_task,
            commands::generation::retry_task,
            commands::generation::clear_finished_tasks,
            // animation
            commands::animation::list_animations,
            commands::animation::get_animation_detail,
            commands::animation::create_animation,
            commands::animation::update_animation,
            commands::animation::delete_animation,
            commands::animation::move_frame,
            commands::animation::delete_frame,
            commands::animation::duplicate_frame,
            commands::animation::update_frame_duration,
            commands::animation::add_frame_from_file,
            commands::animation::split_sprite_sheet,
            commands::animation::build_sprite_sheet,
            commands::animation::export_animation_sheet,
            commands::animation::export_animation_gif,
            commands::animation::export_animation_godot,
            // image processing (color / pixelize)
            commands::image::extract_asset_palette,
            commands::image::apply_asset_palette,
            commands::image::process_asset_pixelize,
            // settings
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::reveal_path,
            commands::settings::persist_project_json,
            // export
            commands::export::export_asset,
        ])
        .run(tauri::generate_context!())
        .expect("PixelForge AI 启动失败");
}
