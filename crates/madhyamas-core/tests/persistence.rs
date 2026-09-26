//! Integration tests for the public storage APIs: body compression and
//! classification, SQLite intercept-store schema migration, and the
//! PostgreSQL traffic store (live-database tests, `#[ignore]`-gated).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use madhyamas_core::intercept::{MatchCondition, MockResponse, MockRule};
use madhyamas_core::storage::body_storage::{
    classify_body, compress_body, decompress_body, BodyStorageType, INLINE_THRESHOLD,
};
use madhyamas_core::storage::{
    InterceptStoreBackend, PostgresTrafficStore, SqliteInterceptStore, TrafficStoreBackend,
};
use madhyamas_core::traffic::{
    HttpMethod, RequestData, ResponseData, TrafficCursor, TrafficEntry, TrafficFilter,
};

// ── body storage ─────────────────────────────────────────────────────────

#[test]
fn test_compress_decompress_roundtrip() {
    let body = b"hello world ".repeat(100);
    let (compressed, was_compressed) = compress_body(&body);
    assert!(was_compressed);
    assert!(compressed.len() < body.len());
    let decompressed = decompress_body(&compressed, was_compressed).unwrap();
    assert_eq!(decompressed, body);
}

#[test]
fn test_compress_small_body_not_compressed() {
    let body = b"hi";
    let (result, was_compressed) = compress_body(body);
    assert!(!was_compressed);
    assert_eq!(result, body);
}

#[test]
fn test_decompress_uncompressed() {
    let body = b"plain bytes";
    let result = decompress_body(body, false).unwrap();
    assert_eq!(result, body);
}

#[test]
fn test_classify_body() {
    assert_eq!(classify_body(None), None);
    assert_eq!(classify_body(Some(b"")), None);
    assert_eq!(classify_body(Some(b"small")), Some(BodyStorageType::Inline));
    let large = vec![0u8; INLINE_THRESHOLD + 1];
    assert_eq!(classify_body(Some(&large)), Some(BodyStorageType::Toast));
}

#[test]
fn test_storage_type_roundtrip() {
    for t in [
        BodyStorageType::Inline,
        BodyStorageType::Toast,
        BodyStorageType::S3,
    ] {
        assert_eq!(BodyStorageType::parse_str(t.as_str()), t);
    }
}

// ── SQLite intercept store ───────────────────────────────────────────────

/// Single-connection in-memory pool (shared in-memory DB across the
/// pool's single connection).
async fn memory_pool() -> sqlx::SqlitePool {
    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite")
}

fn sample_rule() -> MockRule {
    MockRule::new(
        "legacy".to_string(),
        MatchCondition::UrlPattern {
            pattern: "example.com/api".to_string(),
        },
        MockResponse {
            status_code: 200,
            ..MockResponse::default()
        },
    )
}

/// Create an `mock_rules` table in the old (11-column, pre-`description`)
/// shape and seed one rule, simulating a database created by a previous
/// Madhyamas version.
async fn create_old_schema_with_rule(pool: &sqlx::SqlitePool) {
    let condition = serde_json::to_string(&sample_rule().condition).unwrap();
    let response_config = serde_json::to_string(&sample_rule().response_config).unwrap();
    sqlx::query(
        "CREATE TABLE mock_rules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            condition TEXT NOT NULL,
            response_config TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 100,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            hit_count INTEGER NOT NULL DEFAULT 0
        )",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mock_rules \
         (id, name, condition, response_config, enabled, priority, created_at, updated_at, hit_count) \
         VALUES ('rule-1', 'legacy', ?, ?, 1, 100, 1700000000, 1700000000, 3)",
    )
    .bind(&condition)
    .bind(&response_config)
    .execute(pool)
    .await
    .unwrap();
}

/// Regression test for the old-schema → migrate → load path
/// (ShristiLabs/madhyamas#89): a database created with the pre-`description`
/// 11-column `mock_rules` schema must be migrated on store init and load
/// its persisted rules without the "no such column: description" error.

