//! Block list — block requests to matching domains/patterns.
//!
//! The [`BlockListManager`] is an intercept handler that runs at **priority
//! 5** — before rewrites (10), mocks (20), breakpoints (30), and throttle
//! (40). When a request's host matches an enabled block list entry, the
//! pipeline short-circuits with a configurable response (default `403
//! Forbidden` with body `"Blocked by Madhyamas"`). No upstream connection
//! is made.
//!
//! # Pattern matching
//!
//! Each [`BlockListEntry`] has a `pattern` string that supports:
//!
//! | Pattern | Matches |
//! |---------|---------|
//! | `example.com` | `example.com` and any subdomain (`api.example.com`, ...) |
//! | `*.example.com` | Subdomains of `example.com` (but not `example.com` itself) |
//! | `ads.*` | `ads.com`, `ads.net`, etc. (wildcard in the TLD position) |
//! | `*ads*` | Any host containing `ads` (substring match) |
//!
//! Matching is case-insensitive. See [`matches_pattern`] for the full
//! semantics.
//!
//! # Persistence
//!
//! When an [`InterceptStoreBackend`] is attached via [`BlockListManager::with_store`],
//! entries are saved to / loaded from the `block_list_entries` SQLite table,
//! surviving proxy restarts.

use super::handler::{InterceptAction, InterceptHandler};
use crate::storage::InterceptStoreBackend;
use crate::traffic::{RequestData, ResponseData};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Core Types
// ============================================================================

/// A single block list entry.
///
/// When `enabled` and the request host matches `pattern`, the proxy
/// returns `status_code` with `response_body` instead of forwarding
/// the request upstream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockListEntry {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Domain or wildcard pattern to match against the request host.
    ///
    /// See [`matches_pattern`] for supported syntax.
    pub pattern: String,
    /// Optional human-readable note describing why this entry exists.
    #[serde(default)]
    pub note: Option<String>,
    /// Whether this entry is actively blocking. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Number of times this entry has blocked a request.
    #[serde(default)]
    pub hit_count: u64,
    /// HTTP status code returned to the client when blocked. Default: `403`.
    #[serde(default = "default_status_code")]
    pub status_code: u16,
    /// Response body returned to the client when blocked.
    /// Default: `"Blocked by Madhyamas"`.
    #[serde(default = "default_response_body")]
    pub response_body: String,
    /// Response content type. Default: `"text/plain"`.
    #[serde(default = "default_content_type")]
    pub content_type: String,
    /// When the entry was created.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// When the entry was last modified.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}

fn default_status_code() -> u16 {
    403
}

fn default_response_body() -> String {
    "Blocked by Madhyamas".to_string()
}

fn default_content_type() -> String {
    "text/plain".to_string()
}

