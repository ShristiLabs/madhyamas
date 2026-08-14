//! Audit logging for enterprise features
//!
//! Phase 4e wires the [`AuditLogger`] to a persistent [`EnterpriseStore`] so
//! events survive restarts, and adds a SHA-256 hash chain for tamper
//! detection. Each event's `prev_hash` links to the previous event's `hash`,
//! forming an append-only chain. [`AuditLogger::verify_hash_chain`]
//! recomputes the chain and detects any tampering.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use madhyamas_api::auth::{
    AuditError, AuditEvent as ApiAuditEvent, AuditEventType as ApiAuditEventType,
    AuditFilter as ApiAuditFilter, AuditSink,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::store::EnterpriseStore;

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// User login
    Login,
    /// User logout
    Logout,
    /// API key created
    ApiKeyCreated,
    /// API key revoked
    ApiKeyRevoked,
    /// Traffic exported
    TrafficExported,
    /// Session created
    SessionCreated,
    /// Session deleted
    SessionDeleted,
    /// Mock rule created
    MockCreated,
    /// Mock rule deleted
    MockDeleted,
    /// Breakpoint created
    BreakpointCreated,
    /// Breakpoint deleted
    BreakpointDeleted,
    /// Configuration changed
    ConfigChanged,
    /// Custom event
    Custom,
}

/// Audit event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: String,
    /// Event type
    pub event_type: AuditEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// User who performed the action (if authenticated)
    pub user_id: Option<String>,
    /// API key used (if applicable)
    pub api_key_id: Option<String>,
    /// IP address of the client
    pub client_ip: Option<String>,
    /// Human-readable description
    pub description: String,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Hash of the previous event in the tamper-evident chain (Phase 4e).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub prev_hash: Option<String>,
    /// This event's own hash (SHA-256 of canonical fields + prev_hash).
    /// Populated by [`AuditLogger::log`] before persistence.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub hash: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType, description: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event_type,
            timestamp: Utc::now(),
            user_id: None,
            api_key_id: None,
            client_ip: None,
            description: description.into(),
            metadata: HashMap::new(),
            prev_hash: None,
            hash: None,
        }
    }

    /// Set the user ID
    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the API key ID
    pub fn with_api_key(mut self, api_key_id: impl Into<String>) -> Self {
        self.api_key_id = Some(api_key_id.into());
        self
    }

    /// Set the client IP
    pub fn with_client_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = Some(ip.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Filter for querying audit events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    /// Filter by event type
    pub event_type: Option<AuditEventType>,
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by start time
    pub start_time: Option<DateTime<Utc>>,
    /// Filter by end time
    pub end_time: Option<DateTime<Utc>>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Audit logger with optional persistent store backing and hash-chain
/// tamper detection.
///
/// When a store is attached via [`AuditLogger::with_store`], `log` persists
/// events asynchronously (fire-and-forget) and `query`/`get_audit_stats`/
/// `clear` delegate to the store. An in-memory ring buffer is always kept as
/// a fast recent-events cache.
pub struct AuditLogger {
    events: Mutex<Vec<AuditEvent>>,
    max_events: usize,
    store: Option<Arc<dyn EnterpriseStore>>,
}

impl std::fmt::Debug for AuditLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLogger")
            .field("max_events", &self.max_events)
            .field("store", &self.store.is_some())
            .finish()
    }
}

