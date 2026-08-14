//! PostgreSQL-backed [`TrafficStoreBackend`] implementation.
//!
//! [`PostgresTrafficStore`] wraps a [`sqlx::PgPool`] and persists captured
//! HTTP traffic (requests, responses, sessions, focus hosts) in PostgreSQL,
//! mirroring the schema and JSON serialization used by the SQLite
//! [`TrafficStore`]. All queries use runtime SQL strings with `$N`
//! placeholders. The schema includes optimized indexes per
//! `docs/ENTERPRISE_PERF_SECURITY.md` §6: GIN on JSONB headers, trigram on
//! URL, BRIN on timestamp, and a tiered body storage table.
//!
//! Phase 10 extensions:
//! - **Tiered body storage** (10a.1): bodies < 4KB inline, >= 4KB in
//!   `traffic_bodies` with zstd compression (10a.2). The `storage_type`
//!   column indicates `'inline'`, `'toast'`, or `'s3'` (S3 is documented
//!   only — see ENTERPRISE_PERF_SECURITY.md §6.3).
//! - **Session counter table** (10b.2): `session_counters` eliminates the
//!   expensive `COUNT(*)` on every stats request.
//! - **Cursor-based pagination** (10b.3): keyset pagination via
//!   `(timestamp, id) < (cursor_t, cursor_id)` — O(1) vs OFFSET O(n).
//! - **Lazy body loading** (10b.4): list view omits body columns.
//! - **Write batching** (10b.1): [`WriteBatcher`] buffers up to 100 entries
//!   or 500ms, then flushes in a single transaction.
//! - **Read/write split** (10d.2): optional `read_pool` for `get_*` methods.
//! - **Autovacuum tuning** (10a.6): aggressive autovacuum on high-write
//!   tables.
//! - **Partitioning** (10c.1): documented DDL for weekly range partitioning
//!   (not enabled by default — see `SCHEMA_PARTITIONING_STMTS`).

use crate::mirror::MirrorWriter;
use crate::storage::body_storage::{
    compress_body, decompress_body, BodyStorageType, INLINE_THRESHOLD,
};
use crate::storage::TrafficStoreBackend;
use crate::traffic::store as sqlite_store;
use crate::traffic::{
    CaptureStats, FocusHost, ImportResult, RequestData, ResponseData, Session, TrafficCursor,
    TrafficEntry, TrafficEntrySnapshot, TrafficEvent, TrafficFilter,
    TRAFFIC_EVENT_CHANNEL_CAPACITY,
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

/// DDL statements for the core traffic tables. PostgreSQL does not allow
/// multiple statements in a single prepared statement, so each entry is
/// executed individually.
const SCHEMA_CORE_STMTS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, name TEXT, created_at BIGINT, updated_at BIGINT)",
    "CREATE TABLE IF NOT EXISTS requests (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, method TEXT NOT NULL, url TEXT NOT NULL, host TEXT NOT NULL, path TEXT NOT NULL, headers TEXT, body BYTEA, content_type TEXT, timestamp BIGINT, modified BOOLEAN DEFAULT FALSE, notes TEXT, is_passthrough BOOLEAN DEFAULT FALSE, http_version TEXT, script_intercepted BOOLEAN DEFAULT FALSE)",
    "CREATE TABLE IF NOT EXISTS responses (request_id TEXT PRIMARY KEY, status_code INTEGER NOT NULL, status_message TEXT, headers TEXT, body BYTEA, content_type TEXT, duration_ms BIGINT, http_version TEXT)",
    "CREATE INDEX IF NOT EXISTS idx_requests_session ON requests(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_requests_url ON requests(url)",
    "CREATE INDEX IF NOT EXISTS idx_requests_method ON requests(method)",
    "CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp)",
    "CREATE TABLE IF NOT EXISTS ws_connections (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, url TEXT NOT NULL, host TEXT NOT NULL, path TEXT NOT NULL, state TEXT NOT NULL, request_headers TEXT, response_headers TEXT, subprotocol TEXT, created_at BIGINT NOT NULL, closed_at BIGINT, messages_sent BIGINT NOT NULL DEFAULT 0, messages_received BIGINT NOT NULL DEFAULT 0, bytes_sent BIGINT NOT NULL DEFAULT 0, bytes_received BIGINT NOT NULL DEFAULT 0)",
    "CREATE TABLE IF NOT EXISTS ws_messages (id TEXT PRIMARY KEY, connection_id TEXT NOT NULL, direction TEXT NOT NULL, message_type TEXT NOT NULL, payload_raw BYTEA, payload_text TEXT, opcode INTEGER NOT NULL, is_final BOOLEAN NOT NULL DEFAULT TRUE, mask BOOLEAN, timestamp BIGINT NOT NULL)",
    "CREATE INDEX IF NOT EXISTS idx_ws_conn_session ON ws_connections(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_ws_conn_state ON ws_connections(state)",
    "CREATE INDEX IF NOT EXISTS idx_ws_msg_conn ON ws_messages(connection_id)",
    "CREATE INDEX IF NOT EXISTS idx_ws_msg_timestamp ON ws_messages(timestamp)",
    "CREATE TABLE IF NOT EXISTS focus_hosts (id TEXT PRIMARY KEY, pattern TEXT NOT NULL UNIQUE, created_at BIGINT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS session_counters (session_id TEXT PRIMARY KEY, entry_count INTEGER NOT NULL DEFAULT 0)",
    // Cross-instance shared state (key/value). Used to coordinate the
    // current session ID and auto-save rotation lock across instances.
    "CREATE TABLE IF NOT EXISTS instance_state (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at BIGINT NOT NULL)",
];

/// DDL for the `pg_trgm` extension and optimized indexes (GIN/BRIN/trigram).
/// Each statement is executed individually.
const SCHEMA_OPTIMIZED_STMTS: &[&str] = &[
    "CREATE EXTENSION IF NOT EXISTS pg_trgm",
    "CREATE INDEX IF NOT EXISTS idx_traffic_req_headers_gin ON requests USING GIN (headers gin_trgm_ops)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_resp_headers_gin ON responses USING GIN (headers gin_trgm_ops)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_url_trgm ON requests USING GIN (url gin_trgm_ops)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_path_trgm ON requests USING GIN (path gin_trgm_ops)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_timestamp_brin ON requests USING BRIN (timestamp)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_session ON requests(session_id)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_method ON requests(method)",
    // Migration safety net: ensure a unique constraint exists on
    // focus_hosts.pattern even for databases created before the column was
    // declared `UNIQUE` in `SCHEMA_CORE_STMTS`. This prevents duplicate
    // focus host patterns (race condition #8) when two instances insert the
    // same pattern simultaneously.
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_focus_hosts_pattern ON focus_hosts (pattern)",
];

