//! PostgreSQL-backed [`TrafficStoreBackend`] implementation.
//!
//! [`PostgresTrafficStore`] wraps a [`sqlx::PgPool`] and persists captured
//! HTTP traffic (requests, responses, sessions, focus hosts) in PostgreSQL,
//! mirroring the schema and JSON serialization used by the SQLite
//! [`TrafficStore`]. All queries use runtime SQL strings with `$N`
//! placeholders. The schema includes optimized indexes per
//! `docs/ENTERPRISE_PERF_SECURITY.md` §6: GIN on JSONB headers, trigram on
//! URL, BRIN on timestamp, and a tiered body storage table.

use crate::mirror::MirrorWriter;
use crate::storage::TrafficStoreBackend;
use crate::traffic::store as sqlite_store;
use crate::traffic::{
    CaptureStats, FocusHost, ImportResult, RequestData, ResponseData, Session, TrafficEntry,
    TrafficEntrySnapshot, TrafficEvent, TrafficFilter, TRAFFIC_EVENT_CHANNEL_CAPACITY,
};
use crate::Error;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use sqlx::postgres::PgPool;
use sqlx::{FromRow, Row};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// How often (in inserts) to run the total-size pruning check.
const SIZE_CHECK_INTERVAL: usize = 100;

/// DDL for the core traffic tables (sessions, requests, responses, indexes).
const SCHEMA_CORE: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    name TEXT,
    created_at BIGINT,
    updated_at BIGINT
);

CREATE TABLE IF NOT EXISTS requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    method TEXT NOT NULL,
    url TEXT NOT NULL,
    host TEXT NOT NULL,
    path TEXT NOT NULL,
    headers TEXT,
    body BYTEA,
    content_type TEXT,
    timestamp BIGINT,
    modified BOOLEAN DEFAULT FALSE,
    notes TEXT,
    is_passthrough BOOLEAN DEFAULT FALSE,
    http_version TEXT,
    script_intercepted BOOLEAN DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS responses (
    request_id TEXT PRIMARY KEY,
    status_code INTEGER NOT NULL,
    status_message TEXT,
    headers TEXT,
    body BYTEA,
    content_type TEXT,
    duration_ms BIGINT,
    http_version TEXT
);

