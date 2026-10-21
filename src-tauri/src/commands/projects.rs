//! 项目管理命令

use crate::database::{new_id, now_iso, Db, Project};
use crate::errors::{AppError, AppResult};
use crate::filesystem;
use crate::AppState;
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn list_projects(state: State<AppState>) -> AppResult<Vec<Project>> {
    state.db.list_projects()
}

#[tauri::command]
pub fn create_project(
    state: State<AppState>,
    name: String,
    description: String,
    base_dir: Option<String>,
) -> AppResult<Project> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::user("项目名称不能为空。"));
    }
    let base = match base_dir {
        Some(d) if !d.trim().is_empty() => d,
        _ => state
            .db
            .get_setting("project_dir")?
            .ok_or_else(|| AppError::user("未配置项目存储目录，请先在「设置 → Storage」中选择。"))?,
    };
    let base_path = std::path::Path::new(base.trim());
    filesystem::ensure_dir_exists(base_path)?;

    let dir = filesystem::create_unique_dir(base_path, &name)?;
    let project = Project {
        id: new_id(),
        name,
        description: description.trim().to_string(),
        path: dir.to_string_lossy().to_string(),
        created_at: now_iso(),
        updated_at: now_iso(),
        last_opened_at: Some(now_iso()),
        asset_count: 0,
    };
    filesystem::ensure_project_structure(&dir)?;
    filesystem::write_project_json(&project)?;
    state.db.insert_project(&project)?;
    Ok(project)
}

#[tauri::command]
pub fn open_project(state: State<AppState>, project_id: String) -> AppResult<Project> {
    let project = state.db.get_project(&project_id)?;
    filesystem::ensure_dir_exists(std::path::Path::new(&project.path))?;
    state.db.touch_project(&project_id)?;
    // 记住最后打开的项目，供下次启动恢复
    state.db.set_setting("last_project_id", &project_id)?;
    state.db.get_project(&project_id)
}

#[tauri::command]
pub fn rename_project(state: State<AppState>, project_id: String, name: String) -> AppResult<Project> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::user("项目名称不能为空。"));
    }
    state.db.rename_project(&project_id, &name)?;
    let project = state.db.get_project(&project_id)?;
    filesystem::write_project_json(&project)?;
    Ok(project)
}

#[tauri::command]
pub fn duplicate_project(state: State<AppState>, project_id: String) -> AppResult<Project> {
    let src = state.db.get_project(&project_id)?;
    let src_path = std::path::Path::new(&src.path);
    let base_dir = src_path
        .parent()
        .ok_or_else(|| AppError::user("项目目录异常，无法确定复制目标位置。"))?;

    let dir = filesystem::create_unique_dir(base_dir, &format!("{}-copy", src.name))?;
    filesystem::ensure_project_structure(&dir)?;
    filesystem::copy_dir_recursive(src_path, &dir)?;

    let copy = Project {
        id: new_id(),
        name: format!("{} 副本", src.name),
        description: src.description.clone(),
        path: dir.to_string_lossy().to_string(),
        created_at: now_iso(),
        updated_at: now_iso(),
        last_opened_at: None,
        asset_count: 0,
    };
    filesystem::write_project_json(&copy)?;
    state.db.insert_project(&copy)?;
    state
        .db
        .copy_project_assets(&src.id, &copy.id, src_path, &dir)?;
    state.db.get_project(&copy.id)
}

#[tauri::command]
pub fn delete_project(
    state: State<AppState>,
    project_id: String,
    delete_files: bool,
) -> AppResult<()> {
    let project = state.db.get_project(&project_id)?;
    state.db.delete_project(&project_id)?;
    if delete_files {
        let path = std::path::Path::new(&project.path);
        if path.exists() {
            std::fs::remove_dir_all(path)
                .map_err(|e| AppError::user(format!("删除项目目录失败：{e}")))?;
        }
    }
    Ok(())
}

impl Db {
    /// 复制项目时同步复制资产与版本记录，路径前缀替换为新目录
    pub fn copy_project_assets(
        &self,
        src_id: &str,
        dst_id: &str,
        old_root: &std::path::Path,
        new_root: &std::path::Path,
    ) -> AppResult<()> {
        let old_prefix = old_root.to_string_lossy().to_string();
        let new_prefix = new_root.to_string_lossy().to_string();
        let remap = |p: &str| -> String {
            p.strip_prefix(&old_prefix)
                .map(|rest| format!("{new_prefix}{rest}"))
                .unwrap_or_else(|| p.to_string())
        };

        let assets = self.list_assets(src_id)?;
        for asset in assets {
            let detail = self.get_asset_detail(&asset.id)?;
            let new_asset_id = new_id();
            let now = now_iso();
            self.with(|conn| {
                conn.execute(
                    "INSERT INTO assets (id, project_id, name, type, created_at, updated_at, image_path, thumbnail_path, width, height, style_id, provider_id, model, prompt, tags, current_version)
                     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
                    params![
                        new_asset_id,
                        dst_id,
                        asset.name,
                        asset.asset_type,
                        asset.created_at,
                        now,
                        remap(&asset.image_path),
                        asset.thumbnail_path.as_deref().map(|p| remap(p)),
                        asset.width,
                        asset.height,
                        asset.style_id,
                        asset.provider_id,
                        asset.model,
                        asset.prompt,
                        serde_json::to_string(&asset.tags).unwrap_or_default(),
                        asset.current_version
                    ],
                )?;
                for v in &detail.versions {
                    conn.execute(
                        "INSERT INTO asset_versions (id, asset_id, version, image_path, thumbnail_path, width, height, prompt, provider_id, model, note, created_at)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                        params![
                            crate::database::new_id(),
                            new_asset_id,
                            v.version,
                            remap(&v.image_path),
                            v.thumbnail_path.as_deref().map(|p| remap(p)),
                            v.width,
                            v.height,
                            v.prompt,
                            v.provider_id,
                            v.model,
                            v.note,
                            v.created_at
                        ],
                    )?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}
