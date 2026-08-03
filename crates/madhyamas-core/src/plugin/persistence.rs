//! SQLite-backed persistence for plugin state, settings, and invocation logs.
//!
//! This module stores:
//! - **`plugin_state`** — per-plugin enabled flag and user settings (JSON),
//!   restored on startup so plugin enable/disable and settings survive
//!   restarts.
//! - **`plugin_invocations`** — an append-only audit log of every plugin hook
//!   invocation (hook, duration, fuel consumed, success/error, logs, whether
//!   the request/response was modified), exposed via
//!   `GET /api/plugins/{id}/logs`.
//!
//! The store owns its own `rusqlite::Connection` (guarded by a `Mutex`),
//! independent of [`crate::traffic::TrafficStore`]. It uses WAL mode and a
//! 5-second busy timeout for safe concurrent access from the API and proxy
//! threads.

use crate::Error;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;
use tracing::warn;

/// A single persisted plugin-state row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStateRow {
    pub plugin_id: String,
    pub enabled: bool,
    pub settings: HashMap<String, serde_json::Value>,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single invocation-log row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInvocationRow {
    pub id: String,
    pub plugin_id: String,
    pub hook: String,
    pub duration_ms: u64,
    pub fuel_consumed: Option<u64>,
    pub success: bool,
    pub error: Option<String>,
    pub logs: Vec<String>,
    pub modified: bool,
    pub timestamp: DateTime<Utc>,
}

/// SQLite-backed plugin persistence.
pub struct PluginPersistence {
    conn: Mutex<Connection>,
}

impl PluginPersistence {
    /// Open (or create) the persistence store at `path`.
    pub fn open<P: AsRef<Path>>(path: P) -> crate::Result<Arc<Self>> {
        let conn = Connection::open(path).map_err(Error::Database)?;
        Self::init(&conn)?;
        Ok(Arc::new(Self {
            conn: Mutex::new(conn),
        }))
    }

    /// Create an in-memory store (used by tests and the no-persistence fallback).
    pub fn in_memory() -> crate::Result<Arc<Self>> {
        let conn = Connection::open_in_memory().map_err(Error::Database)?;
        Self::init(&conn)?;
        Ok(Arc::new(Self {
            conn: Mutex::new(conn),
        }))
    }

