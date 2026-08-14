//! PostgreSQL-backed [`InterceptStoreBackend`] implementation.
//!
//! [`PostgresInterceptStore`] wraps a [`sqlx::PgPool`] and persists intercept
//! rules (mocks, rewrites, breakpoints, throttle profile, block list entries)
//! in five PostgreSQL tables, mirroring the schema and JSON serialization used
//! by the SQLite backend. All queries use runtime SQL strings with `$N`
//! placeholders.

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use sqlx::{FromRow, PgPool};

use crate::intercept::{BlockListEntry, BreakpointRule, MockRule, RewriteRule, ThrottleProfile};
use crate::storage::InterceptStoreBackend;
use crate::Result;

/// Schema for the `mock_rules` table (new 12-column schema matching
/// [`MockRule`]).
const SCHEMA_MOCK_RULES: &str = "CREATE TABLE IF NOT EXISTS mock_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    collection_id TEXT NOT NULL DEFAULT '',
    condition TEXT NOT NULL,
    response_config TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 100,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    hit_count BIGINT NOT NULL DEFAULT 0
)";

/// Schema for the `rewrite_rules` table.
const SCHEMA_REWRITE_RULES: &str = "CREATE TABLE IF NOT EXISTS rewrite_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    condition TEXT NOT NULL,
    direction TEXT NOT NULL,
    rewrites TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 100,
    created_at BIGINT NOT NULL,
    hit_count BIGINT NOT NULL DEFAULT 0
)";

/// Schema for the `breakpoint_rules` table.
const SCHEMA_BREAKPOINT_RULES: &str = "CREATE TABLE IF NOT EXISTS breakpoint_rules (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    condition TEXT NOT NULL,
    direction TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    priority INTEGER NOT NULL DEFAULT 100
)";

/// Schema for the `throttle_profile` table (singleton row, id = 1).
const SCHEMA_THROTTLE_PROFILE: &str = "CREATE TABLE IF NOT EXISTS throttle_profile (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    name TEXT NOT NULL,
    download_bps BIGINT NOT NULL,
    upload_bps BIGINT NOT NULL,
    latency_ms BIGINT NOT NULL,
    jitter_ms BIGINT NOT NULL,
    packet_loss_percent INTEGER NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT FALSE
)";

/// Schema for the `block_list_entries` table.
const SCHEMA_BLOCK_LIST_ENTRIES: &str = "CREATE TABLE IF NOT EXISTS block_list_entries (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    note TEXT NOT NULL DEFAULT '',
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    hit_count BIGINT NOT NULL DEFAULT 0,
    status_code INTEGER NOT NULL DEFAULT 403,
    response_body TEXT NOT NULL DEFAULT 'Blocked by Madhyamas',
    content_type TEXT NOT NULL DEFAULT 'text/plain',
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL
)";

/// Indexes for enabled/priority columns used during rule lookup.
const SCHEMA_INDEXES: &str = "
CREATE INDEX IF NOT EXISTS idx_mock_enabled ON mock_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_mock_priority ON mock_rules(priority);
CREATE INDEX IF NOT EXISTS idx_rewrite_enabled ON rewrite_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_rewrite_priority ON rewrite_rules(priority);
CREATE INDEX IF NOT EXISTS idx_breakpoint_enabled ON breakpoint_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_block_list_enabled ON block_list_entries(enabled);
";

/// Row shape for `mock_rules` (new 12-column schema).
#[derive(Debug, FromRow)]
struct MockRuleRow {
    id: String,
    name: String,
    description: String,
    tags: String,
    collection_id: String,
    condition: String,
    response_config: String,
    enabled: bool,
    priority: i64,
    created_at: i64,
    updated_at: i64,
    hit_count: i64,
}

/// Row shape for `rewrite_rules`.
#[derive(Debug, FromRow)]
struct RewriteRuleRow {
    id: String,
    name: String,
    condition: String,
    direction: String,
    rewrites: String,
    enabled: bool,
    priority: i64,
    created_at: i64,
    hit_count: i64,
}

/// Row shape for `breakpoint_rules`.
#[derive(Debug, FromRow)]
struct BreakpointRuleRow {
    id: String,
    name: String,
    condition: String,
    direction: String,
    enabled: bool,
    priority: i64,
}

/// Row shape for `throttle_profile`.
#[derive(Debug, FromRow)]
struct ThrottleProfileRow {
    name: String,
    download_bps: i64,
    upload_bps: i64,
    latency_ms: i64,
    jitter_ms: i64,
    packet_loss_percent: i32,
    enabled: bool,
}