impl BlockListEntry {
    /// Create a new entry with the given pattern and defaults for all
    /// other fields.
    pub fn new(pattern: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            pattern,
            note: None,
            enabled: true,
            hit_count: 0,
            status_code: default_status_code(),
            response_body: default_response_body(),
            content_type: default_content_type(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Create a new entry with a custom name/note.
    pub fn with_note(pattern: String, note: String) -> Self {
        let mut entry = Self::new(pattern);
        entry.note = Some(note);
        entry
    }
}

impl Default for BlockListEntry {
    fn default() -> Self {
        Self::new(String::new())
    }
}

// ============================================================================
// Pattern Matching
// ============================================================================

/// Check whether a request host matches a block list pattern.
///
/// # Semantics
///
/// - **Exact match**: `example.com` matches `example.com` and any
///   subdomain (`api.example.com`, `www.example.com`). This mirrors the
///   suffix-matching behavior used by the passthrough-domains and
///   upstream-proxy bypass features.
/// - **Leading wildcard** `*.example.com`: matches subdomains of
///   `example.com` but **not** `example.com` itself.
/// - **General wildcards** (`*`): treated as a glob — `*` matches any
///   sequence of characters. The pattern is converted to a simple
///   glob: `*` → `.*`, other regex metacharacters are escaped.
/// - Matching is **case-insensitive**.
/// - Leading/trailing dots on the host are stripped before comparison.
///
/// # Examples
///
/// ```
/// use madhyamas_core::intercept::matches_pattern;
///
/// assert!(matches_pattern("example.com", "example.com"));
/// assert!(matches_pattern("example.com", "api.example.com"));
/// assert!(matches_pattern("*.example.com", "api.example.com"));
/// assert!(!matches_pattern("*.example.com", "example.com"));
/// assert!(matches_pattern("*ads*", "doubleclick.ads.com"));
/// assert!(!matches_pattern("example.com", "notexample.com"));
/// ```
pub fn matches_pattern(pattern: &str, host: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('.').to_lowercase();
    let host = host.trim().trim_end_matches('.').to_lowercase();

    if pattern.is_empty() || host.is_empty() {
        return false;
    }

    // Leading wildcard: "*.example.com" matches subdomains only.
    if let Some(suffix) = pattern.strip_prefix("*.") {
        // host must be a strict subdomain: "api.example.com" matches,
        // "example.com" does not.
        return host.ends_with(&format!(".{suffix}"));
    }

    // If the pattern contains any wildcard '*', do a glob match.
    if pattern.contains('*') {
        return glob_match(&pattern, &host);
    }

    // Exact domain or suffix match (e.g. "example.com" matches
    // "api.example.com"), consistent with passthrough_domains behavior.
    host == pattern || host.ends_with(&format!(".{pattern}"))
}

/// Simple glob matcher: `*` matches any sequence of characters.
///
/// The pattern is converted to a regex by escaping all regex
/// metacharacters except `*`, which becomes `.*`. The entire host must
/// match (anchored).
fn glob_match(pattern: &str, text: &str) -> bool {
    // Build a regex from the glob pattern.
    let mut regex = String::with_capacity(pattern.len() * 2);
    regex.push('^');
    for ch in pattern.chars() {
        match ch {
            '*' => regex.push_str(".*"),
            c if regex_syntax::is_meta_character(c) => {
                regex.push('\\');
                regex.push(c);
            }
            c => regex.push(c),
        }
    }
    regex.push('$');

    // Use the cached regex compiler for performance.
    super::regex_cache::cached_regex(&regex)
        .map(|re| re.is_match(text))
        .unwrap_or(false)
}

/// Minimal check for regex metacharacters that need escaping.
///
/// This avoids pulling in the `regex_syntax` crate just for this check.
mod regex_syntax {
    /// Returns true if `c` is a regex metacharacter that needs escaping
    /// when building a glob-to-regex conversion (excluding `*` which is
    /// handled separately).
    pub fn is_meta_character(c: char) -> bool {
        matches!(
            c,
            '.' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' | '\\'
        )
    }
}

// ============================================================================
// Manager
// ============================================================================

/// Manages a collection of [`BlockListEntry`] items and implements
/// [`InterceptHandler`] so it participates in the proxy pipeline.
///
/// Block list entries are checked at priority **5** — before rewrites,
/// mocks, breakpoints, and throttle — so blocked requests never reach
/// upstream servers or other intercept handlers.
pub struct BlockListManager {
    entries: RwLock<Vec<BlockListEntry>>,
    store: Option<Arc<dyn InterceptStoreBackend + Send + Sync>>,
}

impl Default for BlockListManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockListManager {
    /// Create an empty block list manager (no persistence).
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            store: None,
        }
    }

    /// Attach a SQLite persistence backend. When set, [`Persistable::save`]
    /// and [`Persistable::load`] read/write entries through this store.
    pub fn with_store(mut self, store: Arc<dyn InterceptStoreBackend + Send + Sync>) -> Self {
        self.store = Some(store);
        self
    }

