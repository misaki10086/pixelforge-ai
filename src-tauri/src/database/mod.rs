//! SQLite 持久层：所有 SQL 集中在此模块，上层（commands / tasks）只调用领域方法。
//! 数据库仅保存元数据；图片等大文件由 filesystem 模块写入项目目录。

pub mod keyring;

use crate::errors::{AppError, AppResult};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

// ---------------------------------------------------------------------------
// 数据模型（serde camelCase，直接对齐前端 TypeScript 类型）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
    pub asset_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub asset_type: String,
    pub created_at: String,
    pub updated_at: String,
    pub image_path: String,
    pub thumbnail_path: Option<String>,
    pub width: i64,
    pub height: i64,
    pub style_id: Option<String>,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub tags: Vec<String>,
    pub current_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetVersion {
    pub id: String,
    pub asset_id: String,
    pub version: i64,
    pub image_path: String,
    pub thumbnail_path: Option<String>,
    pub width: i64,
    pub height: i64,
    pub prompt: String,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetDetail {
    pub asset: Asset,
    pub versions: Vec<AssetVersion>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub model: String,
    pub image_model: String,
    pub request_format: String,
    pub headers: Vec<(String, String)>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 提供给前端的 Provider 视图：绝不包含 API Key 本体，只携带 hasKey 标志
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderView {
    #[serde(flatten)]
    pub row: ProviderRow,
    pub has_key: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderPair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    pub id: Option<String>,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    pub model: String,
    pub image_model: String,
    pub request_format: String,
    pub headers: Vec<HeaderPair>,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub is_default: bool,
}

/// Provider 运行时配置（由 DB 行 + 凭据管理器合成），仅在 Rust 侧流转
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub base_url: String,
    /// 文本模型（generateText 等后续能力使用）
    #[allow(dead_code)]
    pub model: String,
    pub image_model: String,
    pub request_format: String,
    pub headers: std::collections::HashMap<String, String>,
    pub api_key: String,
}

/// 前端提交的生成请求参数（camelCase 序列化）
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationParams {
    pub project_id: String,
    pub asset_type: String,
    pub asset_name: String,
    pub provider_id: String,
    pub description: String,
    pub style: String,
    pub resolution: u32,
    pub perspective: String,
    pub direction: String,
    pub palette: String,
    pub outline: String,
    pub animation: String,
    pub frame_count: u32,
    pub extra_instructions: String,
    pub auto_resize: bool,
    pub remove_background: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderTestResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u128,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationTask {
    pub id: String,
    pub project_id: Option<String>,
    pub provider_id: String,
    pub model: String,
    pub kind: String,
    pub status: String,
    pub progress: f64,
    pub prompt: String,
    pub params: serde_json::Value,
    pub error: Option<String>,
    pub asset_id: Option<String>,
    pub animation_id: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub elapsed_ms: Option<i64>,
}

/// 动画（含帧数统计）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationView {
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub anim_type: String,
    pub fps: i64,
    #[serde(rename = "loop")]
    pub is_loop: bool,
    pub source_asset_id: Option<String>,
    pub prompt: String,
    pub resolution: i64,
    pub created_at: String,
    pub updated_at: String,
    pub frame_count: i64,
}

/// 动画单帧
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationFrame {
    pub id: String,
    pub animation_id: String,
    pub frame_index: i64,
    pub image_path: String,
    pub duration_ms: i64,
    pub width: i64,
    pub height: i64,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct NewAnimation {
    pub project_id: String,
    pub name: String,
    pub anim_type: String,
    pub fps: i64,
    pub is_loop: bool,
    pub source_asset_id: Option<String>,
    pub prompt: String,
    pub resolution: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationDetail {
    pub animation: AnimationView,
    pub frames: Vec<AnimationFrame>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub total_assets: i64,
    pub by_type: Vec<(String, i64)>,
    pub total_animations: i64,
    pub total_tasks: i64,
    pub failed_tasks: i64,
    pub recent_assets: Vec<Asset>,
    pub recent_tasks: Vec<GenerationTask>,
}

// ---------------------------------------------------------------------------
// 连接与迁移
// ---------------------------------------------------------------------------

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(path: &Path) -> AppResult<Self> {
        let conn = Connection::open(path)
            .map_err(|e| AppError::user(format!("无法打开数据库 {}: {}", path.display(), e)))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(Db { conn: Mutex::new(conn) })
    }

    /// 仅测试使用：内存数据库
    pub fn open_in_memory() -> AppResult<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(Db { conn: Mutex::new(conn) })
    }

    pub fn with<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let conn = self.conn.lock();
        f(&conn)
    }
}

