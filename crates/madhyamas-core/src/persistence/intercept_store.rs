//! SQLite persistence for intercept rules (mocks, rewrites, breakpoints)

use crate::intercept::{BreakpointRule, MockRule, RewriteRule, ThrottleProfile};
use crate::Error;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::sync::Arc;

/// Store for persisting intercept rules
pub struct InterceptStore {
    conn: Mutex<Connection>,
}

impl InterceptStore {
    /// Create a new intercept store
    pub fn new<P: AsRef<Path>>(path: P) -> crate::Result<Arc<Self>> {
        let conn = Connection::open(path).map_err(Error::Database)?;

        let store = Arc::new(Self {
            conn: Mutex::new(conn),
        });

        store.create_tables()?;
        Ok(store)
    }

    /// Create an in-memory store
    pub fn in_memory() -> crate::Result<Arc<Self>> {
        let conn = Connection::open_in_memory().map_err(Error::Database)?;

        let store = Arc::new(Self {
            conn: Mutex::new(conn),
        });

        store.create_tables()?;
        Ok(store)
    }

    /// Create database tables
    fn create_tables(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            -- Mock rules table
            CREATE TABLE IF NOT EXISTS mock_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                condition TEXT NOT NULL,
                response TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                created_at INTEGER NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 0
            );

