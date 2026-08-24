//! Integration tests for the public traffic-store API: HAR import/export,
//! recording limits, capture toggles, ignored domains, focus hosts, host
//! pattern matching, session counters, cursor pagination, and lazy body
//! loading.

use std::collections::HashMap;

use base64::Engine;
use madhyamas_core::traffic::host_matches_pattern;
use madhyamas_core::traffic::{
    HttpMethod, RequestData, ResponseData, TrafficCursor, TrafficEntry, TrafficFilter, TrafficStore,
};
use madhyamas_test_utils::{in_memory_traffic_store, tmpdir};
use serde_json::json;

fn make_entry(session_id: &str, host: &str, path: &str, body: Option<Vec<u8>>) -> TrafficEntry {
    let request = RequestData {
        method: HttpMethod::Get,
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

/// Helper: create a simple traffic entry for pagination/counter tests.
fn make_simple_entry(session_id: &str, id: &str) -> TrafficEntry {
    let req = RequestData {
        method: HttpMethod::Get,
        url: format!("https://example.com/{id}"),
        host: "example.com".to_string(),
        path: format!("/{id}"),
        headers: HashMap::new(),
        body: Some(b"test body".to_vec()),
        content_type: Some("text/plain".to_string()),
        http_version: Some("HTTP/1.1".to_string()),
    };
    let mut entry = TrafficEntry::new(session_id, req);
    entry.id = id.to_string();
    entry
}

// ── HAR import ───────────────────────────────────────────────────────────

#[tokio::test]
async fn test_import_har_two_entries() {
    let store = in_memory_traffic_store().await;
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

    let result = store
        .import_har(&har, None)
        .await
        .expect("import should succeed");
    assert_eq!(result.imported_count, 2);
    assert_eq!(result.skipped_count, 0);
    assert!(result.errors.is_empty());

    let entries = store
        .get_traffic_by_session(&result.session_id)
        .await
        .expect("fetch entries");
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn test_import_har_missing_response() {
    let store = in_memory_traffic_store().await;
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

    let result = store
        .import_har(&har, None)
        .await
        .expect("import should succeed");
    assert_eq!(result.imported_count, 1);
    assert_eq!(result.skipped_count, 0);

    let entries = store
        .get_traffic_by_session(&result.session_id)
        .await
        .expect("fetch entries");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].response.is_none());
}

#[tokio::test]
async fn test_import_har_base64_body() {
    let store = in_memory_traffic_store().await;
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

    let result = store
        .import_har(&har, None)
        .await
        .expect("import should succeed");
    assert_eq!(result.imported_count, 1);

    let entries = store
        .get_traffic_by_session(&result.session_id)
        .await
        .expect("fetch entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].request.body.as_deref(), Some(b"Hello" as &[u8]));
    assert_eq!(
        entries[0].response.as_ref().unwrap().body.as_deref(),
        Some(b"Hello" as &[u8])
    );
}

#[tokio::test]
async fn test_import_har_invalid_missing_log() {
    let store = in_memory_traffic_store().await;
    let har = json!({ "foo": "bar" });

    let result = store.import_har(&har, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_import_har_entry_missing_request_skipped() {
    let store = in_memory_traffic_store().await;
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

    let result = store
        .import_har(&har, None)
        .await
        .expect("import should succeed");
    assert_eq!(result.imported_count, 1);
    assert_eq!(result.skipped_count, 1);
    assert_eq!(result.errors.len(), 1);
}

#[tokio::test]
async fn test_import_har_round_trip() {
    let store = in_memory_traffic_store().await;

    // Create a session with a couple of entries via the store.
    let session = store
        .create_session(Some("Round Trip"))
        .await
        .expect("create session");
    store
        .switch_session(&session.id)
        .await
        .expect("switch session");

    let req1 = RequestData {
        method: HttpMethod::Get,
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
    store.store_request(&entry1).await.expect("store req1");
    store
        .store_response(&entry1.id, entry1.response.as_ref().unwrap())
        .await
        .expect("store resp1");

    // Export to HAR, then import it back.
    let har = store.export_har(&session.id).await.expect("export har");
    let result = store
        .import_har(&har, Some("Imported Round Trip"))
        .await
        .expect("import har");

    assert_eq!(result.imported_count, 1);
    assert_eq!(result.skipped_count, 0);

    let imported = store
        .get_traffic_by_session(&result.session_id)
        .await
        .expect("fetch imported");
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].request.method, HttpMethod::Get);
    assert_eq!(imported[0].request.url, "https://example.com/api/1");
    assert_eq!(imported[0].response.as_ref().unwrap().status_code, 200);
}

// ── Recording limits ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_max_entries_prunes_oldest() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_max_entries(5);

    for i in 0..10 {
        let entry = make_entry(&session_id, "example.com", &format!("/p{i}"), None);
        store.store_request(&entry).await.expect("store request");
        // Small sleep to ensure distinct timestamps
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }

    let count = store.get_entry_count().await.expect("entry count");
    assert_eq!(count, 5, "should have pruned to 5 entries");

    // The 5 most recent should remain (paths /p5 through /p9)
    let filter = TrafficFilter::default();
    let entries = store.get_traffic(&filter).await.expect("get traffic");
    let paths: Vec<&str> = entries.iter().map(|e| e.request.path.as_str()).collect();
    for i in 5..10 {
        let p = format!("/p{i}");
        assert!(paths.contains(&p.as_str()), "path {p} should remain");
    }
}

#[tokio::test]
async fn test_pruned_responses_are_deleted() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_max_entries(3);

    // Insert 5 entries, each with a response.
    for i in 0..5 {
        let mut entry = make_entry(&session_id, "example.com", &format!("/r{i}"), None);
        store.store_request(&entry).await.expect("store request");
        entry.response = Some(make_response(Some(format!("resp{i}").into_bytes())));
        store
            .store_response(&entry.id, entry.response.as_ref().unwrap())
            .await
            .expect("store response");
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }

    // Only 3 entries should remain
    assert_eq!(store.get_entry_count().await.unwrap(), 3);

    // Verify no orphaned responses: count responses that belong to
    // remaining requests
    let orphaned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM responses WHERE request_id NOT IN (SELECT id FROM requests)",
    )
    .fetch_one(store.pool())
    .await
    .unwrap_or(0);
    assert_eq!(orphaned, 0, "no orphaned responses should remain");
}