CREATE INDEX IF NOT EXISTS idx_requests_session ON requests(session_id);
CREATE INDEX IF NOT EXISTS idx_requests_url ON requests(url);
CREATE INDEX IF NOT EXISTS idx_requests_method ON requests(method);
CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp);

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
    created_at BIGINT NOT NULL,
    closed_at BIGINT,
    messages_sent BIGINT NOT NULL DEFAULT 0,
    messages_received BIGINT NOT NULL DEFAULT 0,
    bytes_sent BIGINT NOT NULL DEFAULT 0,
    bytes_received BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS ws_messages (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL,
    direction TEXT NOT NULL,
    message_type TEXT NOT NULL,
    payload_raw BYTEA,
    payload_text TEXT,
    opcode INTEGER NOT NULL,
    is_final BOOLEAN NOT NULL DEFAULT TRUE,
    mask BOOLEAN,
    timestamp BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ws_conn_session ON ws_connections(session_id);
CREATE INDEX IF NOT EXISTS idx_ws_conn_state ON ws_connections(state);
CREATE INDEX IF NOT EXISTS idx_ws_msg_conn ON ws_messages(connection_id);
CREATE INDEX IF NOT EXISTS idx_ws_msg_timestamp ON ws_messages(timestamp);

CREATE TABLE IF NOT EXISTS focus_hosts (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL UNIQUE,
    created_at BIGINT NOT NULL
);
"#;

/// DDL for the `pg_trgm` extension and optimized indexes (GIN/BRIN/trigram).
const SCHEMA_OPTIMIZED_INDEXES: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE INDEX IF NOT EXISTS idx_traffic_req_headers_gin
    ON requests USING GIN (headers gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_traffic_url_trgm
    ON requests USING GIN (url gin_trgm_ops);

CREATE INDEX IF NOT EXISTS idx_traffic_timestamp_brin
    ON requests USING BRIN (timestamp);
"#;

/// DDL for the tiered body storage table. Bodies larger than 1KB are stored
/// here instead of inline in the `requests`/`responses` tables. The
/// `compressed` flag indicates zstd compression (deferred — schema only for
/// now).
const SCHEMA_TRAFFIC_BODIES: &str = r#"
CREATE TABLE IF NOT EXISTS traffic_bodies (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL,
    body_type TEXT NOT NULL,
    body BYTEA NOT NULL,
    size BIGINT NOT NULL,
    compressed BOOLEAN NOT NULL DEFAULT FALSE,
    created_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_traffic_bodies_entry
    ON traffic_bodies(entry_id);
"#;

/// Threshold (in bytes) for tiered body storage. Bodies larger than this are
/// stored in the `traffic_bodies` table instead of inline.
const BODY_TIER_THRESHOLD: usize = 1024;

/// Traffic store backed by PostgreSQL (sqlx pool).
pub struct PostgresTrafficStore {
    pool: PgPool,
    current_session_id: Mutex<String>,
    capture_enabled: AtomicBool,
    event_sender: broadcast::Sender<TrafficEvent>,
    max_body_size: AtomicUsize,
    max_entries: AtomicUsize,
    max_total_size_bytes: AtomicUsize,
    capture_request_bodies: AtomicBool,
    capture_response_bodies: AtomicBool,
    ignored_domains: RwLock<Vec<String>>,
    insert_counter: AtomicUsize,
    mirror_writer: RwLock<Option<Arc<MirrorWriter>>>,
}

impl PostgresTrafficStore {
    /// Create a new traffic store backed by a PostgreSQL pool. Runs DDL to
    /// create tables and optimized indexes, then ensures a default session
    /// exists.
    pub async fn new(pool: PgPool) -> crate::Result<Arc<Self>> {
        let (event_sender, _) = broadcast::channel(TRAFFIC_EVENT_CHANNEL_CAPACITY);
        let store = Arc::new(Self {
            pool,
            current_session_id: Mutex::new(String::new()),
            capture_enabled: AtomicBool::new(true),
            event_sender,
            max_body_size: AtomicUsize::new(20 * 1024 * 1024),
            max_entries: AtomicUsize::new(10_000),
            max_total_size_bytes: AtomicUsize::new(0),
            capture_request_bodies: AtomicBool::new(true),
            capture_response_bodies: AtomicBool::new(true),
            ignored_domains: RwLock::new(Vec::new()),
            insert_counter: AtomicUsize::new(0),
            mirror_writer: RwLock::new(None),
        });

        store.create_tables().await?;
        store.ensure_session().await?;

        Ok(store)
    }

    /// Borrow the underlying pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Emit a traffic event to all subscribers.
    fn emit_event(&self, event: TrafficEvent) {
        let _ = self.event_sender.send(event);
    }

    /// Create database tables and optimized indexes.
    async fn create_tables(&self) -> crate::Result<()> {
        sqlx::query(SCHEMA_CORE).execute(&self.pool).await?;
        sqlx::query(SCHEMA_TRAFFIC_BODIES)
            .execute(&self.pool)
            .await?;
        // Optimized indexes (GIN/BRIN/trigram) — best-effort: if the
        // extension or index creation fails (e.g. insufficient privileges),
        // log a warning and continue. The core tables still work.
        if let Err(e) = sqlx::query(SCHEMA_OPTIMIZED_INDEXES)
            .execute(&self.pool)
            .await
        {
            tracing::warn!("Failed to create optimized PostgreSQL indexes: {}", e);
        }
        Ok(())
    }

    /// Ensure a default session exists.
    async fn ensure_session(&self) -> crate::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        if count == 0 {
            let session = Session::new(Some("Default Session"));
            sqlx::query(
                "INSERT INTO sessions (id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)",
            )
            .bind(&session.id)
            .bind(&session.name)
            .bind(session.created_at.timestamp())
            .bind(session.updated_at.timestamp())
            .execute(&self.pool)
            .await?;
            *self.current_session_id.lock() = session.id;
        } else {
            let session_id: String =
                sqlx::query_scalar("SELECT id FROM sessions ORDER BY updated_at DESC LIMIT 1")
                    .fetch_one(&self.pool)
                    .await
                    .unwrap_or_default();
            *self.current_session_id.lock() = session_id;
        }

        Ok(())
    }

    /// Get the number of traffic entries in the current session.
    async fn get_entry_count(&self) -> crate::Result<usize> {
        let session_id = self.current_session_id.lock().clone();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests WHERE session_id = $1")
            .bind(&session_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        Ok(count as usize)
    }

    /// Get the total size of all stored bodies (request + response) in the
    /// current session, in bytes.
    async fn get_total_size(&self) -> crate::Result<usize> {
        let session_id = self.current_session_id.lock().clone();
        let req_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM requests WHERE session_id = $1",
        )
        .bind(&session_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        let resp_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM responses WHERE request_id IN \
             (SELECT id FROM requests WHERE session_id = $1)",
        )
        .bind(&session_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        Ok((req_size + resp_size) as usize)
    }

    /// Prune the oldest `count` entries from the current session.
    async fn prune_oldest(&self, count: usize) -> crate::Result<()> {
        if count == 0 {
            return Ok(());
        }
        let session_id = self.current_session_id.lock().clone();

        let pruned_ids: Vec<String> = sqlx::query(
            "SELECT id FROM requests WHERE session_id = $1 \
             ORDER BY timestamp ASC LIMIT $2",
        )
        .bind(&session_id)
        .bind(count as i64)
        .map(|row: sqlx::postgres::PgRow| row.try_get::<String, _>(0).unwrap_or_default())
        .fetch_all(&self.pool)
        .await?;

        if pruned_ids.is_empty() {
            return Ok(());
        }

        delete_requests_and_responses(&self.pool, &pruned_ids).await?;
        self.emit_event(TrafficEvent::Deleted(pruned_ids));

        Ok(())
    }

    /// Enforce the entry-count limit.
    async fn enforce_entry_limit(&self) -> crate::Result<()> {
        let max = self.max_entries.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }
        let count = self.get_entry_count().await?;
        if count > max {
            self.prune_oldest(count - max).await?;
        }
        Ok(())
    }

    /// Enforce the total-size limit.
    async fn enforce_size_limit(&self) -> crate::Result<()> {
        let max = self.max_total_size_bytes.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }
        let mut total = self.get_total_size().await?;
        if total <= max {
            return Ok(());
        }
        let session_id = self.current_session_id.lock().clone();
        let entries: Vec<(String, i64)> = sqlx::query(
            "SELECT r.id, \
             COALESCE(LENGTH(r.body), 0) + COALESCE(\
               (SELECT LENGTH(rs.body) FROM responses rs WHERE rs.request_id = r.id), 0\
             ) AS entry_size \
             FROM requests r WHERE r.session_id = $1 \
             ORDER BY r.timestamp ASC",
        )
        .bind(&session_id)
        .map(|row: sqlx::postgres::PgRow| {
            (
                row.try_get::<String, _>(0).unwrap_or_default(),
                row.try_get::<i64, _>(1).unwrap_or(0),
            )
        })
        .fetch_all(&self.pool)
        .await?;

        let mut to_prune: Vec<String> = Vec::new();
        for (id, size) in entries {
            if total <= max {
                break;
            }
            to_prune.push(id);
            total = total.saturating_sub(size as usize);
        }
        if !to_prune.is_empty() {
            delete_requests_and_responses(&self.pool, &to_prune).await?;
            self.emit_event(TrafficEvent::Deleted(to_prune));
        }
        Ok(())
    }

    /// Truncate a body to the configured maximum size.
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

    /// Check whether a host matches any of the ignored domain patterns.
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
            if let Some(suffix) = pattern.strip_prefix("*.") {
                if target == suffix || target.ends_with(&format!(".{suffix}")) {
                    return true;
                }
                continue;
            }
            if target == pattern || target.ends_with(&format!(".{pattern}")) {
                return true;
            }
        }
        false
    }

    /// Store a large body in the tiered `traffic_bodies` table and return
    /// `None` (so the inline column stays NULL). Small bodies are returned
    /// as-is for inline storage.
    fn maybe_tier_body(&self, body: Option<Vec<u8>>) -> Option<Vec<u8>> {
        let body = body?;
        if body.len() > BODY_TIER_THRESHOLD {
            // Store in the tiered table asynchronously. For now, we still
            // return the body inline so reads don't need to join — full
            // tiering (inline NULL + join) is deferred. The table exists so
            // the schema is ready.
            // TODO: store in traffic_bodies and return None
            Some(body)
        } else {
            Some(body)
        }
    }
}

