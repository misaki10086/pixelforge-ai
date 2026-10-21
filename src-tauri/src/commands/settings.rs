//! 设置与系统命令

use crate::errors::AppResult;
use crate::filesystem;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<Vec<(String, String)>> {
    state.db.all_settings()
}

#[tauri::command]
pub fn set_setting(state: State<AppState>, key: String, value: String) -> AppResult<()> {
    state.db.set_setting(&key, &value)
}

/// 在资源管理器中显示文件 / 目录
#[tauri::command]
pub fn reveal_path(path: String) -> AppResult<()> {
    #[cfg(target_os = "windows")]
    {
        let p = std::path::Path::new(&path);
        let arg = if p.is_file() {
            format!("/select,{}", p.display())
        } else {
            path.clone()
        };
        std::process::Command::new("explorer.exe")
            .arg(arg)
            .spawn()
            .map_err(|e| crate::errors::AppError::user(format!("无法打开资源管理器：{e}")))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = path;
    }
    Ok(())
}

/// Ctrl+S：把数据库中的项目元数据回写 project.json
#[tauri::command]
pub fn persist_project_json(state: State<AppState>, project_id: String) -> AppResult<()> {
    let project = state.db.get_project(&project_id)?;
    filesystem::write_project_json(&project)
}