#[tokio::test]
async fn test_capture_request_bodies_disabled() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_capture_request_bodies(false);

    let entry = make_entry(
        &session_id,
        "example.com",
        "/api",
        Some(b"request body".to_vec()),
    );
    store.store_request(&entry).await.expect("store request");

    let stored = store
        .get_by_id(&entry.id)
        .await
        .expect("get entry")
        .expect("entry exists");
    assert!(
        stored.request.body.is_none(),
        "request body should not be stored when capture_request_bodies is false"
    );
}

#[tokio::test]
async fn test_capture_response_bodies_disabled() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_capture_response_bodies(false);

    let entry = make_entry(
        &session_id,
        "example.com",
        "/api",
        Some(b"request body".to_vec()),
    );
    store.store_request(&entry).await.expect("store request");
    let resp = make_response(Some(b"response body".to_vec()));
    store
        .store_response(&entry.id, &resp)
        .await
        .expect("store response");

    let stored = store
        .get_by_id(&entry.id)
        .await
        .expect("get entry")
        .expect("entry exists");
    let stored_resp = stored.response.expect("response exists");
    assert!(
        stored_resp.body.is_none(),
        "response body should not be stored when capture_response_bodies is false"
    );
}

#[tokio::test]
async fn test_ignored_domains_skips_storage() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_ignored_domains(vec!["*.example.com".to_string()]);

    // Request to example.com should be skipped
    let entry1 = make_entry(&session_id, "api.example.com", "/skip", None);
    store.store_request(&entry1).await.expect("store request");

    // Request to other.com should be stored
    let entry2 = make_entry(&session_id, "other.com", "/keep", None);
    store.store_request(&entry2).await.expect("store request");

    assert_eq!(
        store.get_entry_count().await.unwrap(),
        1,
        "only non-ignored entry should be stored"
    );
}

