//! Traffic storage using SQLite

use super::{
    CaptureStats, FocusHost, ImportResult, RequestData, ResponseData, Session, TrafficEntry,
    TrafficEntrySnapshot, TrafficEvent, TrafficFilter,
};
use crate::mirror::MirrorWriter;
use crate::Error;
use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Traffic store backed by SQLite
pub struct TrafficStore {
    conn: Mutex<Connection>,
    current_session_id: Mutex<String>,
    /// When false the proxy forwards traffic but does not record it (passthrough mode)
    capture_enabled: AtomicBool,
    /// Broadcast sender for traffic events (WebSocket real-time updates)
    event_sender: broadcast::Sender<TrafficEvent>,
    /// Maximum body size to store in bytes. Bodies larger than this are
    /// truncated before being written to the database. Default: 20 MB.
    max_body_size: std::sync::atomic::AtomicUsize,
    /// Maximum number of traffic entries to keep. When the count exceeds
    /// this limit, the oldest entries are pruned (FIFO). Default: 10,000.
    max_entries: AtomicUsize,
    /// Maximum total recording size in bytes (sum of all stored bodies).
    /// When `0`, no total-size limit is enforced. Default: 0 (unlimited).
    max_total_size_bytes: AtomicUsize,
    /// Whether to capture request bodies. When `false`, request bodies are
    /// not stored (headers and metadata are still recorded). Default: `true`.
    capture_request_bodies: AtomicBool,
    /// Whether to capture response bodies. When `false`, response bodies are
    /// not stored (headers and metadata are still recorded). Default: `true`.
    capture_response_bodies: AtomicBool,
    /// Domains whose traffic should not be recorded. Supports suffix and
    /// wildcard matching (e.g. `*.example.com` matches `api.example.com`).
    ignored_domains: RwLock<Vec<String>>,
    /// Monotonic insert counter used to throttle the (expensive) total-size
    /// check. The size check runs every `SIZE_CHECK_INTERVAL` inserts.
    insert_counter: AtomicUsize,
    /// Optional mirror writer for saving response bodies to disk. When set
    /// and enabled, captured responses are written to disk asynchronously
    /// after being stored in the database.
    mirror_writer: RwLock<Option<Arc<MirrorWriter>>>,
}

/// How often (in inserts) to run the total-size pruning check.
const SIZE_CHECK_INTERVAL: usize = 100;

impl TrafficStore {
    /// Create a new traffic store
    pub fn new<P: AsRef<Path>>(path: P) -> crate::Result<Arc<Self>> {
        let conn = Connection::open(path).map_err(Error::Database)?;
        let (event_sender, _) = broadcast::channel(super::TRAFFIC_EVENT_CHANNEL_CAPACITY);

        let store = Arc::new(Self {
            conn: Mutex::new(conn),
            current_session_id: Mutex::new(String::new()),
            capture_enabled: AtomicBool::new(true),
            event_sender,
            max_body_size: std::sync::atomic::AtomicUsize::new(20 * 1024 * 1024),
            max_entries: AtomicUsize::new(10_000),
            max_total_size_bytes: AtomicUsize::new(0),
            capture_request_bodies: AtomicBool::new(true),
            capture_response_bodies: AtomicBool::new(true),
            ignored_domains: RwLock::new(Vec::new()),
            insert_counter: AtomicUsize::new(0),
            mirror_writer: RwLock::new(None),
        });

        store.create_tables()?;
        store.ensure_session()?;

        Ok(store)
    }

    /// Create an in-memory traffic store
    pub fn in_memory() -> crate::Result<Arc<Self>> {
        let conn = Connection::open_in_memory().map_err(Error::Database)?;
        let (event_sender, _) = broadcast::channel(super::TRAFFIC_EVENT_CHANNEL_CAPACITY);

        let store = Arc::new(Self {
            conn: Mutex::new(conn),
            current_session_id: Mutex::new(String::new()),
            capture_enabled: AtomicBool::new(true),
            event_sender,
            max_body_size: std::sync::atomic::AtomicUsize::new(20 * 1024 * 1024),
            max_entries: AtomicUsize::new(10_000),
            max_total_size_bytes: AtomicUsize::new(0),
            capture_request_bodies: AtomicBool::new(true),
            capture_response_bodies: AtomicBool::new(true),
            ignored_domains: RwLock::new(Vec::new()),
            insert_counter: AtomicUsize::new(0),
            mirror_writer: RwLock::new(None),
        });

        store.create_tables()?;
        store.ensure_session()?;

        Ok(store)
    }

    /// Subscribe to traffic events
    pub fn subscribe(&self) -> broadcast::Receiver<TrafficEvent> {
        self.event_sender.subscribe()
    }

    /// Get the event sender for broadcasting traffic events
    pub fn event_sender(&self) -> broadcast::Sender<TrafficEvent> {
        self.event_sender.clone()
    }

    /// Emit a traffic event to all subscribers
    fn emit_event(&self, event: TrafficEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.event_sender.send(event);
    }

    /// Create database tables
    fn create_tables(&self) -> crate::Result<()> {
        let conn = self.conn.lock();

        // Enable WAL mode for better concurrent read/write performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(Error::Database)?;

        // Set busy timeout to 5 seconds to avoid "database is locked" errors
        // when multiple threads try to write simultaneously.
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(Error::Database)?;

        // Set synchronous to NORMAL (safe with WAL, much faster than FULL)
        conn.execute_batch("PRAGMA synchronous=NORMAL;")
            .map_err(Error::Database)?;

        // Increase cache size for better read performance
        conn.execute_batch("PRAGMA cache_size=-64000;") // 64MB cache
            .map_err(Error::Database)?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT,
                created_at INTEGER,
                updated_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS requests (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                host TEXT NOT NULL,
                path TEXT NOT NULL,
                headers TEXT,
                body BLOB,
                content_type TEXT,
                timestamp INTEGER,
                modified INTEGER DEFAULT 0,
                notes TEXT,
                is_passthrough INTEGER DEFAULT 0,
                http_version TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS responses (
                request_id TEXT PRIMARY KEY,
                status_code INTEGER NOT NULL,
                status_message TEXT,
                headers TEXT,
                body BLOB,
                content_type TEXT,
                duration_ms INTEGER,
                http_version TEXT,
                FOREIGN KEY (request_id) REFERENCES requests(id)
            );

            CREATE INDEX IF NOT EXISTS idx_requests_session ON requests(session_id);
            CREATE INDEX IF NOT EXISTS idx_requests_url ON requests(url);
            CREATE INDEX IF NOT EXISTS idx_requests_method ON requests(method);
            CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp);

            -- WebSocket connections table
            CREATE TABLE IF NOT EXISTS ws_connections (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                url TEXT NOT NULL,
                host TEXT NOT NULL,
                path TEXT NOT NULL,
                state TEXT NOT NULL,
                request_headers TEXT,
                response_headers TEXT,
                subprotocol TEXT,
                created_at INTEGER NOT NULL,
                closed_at INTEGER,
                messages_sent INTEGER NOT NULL DEFAULT 0,
                messages_received INTEGER NOT NULL DEFAULT 0,
                bytes_sent INTEGER NOT NULL DEFAULT 0,
                bytes_received INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            -- WebSocket messages table
            CREATE TABLE IF NOT EXISTS ws_messages (
                id TEXT PRIMARY KEY,
                connection_id TEXT NOT NULL,
                direction TEXT NOT NULL,
                message_type TEXT NOT NULL,
                payload_raw BLOB,
                payload_text TEXT,
                opcode INTEGER NOT NULL,
                is_final INTEGER NOT NULL DEFAULT 1,
                mask INTEGER,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (connection_id) REFERENCES ws_connections(id)
            );

            CREATE INDEX IF NOT EXISTS idx_ws_conn_session ON ws_connections(session_id);
            CREATE INDEX IF NOT EXISTS idx_ws_conn_state ON ws_connections(state);
            CREATE INDEX IF NOT EXISTS idx_ws_msg_conn ON ws_messages(connection_id);
            CREATE INDEX IF NOT EXISTS idx_ws_msg_timestamp ON ws_messages(timestamp);

            -- Focus hosts table (persisted host patterns for highlighting)
            CREATE TABLE IF NOT EXISTS focus_hosts (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL
            );
            "#,
        )
        .map_err(Error::Database)?;

        // Migration: add is_passthrough column to existing requests tables
        // that were created before this feature existed.
        let needs_migration: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(requests)")
                .map_err(Error::Database)?;
            let cols: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(Error::Database)?
                .filter_map(|r| r.ok())
                .collect();
            !cols.iter().any(|c| c == "is_passthrough")
        };
        if needs_migration {
            conn.execute_batch("ALTER TABLE requests ADD COLUMN is_passthrough INTEGER DEFAULT 0;")
                .map_err(Error::Database)?;
        }

