//! Core storage trait definitions (async backends).
//!
//! Defines the five async storage traits that abstract the concrete
//! `rusqlite`-backed store structs in `madhyamas-core`:
//! [`TrafficStoreBackend`], [`ConfigStoreBackend`],
//! [`InterceptStoreBackend`], [`PluginStoreBackend`] and
//! [`ScriptStoreBackend`]. The traits mirror the existing sync `pub fn`
//! signatures (converted to `async fn` for DB-backed methods; kept as
//! regular `fn` for in-memory config and broadcast subscriptions) so that
//! Phase 2c can implement them one-to-one against `sqlx` pools.
//!
//! This module is additive only: nothing uses the traits yet. The existing
//! `rusqlite` stores continue to work unchanged. All traits require
//! `Send + Sync` so they can be held as `Arc<dyn Trait + Send + Sync>` on
//! `AppState`. See `docs/ENTERPRISE_STORAGE_TRAITS.md` §1.2–§1.4 for the
//! design rationale (Approach C: async trait, sqlx only).

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;

pub mod sqlite;

pub use sqlite::{SqliteConfigStore, SqliteInterceptStore};

use crate::intercept::{BlockListEntry, BreakpointRule, MockRule, RewriteRule, ThrottleProfile};
use crate::mirror::MirrorWriter;
use crate::persistence::PersistedConfig;
use crate::traffic::{
    CaptureStats, FocusHost, ImportResult, ResponseData, Session, TrafficEntry, TrafficEvent,
    TrafficFilter,
};
use crate::Result;

#[cfg(feature = "plugins")]
use crate::plugin::{PluginInvocationRow, PluginStateRow};
#[cfg(feature = "scripting")]
use crate::scripting::{Script, ScriptExecution};

/// Traffic store backend — storage, retrieval, mutation, sessions,
/// export/import, focus hosts, real-time events and in-memory capture
/// configuration. DB-backed methods are `async fn`; in-memory config and
/// broadcast subscription methods are sync `fn`.
#[async_trait]
pub trait TrafficStoreBackend: Send + Sync {
    async fn store_request(&self, entry: &TrafficEntry) -> Result<()>;
    async fn store_response(&self, request_id: &str, response: &ResponseData) -> Result<()>;
    async fn get_traffic(&self, filter: &TrafficFilter) -> Result<Vec<TrafficEntry>>;
    async fn get_by_id(&self, id: &str) -> Result<Option<TrafficEntry>>;
    async fn get_entry_count(&self) -> Result<usize>;
    async fn get_capture_stats(&self) -> Result<CaptureStats>;
    async fn clear_traffic(&self) -> Result<()>;
    async fn delete_traffic(&self, ids: &[String]) -> Result<()>;
    async fn count(&self) -> Result<usize>;
    async fn export_har(&self, session_id: &str) -> Result<serde_json::Value>;
    async fn import_har(
        &self,
        har: &serde_json::Value,
        session_name: Option<&str>,
    ) -> Result<ImportResult>;
    async fn list_sessions(&self) -> Result<Vec<Session>>;
    async fn create_session(&self, name: Option<&str>) -> Result<Session>;
    async fn switch_session(&self, session_id: &str) -> Result<()>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    async fn get_traffic_by_session(&self, session_id: &str) -> Result<Vec<TrafficEntry>>;
    async fn add_focus_host(&self, pattern: &str) -> Result<FocusHost>;
    async fn remove_focus_host(&self, id: &str) -> Result<bool>;
    async fn list_focus_hosts(&self) -> Result<Vec<FocusHost>>;
    async fn clear_focus_hosts(&self) -> Result<()>;

    fn subscribe(&self) -> broadcast::Receiver<TrafficEvent>;
    fn event_sender(&self) -> broadcast::Sender<TrafficEvent>;
    fn current_session_id(&self) -> String;
    fn is_capture_enabled(&self) -> bool;
    fn set_capture_enabled(&self, enabled: bool);
    fn set_max_body_size(&self, max: usize);
    fn max_body_size(&self) -> usize;
    fn set_max_entries(&self, max: usize);
    fn max_entries(&self) -> usize;
    fn set_max_total_size_bytes(&self, max: usize);
    fn max_total_size_bytes(&self) -> usize;
    fn set_capture_request_bodies(&self, enabled: bool);
    fn capture_request_bodies(&self) -> bool;
    fn set_capture_response_bodies(&self, enabled: bool);
    fn capture_response_bodies(&self) -> bool;
    fn set_ignored_domains(&self, domains: Vec<String>);
    fn ignored_domains(&self) -> Vec<String>;
    fn set_mirror_writer(&self, writer: Arc<MirrorWriter>);
    fn mirror_writer(&self) -> Option<Arc<MirrorWriter>>;
}