#[tokio::test]
async fn test_ignored_domains_exact_match() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_ignored_domains(vec!["blocked.com".to_string()]);

    let entry = make_entry(&session_id, "blocked.com", "/path", None);
    store.store_request(&entry).await.expect("store request");

    assert_eq!(
        store.get_entry_count().await.unwrap(),
        0,
        "exact match should be ignored"
    );
}

#[tokio::test]
async fn test_ignored_domains_suffix_match() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_ignored_domains(vec!["analytics.com".to_string()]);

    let entry = make_entry(&session_id, "api.analytics.com", "/track", None);
    store.store_request(&entry).await.expect("store request");

    assert_eq!(
        store.get_entry_count().await.unwrap(),
        0,
        "suffix match should be ignored"
    );
}

#[tokio::test]
async fn test_get_capture_stats() {
    let store = in_memory_traffic_store().await;
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
        store.store_request(&entry).await.expect("store request");
    }

    let stats = store.get_capture_stats().await.expect("capture stats");
    assert_eq!(stats.entry_count, 3);
    assert_eq!(stats.max_entries, 100);
    assert!(stats.total_size_bytes > 0);
    assert_eq!(stats.max_total_size_bytes, 1024 * 1024);
    assert!(stats.capture_enabled);
    assert!(stats.capture_request_bodies);
    assert!(stats.capture_response_bodies);
    assert!(stats.ignored_domains.is_empty());
}

#[tokio::test]
async fn test_max_entries_zero_means_unlimited() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.set_max_entries(0);

    for i in 0..20 {
        let entry = make_entry(&session_id, "example.com", &format!("/u{i}"), None);
        store.store_request(&entry).await.expect("store request");
    }

    assert_eq!(
        store.get_entry_count().await.unwrap(),
        20,
        "no pruning when max_entries is 0"
    );
}

// ── Focus hosts ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_add_and_list_focus_host() {
    let store = in_memory_traffic_store().await;
    let host = store
        .add_focus_host("api.example.com")
        .await
        .expect("add focus host");
    assert_eq!(host.pattern, "api.example.com");
    assert!(!host.id.is_empty());

    let hosts = store.list_focus_hosts().await.expect("list focus hosts");
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].pattern, "api.example.com");
}

#[tokio::test]
async fn test_add_focus_host_dedup() {
    let store = in_memory_traffic_store().await;
    store
        .add_focus_host("API.Example.com")
        .await
        .expect("add 1");
    let second = store
        .add_focus_host("api.example.com")
        .await
        .expect("add 2");
    let hosts = store.list_focus_hosts().await.expect("list");
    assert_eq!(hosts.len(), 1, "duplicate pattern should be deduped");
    assert_eq!(second.pattern, hosts[0].pattern);
}

#[tokio::test]
async fn test_remove_focus_host() {
    let store = in_memory_traffic_store().await;
    let host = store.add_focus_host("example.com").await.expect("add");
    let removed = store.remove_focus_host(&host.id).await.expect("remove");
    assert!(removed);
    let hosts = store.list_focus_hosts().await.expect("list");
    assert!(hosts.is_empty());

    let removed_again = store
        .remove_focus_host(&host.id)
        .await
        .expect("remove again");
    assert!(!removed_again, "removing non-existent id returns false");
}

#[tokio::test]
async fn test_clear_focus_hosts() {
    let store = in_memory_traffic_store().await;
    store.add_focus_host("a.com").await.expect("add");
    store.add_focus_host("b.com").await.expect("add");
    store.add_focus_host("c.com").await.expect("add");
    assert_eq!(store.list_focus_hosts().await.expect("list").len(), 3);
    store.clear_focus_hosts().await.expect("clear");
    assert!(store.list_focus_hosts().await.expect("list").is_empty());
}