pub fn migrate(db: &Db) -> AppResult<()> {
    db.with(|conn| {
        conn.execute_batch(SCHEMA_SQL)?;
        // 增量迁移：为旧库补充第二阶段新增列
        ensure_column(conn, "animations", "source_asset_id", "TEXT")?;
        ensure_column(conn, "animations", "prompt", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(conn, "animations", "resolution", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "animation_frames", "width", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "animation_frames", "height", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(conn, "generation_tasks", "animation_id", "TEXT")?;
        Ok(())
    })
}

/// 若列不存在则 ALTER TABLE 补充（SQLite 无 ADD COLUMN IF NOT EXISTS）
fn ensure_column(conn: &Connection, table: &str, column: &str, decl: &str) -> AppResult<()> {
    let exists: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name = ?1"),
        params![column],
        |r| r.get(0),
    )?;
    if exists == 0 {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"), [])?;
    }
    Ok(())
}

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  path TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_opened_at TEXT
);
CREATE TABLE IF NOT EXISTS assets (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  type TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  image_path TEXT NOT NULL,
  thumbnail_path TEXT,
  width INTEGER NOT NULL DEFAULT 0,
  height INTEGER NOT NULL DEFAULT 0,
  style_id TEXT,
  provider_id TEXT,
  model TEXT,
  prompt TEXT NOT NULL DEFAULT '',
  tags TEXT NOT NULL DEFAULT '[]',
  current_version INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX IF NOT EXISTS idx_assets_project ON assets(project_id);
CREATE TABLE IF NOT EXISTS asset_versions (
  id TEXT PRIMARY KEY,
  asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
  version INTEGER NOT NULL,
  image_path TEXT NOT NULL,
  thumbnail_path TEXT,
  width INTEGER,
  height INTEGER,
  prompt TEXT NOT NULL DEFAULT '',
  provider_id TEXT,
  model TEXT,
  note TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  UNIQUE(asset_id, version)
);
CREATE TABLE IF NOT EXISTS animations (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  type TEXT NOT NULL DEFAULT 'custom',
  fps INTEGER NOT NULL DEFAULT 8,
  loop INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS animation_frames (
  id TEXT PRIMARY KEY,
  animation_id TEXT NOT NULL REFERENCES animations(id) ON DELETE CASCADE,
  frame_index INTEGER NOT NULL,
  image_path TEXT NOT NULL,
  duration_ms INTEGER NOT NULL DEFAULT 100,
  created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS styles (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  config TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS providers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  base_url TEXT NOT NULL DEFAULT '',
  model TEXT NOT NULL DEFAULT '',
  image_model TEXT NOT NULL DEFAULT '',
  request_format TEXT NOT NULL DEFAULT 'openai',
  headers TEXT NOT NULL DEFAULT '{}',
  is_default INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS generation_tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT,
  provider_id TEXT NOT NULL,
  model TEXT NOT NULL DEFAULT '',
  kind TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'queued',
  progress REAL NOT NULL DEFAULT 0,
  prompt TEXT NOT NULL DEFAULT '',
  params TEXT NOT NULL DEFAULT '{}',
  error TEXT,
  asset_id TEXT,
  created_at TEXT NOT NULL,
  started_at TEXT,
  finished_at TEXT,
  elapsed_ms INTEGER
);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON generation_tasks(status);
CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
"#;

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

fn row_to_project(row: &Row) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get("id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        path: row.get("path")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        last_opened_at: row.get("last_opened_at")?,
        asset_count: row.get("asset_count")?,
    })
}

const PROJECT_SELECT: &str = "SELECT p.*, (SELECT COUNT(*) FROM assets a WHERE a.project_id = p.id) AS asset_count FROM projects p";

impl Db {
    pub fn list_projects(&self) -> AppResult<Vec<Project>> {
        self.with(|conn| {
            let mut stmt = conn.prepare(&format!("{PROJECT_SELECT} ORDER BY p.last_opened_at DESC, p.created_at DESC"))?;
            let list = stmt
                .query_map([], row_to_project)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }

    pub fn get_project(&self, id: &str) -> AppResult<Project> {
        self.with(|conn| {
            conn.query_row(
                &format!("{PROJECT_SELECT} WHERE p.id = ?1"),
                params![id],
                row_to_project,
            )
            .optional()?
            .ok_or_else(|| AppError::user("项目不存在或已被删除。"))
        })
    }

    pub fn insert_project(&self, p: &Project) -> AppResult<()> {
        self.with(|conn| {
            conn.execute(
                "INSERT INTO projects (id, name, description, path, created_at, updated_at, last_opened_at) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![p.id, p.name, p.description, p.path, p.created_at, p.updated_at, p.last_opened_at],
            )?;
            Ok(())
        })
    }

    pub fn rename_project(&self, id: &str, name: &str) -> AppResult<()> {
        self.with(|conn| {
            let n = conn.execute(
                "UPDATE projects SET name = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, name, now_iso()],
            )?;
            if n == 0 {
                return Err(AppError::user("项目不存在或已被删除。"));
            }
            Ok(())
        })
    }

    pub fn touch_project(&self, id: &str) -> AppResult<()> {
        self.with(|conn| {
            conn.execute(
                "UPDATE projects SET last_opened_at = ?2 WHERE id = ?1",
                params![id, now_iso()],
            )?;
            Ok(())
        })
    }

    pub fn delete_project(&self, id: &str) -> AppResult<()> {
        self.with(|conn| {
            conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// Assets
// ---------------------------------------------------------------------------

fn row_to_asset(row: &Row) -> rusqlite::Result<Asset> {
    let tags_raw: String = row.get("tags")?;
    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
    Ok(Asset {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        name: row.get("name")?,
        asset_type: row.get("type")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        image_path: row.get("image_path")?,
        thumbnail_path: row.get("thumbnail_path")?,
        width: row.get("width")?,
        height: row.get("height")?,
        style_id: row.get("style_id")?,
        provider_id: row.get("provider_id")?,
        model: row.get("model")?,
        prompt: row.get("prompt")?,
        tags,
        current_version: row.get("current_version")?,
    })
}

const ASSET_SELECT: &str = "SELECT * FROM assets";

fn row_to_version(row: &Row) -> rusqlite::Result<AssetVersion> {
    Ok(AssetVersion {
        id: row.get("id")?,
        asset_id: row.get("asset_id")?,
        version: row.get("version")?,
        image_path: row.get("image_path")?,
        thumbnail_path: row.get("thumbnail_path")?,
        width: row.get("width")?,
        height: row.get("height")?,
        prompt: row.get("prompt")?,
        provider_id: row.get("provider_id")?,
        model: row.get("model")?,
        note: row.get("note")?,
        created_at: row.get("created_at")?,
    })
}

#[derive(Debug, Clone)]
pub struct NewAsset {
    pub project_id: String,
    pub name: String,
    pub asset_type: String,
    pub image_path: String,
    pub thumbnail_path: Option<String>,
    pub width: i64,
    pub height: i64,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub version_note: String,
}

impl Db {
    pub fn list_assets(&self, project_id: &str) -> AppResult<Vec<Asset>> {
        self.with(|conn| {
            let mut stmt = conn.prepare(&format!("{ASSET_SELECT} WHERE project_id = ?1 ORDER BY created_at DESC"))?;
            let list = stmt
                .query_map(params![project_id], row_to_asset)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }

    pub fn get_asset(&self, id: &str) -> AppResult<Asset> {
        self.with(|conn| {
            conn.query_row(&format!("{ASSET_SELECT} WHERE id = ?1"), params![id], row_to_asset)
                .optional()?
                .ok_or_else(|| AppError::user("资产不存在或已被删除。"))
        })
    }

    pub fn get_asset_detail(&self, asset_id: &str) -> AppResult<AssetDetail> {
        let asset = self.get_asset(asset_id)?;
        let versions = self.with(|conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM asset_versions WHERE asset_id = ?1 ORDER BY version ASC",
            )?;
            let list = stmt
                .query_map(params![asset_id], row_to_version)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })?;
        Ok(AssetDetail { asset, versions })
    }

    /// 插入资产 + 首个版本（v1），返回资产 id
    pub fn insert_asset_with_version(&self, new: &NewAsset) -> AppResult<String> {
        let asset_id = new_id();
        let version_id = new_id();
        let now = now_iso();
        self.with(|conn| {
            conn.execute(
                "INSERT INTO assets (id, project_id, name, type, created_at, updated_at, image_path, thumbnail_path, width, height, provider_id, model, prompt, tags, current_version)
                 VALUES (?1,?2,?3,?4,?5,?5,?6,?7,?8,?9,?10,?11,?12,'[]',1)",
                params![
                    asset_id,
                    new.project_id,
                    new.name,
                    new.asset_type,
                    now,
                    new.image_path,
                    new.thumbnail_path,
                    new.width,
                    new.height,
                    new.provider_id,
                    new.model,
                    new.prompt
                ],
            )?;
            conn.execute(
                "INSERT INTO asset_versions (id, asset_id, version, image_path, thumbnail_path, width, height, prompt, provider_id, model, note, created_at)
                 VALUES (?1,?2,1,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    version_id,
                    asset_id,
                    new.image_path,
                    new.thumbnail_path,
                    new.width,
                    new.height,
                    new.prompt,
                    new.provider_id,
                    new.model,
                    new.version_note,
                    now
                ],
            )?;
            Ok(())
        })?;
        Ok(asset_id)
    }

    pub fn update_asset_meta(&self, asset_id: &str, name: &str, tags: &[String]) -> AppResult<()> {
        self.with(|conn| {
            let n = conn.execute(
                "UPDATE assets SET name = ?2, tags = ?3, updated_at = ?4 WHERE id = ?1",
                params![asset_id, name, serde_json::to_string(tags).unwrap_or_default(), now_iso()],
            )?;
            if n == 0 {
                return Err(AppError::user("资产不存在或已被删除。"));
            }
            Ok(())
        })
    }

    pub fn delete_asset(&self, asset_id: &str) -> AppResult<Asset> {
        let asset = self.get_asset(asset_id)?;
        self.with(|conn| {
            conn.execute("DELETE FROM assets WHERE id = ?1", params![asset_id])?;
            Ok(())
        })?;
        Ok(asset)
    }

    pub fn list_asset_files(&self, asset_id: &str) -> AppResult<Vec<String>> {
        self.with(|conn| {
            let mut stmt = conn.prepare("SELECT image_path, COALESCE(thumbnail_path,'') FROM asset_versions WHERE asset_id = ?1")?;
            let rows = stmt.query_map(params![asset_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?;
            let mut files = vec![];
            for r in rows {
                let (img, thumb) = r?;
                files.push(img);
                if !thumb.is_empty() {
                    files.push(thumb);
                }
            }
            Ok(files)
        })
    }

    pub fn get_asset_version(&self, version_id: &str) -> AppResult<AssetVersion> {
        self.with(|conn| {
            conn.query_row("SELECT * FROM asset_versions WHERE id = ?1", params![version_id], row_to_version)
                .optional()?
                .ok_or_else(|| AppError::user("资产版本不存在或已被删除。"))
        })
    }

    /// 以某个旧版本为基础创建新版本（Restore：不覆盖任何历史）
    pub fn restore_version(&self, version: &AssetVersion, note: &str) -> AppResult<AssetDetail> {
        let asset = self.get_asset(&version.asset_id)?;
        let new_version = asset.current_version + 1;
        let vid = new_id();
        let now = now_iso();
        self.with(|conn| {
            conn.execute(
                "INSERT INTO asset_versions (id, asset_id, version, image_path, thumbnail_path, width, height, prompt, provider_id, model, note, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    vid,
                    version.asset_id,
                    new_version,
                    version.image_path,
                    version.thumbnail_path,
                    version.width,
                    version.height,
                    version.prompt,
                    version.provider_id,
                    version.model,
                    note,
                    now
                ],
            )?;
            conn.execute(
                "UPDATE assets SET current_version = ?2, image_path = ?3, thumbnail_path = ?4, width = ?5, height = ?6, updated_at = ?7 WHERE id = ?1",
                params![version.asset_id, new_version, version.image_path, version.thumbnail_path, version.width, version.height, now],
            )?;
            Ok(())
        })?;
        self.get_asset_detail(&version.asset_id)
    }

    pub fn delete_asset_version(&self, version_id: &str) -> AppResult<AssetDetail> {
        let version = self.get_asset_version(version_id)?;
        let detail_before = self.get_asset_detail(&version.asset_id)?;
        if detail_before.versions.len() <= 1 {
            return Err(AppError::user("至少保留一个版本，无法删除。"));
        }
        self.with(|conn| {
            conn.execute("DELETE FROM asset_versions WHERE id = ?1", params![version_id])?;
            // 若删除的是当前版本，回退到最高版本
            let remaining: Option<(i64, String, Option<String>, i64, i64)> = conn
                .query_row(
                    "SELECT version, image_path, thumbnail_path, width, height FROM asset_versions WHERE asset_id = ?1 ORDER BY version DESC LIMIT 1",
                    params![version.asset_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )
                .optional()?;
            if let Some((v, img, thumb, w, h)) = remaining {
                conn.execute(
                    "UPDATE assets SET current_version = ?2, image_path = ?3, thumbnail_path = ?4, width = ?5, height = ?6, updated_at = ?7 WHERE id = ?1 AND current_version = ?8",
                    params![version.asset_id, v, img, thumb, w, h, now_iso(), version.version],
                )?;
            }
            Ok(())
        })?;
        self.get_asset_detail(&version.asset_id)
    }
}

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

fn parse_headers(raw: &str) -> Vec<(String, String)> {
    serde_json::from_str::<Vec<HeaderPair>>(raw)
        .map(|pairs| pairs.into_iter().map(|p| (p.key, p.value)).collect())
        .unwrap_or_default()
}

fn row_to_provider(row: &Row) -> rusqlite::Result<ProviderRow> {
    let headers_raw: String = row.get("headers")?;
    Ok(ProviderRow {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: row.get("kind")?,
        base_url: row.get("base_url")?,
        model: row.get("model")?,
        image_model: row.get("image_model")?,
        request_format: row.get("request_format")?,
        headers: parse_headers(&headers_raw),
        is_default: row.get::<_, i64>("is_default")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

impl Db {
    pub fn list_providers(&self) -> AppResult<Vec<ProviderRow>> {
        self.with(|conn| {
            let mut stmt = conn.prepare("SELECT * FROM providers ORDER BY created_at ASC")?;
            let list = stmt.query_map([], row_to_provider)?.collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }

    pub fn get_provider_row(&self, id: &str) -> AppResult<ProviderRow> {
        self.with(|conn| {
            conn.query_row("SELECT * FROM providers WHERE id = ?1", params![id], row_to_provider)
                .optional()?
                .ok_or_else(|| AppError::user("Provider 不存在或已被删除。"))
        })
    }

    pub fn upsert_provider(&self, row: &ProviderRow) -> AppResult<()> {
        let headers_json = serde_json::to_string(
            &row.headers
                .iter()
                .map(|(k, v)| HeaderPair { key: k.clone(), value: v.clone() })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".into());
        self.with(|conn| {
            conn.execute(
                "INSERT INTO providers (id, name, kind, base_url, model, image_model, request_format, headers, is_default, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)
                 ON CONFLICT(id) DO UPDATE SET name=?2, kind=?3, base_url=?4, model=?5, image_model=?6, request_format=?7, headers=?8, is_default=?9, updated_at=?10",
                params![row.id, row.name, row.kind, row.base_url, row.model, row.image_model, row.request_format, headers_json, row.is_default as i64, now_iso()],
            )?;
            Ok(())
        })
    }

    pub fn delete_provider(&self, id: &str) -> AppResult<()> {
        self.with(|conn| {
            conn.execute("DELETE FROM providers WHERE id = ?1", params![id])?;
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// Generation Tasks
// ---------------------------------------------------------------------------

fn row_to_task(row: &Row) -> rusqlite::Result<GenerationTask> {
    let params_raw: String = row.get("params")?;
    Ok(GenerationTask {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        provider_id: row.get("provider_id")?,
        model: row.get("model")?,
        kind: row.get("kind")?,
        status: row.get("status")?,
        progress: row.get("progress")?,
        prompt: row.get("prompt")?,
        params: serde_json::from_str(&params_raw).unwrap_or(serde_json::Value::Null),
        error: row.get("error")?,
        asset_id: row.get("asset_id")?,
        animation_id: row.get("animation_id")?,
        created_at: row.get("created_at")?,
        started_at: row.get("started_at")?,
        finished_at: row.get("finished_at")?,
        elapsed_ms: row.get("elapsed_ms")?,
    })
}

impl Db {
    pub fn insert_task(&self, task: &GenerationTask) -> AppResult<()> {
        self.with(|conn| {
            conn.execute(
                "INSERT INTO generation_tasks (id, project_id, provider_id, model, kind, status, progress, prompt, params, error, asset_id, animation_id, created_at, started_at, finished_at, elapsed_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
                params![
                    task.id,
                    task.project_id,
                    task.provider_id,
                    task.model,
                    task.kind,
                    task.status,
                    task.progress,
                    task.prompt,
                    task.params.to_string(),
                    task.error,
                    task.asset_id,
                    task.animation_id,
                    task.created_at,
                    task.started_at,
                    task.finished_at,
                    task.elapsed_ms
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_task(&self, id: &str) -> AppResult<GenerationTask> {
        self.with(|conn| {
            conn.query_row("SELECT * FROM generation_tasks WHERE id = ?1", params![id], row_to_task)
                .optional()?
                .ok_or_else(|| AppError::user("任务不存在或已被清除。"))
        })
    }

    pub fn list_tasks(&self, limit: i64) -> AppResult<Vec<GenerationTask>> {
        self.with(|conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM generation_tasks ORDER BY created_at DESC LIMIT ?1",
            )?;
            let list = stmt.query_map(params![limit], row_to_task)?.collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }

    pub fn update_task_progress(&self, id: &str, status: &str, progress: f64) -> AppResult<()> {
        self.with(|conn| {
            conn.execute(
                "UPDATE generation_tasks SET status = ?2, progress = ?3, started_at = COALESCE(started_at, ?4) WHERE id = ?1",
                params![id, status, progress, now_iso()],
            )?;
            Ok(())
        })
    }

    pub fn set_task_prompt(&self, id: &str, prompt: &str) -> AppResult<()> {
        self.with(|conn| {
            conn.execute("UPDATE generation_tasks SET prompt = ?2 WHERE id = ?1", params![id, prompt])?;
            Ok(())
        })
    }

    pub fn finish_task(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
        asset_id: Option<&str>,
        animation_id: Option<&str>,
    ) -> AppResult<()> {
        let now = now_iso();
        self.with(|conn| {
            let started: Option<String> = conn
                .query_row("SELECT started_at FROM generation_tasks WHERE id = ?1", params![id], |r| r.get(0))
                .optional()?
                .flatten();
            let elapsed = started.as_deref().and_then(|s| {
                let s = chrono::DateTime::parse_from_rfc3339(s).ok()?;
                let e = chrono::DateTime::parse_from_rfc3339(&now).ok()?;
                Some((e - s).num_milliseconds())
            });
            conn.execute(
                "UPDATE generation_tasks SET status = ?2, progress = CASE WHEN ?2 = 'success' THEN 1 ELSE progress END, error = ?3, asset_id = COALESCE(?4, asset_id), animation_id = COALESCE(?5, animation_id), finished_at = ?6, elapsed_ms = ?7 WHERE id = ?1",
                params![id, status, error, asset_id, animation_id, now, elapsed],
            )?;
            Ok(())
        })
    }

    pub fn clear_finished_tasks(&self) -> AppResult<()> {
        self.with(|conn| {
            conn.execute(
                "DELETE FROM generation_tasks WHERE status IN ('success','failed','cancelled')",
                [],
            )?;
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

impl Db {
    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        self.with(|conn| {
            Ok(conn
                .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0))
                .optional()?)
        })
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        self.with(|conn| {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
                params![key, value],
            )?;
            Ok(())
        })
    }

    pub fn all_settings(&self) -> AppResult<Vec<(String, String)>> {
        self.with(|conn| {
            let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
            let list = stmt
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }
}

// ---------------------------------------------------------------------------
// Animations
// ---------------------------------------------------------------------------

const ANIMATION_SELECT: &str = "SELECT a.*, (SELECT COUNT(*) FROM animation_frames f WHERE f.animation_id = a.id) AS frame_count FROM animations a";

fn row_to_animation(row: &Row) -> rusqlite::Result<AnimationView> {
    Ok(AnimationView {
        id: row.get("id")?,
        project_id: row.get("project_id")?,
        name: row.get("name")?,
        anim_type: row.get("type")?,
        fps: row.get("fps")?,
        is_loop: row.get::<_, i64>("loop")? != 0,
        source_asset_id: row.get("source_asset_id")?,
        prompt: row.get("prompt")?,
        resolution: row.get("resolution")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        frame_count: row.get("frame_count")?,
    })
}

fn row_to_frame(row: &Row) -> rusqlite::Result<AnimationFrame> {
    Ok(AnimationFrame {
        id: row.get("id")?,
        animation_id: row.get("animation_id")?,
        frame_index: row.get("frame_index")?,
        image_path: row.get("image_path")?,
        duration_ms: row.get("duration_ms")?,
        width: row.get("width")?,
        height: row.get("height")?,
        created_at: row.get("created_at")?,
    })
}

impl Db {
    pub fn list_animations(&self, project_id: &str) -> AppResult<Vec<AnimationView>> {
        self.with(|conn| {
            let mut stmt = conn.prepare(&format!(
                "{ANIMATION_SELECT} WHERE a.project_id = ?1 ORDER BY a.created_at DESC"
            ))?;
            let list = stmt
                .query_map(params![project_id], row_to_animation)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }

    pub fn get_animation(&self, id: &str) -> AppResult<AnimationView> {
        self.with(|conn| {
            conn.query_row(
                &format!("{ANIMATION_SELECT} WHERE a.id = ?1"),
                params![id],
                row_to_animation,
            )
            .optional()?
            .ok_or_else(|| AppError::user("动画不存在或已被删除。"))
        })
    }

    pub fn get_animation_detail(&self, id: &str) -> AppResult<AnimationDetail> {
        let animation = self.get_animation(id)?;
        let frames = self.list_frames(id)?;
        Ok(AnimationDetail { animation, frames })
    }

    pub fn create_animation(&self, new: &NewAnimation) -> AppResult<String> {
        let id = new_id();
        let now = now_iso();
        self.with(|conn| {
            conn.execute(
                "INSERT INTO animations (id, project_id, name, type, fps, loop, source_asset_id, prompt, resolution, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
                params![
                    id,
                    new.project_id,
                    new.name,
                    new.anim_type,
                    new.fps,
                    new.is_loop as i64,
                    new.source_asset_id,
                    new.prompt,
                    new.resolution,
                    now
                ],
            )?;
            Ok(())
        })?;
        Ok(id)
    }

    pub fn update_animation_meta(
        &self,
        id: &str,
        name: &str,
        fps: i64,
        is_loop: bool,
    ) -> AppResult<()> {
        self.with(|conn| {
            let n = conn.execute(
                "UPDATE animations SET name = ?2, fps = ?3, loop = ?4, updated_at = ?5 WHERE id = ?1",
                params![id, name, fps, is_loop as i64, now_iso()],
            )?;
            if n == 0 {
                return Err(AppError::user("动画不存在或已被删除。"));
            }
            Ok(())
        })
    }

    pub fn delete_animation(&self, id: &str) -> AppResult<Vec<String>> {
        let frames = self.list_frames(id)?;
        self.with(|conn| {
            conn.execute("DELETE FROM animations WHERE id = ?1", params![id])?;
            Ok(())
        })?;
        Ok(frames.iter().map(|f| f.image_path.clone()).collect())
    }

    pub fn list_frames(&self, animation_id: &str) -> AppResult<Vec<AnimationFrame>> {
        self.with(|conn| {
            let mut stmt = conn.prepare(
                "SELECT * FROM animation_frames WHERE animation_id = ?1 ORDER BY frame_index ASC",
            )?;
            let list = stmt
                .query_map(params![animation_id], row_to_frame)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(list)
        })
    }

    pub fn get_frame(&self, frame_id: &str) -> AppResult<AnimationFrame> {
        self.with(|conn| {
            conn.query_row(
                "SELECT * FROM animation_frames WHERE id = ?1",
                params![frame_id],
                row_to_frame,
            )
            .optional()?
            .ok_or_else(|| AppError::user("动画帧不存在或已被删除。"))
        })
    }

    /// 在指定索引插入帧（其后的帧顺延）
    pub fn insert_frame_at(
        &self,
        animation_id: &str,
        index: i64,
        image_path: &str,
        duration_ms: i64,
        width: i64,
        height: i64,
    ) -> AppResult<String> {
        let fid = new_id();
        self.with(|conn| {
            conn.execute(
                "UPDATE animation_frames SET frame_index = frame_index + 1 WHERE animation_id = ?1 AND frame_index >= ?2",
                params![animation_id, index],
            )?;
            conn.execute(
                "INSERT INTO animation_frames (id, animation_id, frame_index, image_path, duration_ms, width, height, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![fid, animation_id, index, image_path, duration_ms, width, height, now_iso()],
            )?;
            Ok(())
        })?;
        Ok(fid)
    }

    pub fn append_frame(
        &self,
        animation_id: &str,
        image_path: &str,
        duration_ms: i64,
        width: i64,
        height: i64,
    ) -> AppResult<String> {
        let next: i64 = self.with(|conn| {
            Ok(conn
                .query_row(
                    "SELECT COALESCE(MAX(frame_index) + 1, 0) FROM animation_frames WHERE animation_id = ?1",
                    params![animation_id],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or(0))
        })?;
        self.insert_frame_at(animation_id, next, image_path, duration_ms, width, height)
    }

    pub fn delete_frame(&self, frame_id: &str) -> AppResult<()> {
        let frame = self.get_frame(frame_id)?;
        self.with(|conn| {
            conn.execute("DELETE FROM animation_frames WHERE id = ?1", params![frame_id])?;
            conn.execute(
                "UPDATE animation_frames SET frame_index = frame_index - 1 WHERE animation_id = ?1 AND frame_index > ?2",
                params![frame.animation_id, frame.frame_index],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    /// 更新单帧时长（ms），用于逐帧变速（参考 Aseprite 帧时长）
    pub fn update_frame_duration(&self, frame_id: &str, duration_ms: i64) -> AppResult<()> {
        self.with(|conn| {
            let n = conn.execute(
                "UPDATE animation_frames SET duration_ms = ?2 WHERE id = ?1",
                params![frame_id, duration_ms.max(10)],
            )?;
            if n == 0 {
                return Err(AppError::user("动画帧不存在或已被删除。"));
            }
            Ok(())
        })
    }

    pub fn move_frame(&self, animation_id: &str, from_index: i64, to_index: i64) -> AppResult<()> {
        self.with(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM animation_frames WHERE animation_id = ?1 ORDER BY frame_index ASC",
            )?;
            let mut ids: Vec<String> = stmt
                .query_map(params![animation_id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            if from_index < 0 || from_index as usize >= ids.len() {
                return Err(AppError::user("帧序号超出范围。"));
            }
            let moved = ids.remove(from_index as usize);
            let to = to_index.clamp(0, ids.len() as i64) as usize;
            ids.insert(to, moved);
            for (i, id) in ids.iter().enumerate() {
                conn.execute(
                    "UPDATE animation_frames SET frame_index = ?2 WHERE id = ?1",
                    params![id, i as i64],
                )?;
            }
            Ok(())
        })
    }

    /// 重建帧索引连续性
    pub fn normalize_frame_indexes(&self, animation_id: &str) -> AppResult<()> {
        self.with(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM animation_frames WHERE animation_id = ?1 ORDER BY frame_index ASC",
            )?;
            let ids: Vec<String> = stmt
                .query_map(params![animation_id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for (i, id) in ids.iter().enumerate() {
                conn.execute(
                    "UPDATE animation_frames SET frame_index = ?2 WHERE id = ?1",
                    params![id, i as i64],
                )?;
            }
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// Dashboard 统计
// ---------------------------------------------------------------------------

impl Db {
    pub fn dashboard_stats(&self, project_id: Option<&str>) -> AppResult<DashboardStats> {
        let mut stats = DashboardStats::default();
        self.with(|conn| {
            if let Some(pid) = project_id {
                stats.total_assets = conn.query_row(
                    "SELECT COUNT(*) FROM assets WHERE project_id = ?1",
                    params![pid],
                    |r| r.get(0),
                )?;
                let mut stmt = conn.prepare(
                    "SELECT type, COUNT(*) FROM assets WHERE project_id = ?1 GROUP BY type ORDER BY COUNT(*) DESC",
                )?;
                stats.by_type = stmt
                    .query_map(params![pid], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                stats.total_animations = conn.query_row(
                    "SELECT COUNT(*) FROM animations WHERE project_id = ?1",
                    params![pid],
                    |r| r.get(0),
                )?;
                let mut recent = conn.prepare(&format!("{ASSET_SELECT} WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 8"))?;
                stats.recent_assets = recent
                    .query_map(params![pid], row_to_asset)?
                    .collect::<Result<Vec<_>, _>>()?;
            } else {
                stats.total_assets = conn.query_row("SELECT COUNT(*) FROM assets", [], |r| r.get(0))?;
                let mut stmt = conn.prepare("SELECT type, COUNT(*) FROM assets GROUP BY type ORDER BY COUNT(*) DESC")?;
                stats.by_type = stmt
                    .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
                    .collect::<Result<Vec<_>, _>>()?;
                stats.total_animations = conn.query_row("SELECT COUNT(*) FROM animations", [], |r| r.get(0))?;
                let mut recent = conn.prepare(&format!("{ASSET_SELECT} ORDER BY created_at DESC LIMIT 8"))?;
                stats.recent_assets = recent.query_map([], row_to_asset)?.collect::<Result<Vec<_>, _>>()?;
            }
            stats.total_tasks = conn.query_row("SELECT COUNT(*) FROM generation_tasks", [], |r| r.get(0))?;
            stats.failed_tasks = conn.query_row(
                "SELECT COUNT(*) FROM generation_tasks WHERE status = 'failed'",
                [],
                |r| r.get(0),
            )?;
            let mut tasks = conn.prepare("SELECT * FROM generation_tasks ORDER BY created_at DESC LIMIT 6")?;
            stats.recent_tasks = tasks.query_map([], row_to_task)?.collect::<Result<Vec<_>, _>>()?;
            Ok(())
        })?;
        Ok(stats)
    }
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_db() -> Db {
        let db = Db::open_in_memory().unwrap();
        migrate(&db).unwrap();
        db
    }

    fn sample_project(name: &str, path: &str) -> Project {
        Project {
            id: new_id(),
            name: name.into(),
            description: "测试项目".into(),
            path: path.into(),
            created_at: now_iso(),
            updated_at: now_iso(),
            last_opened_at: None,
            asset_count: 0,
        }
    }

    #[test]
    fn project_crud_roundtrip() {
        let db = mem_db();
        let p = sample_project("My RPG", "K:/tmp/my-rpg");
        db.insert_project(&p).unwrap();
        let list = db.list_projects().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "My RPG");

        db.rename_project(&p.id, "My RPG II").unwrap();
        assert_eq!(db.get_project(&p.id).unwrap().name, "My RPG II");

        db.touch_project(&p.id).unwrap();
        assert!(db.get_project(&p.id).unwrap().last_opened_at.is_some());

        db.delete_project(&p.id).unwrap();
        assert!(db.get_project(&p.id).is_err());
    }

    #[test]
    fn asset_versions_and_restore() {
        let db = mem_db();
        let p = sample_project("P", "K:/tmp/p");
        db.insert_project(&p).unwrap();

        let asset_id = db
            .insert_asset_with_version(&NewAsset {
                project_id: p.id.clone(),
                name: "Knight".into(),
                asset_type: "character".into(),
                image_path: "K:/tmp/p/assets/characters/knight.png".into(),
                thumbnail_path: Some("K:/tmp/p/thumbnails/knight.png".into()),
                width: 64,
                height: 64,
                provider_id: Some("prov".into()),
                model: Some("gpt-image-1".into()),
                prompt: "a knight".into(),
                version_note: "初版".into(),
            })
            .unwrap();

        let detail = db.get_asset_detail(&asset_id).unwrap();
        assert_eq!(detail.versions.len(), 1);
        assert_eq!(detail.asset.current_version, 1);

        // restore v1 -> 创建 v2，不覆盖 v1
        let v1 = detail.versions[0].clone();
        let after = db.restore_version(&v1, "还原到 v1").unwrap();
        assert_eq!(after.versions.len(), 2);
        assert_eq!(after.asset.current_version, 2);
        assert_eq!(after.asset.image_path, v1.image_path);

        // 删除一个版本后版本号回退
        let detail2 = db.get_asset_detail(&asset_id).unwrap();
        let v2_id = detail2.versions[1].id.clone();
        let after_del = db.delete_asset_version(&v2_id).unwrap();
        assert_eq!(after_del.versions.len(), 1);
        assert_eq!(after_del.asset.current_version, 1);
    }

    #[test]
    fn provider_upsert_and_delete() {
        let db = mem_db();
        let row = ProviderRow {
            id: "prov1".into(),
            name: "My OpenAI".into(),
            kind: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            image_model: "gpt-image-1".into(),
            request_format: "openai".into(),
            headers: vec![("X-Custom".into(), "1".into())],
            is_default: true,
            created_at: now_iso(),
            updated_at: now_iso(),
        };
        db.upsert_provider(&row).unwrap();
        let got = db.get_provider_row("prov1").unwrap();
        assert_eq!(got.headers, vec![("X-Custom".to_string(), "1".to_string())]);

        let mut updated = row.clone();
        updated.name = "Renamed".into();
        db.upsert_provider(&updated).unwrap();
        assert_eq!(db.list_providers().unwrap().len(), 1);
        assert_eq!(db.get_provider_row("prov1").unwrap().name, "Renamed");

        db.delete_provider("prov1").unwrap();
        assert!(db.get_provider_row("prov1").is_err());
    }

    #[test]
    fn task_lifecycle() {
        let db = mem_db();
        let task = GenerationTask {
            id: "t1".into(),
            project_id: Some("p1".into()),
            provider_id: "prov".into(),
            model: "gpt-image-1".into(),
            kind: "generate_image".into(),
            status: "queued".into(),
            progress: 0.0,
            prompt: "".into(),
            params: serde_json::json!({"assetType": "character"}),
            error: None,
            asset_id: None,
            animation_id: None,
            created_at: now_iso(),
            started_at: None,
            finished_at: None,
            elapsed_ms: None,
        };
        db.insert_task(&task).unwrap();
        db.update_task_progress("t1", "running", 0.4).unwrap();
        db.set_task_prompt("t1", "pixel knight").unwrap();
        db.finish_task("t1", "success", None, Some("asset1"), None).unwrap();

        let t = db.get_task("t1").unwrap();
        assert_eq!(t.status, "success");
        assert_eq!(t.prompt, "pixel knight");
        assert_eq!(t.asset_id.as_deref(), Some("asset1"));
        assert!(t.elapsed_ms.is_some());

        db.clear_finished_tasks().unwrap();
        assert!(db.get_task("t1").is_err());
    }

    #[test]
    fn settings_roundtrip() {
        let db = mem_db();
        assert_eq!(db.get_setting("project_dir").unwrap(), None);
        db.set_setting("project_dir", "K:/projects").unwrap();
        db.set_setting("project_dir", "K:/projects2").unwrap();
        assert_eq!(db.get_setting("project_dir").unwrap().as_deref(), Some("K:/projects2"));
    }

    #[test]
    fn animation_frame_lifecycle() {
        let db = mem_db();
        let p = sample_project("AnimP", "K:/tmp/animp");
        db.insert_project(&p).unwrap();

        let anim_id = db
            .create_animation(&NewAnimation {
                project_id: p.id.clone(),
                name: "Knight Walk".into(),
                anim_type: "walk".into(),
                fps: 8,
                is_loop: true,
                source_asset_id: Some("asset-knight".into()),
                prompt: "walk cycle".into(),
                resolution: 64,
            })
            .unwrap();

        // 追加 4 帧
        for i in 0..4 {
            db.append_frame(&anim_id, &format!("K:/tmp/frame_{i}.png"), 125, 64, 64)
                .unwrap();
        }
        let detail = db.get_animation_detail(&anim_id).unwrap();
        assert_eq!(detail.animation.frame_count, 4);
        assert_eq!(detail.animation.name, "Knight Walk");
        assert!(detail.animation.is_loop);
        assert_eq!(detail.frames.iter().map(|f| f.frame_index).collect::<Vec<_>>(), vec![0, 1, 2, 3]);

        // 移动帧：3 → 0
        db.move_frame(&anim_id, 3, 0).unwrap();
        let frames = db.list_frames(&anim_id).unwrap();
        assert_eq!(frames[0].frame_index, 0);
        assert_eq!(frames.len(), 4);

        // 删除一帧后索引连续
        let victim = frames[1].id.clone();
        db.delete_frame(&victim).unwrap();
        let frames = db.list_frames(&anim_id).unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames.iter().map(|f| f.frame_index).collect::<Vec<_>>(), vec![0, 1, 2]);

        // 更新元数据
        db.update_animation_meta(&anim_id, "Knight Walk v2", 12, false).unwrap();
        let anim = db.get_animation(&anim_id).unwrap();
        assert_eq!(anim.name, "Knight Walk v2");
        assert_eq!(anim.fps, 12);
        assert!(!anim.is_loop);

        // 列表过滤
        let list = db.list_animations(&p.id).unwrap();
        assert_eq!(list.len(), 1);

        db.delete_animation(&anim_id).unwrap();
        assert!(db.get_animation(&anim_id).is_err());
        assert!(db.list_frames(&anim_id).unwrap().is_empty());
    }

    #[test]
    fn dashboard_stats_counts() {
        let db = mem_db();
        let p = sample_project("S", "K:/tmp/s");
        db.insert_project(&p).unwrap();
        db.insert_asset_with_version(&NewAsset {
            project_id: p.id.clone(),
            name: "A".into(),
            asset_type: "character".into(),
            image_path: "x.png".into(),
            thumbnail_path: None,
            width: 32,
            height: 32,
            provider_id: None,
            model: None,
            prompt: String::new(),
            version_note: String::new(),
        })
        .unwrap();
        let stats = db.dashboard_stats(Some(&p.id)).unwrap();
        assert_eq!(stats.total_assets, 1);
        assert_eq!(stats.by_type[0], ("character".into(), 1));
    }
}
