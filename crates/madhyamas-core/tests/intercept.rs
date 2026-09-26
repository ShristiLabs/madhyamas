//! Integration tests for the public intercept API: block list and rewrite
//! rules (managers, entries, pattern matching, handler behavior, and
//! persistence).

use std::collections::HashMap;
use std::sync::Arc;

use madhyamas_core::intercept::{
    matches_pattern, BlockListEntry, BlockListManager, InterceptAction, InterceptHandler,
    MatchCondition, RewriteDirection, RewriteManager, RewriteTemplates,
};
use madhyamas_core::persistence::Persistable;
use madhyamas_core::storage::{InterceptStoreBackend, SqliteInterceptStore};
use madhyamas_core::traffic::{HttpMethod, RequestData, ResponseData};

// ============================================================================
// Block list — pattern matching
// ============================================================================

// ── Pattern matching: exact domain ──────────────────────────────

#[test]
fn exact_domain_matches_itself() {
    assert!(matches_pattern("example.com", "example.com"));
}

#[test]
fn exact_domain_matches_subdomain() {
    assert!(matches_pattern("example.com", "api.example.com"));
    assert!(matches_pattern("example.com", "www.example.com"));
    assert!(matches_pattern("example.com", "a.b.c.example.com"));
}

#[test]
fn exact_domain_does_not_match_unrelated() {
    assert!(!matches_pattern("example.com", "notexample.com"));
    assert!(!matches_pattern("example.com", "example.org"));
    assert!(!matches_pattern("example.com", "evil.com"));
}

// ── Pattern matching: leading wildcard *.domain ─────────────────

#[test]
fn leading_wildcard_matches_subdomain() {
    assert!(matches_pattern("*.example.com", "api.example.com"));
    assert!(matches_pattern("*.example.com", "www.example.com"));
    assert!(matches_pattern("*.example.com", "a.b.example.com"));
}

#[test]
fn leading_wildcard_does_not_match_bare_domain() {
    assert!(!matches_pattern("*.example.com", "example.com"));
}

#[test]
fn leading_wildcard_does_not_match_other_domain() {
    assert!(!matches_pattern("*.example.com", "api.example.org"));
}

// ── Pattern matching: general wildcards ─────────────────────────

#[test]
fn wildcard_in_tld_matches() {
    assert!(matches_pattern("ads.*", "ads.com"));
    assert!(matches_pattern("ads.*", "ads.net"));
    assert!(matches_pattern("ads.*", "ads.example.co.uk"));
}

#[test]
fn wildcard_in_tld_does_not_match_no_dot() {
    // "ads.*" requires at least one character after the dot.
    // Actually "ads.*" → regex ^ads\..*$ which requires a dot.
    assert!(!matches_pattern("ads.*", "ads"));
}

#[test]
fn substring_wildcard_matches() {
    assert!(matches_pattern("*ads*", "doubleclick.ads.com"));
    assert!(matches_pattern("*ads*", "ads.example.com"));
    assert!(matches_pattern("*ads*", "my-ads-server.com"));
}

#[test]
fn substring_wildcard_does_not_match_unrelated() {
    assert!(!matches_pattern("*ads*", "example.com"));
}

#[test]
fn wildcard_matches_everything() {
    assert!(matches_pattern("*", "example.com"));
    assert!(matches_pattern("*", "anything.org"));
}

// ── Pattern matching: case insensitivity ────────────────────────

#[test]
fn matching_is_case_insensitive() {
    assert!(matches_pattern("Example.COM", "api.example.com"));
    assert!(matches_pattern("*.Example.com", "API.EXAMPLE.COM"));
    assert!(matches_pattern("ADS.*", "ads.com"));
}

// ── Pattern matching: edge cases ────────────────────────────────

#[test]
fn trailing_dot_is_stripped() {
    assert!(matches_pattern("example.com.", "example.com"));
    assert!(matches_pattern("example.com", "example.com."));
}

#[test]
fn empty_pattern_never_matches() {
    assert!(!matches_pattern("", "example.com"));
}

#[test]
fn empty_host_never_matches() {
    assert!(!matches_pattern("example.com", ""));
}

#[test]
fn pattern_with_regex_metachars_is_escaped() {
    // A pattern with a literal dot should not be treated as a regex
    // wildcard. "example.com" should match "example.com" but not
    // "examplexcom".
    assert!(!matches_pattern("example.com", "examplexcom"));
}

// ============================================================================
// Block list — entries
// ============================================================================

