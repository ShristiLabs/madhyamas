//! Session management for saving and loading traffic captures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::traffic::{Session, TrafficEntry};
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
    traffic_store: Arc<dyn crate::storage::TrafficStoreBackend + Send + Sync>,
}

impl SessionManager {
    pub fn new(traffic_store: Arc<dyn crate::storage::TrafficStoreBackend + Send + Sync>) -> Self {
        Self { traffic_store }
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Result<Vec<SessionSummary>, Error> {
        let sessions = self.traffic_store.list_sessions().await?;
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
    pub async fn get_session(&self, id: &str) -> Result<Option<Session>, Error> {
        let sessions = self.traffic_store.list_sessions().await?;
        Ok(sessions.into_iter().find(|s| s.id == id))
    }

    /// Create a new session
    pub async fn create_session(&self, name: Option<&str>) -> Result<Session, Error> {
        self.traffic_store.create_session(name).await
    }

    /// Delete a session and its traffic
    pub async fn delete_session(&self, id: &str) -> Result<(), Error> {
        self.traffic_store.delete_session(id).await
    }

    /// Export a session to HAR format
    pub async fn export_session(&self, id: &str) -> Result<SessionExport, Error> {
        let session = self
            .get_session(id)
            .await?
            .ok_or_else(|| Error::Sqlx(sqlx::Error::RowNotFound))?;

        let entries = self.traffic_store.get_traffic_by_session(id).await?;

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

    /// Import a session from export format.
    ///
    /// Only the currently supported export version ("1.0") is accepted.
    /// Future versions should add a migration step here before importing.
    pub async fn import_session(&self, export: SessionExport) -> Result<Session, Error> {
        // Version check — reject unsupported export versions.
        //
        // Migration path for future versions:
        //   * When bumping the export format (e.g. to "1.1" or "2.0"), add a
        //     match arm here that transforms the incoming `SessionExport` into
        //     the current in-memory representation before proceeding.
        //   * Keep old versions loadable whenever possible; only return an
        //     error when the format is too old/new to migrate safely.
        match export.version.as_str() {
            "1.0" => {}
            other => {
                return Err(Error::Config(format!(
                    "Unsupported session export version: '{}' (expected \"1.0\")",
                    other
                )));
            }
        }

        let session = self.create_session(export.session.name.as_deref()).await?;

        for entry in export.entries {
            let mut entry_with_session = entry;
            entry_with_session.session_id = session.id.clone();
            self.traffic_store
                .store_request(&entry_with_session)
                .await?;
            self.traffic_store
                .store_response(
                    &entry_with_session.id,
                    &entry_with_session.response.unwrap_or_default(),
                )
                .await?;
        }

        Ok(session)
    }

    /// Get session metadata
    pub async fn get_session_metadata(&self, id: &str) -> Result<Option<SessionMetadata>, Error> {
        let session = match self.get_session(id).await? {
            Some(s) => s,
            None => return Ok(None),
        };

        let entries = self.traffic_store.get_traffic_by_session(id).await?;

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