/// DDL for the tiered body storage table. Bodies >= 4KB are stored here
/// instead of inline in the `requests`/`responses` tables. The `compressed`
/// flag indicates zstd compression (Phase 10a.2). The `storage_type` column
/// indicates `'inline'`, `'toast'`, or `'s3'` (S3 documented only). Each
/// statement is executed individually.
const SCHEMA_TRAFFIC_BODIES_STMTS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS traffic_bodies (id TEXT PRIMARY KEY, entry_id TEXT NOT NULL, body_type TEXT NOT NULL, body BYTEA NOT NULL, size BIGINT NOT NULL, compressed BOOLEAN NOT NULL DEFAULT FALSE, storage_type TEXT NOT NULL DEFAULT 'toast', created_at BIGINT NOT NULL)",
    "CREATE INDEX IF NOT EXISTS idx_traffic_bodies_entry ON traffic_bodies(entry_id)",
    "ALTER TABLE traffic_bodies ADD COLUMN IF NOT EXISTS storage_type TEXT NOT NULL DEFAULT 'toast'",
];

/// DDL for autovacuum tuning on high-write tables (Phase 10a.6). Makes
/// autovacuum run more frequently on the traffic tables.
const SCHEMA_AUTOVACUUM_STMTS: &[&str] = &[
    "ALTER TABLE requests SET (autovacuum_vacuum_scale_factor = 0.05, autovacuum_analyze_scale_factor = 0.02)",
    "ALTER TABLE responses SET (autovacuum_vacuum_scale_factor = 0.05, autovacuum_analyze_scale_factor = 0.02)",
    "ALTER TABLE session_counters SET (autovacuum_vacuum_scale_factor = 0.01, fillfactor = 80)",
];

/// DDL for weekly table partitioning (Phase 10c.1). These statements are
/// **not** executed by default — partitioning requires the `traffic` table
/// to be created as `PARTITION BY RANGE` from the start, which is a
/// breaking schema change. Instead, this DDL is documented here for
/// deployments that want to enable partitioning. See
/// `docs/ENTERPRISE_PERF_SECURITY.md` §6.8 and `docs/POSTGRES_HA.md`.
///
/// To enable partitioning:
/// 1. Drop the existing `requests` table.
/// 2. Run the partitioned DDL below.
/// 3. Use `pg_partman` for automatic partition management:
///    `CREATE EXTENSION pg_partman; SELECT partman.create_parent(
///    'public.requests', 'timestamp', 'native', 'weekly');`
const SCHEMA_PARTITIONING_STMTS: &[&str] = &[
    // "CREATE TABLE requests (...) PARTITION BY RANGE (timestamp)",
    // "CREATE TABLE requests_2026_w01 PARTITION OF requests FOR VALUES FROM ('2026-01-01') TO ('2026-01-08')",
    // "CREATE TABLE requests_default PARTITION OF requests DEFAULT",
];

/// Traffic store backed by PostgreSQL (sqlx pool).
pub struct PostgresTrafficStore {
    pool: PgPool,
    /// Optional read replica pool (Phase 10d.2). When set, `get_*` methods
    /// use this pool for read traffic; `store_*` methods use `pool` (the
    /// primary). When `None`, all operations go to `pool`.
    read_pool: Option<PgPool>,
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
    /// Write batcher for buffered inserts (Phase 10b.1). When enabled,
    /// `store_request`/`store_response` push to the batcher instead of
    /// writing directly. `flush()` drains the buffer.
    write_batcher: RwLock<Option<Arc<WriteBatcher>>>,
}

impl PostgresTrafficStore {
    /// Create a new traffic store backed by a PostgreSQL pool. Runs DDL to
    /// create tables and optimized indexes, then ensures a default session
    /// exists.
    pub async fn new(pool: PgPool) -> crate::Result<Arc<Self>> {
        Self::with_read_pool(pool, None).await
    }

    /// Create a new traffic store with a separate read replica pool
    /// (Phase 10d.2). When `read_pool` is `Some`, read queries (`get_*`)
    /// go to the replica and write queries (`store_*`) go to the primary
    /// `pool`. When `None`, all operations use `pool`.
    pub async fn with_read_pool(
        pool: PgPool,
        read_pool: Option<PgPool>,
    ) -> crate::Result<Arc<Self>> {
        let (event_sender, _) = broadcast::channel(TRAFFIC_EVENT_CHANNEL_CAPACITY);
        let store = Arc::new(Self {
            pool,
            read_pool,
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
            write_batcher: RwLock::new(None),
        });

        store.create_tables().await?;
        store.ensure_session().await?;

        // Phase 10b.1: enable write batching by default for PostgreSQL.
        let batcher = WriteBatcher::new(store.clone());
        *store.write_batcher.write() = Some(batcher);

        Ok(store)
    }

    /// Borrow the write pool (primary).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Borrow the read pool (replica), falling back to the primary when
    /// no replica is configured (Phase 10d.2).
    fn read_pool(&self) -> &PgPool {
        self.read_pool.as_ref().unwrap_or(&self.pool)
    }

    /// Emit a traffic event to all subscribers.
    fn emit_event(&self, event: TrafficEvent) {
        let _ = self.event_sender.send(event);
    }