    // ── CRUD ───────────────────────────────────────────────────────

    /// Add a block list entry. Returns the entry's ID.
    pub async fn add_entry(&self, entry: BlockListEntry) -> String {
        let id = entry.id.clone();
        if let Some(store) = &self.store {
            if let Err(e) = store.save_block_list_entry(&entry).await {
                tracing::warn!("Failed to persist block list entry: {}", e);
            }
        }
        self.entries.write().push(entry);
        id
    }

    /// Remove a block list entry by ID. Returns `true` if the entry
    /// existed and was removed.
    pub async fn remove_entry(&self, id: &str) -> bool {
        let removed = {
            let mut entries = self.entries.write();
            if let Some(pos) = entries.iter().position(|e| e.id == id) {
                entries.remove(pos);
                true
            } else {
                false
            }
        };
        if removed {
            if let Some(store) = &self.store {
                if let Err(e) = store.delete_block_list_entry(id).await {
                    tracing::warn!("Failed to delete block list entry from store: {}", e);
                }
            }
        }
        removed
    }

    /// Get a copy of all entries.
    pub fn get_entries(&self) -> Vec<BlockListEntry> {
        self.entries.read().clone()
    }

    /// Get a specific entry by ID.
    pub fn get_entry(&self, id: &str) -> Option<BlockListEntry> {
        self.entries.read().iter().find(|e| e.id == id).cloned()
    }

    /// Toggle an entry's enabled state. Returns `true` if the entry
    /// existed.
    pub async fn toggle_entry(&self, id: &str, enabled: bool) -> bool {
        let entry_to_save = {
            let mut entries = self.entries.write();
            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                entry.enabled = enabled;
                entry.updated_at = Utc::now();
                Some(entry.clone())
            } else {
                None
            }
        };
        if let Some(entry) = entry_to_save {
            if let Some(store) = &self.store {
                if let Err(e) = store.save_block_list_entry(&entry).await {
                    tracing::warn!("Failed to persist block list entry toggle: {}", e);
                }
            }
            true
        } else {
            false
        }
    }

    /// Update an existing entry in place. Returns `true` if found.
    pub async fn update_entry(&self, id: &str, mut entry: BlockListEntry) -> bool {
        let entry_to_save = {
            let mut entries = self.entries.write();
            if let Some(pos) = entries.iter().position(|e| e.id == id) {
                // Preserve the original ID and hit_count; update the timestamp.
                entry.id = id.to_string();
                entry.hit_count = entries[pos].hit_count;
                entry.updated_at = Utc::now();
                entries[pos] = entry.clone();
                Some(entry)
            } else {
                None
            }
        };
        if let Some(entry) = entry_to_save {
            if let Some(store) = &self.store {
                if let Err(e) = store.save_block_list_entry(&entry).await {
                    tracing::warn!("Failed to persist block list entry update: {}", e);
                }
            }
            true
        } else {
            false
        }
    }

    /// Clear all entries.
    pub async fn clear(&self) {
        self.entries.write().clear();
        if let Some(store) = &self.store {
            if let Err(e) = store.clear_block_list_entries().await {
                tracing::warn!("Failed to clear block list entries in store: {}", e);
            }
        }
    }

    /// Number of entries (enabled and disabled).
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Whether there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Summary statistics for display in the UI.
    pub fn stats(&self) -> BlockListStats {
        let entries = self.entries.read();
        let total = entries.len();
        let enabled = entries.iter().filter(|e| e.enabled).count();
        let total_hits: u64 = entries.iter().map(|e| e.hit_count).sum();
        BlockListStats {
            total,
            enabled,
            disabled: total - enabled,
            total_hits,
        }
    }

    // ── Matching ───────────────────────────────────────────────────

    /// Find the first enabled entry whose pattern matches the given host.
    /// Returns a clone of the matching entry, if any.
    fn find_matching(&self, host: &str) -> Option<BlockListEntry> {
        self.entries
            .read()
            .iter()
            .find(|e| e.enabled && matches_pattern(&e.pattern, host))
            .cloned()
    }

    /// Increment the hit count of the entry with the given ID. Called
    /// when a request is blocked by that entry.
    async fn increment_hit_count(&self, id: &str) {
        let exists = {
            let mut entries = self.entries.write();
            if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
                entry.hit_count += 1;
                true
            } else {
                false
            }
        };
        if exists {
            if let Some(store) = &self.store {
                if let Err(e) = store.increment_block_list_hit_count(id).await {
                    tracing::warn!("Failed to persist block list hit count: {}", e);
                }
            }
        }
    }
}

