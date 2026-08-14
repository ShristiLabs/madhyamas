//! PostgreSQL-backed [`PluginStoreBackend`] implementation.
//!
//! [`PostgresPluginStore`] wraps a [`sqlx::PgPool`] and persists plugin
//! enabled state, settings, and the invocation audit log in two PostgreSQL
//! tables (`plugin_state`, `plugin_invocations`), mirroring the schema and
//! JSON serialization used by the SQLite backend. All queries use runtime SQL
//! strings with `$N` placeholders.

use async_trait::async_trait;
use chrono::Utc;
use sqlx::{FromRow, PgPool};
use std::collections::HashMap;

use crate::plugin::{PluginInvocationRow, PluginStateRow};
use crate::storage::PluginStoreBackend;
use crate::Result;

/// Schema for the `plugin_state` table.
const SCHEMA_PLUGIN_STATE: &str = "CREATE TABLE IF NOT EXISTS plugin_state (
    plugin_id    TEXT PRIMARY KEY,
    enabled      BOOLEAN NOT NULL DEFAULT FALSE,
    settings     TEXT,
    installed_at TEXT NOT NULL,
    updated_at   TEXT NOT NULL
)";

/// Schema for the `plugin_invocations` table.
const SCHEMA_PLUGIN_INVOCATIONS: &str = "CREATE TABLE IF NOT EXISTS plugin_invocations (
    id            TEXT PRIMARY KEY,
    plugin_id     TEXT NOT NULL,
    hook          TEXT NOT NULL,
    duration_ms   BIGINT NOT NULL,
    fuel_consumed BIGINT,
    success       BOOLEAN NOT NULL,
    error         TEXT,
    logs          TEXT,
    modified      BOOLEAN NOT NULL DEFAULT FALSE,
    timestamp     TEXT NOT NULL
)";

/// Index on `plugin_invocations(plugin_id, timestamp DESC)`.
const SCHEMA_INDEX: &str = "CREATE INDEX IF NOT EXISTS idx_plugin_invocations_plugin
    ON plugin_invocations(plugin_id, timestamp DESC)";

/// Row shape for `plugin_state`.
#[derive(Debug, FromRow)]
struct PluginStateDbRow {
    plugin_id: String,
    enabled: bool,
    settings: Option<String>,
    installed_at: String,
    updated_at: String,
}

/// Row shape for `plugin_invocations`.
#[derive(Debug, FromRow)]
struct PluginInvocationDbRow {
    id: String,
    plugin_id: String,
    hook: String,
    duration_ms: i64,
    fuel_consumed: Option<i64>,
    success: bool,
    error: Option<String>,
    logs: Option<String>,
    modified: bool,
    timestamp: String,
}

/// PostgreSQL-backed plugin persistence store.
pub struct PostgresPluginStore {
    pool: PgPool,
}

impl PostgresPluginStore {
    /// Create a new store over the given pool, ensuring all plugin tables
    /// and indexes exist.
    pub async fn new(pool: PgPool) -> Result<Self> {
        sqlx::query(SCHEMA_PLUGIN_STATE).execute(&pool).await?;
        sqlx::query(SCHEMA_PLUGIN_INVOCATIONS)
            .execute(&pool)
            .await?;
        sqlx::query(SCHEMA_INDEX).execute(&pool).await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn parse_ts(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|e| {
            tracing::warn!("Invalid timestamp in plugin persistence: {} ({})", s, e);
            Utc::now()
        })
}

fn row_to_state(row: PluginStateDbRow) -> PluginStateRow {
    let settings: HashMap<String, serde_json::Value> = row
        .settings
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    PluginStateRow {
        plugin_id: row.plugin_id,
        enabled: row.enabled,
        settings,
        installed_at: parse_ts(&row.installed_at),
        updated_at: parse_ts(&row.updated_at),
    }
}

fn row_to_invocation(row: PluginInvocationDbRow) -> PluginInvocationRow {
    let logs: Vec<String> = row
        .logs
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    PluginInvocationRow {
        id: row.id,
        plugin_id: row.plugin_id,
        hook: row.hook,
        duration_ms: row.duration_ms as u64,
        fuel_consumed: row.fuel_consumed.map(|f| f as u64),
        success: row.success,
        error: row.error,
        logs,
        modified: row.modified,
        timestamp: parse_ts(&row.timestamp),
    }
}

#[async_trait]
impl PluginStoreBackend for PostgresPluginStore {
    async fn save_state(
        &self,
        plugin_id: &str,
        enabled: bool,
        settings: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        let settings_json = serde_json::to_string(settings)?;
        sqlx::query(
            "INSERT INTO plugin_state (plugin_id, enabled, settings, installed_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT(plugin_id) DO UPDATE SET
                enabled = EXCLUDED.enabled,
                settings = EXCLUDED.settings,
                updated_at = EXCLUDED.updated_at",
        )
        .bind(plugin_id)
        .bind(enabled)
        .bind(&settings_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_installed(&self, plugin_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO plugin_state (plugin_id, enabled, settings, installed_at, updated_at)
             VALUES ($1, FALSE, '{}', $2, $3)
             ON CONFLICT(plugin_id) DO NOTHING",
        )
        .bind(plugin_id)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_state(&self, plugin_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM plugin_state WHERE plugin_id = $1")
            .bind(plugin_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_state(&self, plugin_id: &str) -> Result<Option<PluginStateRow>> {
        let row: Option<PluginStateDbRow> = sqlx::query_as::<_, PluginStateDbRow>(
            "SELECT plugin_id, enabled, settings, installed_at, updated_at
             FROM plugin_state WHERE plugin_id = $1",
        )
        .bind(plugin_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_state))
    }

    async fn load_all_states(&self) -> Result<Vec<PluginStateRow>> {
        let rows: Vec<PluginStateDbRow> = sqlx::query_as::<_, PluginStateDbRow>(
            "SELECT plugin_id, enabled, settings, installed_at, updated_at FROM plugin_state",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_state).collect())
    }

    async fn record_invocation(&self, row: &PluginInvocationRow) -> Result<()> {
        let logs_json = serde_json::to_string(&row.logs)?;
        sqlx::query(
            "INSERT INTO plugin_invocations
                (id, plugin_id, hook, duration_ms, fuel_consumed, success, error, logs, modified, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&row.id)
        .bind(&row.plugin_id)
        .bind(&row.hook)
        .bind(row.duration_ms as i64)
        .bind(row.fuel_consumed.map(|f| f as i64))
        .bind(row.success)
        .bind(&row.error)
        .bind(&logs_json)
        .bind(row.modified)
        .bind(row.timestamp.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_invocations(
        &self,
        plugin_id: &str,
        limit: u32,
    ) -> Result<Vec<PluginInvocationRow>> {
        let rows: Vec<PluginInvocationDbRow> = sqlx::query_as::<_, PluginInvocationDbRow>(
            "SELECT id, plugin_id, hook, duration_ms, fuel_consumed, success, error, logs, modified, timestamp
             FROM plugin_invocations
             WHERE plugin_id = $1
             ORDER BY timestamp DESC
             LIMIT $2",
        )
        .bind(plugin_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_invocation).collect())
    }

    async fn prune_invocations(&self, keep: u32) -> Result<()> {
        sqlx::query(
            "DELETE FROM plugin_invocations
             WHERE id NOT IN (
                 SELECT id FROM plugin_invocations
                 ORDER BY timestamp DESC LIMIT $1
             )",
        )
        .bind(keep as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