#[test]
fn entry_new_has_defaults() {
    let entry = BlockListEntry::new("ads.example.com".to_string());
    assert_eq!(entry.pattern, "ads.example.com");
    assert!(entry.enabled);
    assert_eq!(entry.status_code, 403);
    assert_eq!(entry.response_body, "Blocked by Madhyamas");
    assert_eq!(entry.content_type, "text/plain");
    assert_eq!(entry.hit_count, 0);
    assert!(!entry.id.is_empty());
}

#[test]
fn entry_with_note() {
    let entry = BlockListEntry::with_note("ads.com".to_string(), "Block ads".to_string());
    assert_eq!(entry.note.as_deref(), Some("Block ads"));
}

#[test]
fn entry_serializes_and_deserializes() {
    let entry = BlockListEntry::new("example.com".to_string());
    let json = serde_json::to_string(&entry).unwrap();
    let back: BlockListEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(entry.id, back.id);
    assert_eq!(entry.pattern, back.pattern);
}

#[test]
fn entry_deserializes_with_defaults_for_missing_fields() {
    // Simulates an old config that only has id and pattern.
    let json = r#"{"id":"test-1","pattern":"ads.com"}"#;
    let entry: BlockListEntry = serde_json::from_str(json).unwrap();
    assert_eq!(entry.id, "test-1");
    assert_eq!(entry.pattern, "ads.com");
    assert!(entry.enabled); // default
    assert_eq!(entry.status_code, 403); // default
}

// ============================================================================
// Block list — manager CRUD
// ============================================================================

#[tokio::test]
async fn manager_add_and_get_entries() {
    let mgr = BlockListManager::new();
    let id1 = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;
    let id2 = mgr
        .add_entry(BlockListEntry::new("tracker.com".to_string()))
        .await;

    assert_eq!(mgr.len(), 2);
    assert!(!mgr.is_empty());

    let entries = mgr.get_entries();
    assert!(entries.iter().any(|e| e.id == id1));
    assert!(entries.iter().any(|e| e.id == id2));
}

#[tokio::test]
async fn manager_get_entry_by_id() {
    let mgr = BlockListManager::new();
    let id = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;

    assert!(mgr.get_entry(&id).is_some());
    assert!(mgr.get_entry("nonexistent").is_none());
}

#[tokio::test]
async fn manager_remove_entry() {
    let mgr = BlockListManager::new();
    let id = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;

    assert!(mgr.remove_entry(&id).await);
    assert_eq!(mgr.len(), 0);
    assert!(!mgr.remove_entry(&id).await); // already removed
}

#[tokio::test]
async fn manager_toggle_entry() {
    let mgr = BlockListManager::new();
    let id = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;

    assert!(mgr.get_entry(&id).unwrap().enabled);
    assert!(mgr.toggle_entry(&id, false).await);
    assert!(!mgr.get_entry(&id).unwrap().enabled);
    assert!(mgr.toggle_entry(&id, true).await);
    assert!(mgr.get_entry(&id).unwrap().enabled);
    assert!(!mgr.toggle_entry("nonexistent", true).await);
}

#[tokio::test]
async fn manager_update_entry() {
    let mgr = BlockListManager::new();
    let id = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;

    let mut updated = BlockListEntry::new("ads.com".to_string());
    updated.status_code = 503;
    updated.response_body = "Service Unavailable".to_string();
    assert!(mgr.update_entry(&id, updated).await);

    let entry = mgr.get_entry(&id).unwrap();
    assert_eq!(entry.status_code, 503);
    assert_eq!(entry.response_body, "Service Unavailable");
    // ID is preserved.
    assert_eq!(entry.id, id);
}

#[tokio::test]
async fn manager_update_nonexistent_returns_false() {
    let mgr = BlockListManager::new();
    let entry = BlockListEntry::new("ads.com".to_string());
    assert!(!mgr.update_entry("nonexistent", entry).await);
}

#[tokio::test]
async fn manager_clear() {
    let mgr = BlockListManager::new();
    mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;
    mgr.add_entry(BlockListEntry::new("tracker.com".to_string()))
        .await;

    mgr.clear().await;
    assert_eq!(mgr.len(), 0);
    assert!(mgr.is_empty());
}

#[tokio::test]
async fn manager_stats() {
    let mgr = BlockListManager::new();
    let id1 = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;
    mgr.add_entry(BlockListEntry::new("tracker.com".to_string()))
        .await;
    mgr.toggle_entry(&id1, false).await;

    let stats = mgr.stats();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.enabled, 1);
    assert_eq!(stats.disabled, 1);
    assert_eq!(stats.total_hits, 0);
}