/// Summary statistics for the block list, used by the API and UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockListStats {
    /// Total number of entries (enabled + disabled).
    pub total: usize,
    /// Number of currently enabled entries.
    pub enabled: usize,
    /// Number of currently disabled entries.
    pub disabled: usize,
    /// Sum of hit counts across all entries.
    pub total_hits: u64,
}

// ============================================================================
// InterceptHandler implementation
// ============================================================================

#[async_trait::async_trait]
impl InterceptHandler for BlockListManager {
    fn name(&self) -> &'static str {
        "block_list"
    }

    fn priority(&self) -> u32 {
        // Block list runs first — before rewrites (10), mocks (20),
        // breakpoints (30), and throttle (40). A blocked request never
        // reaches upstream or other handlers.
        5
    }

    async fn on_request(&self, request: &mut RequestData) -> InterceptAction {
        if let Some(entry) = self.find_matching(&request.host) {
            tracing::debug!(
                "Block list matched: pattern={} host={} url={}",
                entry.pattern,
                request.host,
                request.url
            );
            // Increment hit count after cloning the entry (the write
            // lock is acquired separately to avoid holding two locks).
            self.increment_hit_count(&entry.id).await;

            let mut headers = std::collections::HashMap::new();
            headers.insert("Content-Type".to_string(), entry.content_type.clone());
            headers.insert(
                "X-Blocked-By".to_string(),
                format!("madhyamas-block-list:{}", entry.pattern),
            );

            return InterceptAction::Respond(ResponseData {
                status_code: entry.status_code,
                status_message: Some(block_status_message(entry.status_code)),
                headers,
                body: Some(entry.response_body.into_bytes()),
                content_type: Some(entry.content_type),
                duration_ms: 0,
                http_version: None,
            });
        }
        InterceptAction::Continue
    }
}

#[async_trait::async_trait]
impl crate::persistence::Persistable for BlockListManager {
    async fn save(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            let entries = self.entries.read().clone();
            for entry in &entries {
                store.save_block_list_entry(entry).await?;
            }
        }
        Ok(())
    }

    async fn load(&self) -> crate::Result<()> {
        if let Some(store) = &self.store {
            let loaded = store.load_block_list_entries().await?;
            *self.entries.write() = loaded;
        }
        Ok(())
    }

    async fn clear(&self) -> crate::Result<()> {
        self.clear().await;
        Ok(())
    }

    fn size(&self) -> usize {
        self.len()
    }
}

