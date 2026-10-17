//! SQLite catalog of downloads and settings.
//!
//! Byte-level progress lives in each download's `.part.meta`; this database
//! only remembers which downloads exist and what state they were left in, so
//! the queue can be rebuilt after a restart or crash.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::settings::Settings;
use crate::task::{TaskRecord, TaskState};

const SCHEMA_VERSION: i32 = 2;

#[derive(Debug)]
pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 1 {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS downloads (
                    id              TEXT PRIMARY KEY,
                    url             TEXT NOT NULL,
                    dir             TEXT NOT NULL,
                    file_name       TEXT,
                    path            TEXT,
                    state           TEXT NOT NULL,
                    total           INTEGER,
                    downloaded      INTEGER NOT NULL DEFAULT 0,
                    error           TEXT,
                    error_kind      TEXT,
                    checksum        TEXT,
                    max_connections INTEGER,
                    created_at      INTEGER NOT NULL,
                    updated_at      INTEGER NOT NULL,
                    completed_at    INTEGER
                );
                CREATE INDEX IF NOT EXISTS downloads_state ON downloads(state);
                CREATE TABLE IF NOT EXISTS settings (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );",
            )?;
            conn.pragma_update(None, "user_version", 1)?;
        }
        if version < 2 {
            conn.execute_batch(
                "ALTER TABLE downloads ADD COLUMN category TEXT;
                 ALTER TABLE downloads ADD COLUMN auto_dir INTEGER NOT NULL DEFAULT 0;",
            )?;
            conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        }
        Ok(Self { conn })
    }

    pub fn upsert(&self, r: &TaskRecord) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO downloads (id, url, dir, file_name, path, state, total, downloaded,
                                    error, error_kind, checksum, max_connections,
                                    created_at, updated_at, completed_at, category, auto_dir)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT(id) DO UPDATE SET
                url = excluded.url, dir = excluded.dir, file_name = excluded.file_name,
                path = excluded.path, state = excluded.state, total = excluded.total,
                downloaded = excluded.downloaded, error = excluded.error,
                error_kind = excluded.error_kind, checksum = excluded.checksum,
                max_connections = excluded.max_connections,
                updated_at = excluded.updated_at, completed_at = excluded.completed_at,
                category = excluded.category, auto_dir = excluded.auto_dir",
            params![
                r.id,
                r.url,
                path_str(&r.dir),
                r.file_name,
                r.path.as_deref().map(path_str),
                r.state.as_str(),
                r.total.map(to_i64),
                to_i64(r.downloaded),
                r.error,
                r.error_kind,
                r.checksum,
                r.max_connections,
                r.created_at,
                r.updated_at,
                r.completed_at,
                r.category,
                r.auto_dir,
            ],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: &str) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM downloads WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn all(&self) -> rusqlite::Result<Vec<TaskRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, url, dir, file_name, path, state, total, downloaded, error, error_kind,
                    checksum, max_connections, created_at, updated_at, completed_at,
                    category, auto_dir
             FROM downloads ORDER BY created_at, rowid",
        )?;
        let rows = stmt.query_map([], row_to_record)?;
        rows.collect()
    }

    pub fn load_settings(&self) -> rusqlite::Result<Option<Settings>> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'settings'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
    }

    pub fn save_settings(&self, settings: &Settings) -> rusqlite::Result<()> {
        let json = serde_json::to_string(settings)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES ('settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![json],
        )?;
        Ok(())
    }
}

fn row_to_record(row: &Row<'_>) -> rusqlite::Result<TaskRecord> {
    let state: String = row.get(5)?;
    Ok(TaskRecord {
        id: row.get(0)?,
        url: row.get(1)?,
        dir: PathBuf::from(row.get::<_, String>(2)?),
        file_name: row.get(3)?,
        path: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
        // Unknown states (a newer version wrote them) become Paused, which
        // is always safe to resume from.
        state: TaskState::parse(&state).unwrap_or(TaskState::Paused),
        total: row.get::<_, Option<i64>>(6)?.map(to_u64),
        downloaded: to_u64(row.get(7)?),
        error: row.get(8)?,
        error_kind: row.get(9)?,
        checksum: row.get(10)?,
        max_connections: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        completed_at: row.get(14)?,
        category: row.get(15)?,
        auto_dir: row.get(16)?,
    })
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

fn to_u64(v: i64) -> u64 {
    u64::try_from(v).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, created: i64) -> TaskRecord {
        TaskRecord {
            id: id.into(),
            url: "https://example.org/a.iso".into(),
            dir: PathBuf::from("C:/Downloads"),
            file_name: Some("a.iso".into()),
            path: None,
            state: TaskState::Queued,
            total: Some(10),
            downloaded: 3,
            error: None,
            error_kind: None,
            checksum: None,
            max_connections: Some(4),
            category: Some("Disk images".into()),
            auto_dir: true,
            created_at: created,
            updated_at: created,
            completed_at: None,
        }
    }

    #[test]
    fn upsert_list_delete() {
        let store = Store::in_memory().unwrap();
        let mut a = record("a", 2);
        store.upsert(&a).unwrap();
        store.upsert(&record("b", 1)).unwrap();
        a.state = TaskState::Completed;
        a.path = Some(PathBuf::from("C:/Downloads/a.iso"));
        store.upsert(&a).unwrap();

        let all = store.all().unwrap();
        assert_eq!(
            all.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["b", "a"]
        );
        assert_eq!(all[1], a);

        store.delete("b").unwrap();
        assert_eq!(store.all().unwrap().len(), 1);
    }

    #[test]
    fn settings_round_trip() {
        let store = Store::in_memory().unwrap();
        assert_eq!(store.load_settings().unwrap(), None);
        let s = Settings {
            speed_limit: 99,
            ..Settings::default()
        };
        store.save_settings(&s).unwrap();
        assert_eq!(store.load_settings().unwrap(), Some(s));
    }

    #[test]
    fn reopens_existing_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("downpipe.db");
        Store::open(&path).unwrap().upsert(&record("x", 1)).unwrap();
        assert_eq!(Store::open(&path).unwrap().all().unwrap().len(), 1);
    }

    #[test]
    fn migrates_a_version_1_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("downpipe.db");
        {
            // The 0.1 schema, exactly as it shipped.
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE downloads (
                    id TEXT PRIMARY KEY, url TEXT NOT NULL, dir TEXT NOT NULL, file_name TEXT,
                    path TEXT, state TEXT NOT NULL, total INTEGER,
                    downloaded INTEGER NOT NULL DEFAULT 0, error TEXT, error_kind TEXT,
                    checksum TEXT, max_connections INTEGER, created_at INTEGER NOT NULL,
                    updated_at INTEGER NOT NULL, completed_at INTEGER);
                 CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 INSERT INTO downloads (id, url, dir, state, created_at, updated_at)
                    VALUES ('old', 'https://example.org/a.zip', 'C:/D', 'paused', 1, 1);
                 PRAGMA user_version = 1;",
            )
            .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let old = store.all().unwrap().pop().unwrap();
        assert_eq!(old.id, "old");
        assert_eq!(old.category, None);
        assert!(!old.auto_dir);
        store.upsert(&record("new", 2)).unwrap();
        assert_eq!(store.all().unwrap().len(), 2);
    }
}