// ============================================================================
// Block list — intercept handler
// ============================================================================

#[test]
fn handler_name_and_priority() {
    let mgr = BlockListManager::new();
    assert_eq!(mgr.name(), "block_list");
    assert_eq!(mgr.priority(), 5);
}

// ── Blocking behavior via on_request ────────────────────────────

fn make_request(host: &str) -> RequestData {
    RequestData {
        method: HttpMethod::Get,
        url: format!("https://{host}/api/test"),
        host: host.to_string(),
        path: "/api/test".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: None,
    }
}

#[tokio::test]
async fn on_request_blocks_matching_host() {
    let mgr = BlockListManager::new();
    mgr.add_entry(BlockListEntry::new("ads.example.com".to_string()))
        .await;

    let mut req = make_request("ads.example.com");
    let action = mgr.on_request(&mut req).await;

    match action {
        InterceptAction::Respond(resp) => {
            assert_eq!(resp.status_code, 403);
            assert_eq!(
                String::from_utf8(resp.body.unwrap()).unwrap(),
                "Blocked by Madhyamas"
            );
            assert_eq!(resp.content_type.as_deref(), Some("text/plain"));
        }
        other => panic!("Expected Respond, got {:?}", other),
    }
}

#[tokio::test]
async fn on_request_blocks_subdomain_of_pattern() {
    let mgr = BlockListManager::new();
    mgr.add_entry(BlockListEntry::new("example.com".to_string()))
        .await;

    let mut req = make_request("api.example.com");
    let action = mgr.on_request(&mut req).await;
    assert!(matches!(action, InterceptAction::Respond(_)));
}

#[tokio::test]
async fn on_request_does_not_block_non_matching() {
    let mgr = BlockListManager::new();
    mgr.add_entry(BlockListEntry::new("ads.example.com".to_string()))
        .await;

    let mut req = make_request("safe.example.org");
    let action = mgr.on_request(&mut req).await;
    assert!(matches!(action, InterceptAction::Continue));
}

#[tokio::test]
async fn on_request_does_not_block_disabled_entry() {
    let mgr = BlockListManager::new();
    let id = mgr
        .add_entry(BlockListEntry::new("ads.example.com".to_string()))
        .await;
    mgr.toggle_entry(&id, false).await;

    let mut req = make_request("ads.example.com");
    let action = mgr.on_request(&mut req).await;
    assert!(matches!(action, InterceptAction::Continue));
}

#[tokio::test]
async fn on_request_increments_hit_count() {
    let mgr = BlockListManager::new();
    let id = mgr
        .add_entry(BlockListEntry::new("ads.example.com".to_string()))
        .await;

    let mut req = make_request("ads.example.com");
    let _ = mgr.on_request(&mut req).await;
    let _ = mgr.on_request(&mut req).await;
    let _ = mgr.on_request(&mut req).await;

    assert_eq!(mgr.get_entry(&id).unwrap().hit_count, 3);
}

#[tokio::test]
async fn on_request_custom_status_and_body() {
    let mgr = BlockListManager::new();
    let mut entry = BlockListEntry::new("blocked.com".to_string());
    entry.status_code = 503;
    entry.response_body = "Service Unavailable".to_string();
    entry.content_type = "application/json".to_string();
    mgr.add_entry(entry).await;

    let mut req = make_request("blocked.com");
    let action = mgr.on_request(&mut req).await;

    match action {
        InterceptAction::Respond(resp) => {
            assert_eq!(resp.status_code, 503);
            assert_eq!(
                String::from_utf8(resp.body.unwrap()).unwrap(),
                "Service Unavailable"
            );
            assert_eq!(resp.content_type.as_deref(), Some("application/json"));
        }
        other => panic!("Expected Respond, got {:?}", other),
    }
}

#[tokio::test]
async fn on_request_adds_x_blocked_by_header() {
    let mgr = BlockListManager::new();
    mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;

    let mut req = make_request("ads.com");
    let action = mgr.on_request(&mut req).await;

    if let InterceptAction::Respond(resp) = action {
        assert!(resp.headers.contains_key("X-Blocked-By"));
        assert!(resp.headers["X-Blocked-By"].contains("ads.com"));
    } else {
        panic!("Expected Respond");
    }
}