/// Return a standard HTTP status message for common block status codes.
fn block_status_message(code: u16) -> String {
    match code {
        403 => "Forbidden".to_string(),
        404 => "Not Found".to_string(),
        451 => "Unavailable For Legal Reasons".to_string(),
        502 => "Bad Gateway".to_string(),
        503 => "Service Unavailable".to_string(),
        _ => format!("Blocked ({code})"),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::Persistable;

    // ── Pattern matching: exact domain ──────────────────────────────

    #[test]
    fn exact_domain_matches_itself() {
        assert!(matches_pattern("example.com", "example.com"));
    }

    #[test]
    fn exact_domain_matches_subdomain() {
        assert!(matches_pattern("example.com", "api.example.com"));
        assert!(matches_pattern("example.com", "www.example.com"));
        assert!(matches_pattern("example.com", "a.b.c.example.com"));
    }

    #[test]
    fn exact_domain_does_not_match_unrelated() {
        assert!(!matches_pattern("example.com", "notexample.com"));
        assert!(!matches_pattern("example.com", "example.org"));
        assert!(!matches_pattern("example.com", "evil.com"));
    }

    // ── Pattern matching: leading wildcard *.domain ─────────────────

    #[test]
    fn leading_wildcard_matches_subdomain() {
        assert!(matches_pattern("*.example.com", "api.example.com"));
        assert!(matches_pattern("*.example.com", "www.example.com"));
        assert!(matches_pattern("*.example.com", "a.b.example.com"));
    }

    #[test]
    fn leading_wildcard_does_not_match_bare_domain() {
        assert!(!matches_pattern("*.example.com", "example.com"));
    }

    #[test]
    fn leading_wildcard_does_not_match_other_domain() {
        assert!(!matches_pattern("*.example.com", "api.example.org"));
    }

    // ── Pattern matching: general wildcards ─────────────────────────

    #[test]
    fn wildcard_in_tld_matches() {
        assert!(matches_pattern("ads.*", "ads.com"));
        assert!(matches_pattern("ads.*", "ads.net"));
        assert!(matches_pattern("ads.*", "ads.example.co.uk"));
    }

    #[test]
    fn wildcard_in_tld_does_not_match_no_dot() {
        // "ads.*" requires at least one character after the dot.
        // Actually "ads.*" → regex ^ads\..*$ which requires a dot.
        assert!(!matches_pattern("ads.*", "ads"));
    }

    #[test]
    fn substring_wildcard_matches() {
        assert!(matches_pattern("*ads*", "doubleclick.ads.com"));
        assert!(matches_pattern("*ads*", "ads.example.com"));
        assert!(matches_pattern("*ads*", "my-ads-server.com"));
    }

    #[test]
    fn substring_wildcard_does_not_match_unrelated() {
        assert!(!matches_pattern("*ads*", "example.com"));
    }

    #[test]
    fn wildcard_matches_everything() {
        assert!(matches_pattern("*", "example.com"));
        assert!(matches_pattern("*", "anything.org"));
    }

    // ── Pattern matching: case insensitivity ────────────────────────

    #[test]
    fn matching_is_case_insensitive() {
        assert!(matches_pattern("Example.COM", "api.example.com"));
        assert!(matches_pattern("*.Example.com", "API.EXAMPLE.COM"));
        assert!(matches_pattern("ADS.*", "ads.com"));
    }

    // ── Pattern matching: edge cases ────────────────────────────────

    #[test]
    fn trailing_dot_is_stripped() {
        assert!(matches_pattern("example.com.", "example.com"));
        assert!(matches_pattern("example.com", "example.com."));
    }

    #[test]
    fn empty_pattern_never_matches() {
        assert!(!matches_pattern("", "example.com"));
    }

    #[test]
    fn empty_host_never_matches() {
        assert!(!matches_pattern("example.com", ""));
    }

    #[test]
    fn pattern_with_regex_metachars_is_escaped() {
        // A pattern with a literal dot should not be treated as a regex
        // wildcard. "example.com" should match "example.com" but not
        // "examplexcom".
        assert!(!matches_pattern("example.com", "examplexcom"));
    }

    // ── BlockListEntry ──────────────────────────────────────────────

    #[test]
    fn entry_new_has_defaults() {
        let entry = BlockListEntry::new("ads.example.com".to_string());
        assert_eq!(entry.pattern, "ads.example.com");
        assert!(entry.enabled);
        assert_eq!(entry.status_code, 403);
        assert_eq!(entry.response_body, "Blocked by Madhyamas");
        assert_eq!(entry.content_type, "text/plain");
        assert_eq!(entry.hit_count, 0);
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn entry_with_note() {
        let entry = BlockListEntry::with_note("ads.com".to_string(), "Block ads".to_string());
        assert_eq!(entry.note.as_deref(), Some("Block ads"));
    }

    #[test]
    fn entry_serializes_and_deserializes() {
        let entry = BlockListEntry::new("example.com".to_string());
        let json = serde_json::to_string(&entry).unwrap();
        let back: BlockListEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry.id, back.id);
        assert_eq!(entry.pattern, back.pattern);
    }

    #[test]
    fn entry_deserializes_with_defaults_for_missing_fields() {
        // Simulates an old config that only has id and pattern.
        let json = r#"{"id":"test-1","pattern":"ads.com"}"#;
        let entry: BlockListEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.id, "test-1");
        assert_eq!(entry.pattern, "ads.com");
        assert!(entry.enabled); // default
        assert_eq!(entry.status_code, 403); // default
    }

    // ── BlockListManager CRUD ───────────────────────────────────────

    #[tokio::test]
    async fn manager_add_and_get_entries() {
        let mgr = BlockListManager::new();
        let id1 = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;
        let id2 = mgr
            .add_entry(BlockListEntry::new("tracker.com".to_string()))
            .await;

        assert_eq!(mgr.len(), 2);
        assert!(!mgr.is_empty());

        let entries = mgr.get_entries();
        assert!(entries.iter().any(|e| e.id == id1));
        assert!(entries.iter().any(|e| e.id == id2));
    }

    #[tokio::test]
    async fn manager_get_entry_by_id() {
        let mgr = BlockListManager::new();
        let id = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;

        assert!(mgr.get_entry(&id).is_some());
        assert!(mgr.get_entry("nonexistent").is_none());
    }

    #[tokio::test]
    async fn manager_remove_entry() {
        let mgr = BlockListManager::new();
        let id = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;

        assert!(mgr.remove_entry(&id).await);
        assert_eq!(mgr.len(), 0);
        assert!(!mgr.remove_entry(&id).await); // already removed
    }

    #[tokio::test]
    async fn manager_toggle_entry() {
        let mgr = BlockListManager::new();
        let id = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;

        assert!(mgr.get_entry(&id).unwrap().enabled);
        assert!(mgr.toggle_entry(&id, false).await);
        assert!(!mgr.get_entry(&id).unwrap().enabled);
        assert!(mgr.toggle_entry(&id, true).await);
        assert!(mgr.get_entry(&id).unwrap().enabled);
        assert!(!mgr.toggle_entry("nonexistent", true).await);
    }

    #[tokio::test]
    async fn manager_update_entry() {
        let mgr = BlockListManager::new();
        let id = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;

        let mut updated = BlockListEntry::new("ads.com".to_string());
        updated.status_code = 503;
        updated.response_body = "Service Unavailable".to_string();
        assert!(mgr.update_entry(&id, updated).await);

        let entry = mgr.get_entry(&id).unwrap();
        assert_eq!(entry.status_code, 503);
        assert_eq!(entry.response_body, "Service Unavailable");
        // ID is preserved.
        assert_eq!(entry.id, id);
    }

    #[tokio::test]
    async fn manager_update_nonexistent_returns_false() {
        let mgr = BlockListManager::new();
        let entry = BlockListEntry::new("ads.com".to_string());
        assert!(!mgr.update_entry("nonexistent", entry).await);
    }

    #[tokio::test]
    async fn manager_clear() {
        let mgr = BlockListManager::new();
        mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;
        mgr.add_entry(BlockListEntry::new("tracker.com".to_string()))
            .await;

        mgr.clear().await;
        assert_eq!(mgr.len(), 0);
        assert!(mgr.is_empty());
    }

    #[tokio::test]
    async fn manager_stats() {
        let mgr = BlockListManager::new();
        let id1 = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;
        mgr.add_entry(BlockListEntry::new("tracker.com".to_string()))
            .await;
        mgr.toggle_entry(&id1, false).await;

        let stats = mgr.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.enabled, 1);
        assert_eq!(stats.disabled, 1);
        assert_eq!(stats.total_hits, 0);
    }

    // ── InterceptHandler trait ──────────────────────────────────────

    #[test]
    fn handler_name_and_priority() {
        let mgr = BlockListManager::new();
        assert_eq!(mgr.name(), "block_list");
        assert_eq!(mgr.priority(), 5);
    }

    // ── Blocking behavior via on_request ────────────────────────────

    fn make_request(host: &str) -> RequestData {
        RequestData {
            method: crate::traffic::HttpMethod::Get,
            url: format!("https://{host}/api/test"),
            host: host.to_string(),
            path: "/api/test".to_string(),
            headers: std::collections::HashMap::new(),
            body: None,
            content_type: None,
            http_version: None,
        }
    }

    #[tokio::test]
    async fn on_request_blocks_matching_host() {
        let mgr = BlockListManager::new();
        mgr.add_entry(BlockListEntry::new("ads.example.com".to_string()))
            .await;

        let mut req = make_request("ads.example.com");
        let action = mgr.on_request(&mut req).await;

        match action {
            InterceptAction::Respond(resp) => {
                assert_eq!(resp.status_code, 403);
                assert_eq!(
                    String::from_utf8(resp.body.unwrap()).unwrap(),
                    "Blocked by Madhyamas"
                );
                assert_eq!(resp.content_type.as_deref(), Some("text/plain"));
            }
            other => panic!("Expected Respond, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_request_blocks_subdomain_of_pattern() {
        let mgr = BlockListManager::new();
        mgr.add_entry(BlockListEntry::new("example.com".to_string()))
            .await;

        let mut req = make_request("api.example.com");
        let action = mgr.on_request(&mut req).await;
        assert!(matches!(action, InterceptAction::Respond(_)));
    }

    #[tokio::test]
    async fn on_request_does_not_block_non_matching() {
        let mgr = BlockListManager::new();
        mgr.add_entry(BlockListEntry::new("ads.example.com".to_string()))
            .await;

        let mut req = make_request("safe.example.org");
        let action = mgr.on_request(&mut req).await;
        assert!(matches!(action, InterceptAction::Continue));
    }

    #[tokio::test]
    async fn on_request_does_not_block_disabled_entry() {
        let mgr = BlockListManager::new();
        let id = mgr
            .add_entry(BlockListEntry::new("ads.example.com".to_string()))
            .await;
        mgr.toggle_entry(&id, false).await;

        let mut req = make_request("ads.example.com");
        let action = mgr.on_request(&mut req).await;
        assert!(matches!(action, InterceptAction::Continue));
    }

    #[tokio::test]
    async fn on_request_increments_hit_count() {
        let mgr = BlockListManager::new();
        let id = mgr
            .add_entry(BlockListEntry::new("ads.example.com".to_string()))
            .await;

        let mut req = make_request("ads.example.com");
        let _ = mgr.on_request(&mut req).await;
        let _ = mgr.on_request(&mut req).await;
        let _ = mgr.on_request(&mut req).await;

        assert_eq!(mgr.get_entry(&id).unwrap().hit_count, 3);
    }

    #[tokio::test]
    async fn on_request_custom_status_and_body() {
        let mgr = BlockListManager::new();
        let mut entry = BlockListEntry::new("blocked.com".to_string());
        entry.status_code = 503;
        entry.response_body = "Service Unavailable".to_string();
        entry.content_type = "application/json".to_string();
        mgr.add_entry(entry).await;

        let mut req = make_request("blocked.com");
        let action = mgr.on_request(&mut req).await;

        match action {
            InterceptAction::Respond(resp) => {
                assert_eq!(resp.status_code, 503);
                assert_eq!(
                    String::from_utf8(resp.body.unwrap()).unwrap(),
                    "Service Unavailable"
                );
                assert_eq!(resp.content_type.as_deref(), Some("application/json"));
            }
            other => panic!("Expected Respond, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_request_adds_x_blocked_by_header() {
        let mgr = BlockListManager::new();
        mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;

        let mut req = make_request("ads.com");
        let action = mgr.on_request(&mut req).await;

        if let InterceptAction::Respond(resp) = action {
            assert!(resp.headers.contains_key("X-Blocked-By"));
            assert!(resp.headers["X-Blocked-By"].contains("ads.com"));
        } else {
            panic!("Expected Respond");
        }
    }

    #[tokio::test]
    async fn first_matching_entry_wins() {
        let mgr = BlockListManager::new();
        let mut e1 = BlockListEntry::new("ads.com".to_string());
        e1.status_code = 403;
        mgr.add_entry(e1).await;
        let mut e2 = BlockListEntry::new("ads.com".to_string());
        e2.status_code = 503;
        mgr.add_entry(e2).await;

        let mut req = make_request("ads.com");
        let action = mgr.on_request(&mut req).await;
        if let InterceptAction::Respond(resp) = action {
            // First entry (403) should win.
            assert_eq!(resp.status_code, 403);
        } else {
            panic!("Expected Respond");
        }
    }

    // ── Persistence (in-memory store) ───────────────────────────────

    async fn make_in_memory_store() -> Arc<dyn InterceptStoreBackend + Send + Sync> {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        Arc::new(
            crate::storage::SqliteInterceptStore::new(pool)
                .await
                .unwrap(),
        )
    }

    #[tokio::test]
    async fn persistence_load_save_roundtrip() {
        let store = make_in_memory_store().await;
        let mgr = BlockListManager::new().with_store(store.clone());

        mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;
        mgr.add_entry(BlockListEntry::with_note(
            "tracker.com".to_string(),
            "Analytics tracker".to_string(),
        ))
        .await;

        // Save to store.
        mgr.save().await.unwrap();

        // Create a new manager backed by the same store and load.
        let mgr2 = BlockListManager::new().with_store(store);
        mgr2.load().await.unwrap();

        assert_eq!(mgr2.len(), 2);
        let entries = mgr2.get_entries();
        assert!(entries.iter().any(|e| e.pattern == "ads.com"));
        assert!(entries.iter().any(|e| e.pattern == "tracker.com"));
    }

    #[tokio::test]
    async fn persistence_delete_removes_from_store() {
        let store = make_in_memory_store().await;
        let mgr = BlockListManager::new().with_store(store.clone());

        let id = mgr
            .add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;
        mgr.remove_entry(&id).await;

        // Loading from store should not find the deleted entry.
        let mgr2 = BlockListManager::new().with_store(store);
        mgr2.load().await.unwrap();
        assert_eq!(mgr2.len(), 0);
    }

    #[tokio::test]
    async fn persistence_clear_removes_all() {
        let store = make_in_memory_store().await;
        let mgr = BlockListManager::new().with_store(store.clone());

        mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
            .await;
        mgr.add_entry(BlockListEntry::new("tracker.com".to_string()))
            .await;
        mgr.save().await.unwrap();

        mgr.clear().await;

        let mgr2 = BlockListManager::new().with_store(store);
        mgr2.load().await.unwrap();
        assert_eq!(mgr2.len(), 0);
    }

    // ── Block status message ────────────────────────────────────────

    #[test]
    fn block_status_message_known_codes() {
        assert_eq!(block_status_message(403), "Forbidden");
        assert_eq!(block_status_message(404), "Not Found");
        assert_eq!(block_status_message(451), "Unavailable For Legal Reasons");
        assert_eq!(block_status_message(503), "Service Unavailable");
    }

    #[test]
    fn block_status_message_unknown_code() {
        let msg = block_status_message(418);
        assert!(msg.contains("418"));
    }
}
