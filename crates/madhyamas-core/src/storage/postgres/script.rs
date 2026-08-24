//! PostgreSQL-backed [`ScriptStoreBackend`] implementation.
//!
//! [`PostgresScriptStore`] wraps a [`sqlx::PgPool`] and persists script
//! definitions and execution history in two PostgreSQL tables (`scripts`,
//! `script_executions`), mirroring the schema and JSON serialization used
//! by the SQLite backend. All queries use runtime SQL strings with `$N`
//! placeholders.

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};

use crate::scripting::{Script, ScriptErrorPolicy, ScriptExecution};
use crate::storage::ScriptStoreBackend;
use crate::Result;

/// Schema for the `scripts` table.
const SCHEMA_SCRIPTS: &str = "CREATE TABLE IF NOT EXISTS scripts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    source TEXT NOT NULL,
    hooks TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 100,
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL,
    match_json TEXT,
    on_error TEXT NOT NULL DEFAULT 'stop_chain',
    env_grants TEXT NOT NULL DEFAULT '[]',
    secret_grants TEXT NOT NULL DEFAULT '[]'
)";

/// Schema for the `script_executions` table.
const SCHEMA_SCRIPT_EXECUTIONS: &str = "CREATE TABLE IF NOT EXISTS script_executions (
    id TEXT PRIMARY KEY,
    script_id TEXT NOT NULL,
    duration_ms BIGINT NOT NULL,
    success BOOLEAN NOT NULL,
    error TEXT,
    console TEXT,
    timestamp TEXT NOT NULL,
    traffic_entry_id TEXT,
    hook TEXT
)";

/// Indexes for the `script_executions` table. Each statement is executed
/// individually.
const SCHEMA_INDEX_STMTS: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_script_exec_script ON script_executions(script_id)",
    "CREATE INDEX IF NOT EXISTS idx_script_exec_ts ON script_executions(timestamp)",
    "CREATE INDEX IF NOT EXISTS idx_script_exec_traffic ON script_executions(traffic_entry_id)",
];

/// Row shape for `scripts`.
#[derive(Debug, FromRow)]
struct ScriptDbRow {
    id: String,
    name: String,
    description: Option<String>,
    source: String,
    hooks: String,
    enabled: bool,
    priority: i32,
    created_at: String,
    modified_at: String,
    match_json: Option<String>,
    on_error: String,
    env_grants: String,
    secret_grants: String,
}

/// Row shape for `script_executions`.
#[derive(Debug, FromRow)]
struct ScriptExecutionDbRow {
    script_id: String,
    duration_ms: i64,
    success: bool,
    error: Option<String>,
    console: Option<String>,
    timestamp: String,
    traffic_entry_id: Option<String>,
    hook: Option<String>,
}

/// PostgreSQL-backed script persistence store.
pub struct PostgresScriptStore {
    pool: PgPool,
}