#[tokio::test]
async fn migrates_old_mock_rules_schema_and_loads_rules() {
    let pool = memory_pool().await;
    create_old_schema_with_rule(&pool).await;

    // Store init must migrate missing columns instead of failing later
    // SELECTs (previously: "no such column: description").
    let store = SqliteInterceptStore::new(pool).await.unwrap();
    let rules = store.load_mock_rules().await.unwrap();

    assert_eq!(rules.len(), 1);
    let rule = &rules[0];
    assert_eq!(rule.id, "rule-1");
    assert_eq!(rule.name, "legacy");
    // Migrated columns get their documented defaults.
    assert_eq!(rule.description, None);
    assert!(rule.tags.is_empty());
    assert_eq!(rule.collection_id, None);
    // Pre-existing data is preserved.
    assert_eq!(rule.hit_count, 3);
    assert_eq!(rule.priority, 100);

    // The new columns physically exist with the right defaults.
    let (description, tags, collection_id): (String, String, String) = sqlx::query_as(
        "SELECT description, tags, collection_id FROM mock_rules WHERE id = 'rule-1'",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert_eq!(description, "");
    assert_eq!(tags, "[]");
    assert_eq!(collection_id, "");
}

/// Fresh databases are unaffected: the 12-column schema is created and
/// the migration is a no-op.
#[tokio::test]
async fn fresh_database_roundtrips_mock_rules() {
    let store = SqliteInterceptStore::new(memory_pool().await)
        .await
        .unwrap();

    let mut rule = sample_rule();
    rule.description = Some("docs".to_string());
    rule.tags = vec!["api".to_string()];
    rule.collection_id = Some("coll-1".to_string());
    store.save_mock_rule(&rule).await.unwrap();

    let loaded = store.load_mock_rules().await.unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, rule.id);
    assert_eq!(loaded[0].description.as_deref(), Some("docs"));
    assert_eq!(loaded[0].tags, vec!["api".to_string()]);
    assert_eq!(loaded[0].collection_id.as_deref(), Some("coll-1"));
}

/// Running store init twice (restart against a migrated DB) is a no-op
/// and rules keep loading.
#[tokio::test]
async fn migration_is_idempotent_across_restarts() {
    let pool = memory_pool().await;
    create_old_schema_with_rule(&pool).await;

    let store = SqliteInterceptStore::new(pool.clone()).await.unwrap();
    assert_eq!(store.load_mock_rules().await.unwrap().len(), 1);
    // Second init against the migrated database must be a no-op.
    let store = SqliteInterceptStore::new(pool).await.unwrap();
    assert_eq!(store.load_mock_rules().await.unwrap().len(), 1);
}

// ── PostgreSQL traffic store (live-database tests) ───────────────────────