            -- Rewrite rules table
            CREATE TABLE IF NOT EXISTS rewrite_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                condition TEXT NOT NULL,
                direction TEXT NOT NULL,
                rewrites TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                created_at INTEGER NOT NULL,
                hit_count INTEGER NOT NULL DEFAULT 0
            );

            -- Breakpoint rules table
            CREATE TABLE IF NOT EXISTS breakpoint_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                condition TEXT NOT NULL,
                direction TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100
            );

            -- Throttle profile table
            CREATE TABLE IF NOT EXISTS throttle_profile (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                name TEXT NOT NULL,
                download_bps INTEGER NOT NULL,
                upload_bps INTEGER NOT NULL,
                latency_ms INTEGER NOT NULL,
                jitter_ms INTEGER NOT NULL,
                packet_loss_percent INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 0
            );

            -- Create indexes
            CREATE INDEX IF NOT EXISTS idx_mock_enabled ON mock_rules(enabled);
            CREATE INDEX IF NOT EXISTS idx_mock_priority ON mock_rules(priority);
            CREATE INDEX IF NOT EXISTS idx_rewrite_enabled ON rewrite_rules(enabled);
            CREATE INDEX IF NOT EXISTS idx_rewrite_priority ON rewrite_rules(priority);
            CREATE INDEX IF NOT EXISTS idx_breakpoint_enabled ON breakpoint_rules(enabled);
            "#,
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    // ==================== Mock Rules ====================

    /// Save a mock rule
    pub fn save_mock_rule(&self, rule: &MockRule) -> crate::Result<()> {
        let conn = self.conn.lock();
        let condition = serde_json::to_string(&rule.condition)?;
        let response_config = serde_json::to_string(&rule.response_config)?;
        let description = rule.description.as_deref().unwrap_or("");
        let tags = serde_json::to_string(&rule.tags)?;
        let collection_id = rule.collection_id.as_deref().unwrap_or("");

        conn.execute(
            r#"INSERT OR REPLACE INTO mock_rules
               (id, name, description, tags, collection_id, condition, response_config, enabled, priority, created_at, updated_at, hit_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                rule.id,
                rule.name,
                description,
                tags,
                collection_id,
                condition,
                response_config,
                rule.enabled as i32,
                rule.priority,
                rule.created_at.timestamp(),
                rule.updated_at.timestamp(),
                rule.hit_count as i64
            ],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Load all mock rules
    pub fn load_mock_rules(&self) -> crate::Result<Vec<MockRule>> {
        let conn = self.conn.lock();

        // Try new schema first, fall back to old schema for backward compatibility
        let rules = conn.prepare(
            "SELECT id, name, description, tags, collection_id, condition, response_config, enabled, priority, created_at, updated_at, hit_count FROM mock_rules ORDER BY priority"
        ).or_else(|_| {
            // Fall back to old schema
            conn.prepare("SELECT id, name, condition, response, enabled, priority, created_at, hit_count FROM mock_rules ORDER BY priority")
        }).map_err(Error::Database)?
        .query_map([], |row| {
            // Try to read new schema fields
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;

            // Check if we have the new schema (12 columns) or old schema (8 columns)
            let (description, tags, collection_id, condition_json, response_config_json, enabled, priority, created_at, updated_at, hit_count) =
                if row.as_ref().column_count() >= 12 {
                    let description: String = row.get(2)?;
                    let tags_json: String = row.get(3)?;
                    let collection_id: String = row.get(4)?;
                    let condition_json: String = row.get(5)?;
                    let response_config_json: String = row.get(6)?;
                    let enabled: i32 = row.get(7)?;
                    let priority: u32 = row.get(8)?;
                    let created_at: i64 = row.get(9)?;
                    let updated_at: i64 = row.get(10)?;
                    let hit_count: i64 = row.get(11)?;
                    (
                        if description.is_empty() { None } else { Some(description) },
                        serde_json::from_str(&tags_json).unwrap_or_default(),
                        if collection_id.is_empty() { None } else { Some(collection_id) },
                        condition_json,
                        response_config_json,
                        enabled,
                        priority,
                        created_at,
                        updated_at,
                        hit_count
                    )
                } else {
                    // Old schema - convert response to response_config
                    let condition_json: String = row.get(2)?;
                    let response_json: String = row.get(3)?;
                    let enabled: i32 = row.get(4)?;
                    let priority: u32 = row.get(5)?;
                    let created_at: i64 = row.get(6)?;
                    let hit_count: i64 = row.get(7)?;

                    // Wrap old response in Single config
                    let response_config_json = format!(r#"{{"type":"single","response":{}}}"#, response_json);
                    (None, Vec::new(), None, condition_json, response_config_json, enabled, priority, created_at, created_at, hit_count)
                };

            let condition = serde_json::from_str(&condition_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let response_config = serde_json::from_str(&response_config_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

            Ok(MockRule {
                id,
                name,
                description,
                tags,
                collection_id,
                condition,
                response_config,
                enabled: enabled != 0,
                priority,
                created_at: chrono::DateTime::from_timestamp(created_at, 0)
                    .unwrap_or(chrono::Utc::now()),
                updated_at: chrono::DateTime::from_timestamp(updated_at, 0)
                    .unwrap_or(chrono::Utc::now()),
                hit_count: hit_count as u64,
                expiration: None,
                version: 1,
                version_history: Vec::new(),
                response_schema: None,
                response_script: None,
            })
        }).map_err(Error::Database)?
        .collect::<Result<Vec<_>, _>>().map_err(Error::Database)?;

        Ok(rules)
    }

    /// Delete a mock rule
    pub fn delete_mock_rule(&self, id: &str) -> crate::Result<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM mock_rules WHERE id = ?1", params![id])
            .map_err(Error::Database)?;
        Ok(rows > 0)
    }

    /// Update mock rule hit count
    pub fn increment_mock_hit_count(&self, id: &str) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE mock_rules SET hit_count = hit_count + 1 WHERE id = ?1",
            params![id],
        )
        .map_err(Error::Database)?;
        Ok(())
    }

    // ==================== Rewrite Rules ====================

    /// Save a rewrite rule
    pub fn save_rewrite_rule(&self, rule: &RewriteRule) -> crate::Result<()> {
        let conn = self.conn.lock();
        let condition = serde_json::to_string(&rule.condition)?;
        let direction = serde_json::to_string(&rule.direction)?;
        let rewrites = serde_json::to_string(&rule.rewrites)?;

        conn.execute(
            r#"INSERT OR REPLACE INTO rewrite_rules
               (id, name, condition, direction, rewrites, enabled, priority, created_at, hit_count)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                rule.id,
                rule.name,
                condition,
                direction,
                rewrites,
                rule.enabled as i32,
                rule.priority,
                rule.created_at.timestamp(),
                rule.hit_count as i64
            ],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Load all rewrite rules
    pub fn load_rewrite_rules(&self) -> crate::Result<Vec<RewriteRule>> {
        let conn = self.conn.lock();

        let rules = conn.prepare(
            "SELECT id, name, condition, direction, rewrites, enabled, priority, created_at, hit_count FROM rewrite_rules ORDER BY priority"
        ).map_err(Error::Database)?
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let condition_json: String = row.get(2)?;
            let direction_json: String = row.get(3)?;
            let rewrites_json: String = row.get(4)?;
            let enabled: i32 = row.get(5)?;
            let priority: u32 = row.get(6)?;
            let created_at: i64 = row.get(7)?;
            let hit_count: i64 = row.get(8)?;

            let condition = serde_json::from_str(&condition_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let direction = serde_json::from_str(&direction_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let rewrites = serde_json::from_str(&rewrites_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

            Ok(RewriteRule {
                id,
                name,
                condition,
                direction,
                rewrites,
                enabled: enabled != 0,
                priority,
                created_at: chrono::DateTime::from_timestamp(created_at, 0)
                    .unwrap_or(chrono::Utc::now()),
                hit_count: hit_count as u64,
            })
        }).map_err(Error::Database)?
        .collect::<Result<Vec<_>, _>>().map_err(Error::Database)?;

        Ok(rules)
    }

    /// Delete a rewrite rule
    pub fn delete_rewrite_rule(&self, id: &str) -> crate::Result<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM rewrite_rules WHERE id = ?1", params![id])
            .map_err(Error::Database)?;
        Ok(rows > 0)
    }

    // ==================== Breakpoint Rules ====================

    /// Save a breakpoint rule
    pub fn save_breakpoint_rule(&self, rule: &BreakpointRule) -> crate::Result<()> {
        let conn = self.conn.lock();
        let condition = serde_json::to_string(&rule.condition)?;
        let direction = serde_json::to_string(&rule.direction)?;

        conn.execute(
            r#"INSERT OR REPLACE INTO breakpoint_rules
               (id, name, condition, direction, enabled, priority)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            params![
                rule.id,
                rule.name,
                condition,
                direction,
                rule.enabled as i32,
                rule.priority
            ],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// Load all breakpoint rules
    pub fn load_breakpoint_rules(&self) -> crate::Result<Vec<BreakpointRule>> {
        let conn = self.conn.lock();

        let rules = conn.prepare(
            "SELECT id, name, condition, direction, enabled, priority FROM breakpoint_rules ORDER BY priority"
        ).map_err(Error::Database)?
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let condition_json: String = row.get(2)?;
            let direction_json: String = row.get(3)?;
            let enabled: i32 = row.get(4)?;
            let priority: u32 = row.get(5)?;

            let condition = serde_json::from_str(&condition_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            let direction = serde_json::from_str(&direction_json)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;

            Ok(BreakpointRule {
                id,
                name,
                condition,
                direction,
                enabled: enabled != 0,
                priority,
            })
        }).map_err(Error::Database)?
        .collect::<Result<Vec<_>, _>>().map_err(Error::Database)?;

        Ok(rules)
    }

    /// Delete a breakpoint rule
    pub fn delete_breakpoint_rule(&self, id: &str) -> crate::Result<bool> {
        let conn = self.conn.lock();
        let rows = conn
            .execute("DELETE FROM breakpoint_rules WHERE id = ?1", params![id])
            .map_err(Error::Database)?;
        Ok(rows > 0)
    }

    // ==================== Throttle Profile ====================

    /// Save throttle profile
    pub fn save_throttle_profile(
        &self,
        profile: &ThrottleProfile,
        enabled: bool,
    ) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT OR REPLACE INTO throttle_profile
               (id, name, download_bps, upload_bps, latency_ms, jitter_ms, packet_loss_percent, enabled)
               VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                profile.name,
                profile.download_bps as i64,
                profile.upload_bps as i64,
                profile.latency_ms as i64,
                profile.jitter_ms as i64,
                profile.packet_loss_percent as i32,
                enabled as i32
            ]
        ).map_err(Error::Database)?;

        Ok(())
    }

    /// Load throttle profile
    pub fn load_throttle_profile(&self) -> crate::Result<Option<(ThrottleProfile, bool)>> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT name, download_bps, upload_bps, latency_ms, jitter_ms, packet_loss_percent, enabled FROM throttle_profile WHERE id = 1",
            [],
            |row| {
                let name: String = row.get(0)?;
                let download_bps: i64 = row.get(1)?;
                let upload_bps: i64 = row.get(2)?;
                let latency_ms: i64 = row.get(3)?;
                let jitter_ms: i64 = row.get(4)?;
                let packet_loss_percent: i32 = row.get(5)?;
                let enabled: i32 = row.get(6)?;

                Ok((
                    ThrottleProfile {
                        name,
                        download_bps: download_bps as u64,
                        upload_bps: upload_bps as u64,
                        latency_ms: latency_ms as u64,
                        jitter_ms: jitter_ms as u64,
                        packet_loss_percent: packet_loss_percent as u8,
                    },
                    enabled != 0
                ))
            }
        ).optional().map_err(Error::Database)?;

        Ok(result)
    }

    // ==================== Bulk Operations ====================

    /// Clear all rules of a specific type
    pub fn clear_mock_rules(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM mock_rules", [])
            .map_err(Error::Database)?;
        Ok(())
    }

    /// Clear all rewrite rules
    pub fn clear_rewrite_rules(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM rewrite_rules", [])
            .map_err(Error::Database)?;
        Ok(())
    }

    /// Clear all breakpoint rules
    pub fn clear_breakpoint_rules(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM breakpoint_rules", [])
            .map_err(Error::Database)?;
        Ok(())
    }

    /// Export all rules to JSON
    pub fn export_all(&self) -> crate::Result<String> {
        let mocks = self.load_mock_rules()?;
        let rewrites = self.load_rewrite_rules()?;
        let breakpoints = self.load_breakpoint_rules()?;
        let throttle = self.load_throttle_profile()?;

        let export = serde_json::json!({
            "mocks": mocks,
            "rewrites": rewrites,
            "breakpoints": breakpoints,
            "throttle": throttle,
        });

        Ok(serde_json::to_string_pretty(&export)?)
    }

    /// Import rules from JSON
    pub fn import_all(&self, json: &str) -> crate::Result<()> {
        let import: serde_json::Value = serde_json::from_str(json)?;

        if let Some(mocks) = import.get("mocks") {
            let rules: Vec<MockRule> = serde_json::from_value(mocks.clone())?;
            for rule in rules {
                self.save_mock_rule(&rule)?;
            }
        }

        if let Some(rewrites) = import.get("rewrites") {
            let rules: Vec<RewriteRule> = serde_json::from_value(rewrites.clone())?;
            for rule in rules {
                self.save_rewrite_rule(&rule)?;
            }
        }

        if let Some(breakpoints) = import.get("breakpoints") {
            let rules: Vec<BreakpointRule> = serde_json::from_value(breakpoints.clone())?;
            for rule in rules {
                self.save_breakpoint_rule(&rule)?;
            }
        }

        if let Some(throttle) = import.get("throttle") {
            if let Some((profile, enabled)) = throttle.as_object().and_then(|obj| {
                let name = obj.get("name")?.as_str()?;
                let profile = ThrottleProfile {
                    name: name.to_string(),
                    download_bps: obj.get("download_bps")?.as_u64()?,
                    upload_bps: obj.get("upload_bps")?.as_u64()?,
                    latency_ms: obj.get("latency_ms")?.as_u64()?,
                    jitter_ms: obj.get("jitter_ms")?.as_u64()?,
                    packet_loss_percent: obj.get("packet_loss_percent")?.as_u64()? as u8,
                };
                let enabled = obj.get("enabled")?.as_bool()?;
                Some((profile, enabled))
            }) {
                self.save_throttle_profile(&profile, enabled)?;
            }
        }

        Ok(())
    }
}