#[tokio::test]
async fn first_matching_entry_wins() {
    let mgr = BlockListManager::new();
    let mut e1 = BlockListEntry::new("ads.com".to_string());
    e1.status_code = 403;
    mgr.add_entry(e1).await;
    let mut e2 = BlockListEntry::new("ads.com".to_string());
    e2.status_code = 503;
    mgr.add_entry(e2).await;

    let mut req = make_request("ads.com");
    let action = mgr.on_request(&mut req).await;
    if let InterceptAction::Respond(resp) = action {
        // First entry (403) should win.
        assert_eq!(resp.status_code, 403);
    } else {
        panic!("Expected Respond");
    }
}

// ============================================================================
// Block list — persistence (in-memory store)
// ============================================================================

async fn make_in_memory_store() -> Arc<dyn InterceptStoreBackend + Send + Sync> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    Arc::new(SqliteInterceptStore::new(pool).await.unwrap())
}

#[tokio::test]
async fn persistence_load_save_roundtrip() {
    let store = make_in_memory_store().await;
    let mgr = BlockListManager::new().with_store(store.clone());

    mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;
    mgr.add_entry(BlockListEntry::with_note(
        "tracker.com".to_string(),
        "Analytics tracker".to_string(),
    ))
    .await;

    // Save to store.
    mgr.save().await.unwrap();

    // Create a new manager backed by the same store and load.
    let mgr2 = BlockListManager::new().with_store(store);
    mgr2.load().await.unwrap();

    assert_eq!(mgr2.len(), 2);
    let entries = mgr2.get_entries();
    assert!(entries.iter().any(|e| e.pattern == "ads.com"));
    assert!(entries.iter().any(|e| e.pattern == "tracker.com"));
}

#[tokio::test]
async fn persistence_delete_removes_from_store() {
    let store = make_in_memory_store().await;
    let mgr = BlockListManager::new().with_store(store.clone());

    let id = mgr
        .add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;
    mgr.remove_entry(&id).await;

    // Loading from store should not find the deleted entry.
    let mgr2 = BlockListManager::new().with_store(store);
    mgr2.load().await.unwrap();
    assert_eq!(mgr2.len(), 0);
}

#[tokio::test]
async fn persistence_clear_removes_all() {
    let store = make_in_memory_store().await;
    let mgr = BlockListManager::new().with_store(store.clone());

    mgr.add_entry(BlockListEntry::new("ads.com".to_string()))
        .await;
    mgr.add_entry(BlockListEntry::new("tracker.com".to_string()))
        .await;
    mgr.save().await.unwrap();

    mgr.clear().await;

    let mgr2 = BlockListManager::new().with_store(store);
    mgr2.load().await.unwrap();
    assert_eq!(mgr2.len(), 0);
}

// ============================================================================
// Rewrite rules — templates
// ============================================================================

// ── No Caching template ──────────────────────────────────────────

#[test]
fn no_caching_template_has_expected_metadata() {
    let rule = RewriteTemplates::no_caching();
    assert_eq!(rule.name, "No Caching");
    assert_eq!(rule.direction, RewriteDirection::Both);
    assert!(rule.enabled, "template should be enabled by default");
    assert!(matches!(rule.condition, MatchCondition::All));
    // 2 request-side removes + 3 response-side removes + 3 sets = 8.
    assert_eq!(rule.rewrites.len(), 8, "No Caching should have 8 actions");
}

#[tokio::test]
async fn no_caching_template_strips_conditional_request_headers() {
    let manager = RewriteManager::new();
    manager.add_rule(RewriteTemplates::no_caching()).await;

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/page".to_string(),
        host: "example.com".to_string(),
        path: "/page".to_string(),
        headers: HashMap::from([
            (
                "If-Modified-Since".to_string(),
                "Wed, 21 Oct 2025 07:28:00 GMT".to_string(),
            ),
            ("If-None-Match".to_string(), "\"abc123\"".to_string()),
            ("Accept".to_string(), "text/html".to_string()),
        ]),
        body: None,
        content_type: None,
        http_version: None,
    };

    manager.rewrite_request(&mut request, None);

    assert!(
        !request.headers.contains_key("If-Modified-Since"),
        "If-Modified-Since should be stripped from the request"
    );
    assert!(
        !request.headers.contains_key("If-None-Match"),
        "If-None-Match should be stripped from the request"
    );
    assert_eq!(
        request.headers.get("Accept"),
        Some(&"text/html".to_string()),
        "non-cache headers should be preserved"
    );
}