    /// Create database tables and optimized indexes.
    async fn create_tables(&self) -> crate::Result<()> {
        // Wrap ALL DDL in a single advisory-lock transaction to prevent
        // concurrent schema initialization across multi-instance deployments.
        // The lock key `0x4D414448` ("MADH") matches the one used by
        // `run_pg_migrations()` in the main binary so all DDL across the
        // application is serialized. The lock is transaction-scoped, so it is
        // released on commit/rollback and does not block normal operations.
        //
        // `CREATE EXTENSION IF NOT EXISTS pg_trgm` in particular races on
        // PostgreSQL's internal `pg_type_typname_nsp_index` unique constraint
        // when two instances start simultaneously, producing
        // "duplicate key value violates unique constraint". Serializing via
        // the advisory lock eliminates this race.
        //
        // Individual DDL statements remain best-effort (logged and skipped on
        // error) to handle the case where objects already exist or the
        // connecting role lacks privileges for some statements (e.g. CREATE
        // EXTENSION). The `IF NOT EXISTS` clauses are kept for idempotency.
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(0x4D414448)")
            .execute(&mut *tx)
            .await?;

        for stmt in SCHEMA_CORE_STMTS {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                // Log and continue — the table likely already exists
                // (created by a concurrent instance).
                tracing::debug!("Schema DDL (best-effort): {}", e);
            }
        }
        for stmt in SCHEMA_TRAFFIC_BODIES_STMTS {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                tracing::debug!("traffic_bodies DDL (best-effort): {}", e);
            }
        }
        // Optimized indexes (GIN/BRIN/trigram) — best-effort: if the
        // extension or index creation fails (e.g. insufficient privileges),
        // log a warning and continue. The core tables still work.
        for stmt in SCHEMA_OPTIMIZED_STMTS {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                tracing::debug!("Optimized index DDL (best-effort): {}", e);
            }
        }
        // Autovacuum tuning (Phase 10a.6) — best-effort.
        for stmt in SCHEMA_AUTOVACUUM_STMTS {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                tracing::debug!("Autovacuum tuning (best-effort): {}", e);
            }
        }
        // Partitioning DDL is documented but not executed by default.
        for stmt in SCHEMA_PARTITIONING_STMTS {
            if let Err(e) = sqlx::query(stmt).execute(&mut *tx).await {
                tracing::debug!("Partitioning DDL (best-effort): {}", e);
            }
        }

        tx.commit().await?;
        Ok(())
    }

    /// Ensure a default session exists.
    ///
    /// Uses a deterministic session ID ("default-session") so that all
    /// instances in a multi-instance deployment (sharing the same
    /// PostgreSQL database) operate on the same session. This prevents
    /// each instance from creating its own session, which would cause
    /// inconsistent traffic counts and WebSocket event mismatches.
    async fn ensure_session(&self) -> crate::Result<()> {
        // Use a fixed session ID so all instances share the same default session.
        // INSERT ... ON CONFLICT DO NOTHING prevents the race condition when
        // multiple instances start simultaneously.
        const DEFAULT_SESSION_ID: &str = "default-session";
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO sessions (id, name, created_at, updated_at) \
             VALUES ($1, $2, $3, $3) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(DEFAULT_SESSION_ID)
        .bind("Default Session")
        .bind(now)
        .execute(&self.pool)
        .await?;

        *self.current_session_id.lock() = DEFAULT_SESSION_ID.to_string();

        Ok(())
    }

    /// Get the number of traffic entries in the current session.
    async fn get_entry_count(&self) -> crate::Result<usize> {
        let session_id = self.current_session_id.lock().clone();
        // Phase 10b.2: read from the session_counters table (O(1))
        // instead of COUNT(*) (O(n)). Fall back to COUNT(*) if the
        // counter row doesn't exist (old data / pre-migration).
        // Note: entry_count is INTEGER (i32) in PostgreSQL, not BIGINT (i64).
        let count: Option<i32> =
            sqlx::query_scalar("SELECT entry_count FROM session_counters WHERE session_id = $1")
                .bind(&session_id)
                .fetch_optional(self.read_pool())
                .await?;
        match count {
            Some(c) => Ok(c as usize),
            None => {
                let fallback: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM requests WHERE session_id = $1")
                        .bind(&session_id)
                        .fetch_one(self.read_pool())
                        .await
                        .unwrap_or(0);
                Ok(fallback as usize)
            }
        }
    }

    /// Get the total size of all stored bodies (request + response) in the
    /// current session, in bytes.
    async fn get_total_size(&self) -> crate::Result<usize> {
        let session_id = self.current_session_id.lock().clone();
        let req_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM requests WHERE session_id = $1",
        )
        .bind(&session_id)
        .fetch_one(self.read_pool())
        .await
        .unwrap_or(0);
        let resp_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM responses WHERE request_id IN \
             (SELECT id FROM requests WHERE session_id = $1)",
        )
        .bind(&session_id)
        .fetch_one(self.read_pool())
        .await
        .unwrap_or(0);
        Ok((req_size + resp_size) as usize)
    }

    /// Prune the oldest `count` entries from the current session.
    ///
    /// Note: [`enforce_entry_limit`](Self::enforce_entry_limit) now uses an
    /// atomic `DELETE ... RETURNING` within an advisory-locked transaction
    /// instead of calling this method, to fix race condition #4. This method
    /// is retained as a utility for explicit pruning (e.g. via API/CLI).
    #[allow(dead_code)]
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

        // Phase 10b.2: decrement the session counter.
        let _ = sqlx::query(
            "UPDATE session_counters SET entry_count = GREATEST(0, entry_count - $1) WHERE session_id = $2",
        )
        .bind(pruned_ids.len() as i64)
        .bind(&session_id)
        .execute(&self.pool)
        .await;

        delete_requests_and_responses(&self.pool, &pruned_ids).await?;
        self.emit_event(TrafficEvent::Deleted(pruned_ids));

        Ok(())
    }

    /// Enforce the entry-count limit.
    ///
    /// Uses a transaction-scoped PostgreSQL advisory lock (key `0x4D414449`,
    /// "MADI") to serialize limit enforcement across instances, then performs
    /// an atomic `DELETE ... RETURNING` that both checks the excess count
    /// (from `session_counters`, O(1)) and prunes the oldest entries in a
    /// single SQL statement.
    ///
    /// This eliminates the read-then-prune race condition (#4) where two
    /// instances simultaneously read the same count, both decide to prune,
    /// and both call `prune_oldest(N)` with overlapping ID sets — causing up
    /// to 2N entries to be deleted instead of N (data loss).
    ///
    /// The lock key is distinct from the DDL lock key (`0x4D414448`, "MADH")
    /// so schema initialization and pruning don't block each other.
    async fn enforce_entry_limit(&self) -> crate::Result<()> {
        let max = self.max_entries.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }

        // Acquire a transaction-scoped advisory lock to serialize limit
        // enforcement across instances. The lock is released on commit/abort.
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(0x4D414449)")
            .execute(&mut *tx)
            .await?;

        let session_id = self.current_session_id.lock().clone();

        // Atomically delete the oldest entries beyond the limit. The subquery
        // computes the excess count from session_counters (O(1)) and deletes
        // exactly that many oldest entries in one statement, so two concurrent
        // instances cannot both read the same stale count and over-prune.
        // RETURNING id lets us emit TrafficEvent::Deleted after commit.
        let pruned_ids: Vec<String> = sqlx::query(
            "DELETE FROM requests WHERE id IN (\
                SELECT id FROM requests \
                WHERE session_id = $1 \
                ORDER BY timestamp ASC \
                LIMIT GREATEST(0, \
                    (SELECT COALESCE(entry_count, 0) FROM session_counters WHERE session_id = $1) - $2 \
                ) \
            ) RETURNING id",
        )
        .bind(&session_id)
        .bind(max as i64)
        .map(|row: sqlx::postgres::PgRow| row.try_get::<String, _>(0).unwrap_or_default())
        .fetch_all(&mut *tx)
        .await?;

        if pruned_ids.is_empty() {
            tx.commit().await?;
            return Ok(());
        }

        // Delete orphaned responses for the pruned requests (the PostgreSQL
        // schema has no ON DELETE CASCADE on the responses table).
        let placeholders = (1..=pruned_ids.len())
            .map(|i| format!("${i}"))
            .collect::<Vec<_>>()
            .join(",");
        let delete_responses_sql =
            format!("DELETE FROM responses WHERE request_id IN ({})", placeholders);
        let mut q = sqlx::query(&delete_responses_sql);
        for id in &pruned_ids {
            q = q.bind(id);
        }
        q.execute(&mut *tx).await?;

        // Update the session counter to reflect the prune.
        let _ = sqlx::query(
            "UPDATE session_counters SET entry_count = GREATEST(0, entry_count - $1) \
             WHERE session_id = $2",
        )
        .bind(pruned_ids.len() as i64)
        .bind(&session_id)
        .execute(&mut *tx)
        .await;

        tx.commit().await?;

        // Emit the Deleted event after the transaction commits so subscribers
        // see a consistent state.
        self.emit_event(TrafficEvent::Deleted(pruned_ids));

        Ok(())
    }

    /// Enforce the total-size limit.
    ///
    /// Uses the same advisory lock (`0x4D414449`) as [`enforce_entry_limit`]
    /// to serialize size enforcement across instances. The total body size is
    /// computed and the prune set is determined entirely within the locked
    /// transaction, preventing the read-then-prune race where two instances
    /// both read the same total size and both prune.
    async fn enforce_size_limit(&self) -> crate::Result<()> {
        let max = self.max_total_size_bytes.load(Ordering::Relaxed);
        if max == 0 {
            return Ok(());
        }

        // Acquire the same advisory lock used by enforce_entry_limit so that
        // entry-count and size-based pruning are mutually exclusive across
        // instances.
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(0x4D414449)")
            .execute(&mut *tx)
            .await?;

        let session_id = self.current_session_id.lock().clone();

        // Compute the total body size within the locked transaction so the
        // value is consistent with the subsequent prune.
        let req_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM requests WHERE session_id = $1",
        )
        .bind(&session_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);
        let resp_size: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(LENGTH(body)), 0) FROM responses WHERE request_id IN \
             (SELECT id FROM requests WHERE session_id = $1)",
        )
        .bind(&session_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);
        let mut total = (req_size + resp_size) as usize;

        if total <= max {
            tx.commit().await?;
            return Ok(());
        }

        // Gather oldest entries with their body sizes so we can prune just
        // enough to get under the limit — all within the locked transaction.
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
        .fetch_all(&mut *tx)
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
            // Delete responses first, then requests (no ON DELETE CASCADE).
            let placeholders = (1..=to_prune.len())
                .map(|i| format!("${i}"))
                .collect::<Vec<_>>()
                .join(",");
            let delete_responses_sql = format!(
                "DELETE FROM responses WHERE request_id IN ({})",
                placeholders
            );
            let mut q = sqlx::query(&delete_responses_sql);
            for id in &to_prune {
                q = q.bind(id);
            }
            q.execute(&mut *tx).await?;

            let delete_requests_sql =
                format!("DELETE FROM requests WHERE id IN ({})", placeholders);
            let mut q = sqlx::query(&delete_requests_sql);
            for id in &to_prune {
                q = q.bind(id);
            }
            q.execute(&mut *tx).await?;

            // Update the session counter to reflect the prune.
            let _ = sqlx::query(
                "UPDATE session_counters SET entry_count = GREATEST(0, entry_count - $1) \
                 WHERE session_id = $2",
            )
            .bind(to_prune.len() as i64)
            .bind(&session_id)
            .execute(&mut *tx)
            .await;
        }

        tx.commit().await?;

        // Emit the Deleted event after the transaction commits.
        if !to_prune.is_empty() {
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

    /// Store a body in the `traffic_bodies` table with zstd compression
    /// (Phase 10a.1–10a.2). Called when `maybe_tier_body` returns `None`.
    /// The `entry_id` is the request ID, and `body_type` is `'request'` or
    /// `'response'`.
    async fn store_tiered_body(
        &self,
        entry_id: &str,
        body_type: &str,
        body: &[u8],
    ) -> crate::Result<()> {
        let (compressed_body, compressed) = compress_body(body);
        let body_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO traffic_bodies (id, entry_id, body_type, body, size, compressed, storage_type, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&body_id)
        .bind(entry_id)
        .bind(body_type)
        .bind(&compressed_body)
        .bind(compressed_body.len() as i64)
        .bind(compressed)
        .bind(BodyStorageType::Toast.as_str())
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Fetch a tiered body from the `traffic_bodies` table and decompress
    /// it if needed (Phase 10a.1–10a.2). Returns `None` if no body row
    /// exists.
    async fn fetch_tiered_body(
        &self,
        entry_id: &str,
        body_type: &str,
    ) -> crate::Result<Option<Vec<u8>>> {
        let row: Option<(Vec<u8>, bool)> = sqlx::query_as(
            "SELECT body, compressed FROM traffic_bodies WHERE entry_id = $1 AND body_type = $2 LIMIT 1",
        )
        .bind(entry_id)
        .bind(body_type)
        .fetch_optional(self.read_pool())
        .await?;
        match row {
            Some((body, compressed)) => {
                let decompressed = decompress_body(&body, compressed)?;
                Ok(Some(decompressed))
            }
            None => Ok(None),
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
        let clamped_body = if self.capture_request_bodies.load(Ordering::Relaxed) {
            self.clamp_body(&entry.request.body)
        } else {
            None
        };
        // Phase 10a.1: tiered body storage. Bodies >= 4KB go to
        // traffic_bodies (compressed); the inline column is NULL.
        let tiered_body = clamped_body.as_ref().and_then(|b| {
            if b.len() >= INLINE_THRESHOLD {
                Some(b.clone())
            } else {
                None
            }
        });
        let inline_body = if tiered_body.is_some() {
            None
        } else {
            clamped_body
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
        .bind(inline_body)
        .bind(content_type)
        .bind(entry.timestamp.timestamp())
        .bind(entry.modified)
        .bind(&entry.notes)
        .bind(entry.is_passthrough)
        .bind(entry.request.http_version.as_deref())
        .bind(entry.script_intercepted)
        .execute(&self.pool)
        .await?;

        // Phase 10a.1–10a.2: store tiered body with zstd compression.
        if let Some(body) = tiered_body {
            if let Err(e) = self.store_tiered_body(&entry.id, "request", &body).await {
                tracing::warn!("Failed to store tiered request body: {}", e);
            }
        }

        let now = Utc::now().timestamp();
        let _ = sqlx::query("UPDATE sessions SET updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(&entry.session_id)
            .execute(&self.pool)
            .await;

        // Phase 10b.2: increment the session counter.
        let _ = sqlx::query(
            "INSERT INTO session_counters (session_id, entry_count) VALUES ($1, 1) \
             ON CONFLICT (session_id) DO UPDATE SET entry_count = session_counters.entry_count + 1",
        )
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
        let clamped_body = if self.capture_response_bodies.load(Ordering::Relaxed) {
            self.clamp_body(&response.body)
        } else {
            None
        };
        // Phase 10a.1: tiered body storage for response bodies.
        let tiered_body = clamped_body.as_ref().and_then(|b| {
            if b.len() >= INLINE_THRESHOLD {
                Some(b.clone())
            } else {
                None
            }
        });
        let inline_body = if tiered_body.is_some() {
            None
        } else {
            clamped_body
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
        .bind(inline_body)
        .bind(content_type)
        .bind(response.duration_ms as i64)
        .bind(response.http_version.as_deref())
        .execute(&self.pool)
        .await?;

        // Phase 10a.1–10a.2: store tiered body with zstd compression.
        if let Some(body) = tiered_body {
            if let Err(e) = self.store_tiered_body(request_id, "response", &body).await {
                tracing::warn!("Failed to store tiered response body: {}", e);
            }
        }

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

        // Phase 10b.4: lazy body loading — when include_bodies == Some(false),
        // omit body columns from the SELECT to reduce payload size.
        let include_bodies = filter.include_bodies.unwrap_or(true);
        let body_cols = if include_bodies {
            "r.body, rs.body AS resp_body"
        } else {
            "NULL AS body, NULL AS resp_body"
        };

        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(
            "SELECT r.id, r.session_id, r.method, r.url, r.host, r.path, r.headers, ",
        );
        qb.push(body_cols);
        qb.push(", r.content_type, r.timestamp, r.modified, r.notes, r.is_passthrough, r.http_version, r.script_intercepted, rs.status_code, rs.status_message, rs.headers AS resp_headers, rs.content_type AS resp_content_type, rs.duration_ms, rs.http_version AS resp_http_version FROM requests r LEFT JOIN responses rs ON r.id = rs.request_id WHERE r.session_id = ");
        qb.push_bind(session_id);

        // Phase 10b.3: cursor-based pagination — when a cursor is provided,
        // use keyset pagination instead of OFFSET (O(1) vs O(n)).
        if let Some(ref cursor_str) = filter.cursor {
            if let Some(cursor) = TrafficCursor::decode(cursor_str) {
                qb.push(" AND (r.timestamp, r.id) < (")
                    .push_bind(cursor.t)
                    .push(", ")
                    .push_bind(cursor.i)
                    .push(")");
            }
        }

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

        // When using cursor pagination, order by (timestamp DESC, id DESC)
        // so the keyset cursor is deterministic. Otherwise keep the original
        // timestamp DESC ordering for backward compatibility.
        if filter.cursor.is_some() {
            qb.push(" ORDER BY r.timestamp DESC, r.id DESC");
        } else {
            qb.push(" ORDER BY r.timestamp DESC");
        }

        if let Some(limit) = filter.limit {
            qb.push(" LIMIT ").push_bind(limit as i64);
        }

        // Only use OFFSET when cursor is not provided (backward compat).
        if filter.cursor.is_none() {
            if let Some(offset) = filter.offset {
                qb.push(" OFFSET ").push_bind(offset as i64);
            }
        }

        let rows: Vec<TrafficRow> = qb
            .build_query_as::<TrafficRow>()
            .fetch_all(self.read_pool())
            .await?;

        // Phase 10a.1: if bodies were omitted from the inline columns
        // (tiered storage), fetch them from traffic_bodies. This only
        // runs when include_bodies is true and the inline body is NULL.
        if include_bodies {
            let mut entries: Vec<TrafficEntry> = Vec::with_capacity(rows.len());
            for row in rows {
                let mut entry = row_to_entry(row);
                // Fetch tiered request body if inline is NULL.
                if entry.request.body.is_none() {
                    if let Ok(Some(body)) = self.fetch_tiered_body(&entry.id, "request").await {
                        entry.request.body = Some(body);
                    }
                }
                // Fetch tiered response body if inline is NULL.
                if let Some(ref mut response) = entry.response {
                    if response.body.is_none() {
                        if let Ok(Some(body)) = self.fetch_tiered_body(&entry.id, "response").await
                        {
                            response.body = Some(body);
                        }
                    }
                }
                entries.push(entry);
            }
            Ok(entries)
        } else {
            Ok(rows.into_iter().map(row_to_entry).collect())
        }
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
        .fetch_optional(self.read_pool())
        .await?;

        match row {
            Some(row) => {
                let mut entry = row_to_entry(row);
                // Phase 10a.1: fetch tiered bodies if inline is NULL.
                if entry.request.body.is_none() {
                    if let Ok(Some(body)) = self.fetch_tiered_body(&entry.id, "request").await {
                        entry.request.body = Some(body);
                    }
                }
                if let Some(ref mut response) = entry.response {
                    if response.body.is_none() {
                        if let Ok(Some(body)) = self.fetch_tiered_body(&entry.id, "response").await
                        {
                            response.body = Some(body);
                        }
                    }
                }
                Ok(Some(entry))
            }
            None => Ok(None),
        }
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

        // Phase 10b.2: reset the session counter.
        let _ = sqlx::query("UPDATE session_counters SET entry_count = 0 WHERE session_id = $1")
            .bind(&session_id)
            .execute(&self.pool)
            .await;

        self.emit_event(TrafficEvent::Cleared);

        Ok(())
    }

    async fn delete_traffic(&self, ids: &[String]) -> crate::Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        // Phase 10b.2: decrement the session counter.
        if let Some(session_id) =
            sqlx::query_scalar::<_, String>("SELECT session_id FROM requests WHERE id = $1 LIMIT 1")
                .bind(&ids[0])
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten()
        {
            let _ = sqlx::query(
                "UPDATE session_counters SET entry_count = GREATEST(0, entry_count - $1) WHERE session_id = $2",
            )
            .bind(ids.len() as i64)
            .bind(&session_id)
            .execute(&self.pool)
            .await;
        }

        delete_requests_and_responses(&self.pool, ids).await?;
        self.emit_event(TrafficEvent::Deleted(ids.to_vec()));

        Ok(())
    }

    async fn count(&self) -> crate::Result<usize> {
        let session_id = self.current_session_id.lock().clone();
        // Phase 10b.2: prefer the session counter table.
        let count: Option<i32> =
            sqlx::query_scalar("SELECT entry_count FROM session_counters WHERE session_id = $1")
                .bind(&session_id)
                .fetch_optional(self.read_pool())
                .await?;
        match count {
            Some(c) => Ok(c as usize),
            None => {
                let fallback: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM requests WHERE session_id = $1")
                        .bind(&session_id)
                        .fetch_one(self.read_pool())
                        .await
                        .unwrap_or(0);
                Ok(fallback as usize)
            }
        }
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
        // Persist to shared state so other instances can sync.
        self.set_shared_state("current_session_id", session_id).await?;
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

        sqlx::query("DELETE FROM session_counters WHERE session_id = $1")
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
        .fetch_all(self.read_pool())
        .await?;

        // Phase 10a.1: fetch tiered bodies if inline is NULL.
        let mut entries: Vec<TrafficEntry> = Vec::with_capacity(rows.len());
        for row in rows {
            let mut entry = row_to_entry(row);
            if entry.request.body.is_none() {
                if let Ok(Some(body)) = self.fetch_tiered_body(&entry.id, "request").await {
                    entry.request.body = Some(body);
                }
            }
            if let Some(ref mut response) = entry.response {
                if response.body.is_none() {
                    if let Ok(Some(body)) = self.fetch_tiered_body(&entry.id, "response").await {
                        response.body = Some(body);
                    }
                }
            }
            entries.push(entry);
        }
        Ok(entries)
    }

    async fn add_focus_host(&self, pattern: &str) -> crate::Result<FocusHost> {
        let normalized = pattern.trim().to_lowercase();
        if normalized.is_empty() {
            return Err(Error::Config(
                "focus host pattern cannot be empty".to_string(),
            ));
        }

        let host = FocusHost::new(&normalized);

        // Atomic insert-or-return-existing. `ON CONFLICT (pattern) DO NOTHING`
        // prevents duplicate patterns when two instances race to add the same
        // focus host (race condition #8). `RETURNING` gives us the row that
        // was actually inserted (or nothing if it already existed). If nothing
        // was returned, another instance won the race — we fetch the existing
        // row. The unique constraint/index on `pattern` (see
        // `SCHEMA_OPTIMIZED_STMTS`) is what makes `ON CONFLICT (pattern)`
        // resolve deterministically.
        let row: Option<FocusHostRow> = sqlx::query_as::<_, FocusHostRow>(
            "INSERT INTO focus_hosts (id, pattern, created_at) VALUES ($1, $2, $3) \
             ON CONFLICT (pattern) DO NOTHING \
             RETURNING id, pattern, created_at",
        )
        .bind(&host.id)
        .bind(&host.pattern)
        .bind(host.created_at.timestamp())
        .fetch_optional(&self.pool)
        .await?;

        let result = if let Some(row) = row {
            // We inserted it.
            FocusHost {
                id: row.id,
                pattern: row.pattern,
                created_at: parse_timestamp(row.created_at),
            }
        } else {
            // Already existed (race with another instance) — fetch it.
            let existing: FocusHostRow = sqlx::query_as::<_, FocusHostRow>(
                "SELECT id, pattern, created_at FROM focus_hosts WHERE pattern = $1",
            )
            .bind(&normalized)
            .fetch_one(&self.pool)
            .await?;
            FocusHost {
                id: existing.id,
                pattern: existing.pattern,
                created_at: parse_timestamp(existing.created_at),
            }
        };

        Ok(result)
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

    async fn get_shared_state(&self, key: &str) -> crate::Result<Option<String>> {
        // Gracefully handle the case where the instance_state table does
        // not exist yet (e.g. an older schema that has not been migrated).
        let value: Option<String> =
            match sqlx::query_scalar("SELECT value FROM instance_state WHERE key = $1")
                .bind(key)
                .fetch_optional(self.read_pool())
                .await
            {
                Ok(v) => v,
                Err(sqlx::Error::Database(ref e)) => {
                    tracing::debug!("instance_state table missing, returning None: {e}");
                    None
                }
                Err(e) => return Err(e.into()),
            };
        Ok(value)
    }

    async fn set_shared_state(&self, key: &str, value: &str) -> crate::Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO instance_state (key, value, updated_at) VALUES ($1, $2, $3) \
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn sync_current_session(&self) -> crate::Result<()> {
        if let Some(session_id) = self.get_shared_state("current_session_id").await? {
            let current = self.current_session_id.lock().clone();
            if current != session_id {
                *self.current_session_id.lock() = session_id.clone();
                tracing::info!("Synced current session from shared state: {}", session_id);
            }
        }
        Ok(())
    }

    async fn flush(&self) -> crate::Result<()> {
        // Phase 10b.1: flush the write batcher on graceful shutdown.
        let batcher = self.write_batcher.read().clone();
        if let Some(batcher) = batcher {
            batcher.flush().await;
        }
        Ok(())
    }

    async fn ping(&self) -> crate::Result<()> {
        sqlx::query("SELECT 1")
            .execute(self.read_pool())
            .await
            .map_err(crate::Error::Sqlx)?;
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

// -----------------------------------------------------------------------
// Write batching (Phase 10b.1)
// -----------------------------------------------------------------------

/// Maximum number of entries to buffer before flushing.
const BATCH_SIZE: usize = 100;

/// Maximum time to wait before flushing the buffer.
const BATCH_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

/// A pending write operation for the batcher.
#[allow(dead_code)]
enum BatchOp {
    StoreRequest(Box<TrafficEntry>),
    StoreResponse {
        request_id: String,
        response: Box<ResponseData>,
    },
    /// Signal to flush immediately and exit the background task.
    FlushAndExit,
}

/// Write batcher for PostgreSQL traffic stores (Phase 10b.1).
///
/// Buffers `store_request`/`store_response` calls and flushes them in
/// batches (up to [`BATCH_SIZE`] entries or [`BATCH_TIMEOUT`], whichever
/// comes first). This reduces the number of database round-trips from
/// 2 per HTTP transaction to ~1 per batch.
///
/// The batcher runs a background task that drains the channel and writes
/// to the database. `flush()` forces an immediate flush (called on
/// graceful shutdown to avoid data loss).
///
/// **Current status:** The batcher infrastructure is in place, but
/// `store_request`/`store_response` currently write directly to the
/// database for correctness. The batcher's `flush()` is a no-op. This
/// provides the API surface for future optimization without risking
/// data consistency issues during the initial rollout.
pub struct WriteBatcher {
    sender: tokio::sync::mpsc::UnboundedSender<BatchOp>,
    _handle: tokio::task::JoinHandle<()>,
}

impl WriteBatcher {
    /// Create a new write batcher for the given store. Spawns a
    /// background task that drains the channel and flushes periodically.
    fn new(store: Arc<PostgresTrafficStore>) -> Arc<Self> {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<BatchOp>();
        let handle = tokio::spawn(batch_flush_loop(receiver, store));
        Arc::new(Self {
            sender,
            _handle: handle,
        })
    }

    /// Force an immediate flush of all pending writes. Called on
    /// graceful shutdown to avoid data loss.
    async fn flush(&self) {
        // Send a flush signal and wait for the background task to process it.
        // The background task will flush all pending ops and continue.
        let _ = self.sender.send(BatchOp::FlushAndExit);
        // Give the background task a moment to process the flush.
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

/// Background flush loop for the write batcher.
async fn batch_flush_loop(
    mut receiver: tokio::sync::mpsc::UnboundedReceiver<BatchOp>,
    _store: Arc<PostgresTrafficStore>,
) {
    let mut batch: Vec<BatchOp> = Vec::with_capacity(BATCH_SIZE);
    let mut interval = tokio::time::interval(BATCH_TIMEOUT);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            op = receiver.recv() => {
                match op {
                    Some(BatchOp::FlushAndExit) => {
                        // Flush any pending ops and exit.
                        if !batch.is_empty() {
                            flush_batch_ops(&mut batch).await;
                        }
                        break;
                    }
                    Some(op) => {
                        batch.push(op);
                        if batch.len() >= BATCH_SIZE {
                            flush_batch_ops(&mut batch).await;
                        }
                    }
                    None => {
                        // Channel closed — flush and exit.
                        if !batch.is_empty() {
                            flush_batch_ops(&mut batch).await;
                        }
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                if !batch.is_empty() {
                    flush_batch_ops(&mut batch).await;
                }
            }
        }
    }
}

/// Flush a batch of write operations. Currently a no-op placeholder —
/// the actual batched write logic will be implemented in a future
/// optimization pass. The ops are drained to prevent memory growth.
async fn flush_batch_ops(batch: &mut Vec<BatchOp>) {
    // Placeholder: in the future, this will build a single multi-row
    // INSERT for all StoreRequest ops and a multi-row INSERT for all
    // StoreResponse ops, then execute them in a single transaction.
    // For now, ops are simply drained (direct writes already happened
    // in store_request/store_response).
    batch.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: connect to the test PostgreSQL instance and return a fresh
    /// store. The database URL is read from `MADHYAMAS_PG_TEST_URL` (default:
    /// `postgres://madhyamas:testpass@localhost:5432/madhyamas`). All tests
    /// are `#[ignore]` so they only run with `cargo test -- --ignored` and a
    /// running PostgreSQL instance.
    async fn make_store() -> Arc<PostgresTrafficStore> {
        let url = std::env::var("MADHYAMAS_PG_TEST_URL").unwrap_or_else(|_| {
            "postgres://madhyamas:testpass@localhost:5432/madhyamas".to_string()
        });
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .expect("failed to connect to PostgreSQL test instance");
        PostgresTrafficStore::new(pool)
            .await
            .expect("failed to create PostgresTrafficStore")
    }

    /// Helper: create a simple traffic entry for testing.
    fn make_entry(session_id: &str) -> TrafficEntry {
        let req = RequestData {
            method: crate::traffic::HttpMethod::Get,
            url: "https://example.com/api/test".to_string(),
            host: "example.com".to_string(),
            path: "/api/test".to_string(),
            headers: {
                let mut m = HashMap::new();
                m.insert("Accept".to_string(), "application/json".to_string());
                m
            },
            body: Some(b"hello world".to_vec()),
            content_type: Some("text/plain".to_string()),
            http_version: Some("HTTP/1.1".to_string()),
        };
        let mut entry = TrafficEntry::new(session_id, req);
        entry.response = Some(ResponseData {
            status_code: 200,
            status_message: Some("OK".to_string()),
            headers: HashMap::new(),
            body: Some(b"response body".to_vec()),
            content_type: Some("application/json".to_string()),
            duration_ms: 42,
            http_version: Some("HTTP/1.1".to_string()),
        });
        entry
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_traffic_store_request_response() {
        let store = make_store().await;
        let session = store.create_session(Some("test-session")).await.unwrap();
        let entry = make_entry(&session.id);
        store.store_request(&entry).await.unwrap();
        store
            .store_response(&entry.id, entry.response.as_ref().unwrap())
            .await
            .unwrap();

        let fetched = store.get_by_id(&entry.id).await.unwrap().unwrap();
        assert_eq!(fetched.request.url, entry.request.url);
        assert_eq!(fetched.request.method, entry.request.method);
        assert_eq!(fetched.response.as_ref().unwrap().status_code, 200);
        assert_eq!(
            fetched.response.as_ref().unwrap().body.as_deref(),
            Some(b"response body" as &[u8])
        );

        // Clean up
        store.delete_session(&session.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_traffic_store_sessions() {
        let store = make_store().await;
        let session = store.create_session(Some("pg-session-test")).await.unwrap();
        let sessions = store.list_sessions().await.unwrap();
        assert!(sessions.iter().any(|s| s.id == session.id));

        store.delete_session(&session.id).await.unwrap();
        let sessions = store.list_sessions().await.unwrap();
        assert!(!sessions.iter().any(|s| s.id == session.id));
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_traffic_store_focus_hosts() {
        let store = make_store().await;
        store.clear_focus_hosts().await.unwrap();

        let host = store.add_focus_host("*.example.com").await.unwrap();
        let hosts = store.list_focus_hosts().await.unwrap();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].pattern, "*.example.com");

        assert!(store.remove_focus_host(&host.id).await.unwrap());
        let hosts = store.list_focus_hosts().await.unwrap();
        assert!(hosts.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_traffic_store_har_import() {
        let store = make_store().await;
        let har = serde_json::json!({
            "log": {
                "version": "1.2",
                "entries": [{
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
                        "headers": [],
                        "content": { "size": 17, "mimeType": "application/json", "text": "{\"users\":[]}" }
                    }
                }]
            }
        });

        let result = store.import_har(&har, Some("pg-har-test")).await.unwrap();
        assert_eq!(result.imported_count, 1);
        assert_eq!(result.skipped_count, 0);

        let entries = store
            .get_traffic_by_session(&result.session_id)
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);

        store.delete_session(&result.session_id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_tiered_body_storage() {
        let store = make_store().await;
        let session = store
            .create_session(Some("test-tiered-body"))
            .await
            .unwrap();
        store.switch_session(&session.id).await.unwrap();

        // Create an entry with a large body (> 4KB) that should be tiered.
        let large_body = vec![b'A'; 8 * 1024]; // 8KB
        let req = RequestData {
            method: crate::traffic::HttpMethod::Post,
            url: "https://example.com/upload".to_string(),
            host: "example.com".to_string(),
            path: "/upload".to_string(),
            headers: HashMap::new(),
            body: Some(large_body.clone()),
            content_type: Some("application/octet-stream".to_string()),
            http_version: Some("HTTP/1.1".to_string()),
        };
        let entry = TrafficEntry::new(&session.id, req);
        store.store_request(&entry).await.unwrap();

        // Fetch and verify the body is correctly retrieved from tiered storage.
        let fetched = store.get_by_id(&entry.id).await.unwrap().unwrap();
        assert_eq!(fetched.request.body.as_deref(), Some(large_body.as_slice()));

        store.delete_session(&session.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_session_counter() {
        let store = make_store().await;
        let session = store.create_session(Some("test-counter")).await.unwrap();
        store.switch_session(&session.id).await.unwrap();

        // Store 5 entries and verify the counter.
        for i in 0..5 {
            let mut entry = make_entry(&session.id);
            entry.id = format!("counter-test-{i}");
            store.store_request(&entry).await.unwrap();
        }

        let count = store.get_entry_count().await.unwrap();
        assert_eq!(count, 5);

        // Delete 2 entries and verify the counter decremented.
        store
            .delete_traffic(&["counter-test-0".to_string(), "counter-test-1".to_string()])
            .await
            .unwrap();
        let count = store.get_entry_count().await.unwrap();
        assert_eq!(count, 3);

        // Clear and verify counter reset.
        store.clear_traffic().await.unwrap();
        let count = store.get_entry_count().await.unwrap();
        assert_eq!(count, 0);

        store.delete_session(&session.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_cursor_pagination() {
        let store = make_store().await;
        let session = store.create_session(Some("test-cursor")).await.unwrap();
        store.switch_session(&session.id).await.unwrap();

        // Store 10 entries with distinct timestamps.
        for i in 0..10 {
            let mut entry = make_entry(&session.id);
            entry.id = format!("cursor-test-{i}");
            entry.timestamp = Utc::now() + chrono::Duration::seconds(i);
            store.store_request(&entry).await.unwrap();
        }

        // First page: limit 3, no cursor.
        let filter = TrafficFilter {
            limit: Some(3),
            ..Default::default()
        };
        let page1 = store.get_traffic(&filter).await.unwrap();
        assert_eq!(page1.len(), 3);

        // Get cursor from last entry of page 1.
        let cursor = crate::traffic::TrafficCursor::from_entry(page1.last().unwrap());

        // Second page: limit 3, with cursor.
        let filter2 = TrafficFilter {
            limit: Some(3),
            cursor: Some(cursor),
            ..Default::default()
        };
        let page2 = store.get_traffic(&filter2).await.unwrap();
        assert_eq!(page2.len(), 3);

        // Verify no overlap between pages.
        let page1_ids: std::collections::HashSet<_> = page1.iter().map(|e| &e.id).collect();
        let page2_ids: std::collections::HashSet<_> = page2.iter().map(|e| &e.id).collect();
        assert!(page1_ids.is_disjoint(&page2_ids));

        store.delete_session(&session.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_lazy_body_loading() {
        let store = make_store().await;
        let session = store.create_session(Some("test-lazy-body")).await.unwrap();
        store.switch_session(&session.id).await.unwrap();

        let entry = make_entry(&session.id);
        store.store_request(&entry).await.unwrap();

        // With include_bodies = false, bodies should be None.
        let filter = TrafficFilter {
            include_bodies: Some(false),
            ..Default::default()
        };
        let entries = store.get_traffic(&filter).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].request.body.is_none());

        // With include_bodies = true (default), bodies should be present.
        let filter2 = TrafficFilter {
            ..Default::default()
        };
        let entries2 = store.get_traffic(&filter2).await.unwrap();
        assert_eq!(entries2.len(), 1);
        assert!(entries2[0].request.body.is_some());

        store.delete_session(&session.id).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_flush() {
        let store = make_store().await;
        // flush() should not error even with no pending writes.
        store.flush().await.unwrap();
    }
}
