use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::clipboard::models::{ClipItem, ContentType, Settings};

/// 数据库管理器（线程安全）
pub struct Database {
    pub conn: Mutex<Connection>,
    pub data_dir: PathBuf,
}

impl Database {
    /// 打开（或创建）数据库，自动建表
    pub fn open(data_dir: PathBuf) -> SqliteResult<Self> {
        std::fs::create_dir_all(&data_dir).ok();
        let db_path = data_dir.join("clipboard.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        Self::run_migrations(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
            data_dir,
        })
    }

    /// 创建表结构
    fn run_migrations(conn: &Connection) -> SqliteResult<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clips (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT    NOT NULL,
                text_content TEXT,
                image_path   TEXT,
                image_hash   TEXT,
                created_at   INTEGER NOT NULL,
                is_pinned    INTEGER DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_clips_created_at
                ON clips(created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_clips_is_pinned
                ON clips(is_pinned);

            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            INSERT OR IGNORE INTO settings (key, value) VALUES ('retention_days', '30');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('storage_cap_mb', '500');
            INSERT OR IGNORE INTO settings (key, value) VALUES ('autostart', 'false');"
        )?;
        Ok(())
    }

    // ========== Clips CRUD ==========

    /// 插入新的剪贴板条目，返回自增 ID
    pub fn insert_clip(&self, item: &ClipItem) -> SqliteResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO clips (content_type, text_content, image_path, image_hash, created_at, is_pinned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                item.content_type.as_str(),
                item.text_content,
                item.image_path,
                item.image_hash,
                item.created_at,
                item.is_pinned as i32,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 获取历史列表（置顶优先，按时间降序）
    pub fn get_history(&self, limit: Option<i64>) -> SqliteResult<Vec<ClipItem>> {
        let conn = self.conn.lock().unwrap();
        let limit_val = limit.unwrap_or(500);
        let mut stmt = conn.prepare(
            "SELECT id, content_type, text_content, image_path, created_at, is_pinned
             FROM clips
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit_val], Self::row_to_item)?;
        rows.collect()
    }

    /// 搜索文字内容（大小写不敏感）
    pub fn search_clips(&self, query: &str) -> SqliteResult<Vec<ClipItem>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, content_type, text_content, image_path, created_at, is_pinned
             FROM clips
             WHERE content_type = 'text' AND text_content LIKE ?1
             ORDER BY is_pinned DESC, created_at DESC
             LIMIT 200"
        )?;
        let rows = stmt.query_map([pattern], Self::row_to_item)?;
        rows.collect()
    }

    /// 切换置顶状态
    pub fn toggle_pin(&self, id: i64) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();
        let current: bool = conn.query_row(
            "SELECT is_pinned FROM clips WHERE id = ?1",
            [id],
            |row| row.get::<_, i32>(0),
        ).map(|v| v != 0)?;

        let new_val = if current { 0 } else { 1 };
        conn.execute("UPDATE clips SET is_pinned = ?1 WHERE id = ?2", [new_val, id])?;
        Ok(!current)
    }

    /// 删除单条记录，返回关联的图片路径
    pub fn delete_clip(&self, id: i64) -> SqliteResult<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let image_path: Option<String> = conn.query_row(
            "SELECT image_path FROM clips WHERE id = ?1",
            [id],
            |row| row.get(0),
        ).ok();
        conn.execute("DELETE FROM clips WHERE id = ?1", [id])?;
        Ok(image_path)
    }

    /// 按 ID 获取单条记录
    pub fn get_clip_by_id(&self, id: i64) -> SqliteResult<Option<ClipItem>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, content_type, text_content, image_path, created_at, is_pinned
             FROM clips WHERE id = ?1",
            [id],
            Self::row_to_item,
        ).map(Some).or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            e => Err(e),
        })
    }

    /// 删除过期记录（返回被删除的图片路径列表）
    pub fn cleanup_old_clips(&self, retention_days: u32) -> SqliteResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let cutoff = chrono::Utc::now().timestamp() - (retention_days as i64 * 86400);

        // 先收集要删除的图片路径
        let mut stmt = conn.prepare(
            "SELECT image_path FROM clips
             WHERE is_pinned = 0 AND created_at < ?1 AND content_type = 'image'"
        )?;
        let image_paths: Vec<String> = stmt
            .query_map([cutoff], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();

        conn.execute(
            "DELETE FROM clips WHERE is_pinned = 0 AND created_at < ?1",
            [cutoff],
        )?;

        Ok(image_paths)
    }

    /// 获取存储统计（按条目数量 + 图片文件大小，不使用 DB 文件大小——SQLite 不自动缩容）
    pub fn get_storage_stats(&self) -> SqliteResult<(u64, u64)> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row(
            "SELECT COUNT(*) FROM clips", [], |row| row.get(0),
        )?;
        // 估算文本数据大小
        let text_bytes: u64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(text_content)), 0) FROM clips WHERE content_type = 'text'",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        Ok((count, text_bytes))
    }

    /// 删除最旧的未置顶条目（用于容量上限清理）
    pub fn delete_oldest_unpinned(&self, count: i32) -> SqliteResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        // 先查图片路径，确保 ID 一致
        let mut stmt = conn.prepare(
            "SELECT id, image_path FROM clips WHERE is_pinned = 0
             ORDER BY created_at ASC LIMIT ?1"
        )?;
        let rows: Vec<(i64, Option<String>)> = stmt
            .query_map([count], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
        let image_paths: Vec<String> = rows.into_iter()
            .filter_map(|(_, path)| path)
            .collect();

        if !ids.is_empty() {
            let placeholders: Vec<String> = ids.iter().enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect();
            let sql = format!(
                "DELETE FROM clips WHERE id IN ({})",
                placeholders.join(",")
            );
            let params: Vec<rusqlite::types::Value> = ids.iter()
                .map(|id| rusqlite::types::Value::from(*id))
                .collect();
            conn.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
        }

        Ok(image_paths)
    }

    // ========== Settings ==========

    pub fn get_setting(&self, key: &str) -> SqliteResult<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
    }

    pub fn set_setting(&self, key: &str, value: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> SqliteResult<Settings> {
        let retention: u32 = self.get_setting("retention_days")
            .unwrap_or_default()
            .parse()
            .unwrap_or(30);
        let storage_cap: u32 = self.get_setting("storage_cap_mb")
            .unwrap_or_default()
            .parse()
            .unwrap_or(500);
        let autostart: bool = self.get_setting("autostart")
            .unwrap_or_default()
            .parse()
            .unwrap_or(false);

        Ok(Settings {
            retention_days: retention,
            storage_cap_mb: storage_cap,
            autostart,
        })
    }

    // ========== Helper ==========

    fn row_to_item(row: &rusqlite::Row) -> SqliteResult<ClipItem> {
        let content_type_str: String = row.get(1)?;
        let content_type = ContentType::from_str(&content_type_str)
            .unwrap_or(ContentType::Text); // 容错：未知类型当文字处理
        Ok(ClipItem {
            id: row.get(0)?,
            content_type,
            text_content: row.get(2)?,
            image_path: row.get(3)?,
            image_hash: None,
            created_at: row.get(4)?,
            is_pinned: row.get::<_, i32>(5)? != 0,
        })
    }
}