#[tokio::test]
async fn no_caching_template_strips_and_sets_response_headers() {
    let manager = RewriteManager::new();
    manager.add_rule(RewriteTemplates::no_caching()).await;

    let request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/page".to_string(),
        host: "example.com".to_string(),
        path: "/page".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: None,
    };

    let mut response = ResponseData {
        status_code: 200,
        status_message: Some("OK".to_string()),
        headers: HashMap::from([
            ("ETag".to_string(), "\"abc123\"".to_string()),
            (
                "Last-Modified".to_string(),
                "Wed, 21 Oct 2025 07:28:00 GMT".to_string(),
            ),
            (
                "Expires".to_string(),
                "Thu, 21 Oct 2026 07:28:00 GMT".to_string(),
            ),
            ("Content-Type".to_string(), "text/html".to_string()),
        ]),
        body: None,
        content_type: Some("text/html".to_string()),
        duration_ms: 0,
        http_version: None,
    };

    manager.rewrite_response(&request, &mut response, None);

    assert!(
        !response.headers.contains_key("ETag"),
        "ETag should be stripped from the response"
    );
    assert!(
        !response.headers.contains_key("Last-Modified"),
        "Last-Modified should be stripped from the response"
    );
    assert!(
        !response.headers.contains_key("Expires")
            || response.headers.get("Expires") == Some(&"0".to_string()),
        "Expires should be replaced with 0"
    );
    assert_eq!(
        response.headers.get("Cache-Control"),
        Some(&"no-cache, no-store, must-revalidate".to_string()),
        "Cache-Control no-cache directive should be set"
    );
    assert_eq!(
        response.headers.get("Pragma"),
        Some(&"no-cache".to_string()),
        "Pragma no-cache should be set"
    );
    assert_eq!(
        response.headers.get("Expires"),
        Some(&"0".to_string()),
        "Expires should be set to 0"
    );
    assert_eq!(
        response.headers.get("Content-Type"),
        Some(&"text/html".to_string()),
        "non-cache headers should be preserved"
    );
}

#[tokio::test]
async fn no_caching_template_can_be_disabled() {
    let manager = RewriteManager::new();
    let id = manager.add_rule(RewriteTemplates::no_caching()).await;
    assert!(manager.toggle_rule(&id, false), "toggle should succeed");

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/page".to_string(),
        host: "example.com".to_string(),
        path: "/page".to_string(),
        headers: HashMap::from([("If-None-Match".to_string(), "\"abc123\"".to_string())]),
        body: None,
        content_type: None,
        http_version: None,
    };

    manager.rewrite_request(&mut request, None);

    assert_eq!(
        request.headers.get("If-None-Match"),
        Some(&"\"abc123\"".to_string()),
        "disabled rule should not strip headers"
    );
}

// ── Block Cookies template ──────────────────────────────────────

#[test]
fn block_cookies_template_has_expected_metadata() {
    let rule = RewriteTemplates::block_cookies();
    assert_eq!(rule.name, "Block Cookies");
    assert_eq!(rule.direction, RewriteDirection::Both);
    assert!(rule.enabled, "template should be enabled by default");
    assert!(matches!(rule.condition, MatchCondition::All));
    assert_eq!(
        rule.rewrites.len(),
        2,
        "Block Cookies should have 2 actions"
    );
}

#[tokio::test]
async fn block_cookies_template_strips_cookie_request_header() {
    let manager = RewriteManager::new();
    manager.add_rule(RewriteTemplates::block_cookies()).await;

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/dashboard".to_string(),
        host: "example.com".to_string(),
        path: "/dashboard".to_string(),
        headers: HashMap::from([
            (
                "Cookie".to_string(),
                "session=abc123; theme=dark".to_string(),
            ),
            ("Accept".to_string(), "text/html".to_string()),
        ]),
        body: None,
        content_type: None,
        http_version: None,
    };

    manager.rewrite_request(&mut request, None);

    assert!(
        !request.headers.contains_key("Cookie"),
        "Cookie header should be stripped from the request"
    );
    assert_eq!(
        request.headers.get("Accept"),
        Some(&"text/html".to_string()),
        "non-cookie headers should be preserved"
    );
}