/// Row shape for `block_list_entries`.
#[derive(Debug, FromRow)]
struct BlockListEntryRow {
    id: String,
    pattern: String,
    note: String,
    enabled: bool,
    hit_count: i64,
    status_code: i32,
    response_body: String,
    content_type: String,
    created_at: i64,
    updated_at: i64,
}

/// PostgreSQL-backed intercept rules store.
pub struct PostgresInterceptStore {
    pool: PgPool,
}

impl PostgresInterceptStore {
    /// Create a new store over the given pool, ensuring all intercept tables
    /// and indexes exist.
    pub async fn new(pool: PgPool) -> Result<Self> {
        sqlx::query(SCHEMA_MOCK_RULES).execute(&pool).await?;
        sqlx::query(SCHEMA_REWRITE_RULES).execute(&pool).await?;
        sqlx::query(SCHEMA_BREAKPOINT_RULES).execute(&pool).await?;
        sqlx::query(SCHEMA_THROTTLE_PROFILE).execute(&pool).await?;
        sqlx::query(SCHEMA_BLOCK_LIST_ENTRIES)
            .execute(&pool)
            .await?;
        sqlx::query(SCHEMA_INDEXES).execute(&pool).await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn parse_timestamp(ts: i64) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

#[async_trait]
impl InterceptStoreBackend for PostgresInterceptStore {
    async fn save_mock_rule(&self, rule: &MockRule) -> Result<()> {
        let condition = serde_json::to_string(&rule.condition)?;
        let response_config = serde_json::to_string(&rule.response_config)?;
        let description = rule.description.as_deref().unwrap_or("");
        let tags = serde_json::to_string(&rule.tags)?;
        let collection_id = rule.collection_id.as_deref().unwrap_or("");

        sqlx::query(
            "INSERT INTO mock_rules \
             (id, name, description, tags, collection_id, condition, response_config, enabled, priority, created_at, updated_at, hit_count) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
             ON CONFLICT (id) DO UPDATE SET \
                name = EXCLUDED.name, description = EXCLUDED.description, \
                tags = EXCLUDED.tags, collection_id = EXCLUDED.collection_id, \
                condition = EXCLUDED.condition, response_config = EXCLUDED.response_config, \
                enabled = EXCLUDED.enabled, priority = EXCLUDED.priority, \
                created_at = EXCLUDED.created_at, updated_at = EXCLUDED.updated_at, \
                hit_count = EXCLUDED.hit_count",
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(description)
        .bind(&tags)
        .bind(collection_id)
        .bind(&condition)
        .bind(&response_config)
        .bind(rule.enabled)
        .bind(rule.priority as i64)
        .bind(rule.created_at.timestamp())
        .bind(rule.updated_at.timestamp())
        .bind(rule.hit_count as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_mock_rules(&self) -> Result<Vec<MockRule>> {
        let rows: Vec<MockRuleRow> = sqlx::query_as::<_, MockRuleRow>(
            "SELECT id, name, description, tags, collection_id, condition, response_config, \
             enabled, priority, created_at, updated_at, hit_count \
             FROM mock_rules ORDER BY priority",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::with_capacity(rows.len());
        for row in rows {
            let condition = serde_json::from_str(&row.condition)?;
            let response_config = serde_json::from_str(&row.response_config)?;
            let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
            rules.push(MockRule {
                id: row.id,
                name: row.name,
                description: if row.description.is_empty() {
                    None
                } else {
                    Some(row.description)
                },
                tags,
                collection_id: if row.collection_id.is_empty() {
                    None
                } else {
                    Some(row.collection_id)
                },
                condition,
                response_config,
                enabled: row.enabled,
                priority: row.priority as u32,
                created_at: parse_timestamp(row.created_at),
                updated_at: parse_timestamp(row.updated_at),
                hit_count: row.hit_count as u64,
                expiration: None,
                version: 1,
                version_history: Vec::new(),
                response_schema: None,
                response_script: None,
            });
        }
        Ok(rules)
    }

    async fn delete_mock_rule(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM mock_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn increment_mock_hit_count(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE mock_rules SET hit_count = hit_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn save_rewrite_rule(&self, rule: &RewriteRule) -> Result<()> {
        let condition = serde_json::to_string(&rule.condition)?;
        let direction = serde_json::to_string(&rule.direction)?;
        let rewrites = serde_json::to_string(&rule.rewrites)?;

        sqlx::query(
            "INSERT INTO rewrite_rules \
             (id, name, condition, direction, rewrites, enabled, priority, created_at, hit_count) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (id) DO UPDATE SET \
                name = EXCLUDED.name, condition = EXCLUDED.condition, \
                direction = EXCLUDED.direction, rewrites = EXCLUDED.rewrites, \
                enabled = EXCLUDED.enabled, priority = EXCLUDED.priority, \
                created_at = EXCLUDED.created_at, hit_count = EXCLUDED.hit_count",
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&condition)
        .bind(&direction)
        .bind(&rewrites)
        .bind(rule.enabled)
        .bind(rule.priority as i64)
        .bind(rule.created_at.timestamp())
        .bind(rule.hit_count as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_rewrite_rules(&self) -> Result<Vec<RewriteRule>> {
        let rows: Vec<RewriteRuleRow> = sqlx::query_as::<_, RewriteRuleRow>(
            "SELECT id, name, condition, direction, rewrites, enabled, priority, created_at, hit_count \
             FROM rewrite_rules ORDER BY priority",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::with_capacity(rows.len());
        for row in rows {
            let condition = serde_json::from_str(&row.condition)?;
            let direction = serde_json::from_str(&row.direction)?;
            let rewrites = serde_json::from_str(&row.rewrites)?;
            rules.push(RewriteRule {
                id: row.id,
                name: row.name,
                condition,
                direction,
                rewrites,
                enabled: row.enabled,
                priority: row.priority as u32,
                created_at: parse_timestamp(row.created_at),
                hit_count: row.hit_count as u64,
            });
        }
        Ok(rules)
    }

    async fn delete_rewrite_rule(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM rewrite_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn save_breakpoint_rule(&self, rule: &BreakpointRule) -> Result<()> {
        let condition = serde_json::to_string(&rule.condition)?;
        let direction = serde_json::to_string(&rule.direction)?;

        sqlx::query(
            "INSERT INTO breakpoint_rules \
             (id, name, condition, direction, enabled, priority) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (id) DO UPDATE SET \
                name = EXCLUDED.name, condition = EXCLUDED.condition, \
                direction = EXCLUDED.direction, enabled = EXCLUDED.enabled, \
                priority = EXCLUDED.priority",
        )
        .bind(&rule.id)
        .bind(&rule.name)
        .bind(&condition)
        .bind(&direction)
        .bind(rule.enabled)
        .bind(rule.priority as i64)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_breakpoint_rules(&self) -> Result<Vec<BreakpointRule>> {
        let rows: Vec<BreakpointRuleRow> = sqlx::query_as::<_, BreakpointRuleRow>(
            "SELECT id, name, condition, direction, enabled, priority \
             FROM breakpoint_rules ORDER BY priority",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::with_capacity(rows.len());
        for row in rows {
            let condition = serde_json::from_str(&row.condition)?;
            let direction = serde_json::from_str(&row.direction)?;
            rules.push(BreakpointRule {
                id: row.id,
                name: row.name,
                condition,
                direction,
                enabled: row.enabled,
                priority: row.priority as u32,
            });
        }
        Ok(rules)
    }

    async fn delete_breakpoint_rule(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM breakpoint_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn save_throttle_profile(&self, profile: &ThrottleProfile, enabled: bool) -> Result<()> {
        sqlx::query(
            "INSERT INTO throttle_profile \
             (id, name, download_bps, upload_bps, latency_ms, jitter_ms, packet_loss_percent, enabled) \
             VALUES (1, $1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO UPDATE SET \
                name = EXCLUDED.name, download_bps = EXCLUDED.download_bps, \
                upload_bps = EXCLUDED.upload_bps, latency_ms = EXCLUDED.latency_ms, \
                jitter_ms = EXCLUDED.jitter_ms, \
                packet_loss_percent = EXCLUDED.packet_loss_percent, \
                enabled = EXCLUDED.enabled",
        )
        .bind(&profile.name)
        .bind(profile.download_bps as i64)
        .bind(profile.upload_bps as i64)
        .bind(profile.latency_ms as i64)
        .bind(profile.jitter_ms as i64)
        .bind(profile.packet_loss_percent as i32)
        .bind(enabled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_throttle_profile(&self) -> Result<Option<(ThrottleProfile, bool)>> {
        let row: Option<ThrottleProfileRow> = sqlx::query_as::<_, ThrottleProfileRow>(
            "SELECT name, download_bps, upload_bps, latency_ms, jitter_ms, packet_loss_percent, enabled \
             FROM throttle_profile WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| {
            (
                ThrottleProfile {
                    name: r.name,
                    download_bps: r.download_bps as u64,
                    upload_bps: r.upload_bps as u64,
                    latency_ms: r.latency_ms as u64,
                    jitter_ms: r.jitter_ms as u64,
                    packet_loss_percent: r.packet_loss_percent as u8,
                },
                r.enabled,
            )
        }))
    }

    async fn save_block_list_entry(&self, entry: &BlockListEntry) -> Result<()> {
        let note = entry.note.as_deref().unwrap_or("");
        sqlx::query(
            "INSERT INTO block_list_entries \
             (id, pattern, note, enabled, hit_count, status_code, response_body, content_type, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (id) DO UPDATE SET \
                pattern = EXCLUDED.pattern, note = EXCLUDED.note, \
                enabled = EXCLUDED.enabled, hit_count = EXCLUDED.hit_count, \
                status_code = EXCLUDED.status_code, response_body = EXCLUDED.response_body, \
                content_type = EXCLUDED.content_type, created_at = EXCLUDED.created_at, \
                updated_at = EXCLUDED.updated_at",
        )
        .bind(&entry.id)
        .bind(&entry.pattern)
        .bind(note)
        .bind(entry.enabled)
        .bind(entry.hit_count as i64)
        .bind(entry.status_code as i32)
        .bind(&entry.response_body)
        .bind(&entry.content_type)
        .bind(entry.created_at.timestamp())
        .bind(entry.updated_at.timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn load_block_list_entries(&self) -> Result<Vec<BlockListEntry>> {
        let rows: Vec<BlockListEntryRow> = sqlx::query_as::<_, BlockListEntryRow>(
            "SELECT id, pattern, note, enabled, hit_count, status_code, response_body, content_type, created_at, updated_at \
             FROM block_list_entries ORDER BY created_at",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            entries.push(BlockListEntry {
                id: row.id,
                pattern: row.pattern,
                note: if row.note.is_empty() {
                    None
                } else {
                    Some(row.note)
                },
                enabled: row.enabled,
                hit_count: row.hit_count as u64,
                status_code: row.status_code as u16,
                response_body: row.response_body,
                content_type: row.content_type,
                created_at: parse_timestamp(row.created_at),
                updated_at: parse_timestamp(row.updated_at),
            });
        }
        Ok(entries)
    }

    async fn delete_block_list_entry(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM block_list_entries WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn increment_block_list_hit_count(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE block_list_entries SET hit_count = hit_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn clear_block_list_entries(&self) -> Result<()> {
        sqlx::query("DELETE FROM block_list_entries")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn clear_mock_rules(&self) -> Result<()> {
        sqlx::query("DELETE FROM mock_rules")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn clear_rewrite_rules(&self) -> Result<()> {
        sqlx::query("DELETE FROM rewrite_rules")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn clear_breakpoint_rules(&self) -> Result<()> {
        sqlx::query("DELETE FROM breakpoint_rules")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn export_all(&self) -> Result<String> {
        let mocks = self.load_mock_rules().await?;
        let rewrites = self.load_rewrite_rules().await?;
        let breakpoints = self.load_breakpoint_rules().await?;
        let throttle = self.load_throttle_profile().await?;
        let block_list = self.load_block_list_entries().await?;

        let export = serde_json::json!({
            "mocks": mocks,
            "rewrites": rewrites,
            "breakpoints": breakpoints,
            "throttle": throttle,
            "block_list": block_list,
        });

        Ok(serde_json::to_string_pretty(&export)?)
    }

    async fn import_all(&self, json: &str) -> Result<()> {
        let import: Value = serde_json::from_str(json)?;

        if let Some(mocks) = import.get("mocks") {
            let rules: Vec<MockRule> = serde_json::from_value(mocks.clone())?;
            for rule in rules {
                self.save_mock_rule(&rule).await?;
            }
        }

        if let Some(rewrites) = import.get("rewrites") {
            let rules: Vec<RewriteRule> = serde_json::from_value(rewrites.clone())?;
            for rule in rules {
                self.save_rewrite_rule(&rule).await?;
            }
        }

        if let Some(breakpoints) = import.get("breakpoints") {
            let rules: Vec<BreakpointRule> = serde_json::from_value(breakpoints.clone())?;
            for rule in rules {
                self.save_breakpoint_rule(&rule).await?;
            }
        }

        if let Some(block_list) = import.get("block_list") {
            let entries: Vec<BlockListEntry> = serde_json::from_value(block_list.clone())?;
            for entry in entries {
                self.save_block_list_entry(&entry).await?;
            }
        }

        if let Some(throttle) = import.get("throttle") {
            if let Some((profile, enabled)) =
                serde_json::from_value::<Option<(ThrottleProfile, bool)>>(throttle.clone())?
            {
                self.save_throttle_profile(&profile, enabled).await?;
            }
        }

        Ok(())
    }
}
