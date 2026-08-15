//! Plugin persistence row types.
//!
//! [`PluginStateRow`] and [`PluginInvocationRow`] are plain serializable
//! structs used by both the async [`crate::storage::PluginStoreBackend`]
//! trait and its [`crate::storage::SqlitePluginStore`] implementation.
//! The former sync `PluginPersistence` struct has been replaced by the
//! sqlx-backed store in Phase 2c.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