#[tokio::test]
async fn block_cookies_template_strips_set_cookie_response_header() {
    let manager = RewriteManager::new();
    manager.add_rule(RewriteTemplates::block_cookies()).await;

    let request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/login".to_string(),
        host: "example.com".to_string(),
        path: "/login".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: None,
    };

    let mut response = ResponseData {
        status_code: 200,
        status_message: Some("OK".to_string()),
        headers: HashMap::from([
            (
                "Set-Cookie".to_string(),
                "session=xyz; Path=/; HttpOnly".to_string(),
            ),
            ("Content-Type".to_string(), "text/html".to_string()),
        ]),
        body: None,
        content_type: Some("text/html".to_string()),
        duration_ms: 0,
        http_version: None,
    };

    manager.rewrite_response(&request, &mut response, None);

    assert!(
        !response.headers.contains_key("Set-Cookie"),
        "Set-Cookie header should be stripped from the response"
    );
    assert_eq!(
        response.headers.get("Content-Type"),
        Some(&"text/html".to_string()),
        "non-cookie headers should be preserved"
    );
}

#[tokio::test]
async fn block_cookies_template_can_be_disabled() {
    let manager = RewriteManager::new();
    let id = manager.add_rule(RewriteTemplates::block_cookies()).await;
    assert!(manager.toggle_rule(&id, false), "toggle should succeed");

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/page".to_string(),
        host: "example.com".to_string(),
        path: "/page".to_string(),
        headers: HashMap::from([("Cookie".to_string(), "session=abc123".to_string())]),
        body: None,
        content_type: None,
        http_version: None,
    };

    manager.rewrite_request(&mut request, None);

    assert_eq!(
        request.headers.get("Cookie"),
        Some(&"session=abc123".to_string()),
        "disabled rule should not strip the Cookie header"
    );
}

// ── Template interaction with the manager ───────────────────────

#[tokio::test]
async fn both_templates_can_coexist() {
    let manager = RewriteManager::new();
    manager.add_rule(RewriteTemplates::no_caching()).await;
    manager.add_rule(RewriteTemplates::block_cookies()).await;

    assert_eq!(manager.get_rules().len(), 2);

    let mut request = RequestData {
        method: HttpMethod::Get,
        url: "https://example.com/page".to_string(),
        host: "example.com".to_string(),
        path: "/page".to_string(),
        headers: HashMap::from([
            ("If-None-Match".to_string(), "\"abc\"".to_string()),
            ("Cookie".to_string(), "session=xyz".to_string()),
            ("Accept".to_string(), "*/*".to_string()),
        ]),
        body: None,
        content_type: None,
        http_version: None,
    };

    manager.rewrite_request(&mut request, None);

    assert!(!request.headers.contains_key("If-None-Match"));
    assert!(!request.headers.contains_key("Cookie"));
    assert_eq!(request.headers.get("Accept"), Some(&"*/*".to_string()));
}

#[test]
fn templates_generate_unique_ids() {
    let rule_a = RewriteTemplates::no_caching();
    let rule_b = RewriteTemplates::no_caching();
    assert_ne!(
        rule_a.id, rule_b.id,
        "each template instance gets a unique id"
    );
}

// ============================================================================
// Device-scoped rules (issue #109)
// ============================================================================

fn scoped_request(url: &str) -> RequestData {
    // Derive the host from the URL so block-list host matching sees the
    // same authority the URL carries.
    let host = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("example.com")
        .to_string();
    RequestData {
        method: HttpMethod::Get,
        url: url.to_string(),
        host,
        path: "/page".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        http_version: None,
    }
}

#[tokio::test]
async fn device_scoped_mock_matches_only_bound_device() {
    use madhyamas_core::intercept::{MockManager, MockResponse, MockRule};

    let manager = MockManager::new();
    let mut scoped = MockRule::new(
        "scoped".to_string(),
        MatchCondition::UrlPattern {
            pattern: "example.com/scoped".to_string(),
        },
        MockResponse {
            status_code: 200,
            ..MockResponse::default()
        },
    );
    scoped.device_id = Some("device-x".to_string());
    let mut global = MockRule::new(
        "global".to_string(),
        MatchCondition::UrlPattern {
            pattern: "example.com/global".to_string(),
        },
        MockResponse {
            status_code: 200,
            ..MockResponse::default()
        },
    );
    global.device_id = None;
    manager.add_rule(scoped).await;
    manager.add_rule(global).await;

    let scoped_req = scoped_request("https://example.com/scoped");
    let global_req = scoped_request("https://example.com/global");

    // The bound device sees the scoped rule.
    let hit = manager
        .find_matching_mock(&scoped_req, Some("device-x"))
        .expect("device X must match its scoped mock");
    assert_eq!(hit.name, "scoped");
    // Another device, and unattributed traffic, flow through untouched.
    assert!(manager
        .find_matching_mock(&scoped_req, Some("device-y"))
        .is_none());
    assert!(manager.find_matching_mock(&scoped_req, None).is_none());

    // A global rule keeps applying to everyone (pre-#109 behavior).
    for device in [Some("device-x"), Some("device-y"), None] {
        let hit = manager
            .find_matching_mock(&global_req, device)
            .unwrap_or_else(|| panic!("global mock must match device {device:?}"));
        assert_eq!(hit.name, "global");
    }
}