    fn init(conn: &Connection) -> crate::Result<()> {
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(Error::Database)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(Error::Database)?;
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS plugin_state (
                plugin_id    TEXT PRIMARY KEY,
                enabled      INTEGER NOT NULL DEFAULT 0,
                settings     TEXT,
                installed_at TEXT NOT NULL,
                updated_at   TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS plugin_invocations (
                id            TEXT PRIMARY KEY,
                plugin_id     TEXT NOT NULL,
                hook          TEXT NOT NULL,
                duration_ms   INTEGER NOT NULL,
                fuel_consumed INTEGER,
                success       INTEGER NOT NULL,
                error         TEXT,
                logs          TEXT,
                modified      INTEGER NOT NULL DEFAULT 0,
                timestamp     TEXT NOT NULL,
                FOREIGN KEY (plugin_id) REFERENCES plugin_state(plugin_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_plugin_invocations_plugin
                ON plugin_invocations(plugin_id, timestamp DESC);
            "#,
        )
        .map_err(Error::Database)?;
        Ok(())
    }

    /// Upsert the enabled flag and settings for a plugin.
    pub fn save_state(
        &self,
        plugin_id: &str,
        enabled: bool,
        settings: &HashMap<String, serde_json::Value>,
    ) -> crate::Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let settings_json = serde_json::to_string(settings).map_err(Error::Serialization)?;
        conn.execute(
            "INSERT INTO plugin_state (plugin_id, enabled, settings, installed_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(plugin_id) DO UPDATE SET
                enabled = excluded.enabled,
                settings = excluded.settings,
                updated_at = excluded.updated_at",
            params![plugin_id, enabled as i64, settings_json, now],
        )
        .map_err(Error::Database)?;
        Ok(())
    }

    /// Mark a plugin as installed (insert a state row with default disabled
    /// state if it does not already exist).
    pub fn mark_installed(&self, plugin_id: &str) -> crate::Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT OR IGNORE INTO plugin_state (plugin_id, enabled, settings, installed_at, updated_at)
             VALUES (?1, 0, '{}', ?2, ?2)",
            params![plugin_id, now],
        )
        .map_err(Error::Database)?;
        Ok(())
    }

    /// Remove the state row for a plugin (called on uninstall).
    pub fn remove_state(&self, plugin_id: &str) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM plugin_state WHERE plugin_id = ?1",
            params![plugin_id],
        )
        .map_err(Error::Database)?;
        Ok(())
    }

    /// Load the persisted state for a plugin.
    pub fn load_state(&self, plugin_id: &str) -> crate::Result<Option<PluginStateRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT plugin_id, enabled, settings, installed_at, updated_at
                 FROM plugin_state WHERE plugin_id = ?1",
            )
            .map_err(Error::Database)?;
        let row = stmt
            .query_row(params![plugin_id], |r| {
                let enabled: i64 = r.get(1)?;
                let settings_json: String = r.get::<_, Option<String>>(2)?.unwrap_or_default();
                let installed_at: String = r.get(3)?;
                let updated_at: String = r.get(4)?;
                Ok((
                    r.get::<_, String>(0)?,
                    enabled,
                    settings_json,
                    installed_at,
                    updated_at,
                ))
            })
            .optional()
            .map_err(Error::Database)?;

        match row {
            Some((id, enabled, settings_json, installed_at, updated_at)) => {
                let settings: HashMap<String, serde_json::Value> = if settings_json.is_empty() {
                    HashMap::new()
                } else {
                    serde_json::from_str(&settings_json).unwrap_or_default()
                };
                Ok(Some(PluginStateRow {
                    plugin_id: id,
                    enabled: enabled != 0,
                    settings,
                    installed_at: parse_ts(&installed_at),
                    updated_at: parse_ts(&updated_at),
                }))
            }
            None => Ok(None),
        }
    }

    /// Load all persisted plugin states (used on startup to restore state).
    pub fn load_all_states(&self) -> crate::Result<Vec<PluginStateRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT plugin_id, enabled, settings, installed_at, updated_at FROM plugin_state",
            )
            .map_err(Error::Database)?;
        let rows = stmt
            .query_map([], |r| {
                let enabled: i64 = r.get(1)?;
                let settings_json: String = r.get::<_, Option<String>>(2)?.unwrap_or_default();
                let installed_at: String = r.get(3)?;
                let updated_at: String = r.get(4)?;
                Ok((
                    r.get::<_, String>(0)?,
                    enabled,
                    settings_json,
                    installed_at,
                    updated_at,
                ))
            })
            .map_err(Error::Database)?;

        let mut out = Vec::new();
        for r in rows {
            let (id, enabled, settings_json, installed_at, updated_at) =
                r.map_err(Error::Database)?;
            let settings: HashMap<String, serde_json::Value> = if settings_json.is_empty() {
                HashMap::new()
            } else {
                serde_json::from_str(&settings_json).unwrap_or_default()
            };
            out.push(PluginStateRow {
                plugin_id: id,
                enabled: enabled != 0,
                settings,
                installed_at: parse_ts(&installed_at),
                updated_at: parse_ts(&updated_at),
            });
        }
        Ok(out)
    }

    /// Record a plugin invocation in the audit log.
    pub fn record_invocation(&self, row: &PluginInvocationRow) -> crate::Result<()> {
        let conn = self.conn.lock();
        let logs_json = serde_json::to_string(&row.logs).map_err(Error::Serialization)?;
        conn.execute(
            "INSERT INTO plugin_invocations
                (id, plugin_id, hook, duration_ms, fuel_consumed, success, error, logs, modified, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                row.id,
                row.plugin_id,
                row.hook,
                row.duration_ms as i64,
                row.fuel_consumed.map(|f| f as i64),
                row.success as i64,
                row.error,
                logs_json,
                row.modified as i64,
                row.timestamp.to_rfc3339(),
            ],
        )
        .map_err(Error::Database)?;
        Ok(())
    }

    /// List recent invocations for a plugin (newest first), up to `limit`.
    pub fn list_invocations(
        &self,
        plugin_id: &str,
        limit: u32,
    ) -> crate::Result<Vec<PluginInvocationRow>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, plugin_id, hook, duration_ms, fuel_consumed, success, error, logs, modified, timestamp
                 FROM plugin_invocations
                 WHERE plugin_id = ?1
                 ORDER BY timestamp DESC
                 LIMIT ?2",
            )
            .map_err(Error::Database)?;
        let rows = stmt
            .query_map(params![plugin_id, limit as i64], |r| {
                let fuel: Option<i64> = r.get(4)?;
                let success: i64 = r.get(5)?;
                let modified: i64 = r.get(8)?;
                let logs_json: String = r
                    .get::<_, Option<String>>(7)?
                    .unwrap_or_else(|| "[]".to_string());
                Ok(PluginInvocationRow {
                    id: r.get(0)?,
                    plugin_id: r.get(1)?,
                    hook: r.get(2)?,
                    duration_ms: r.get::<_, i64>(3)? as u64,
                    fuel_consumed: fuel.map(|f| f as u64),
                    success: success != 0,
                    error: r.get(6)?,
                    logs: serde_json::from_str(&logs_json).unwrap_or_default(),
                    modified: modified != 0,
                    timestamp: parse_ts(&r.get::<_, String>(9)?),
                })
            })
            .map_err(Error::Database)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(Error::Database)?);
        }
        Ok(out)
    }

    /// Delete invocation logs older than `keep` rows per plugin (housekeeping).
    pub fn prune_invocations(&self, keep: u32) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM plugin_invocations
             WHERE id NOT IN (
                 SELECT id FROM plugin_invocations
                 ORDER BY timestamp DESC LIMIT ?1
             )",
            params![keep as i64],
        )
        .map_err(Error::Database)?;
        Ok(())
    }
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|e| {
            warn!("Invalid timestamp in plugin persistence: {} ({})", s, e);
            Utc::now()
        })
}