/// Configuration store backend — generic typed get/set over a
/// `serde_json::Value` core, plus delete, load/save of the full
/// [`PersistedConfig`], and export/import. The non-generic `get_value` /
/// `set_value` methods keep the trait object-safe (`Arc<dyn
/// ConfigStoreBackend + Send + Sync>`); the typed `get` / `set` default
/// methods are bounded by `Self: Sized` so they do not affect object
/// safety.
#[async_trait]
pub trait ConfigStoreBackend: Send + Sync {
    async fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>>;
    async fn set_value(&self, key: &str, value: &serde_json::Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn load_config(&self) -> Result<PersistedConfig>;
    async fn save_config(&self, config: &PersistedConfig) -> Result<()>;
    async fn export(&self) -> Result<String>;
    async fn import(&self, json: &str) -> Result<()>;

    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>>
    where
        Self: Sized,
    {
        match self.get_value(key).await? {
            Some(value) => {
                let typed = serde_json::from_value(value)?;
                Ok(Some(typed))
            }
            None => Ok(None),
        }
    }

    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()>
    where
        Self: Sized,
    {
        let json = serde_json::to_value(value)?;
        self.set_value(key, &json).await
    }
}

/// Intercept rules store backend — mocks, rewrites, breakpoints, throttle
/// and blocklist persistence, plus bulk clear and export/import.
#[async_trait]
pub trait InterceptStoreBackend: Send + Sync {
    async fn save_mock_rule(&self, rule: &MockRule) -> Result<()>;
    async fn load_mock_rules(&self) -> Result<Vec<MockRule>>;
    async fn delete_mock_rule(&self, id: &str) -> Result<bool>;
    async fn increment_mock_hit_count(&self, id: &str) -> Result<()>;
    async fn save_rewrite_rule(&self, rule: &RewriteRule) -> Result<()>;
    async fn load_rewrite_rules(&self) -> Result<Vec<RewriteRule>>;
    async fn delete_rewrite_rule(&self, id: &str) -> Result<bool>;
    async fn save_breakpoint_rule(&self, rule: &BreakpointRule) -> Result<()>;
    async fn load_breakpoint_rules(&self) -> Result<Vec<BreakpointRule>>;
    async fn delete_breakpoint_rule(&self, id: &str) -> Result<bool>;
    async fn save_throttle_profile(&self, profile: &ThrottleProfile, enabled: bool) -> Result<()>;
    async fn load_throttle_profile(&self) -> Result<Option<(ThrottleProfile, bool)>>;
    async fn save_block_list_entry(&self, entry: &BlockListEntry) -> Result<()>;
    async fn load_block_list_entries(&self) -> Result<Vec<BlockListEntry>>;
    async fn delete_block_list_entry(&self, id: &str) -> Result<bool>;
    async fn increment_block_list_hit_count(&self, id: &str) -> Result<()>;
    async fn clear_block_list_entries(&self) -> Result<()>;
    async fn clear_mock_rules(&self) -> Result<()>;
    async fn clear_rewrite_rules(&self) -> Result<()>;
    async fn clear_breakpoint_rules(&self) -> Result<()>;
    async fn export_all(&self) -> Result<String>;
    async fn import_all(&self, json: &str) -> Result<()>;
}

/// Plugin registry store backend — plugin enabled state, settings and the
/// invocation audit log. Available when the `plugins` feature is enabled.
#[cfg(feature = "plugins")]
#[async_trait]
pub trait PluginStoreBackend: Send + Sync {
    async fn save_state(
        &self,
        plugin_id: &str,
        enabled: bool,
        settings: &HashMap<String, serde_json::Value>,
    ) -> Result<()>;
    async fn mark_installed(&self, plugin_id: &str) -> Result<()>;
    async fn remove_state(&self, plugin_id: &str) -> Result<()>;
    async fn load_state(&self, plugin_id: &str) -> Result<Option<PluginStateRow>>;
    async fn load_all_states(&self) -> Result<Vec<PluginStateRow>>;
    async fn record_invocation(&self, row: &PluginInvocationRow) -> Result<()>;
    async fn list_invocations(
        &self,
        plugin_id: &str,
        limit: u32,
    ) -> Result<Vec<PluginInvocationRow>>;
    async fn prune_invocations(&self, keep: u32) -> Result<()>;
}

/// Script store backend — script definitions and execution history.
/// Available when the `scripting` feature is enabled.
#[cfg(feature = "scripting")]
#[async_trait]
pub trait ScriptStoreBackend: Send + Sync {
    async fn save_script(&self, script: &Script) -> Result<()>;
    async fn load_scripts(&self) -> Result<Vec<Script>>;
    async fn delete_script(&self, id: &str) -> Result<bool>;
    async fn save_execution(&self, exec: &ScriptExecution) -> Result<()>;
    async fn load_all_executions(&self, limit: usize) -> Result<Vec<ScriptExecution>>;
    async fn load_executions(&self, script_id: &str, limit: usize) -> Result<Vec<ScriptExecution>>;
    async fn load_executions_by_traffic(
        &self,
        traffic_entry_id: &str,
        limit: usize,
    ) -> Result<Vec<ScriptExecution>>;
    async fn clear_executions(&self, script_id: Option<&str>) -> Result<()>;
}