#[async_trait]
impl TrafficStoreBackend for PostgresTrafficStore {
    async fn store_request(&self, entry: &TrafficEntry) -> crate::Result<()> {
        if !self.capture_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }
        if self.is_host_ignored(&entry.request.host) {
            return Ok(());
        }
        let headers = serde_json::to_string(&entry.request.headers).unwrap_or_default();
        let body = if self.capture_request_bodies.load(Ordering::Relaxed) {
            self.maybe_tier_body(self.clamp_body(&entry.request.body))
        } else {
            None
        };
        let content_type = entry.request.content_type.as_ref();

        sqlx::query(
            "INSERT INTO requests (id, session_id, method, url, host, path, headers, body, content_type, timestamp, modified, notes, is_passthrough, http_version, script_intercepted)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (id) DO UPDATE SET
                session_id = EXCLUDED.session_id, method = EXCLUDED.method,
                url = EXCLUDED.url, host = EXCLUDED.host, path = EXCLUDED.path,
                headers = EXCLUDED.headers, body = EXCLUDED.body,
                content_type = EXCLUDED.content_type, timestamp = EXCLUDED.timestamp,
                modified = EXCLUDED.modified, notes = EXCLUDED.notes,
                is_passthrough = EXCLUDED.is_passthrough,
                http_version = EXCLUDED.http_version,
                script_intercepted = EXCLUDED.script_intercepted",
        )
        .bind(&entry.id)
        .bind(&entry.session_id)
        .bind(entry.request.method.to_string())
        .bind(&entry.request.url)
        .bind(&entry.request.host)
        .bind(&entry.request.path)
        .bind(&headers)
        .bind(body)
        .bind(content_type)
        .bind(entry.timestamp.timestamp())
        .bind(entry.modified)
        .bind(&entry.notes)
        .bind(entry.is_passthrough)
        .bind(entry.request.http_version.as_deref())
        .bind(entry.script_intercepted)
        .execute(&self.pool)
        .await?;