        // Migration: add http_version column to requests/responses tables
        // for older databases created before HTTP/2 downstream support.
        let needs_req_h2: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(requests)")
                .map_err(Error::Database)?;
            let cols: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(Error::Database)?
                .filter_map(|r| r.ok())
                .collect();
            !cols.iter().any(|c| c == "http_version")
        };
        if needs_req_h2 {
            conn.execute_batch("ALTER TABLE requests ADD COLUMN http_version TEXT;")
                .map_err(Error::Database)?;
        }

        let needs_resp_h2: bool = {
            let mut stmt = conn
                .prepare("PRAGMA table_info(responses)")
                .map_err(Error::Database)?;
            let cols: Vec<String> = stmt
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(Error::Database)?
                .filter_map(|r| r.ok())
                .collect();
            !cols.iter().any(|c| c == "http_version")
        };
        if needs_resp_h2 {
            conn.execute_batch("ALTER TABLE responses ADD COLUMN http_version TEXT;")
                .map_err(Error::Database)?;
        }

        Ok(())
    }

    /// Ensure a session exists
    fn ensure_session(&self) -> crate::Result<()> {
        let conn = self.conn.lock();

        // Check if any session exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap_or(0);

        if count == 0 {
            // Create default session
            let session = Session::new(Some("Default Session"));
            conn.execute(
                "INSERT INTO sessions (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![
                    session.id,
                    session.name,
                    session.created_at.timestamp(),
                    session.updated_at.timestamp()
                ],
            )
            .map_err(Error::Database)?;

            *self.current_session_id.lock() = session.id;
        } else {
            // Get the most recent session
            let session_id: String = conn
                .query_row(
                    "SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or_default();

            *self.current_session_id.lock() = session_id;
        }

        Ok(())
    }

    /// Get the current session ID
    pub fn current_session_id(&self) -> String {
        self.current_session_id.lock().clone()
    }

    /// Returns whether traffic capture is currently active
    pub fn is_capture_enabled(&self) -> bool {
        self.capture_enabled.load(Ordering::Relaxed)
    }

    /// Enable or disable traffic capture (passthrough mode when false)
    pub fn set_capture_enabled(&self, enabled: bool) {
        self.capture_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Set the maximum body size (in bytes) to store. Bodies larger than
    /// this are truncated before being written to the database.
    pub fn set_max_body_size(&self, max: usize) {
        self.max_body_size.store(max, Ordering::Relaxed);
    }

    /// Get the current maximum body size (in bytes).
    pub fn max_body_size(&self) -> usize {
        self.max_body_size.load(Ordering::Relaxed)
    }

    /// Set the maximum number of traffic entries to keep. When the count
    /// exceeds this limit, the oldest entries are pruned (FIFO).
    pub fn set_max_entries(&self, max: usize) {
        self.max_entries.store(max, Ordering::Relaxed);
    }

    /// Get the current maximum entry count.
    pub fn max_entries(&self) -> usize {
        self.max_entries.load(Ordering::Relaxed)
    }

    /// Set the maximum total recording size in bytes. When `0`, no
    /// total-size limit is enforced.
    pub fn set_max_total_size_bytes(&self, max: usize) {
        self.max_total_size_bytes.store(max, Ordering::Relaxed);
    }

    /// Get the current maximum total recording size in bytes (`0` = unlimited).
    pub fn max_total_size_bytes(&self) -> usize {
        self.max_total_size_bytes.load(Ordering::Relaxed)
    }

    /// Set whether request bodies should be captured.
    pub fn set_capture_request_bodies(&self, enabled: bool) {
        self.capture_request_bodies
            .store(enabled, Ordering::Relaxed);
    }

    /// Whether request bodies are currently being captured.
    pub fn capture_request_bodies(&self) -> bool {
        self.capture_request_bodies.load(Ordering::Relaxed)
    }

    /// Set whether response bodies should be captured.
    pub fn set_capture_response_bodies(&self, enabled: bool) {
        self.capture_response_bodies
            .store(enabled, Ordering::Relaxed);
    }

    /// Whether response bodies are currently being captured.
    pub fn capture_response_bodies(&self) -> bool {
        self.capture_response_bodies.load(Ordering::Relaxed)
    }

    /// Set the list of ignored domains. Traffic from matching hosts is not
    /// recorded. Supports suffix and wildcard matching (e.g. `*.example.com`).
    pub fn set_ignored_domains(&self, domains: Vec<String>) {
        let cleaned: Vec<String> = domains
            .iter()
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect();
        *self.ignored_domains.write() = cleaned;
    }

    /// Get the current list of ignored domains.
    pub fn ignored_domains(&self) -> Vec<String> {
        self.ignored_domains.read().clone()
    }

    /// Set the mirror writer used to save response bodies to disk. When set
    /// and enabled, captured responses are written to disk asynchronously
    /// after being stored in the database.
    pub fn set_mirror_writer(&self, writer: Arc<MirrorWriter>) {
        *self.mirror_writer.write() = Some(writer);
    }

    /// Get the mirror writer, if one is attached.
    pub fn mirror_writer(&self) -> Option<Arc<MirrorWriter>> {
        self.mirror_writer.read().clone()
    }

    /// Check whether a host matches any of the ignored domain patterns.
    /// Matching is case-insensitive and supports:
    /// - Exact hostname: `example.com`
    /// - Suffix matching: `example.com` matches `api.example.com`
    /// - Wildcard subdomain: `*.example.com` matches `api.example.com`
    fn is_host_ignored(&self, host: &str) -> bool {
        let domains = self.ignored_domains.read();
        if domains.is_empty() {
            return false;
        }
        let target = host.trim().trim_end_matches('.').to_lowercase();
        if target.is_empty() {
            return false;
        }
        for pattern in domains.iter() {
            let pattern = pattern.trim().trim_end_matches('.');
            if pattern.is_empty() {
                continue;
            }
            // Wildcard subdomain: *.example.com
            if let Some(suffix) = pattern.strip_prefix("*.") {
                if target == suffix || target.ends_with(&format!(".{suffix}")) {
                    return true;
                }
                continue;
            }
            // Exact or suffix match
            if target == pattern || target.ends_with(&format!(".{pattern}")) {
                return true;
            }
        }
        false
    }

    /// Get the number of traffic entries in the current session.
    pub fn get_entry_count(&self) -> crate::Result<usize> {
        let conn = self.conn.lock();
        let session_id = self.current_session_id.lock().clone();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests WHERE session_id = ?1",
                params![&session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as usize)
    }

    /// Get the total size of all stored bodies (request + response) in the
    /// current session, in bytes.
    fn get_total_size(&self) -> crate::Result<usize> {
        let conn = self.conn.lock();
        let session_id = self.current_session_id.lock().clone();
        let req_size: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM requests WHERE session_id = ?1",
                params![&session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let resp_size: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM responses WHERE request_id IN \
                 (SELECT id FROM requests WHERE session_id = ?1)",
                params![&session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok((req_size + resp_size) as usize)
    }

    /// Prune the oldest `count` entries from the current session. Deletes
    /// associated responses first to avoid orphaned rows, then emits a
    /// `Deleted` event so the web UI updates via WebSocket.
    fn prune_oldest(&self, count: usize) -> crate::Result<()> {
        if count == 0 {
            return Ok(());
        }
        let conn = self.conn.lock();
        let session_id = self.current_session_id.lock().clone();

        // Collect the IDs of the oldest entries to be pruned (for the event).
        let pruned_ids: Vec<String> = {
            let mut stmt = conn
                .prepare(
                    "SELECT id FROM requests WHERE session_id = ?1 \
                     ORDER BY timestamp ASC LIMIT ?2",
                )
                .map_err(Error::Database)?;
            let rows = stmt
                .query_map(params![&session_id, count as i64], |row| {
                    row.get::<_, String>(0)
                })
                .map_err(Error::Database)?;
            let mut ids = Vec::new();
            for row in rows {
                ids.push(row.map_err(Error::Database)?);
            }
            ids
        };

        if pruned_ids.is_empty() {
            return Ok(());
        }

        // Delete responses for the pruned requests.
        let placeholders: Vec<String> = (1..=pruned_ids.len()).map(|i| format!("?{i}")).collect();
        let placeholders_str = placeholders.join(",");
        let delete_responses_sql =
            format!("DELETE FROM responses WHERE request_id IN ({placeholders_str})");
        let params_refs: Vec<&dyn rusqlite::ToSql> = pruned_ids
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();
        conn.execute(&delete_responses_sql, params_refs.as_slice())
            .map_err(Error::Database)?;

        // Delete the requests.
        let delete_requests_sql = format!("DELETE FROM requests WHERE id IN ({placeholders_str})");
        conn.execute(&delete_requests_sql, params_refs.as_slice())
            .map_err(Error::Database)?;

        // Emit a deleted event so the web UI removes the pruned entries.
        drop(conn);
        self.emit_event(TrafficEvent::Deleted(pruned_ids));

        Ok(())
    }

    /// Enforce the entry-count limit: if the current session has more
    /// entries than `max_entries`, prune the oldest surplus.
    fn enforce_entry_limit(&self) -> crate::Result<()> {
        let max = self.max_entries.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }
        let count = self.get_entry_count()?;
        if count > max {
            self.prune_oldest(count - max)?;
        }
        Ok(())
    }

    /// Enforce the total-size limit: if the sum of all stored bodies
    /// exceeds `max_total_size_bytes`, prune oldest entries until under
    /// the limit. Only runs when `max_total_size_bytes` is non-zero.
    fn enforce_size_limit(&self) -> crate::Result<()> {
        let max = self.max_total_size_bytes.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }
        let mut total = self.get_total_size()?;
        if total <= max {
            return Ok(());
        }
        // Prune oldest entries in batches until under the limit.
        let conn = self.conn.lock();
        let session_id = self.current_session_id.lock().clone();
        // Gather oldest entries with their body sizes so we can prune
        // just enough to get under the limit.
        let entries: Vec<(String, i64)> = {
            let mut stmt = conn
                .prepare(
                    "SELECT r.id, \
                     COALESCE(LENGTH(r.body), 0) + COALESCE(\
                       (SELECT LENGTH(rs.body) FROM responses rs WHERE rs.request_id = r.id), 0\
                     ) AS entry_size \
                     FROM requests r WHERE r.session_id = ?1 \
                     ORDER BY r.timestamp ASC",
                )
                .map_err(Error::Database)?;
            let rows = stmt
                .query_map(params![&session_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(Error::Database)?;
            let mut v = Vec::new();
            for row in rows {
                v.push(row.map_err(Error::Database)?);
            }
            v
        };
        drop(conn);

        let mut to_prune: Vec<String> = Vec::new();
        for (id, size) in entries {
            if total <= max {
                break;
            }
            to_prune.push(id);
            total = total.saturating_sub(size as usize);
        }
        if !to_prune.is_empty() {
            let conn = self.conn.lock();
            let placeholders: Vec<String> = (1..=to_prune.len()).map(|i| format!("?{i}")).collect();
            let placeholders_str = placeholders.join(",");
            let delete_responses_sql =
                format!("DELETE FROM responses WHERE request_id IN ({placeholders_str})");
            let params_refs: Vec<&dyn rusqlite::ToSql> =
                to_prune.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
            conn.execute(&delete_responses_sql, params_refs.as_slice())
                .map_err(Error::Database)?;
            let delete_requests_sql =
                format!("DELETE FROM requests WHERE id IN ({placeholders_str})");
            conn.execute(&delete_requests_sql, params_refs.as_slice())
                .map_err(Error::Database)?;
            drop(conn);
            self.emit_event(TrafficEvent::Deleted(to_prune));
        }
        Ok(())
    }

    /// Get recording quota statistics for the current session.
    pub fn get_capture_stats(&self) -> crate::Result<CaptureStats> {
        let entry_count = self.get_entry_count()?;
        let total_size_bytes = self.get_total_size()?;
        Ok(CaptureStats {
            entry_count,
            max_entries: self.max_entries.load(Ordering::Relaxed),
            total_size_bytes,
            max_total_size_bytes: self.max_total_size_bytes.load(Ordering::Relaxed),
            max_body_size: self.max_body_size.load(Ordering::Relaxed),
            capture_enabled: self.capture_enabled.load(Ordering::Relaxed),
            capture_request_bodies: self.capture_request_bodies.load(Ordering::Relaxed),
            capture_response_bodies: self.capture_response_bodies.load(Ordering::Relaxed),
            ignored_domains: self.ignored_domains.read().clone(),
        })
    }

    /// Truncate a body to the configured maximum size (in-place via clone).
    fn clamp_body(&self, body: &Option<Vec<u8>>) -> Option<Vec<u8>> {
        let max = self.max_body_size.load(Ordering::Relaxed);
        body.as_ref().map(|b| {
            if b.len() > max {
                let mut truncated = b.clone();
                truncated.truncate(max);
                truncated
            } else {
                b.clone()
            }
        })
    }

    /// Store a request
    pub fn store_request(&self, entry: &TrafficEntry) -> crate::Result<()> {
        if !self.capture_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }
        // Skip storage if the host matches an ignored domain pattern.
        if self.is_host_ignored(&entry.request.host) {
            return Ok(());
        }
        let conn = self.conn.lock();
        let headers = serde_json::to_string(&entry.request.headers).unwrap_or_default();
        // Only store the request body if capture_request_bodies is enabled.
        let body = if self.capture_request_bodies.load(Ordering::Relaxed) {
            self.clamp_body(&entry.request.body)
        } else {
            None
        };
        let content_type = entry.request.content_type.as_ref();

        conn.execute(
            "INSERT OR REPLACE INTO requests (id, session_id, method, url, host, path, headers, body, content_type, timestamp, modified, notes, is_passthrough, http_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                entry.id,
                entry.session_id,
                entry.request.method.to_string(),
                entry.request.url,
                entry.request.host,
                entry.request.path,
                headers,
                body,
                content_type,
                entry.timestamp.timestamp(),
                entry.modified as i32,
                entry.notes,
                entry.is_passthrough as i32,
                entry.request.http_version.as_deref()
            ]
        ).map_err(Error::Database)?;

        // Update session updated_at
        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![Utc::now().timestamp(), entry.session_id],
        )
        .ok();

        // Emit traffic added event
        let snapshot = TrafficEntrySnapshot::from(entry);
        drop(conn);
        self.emit_event(TrafficEvent::Added(snapshot));

        // Enforce entry-count limit (cheap check on every insert).
        self.enforce_entry_limit()?;

        // Enforce total-size limit periodically (expensive check).
        let prev = self.insert_counter.fetch_add(1, Ordering::Relaxed);
        if prev.is_multiple_of(SIZE_CHECK_INTERVAL) {
            self.enforce_size_limit()?;
        }

        Ok(())
    }

    /// Store a response for a request
    pub fn store_response(
        &self,
        request_id: &str,
        response: &crate::traffic::ResponseData,
    ) -> crate::Result<()> {
        if !self.capture_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }
        let conn = self.conn.lock();
        let headers = serde_json::to_string(&response.headers).unwrap_or_default();
        // Only store the response body if capture_response_bodies is enabled.
        let body = if self.capture_response_bodies.load(Ordering::Relaxed) {
            self.clamp_body(&response.body)
        } else {
            None
        };
        let content_type = response.content_type.as_ref();

        conn.execute(
            "INSERT OR REPLACE INTO responses (request_id, status_code, status_message, headers, body, content_type, duration_ms, http_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                request_id,
                response.status_code,
                response.status_message,
                headers,
                body,
                content_type,
                response.duration_ms as i64,
                response.http_version.as_deref()
            ]
        ).map_err(Error::Database)?;

        // Drop the connection lock before fetching the full entry
        drop(conn);

        // Emit traffic updated event with full entry data
        if let Ok(Some(entry)) = self.get_by_id(request_id) {
            let snapshot = TrafficEntrySnapshot::from(&entry);
            self.emit_event(TrafficEvent::Updated(snapshot));

            // Mirror the response to disk if a mirror writer is attached.
            // Passthrough entries have no captured body, so they are skipped.
            // The write is spawned on a background task to avoid blocking the
            // proxy pipeline.
            if !entry.is_passthrough {
                if let Some(mirror) = self.mirror_writer.read().clone() {
                    if mirror.is_enabled() {
                        let mirror = mirror.clone();
                        let entry = entry.clone();
                        let max_body_size = self.max_body_size();
                        tokio::spawn(async move {
                            let body_truncated = entry
                                .response
                                .as_ref()
                                .and_then(|r| r.body.as_ref())
                                .map(|b| b.len() > max_body_size)
                                .unwrap_or(false);
                            if let Some(response) = &entry.response {
                                if let Err(e) = mirror.write_response(
                                    &entry.request.host,
                                    &entry.request.path,
                                    &entry.request.method.to_string(),
                                    &entry.request.url,
                                    response,
                                    entry.timestamp,
                                    body_truncated,
                                ) {
                                    tracing::warn!(
                                        "Mirror write failed for {}: {}",
                                        entry.request.url,
                                        e
                                    );
                                }
                            }
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Get traffic with optional filter
    pub fn get_traffic(&self, filter: &TrafficFilter) -> crate::Result<Vec<TrafficEntry>> {
        let conn = self.conn.lock();

        let mut sql = String::from(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, r.body, r.content_type,
                    r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version,
                    rs.status_code, rs.status_message, rs.headers, rs.body, rs.content_type, rs.duration_ms, rs.http_version
             FROM requests r
             LEFT JOIN responses rs ON r.id = rs.request_id
             WHERE r.session_id = ?1"
        );

        let mut bind_params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(self.current_session_id.lock().clone())];

        if let Some(ref pattern) = filter.url_pattern {
            sql.push_str(&format!(" AND r.url LIKE ?{}", bind_params.len() + 1));
            bind_params.push(Box::new(format!("%{}%", pattern)));
        }

        if let Some(ref method) = filter.method {
            sql.push_str(&format!(" AND r.method = ?{}", bind_params.len() + 1));
            bind_params.push(Box::new(method.to_string()));
        }

        if let Some(min) = filter.status_min {
            sql.push_str(&format!(
                " AND rs.status_code >= ?{}",
                bind_params.len() + 1
            ));
            bind_params.push(Box::new(min as i32));
        }

        if let Some(max) = filter.status_max {
            sql.push_str(&format!(
                " AND rs.status_code <= ?{}",
                bind_params.len() + 1
            ));
            bind_params.push(Box::new(max as i32));
        }

        if let Some(ref search) = filter.search {
            sql.push_str(&format!(
                " AND (r.url LIKE ?{} OR r.path LIKE ?{})",
                bind_params.len() + 1,
                bind_params.len() + 2
            ));
            let search_pattern = format!("%{}%", search);
            bind_params.push(Box::new(search_pattern.clone()));
            bind_params.push(Box::new(search_pattern));
        }

        if let Some(ref file_type) = filter.file_type {
            sql.push_str(&format!(" AND r.path LIKE ?{}", bind_params.len() + 1));
            bind_params.push(Box::new(format!("%{}", file_type)));
        }

        if let Some(ref header) = filter.header {
            // Filter by header - supports "key:value" or just "key"
            sql.push_str(&format!(" AND r.headers LIKE ?{}", bind_params.len() + 1));
            bind_params.push(Box::new(format!("%{}%", header)));
        }

        if let Some(ref cookie) = filter.cookie {
            // Filter by cookie - check if Cookie header contains the value
            sql.push_str(&format!(" AND r.headers LIKE ?{}", bind_params.len() + 1));
            bind_params.push(Box::new(format!("%Cookie%{}%", cookie)));
        }

        if let Some(passthrough) = filter.is_passthrough {
            sql.push_str(&format!(
                " AND r.is_passthrough = ?{}",
                bind_params.len() + 1
            ));
            bind_params.push(Box::new(passthrough as i32));
        }

        if let Some(ref host) = filter.host {
            sql.push_str(&format!(" AND r.host LIKE ?{}", bind_params.len() + 1));
            bind_params.push(Box::new(format!("%{}%", host)));
        }

        sql.push_str(" ORDER BY r.timestamp DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let params_refs: Vec<&dyn rusqlite::ToSql> =
            bind_params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let entries = stmt
            .query_map(params_refs.as_slice(), |row| {
                let id: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let method: String = row.get(2)?;
                let url: String = row.get(3)?;
                let host: String = row.get(4)?;
                let path: String = row.get(5)?;
                let headers_json: String = row.get(6)?;
                let body: Option<Vec<u8>> = row.get(7)?;
                let content_type: Option<String> = row.get(8)?;
                let timestamp_i64: i64 = row.get(9)?;
                let modified: i32 = row.get(10)?;
                let notes: Option<String> = row.get(11)?;
                let is_passthrough: i32 = row.get(12)?;

                let req_http_version: Option<String> = row.get(13)?;
                let status_code: Option<i32> = row.get(14)?;
                let status_message: Option<String> = row.get(15)?;
                let resp_headers_json: Option<String> = row.get(16)?;
                let resp_body: Option<Vec<u8>> = row.get(17)?;
                let resp_content_type: Option<String> = row.get(18)?;
                let duration_ms: Option<i64> = row.get(19)?;
                let resp_http_version: Option<String> = row.get(20)?;

                let headers: std::collections::HashMap<String, String> =
                    serde_json::from_str(&headers_json).unwrap_or_default();

                let request = crate::traffic::RequestData {
                    method: method.parse().unwrap_or(crate::traffic::HttpMethod::Get),
                    url,
                    host,
                    path,
                    headers,
                    body,
                    content_type,
                    http_version: req_http_version,
                };

                let response = status_code.map(|code| {
                    let resp_headers: std::collections::HashMap<String, String> = resp_headers_json
                        .as_ref()
                        .and_then(|h| serde_json::from_str(h).ok())
                        .unwrap_or_default();

                    crate::traffic::ResponseData {
                        status_code: code as u16,
                        status_message,
                        headers: resp_headers,
                        body: resp_body,
                        content_type: resp_content_type,
                        duration_ms: duration_ms.unwrap_or(0) as u64,
                        http_version: resp_http_version,
                    }
                });

                let request_size = request.size();
                let response_size = response.as_ref().map(|r| r.size());
                Ok(TrafficEntry {
                    id,
                    session_id,
                    request,
                    response,
                    timestamp: DateTime::from_timestamp(timestamp_i64, 0).unwrap_or(Utc::now()),
                    modified: modified != 0,
                    notes,
                    request_size,
                    response_size,
                    is_passthrough: is_passthrough != 0,
                })
            })
            .map_err(Error::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::Database)?;

        Ok(entries)
    }

    /// Get a single traffic entry by ID
    pub fn get_by_id(&self, id: &str) -> crate::Result<Option<TrafficEntry>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, r.body, r.content_type,
                    r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version,
                    rs.status_code, rs.status_message, rs.headers, rs.body, rs.content_type, rs.duration_ms, rs.http_version
             FROM requests r
             LEFT JOIN responses rs ON r.id = rs.request_id
             WHERE r.id = ?1"
        ).map_err(Error::Database)?;

        let entry = stmt
            .query_row(params![id], |row| {
                let id: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let method: String = row.get(2)?;
                let url: String = row.get(3)?;
                let host: String = row.get(4)?;
                let path: String = row.get(5)?;
                let headers_json: String = row.get(6)?;
                let body: Option<Vec<u8>> = row.get(7)?;
                let content_type: Option<String> = row.get(8)?;
                let timestamp_i64: i64 = row.get(9)?;
                let modified: i32 = row.get(10)?;
                let notes: Option<String> = row.get(11)?;
                let is_passthrough: i32 = row.get(12)?;

                let req_http_version: Option<String> = row.get(13)?;
                let status_code: Option<i32> = row.get(14)?;
                let status_message: Option<String> = row.get(15)?;
                let resp_headers_json: Option<String> = row.get(16)?;
                let resp_body: Option<Vec<u8>> = row.get(17)?;
                let resp_content_type: Option<String> = row.get(18)?;
                let duration_ms: Option<i64> = row.get(19)?;
                let resp_http_version: Option<String> = row.get(20)?;

                let headers: std::collections::HashMap<String, String> =
                    serde_json::from_str(&headers_json).unwrap_or_default();

                let request = crate::traffic::RequestData {
                    method: method.parse().unwrap_or(crate::traffic::HttpMethod::Get),
                    url,
                    host,
                    path,
                    headers,
                    body,
                    content_type,
                    http_version: req_http_version,
                };

                let response = status_code.map(|code| {
                    let resp_headers: std::collections::HashMap<String, String> = resp_headers_json
                        .as_ref()
                        .and_then(|h| serde_json::from_str(h).ok())
                        .unwrap_or_default();

                    crate::traffic::ResponseData {
                        status_code: code as u16,
                        status_message,
                        headers: resp_headers,
                        body: resp_body,
                        content_type: resp_content_type,
                        duration_ms: duration_ms.unwrap_or(0) as u64,
                        http_version: resp_http_version,
                    }
                });

                let request_size = request.size();
                let response_size = response.as_ref().map(|r| r.size());
                Ok(TrafficEntry {
                    id,
                    session_id,
                    request,
                    response,
                    timestamp: DateTime::from_timestamp(timestamp_i64, 0).unwrap_or(Utc::now()),
                    modified: modified != 0,
                    notes,
                    request_size,
                    response_size,
                    is_passthrough: is_passthrough != 0,
                })
            })
            .ok();

        Ok(entry)
    }

    /// Clear all traffic for the current session
    pub fn clear_traffic(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        let session_id = self.current_session_id.lock().clone();

        conn.execute("DELETE FROM responses WHERE request_id IN (SELECT id FROM requests WHERE session_id = ?1)", params![&session_id])
            .map_err(Error::Database)?;

        conn.execute(
            "DELETE FROM requests WHERE session_id = ?1",
            params![&session_id],
        )
        .map_err(Error::Database)?;

        // Emit traffic cleared event
        drop(conn);
        self.emit_event(TrafficEvent::Cleared);

        Ok(())
    }

    /// Delete specific traffic entries by IDs
    pub fn delete_traffic(&self, ids: &[String]) -> crate::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();
        let placeholders: Vec<String> = (1..=ids.len()).map(|i| format!("?{}", i)).collect();
        let placeholders_str = placeholders.join(",");

        // Delete responses first
        let delete_responses_sql = format!(
            "DELETE FROM responses WHERE request_id IN ({})",
            placeholders_str
        );
        let params: Vec<&dyn rusqlite::ToSql> =
            ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
        conn.execute(&delete_responses_sql, params.as_slice())
            .map_err(Error::Database)?;

        // Delete requests
        let delete_requests_sql =
            format!("DELETE FROM requests WHERE id IN ({})", placeholders_str);
        conn.execute(&delete_requests_sql, params.as_slice())
            .map_err(Error::Database)?;

        // Emit traffic deleted event
        drop(conn);
        self.emit_event(TrafficEvent::Deleted(ids.to_vec()));

        Ok(())
    }

    /// Get traffic count
    pub fn count(&self) -> crate::Result<usize> {
        let conn = self.conn.lock();
        let session_id = self.current_session_id.lock().clone();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM requests WHERE session_id = ?1",
                params![&session_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        Ok(count as usize)
    }

    /// Export traffic as HAR format
    pub fn export_har(&self, session_id: &str) -> crate::Result<serde_json::Value> {
        let entries = self.get_traffic_by_session(session_id)?;

        let har = serde_json::json!({
            "log": {
                "version": "1.2",
                "creator": {
                    "name": "Madhyamas",
                    "version": "0.1.0"
                },
                "entries": entries.iter().map(|entry| {
                    serde_json::json!({
                        "startedDateTime": entry.timestamp.to_rfc3339(),
                        "request": {
                            "method": entry.request.method.to_string(),
                            "url": entry.request.url,
                            "httpVersion": entry.request.http_version_label(),
                            "headers": entry.request.headers.iter().map(|(k, v)| {
                                serde_json::json!({"name": k, "value": v})
                            }).collect::<Vec<_>>(),
                            "bodySize": entry.request.body.as_ref().map(|b| b.len()).unwrap_or(0),
                        },
                        "response": entry.response.as_ref().map(|resp| {
                            serde_json::json!({
                                "status": resp.status_code,
                                "statusText": resp.status_message.clone().unwrap_or_default(),
                                "httpVersion": resp.http_version_label(),
                                "headers": resp.headers.iter().map(|(k, v)| {
                                    serde_json::json!({"name": k, "value": v})
                                }).collect::<Vec<_>>(),
                                "content": {
                                    "size": resp.body.as_ref().map(|b| b.len()).unwrap_or(0),
                                    "mimeType": resp.content_type.clone().unwrap_or_default(),
                                }
                            })
                        }).unwrap_or(serde_json::json!(null)),
                        "time": entry.response.as_ref().map(|r| r.duration_ms).unwrap_or(0),
                    })
                }).collect::<Vec<_>>()
            }
        });

        Ok(har)
    }

    /// Import traffic from a HAR (HTTP Archive) JSON document.
    ///
    /// A new session is created (named after `session_name` or
    /// `"Imported HAR"` by default) and each `log.entries[]` entry is
    /// converted into a [`TrafficEntry`] and stored via [`store_request`] /
    /// [`store_response`].
    ///
    /// Invalid entries are skipped rather than aborting the entire import;
    /// their error messages are collected in the returned [`ImportResult`].
    /// Both HAR 1.1 and 1.2 are accepted. Base64-encoded bodies
    /// (`content.encoding == "base64"`) are decoded before storage.
    ///
    /// [`store_request`]: TrafficStore::store_request
    /// [`store_response`]: TrafficStore::store_response
    pub fn import_har(
        &self,
        har: &serde_json::Value,
        session_name: Option<&str>,
    ) -> crate::Result<ImportResult> {
        let log = har
            .get("log")
            .ok_or_else(|| Error::Config("Invalid HAR: missing 'log' field".to_string()))?;

        let entries = log
            .get("entries")
            .and_then(|e| e.as_array())
            .ok_or_else(|| Error::Config("Invalid HAR: missing 'log.entries' array".to_string()))?;

        // Create a new session for the imported traffic.
        let name = session_name.unwrap_or("Imported HAR");
        let session = self.create_session(Some(name))?;

        let mut imported_count = 0usize;
        let mut skipped_count = 0usize;
        let mut errors: Vec<String> = Vec::new();

        for (idx, entry) in entries.iter().enumerate() {
            match self.convert_har_entry(entry, &session.id) {
                Ok(entry) => {
                    if let Err(e) = self.store_request(&entry) {
                        skipped_count += 1;
                        errors.push(format!("entry {}: failed to store request: {}", idx, e));
                        continue;
                    }
                    if let Some(ref response) = entry.response {
                        if let Err(e) = self.store_response(&entry.id, response) {
                            skipped_count += 1;
                            errors.push(format!("entry {}: failed to store response: {}", idx, e));
                            continue;
                        }
                    }
                    imported_count += 1;
                }
                Err(e) => {
                    skipped_count += 1;
                    errors.push(format!("entry {}: {}", idx, e));
                }
            }
        }

        Ok(ImportResult {
            session_id: session.id,
            imported_count,
            skipped_count,
            errors,
        })
    }

    /// Convert a single HAR `log.entries[]` object into a [`TrafficEntry`]
    /// belonging to `session_id`. Returns an error when the entry is missing
    /// the required `request` object.
    fn convert_har_entry(
        &self,
        entry: &serde_json::Value,
        session_id: &str,
    ) -> crate::Result<TrafficEntry> {
        let request = entry
            .get("request")
            .ok_or_else(|| Error::Config("HAR entry missing 'request' field".to_string()))?;

        let method_str = request
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("GET");
        let method = method_str
            .parse::<crate::traffic::HttpMethod>()
            .unwrap_or(crate::traffic::HttpMethod::Get);

        let url = request
            .get("url")
            .and_then(|u| u.as_str())
            .unwrap_or("")
            .to_string();

        let (host, path) = parse_url(&url);

        let headers = parse_har_headers(request.get("headers"));
        let content_type = header_value(&headers, "content-type");

        let body = parse_har_post_data(request.get("postData"));
        let http_version = request
            .get("httpVersion")
            .and_then(|v| v.as_str())
            .map(normalize_http_version);

        let request_data = RequestData {
            method,
            url: url.clone(),
            host,
            path,
            headers,
            body,
            content_type,
            http_version,
        };

        let response = entry
            .get("response")
            .and_then(|r| r.as_object())
            .map(|resp| {
                let status_code = resp.get("status").and_then(|s| s.as_u64()).unwrap_or(0) as u16;
                let status_message = resp
                    .get("statusText")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                let resp_headers = parse_har_headers(resp.get("headers"));
                let resp_content_type = header_value(&resp_headers, "content-type").or_else(|| {
                    resp.get("content")
                        .and_then(|c| c.get("mimeType"))
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string())
                });
                let resp_body = parse_har_content(resp.get("content"));
                let duration_ms = entry
                    .get("time")
                    .and_then(|t| t.as_f64())
                    .map(|t| t as u64)
                    .unwrap_or(0);
                let resp_http_version = resp
                    .get("httpVersion")
                    .and_then(|v| v.as_str())
                    .map(normalize_http_version);

                ResponseData {
                    status_code,
                    status_message,
                    headers: resp_headers,
                    body: resp_body,
                    content_type: resp_content_type,
                    duration_ms,
                    http_version: resp_http_version,
                }
            });

        let timestamp = entry
            .get("startedDateTime")
            .and_then(|t| t.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let request_size = request_data.size();
        let response_size = response.as_ref().map(|r| r.size());

        Ok(TrafficEntry {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            request: request_data,
            response,
            timestamp,
            modified: false,
            notes: None,
            request_size,
            response_size,
            is_passthrough: false,
        })
    }

    /// List all sessions
    pub fn list_sessions(&self) -> crate::Result<Vec<Session>> {
        let conn = self.conn.lock();

        let sessions = conn
            .prepare(
                "SELECT id, name, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
            )
            .map_err(Error::Database)?
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: Option<String> = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                let updated_at: i64 = row.get(3)?;

                Ok(Session {
                    id,
                    name,
                    created_at: DateTime::from_timestamp(created_at, 0).unwrap_or(Utc::now()),
                    updated_at: DateTime::from_timestamp(updated_at, 0).unwrap_or(Utc::now()),
                })
            })
            .map_err(Error::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::Database)?;

        Ok(sessions)
    }

    /// Create a new session
    pub fn create_session(&self, name: Option<&str>) -> crate::Result<Session> {
        let session = Session::new(name);
        let conn = self.conn.lock();

        conn.execute(
            "INSERT INTO sessions (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                session.id,
                session.name,
                session.created_at.timestamp(),
                session.updated_at.timestamp()
            ],
        )
        .map_err(Error::Database)?;

        Ok(session)
    }

    /// Switch to a different session
    pub fn switch_session(&self, session_id: &str) -> crate::Result<()> {
        let conn = self.conn.lock();

        // Check if session exists
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .map_err(Error::Database)?;

        if count == 0 {
            return Err(Error::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        *self.current_session_id.lock() = session_id.to_string();
        Ok(())
    }

    /// Delete a session and all its traffic
    pub fn delete_session(&self, session_id: &str) -> crate::Result<()> {
        let conn = self.conn.lock();

        // Delete responses for requests in this session
        conn.execute(
            "DELETE FROM responses WHERE request_id IN (SELECT id FROM requests WHERE session_id = ?1)",
            params![session_id]
        ).map_err(Error::Database)?;

        // Delete requests in this session
        conn.execute(
            "DELETE FROM requests WHERE session_id = ?1",
            params![session_id],
        )
        .map_err(Error::Database)?;

        // Delete the session
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])
            .map_err(Error::Database)?;

        Ok(())
    }

    /// Get all traffic for a specific session
    pub fn get_traffic_by_session(&self, session_id: &str) -> crate::Result<Vec<TrafficEntry>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, r.body, r.content_type,
                    r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version,
                    rs.status_code, rs.status_message, rs.headers, rs.body, rs.content_type, rs.duration_ms, rs.http_version
             FROM requests r
             LEFT JOIN responses rs ON r.id = rs.request_id
             WHERE r.session_id = ?1
             ORDER BY r.timestamp DESC"
        ).map_err(Error::Database)?;

        let entries = stmt
            .query_map(params![session_id], |row| {
                let id: String = row.get(0)?;
                let session_id: String = row.get(1)?;
                let method: String = row.get(2)?;
                let url: String = row.get(3)?;
                let host: String = row.get(4)?;
                let path: String = row.get(5)?;
                let headers_json: String = row.get(6)?;
                let body: Option<Vec<u8>> = row.get(7)?;
                let content_type: Option<String> = row.get(8)?;
                let timestamp_i64: i64 = row.get(9)?;
                let modified: i32 = row.get(10)?;
                let notes: Option<String> = row.get(11)?;
                let is_passthrough: i32 = row.get(12)?;

                let req_http_version: Option<String> = row.get(13)?;
                let status_code: Option<i32> = row.get(14)?;
                let status_message: Option<String> = row.get(15)?;
                let resp_headers_json: Option<String> = row.get(16)?;
                let resp_body: Option<Vec<u8>> = row.get(17)?;
                let resp_content_type: Option<String> = row.get(18)?;
                let duration_ms: Option<i64> = row.get(19)?;
                let resp_http_version: Option<String> = row.get(20)?;

                let headers: std::collections::HashMap<String, String> =
                    serde_json::from_str(&headers_json).unwrap_or_default();

                let request = crate::traffic::RequestData {
                    method: method.parse().unwrap_or(crate::traffic::HttpMethod::Get),
                    url,
                    host,
                    path,
                    headers,
                    body,
                    content_type,
                    http_version: req_http_version,
                };

                let response = status_code.map(|code| {
                    let resp_headers: std::collections::HashMap<String, String> = resp_headers_json
                        .as_ref()
                        .and_then(|h| serde_json::from_str(h).ok())
                        .unwrap_or_default();

                    crate::traffic::ResponseData {
                        status_code: code as u16,
                        status_message,
                        headers: resp_headers,
                        body: resp_body,
                        content_type: resp_content_type,
                        duration_ms: duration_ms.unwrap_or(0) as u64,
                        http_version: resp_http_version,
                    }
                });

                let request_size = request.size();
                let response_size = response.as_ref().map(|r| r.size());
                Ok(TrafficEntry {
                    id,
                    session_id,
                    request,
                    response,
                    timestamp: DateTime::from_timestamp(timestamp_i64, 0).unwrap_or(Utc::now()),
                    modified: modified != 0,
                    notes,
                    request_size,
                    response_size,
                    is_passthrough: is_passthrough != 0,
                })
            })
            .map_err(Error::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::Database)?;

        Ok(entries)
    }

    // -----------------------------------------------------------------------
    // Focus hosts
    // -----------------------------------------------------------------------

    /// Add a focus host pattern. If the pattern already exists (case-insensitive),
    /// the existing entry is returned without creating a duplicate.
    pub fn add_focus_host(&self, pattern: &str) -> crate::Result<FocusHost> {
        let normalized = pattern.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(Error::Config(
                "focus host pattern cannot be empty".to_string(),
            ));
        }
        let conn = self.conn.lock();

        // Check for an existing entry with the same pattern.
        let existing: Option<FocusHost> = conn
            .query_row(
                "SELECT id, pattern, created_at FROM focus_hosts WHERE pattern = ?1",
                params![&normalized],
                |row| {
                    let id: String = row.get(0)?;
                    let pat: String = row.get(1)?;
                    let created_at: i64 = row.get(2)?;
                    Ok(FocusHost {
                        id,
                        pattern: pat,
                        created_at: DateTime::from_timestamp(created_at, 0).unwrap_or(Utc::now()),
                    })
                },
            )
            .ok();
        if let Some(entry) = existing {
            return Ok(entry);
        }

        let host = FocusHost::new(&normalized);
        conn.execute(
            "INSERT INTO focus_hosts (id, pattern, created_at) VALUES (?1, ?2, ?3)",
            params![&host.id, &host.pattern, host.created_at.timestamp()],
        )
        .map_err(Error::Database)?;
        Ok(host)
    }

    /// Remove a focus host by ID. Returns `true` if a row was deleted.
    pub fn remove_focus_host(&self, id: &str) -> crate::Result<bool> {
        let conn = self.conn.lock();
        let affected = conn
            .execute("DELETE FROM focus_hosts WHERE id = ?1", params![id])
            .map_err(Error::Database)?;
        Ok(affected > 0)
    }

    /// List all focus hosts ordered by creation time (oldest first).
    pub fn list_focus_hosts(&self) -> crate::Result<Vec<FocusHost>> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, pattern, created_at FROM focus_hosts ORDER BY created_at ASC")
            .map_err(Error::Database)?;
        let hosts = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let pattern: String = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                Ok(FocusHost {
                    id,
                    pattern,
                    created_at: DateTime::from_timestamp(created_at, 0).unwrap_or(Utc::now()),
                })
            })
            .map_err(Error::Database)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Error::Database)?;
        Ok(hosts)
    }

    /// Remove all focus hosts.
    pub fn clear_focus_hosts(&self) -> crate::Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM focus_hosts", [])
            .map_err(Error::Database)?;
        Ok(())
    }
}

