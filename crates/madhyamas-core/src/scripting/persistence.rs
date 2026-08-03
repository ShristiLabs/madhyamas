//! SQLite-backed persistence for scripts and execution history.
//!
//! Scripts are stored in the `scripts` table and execution records in the
//! `script_executions` table.  Both tables are created inside the same SQLite
//! database used by [`crate::traffic::TrafficStore`] (`~/.madhyamas/traffic.db`
//! by default), so a single database connection serves all persistence needs.

use rusqlite::{params, Connection};

use super::runtime::{Script, ScriptErrorPolicy, ScriptExecution};

/// Persist and load scripts from SQLite.
pub struct ScriptPersistence;

impl ScriptPersistence {
    /// Create the `scripts` and `script_executions` tables if they do not
    /// already exist.  Safe to call on every startup.  Also runs migrations
    /// to add the `match_json` and `on_error` columns to pre-existing
    /// `scripts` tables.
    pub fn create_tables(conn: &Connection) -> crate::Result<()> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS scripts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                source TEXT NOT NULL,
                hooks TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                created_at TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                match_json TEXT,
                on_error TEXT NOT NULL DEFAULT 'stop_chain'
            );

            CREATE TABLE IF NOT EXISTS script_executions (
                id TEXT PRIMARY KEY,
                script_id TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                success INTEGER NOT NULL,
                error TEXT,
                console TEXT,
                timestamp TEXT NOT NULL,
                traffic_entry_id TEXT,
                hook TEXT,
                FOREIGN KEY (script_id) REFERENCES scripts(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_script_exec_script
                ON script_executions(script_id);
            CREATE INDEX IF NOT EXISTS idx_script_exec_ts
                ON script_executions(timestamp);
            "#,
        )?;

        // Migration: add match_json column to pre-existing tables that
        // were created before the column existed.  SQLite's ALTER TABLE
        // ADD COLUMN fails if the column already exists, so we check the
        // pragma table_info first.
        Self::ensure_column(conn, "scripts", "match_json", "TEXT")?;

        // Migration: add on_error column to pre-existing scripts tables.
        // Stores the per-script error policy ('continue' or 'stop_chain').
        Self::ensure_column(
            conn,
            "scripts",
            "on_error",
            "TEXT NOT NULL DEFAULT 'stop_chain'",
        )?;

        // Migration: add traffic_entry_id and hook columns to
        // pre-existing script_executions tables.
        Self::ensure_column(conn, "script_executions", "traffic_entry_id", "TEXT")?;
        Self::ensure_column(conn, "script_executions", "hook", "TEXT")?;

        // Create the traffic_entry_id index AFTER the migration so it
        // succeeds on pre-existing databases that didn't have the column.
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_script_exec_traffic
                ON script_executions(traffic_entry_id);",
        )?;

        Ok(())
    }

    /// Add a column to a table if it does not already exist.
    fn ensure_column(conn: &Connection, table: &str, column: &str, def: &str) -> crate::Result<()> {
        let pragma = format!("PRAGMA table_info({table})");
        let mut stmt = conn.prepare(&pragma)?;
        let exists: bool = stmt
            .query_map([], |row| {
                let name: String = row.get(1)?;
                Ok(name)
            })?
            .filter_map(|r| r.ok())
            .any(|c| c == column);
        drop(stmt);
        if !exists {
            let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {def}");
            conn.execute(&sql, [])?;
        }
        Ok(())
    }

    /// Save (upsert) a single script.
    pub fn save_script(conn: &Connection, script: &Script) -> crate::Result<()> {
        let hooks_json = serde_json::to_string(&script.hooks)?;
        let match_json = match &script.match_filter {
            Some(f) if !f.is_empty() => Some(serde_json::to_string(f)?),
            _ => None,
        };
        let on_error_json = serde_json::to_string(&script.on_error)?;
        conn.execute(
            "INSERT OR REPLACE INTO scripts
                (id, name, description, source, hooks, enabled, priority, created_at, modified_at, match_json, on_error)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                script.id,
                script.name,
                script.description,
                script.source,
                hooks_json,
                script.enabled as i32,
                script.priority as i32,
                script.created_at.to_rfc3339(),
                script.modified_at.to_rfc3339(),
                match_json,
                on_error_json,
            ],
        )?;
        Ok(())
    }

    /// Delete a script by ID.
    pub fn delete_script(conn: &Connection, id: &str) -> crate::Result<bool> {
        let rows = conn.execute("DELETE FROM scripts WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    /// Load all scripts from the database.
    pub fn load_scripts(conn: &Connection) -> crate::Result<Vec<Script>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, source, hooks, enabled, priority, created_at, modified_at, match_json, on_error
             FROM scripts ORDER BY priority ASC, name ASC",
        )?;

        let scripts = stmt
            .query_map([], |row| {
                let hooks_json: String = row.get(4)?;
                let hooks: Vec<String> = serde_json::from_str(&hooks_json).unwrap_or_default();
                let created_at: String = row.get(7)?;
                let modified_at: String = row.get(8)?;
                let match_json: Option<String> = row.get(9)?;
                let match_filter = match_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok());
                let on_error_json: String = row.get::<_, String>(10)?;
                let on_error: ScriptErrorPolicy =
                    serde_json::from_str(&on_error_json).unwrap_or_default();
                Ok(Script {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    source: row.get(3)?,
                    hooks,
                    enabled: row.get::<_, i32>(5)? != 0,
                    priority: row.get::<_, i32>(6)? as u32,
                    created_at: parse_rfc3339(&created_at),
                    modified_at: parse_rfc3339(&modified_at),
                    match_filter,
                    on_error,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(scripts)
    }

    /// Record a script execution in the database.
    pub fn save_execution(conn: &Connection, exec: &ScriptExecution) -> crate::Result<()> {
        let console_json = serde_json::to_string(&exec.console)?;
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO script_executions
                (id, script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                exec.script_id,
                exec.duration_ms as i64,
                exec.success as i32,
                exec.error,
                console_json,
                exec.timestamp.to_rfc3339(),
                exec.traffic_entry_id,
                exec.hook,
            ],
        )?;
        Ok(())
    }

    /// Load recent executions across **all** scripts (most recent first).
    /// Used to populate the global History view in the UI.
    pub fn load_all_executions(
        conn: &Connection,
        limit: usize,
    ) -> crate::Result<Vec<ScriptExecution>> {
        let mut stmt = conn.prepare(
            "SELECT script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook
             FROM script_executions
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let execs = stmt
            .query_map(params![limit as i64], |row| {
                let console_json: Option<String> = row.get(4)?;
                let console: Vec<String> = console_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let timestamp: String = row.get(5)?;
                Ok(ScriptExecution {
                    script_id: row.get(0)?,
                    duration_ms: row.get::<_, i64>(1)? as u64,
                    success: row.get::<_, i32>(2)? != 0,
                    error: row.get(3)?,
                    console,
                    timestamp: parse_rfc3339(&timestamp),
                    traffic_entry_id: row.get(6)?,
                    hook: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(execs)
    }

    /// Load execution history for a script (most recent first).
    pub fn load_executions(
        conn: &Connection,
        script_id: &str,
        limit: usize,
    ) -> crate::Result<Vec<ScriptExecution>> {
        let mut stmt = conn.prepare(
            "SELECT script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook
             FROM script_executions
             WHERE script_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let execs = stmt
            .query_map(params![script_id, limit as i64], |row| {
                let console_json: Option<String> = row.get(4)?;
                let console: Vec<String> = console_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let timestamp: String = row.get(5)?;
                Ok(ScriptExecution {
                    script_id: row.get(0)?,
                    duration_ms: row.get::<_, i64>(1)? as u64,
                    success: row.get::<_, i32>(2)? != 0,
                    error: row.get(3)?,
                    console,
                    timestamp: parse_rfc3339(&timestamp),
                    traffic_entry_id: row.get(6)?,
                    hook: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(execs)
    }

    /// Load all script executions for a given traffic entry (most recent
    /// first).  Used to show which scripts ran on a particular request.
    pub fn load_executions_by_traffic(
        conn: &Connection,
        traffic_entry_id: &str,
        limit: usize,
    ) -> crate::Result<Vec<ScriptExecution>> {
        let mut stmt = conn.prepare(
            "SELECT script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook
             FROM script_executions
             WHERE traffic_entry_id = ?1
             ORDER BY timestamp ASC
             LIMIT ?2",
        )?;

        let execs = stmt
            .query_map(params![traffic_entry_id, limit as i64], |row| {
                let console_json: Option<String> = row.get(4)?;
                let console: Vec<String> = console_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let timestamp: String = row.get(5)?;
                Ok(ScriptExecution {
                    script_id: row.get(0)?,
                    duration_ms: row.get::<_, i64>(1)? as u64,
                    success: row.get::<_, i32>(2)? != 0,
                    error: row.get(3)?,
                    console,
                    timestamp: parse_rfc3339(&timestamp),
                    traffic_entry_id: row.get(6)?,
                    hook: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(execs)
    }

    /// Clear execution history for a specific script (or all if `script_id`
    /// is `None`).
    pub fn clear_executions(conn: &Connection, script_id: Option<&str>) -> crate::Result<()> {
        match script_id {
            Some(id) => {
                conn.execute(
                    "DELETE FROM script_executions WHERE script_id = ?1",
                    params![id],
                )?;
            }
            None => {
                conn.execute("DELETE FROM script_executions", [])?;
            }
        }
        Ok(())
    }
}

/// Parse an RFC 3339 timestamp, falling back to `now()` on failure.
fn parse_rfc3339(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        ScriptPersistence::create_tables(&conn).unwrap();
        conn
    }

    fn sample_script(name: &str) -> Script {
        let mut s = Script::new(name.to_string(), "function onRequest() {}".to_string());
        s.hooks = vec!["on_request".to_string()];
        s
    }

    #[test]
    fn save_and_load_script() {
        let conn = mem_conn();
        let script = sample_script("Test Script");
        ScriptPersistence::save_script(&conn, &script).unwrap();

        let loaded = ScriptPersistence::load_scripts(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Test Script");
        assert_eq!(loaded[0].hooks, vec!["on_request".to_string()]);
        assert!(loaded[0].enabled);
    }

    #[test]
    fn delete_script() {
        let conn = mem_conn();
        let script = sample_script("To Delete");
        ScriptPersistence::save_script(&conn, &script).unwrap();
        assert!(ScriptPersistence::delete_script(&conn, &script.id).unwrap());
        assert!(ScriptPersistence::load_scripts(&conn).unwrap().is_empty());
    }

    #[test]
    fn save_and_load_execution() {
        let conn = mem_conn();
        let script = sample_script("Exec Test");
        ScriptPersistence::save_script(&conn, &script).unwrap();

        let exec = ScriptExecution {
            script_id: script.id.clone(),
            duration_ms: 42,
            success: true,
            error: None,
            console: vec!["hello".to_string()],
            timestamp: chrono::Utc::now(),
            traffic_entry_id: None,
            hook: None,
        };
        ScriptPersistence::save_execution(&conn, &exec).unwrap();

        let execs = ScriptPersistence::load_executions(&conn, &script.id, 10).unwrap();
        assert_eq!(execs.len(), 1);
        assert_eq!(execs[0].duration_ms, 42);
        assert!(execs[0].success);
        assert_eq!(execs[0].console, vec!["hello".to_string()]);
    }

    #[test]
    fn upsert_replaces_existing() {
        let conn = mem_conn();
        let mut script = sample_script("Upsert");
        ScriptPersistence::save_script(&conn, &script).unwrap();

        script.name = "Upsert Updated".to_string();
        ScriptPersistence::save_script(&conn, &script).unwrap();

        let loaded = ScriptPersistence::load_scripts(&conn).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "Upsert Updated");
    }
}