impl Debug for PluginPersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginPersistence").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn persistence_roundtrip() {
        let store = PluginPersistence::in_memory().unwrap();
        let mut settings = HashMap::new();
        settings.insert("depth".to_string(), json!("deep"));
        settings.insert("verbose".to_string(), json!(true));

        store.save_state("my.plugin", true, &settings).unwrap();
        let row = store.load_state("my.plugin").unwrap().unwrap();
        assert!(row.enabled);
        assert_eq!(row.settings.get("depth"), Some(&json!("deep")));

        // Update enabled only.
        store.save_state("my.plugin", false, &settings).unwrap();
        let row = store.load_state("my.plugin").unwrap().unwrap();
        assert!(!row.enabled);

        // Invocation logging.
        let inv = PluginInvocationRow {
            id: "inv1".to_string(),
            plugin_id: "my.plugin".to_string(),
            hook: "on_request".to_string(),
            duration_ms: 12,
            fuel_consumed: Some(1000),
            success: true,
            error: None,
            logs: vec!["hello".to_string()],
            modified: true,
            timestamp: Utc::now(),
        };
        store.record_invocation(&inv).unwrap();
        let logs = store.list_invocations("my.plugin", 10).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].hook, "on_request");
        assert!(logs[0].modified);
        assert_eq!(logs[0].logs, vec!["hello".to_string()]);

        // Remove state (uninstall) cascades to invocations.
        store.remove_state("my.plugin").unwrap();
        assert!(store.load_state("my.plugin").unwrap().is_none());
        // FK ON DELETE CASCADE — invocations should be gone.
        let logs = store.list_invocations("my.plugin", 10).unwrap();
        assert!(logs.is_empty());
    }

    #[test]
    fn load_all_states() {
        let store = PluginPersistence::in_memory().unwrap();
        store.save_state("a", true, &HashMap::new()).unwrap();
        store.save_state("b", false, &HashMap::new()).unwrap();
        let all = store.load_all_states().unwrap();
        assert_eq!(all.len(), 2);
    }
}