#[tokio::test]
async fn device_scoped_rewrite_applies_only_to_bound_device() {
    use madhyamas_core::intercept::{RewriteAction, RewriteRule};

    let manager = RewriteManager::new();
    let mut rule = RewriteRule::new(
        "scoped header".to_string(),
        MatchCondition::All,
        RewriteDirection::Request,
        vec![RewriteAction::SetHeader {
            name: "X-Device-Scope".to_string(),
            value: "hit".to_string(),
        }],
    );
    rule.device_id = Some("device-x".to_string());
    manager.add_rule(rule).await;

    let mut for_x = scoped_request("https://example.com/anywhere");
    manager.rewrite_request(&mut for_x, Some("device-x"));
    assert_eq!(
        for_x.headers.get("X-Device-Scope").map(String::as_str),
        Some("hit"),
        "the bound device's request is rewritten"
    );

    for device in [Some("device-y"), None] {
        let mut req = scoped_request("https://example.com/anywhere");
        manager.rewrite_request(&mut req, device);
        assert!(
            !req.headers.contains_key("X-Device-Scope"),
            "device {device:?} must flow through the scoped rewrite untouched"
        );
    }
}

#[tokio::test]
async fn device_scoped_breakpoint_pauses_only_bound_device() {
    use madhyamas_core::intercept::{BreakpointManager, BreakpointRule, InterceptDirection};

    let manager = BreakpointManager::new(8);
    let mut rule = BreakpointRule::new(
        "scoped bp".to_string(),
        MatchCondition::All,
        InterceptDirection::Both,
    );
    rule.device_id = Some("device-x".to_string());
    manager.add_rule(rule).await;

    let req = scoped_request("https://example.com/paused");
    let mut response = ResponseData {
        status_code: 200,
        status_message: None,
        headers: HashMap::new(),
        body: None,
        content_type: None,
        duration_ms: 0,
        http_version: None,
    };

    // The bound device pauses...
    let hit = manager
        .check_request(&req, Some("device-x"))
        .expect("device X's request hits its scoped breakpoint");
    assert_eq!(hit.name, "scoped bp");
    manager
        .check_response(&req, &response, Some("device-x"))
        .expect("device X's response hits its scoped breakpoint");
    // ...while every other device flows through in both directions.
    for device in [Some("device-y"), None] {
        assert!(
            manager.check_request(&req, device).is_none(),
            "device {device:?} must not be paused by a foreign scoped breakpoint"
        );
        assert!(manager.check_response(&req, &response, device).is_none());
    }

    // Sanity on the response mutation so the compiler sees use.
    response.duration_ms = 0;
}

#[tokio::test]
async fn device_scoped_block_list_blocks_only_bound_device() {
    let manager = BlockListManager::new();
    let mut entry = BlockListEntry::new("blocked.example.com".to_string());
    entry.device_id = Some("device-x".to_string());
    manager.add_entry(entry).await;

    let req = scoped_request("https://blocked.example.com/thing");

    // The bound device is blocked by the scoped entry...
    let action = manager.evaluate(&req, Some("device-x")).await;
    match action {
        InterceptAction::Respond(resp) => assert_eq!(resp.status_code, 403),
        InterceptAction::Continue => panic!("device X must be blocked, got Continue"),
        InterceptAction::Abort => panic!("device X must be blocked, got Abort"),
    }
    // ...while other devices and unattributed traffic flow through.
    for device in [Some("device-y"), None] {
        let action = manager.evaluate(&req, device).await;
        assert!(
            matches!(action, InterceptAction::Continue),
            "device {device:?} must flow through a foreign scoped block entry"
        );
    }
    // The generic trait surface (no device context) also lets it pass —
    // the pipeline's dedicated branch is the only device-aware path.
    let mut trait_req = req.clone();
    let action = InterceptHandler::on_request(&manager, &mut trait_req).await;
    assert!(matches!(action, InterceptAction::Continue));

    // A global entry keeps blocking everyone.
    manager
        .add_entry(BlockListEntry::new("global.example.com".to_string()))
        .await;
    let global_req = scoped_request("https://global.example.com/thing");
    for device in [Some("device-x"), Some("device-y"), None] {
        let action = manager.evaluate(&global_req, device).await;
        assert!(
            matches!(action, InterceptAction::Respond(_)),
            "global block entry must block device {device:?}"
        );
    }
}