impl PostgresScriptStore {
    /// Create a new store over the given pool, ensuring all script tables
    /// and indexes exist.
    pub async fn new(pool: PgPool) -> Result<Self> {
        let mut tx = pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(0x4D414448)")
            .execute(&mut *tx)
            .await?;
        sqlx::query(SCHEMA_SCRIPTS).execute(&mut *tx).await?;
        sqlx::query(SCHEMA_SCRIPT_EXECUTIONS)
            .execute(&mut *tx)
            .await?;
        for stmt in SCHEMA_INDEX_STMTS {
            sqlx::query(stmt).execute(&mut *tx).await?;
        }
        // Best-effort migrations for pre-existing databases: add the
        // grants columns (issue #87). Errors (column already exists) are
        // ignored.
        let _ = sqlx::query("ALTER TABLE scripts ADD COLUMN env_grants TEXT NOT NULL DEFAULT '[]'")
            .execute(&mut *tx)
            .await;
        let _ =
            sqlx::query("ALTER TABLE scripts ADD COLUMN secret_grants TEXT NOT NULL DEFAULT '[]'")
                .execute(&mut *tx)
                .await;
        tx.commit().await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn parse_rfc3339(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_else(|_| chrono::Utc::now())
}

fn row_to_script(row: ScriptDbRow) -> Script {
    let hooks: Vec<String> = serde_json::from_str(&row.hooks).unwrap_or_default();
    let match_filter = row
        .match_json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());
    let on_error: ScriptErrorPolicy = serde_json::from_str(&row.on_error).unwrap_or_default();
    let env_grants: Vec<String> = serde_json::from_str(&row.env_grants).unwrap_or_default();
    let secret_grants: Vec<String> = serde_json::from_str(&row.secret_grants).unwrap_or_default();
    Script {
        id: row.id,
        name: row.name,
        description: row.description,
        source: row.source,
        hooks,
        enabled: row.enabled,
        priority: row.priority as u32,
        created_at: parse_rfc3339(&row.created_at),
        modified_at: parse_rfc3339(&row.modified_at),
        match_filter,
        on_error,
        env_grants,
        secret_grants,
    }
}

fn row_to_execution(row: ScriptExecutionDbRow) -> ScriptExecution {
    let console: Vec<String> = row
        .console
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    ScriptExecution {
        script_id: row.script_id,
        duration_ms: row.duration_ms as u64,
        success: row.success,
        error: row.error,
        console,
        timestamp: parse_rfc3339(&row.timestamp),
        traffic_entry_id: row.traffic_entry_id,
        hook: row.hook,
    }
}

#[async_trait]
impl ScriptStoreBackend for PostgresScriptStore {
    async fn save_script(&self, script: &Script) -> Result<()> {
        let hooks_json = serde_json::to_string(&script.hooks)?;
        let match_json = match &script.match_filter {
            Some(f) if !f.is_empty() => Some(serde_json::to_string(f)?),
            _ => None,
        };
        let on_error_json = serde_json::to_string(&script.on_error)?;
        let env_grants_json = serde_json::to_string(&script.env_grants)?;
        let secret_grants_json = serde_json::to_string(&script.secret_grants)?;
        sqlx::query(
            "INSERT INTO scripts
                (id, name, description, source, hooks, enabled, priority, created_at, modified_at, match_json, on_error, env_grants, secret_grants)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name, description = EXCLUDED.description,
                source = EXCLUDED.source, hooks = EXCLUDED.hooks,
                enabled = EXCLUDED.enabled, priority = EXCLUDED.priority,
                created_at = EXCLUDED.created_at, modified_at = EXCLUDED.modified_at,
                match_json = EXCLUDED.match_json, on_error, env_grants, secret_grants = EXCLUDED.on_error, env_grants = EXCLUDED.env_grants, secret_grants = EXCLUDED.secret_grants",
        )
        .bind(&script.id)
        .bind(&script.name)
        .bind(&script.description)
        .bind(&script.source)
        .bind(&hooks_json)
        .bind(script.enabled)
        .bind(script.priority as i32)
        .bind(script.created_at.to_rfc3339())
        .bind(script.modified_at.to_rfc3339())
        .bind(&match_json)
        .bind(&on_error_json)
        .bind(&env_grants_json)
        .bind(&secret_grants_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_scripts(&self) -> Result<Vec<Script>> {
        let rows: Vec<ScriptDbRow> = sqlx::query_as::<_, ScriptDbRow>(
            "SELECT id, name, description, source, hooks, enabled, priority, created_at, modified_at, match_json, on_error, env_grants, secret_grants
             FROM scripts ORDER BY priority ASC, name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_script).collect())
    }

    async fn delete_script(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM scripts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn save_execution(&self, exec: &ScriptExecution) -> Result<()> {
        let console_json = serde_json::to_string(&exec.console)?;
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO script_executions
                (id, script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&id)
        .bind(&exec.script_id)
        .bind(exec.duration_ms as i64)
        .bind(exec.success)
        .bind(&exec.error)
        .bind(&console_json)
        .bind(exec.timestamp.to_rfc3339())
        .bind(&exec.traffic_entry_id)
        .bind(&exec.hook)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_all_executions(&self, limit: usize) -> Result<Vec<ScriptExecution>> {
        let rows: Vec<ScriptExecutionDbRow> = sqlx::query_as::<_, ScriptExecutionDbRow>(
            "SELECT script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook
             FROM script_executions
             ORDER BY timestamp DESC
             LIMIT $1",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_execution).collect())
    }

    async fn load_executions(&self, script_id: &str, limit: usize) -> Result<Vec<ScriptExecution>> {
        let rows: Vec<ScriptExecutionDbRow> = sqlx::query_as::<_, ScriptExecutionDbRow>(
            "SELECT script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook
             FROM script_executions
             WHERE script_id = $1
             ORDER BY timestamp DESC
             LIMIT $2",
        )
        .bind(script_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_execution).collect())
    }

    async fn load_executions_by_traffic(
        &self,
        traffic_entry_id: &str,
        limit: usize,
    ) -> Result<Vec<ScriptExecution>> {
        let rows: Vec<ScriptExecutionDbRow> = sqlx::query_as::<_, ScriptExecutionDbRow>(
            "SELECT script_id, duration_ms, success, error, console, timestamp, traffic_entry_id, hook
             FROM script_executions
             WHERE traffic_entry_id = $1
             ORDER BY timestamp ASC
             LIMIT $2",
        )
        .bind(traffic_entry_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_execution).collect())
    }

    async fn clear_executions(&self, script_id: Option<&str>) -> Result<()> {
        match script_id {
            Some(id) => {
                sqlx::query("DELETE FROM script_executions WHERE script_id = $1")
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
            }
            None => {
                sqlx::query("DELETE FROM script_executions")
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }
}
