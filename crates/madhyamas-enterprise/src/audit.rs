//! Audit logging for enterprise features

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use madhyamas_api::auth::{
    AuditError, AuditEvent as ApiAuditEvent, AuditEventType as ApiAuditEventType,
    AuditFilter as ApiAuditFilter, AuditSink,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

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

/// Audit logger
#[derive(Debug)]
pub struct AuditLogger {
    events: Mutex<Vec<AuditEvent>>,
    max_events: usize,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            max_events,
        }
    }

    /// Log an audit event
    pub fn log(&self, event: AuditEvent) {
        let mut events = self.events.lock();
        if events.len() >= self.max_events {
            events.remove(0);
        }
        events.push(event);
    }

    /// Query audit events with filter
    pub fn query(&self, filter: &AuditFilter) -> Vec<AuditEvent> {
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

    /// Get all events
    pub fn all_events(&self) -> Vec<AuditEvent> {
        self.events.lock().clone()
    }

    /// Clear all events
    pub fn clear(&self) {
        self.events.lock().clear();
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
        let events = self.query(&local_filter);
        Ok(events.into_iter().map(From::from).collect())
    }

    async fn export_events(&self, filter: &ApiAuditFilter) -> Result<Vec<u8>, AuditError> {
        let local_filter: AuditFilter = filter.clone().into();
        let events: Vec<ApiAuditEvent> = self
            .query(&local_filter)
            .into_iter()
            .map(From::from)
            .collect();
        serde_json::to_vec(&events).map_err(|e| AuditError::LogError {
            message: e.to_string(),
        })
    }
}
