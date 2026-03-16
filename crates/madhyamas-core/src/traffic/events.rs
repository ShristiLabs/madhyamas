//! Traffic event system for real-time WebSocket updates

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use super::TrafficEntry;

/// Events emitted when traffic changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum TrafficEvent {
    /// A new traffic entry was added (request captured)
    Added(TrafficEntrySnapshot),
    /// A traffic entry was updated (response received)
    Updated(TrafficEntrySnapshot),
    /// Specific traffic entries were deleted
    Deleted(Vec<String>),
    /// All traffic was cleared
    Cleared,
    /// Traffic count changed
    CountUpdate(usize),
}

/// Lightweight snapshot of a traffic entry for WebSocket transmission
/// Excludes large bodies to reduce bandwidth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficEntrySnapshot {
    pub id: String,
    pub session_id: String,
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub status_code: Option<u16>,
    pub status_message: Option<String>,
    pub content_type: Option<String>,
    pub response_content_type: Option<String>,
    pub duration_ms: Option<u64>,
    pub request_size: usize,
    pub response_size: Option<usize>,
    pub timestamp: String,
    pub modified: bool,
    pub has_request_body: bool,
    pub has_response_body: bool,
}

impl From<&TrafficEntry> for TrafficEntrySnapshot {
    fn from(entry: &TrafficEntry) -> Self {
        Self {
            id: entry.id.clone(),
            session_id: entry.session_id.clone(),
            method: entry.request.method.to_string(),
            url: entry.request.url.clone(),
            host: entry.request.host.clone(),
            path: entry.request.path.clone(),
            status_code: entry.response.as_ref().map(|r| r.status_code),
            status_message: entry.response.as_ref().and_then(|r| r.status_message.clone()),
            content_type: entry.request.content_type.clone(),
            response_content_type: entry.response.as_ref().and_then(|r| r.content_type.clone()),
            duration_ms: entry.response.as_ref().map(|r| r.duration_ms),
            request_size: entry.request.body.as_ref().map(|b| b.len()).unwrap_or(0),
            response_size: entry.response.as_ref().map(|r| r.body.as_ref().map(|b| b.len()).unwrap_or(0)),
            timestamp: entry.timestamp.to_rfc3339(),
            modified: entry.modified,
            has_request_body: entry.request.body.is_some(),
            has_response_body: entry.response.as_ref().map(|r| r.body.is_some()).unwrap_or(false),
        }
    }
}

/// WebSocket message types sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsServerMessage {
    /// Traffic event notification
    Traffic(TrafficEvent),
    /// Initial traffic list on connection
    InitialTraffic(Vec<TrafficEntrySnapshot>),
    /// Connection established acknowledgment
    Connected { client_id: String },
    /// Pong response to client ping
    Pong,
    /// Error message
    Error { message: String },
}

/// WebSocket message types sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsClientMessage {
    /// Subscribe to traffic updates with optional filter
    Subscribe {
        #[serde(default)]
        filter: Option<TrafficSubscriptionFilter>,
    },
    /// Unsubscribe from traffic updates
    Unsubscribe,
    /// Request initial traffic data
    GetInitialTraffic {
        #[serde(default)]
        limit: Option<usize>,
    },
    /// Ping to keep connection alive
    Ping,
}

/// Filter for traffic subscription
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficSubscriptionFilter {
    pub search: Option<String>,
    pub method: Option<String>,
    pub status_code: Option<String>,
}

/// Broadcast channel capacity for traffic events
pub const TRAFFIC_EVENT_CHANNEL_CAPACITY: usize = 1024;

/// Create a new traffic event broadcast channel
pub fn create_traffic_event_channel() -> (
    broadcast::Sender<TrafficEvent>,
    broadcast::Receiver<TrafficEvent>,
) {
    broadcast::channel(TRAFFIC_EVENT_CHANNEL_CAPACITY)
}
