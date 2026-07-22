//! Session management for saving and loading traffic captures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::traffic::{Session, TrafficEntry, TrafficStore};
use crate::Error;

/// Metadata about a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub request_count: usize,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

/// Summary of a session for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub request_count: usize,
}

/// Export format for sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    pub version: String,
    pub exported_at: DateTime<Utc>,
    pub session: SessionMetadata,
    pub entries: Vec<TrafficEntry>,
}

/// Preset for common session configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPreset {
    pub name: String,
    pub description: String,
    pub filter_host_patterns: Vec<String>,
    pub auto_clear_older_than_hours: Option<u64>,
}

/// Get common session presets
pub fn get_common_presets() -> Vec<SessionPreset> {
    vec![
        SessionPreset {
            name: "API Debugging".to_string(),
            description: "Focus on API traffic, filter out static assets".to_string(),
            filter_host_patterns: vec!["/api/.*".to_string()],
            auto_clear_older_than_hours: None,
        },
        SessionPreset {
            name: "Mobile App".to_string(),
            description: "Capture mobile app traffic with common API patterns".to_string(),
            filter_host_patterns: vec![],
            auto_clear_older_than_hours: None,
        },
        SessionPreset {
            name: "Performance Testing".to_string(),
            description: "Auto-clear old entries to maintain performance".to_string(),
            filter_host_patterns: vec![],
            auto_clear_older_than_hours: Some(24),
        },
    ]
}

/// Manages session persistence and export/import
pub struct SessionManager {
    traffic_store: Arc<TrafficStore>,
}

impl SessionManager {
    pub fn new(traffic_store: Arc<TrafficStore>) -> Self {
        Self { traffic_store }
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, Error> {
        let sessions = self.traffic_store.list_sessions()?;
        Ok(sessions
            .into_iter()
            .map(|s| SessionSummary {
                id: s.id,
                name: s.name,
                created_at: s.created_at,
                updated_at: s.updated_at,
                request_count: 0, // Would need to query count per session
            })
            .collect())
    }

    /// Get a specific session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<Session>, Error> {
        let sessions = self.traffic_store.list_sessions()?;
        Ok(sessions.into_iter().find(|s| s.id == id))
    }

    /// Create a new session
    pub fn create_session(&self, name: Option<&str>) -> Result<Session, Error> {
        self.traffic_store.create_session(name)
    }

    /// Delete a session and its traffic
    pub fn delete_session(&self, id: &str) -> Result<(), Error> {
        self.traffic_store.delete_session(id)
    }

    /// Export a session to HAR format
    pub fn export_session(&self, id: &str) -> Result<SessionExport, Error> {
        let session = self
            .get_session(id)?
            .ok_or_else(|| Error::Database(rusqlite::Error::QueryReturnedNoRows))?;

        let entries = self.traffic_store.get_traffic_by_session(id)?;

        Ok(SessionExport {
            version: "1.0".to_string(),
            exported_at: Utc::now(),
            session: SessionMetadata {
                id: session.id,
                name: session.name,
                created_at: session.created_at,
                updated_at: session.updated_at,
                request_count: entries.len(),
                notes: None,
                tags: Vec::new(),
            },
            entries,
        })
    }

    /// Import a session from export format
    pub fn import_session(&self, export: SessionExport) -> Result<Session, Error> {
        let session = self.create_session(export.session.name.as_deref())?;

        for entry in export.entries {
            let mut entry_with_session = entry;
            entry_with_session.session_id = session.id.clone();
            self.traffic_store.store_request(&entry_with_session)?;
            self.traffic_store.store_response(&entry_with_session.id, &entry_with_session.response.unwrap_or_default())?;
        }

        Ok(session)
    }

    /// Get session metadata
    pub fn get_session_metadata(&self, id: &str) -> Result<Option<SessionMetadata>, Error> {
        let session = match self.get_session(id)? {
            Some(s) => s,
            None => return Ok(None),
        };

        let entries = self.traffic_store.get_traffic_by_session(id)?;

        Ok(Some(SessionMetadata {
            id: session.id,
            name: session.name,
            created_at: session.created_at,
            updated_at: session.updated_at,
            request_count: entries.len(),
            notes: None,
            tags: Vec::new(),
        }))
    }
}