impl AuditLogger {
    /// Create a new audit logger with the given in-memory ring buffer size.
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            max_events,
            store: None,
        }
    }

    /// Attach a persistent enterprise store for audit event persistence.
    pub fn with_store(mut self, store: Arc<dyn EnterpriseStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Compute the SHA-256 hash of an audit event's canonical fields.
    ///
    /// The hash covers `id`, `event_type`, `timestamp`, `description`, and
    /// `prev_hash` — the core fields that form the tamper-evident chain.
    /// Metadata, user_id, etc. are intentionally excluded from the chain hash
    /// to keep it stable and focused on the event identity.
    pub fn compute_hash(event: &AuditEvent) -> String {
        let canonical = serde_json::json!({
            "id": event.id,
            "event_type": format!("{:?}", event.event_type),
            "timestamp": event.timestamp.to_rfc3339(),
            "description": event.description,
            "prev_hash": event.prev_hash,
        });
        let mut hasher = Sha256::new();
        hasher.update(canonical.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Log an audit event. When a store is attached, the event's hash chain
    /// is computed and the event is persisted asynchronously
    /// (fire-and-forget). The in-memory ring buffer is always updated.
    pub fn log(&self, mut event: AuditEvent) {
        // Compute hash chain: set prev_hash to the last in-memory event's
        // hash, then compute this event's own hash.
        {
            let events = self.events.lock();
            if let Some(last) = events.last() {
                event.prev_hash = last.hash.clone();
            }
        }
        event.hash = Some(Self::compute_hash(&event));

        let mut events = self.events.lock();
        if events.len() >= self.max_events {
            events.remove(0);
        }
        events.push(event.clone());
        drop(events);

        // Fire-and-forget persistence to the store.
        if let Some(ref store) = self.store {
            let store = Arc::clone(store);
            tokio::spawn(async move {
                let _ = store.log_audit_event(&event).await;
            });
        }
    }

    /// Query audit events with filter. Delegates to the store when attached;
    /// otherwise falls back to the in-memory ring buffer.
    pub async fn query(&self, filter: &AuditFilter) -> Vec<AuditEvent> {
        if let Some(ref store) = self.store {
            return store.query_audit_events(filter).await.unwrap_or_default();
        }
        self.query_in_memory(filter)
    }

    /// Query the in-memory ring buffer (synchronous, used as fallback).
    pub fn query_in_memory(&self, filter: &AuditFilter) -> Vec<AuditEvent> {
        let events = self.events.lock();
        events
            .iter()
            .filter(|event| {
                if let Some(ref event_type) = filter.event_type {
                    if event.event_type != *event_type {
                        return false;
                    }
                }
                if let Some(ref user_id) = filter.user_id {
                    if event.user_id.as_ref() != Some(user_id) {
                        return false;
                    }
                }
                if let Some(start_time) = filter.start_time {
                    if event.timestamp < start_time {
                        return false;
                    }
                }
                if let Some(end_time) = filter.end_time {
                    if event.timestamp > end_time {
                        return false;
                    }
                }
                true
            })
            .skip(filter.offset.unwrap_or(0))
            .take(filter.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect()
    }

    /// Get all events from the in-memory ring buffer.
    pub fn all_events(&self) -> Vec<AuditEvent> {
        self.events.lock().clone()
    }

    /// Clear all events. Delegates to the store when attached and also
    /// clears the in-memory ring buffer.
    pub async fn clear(&self) {
        if let Some(ref store) = self.store {
            let _ = store.clear_audit_events().await;
        }
        self.events.lock().clear();
    }

    /// Verify the hash chain of all persisted audit events.
    ///
    /// Recomputes each event's hash from its fields + the previous event's
    /// hash, and checks that the chain is consistent. Returns `Ok(true)` if
    /// the chain is intact, `Ok(false)` if tampering is detected, or `Err`
    /// if the store is not attached or the query fails.
    pub async fn verify_hash_chain(&self) -> Result<bool, AuditError> {
        let store = self.store.as_ref().ok_or_else(|| AuditError::LogError {
            message: "hash chain verification requires a persistent store".to_string(),
        })?;
        let events = store
            .query_audit_events(&AuditFilter {
                limit: None,
                offset: None,
                ..Default::default()
            })
            .await
            .map_err(|e| AuditError::LogError {
                message: format!("query failed: {e}"),
            })?;
        // Events come back ordered by timestamp DESC; reverse for chain
        // verification (oldest first).
        let mut events = events;
        events.reverse();
        let mut prev_hash: Option<String> = None;
        for event in &events {
            if event.prev_hash != prev_hash {
                return Ok(false);
            }
            let computed = Self::compute_hash(event);
            if event.hash.as_deref() != Some(computed.as_str()) {
                return Ok(false);
            }
            prev_hash = event.hash.clone();
        }
        Ok(true)
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}

// ---------------------------------------------------------------------------
// Conversions between local audit types and the API-layer trait types
// ---------------------------------------------------------------------------

impl From<ApiAuditEventType> for AuditEventType {
    fn from(value: ApiAuditEventType) -> Self {
        match value {
            ApiAuditEventType::Login => AuditEventType::Login,
            ApiAuditEventType::Logout => AuditEventType::Logout,
            ApiAuditEventType::ApiKeyCreated => AuditEventType::ApiKeyCreated,
            ApiAuditEventType::ApiKeyRevoked => AuditEventType::ApiKeyRevoked,
            ApiAuditEventType::TrafficExported => AuditEventType::TrafficExported,
            ApiAuditEventType::SessionCreated => AuditEventType::SessionCreated,
            ApiAuditEventType::SessionDeleted => AuditEventType::SessionDeleted,
            ApiAuditEventType::MockCreated => AuditEventType::MockCreated,
            ApiAuditEventType::MockDeleted => AuditEventType::MockDeleted,
            ApiAuditEventType::BreakpointCreated => AuditEventType::BreakpointCreated,
            ApiAuditEventType::BreakpointDeleted => AuditEventType::BreakpointDeleted,
            ApiAuditEventType::ConfigChanged => AuditEventType::ConfigChanged,
            ApiAuditEventType::Admin | ApiAuditEventType::Custom => AuditEventType::Custom,
        }
    }
}

impl From<AuditEventType> for ApiAuditEventType {
    fn from(value: AuditEventType) -> Self {
        match value {
            AuditEventType::Login => ApiAuditEventType::Login,
            AuditEventType::Logout => ApiAuditEventType::Logout,
            AuditEventType::ApiKeyCreated => ApiAuditEventType::ApiKeyCreated,
            AuditEventType::ApiKeyRevoked => ApiAuditEventType::ApiKeyRevoked,
            AuditEventType::TrafficExported => ApiAuditEventType::TrafficExported,
            AuditEventType::SessionCreated => ApiAuditEventType::SessionCreated,
            AuditEventType::SessionDeleted => ApiAuditEventType::SessionDeleted,
            AuditEventType::MockCreated => ApiAuditEventType::MockCreated,
            AuditEventType::MockDeleted => ApiAuditEventType::MockDeleted,
            AuditEventType::BreakpointCreated => ApiAuditEventType::BreakpointCreated,
            AuditEventType::BreakpointDeleted => ApiAuditEventType::BreakpointDeleted,
            AuditEventType::ConfigChanged => ApiAuditEventType::ConfigChanged,
            AuditEventType::Custom => ApiAuditEventType::Custom,
        }
    }
}

impl From<ApiAuditEvent> for AuditEvent {
    fn from(value: ApiAuditEvent) -> Self {
        Self {
            id: value.id,
            event_type: value.event_type.into(),
            timestamp: value.timestamp,
            user_id: value.user_id,
            api_key_id: value.api_key_id,
            client_ip: value.client_ip,
            description: value.description,
            metadata: value.metadata,
            prev_hash: None,
            hash: None,
        }
    }
}

impl From<AuditEvent> for ApiAuditEvent {
    fn from(value: AuditEvent) -> Self {
        Self {
            id: value.id,
            event_type: value.event_type.into(),
            timestamp: value.timestamp,
            user_id: value.user_id,
            api_key_id: value.api_key_id,
            client_ip: value.client_ip,
            description: value.description,
            metadata: value.metadata,
        }
    }
}

impl From<ApiAuditFilter> for AuditFilter {
    fn from(value: ApiAuditFilter) -> Self {
        Self {
            event_type: value.event_type.map(From::from),
            user_id: value.user_id,
            start_time: value.start_time,
            end_time: value.end_time,
            limit: value.limit,
            offset: value.offset,
        }
    }
}

#[async_trait]
impl AuditSink for AuditLogger {
    async fn log_event(&self, event: ApiAuditEvent) -> Result<(), AuditError> {
        self.log(event.into());
        Ok(())
    }

    async fn query_events(
        &self,
        filter: &ApiAuditFilter,
    ) -> Result<Vec<ApiAuditEvent>, AuditError> {
        let local_filter: AuditFilter = filter.clone().into();
        let events = self.query(&local_filter).await;
        Ok(events.into_iter().map(From::from).collect())
    }

    async fn export_events(&self, filter: &ApiAuditFilter) -> Result<Vec<u8>, AuditError> {
        let local_filter: AuditFilter = filter.clone().into();
        let events: Vec<ApiAuditEvent> = self
            .query(&local_filter)
            .await
            .into_iter()
            .map(From::from)
            .collect();
        serde_json::to_vec(&events).map_err(|e| AuditError::LogError {
            message: e.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_store() -> Arc<dyn EnterpriseStore> {
        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("open in-memory pool");
        Arc::new(
            crate::store::SqliteEnterpriseStore::new(pool)
                .await
                .expect("init store"),
        )
    }

    #[tokio::test]
    async fn test_log_and_query() {
        let store = test_store().await;
        let logger = AuditLogger::default().with_store(store.clone());

        logger.log(AuditEvent::new(AuditEventType::Login, "user logged in").with_user("u1"));
        logger.log(AuditEvent::new(AuditEventType::Logout, "user logged out").with_user("u1"));
        // Give fire-and-forget spawns time to persist.
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let events = logger
            .query(&AuditFilter {
                limit: Some(100),
                ..Default::default()
            })
            .await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_filter_by_user() {
        let store = test_store().await;
        let logger = AuditLogger::default().with_store(store.clone());

        logger.log(AuditEvent::new(AuditEventType::Login, "user1 login").with_user("user1"));
        logger.log(AuditEvent::new(AuditEventType::Login, "user2 login").with_user("user2"));
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let events = logger
            .query(&AuditFilter {
                user_id: Some("user1".to_string()),
                limit: Some(100),
                ..Default::default()
            })
            .await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].user_id.as_deref(), Some("user1"));
    }

    #[tokio::test]
    async fn test_hash_chain() {
        let store = test_store().await;
        let logger = AuditLogger::default().with_store(store.clone());

        logger.log(AuditEvent::new(AuditEventType::Login, "event 1"));
        logger.log(AuditEvent::new(AuditEventType::Logout, "event 2"));
        logger.log(AuditEvent::new(AuditEventType::Custom, "event 3"));
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let ok = logger.verify_hash_chain().await.expect("verify");
        assert!(ok, "hash chain should be intact");
    }

    #[tokio::test]
    async fn test_hash_chain_tamper_detection() {
        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("open in-memory pool");
        let store = Arc::new(
            crate::store::SqliteEnterpriseStore::new(pool.clone())
                .await
                .expect("init store"),
        ) as Arc<dyn EnterpriseStore>;
        let logger = AuditLogger::default().with_store(store.clone());

        logger.log(AuditEvent::new(AuditEventType::Login, "event 1"));
        logger.log(AuditEvent::new(AuditEventType::Logout, "event 2"));
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        // Tamper: directly modify an event's description in the DB, breaking
        // its hash without updating the chain.
        sqlx::query(
            "UPDATE audit_events SET description = 'tampered' WHERE description = 'event 1'",
        )
        .execute(&pool)
        .await
        .expect("tamper");

        let ok = logger.verify_hash_chain().await.expect("verify");
        assert!(!ok, "tampered chain should be detected");
    }
}