/// Helper: connect to the test PostgreSQL instance and return a fresh
/// store. The database URL is read from `MADHYAMAS_PG_TEST_URL` (default:
/// `postgres://madhyamas:testpass@localhost:5432/madhyamas`). All tests
/// are `#[ignore]` so they only run with `cargo test -- --ignored` and a
/// running PostgreSQL instance.
async fn pg_store() -> Arc<PostgresTrafficStore> {
    let url = std::env::var("MADHYAMAS_PG_TEST_URL")
        .unwrap_or_else(|_| "postgres://madhyamas:testpass@localhost:5432/madhyamas".to_string());
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
        method: HttpMethod::Get,
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
    let store = pg_store().await;
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
    let store = pg_store().await;
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
    let store = pg_store().await;
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
    let store = pg_store().await;
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
    let store = pg_store().await;
    let session = store
        .create_session(Some("test-tiered-body"))
        .await
        .unwrap();
    store.switch_session(&session.id).await.unwrap();

    // Create an entry with a large body (> 4KB) that should be tiered.
    let large_body = vec![b'A'; 8 * 1024]; // 8KB
    let req = RequestData {
        method: HttpMethod::Post,
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
    let store = pg_store().await;
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
    let store = pg_store().await;
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
    let cursor = TrafficCursor::from_entry(page1.last().unwrap());

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
    let store = pg_store().await;
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
    let store = pg_store().await;
    // flush() should not error even with no pending writes.
    store.flush().await.unwrap();
}

/// Issue #103: `client_addr` round-trips through the PostgreSQL store
/// (INSERT ... ON CONFLICT + SELECT mapping). `#[ignore]`-gated like the
/// other live-database tests (requires DATABASE_URL).
#[tokio::test]
#[ignore]
async fn test_pg_client_addr_roundtrip() {
    let store = pg_store().await;
    let session = store
        .create_session(Some("client-addr-session"))
        .await
        .unwrap();

    // Entry without attribution stores NULL.
    let plain = make_entry(&session.id);
    store.store_request(&plain).await.unwrap();
    let fetched_plain = store.get_by_id(&plain.id).await.unwrap().unwrap();
    assert_eq!(fetched_plain.client_addr, None);

    // Attributed entry round-trips.
    let mut attributed = make_entry(&session.id);
    attributed.client_addr = Some("203.0.113.9:51000".to_string());
    store.store_request(&attributed).await.unwrap();
    let fetched = store.get_by_id(&attributed.id).await.unwrap().unwrap();
    assert_eq!(fetched.client_addr.as_deref(), Some("203.0.113.9:51000"));
}

/// Issue #105: `device_id` round-trips through the PostgreSQL store, the
/// device filter scopes `get_traffic` across sessions, and the per-device
/// session is auto-created with the deterministic id. `#[ignore]`-gated
/// like the other live-database tests (requires MADHYAMAS_PG_TEST_URL).
#[tokio::test]
#[ignore]
async fn test_pg_device_attribution_and_filter() {
    use madhyamas_core::traffic::device_session_id;

    let store = pg_store().await;

    let device_id = format!(
        "dev-pg-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );
    let device_session = store
        .session_for_device(Some(&device_id), Some("PG Phone"))
        .await;
    assert_eq!(device_session, device_session_id(&device_id));

    // Device-attributed entry in the device session.
    let mut attributed = make_entry(&device_session);
    attributed.device_id = Some(device_id.clone());
    store.store_request(&attributed).await.unwrap();

    // Round-trip + filter scope.
    let fetched = store.get_by_id(&attributed.id).await.unwrap().unwrap();
    assert_eq!(fetched.device_id.as_deref(), Some(device_id.as_str()));
    assert_eq!(fetched.session_id, device_session);

    let filtered = store
        .get_traffic(&TrafficFilter {
            device_id: Some(device_id.clone()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, attributed.id);

    // The auto-created session row is named after the device record.
    let sessions = store.list_sessions().await.unwrap();
    let row = sessions
        .iter()
        .find(|s| s.id == device_session)
        .expect("device session row exists");
    assert_eq!(row.name.as_deref(), Some("Device: PG Phone"));
}

// ── Device-scoped rules (issue #109) ──────────────────────────────────────

/// Every rule type round-trips its device scope through the SQLite
/// intercept store (`None` = global, `Some(id)` = device-scoped).
#[tokio::test]
async fn intercept_rules_roundtrip_device_scope() {
    use madhyamas_core::intercept::{
        BlockListEntry, BreakpointRule, InterceptDirection, RewriteAction, RewriteDirection,
        RewriteRule, ThrottleProfile,
    };

    let store = SqliteInterceptStore::new(memory_pool().await)
        .await
        .unwrap();

    // Mocks: one global, one scoped.
    let mut scoped_mock = sample_rule();
    scoped_mock.device_id = Some("device-x".to_string());
    store.save_mock_rule(&scoped_mock).await.unwrap();
    store.save_mock_rule(&sample_rule()).await.unwrap();
    let mocks = store.load_mock_rules().await.unwrap();
    assert_eq!(mocks.len(), 2);
    let scoped = mocks
        .iter()
        .find(|r| r.device_id.as_deref() == Some("device-x"))
        .expect("scoped mock must load with its device_id");
    assert_eq!(scoped.id, scoped_mock.id);
    assert!(
        mocks.iter().filter(|r| r.device_id.is_none()).count() >= 1,
        "global mocks must load with device_id = None"
    );

    // Rewrites.
    let mut rewrite = RewriteRule::new(
        "scoped rewrite".to_string(),
        MatchCondition::All,
        RewriteDirection::Request,
        vec![RewriteAction::SetHeader {
            name: "X-A".to_string(),
            value: "b".to_string(),
        }],
    );
    rewrite.device_id = Some("device-y".to_string());
    store.save_rewrite_rule(&rewrite).await.unwrap();
    let rewrites = store.load_rewrite_rules().await.unwrap();
    assert_eq!(rewrites.len(), 1);
    assert_eq!(rewrites[0].device_id.as_deref(), Some("device-y"));

    // Breakpoints.
    let mut bp = BreakpointRule::new(
        "scoped bp".to_string(),
        MatchCondition::All,
        InterceptDirection::Request,
    );
    bp.device_id = Some("device-x".to_string());
    store.save_breakpoint_rule(&bp).await.unwrap();
    let bps = store.load_breakpoint_rules().await.unwrap();
    assert_eq!(bps.len(), 1);
    assert_eq!(bps[0].device_id.as_deref(), Some("device-x"));

    // Throttle singleton carries its scope.
    let mut profile = ThrottleProfile {
        name: "3G".to_string(),
        ..ThrottleProfile::three_g()
    };
    profile.device_id = Some("device-x".to_string());
    store.save_throttle_profile(&profile, true).await.unwrap();
    let (loaded_profile, enabled) = store.load_throttle_profile().await.unwrap().unwrap();
    assert_eq!(loaded_profile.device_id.as_deref(), Some("device-x"));
    assert!(enabled);

    // Block list entries.
    let mut entry = BlockListEntry::new("ads.example.com".to_string());
    entry.device_id = Some("device-y".to_string());
    store.save_block_list_entry(&entry).await.unwrap();
    let entries = store.load_block_list_entries().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].device_id.as_deref(), Some("device-y"));

    // The export bundle carries scopes too (persistence export path).
    let bundle = store.export_all().await.unwrap();
    assert!(bundle.contains("device-x"), "export must carry the scope");
}

/// Build all five intercept tables in their pre-#109 shape (no
/// `device_id` column) and seed one row each, simulating a database
/// created before device-scoped rules existed.
async fn create_pre_device_scope_schema(pool: &sqlx::SqlitePool) {
    let condition = serde_json::to_string(&sample_rule().condition).unwrap();
    let response_config = serde_json::to_string(&sample_rule().response_config).unwrap();
    sqlx::query(
        "CREATE TABLE mock_rules (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, condition TEXT NOT NULL,
            response_config TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 100, created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL, hit_count INTEGER NOT NULL DEFAULT 0)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO mock_rules (id, name, condition, response_config, enabled, priority, created_at, updated_at, hit_count) \
         VALUES ('m-legacy', 'legacy mock', ?, ?, 1, 100, 1700000000, 1700000000, 0)",
    )
    .bind(&condition)
    .bind(&response_config)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE rewrite_rules (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, condition TEXT NOT NULL,
            direction TEXT NOT NULL, rewrites TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1, priority INTEGER NOT NULL DEFAULT 100,
            created_at INTEGER NOT NULL, hit_count INTEGER NOT NULL DEFAULT 0)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO rewrite_rules (id, name, condition, direction, rewrites, enabled, priority, created_at, hit_count) \
         VALUES ('r-legacy', 'legacy rewrite', ?, '\"request\"', '[]', 1, 100, 1700000000, 0)",
    )
    .bind(&condition)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE breakpoint_rules (
            id TEXT PRIMARY KEY, name TEXT NOT NULL, condition TEXT NOT NULL,
            direction TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 100)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO breakpoint_rules (id, name, condition, direction, enabled, priority) \
         VALUES ('b-legacy', 'legacy bp', ?, '\"request\"', 1, 100)",
    )
    .bind(&condition)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE throttle_profile (
            id INTEGER PRIMARY KEY CHECK (id = 1), name TEXT NOT NULL,
            download_bps INTEGER NOT NULL, upload_bps INTEGER NOT NULL,
            latency_ms INTEGER NOT NULL, jitter_ms INTEGER NOT NULL,
            packet_loss_percent INTEGER NOT NULL, enabled INTEGER NOT NULL DEFAULT 0)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO throttle_profile (id, name, download_bps, upload_bps, latency_ms, jitter_ms, packet_loss_percent, enabled) \
         VALUES (1, '3G', 1000000, 500000, 100, 20, 0, 1)",
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "CREATE TABLE block_list_entries (
            id TEXT PRIMARY KEY, pattern TEXT NOT NULL, note TEXT NOT NULL DEFAULT '',
            enabled INTEGER NOT NULL DEFAULT 1, hit_count INTEGER NOT NULL DEFAULT 0,
            status_code INTEGER NOT NULL DEFAULT 403,
            response_body TEXT NOT NULL DEFAULT 'Blocked by Madhyamas',
            content_type TEXT NOT NULL DEFAULT 'text/plain',
            created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL)",
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO block_list_entries (id, pattern, note, enabled, hit_count, status_code, response_body, content_type, created_at, updated_at) \
         VALUES ('bl-legacy', 'ads.example.com', '', 1, 0, 403, 'Blocked by Madhyamas', 'text/plain', 1700000000, 1700000000)",
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Issue #109 migration: a database created before device-scoped rules
/// gains the `device_id` column on every intercept table at store init,
/// and every pre-existing row loads as a global rule (`device_id` =
/// `None`) — behavior identical to before the upgrade.
#[tokio::test]
async fn migrates_pre_device_scope_schema_rows_read_as_global() {
    let pool = memory_pool().await;
    create_pre_device_scope_schema(&pool).await;

    // Store init migrates all five tables instead of failing SELECTs.
    let store = SqliteInterceptStore::new(pool).await.unwrap();

    let mocks = store.load_mock_rules().await.unwrap();
    assert_eq!(mocks.len(), 1);
    assert_eq!(mocks[0].device_id, None, "legacy mock reads as global");
    assert_eq!(mocks[0].id, "m-legacy");

    let rewrites = store.load_rewrite_rules().await.unwrap();
    assert_eq!(rewrites.len(), 1);
    assert_eq!(
        rewrites[0].device_id, None,
        "legacy rewrite reads as global"
    );

    let bps = store.load_breakpoint_rules().await.unwrap();
    assert_eq!(bps.len(), 1);
    assert_eq!(bps[0].device_id, None, "legacy breakpoint reads as global");

    let (profile, enabled) = store
        .load_throttle_profile()
        .await
        .unwrap()
        .expect("legacy throttle row must load");
    assert_eq!(profile.device_id, None, "legacy throttle reads as global");
    assert!(enabled);

    let entries = store.load_block_list_entries().await.unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].device_id, None,
        "legacy block entry reads as global"
    );

    // The columns physically exist: a scoped write after migration works.
    let mut scoped = sample_rule();
    scoped.device_id = Some("device-post-migration".to_string());
    store.save_mock_rule(&scoped).await.unwrap();
    let mocks = store.load_mock_rules().await.unwrap();
    assert_eq!(mocks.len(), 2);
    assert!(
        mocks
            .iter()
            .any(|r| r.device_id.as_deref() == Some("device-post-migration")),
        "scoped rule written after migration must persist its scope"
    );
}