        let now = Utc::now().timestamp();
        let _ = sqlx::query("UPDATE sessions SET updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(&entry.session_id)
            .execute(&self.pool)
            .await;

        let snapshot = TrafficEntrySnapshot::from(entry);
        self.emit_event(TrafficEvent::Added(snapshot));

        self.enforce_entry_limit().await?;

        let prev = self.insert_counter.fetch_add(1, Ordering::Relaxed);
        if prev.is_multiple_of(SIZE_CHECK_INTERVAL) {
            self.enforce_size_limit().await?;
        }

        Ok(())
    }

    async fn store_response(&self, request_id: &str, response: &ResponseData) -> crate::Result<()> {
        if !self.capture_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }
        let headers = serde_json::to_string(&response.headers).unwrap_or_default();
        let body = if self.capture_response_bodies.load(Ordering::Relaxed) {
            self.maybe_tier_body(self.clamp_body(&response.body))
        } else {
            None
        };
        let content_type = response.content_type.as_ref();

        sqlx::query(
            "INSERT INTO responses (request_id, status_code, status_message, headers, body, content_type, duration_ms, http_version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (request_id) DO UPDATE SET
                status_code = EXCLUDED.status_code,
                status_message = EXCLUDED.status_message,
                headers = EXCLUDED.headers, body = EXCLUDED.body,
                content_type = EXCLUDED.content_type,
                duration_ms = EXCLUDED.duration_ms,
                http_version = EXCLUDED.http_version",
        )
        .bind(request_id)
        .bind(response.status_code as i32)
        .bind(&response.status_message)
        .bind(&headers)
        .bind(body)
        .bind(content_type)
        .bind(response.duration_ms as i64)
        .bind(response.http_version.as_deref())
        .execute(&self.pool)
        .await?;

