//! Audit logging for enterprise features

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    events: Vec<AuditEvent>,
    max_events: usize,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
        }
    }

    /// Log an audit event
    pub fn log(&mut self, event: AuditEvent) {
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Query audit events with filter
    pub fn query(&self, filter: &AuditFilter) -> Vec<&AuditEvent> {
        self.events
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
            .collect()
    }

    /// Get all events
    pub fn all_events(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(10000)
    }
}