#[tokio::test]
async fn throttle_latency_is_gated_by_device_scope() {
    use madhyamas_core::intercept::{ThrottleManager, ThrottleProfile};
    use std::time::Duration;

    let manager = ThrottleManager::new();
    let mut profile = ThrottleProfile {
        name: "scoped slow".to_string(),
        latency_ms: 5_000,
        jitter_ms: 0,
        ..ThrottleProfile::none()
    };
    profile.device_id = Some("device-x".to_string());
    manager.set_profile(profile).await;
    manager.set_enabled(true).await;

    // The bound device sleeps (a 5 s latency cannot finish inside 50 ms).
    let sleeping = manager.apply_latency(Some("device-x"));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), sleeping)
            .await
            .is_err(),
        "the bound device must experience the configured latency"
    );

    // Every other device (and unattributed traffic) skips the sleep
    // entirely — the call returns well inside the window.
    for device in [Some("device-y"), None] {
        let call = manager.apply_latency(device);
        assert!(
            tokio::time::timeout(Duration::from_millis(50), call)
                .await
                .is_ok(),
            "device {device:?} must not be throttled by a foreign scoped profile"
        );
    }
}

#[test]
fn rule_json_without_device_id_deserializes_as_global() {
    // Pre-#109 payloads (no device_id key) keep deserializing with the
    // field defaulting to None — backward compatibility for stored rules,
    // exports, and the full-replace update bodies.
    let mock: MockRuleExisting = serde_json::from_str(
        r#"{"id":"m1","name":"n","condition":{"type":"all"},
            "response_config":{"type":"single","response":{"status_code":200}},
            "enabled":true,"priority":100,
            "created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z","hit_count":0}"#,
    )
    .expect("mock without device_id must deserialize");
    assert_eq!(mock.device_id, None);

    let rewrite: madhyamas_core::intercept::RewriteRule = serde_json::from_str(
        r#"{"id":"r1","name":"n","condition":{"type":"all"},"direction":"request",
            "rewrites":[],"enabled":true,"priority":100,
            "created_at":"2024-01-01T00:00:00Z","hit_count":0}"#,
    )
    .expect("rewrite without device_id must deserialize");
    assert_eq!(rewrite.device_id, None);

    let bp: madhyamas_core::intercept::BreakpointRule = serde_json::from_str(
        r#"{"id":"b1","name":"n","condition":{"type":"all"},"direction":"request",
            "enabled":true,"priority":100}"#,
    )
    .expect("breakpoint without device_id must deserialize");
    assert_eq!(bp.device_id, None);

    let block: BlockListEntry = serde_json::from_str(
        r#"{"id":"bl1","pattern":"example.com","enabled":true,"hit_count":0,
            "status_code":403,"response_body":"b","content_type":"text/plain"}"#,
    )
    .expect("block entry without device_id must deserialize");
    assert_eq!(block.device_id, None);

    let throttle: madhyamas_core::intercept::ThrottleProfile = serde_json::from_str(
        r#"{"name":"n","download_bps":0,"upload_bps":0,"latency_ms":0,
            "jitter_ms":0,"packet_loss_percent":0}"#,
    )
    .expect("throttle profile without device_id must deserialize");
    assert_eq!(throttle.device_id, None);

    // Explicit values survive a round trip.
    let scoped: madhyamas_core::intercept::RewriteRule = serde_json::from_str(
        r#"{"id":"r2","name":"n","condition":{"type":"all"},"direction":"request",
            "rewrites":[],"enabled":true,"priority":100,
            "created_at":"2024-01-01T00:00:00Z","hit_count":0,
            "device_id":"device-x"}"#,
    )
    .expect("rewrite with device_id must deserialize");
    assert_eq!(scoped.device_id.as_deref(), Some("device-x"));
}

/// Local alias so the serde test above reads naturally; MockRule's
/// timestamps deserialize from epoch integers in stored-rule JSON.
type MockRuleExisting = madhyamas_core::intercept::MockRule;