/// Parse a full URL string into `(host, path)` components.
///
/// Uses the `url` crate when the string is a valid absolute URL; otherwise
/// falls back to a simple manual split on the first `/` after the host.
fn parse_url(url: &str) -> (String, String) {
    if let Ok(parsed) = url::Url::parse(url) {
        let host = parsed.host_str().unwrap_or("").to_string();
        let path = if let Some(query) = parsed.query() {
            format!("{}?{}", parsed.path(), query)
        } else {
            parsed.path().to_string()
        };
        (host, path)
    } else {
        // Fallback: try to split manually for relative or unusual URLs.
        if let Some(rest) = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
        {
            if let Some((h, p)) = rest.split_once('/') {
                (h.to_string(), format!("/{}", p))
            } else {
                (rest.to_string(), String::from("/"))
            }
        } else if let Some((h, p)) = url.split_once('/') {
            (h.to_string(), format!("/{}", p))
        } else {
            (url.to_string(), String::from("/"))
        }
    }
}

/// Convert a HAR `httpVersion` string (e.g. `"HTTP/1.1"`, `"http/2.0"`) into
/// the canonical form used by Madhyamas (`"HTTP/1.1"`, `"HTTP/2"`).
fn normalize_http_version(version: &str) -> String {
    let upper = version.to_uppercase();
    match upper.as_str() {
        "HTTP/1.0" | "HTTP/1" => "HTTP/1.0".to_string(),
        "HTTP/1.1" => "HTTP/1.1".to_string(),
        "HTTP/2" | "HTTP/2.0" | "H2" => "HTTP/2".to_string(),
        "HTTP/3" | "HTTP/3.0" | "H3" => "HTTP/3".to_string(),
        _ => version.to_string(),
    }
}