#[tokio::test]
async fn test_focus_host_persistence() {
    let dir = tmpdir("focus-persist");
    let db_path = dir.path().join("focus_test.db");

    let store = TrafficStore::new(&db_path).await.expect("create store");
    store
        .add_focus_host("persist.example.com")
        .await
        .expect("add");
    store.add_focus_host("*.wildcard.com").await.expect("add");
    assert_eq!(store.list_focus_hosts().await.expect("list").len(), 2);

    drop(store);

    let store2 = TrafficStore::new(&db_path).await.expect("reopen store");
    let hosts = store2.list_focus_hosts().await.expect("list");
    assert_eq!(hosts.len(), 2, "focus hosts should persist across restarts");
    let patterns: Vec<String> = hosts.iter().map(|h| h.pattern.clone()).collect();
    assert!(patterns.contains(&"persist.example.com".to_string()));
    assert!(patterns.contains(&"*.wildcard.com".to_string()));
}

// ── Host pattern matching ────────────────────────────────────────────────

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

// ── Session counter, cursor pagination, lazy body loading ────────────────

#[tokio::test]
async fn test_session_counter() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.clear_traffic().await.expect("clear");

    // Store 5 entries.
    for i in 0..5 {
        let entry = make_simple_entry(&session_id, &format!("counter-{i}"));
        store.store_request(&entry).await.expect("store");
    }

    // Counter should report 5 (O(1) lookup).
    let count = store.get_entry_count().await.expect("count");
    assert_eq!(count, 5);

    // Delete 2 and verify counter decremented.
    store
        .delete_traffic(&["counter-0".to_string(), "counter-1".to_string()])
        .await
        .expect("delete");
    let count = store.get_entry_count().await.expect("count");
    assert_eq!(count, 3);

    // Clear and verify counter reset.
    store.clear_traffic().await.expect("clear");
    let count = store.get_entry_count().await.expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_cursor_pagination() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.clear_traffic().await.expect("clear");

    // Store 10 entries with distinct timestamps.
    for i in 0..10 {
        let mut entry = make_simple_entry(&session_id, &format!("cursor-{i}"));
        entry.timestamp = chrono::Utc::now() + chrono::Duration::seconds(i);
        store.store_request(&entry).await.expect("store");
    }

    // First page: limit 3, no cursor.
    let filter = TrafficFilter {
        limit: Some(3),
        ..Default::default()
    };
    let page1 = store.get_traffic(&filter).await.expect("page1");
    assert_eq!(page1.len(), 3);

    // Get cursor from last entry of page 1.
    let cursor = TrafficCursor::from_entry(page1.last().unwrap());

    // Second page: limit 3, with cursor.
    let filter2 = TrafficFilter {
        limit: Some(3),
        cursor: Some(cursor),
        ..Default::default()
    };
    let page2 = store.get_traffic(&filter2).await.expect("page2");
    assert_eq!(page2.len(), 3);

    // Verify no overlap between pages.
    let page1_ids: std::collections::HashSet<_> = page1.iter().map(|e| &e.id).collect();
    let page2_ids: std::collections::HashSet<_> = page2.iter().map(|e| &e.id).collect();
    assert!(page1_ids.is_disjoint(&page2_ids));
}

#[tokio::test]
async fn test_lazy_body_loading() {
    let store = in_memory_traffic_store().await;
    let session_id = store.current_session_id();
    store.clear_traffic().await.expect("clear");

    let entry = make_simple_entry(&session_id, "lazy-body-test");
    store.store_request(&entry).await.expect("store");

    // With include_bodies = false, bodies should be None.
    let filter = TrafficFilter {
        include_bodies: Some(false),
        ..Default::default()
    };
    let entries = store.get_traffic(&filter).await.expect("get");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].request.body.is_none());

    // With include_bodies = true (default), bodies should be present.
    let filter2 = TrafficFilter {
        ..Default::default()
    };
    let entries2 = store.get_traffic(&filter2).await.expect("get");
    assert_eq!(entries2.len(), 1);
    assert!(entries2[0].request.body.is_some());
}