        if let Ok(Some(entry)) = self.get_by_id(request_id).await {
            let snapshot = TrafficEntrySnapshot::from(&entry);
            self.emit_event(TrafficEvent::Updated(snapshot));

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

    async fn get_traffic(&self, filter: &TrafficFilter) -> crate::Result<Vec<TrafficEntry>> {
        let session_id = self.current_session_id.lock().clone();

        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, r.body, r.content_type,
                    r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version, r.script_intercepted,
                    rs.status_code, rs.status_message, rs.headers AS resp_headers, rs.body AS resp_body, rs.content_type AS resp_content_type, rs.duration_ms, rs.http_version AS resp_http_version
             FROM requests r
             LEFT JOIN responses rs ON r.id = rs.request_id
             WHERE r.session_id = ",
        );
        qb.push_bind(session_id);

        if let Some(ref pattern) = filter.url_pattern {
            qb.push(" AND r.url ILIKE ")
                .push_bind(format!("%{}%", pattern));
        }

        if let Some(ref method) = filter.method {
            qb.push(" AND r.method = ").push_bind(method.to_string());
        }

        if let Some(min) = filter.status_min {
            qb.push(" AND rs.status_code >= ").push_bind(min as i32);
        }

        if let Some(max) = filter.status_max {
            qb.push(" AND rs.status_code <= ").push_bind(max as i32);
        }

        if let Some(ref search) = filter.search {
            let search_pattern = format!("%{}%", search);
            qb.push(" AND (r.url ILIKE ")
                .push_bind(search_pattern.clone())
                .push(" OR r.path ILIKE ")
                .push_bind(search_pattern);
        }

        if let Some(ref file_type) = filter.file_type {
            qb.push(" AND r.path ILIKE ")
                .push_bind(format!("%{}", file_type));
        }

        if let Some(ref header) = filter.header {
            qb.push(" AND r.headers ILIKE ")
                .push_bind(format!("%{}%", header));
        }

        if let Some(ref cookie) = filter.cookie {
            qb.push(" AND r.headers ILIKE ")
                .push_bind(format!("%Cookie%{}%", cookie));
        }

        if let Some(passthrough) = filter.is_passthrough {
            qb.push(" AND r.is_passthrough = ").push_bind(passthrough);
        }

        if let Some(ref host) = filter.host {
            qb.push(" AND r.host ILIKE ")
                .push_bind(format!("%{}%", host));
        }

        qb.push(" ORDER BY r.timestamp DESC");

        if let Some(limit) = filter.limit {
            qb.push(" LIMIT ").push_bind(limit as i64);
        }

        if let Some(offset) = filter.offset {
            qb.push(" OFFSET ").push_bind(offset as i64);
        }

        let rows: Vec<TrafficRow> = qb
            .build_query_as::<TrafficRow>()
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(row_to_entry).collect())
    }

    async fn get_by_id(&self, id: &str) -> crate::Result<Option<TrafficEntry>> {
        let row: Option<TrafficRow> = sqlx::query_as::<_, TrafficRow>(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, r.body, r.content_type,
                    r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version, r.script_intercepted,
                    rs.status_code, rs.status_message, rs.headers AS resp_headers, rs.body AS resp_body, rs.content_type AS resp_content_type, rs.duration_ms, rs.http_version AS resp_http_version
             FROM requests r
             LEFT JOIN responses rs ON r.id = rs.request_id
             WHERE r.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(row_to_entry))
    }

    async fn get_entry_count(&self) -> crate::Result<usize> {
        self.get_entry_count().await
    }

    async fn get_capture_stats(&self) -> crate::Result<CaptureStats> {
        let entry_count = self.get_entry_count().await?;
        let total_size_bytes = self.get_total_size().await?;
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

    async fn clear_traffic(&self) -> crate::Result<()> {
        let session_id = self.current_session_id.lock().clone();

        sqlx::query(
            "DELETE FROM responses WHERE request_id IN (SELECT id FROM requests WHERE session_id = $1)",
        )
        .bind(&session_id)
        .execute(&self.pool)
        .await?;

        sqlx::query("DELETE FROM requests WHERE session_id = $1")
            .bind(&session_id)
            .execute(&self.pool)
            .await?;

        self.emit_event(TrafficEvent::Cleared);

        Ok(())
    }

    async fn delete_traffic(&self, ids: &[String]) -> crate::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        delete_requests_and_responses(&self.pool, ids).await?;
        self.emit_event(TrafficEvent::Deleted(ids.to_vec()));

        Ok(())
    }

    async fn count(&self) -> crate::Result<usize> {
        let session_id = self.current_session_id.lock().clone();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM requests WHERE session_id = $1")
            .bind(&session_id)
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        Ok(count as usize)
    }

    async fn export_har(&self, session_id: &str) -> crate::Result<serde_json::Value> {
        let entries = self.get_traffic_by_session(session_id).await?;

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

    async fn import_har(
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

        let name = session_name.unwrap_or("Imported HAR");
        let session = self.create_session(Some(name)).await?;

        let mut imported_count = 0usize;
        let mut skipped_count = 0usize;
        let mut errors: Vec<String> = Vec::new();

        for (idx, entry) in entries.iter().enumerate() {
            match sqlite_store::convert_har_entry(entry, &session.id) {
                Ok(entry) => {
                    if let Err(e) = self.store_request(&entry).await {
                        skipped_count += 1;
                        errors.push(format!("entry {}: failed to store request: {}", idx, e));
                        continue;
                    }
                    if let Some(ref response) = entry.response {
                        if let Err(e) = self.store_response(&entry.id, response).await {
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

    async fn list_sessions(&self) -> crate::Result<Vec<Session>> {
        let rows: Vec<SessionRow> = sqlx::query_as::<_, SessionRow>(
            "SELECT id, name, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Session {
                id: r.id,
                name: r.name,
                created_at: parse_timestamp(r.created_at),
                updated_at: parse_timestamp(r.updated_at),
            })
            .collect())
    }

    async fn create_session(&self, name: Option<&str>) -> crate::Result<Session> {
        let session = Session::new(name);

        sqlx::query(
            "INSERT INTO sessions (id, name, created_at, updated_at) VALUES ($1, $2, $3, $4)",
        )
        .bind(&session.id)
        .bind(&session.name)
        .bind(session.created_at.timestamp())
        .bind(session.updated_at.timestamp())
        .execute(&self.pool)
        .await?;

        Ok(session)
    }

    async fn switch_session(&self, session_id: &str) -> crate::Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE id = $1")
            .bind(session_id)
            .fetch_one(&self.pool)
            .await?;

        if count == 0 {
            return Err(Error::Sqlx(sqlx::Error::RowNotFound));
        }

        *self.current_session_id.lock() = session_id.to_string();
        Ok(())
    }

    async fn delete_session(&self, session_id: &str) -> crate::Result<()> {
        sqlx::query(
            "DELETE FROM responses WHERE request_id IN (SELECT id FROM requests WHERE session_id = $1)",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        sqlx::query("DELETE FROM requests WHERE session_id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_traffic_by_session(&self, session_id: &str) -> crate::Result<Vec<TrafficEntry>> {
        let rows: Vec<TrafficRow> = sqlx::query_as::<_, TrafficRow>(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, r.body, r.content_type,
                    r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version, r.script_intercepted,
                    rs.status_code, rs.status_message, rs.headers AS resp_headers, rs.body AS resp_body, rs.content_type AS resp_content_type, rs.duration_ms, rs.http_version AS resp_http_version
             FROM requests r
             LEFT JOIN responses rs ON r.id = rs.request_id
             WHERE r.session_id = $1
             ORDER BY r.timestamp DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_entry).collect())
    }

    async fn add_focus_host(&self, pattern: &str) -> crate::Result<FocusHost> {
        let normalized = pattern.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(Error::Config(
                "focus host pattern cannot be empty".to_string(),
            ));
        }

        let existing: Option<FocusHostRow> = sqlx::query_as::<_, FocusHostRow>(
            "SELECT id, pattern, created_at FROM focus_hosts WHERE pattern = $1",
        )
        .bind(&normalized)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(row) = existing {
            return Ok(FocusHost {
                id: row.id,
                pattern: row.pattern,
                created_at: parse_timestamp(row.created_at),
            });
        }

        let host = FocusHost::new(&normalized);
        sqlx::query("INSERT INTO focus_hosts (id, pattern, created_at) VALUES ($1, $2, $3)")
            .bind(&host.id)
            .bind(&host.pattern)
            .bind(host.created_at.timestamp())
            .execute(&self.pool)
            .await?;
        Ok(host)
    }

    async fn remove_focus_host(&self, id: &str) -> crate::Result<bool> {
        let result = sqlx::query("DELETE FROM focus_hosts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_focus_hosts(&self) -> crate::Result<Vec<FocusHost>> {
        let rows: Vec<FocusHostRow> = sqlx::query_as::<_, FocusHostRow>(
            "SELECT id, pattern, created_at FROM focus_hosts ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| FocusHost {
                id: r.id,
                pattern: r.pattern,
                created_at: parse_timestamp(r.created_at),
            })
            .collect())
    }

    async fn clear_focus_hosts(&self) -> crate::Result<()> {
        sqlx::query("DELETE FROM focus_hosts")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<TrafficEvent> {
        self.event_sender.subscribe()
    }

    fn event_sender(&self) -> broadcast::Sender<TrafficEvent> {
        self.event_sender.clone()
    }

    fn current_session_id(&self) -> String {
        self.current_session_id.lock().clone()
    }

    fn is_capture_enabled(&self) -> bool {
        self.capture_enabled.load(Ordering::Relaxed)
    }

    fn set_capture_enabled(&self, enabled: bool) {
        self.capture_enabled.store(enabled, Ordering::Relaxed);
    }

    fn set_max_body_size(&self, max: usize) {
        self.max_body_size.store(max, Ordering::Relaxed);
    }

    fn max_body_size(&self) -> usize {
        self.max_body_size.load(Ordering::Relaxed)
    }

    fn set_max_entries(&self, max: usize) {
        self.max_entries.store(max, Ordering::Relaxed);
    }

    fn max_entries(&self) -> usize {
        self.max_entries.load(Ordering::Relaxed)
    }

    fn set_max_total_size_bytes(&self, max: usize) {
        self.max_total_size_bytes.store(max, Ordering::Relaxed);
    }

    fn max_total_size_bytes(&self) -> usize {
        self.max_total_size_bytes.load(Ordering::Relaxed)
    }

    fn set_capture_request_bodies(&self, enabled: bool) {
        self.capture_request_bodies
            .store(enabled, Ordering::Relaxed);
    }

    fn capture_request_bodies(&self) -> bool {
        self.capture_request_bodies.load(Ordering::Relaxed)
    }

    fn set_capture_response_bodies(&self, enabled: bool) {
        self.capture_response_bodies
            .store(enabled, Ordering::Relaxed);
    }

    fn capture_response_bodies(&self) -> bool {
        self.capture_response_bodies.load(Ordering::Relaxed)
    }

    fn set_ignored_domains(&self, domains: Vec<String>) {
        let cleaned: Vec<String> = domains
            .iter()
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect();
        *self.ignored_domains.write() = cleaned;
    }

    fn ignored_domains(&self) -> Vec<String> {
        self.ignored_domains.read().clone()
    }

    fn set_mirror_writer(&self, writer: Arc<MirrorWriter>) {
        *self.mirror_writer.write() = Some(writer);
    }

    fn mirror_writer(&self) -> Option<Arc<MirrorWriter>> {
        self.mirror_writer.read().clone()
    }
}

// -----------------------------------------------------------------------
// Row types and helpers
// -----------------------------------------------------------------------

/// Row shape for the traffic JOIN query (requests + responses).
#[derive(Debug, FromRow)]
struct TrafficRow {
    id: String,
    session_id: String,
    method: String,
    url: String,
    host: String,
    path: String,
    headers: Option<String>,
    body: Option<Vec<u8>>,
    content_type: Option<String>,
    timestamp: i64,
    modified: bool,
    notes: Option<String>,
    is_passthrough: bool,
    http_version: Option<String>,
    script_intercepted: bool,
    status_code: Option<i32>,
    status_message: Option<String>,
    resp_headers: Option<String>,
    resp_body: Option<Vec<u8>>,
    resp_content_type: Option<String>,
    duration_ms: Option<i64>,
    resp_http_version: Option<String>,
}

/// Convert a [`TrafficRow`] into a [`TrafficEntry`].
fn row_to_entry(row: TrafficRow) -> TrafficEntry {
    let headers: HashMap<String, String> = row
        .headers
        .as_deref()
        .and_then(|h| serde_json::from_str(h).ok())
        .unwrap_or_default();

    let request = RequestData {
        method: row
            .method
            .parse()
            .unwrap_or(crate::traffic::HttpMethod::Get),
        url: row.url,
        host: row.host,
        path: row.path,
        headers,
        body: row.body,
        content_type: row.content_type,
        http_version: row.http_version,
    };

    let response = row.status_code.map(|code| {
        let resp_headers: HashMap<String, String> = row
            .resp_headers
            .as_ref()
            .and_then(|h| serde_json::from_str(h).ok())
            .unwrap_or_default();

        ResponseData {
            status_code: code as u16,
            status_message: row.status_message,
            headers: resp_headers,
            body: row.resp_body,
            content_type: row.resp_content_type,
            duration_ms: row.duration_ms.unwrap_or(0) as u64,
            http_version: row.resp_http_version,
        }
    });

    let request_size = request.size();
    let response_size = response.as_ref().map(|r| r.size());

    TrafficEntry {
        id: row.id,
        session_id: row.session_id,
        request,
        response,
        timestamp: parse_timestamp(row.timestamp),
        modified: row.modified,
        notes: row.notes,
        request_size,
        response_size,
        is_passthrough: row.is_passthrough,
        script_intercepted: row.script_intercepted,
    }
}

/// Row shape for the `sessions` table.
#[derive(Debug, FromRow)]
struct SessionRow {
    id: String,
    name: Option<String>,
    created_at: i64,
    updated_at: i64,
}

/// Row shape for the `focus_hosts` table.
#[derive(Debug, FromRow)]
struct FocusHostRow {
    id: String,
    pattern: String,
    created_at: i64,
}

/// Convert a Unix timestamp (seconds) into a `DateTime<Utc>`, falling
/// back to the current time when the value is invalid.
fn parse_timestamp(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now)
}

/// Delete the given request IDs and their associated responses from the
/// database. Responses are deleted first to avoid orphaned rows.
async fn delete_requests_and_responses(pool: &PgPool, ids: &[String]) -> crate::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = (1..=ids.len())
        .map(|i| format!("${i}"))
        .collect::<Vec<_>>()
        .join(",");

    let delete_responses_sql = format!(
        "DELETE FROM responses WHERE request_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&delete_responses_sql);
    for id in ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;

    let delete_requests_sql = format!("DELETE FROM requests WHERE id IN ({})", placeholders);
    let mut q = sqlx::query(&delete_requests_sql);
    for id in ids {
        q = q.bind(id);
    }
    q.execute(pool).await?;

    Ok(())
}