/// Parse a HAR `headers` array (`[{"name":..,"value":..}, ...]`) into a
/// `HashMap<String, String>`. Malformed entries are silently skipped.
fn parse_har_headers(headers: Option<&serde_json::Value>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(arr) = headers.and_then(|h| h.as_array()) {
        for h in arr {
            if let (Some(name), Some(value)) = (
                h.get("name").and_then(|n| n.as_str()),
                h.get("value").and_then(|v| v.as_str()),
            ) {
                if !name.is_empty() {
                    map.insert(name.to_string(), value.to_string());
                }
            }
        }
    }
    map
}

/// Look up a header value case-insensitively.
fn header_value(headers: &HashMap<String, String>, name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

/// Parse a HAR request `postData` object into an optional byte body.
/// Handles `encoding: "base64"` for binary payloads.
fn parse_har_post_data(post_data: Option<&serde_json::Value>) -> Option<Vec<u8>> {
    let pd = post_data?;
    let text = pd.get("text").and_then(|t| t.as_str())?;
    let encoding = pd.get("encoding").and_then(|e| e.as_str()).unwrap_or("");
    if encoding.eq_ignore_ascii_case("base64") {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(text)
            .ok()
            .or_else(|| Some(text.as_bytes().to_vec()))
    } else {
        Some(text.as_bytes().to_vec())
    }
}

/// Parse a HAR response `content` object into an optional byte body.
/// Handles `encoding: "base64"` for binary payloads.
fn parse_har_content(content: Option<&serde_json::Value>) -> Option<Vec<u8>> {
    let content = content?;
    let text = content.get("text").and_then(|t| t.as_str())?;
    let encoding = content
        .get("encoding")
        .and_then(|e| e.as_str())
        .unwrap_or("");
    if encoding.eq_ignore_ascii_case("base64") {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(text)
            .ok()
            .or_else(|| Some(text.as_bytes().to_vec()))
    } else {
        Some(text.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod har_import_tests {
    use super::*;
    use base64::Engine;
    use serde_json::json;

    fn make_store() -> Arc<TrafficStore> {
        TrafficStore::in_memory().expect("failed to create in-memory store")
    }

    #[test]
    fn test_import_har_two_entries() {
        let store = make_store();
        let har = json!({
            "log": {
                "version": "1.2",
                "creator": { "name": "test", "version": "1.0" },
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00Z",
                        "time": 42.0,
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/api/users",
                            "httpVersion": "HTTP/1.1",
                            "headers": [{"name": "Accept", "value": "application/json"}]
                        },
                        "response": {
                            "status": 200,
                            "statusText": "OK",
                            "httpVersion": "HTTP/1.1",
                            "headers": [{"name": "Content-Type", "value": "application/json"}],
                            "content": { "size": 17, "mimeType": "application/json", "text": "{\"users\":[]}" }
                        }
                    },
                    {
                        "startedDateTime": "2024-01-01T00:00:01Z",
                        "time": 10.0,
                        "request": {
                            "method": "POST",
                            "url": "https://example.com/api/login",
                            "headers": [{"name": "Content-Type", "value": "application/json"}],
                            "postData": { "mimeType": "application/json", "text": "{\"user\":\"a\"}" }
                        },
                        "response": {
                            "status": 204,
                            "statusText": "No Content",
                            "headers": [],
                            "content": { "size": 0, "mimeType": "" }
                        }
                    }
                ]
            }
        });

        let result = store.import_har(&har, None).expect("import should succeed");
        assert_eq!(result.imported_count, 2);
        assert_eq!(result.skipped_count, 0);
        assert!(result.errors.is_empty());

        let entries = store
            .get_traffic_by_session(&result.session_id)
            .expect("fetch entries");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_import_har_missing_response() {
        let store = make_store();
        let har = json!({
            "log": {
                "version": "1.2",
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00Z",
                        "time": 0,
                        "request": {
                            "method": "GET",
                            "url": "https://example.com/pending"
                        }
                    }
                ]
            }
        });

        let result = store.import_har(&har, None).expect("import should succeed");
        assert_eq!(result.imported_count, 1);
        assert_eq!(result.skipped_count, 0);

        let entries = store
            .get_traffic_by_session(&result.session_id)
            .expect("fetch entries");
        assert_eq!(entries.len(), 1);
        assert!(entries[0].response.is_none());
    }

    #[test]
    fn test_import_har_base64_body() {
        let store = make_store();
        // "Hello" base64-encoded
        let encoded = base64::engine::general_purpose::STANDARD.encode(b"Hello");
        let har = json!({
            "log": {
                "version": "1.2",
                "entries": [
                    {
                        "startedDateTime": "2024-01-01T00:00:00Z",
                        "time": 0,
                        "request": {
                            "method": "POST",
                            "url": "https://example.com/upload",
                            "postData": { "text": encoded, "encoding": "base64" }
                        },
                        "response": {
                            "status": 200,
                            "headers": [],
                            "content": { "text": encoded, "encoding": "base64" }
                        }
                    }
                ]
            }
        });

        let result = store.import_har(&har, None).expect("import should succeed");
        assert_eq!(result.imported_count, 1);

        let entries = store
            .get_traffic_by_session(&result.session_id)
            .expect("fetch entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].request.body.as_deref(), Some(b"Hello" as &[u8]));
        assert_eq!(
            entries[0].response.as_ref().unwrap().body.as_deref(),
            Some(b"Hello" as &[u8])
        );
    }

    #[test]
    fn test_import_har_invalid_missing_log() {
        let store = make_store();
        let har = json!({ "foo": "bar" });

        let result = store.import_har(&har, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_import_har_entry_missing_request_skipped() {
        let store = make_store();
        let har = json!({
            "log": {
                "version": "1.2",
                "entries": [
                    { "startedDateTime": "2024-01-01T00:00:00Z", "time": 0 },
                    {
                        "startedDateTime": "2024-01-01T00:00:01Z",
                        "time": 0,
                        "request": { "method": "GET", "url": "https://example.com/ok" }
                    }
                ]
            }
        });

        let result = store.import_har(&har, None).expect("import should succeed");
        assert_eq!(result.imported_count, 1);
        assert_eq!(result.skipped_count, 1);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_import_har_round_trip() {
        let store = make_store();

        // Create a session with a couple of entries via the store.
        let session = store
            .create_session(Some("Round Trip"))
            .expect("create session");
        store.switch_session(&session.id).expect("switch session");

        let req1 = RequestData {
            method: crate::traffic::HttpMethod::Get,
            url: "https://example.com/api/1".to_string(),
            host: "example.com".to_string(),
            path: "/api/1".to_string(),
            headers: {
                let mut m = HashMap::new();
                m.insert("Accept".to_string(), "text/html".to_string());
                m
            },
            body: None,
            content_type: None,
            http_version: Some("HTTP/1.1".to_string()),
        };
        let mut entry1 = TrafficEntry::new(&session.id, req1);
        entry1.response = Some(ResponseData {
            status_code: 200,
            status_message: Some("OK".to_string()),
            headers: HashMap::new(),
            body: Some(b"hello".to_vec()),
            content_type: Some("text/html".to_string()),
            duration_ms: 5,
            http_version: Some("HTTP/1.1".to_string()),
        });
        entry1.response_size = Some(entry1.response.as_ref().unwrap().size());
        store.store_request(&entry1).expect("store req1");
        store
            .store_response(&entry1.id, entry1.response.as_ref().unwrap())
            .expect("store resp1");

        // Export to HAR, then import it back.
        let har = store.export_har(&session.id).expect("export har");
        let result = store
            .import_har(&har, Some("Imported Round Trip"))
            .expect("import har");

        assert_eq!(result.imported_count, 1);
        assert_eq!(result.skipped_count, 0);

        let imported = store
            .get_traffic_by_session(&result.session_id)
            .expect("fetch imported");
        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].request.method, crate::traffic::HttpMethod::Get);
        assert_eq!(imported[0].request.url, "https://example.com/api/1");
        assert_eq!(imported[0].response.as_ref().unwrap().status_code, 200);
    }

    #[test]
    fn test_parse_url_absolute() {
        let (host, path) = parse_url("https://api.example.com/v1/users?page=1");
        assert_eq!(host, "api.example.com");
        assert_eq!(path, "/v1/users?page=1");
    }

    #[test]
    fn test_parse_url_no_scheme() {
        let (host, path) = parse_url("example.com/path");
        assert_eq!(host, "example.com");
        assert_eq!(path, "/path");
    }

    #[test]
    fn test_normalize_http_version() {
        assert_eq!(normalize_http_version("HTTP/1.1"), "HTTP/1.1");
        assert_eq!(normalize_http_version("http/2.0"), "HTTP/2");
        assert_eq!(normalize_http_version("h2"), "HTTP/2");
        assert_eq!(normalize_http_version("HTTP/3"), "HTTP/3");
    }
}

#[cfg(test)]
mod recording_limits_tests {
    use super::*;
    use crate::traffic::host_matches_pattern;
    use std::collections::HashMap;

    fn make_store() -> Arc<TrafficStore> {
        TrafficStore::in_memory().expect("failed to create in-memory store")
    }

    fn make_entry(session_id: &str, host: &str, path: &str, body: Option<Vec<u8>>) -> TrafficEntry {
        let request = RequestData {
            method: crate::traffic::HttpMethod::Get,
            url: format!("https://{host}{path}"),
            host: host.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body,
            content_type: None,
            http_version: Some("HTTP/1.1".to_string()),
        };
        let mut entry = TrafficEntry::new(session_id, request);
        // Ensure the entry uses the provided session_id
        entry.session_id = session_id.to_string();
        entry
    }

    fn make_response(body: Option<Vec<u8>>) -> ResponseData {
        ResponseData {
            status_code: 200,
            status_message: Some("OK".to_string()),
            headers: HashMap::new(),
            body,
            content_type: Some("application/json".to_string()),
            duration_ms: 10,
            http_version: Some("HTTP/1.1".to_string()),
        }
    }

    #[test]
    fn test_max_entries_prunes_oldest() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_max_entries(5);

        for i in 0..10 {
            let entry = make_entry(&session_id, "example.com", &format!("/p{i}"), None);
            store.store_request(&entry).expect("store request");
            // Small sleep to ensure distinct timestamps
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        let count = store.get_entry_count().expect("entry count");
        assert_eq!(count, 5, "should have pruned to 5 entries");

        // The 5 most recent should remain (paths /p5 through /p9)
        let filter = TrafficFilter::default();
        let entries = store.get_traffic(&filter).expect("get traffic");
        let paths: Vec<&str> = entries.iter().map(|e| e.request.path.as_str()).collect();
        for i in 5..10 {
            let p = format!("/p{i}");
            assert!(paths.contains(&p.as_str()), "path {p} should remain");
        }
    }

    #[test]
    fn test_pruned_responses_are_deleted() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_max_entries(3);

        // Insert 5 entries, each with a response.
        for i in 0..5 {
            let mut entry = make_entry(&session_id, "example.com", &format!("/r{i}"), None);
            store.store_request(&entry).expect("store request");
            entry.response = Some(make_response(Some(format!("resp{i}").into_bytes())));
            store
                .store_response(&entry.id, entry.response.as_ref().unwrap())
                .expect("store response");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        // Only 3 entries should remain
        assert_eq!(store.get_entry_count().unwrap(), 3);

        // Verify no orphaned responses: count responses that belong to
        // remaining requests
        let conn = store.conn.lock();
        let orphaned: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM responses WHERE request_id NOT IN \
                 (SELECT id FROM requests)",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        drop(conn);
        assert_eq!(orphaned, 0, "no orphaned responses should remain");
    }

    #[test]
    fn test_capture_request_bodies_disabled() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_capture_request_bodies(false);

        let entry = make_entry(
            &session_id,
            "example.com",
            "/api",
            Some(b"request body".to_vec()),
        );
        store.store_request(&entry).expect("store request");

        let stored = store
            .get_by_id(&entry.id)
            .expect("get entry")
            .expect("entry exists");
        assert!(
            stored.request.body.is_none(),
            "request body should not be stored when capture_request_bodies is false"
        );
    }

    #[test]
    fn test_capture_response_bodies_disabled() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_capture_response_bodies(false);

        let entry = make_entry(
            &session_id,
            "example.com",
            "/api",
            Some(b"request body".to_vec()),
        );
        store.store_request(&entry).expect("store request");
        let resp = make_response(Some(b"response body".to_vec()));
        store
            .store_response(&entry.id, &resp)
            .expect("store response");

        let stored = store
            .get_by_id(&entry.id)
            .expect("get entry")
            .expect("entry exists");
        let stored_resp = stored.response.expect("response exists");
        assert!(
            stored_resp.body.is_none(),
            "response body should not be stored when capture_response_bodies is false"
        );
    }

    #[test]
    fn test_ignored_domains_skips_storage() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_ignored_domains(vec!["*.example.com".to_string()]);

        // Request to example.com should be skipped
        let entry1 = make_entry(&session_id, "api.example.com", "/skip", None);
        store.store_request(&entry1).expect("store request");

        // Request to other.com should be stored
        let entry2 = make_entry(&session_id, "other.com", "/keep", None);
        store.store_request(&entry2).expect("store request");

        assert_eq!(
            store.get_entry_count().unwrap(),
            1,
            "only non-ignored entry should be stored"
        );
    }

    #[test]
    fn test_ignored_domains_exact_match() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_ignored_domains(vec!["blocked.com".to_string()]);

        let entry = make_entry(&session_id, "blocked.com", "/path", None);
        store.store_request(&entry).expect("store request");

        assert_eq!(
            store.get_entry_count().unwrap(),
            0,
            "exact match should be ignored"
        );
    }

    #[test]
    fn test_ignored_domains_suffix_match() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_ignored_domains(vec!["analytics.com".to_string()]);

        let entry = make_entry(&session_id, "api.analytics.com", "/track", None);
        store.store_request(&entry).expect("store request");

        assert_eq!(
            store.get_entry_count().unwrap(),
            0,
            "suffix match should be ignored"
        );
    }

    #[test]
    fn test_get_capture_stats() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_max_entries(100);
        store.set_max_total_size_bytes(1024 * 1024);

        // Insert 3 entries with bodies
        for i in 0..3 {
            let entry = make_entry(
                &session_id,
                "example.com",
                &format!("/s{i}"),
                Some(format!("body{i}").into_bytes()),
            );
            store.store_request(&entry).expect("store request");
        }

        let stats = store.get_capture_stats().expect("capture stats");
        assert_eq!(stats.entry_count, 3);
        assert_eq!(stats.max_entries, 100);
        assert!(stats.total_size_bytes > 0);
        assert_eq!(stats.max_total_size_bytes, 1024 * 1024);
        assert!(stats.capture_enabled);
        assert!(stats.capture_request_bodies);
        assert!(stats.capture_response_bodies);
        assert!(stats.ignored_domains.is_empty());
    }

    #[test]
    fn test_total_size_limit_pruning() {
        let store = make_store();
        let session_id = store.current_session_id();
        // Set a very small total size limit (enough for ~2 entries with bodies)
        store.set_max_total_size_bytes(30);
        // Disable entry-count limit so only size pruning applies
        store.set_max_entries(0);

        // Insert entries with bodies larger than the limit can hold
        for i in 0..10 {
            let entry = make_entry(
                &session_id,
                "example.com",
                &format!("/sz{i}"),
                Some(vec![b'x'; 20]),
            );
            store.store_request(&entry).expect("store request");
            // Force size check on every insert by resetting the counter
            store.insert_counter.store(0, Ordering::Relaxed);
        }

        let total = store.get_total_size().expect("total size");
        assert!(
            total <= 30,
            "total size {total} should be under the 30-byte limit"
        );
    }

    #[test]
    fn test_max_entries_zero_means_unlimited() {
        let store = make_store();
        let session_id = store.current_session_id();
        store.set_max_entries(0);

        for i in 0..20 {
            let entry = make_entry(&session_id, "example.com", &format!("/u{i}"), None);
            store.store_request(&entry).expect("store request");
        }

        assert_eq!(
            store.get_entry_count().unwrap(),
            20,
            "no pruning when max_entries is 0"
        );
    }

    // -----------------------------------------------------------------------
    // Focus host tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_add_and_list_focus_host() {
        let store = make_store();
        let host = store
            .add_focus_host("api.example.com")
            .expect("add focus host");
        assert_eq!(host.pattern, "api.example.com");
        assert!(!host.id.is_empty());

        let hosts = store.list_focus_hosts().expect("list focus hosts");
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].pattern, "api.example.com");
    }

    #[test]
    fn test_add_focus_host_dedup() {
        let store = make_store();
        store.add_focus_host("API.Example.com").expect("add 1");
        let second = store.add_focus_host("api.example.com").expect("add 2");
        let hosts = store.list_focus_hosts().expect("list");
        assert_eq!(hosts.len(), 1, "duplicate pattern should be deduped");
        assert_eq!(second.pattern, hosts[0].pattern);
    }

    #[test]
    fn test_remove_focus_host() {
        let store = make_store();
        let host = store.add_focus_host("example.com").expect("add");
        let removed = store.remove_focus_host(&host.id).expect("remove");
        assert!(removed);
        let hosts = store.list_focus_hosts().expect("list");
        assert!(hosts.is_empty());

        let removed_again = store.remove_focus_host(&host.id).expect("remove again");
        assert!(!removed_again, "removing non-existent id returns false");
    }

    #[test]
    fn test_clear_focus_hosts() {
        let store = make_store();
        store.add_focus_host("a.com").expect("add");
        store.add_focus_host("b.com").expect("add");
        store.add_focus_host("c.com").expect("add");
        assert_eq!(store.list_focus_hosts().expect("list").len(), 3);
        store.clear_focus_hosts().expect("clear");
        assert!(store.list_focus_hosts().expect("list").is_empty());
    }

    #[test]
    fn test_focus_host_persistence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("focus_test.db");

        let store = TrafficStore::new(&db_path).expect("create store");
        store.add_focus_host("persist.example.com").expect("add");
        store.add_focus_host("*.wildcard.com").expect("add");
        assert_eq!(store.list_focus_hosts().expect("list").len(), 2);

        drop(store);

        let store2 = TrafficStore::new(&db_path).expect("reopen store");
        let hosts = store2.list_focus_hosts().expect("list");
        assert_eq!(hosts.len(), 2, "focus hosts should persist across restarts");
        let patterns: Vec<String> = hosts.iter().map(|h| h.pattern.clone()).collect();
        assert!(patterns.contains(&"persist.example.com".to_string()));
        assert!(patterns.contains(&"*.wildcard.com".to_string()));
    }

    #[test]
    fn test_host_matches_pattern_exact() {
        assert!(host_matches_pattern("api.example.com", "api.example.com"));
        assert!(!host_matches_pattern("other.com", "api.example.com"));
    }

    #[test]
    fn test_host_matches_pattern_suffix() {
        assert!(host_matches_pattern("api.example.com", "example.com"));
        assert!(host_matches_pattern("sub.api.example.com", "example.com"));
        assert!(!host_matches_pattern("notexample.com", "example.com"));
    }

    #[test]
    fn test_host_matches_pattern_wildcard_subdomain() {
        assert!(host_matches_pattern("api.example.com", "*.example.com"));
        assert!(host_matches_pattern("sub.api.example.com", "*.example.com"));
        assert!(!host_matches_pattern("example.com", "*.example.com"));
    }

    #[test]
    fn test_host_matches_pattern_glob() {
        assert!(host_matches_pattern("api.example.com", "*api*"));
        assert!(host_matches_pattern("api.example.com", "api.*"));
        assert!(!host_matches_pattern("example.com", "*api*"));
    }

    #[test]
    fn test_host_matches_pattern_case_insensitive() {
        assert!(host_matches_pattern("API.Example.COM", "api.example.com"));
        assert!(host_matches_pattern("api.example.com", "API.EXAMPLE.COM"));
    }
}
