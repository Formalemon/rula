// ============================================================================
// Database - SQLite persistence for app preferences and usage stats
// ============================================================================

use rusqlite::{params, Connection, Result};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let mut path = dirs::data_local_dir().unwrap_or(PathBuf::from("."));
        path.push("rula");
        std::fs::create_dir_all(&path).ok();

        path.push("db.sqlite");
        let conn = Connection::open(path)?;

        // Create table with all needed fields
        conn.execute(
            "CREATE TABLE IF NOT EXISTS app_prefs (
                app_name TEXT PRIMARY KEY,
                is_tui BOOLEAN NOT NULL DEFAULT 0,
                score INTEGER NOT NULL DEFAULT 0,
                usage INTEGER NOT NULL DEFAULT 0,
                last_used INTEGER DEFAULT 0
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Get all app data: (is_tui, score, usage, last_used)
    pub fn get_app_data(&self, app_name: &str) -> (bool, i32, i32, u64) {
        self.conn
            .query_row(
                "SELECT is_tui, score, usage, last_used FROM app_prefs WHERE app_name = ?1",
                params![app_name],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get::<_, i64>(3).unwrap_or(0) as u64,
                    ))
                },
            )
            .unwrap_or((false, 0, 0, 0))
    }

    /// OPTIMIZATION: Batch get all app data in a single query
    /// Returns HashMap<app_name, (is_tui, score, usage, last_used)>
    pub fn get_all_app_data(&self) -> std::collections::HashMap<String, (bool, i32, i32, u64)> {
        let mut stmt = match self.conn.prepare(
            "SELECT app_name, is_tui, score, usage, last_used FROM app_prefs"
        ) {
            Ok(stmt) => stmt,
            Err(_) => return std::collections::HashMap::new(),
        };

        let rows = match stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, bool>(1)?,
                row.get::<_, i32>(2)?,
                row.get::<_, i32>(3)?,
                row.get::<_, i64>(4).unwrap_or(0) as u64,
            ))
        }) {
            Ok(rows) => rows,
            Err(_) => return std::collections::HashMap::new(),
        };

        let mut map = std::collections::HashMap::new();
        for row in rows.flatten() {
            let (name, is_tui, score, usage, last_used) = row;
            map.insert(name, (is_tui, score, usage, last_used));
        }

        map
    }

    /// Increment usage count and update last_used timestamp
    pub fn increment_usage(&self, app_name: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.conn.execute(
            "INSERT INTO app_prefs (app_name, usage, last_used) VALUES (?1, 1, ?2)
             ON CONFLICT(app_name) DO UPDATE SET
                usage = usage + 1,
                last_used = ?2",
            params![app_name, now as i64],
        )?;

        Ok(())
    }

    /// Set TUI mode preference for an app
    pub fn set_tui_mode(&self, app_name: &str, is_tui: bool) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_prefs (app_name, is_tui) VALUES (?1, ?2)
             ON CONFLICT(app_name) DO UPDATE SET is_tui = ?2",
            params![app_name, is_tui],
        )?;

        Ok(())
    }

    /// Set base score for an app (used during seeding)
    pub fn set_base_score(&self, app_name: &str, score: i32) -> Result<()> {
        self.conn.execute(
            "INSERT INTO app_prefs (app_name, score) VALUES (?1, ?2)
             ON CONFLICT(app_name) DO UPDATE SET score = ?2",
            params![app_name, score],
        )?;

        Ok(())
    }

    /// Check if an app has a database entry
    pub fn has_entry(&self, app_name: &str) -> bool {
        let stmt = self
            .conn
            .prepare("SELECT 1 FROM app_prefs WHERE app_name = ?1")
            .ok();

        if let Some(mut stmt) = stmt {
            return stmt.exists(params![app_name]).unwrap_or(false);
        }

        false
    }

    /// Check if an app is marked as TUI
    pub fn is_tui_app(&self, app_name: &str) -> bool {
        let (is_tui, _, _, _) = self.get_app_data(app_name);
        is_tui
    }
}
