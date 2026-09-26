//! Full-stack integration tests for device-scoped intercept rules
//! (issue #109): the REAL `/api` router (production `create_router` with
//! the real intercept handlers) guarded by the REAL enterprise auth
//! middleware, driven by three principal kinds — a device-derived agent
//! key, a plain user API key, and a JWT web session. Pins the two-axis
//! enforcement on the rule surface: the capability axis (#107) is
//! unchanged, and the new data axis restricts agent keys to their parent
//! device's rule namespace (shared per device — any of the device's
//! agents edits the device's scoped rules).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use madhyamas_api::{AppState, RateLimitConfig};
use madhyamas_core::TrafficStore;
use madhyamas_enterprise::auth::{generate_agent_key, hash_api_key};
use madhyamas_enterprise::middleware::auth_middleware;
use madhyamas_enterprise::store::{AgentKeyRecord, ApiKeyRecord, AuthSession, DeviceRecord};
use madhyamas_enterprise::{AuditEventType, AuditFilter, AuditLogger, AuthConfig, AuthManager};
use madhyamas_test_utils::enterprise::{seed_user, test_store};
use tower::ServiceExt;

/// Every rule-feature scope: the agent principal passes the capability
/// axis everywhere, so any 403/404 in these tests is the DATA axis.
const RULE_SCOPES: &[&str] = &[
    "mocks:read",
    "mocks:write",
    "rewrites:read",
    "rewrites:write",
    "breakpoints:read",
    "breakpoints:write",
    "blocklist:read",
    "blocklist:write",
    "throttle:read",
    "throttle:write",
];

fn strict_manager() -> AuthManager {
    AuthManager::new(AuthConfig {
        enabled: true,
        require_auth: true,
        jwt_secret: "test-secret-key-for-tests".to_string(),
        ..AuthConfig::default()
    })
}

async fn seed_device(
    store: &Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    owner: &str,
    name: &str,
) -> DeviceRecord {
    let device = DeviceRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        owner_user_id: owner.to_string(),
        install_uuid: None,
        mac_address: None,
        status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_seen: None,
    };
    store.create_device(&device).await.expect("create device");
    device
}

async fn seed_agent_key(
    store: &Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    device: &DeviceRecord,
) -> String {
    let key = generate_agent_key();
    let record = AgentKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        parent_device_id: device.id.clone(),
        owner_user_id: device.owner_user_id.clone(),
        name: "rule-scoping agent".to_string(),
        key_hash: hash_api_key(&key),
        key_prefix: key.chars().take(12).collect(),
        scopes: serde_json::to_string(RULE_SCOPES).expect("scopes json"),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        revoked_at: None,
        last_used_at: None,
    };
    store
        .create_agent_key(&record)
        .await
        .expect("persist agent key");
    key
}

async fn seed_user_key(
    store: &Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    user_id: &str,
) -> String {
    let key = madhyamas_enterprise::ApiKey::generate(user_id, "rule-scoping user key");
    let record = ApiKeyRecord {
        id: key.id.clone(),
        user_id: user_id.to_string(),
        name: key.name.clone(),
        key_hash: hash_api_key(&key.key),
        key_prefix: key.key.chars().take(12).collect(),
        scopes: serde_json::to_string(RULE_SCOPES).expect("scopes json"),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_api_key(&record)
        .await
        .expect("persist user key");
    key.key
}

async fn seed_jwt(
    manager: &AuthManager,
    store: &Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    user_id: &str,
    role: &str,
) -> String {
    let (token, _refresh, session_id, expires_at) = manager
        .generate_token_pair(user_id, role)
        .expect("mint pair");
    let now = chrono::Utc::now();
    let expires_dt = chrono::DateTime::<chrono::Utc>::from_timestamp(expires_at, 0)
        .unwrap_or(now + chrono::Duration::hours(1));
    store
        .create_session(&AuthSession {
            id: session_id,
            user_id: user_id.to_string(),
            jwt_jti: String::new(),
            created_at: now.to_rfc3339(),
            expires_at: expires_dt.to_rfc3339(),
            last_activity: now.to_rfc3339(),
            revoked: false,
        })
        .await
        .expect("persist session");
    token
}

/// The full production stack: real `/api` routes + real middleware.
struct Stack {
    app: axum::Router,
    audit: Arc<AuditLogger>,
    mock_manager: Arc<madhyamas_core::intercept::MockManager>,
    agent_key: String,
    user_key: String,
    jwt: String,
    device_x: DeviceRecord,
    device_y: DeviceRecord,
}

async fn stack() -> Stack {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let device_x = seed_device(&store, &uid, "Device X").await;
    let device_y = seed_device(&store, &uid, "Device Y").await;
    let agent_key = seed_agent_key(&store, &device_x).await;
    let user_key = seed_user_key(&store, &uid).await;

    let manager = Arc::new(strict_manager().with_store(store.clone()));
    let jwt = seed_jwt(&manager, &store, &uid, "admin").await;
    let audit = Arc::new(AuditLogger::new(512));

    let api_state = AppState::new(TrafficStore::new(":memory:").await.expect("traffic store"))
        .with_audit_sink(audit.clone());
    let mock_manager = api_state.mock_manager.clone();

    let auth = manager.clone();
    let auth_store = store.clone();
    let auth_audit = audit.clone();
    let boxed: madhyamas_api::ApiAuthMiddleware = Arc::new(move |request, next| {
        let auth = auth.clone();
        let auth_store = auth_store.clone();
        let auth_audit = auth_audit.clone();
        Box::pin(async move {
            auth_middleware(
                axum::Extension(auth),
                axum::Extension(auth_store),
                axum::Extension(auth_audit),
                request,
                next,
            )
            .await
        })
    });

    let app =
        madhyamas_api::create_router(api_state, RateLimitConfig::default(), None, "", Some(boxed));

    Stack {
        app,
        audit,
        mock_manager,
        agent_key,
        user_key,
        jwt,
        device_x,
        device_y,
    }
}

fn json_request(
    method: &str,
    uri: &str,
    credential: &str,
    body: serde_json::Value,
) -> Request<Body> {
    let (header, value) = if credential.starts_with("eyJ") {
        ("Authorization", format!("Bearer {credential}"))
    } else {
        ("X-API-Key", credential.to_string())
    };
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .header(header, value)
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.clone().oneshot(request).await.expect("send request");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap_or_default();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, body)
}

fn mock_body(name: &str) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "condition": {"type": "url_pattern", "pattern": "example.com/scoped-test"},
        "response": {"status_code": 200, "body": "mocked"}
    })
}

// ── Mocks: default scoping + rejections ──────────────────────────────

#[tokio::test]
async fn agent_mock_create_defaults_to_parent_and_rejects_global_and_foreign() {
    let s = stack().await;

    // Omitted device_id: scoped to the parent device by default.
    let (status, body) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks",
            &s.agent_key,
            mock_body("agent default"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let (status, list) = send(
        &s.app,
        json_request("GET", "/api/mocks", &s.agent_key, serde_json::Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rules = list.as_array().expect("list");
    let created = rules
        .iter()
        .find(|r| r["name"] == "agent default")
        .expect("created rule visible to its agent");
    assert_eq!(
        created["device_id"].as_str(),
        Some(s.device_x.id.as_str()),
        "omitted device_id must default to the parent device"
    );

    // Explicit null (global) is rejected — agents cannot create global rules.
    let mut global = mock_body("agent global");
    global["device_id"] = serde_json::Value::Null;
    let (status, body) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, global),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // A foreign device is rejected.
    let mut foreign = mock_body("agent foreign");
    foreign["device_id"] = serde_json::json!(s.device_y.id);
    let (status, body) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, foreign),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    // The same explicit parent device is accepted.
    let mut own = mock_body("agent explicit own");
    own["device_id"] = serde_json::json!(s.device_x.id);
    let (status, body) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, own),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}

#[tokio::test]
async fn agent_cannot_see_or_mutate_global_or_foreign_mocks() {
    let s = stack().await;

    // The owner seeds a global mock and one scoped to the OTHER device.
    let mut global = mock_body("owner global");
    global["device_id"] = serde_json::Value::Null;
    let (_, global_body) = send(&s.app, json_request("POST", "/api/mocks", &s.jwt, global)).await;
    let global_id = global_body["id"].as_str().expect("global id").to_string();

    let mut foreign = mock_body("owner foreign");
    foreign["device_id"] = serde_json::json!(s.device_y.id);
    let (_, foreign_body) = send(&s.app, json_request("POST", "/api/mocks", &s.jwt, foreign)).await;
    let foreign_id = foreign_body["id"].as_str().expect("foreign id").to_string();

    // The agent's list excludes both.
    let (status, list) = send(
        &s.app,
        json_request("GET", "/api/mocks", &s.agent_key, serde_json::Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = list
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|r| r["name"].as_str())
        .collect();
    assert!(
        !names.contains(&"owner global"),
        "global rules are hidden from agents"
    );
    assert!(
        !names.contains(&"owner foreign"),
        "other-device rules are hidden"
    );

    // Reads and mutations 404 (indistinguishable from a missing rule).
    for id in [&global_id, &foreign_id] {
        let (status, _) = send(
            &s.app,
            json_request(
                "GET",
                &format!("/api/mocks/{id}"),
                &s.agent_key,
                serde_json::Value::Null,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "agent GET of {id} must 404");

        // A valid full-rule PUT body (fetched by the owner) — still 404.
        let (status, rule_json) = send(
            &s.app,
            json_request(
                "GET",
                &format!("/api/mocks/{id}"),
                &s.jwt,
                serde_json::Value::Null,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let mut hijack = rule_json.clone();
        hijack["name"] = serde_json::json!("hijack");
        let (status, _) = send(
            &s.app,
            json_request("PUT", &format!("/api/mocks/{id}"), &s.agent_key, hijack),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "agent PUT of {id} must 404");

        let (status, _) = send(
            &s.app,
            json_request(
                "DELETE",
                &format!("/api/mocks/{id}"),
                &s.agent_key,
                serde_json::Value::Null,
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "agent DELETE of {id} must 404"
        );

        let (status, _) = send(
            &s.app,
            json_request(
                "POST",
                &format!("/api/mocks/{id}/toggle"),
                &s.agent_key,
                serde_json::json!({"enabled": false}),
            ),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "agent toggle of {id} must 404"
        );
    }

    // The owner still sees and manages everything.
    let (status, _) = send(
        &s.app,
        json_request(
            "GET",
            &format!("/api/mocks/{global_id}"),
            &s.jwt,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "owner JWT reads the global rule");
}

#[tokio::test]
async fn agent_update_keeps_rule_pinned_to_parent_device() {
    let s = stack().await;

    let (_, created) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, mock_body("pinned")),
    )
    .await;
    let id = created["id"].as_str().expect("id").to_string();

    // Full-replace update (a valid full-rule body fetched by the agent)
    // attempting to globalize the rule via an explicit null device_id:
    // the scope is forced back to the parent device.
    let (status, rule_json) = send(
        &s.app,
        json_request(
            "GET",
            &format!("/api/mocks/{id}"),
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut updated = rule_json.clone();
    updated["name"] = serde_json::json!("pinned v2");
    updated["device_id"] = serde_json::Value::Null; // attempt to globalize
    let (status, body) = send(
        &s.app,
        json_request("PUT", &format!("/api/mocks/{id}"), &s.agent_key, updated),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (_, list) = send(
        &s.app,
        json_request("GET", "/api/mocks", &s.agent_key, serde_json::Value::Null),
    )
    .await;
    let rule = list
        .as_array()
        .expect("list")
        .iter()
        .find(|r| r["id"] == id.as_str())
        .expect("rule still visible");
    assert_eq!(
        rule["device_id"].as_str(),
        Some(s.device_x.id.as_str()),
        "agent updates cannot change the device scope"
    );
}

#[tokio::test]
async fn user_key_and_jwt_are_unrestricted_on_the_device_axis() {
    let s = stack().await;

    // A plain user key (non-agent) keeps its pre-#109 powers: global rules.
    let (status, _) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks",
            &s.user_key,
            mock_body("user key global"),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "user keys create global rules");

    // The JWT owner creates both scopes explicitly.
    let mut scoped = mock_body("jwt scoped");
    scoped["device_id"] = serde_json::json!(s.device_x.id);
    let (status, _) = send(&s.app, json_request("POST", "/api/mocks", &s.jwt, scoped)).await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, list) = send(
        &s.app,
        json_request("GET", "/api/mocks", &s.jwt, serde_json::Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = list
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|r| r["name"].as_str())
        .collect();
    assert!(names.contains(&"user key global"));
    assert!(names.contains(&"jwt scoped"), "the owner sees every scope");
}

// ── Mocks: export / import / batch ───────────────────────────────────

#[tokio::test]
async fn agent_export_excludes_global_and_import_lands_in_parent_namespace() {
    let s = stack().await;

    send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks",
            &s.jwt,
            mock_body("owner secret global"),
        ),
    )
    .await;
    send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, mock_body("agent own")),
    )
    .await;

    let (status, exported) = send(
        &s.app,
        json_request(
            "GET",
            "/api/mocks/export",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = exported
        .as_array()
        .expect("export array")
        .iter()
        .filter_map(|r| r["name"].as_str())
        .collect();
    assert!(names.contains(&"agent own"));
    assert!(
        !names.contains(&"owner secret global"),
        "global rules never leave via export"
    );

    // A HAR import by the agent lands in the parent-device namespace.
    let har = serde_json::json!({
        "format": "har",
        "data": serde_json::json!({
            "log": {"entries": [{"request": {"url": "https://har.example/a", "method": "GET"},
                                  "response": {"status": 200}}]}
        })
        .to_string()
    });
    let (status, body) = send(
        &s.app,
        json_request("POST", "/api/mocks/import", &s.agent_key, har),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    assert_eq!(body["imported"], 1);

    let (_, list) = send(
        &s.app,
        json_request("GET", "/api/mocks", &s.agent_key, serde_json::Value::Null),
    )
    .await;
    let imported = list
        .as_array()
        .expect("list")
        .iter()
        .find(|r| r["name"].as_str().is_some_and(|n| n.starts_with("HAR:")))
        .expect("imported rule visible");
    assert_eq!(
        imported["device_id"].as_str(),
        Some(s.device_x.id.as_str()),
        "agent imports are scoped to the parent device"
    );
}

#[tokio::test]
async fn agent_batch_toggle_reports_foreign_ids_as_not_found() {
    let s = stack().await;

    let (_, global_body) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.jwt, mock_body("batch global")),
    )
    .await;
    let global_id = global_body["id"].as_str().expect("id").to_string();
    let (_, own_body) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, mock_body("batch own")),
    )
    .await;
    let own_id = own_body["id"].as_str().expect("id").to_string();

    let (status, body) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks/batch-toggle",
            &s.agent_key,
            serde_json::json!({"ids": [own_id, global_id, "no-such-id"], "enabled": false}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["updated"], 1, "only the agent's own rule toggles");
    let not_found = body["not_found"].as_array().expect("not_found");
    assert!(
        not_found.iter().any(|v| v == global_id.as_str()),
        "global id reports not_found"
    );
    assert!(not_found.iter().any(|v| v == "no-such-id"));
}

// ── Rewrites / breakpoints / block list: same data axis ──────────────

#[tokio::test]
async fn every_rule_type_defaults_and_rejects_for_agent_keys() {
    let s = stack().await;

    // Rewrites.
    let rewrite = serde_json::json!({
        "name": "scoped rewrite",
        "condition": {"type": "all"},
        "direction": "request",
        "rewrites": []
    });
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/rewrites", &s.agent_key, rewrite.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let mut global_rewrite = rewrite.clone();
    global_rewrite["device_id"] = serde_json::Value::Null;
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/rewrites", &s.agent_key, global_rewrite),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "agent rewrite global rejected"
    );
    let (_, rewrites) = send(
        &s.app,
        json_request(
            "GET",
            "/api/rewrites",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    let rule = rewrites.as_array().expect("list")[0].clone();
    assert_eq!(rule["device_id"].as_str(), Some(s.device_x.id.as_str()));

    // Breakpoints.
    let bp = serde_json::json!({
        "name": "scoped bp",
        "condition": {"type": "all"},
        "direction": "request"
    });
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/breakpoints", &s.agent_key, bp.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let mut foreign_bp = bp.clone();
    foreign_bp["device_id"] = serde_json::json!(s.device_y.id);
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/breakpoints", &s.agent_key, foreign_bp),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "agent foreign-device breakpoint rejected"
    );
    let (_, bps) = send(
        &s.app,
        json_request(
            "GET",
            "/api/breakpoints",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    let rule = bps.as_array().expect("list")[0].clone();
    assert_eq!(rule["device_id"].as_str(), Some(s.device_x.id.as_str()));

    // Block list.
    let block = serde_json::json!({"pattern": "ads.example.com"});
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/blocklist", &s.agent_key, block.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let mut global_block = block.clone();
    global_block["device_id"] = serde_json::Value::Null;
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/blocklist", &s.agent_key, global_block),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "agent global block entry rejected"
    );
    let (_, entries) = send(
        &s.app,
        json_request(
            "GET",
            "/api/blocklist",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    let entry = entries.as_array().expect("list")[0].clone();
    assert_eq!(entry["device_id"].as_str(), Some(s.device_x.id.as_str()));

    // The owner's global rules stay hidden from the agent's lists.
    send(
        &s.app,
        json_request(
            "POST",
            "/api/blocklist",
            &s.jwt,
            serde_json::json!({"pattern": "owner.example.com"}),
        ),
    )
    .await;
    let (_, entries) = send(
        &s.app,
        json_request(
            "GET",
            "/api/blocklist",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    let patterns: Vec<&str> = entries
        .as_array()
        .expect("list")
        .iter()
        .filter_map(|e| e["pattern"].as_str())
        .collect();
    assert!(
        !patterns.contains(&"owner.example.com"),
        "global block entries are hidden"
    );
}

// ── Throttle singleton ────────────────────────────────────────────────

#[tokio::test]
async fn throttle_singleton_semantics_for_agents() {
    let s = stack().await;
    let profile = serde_json::json!({
        "profile": {
            "name": "3G",
            "download_bps": 1_000_000,
            "upload_bps": 500_000,
            "latency_ms": 100,
            "jitter_ms": 20,
            "packet_loss_percent": 0
        },
        "enabled": true
    });

    // The agent's set is forced into its device's scope.
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/throttle", &s.agent_key, profile.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = send(
        &s.app,
        json_request(
            "GET",
            "/api/throttle",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["profile"]["device_id"].as_str(),
        Some(s.device_x.id.as_str()),
        "agent-set profile is scoped to the parent device"
    );
    assert_eq!(
        body["enabled"], true,
        "the agent sees its own profile enabled"
    );

    // The owner re-sets a GLOBAL profile (no device_id in the payload).
    let mut global_profile = profile.clone();
    global_profile["profile"]["name"] = serde_json::json!("DSL");
    let (status, _) = send(
        &s.app,
        json_request("POST", "/api/throttle", &s.jwt, global_profile),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // The agent no longer sees any profile (global hidden) and cannot
    // toggle the global one.
    let (status, body) = send(
        &s.app,
        json_request(
            "GET",
            "/api/throttle",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["profile"]["device_id"].is_null(),
        "the global profile is invisible to the agent"
    );
    assert_eq!(body["enabled"], false, "reported disabled for the agent");

    let (status, _) = send(
        &s.app,
        json_request(
            "POST",
            "/api/throttle/enabled",
            &s.agent_key,
            serde_json::json!({"enabled": false}),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "an agent cannot toggle the owner's global profile"
    );

    // The owner sees the global profile and toggles freely.
    let (_, body) = send(
        &s.app,
        json_request("GET", "/api/throttle", &s.jwt, serde_json::Value::Null),
    )
    .await;
    assert_eq!(body["profile"]["name"], "DSL");
    let (status, _) = send(
        &s.app,
        json_request(
            "POST",
            "/api/throttle/enabled",
            &s.jwt,
            serde_json::json!({"enabled": false}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

// ── Mock collections stay global with agent guards ────────────────────

#[tokio::test]
async fn agent_cannot_toggle_collections_or_delete_their_rules() {
    let s = stack().await;

    let (_, coll) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks/collections",
            &s.jwt,
            serde_json::json!({"name": "owner collection"}),
        ),
    )
    .await;
    let coll_id = coll["id"].as_str().expect("collection id").to_string();

    let (status, _) = send(
        &s.app,
        json_request(
            "POST",
            &format!("/api/mocks/collections/{coll_id}/toggle"),
            &s.agent_key,
            serde_json::json!({"enabled": false}),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "collection toggles flip cross-scope rules"
    );

    let (status, _) = send(
        &s.app,
        json_request(
            "DELETE",
            &format!("/api/mocks/collections/{coll_id}"),
            &s.agent_key,
            serde_json::json!({"delete_rules": true}),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "rule-deleting collection deletes are guarded"
    );

    // The shell deletion without rule deletion remains available.
    let (status, _) = send(
        &s.app,
        json_request(
            "DELETE",
            &format!("/api/mocks/collections/{coll_id}"),
            &s.agent_key,
            serde_json::json!({"delete_rules": false}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ── Preview hides global matches from agents ──────────────────────────

#[tokio::test]
async fn preview_reports_no_match_for_agent_when_global_rule_matches() {
    let s = stack().await;

    let mut global = mock_body("preview global");
    global["condition"] =
        serde_json::json!({"type": "url_pattern", "pattern": "preview.example.com/thing"});
    send(&s.app, json_request("POST", "/api/mocks", &s.jwt, global)).await;

    let preview_req = serde_json::json!({
        "request": {
            "method": "GET",
            "url": "https://preview.example.com/thing",
            "host": "preview.example.com",
            "path": "/thing",
            "headers": {},
            "body": null,
            "content_type": null
        }
    });

    // The agent is told "no match" — the global rule is not disclosed.
    let (status, body) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks/preview",
            &s.agent_key,
            preview_req.clone(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["matched"], false,
        "global matches are hidden from agents"
    );

    // The owner sees the match.
    let (status, body) = send(
        &s.app,
        json_request("POST", "/api/mocks/preview", &s.jwt, preview_req),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["matched"], true);
    assert_eq!(body["rule_name"], "preview global");
}

// ── Audit ─────────────────────────────────────────────────────────────

/// Wait (bounded) for an audit event of the given type to appear in the
/// in-memory logger — rule-mutation events are written fire-and-forget.
async fn await_audit_event(
    audit: &AuditLogger,
    event_type: AuditEventType,
) -> madhyamas_enterprise::AuditEvent {
    for _ in 0..400 {
        let events = audit
            .query_in_memory(&AuditFilter {
                event_type: Some(event_type),
                ..Default::default()
            })
            .into_iter()
            .max_by_key(|e| e.timestamp);
        if let Some(event) = events {
            return event;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("audit event {event_type:?} never appeared");
}

#[tokio::test]
async fn rule_mutations_are_audited_with_key_and_device_scope() {
    let s = stack().await;

    let (_, created) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.agent_key, mock_body("audited")),
    )
    .await;
    let id = created["id"].as_str().expect("id").to_string();

    let event = await_audit_event(&s.audit, AuditEventType::MockCreated).await;
    assert_eq!(
        event.metadata.get("rule_type").and_then(|v| v.as_str()),
        Some("mock")
    );
    assert_eq!(
        event.metadata.get("action").and_then(|v| v.as_str()),
        Some("create")
    );
    assert_eq!(
        event.metadata.get("device_id").and_then(|v| v.as_str()),
        Some(s.device_x.id.as_str()),
        "the rule's device scope rides on the audit event"
    );
    assert!(
        event.api_key_id.is_some(),
        "the acting agent key's record id is recorded"
    );
    // No key material anywhere in the serialized event.
    let serialized = serde_json::to_string(&event).unwrap_or_default();
    assert!(
        !serialized.contains(&s.agent_key),
        "audit must never carry key material"
    );

    send(
        &s.app,
        json_request(
            "DELETE",
            &format!("/api/mocks/{id}"),
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    let event = await_audit_event(&s.audit, AuditEventType::MockDeleted).await;
    assert_eq!(
        event.metadata.get("rule_id").and_then(|v| v.as_str()),
        Some(id.as_str())
    );
    assert_eq!(
        event.metadata.get("device_id").and_then(|v| v.as_str()),
        Some(s.device_x.id.as_str())
    );

    // Non-create/delete mutations land as Custom events with structure.
    let (_, created) = send(
        &s.app,
        json_request(
            "POST",
            "/api/rewrites",
            &s.agent_key,
            serde_json::json!({
                "name": "audited rewrite",
                "condition": {"type": "all"},
                "direction": "request",
                "rewrites": []
            }),
        ),
    )
    .await;
    let rewrite_id = created["id"].as_str().expect("id").to_string();
    send(
        &s.app,
        json_request(
            "POST",
            &format!("/api/rewrites/{rewrite_id}/toggle"),
            &s.agent_key,
            serde_json::json!({"enabled": false}),
        ),
    )
    .await;
    for _ in 0..400 {
        let customs = audit_query_custom(&s.audit);
        if customs.iter().any(|e| {
            e.metadata.get("rule_type").and_then(|v| v.as_str()) == Some("rewrite")
                && e.metadata.get("action").and_then(|v| v.as_str()) == Some("toggle")
        }) {
            return; // found
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("custom rewrite-toggle audit event never appeared");
}

fn audit_query_custom(audit: &AuditLogger) -> Vec<madhyamas_enterprise::AuditEvent> {
    audit.query_in_memory(&AuditFilter {
        event_type: Some(AuditEventType::Custom),
        ..Default::default()
    })
}

// ── OSS-parity: no RuleActor, no restriction ──────────────────────────

/// Without the enterprise middleware there is no `RuleActor` extension:
/// the same handlers behave exactly as in the OSS tier (create is
/// global by default, everything visible).
#[tokio::test]
async fn without_rule_actor_no_device_axis_restriction() {
    let api_state = AppState::new(TrafficStore::new(":memory:").await.expect("traffic store"));
    let app = madhyamas_api::create_router(
        api_state,
        RateLimitConfig::default(),
        None,
        "",
        None, // no auth middleware — OSS shape
    );

    let (status, _) = send(
        &app,
        json_request("POST", "/api/mocks", "unused", mock_body("oss rule")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, list) = send(
        &app,
        json_request("GET", "/api/mocks", "unused", serde_json::Value::Null),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rule = list.as_array().expect("list")[0].clone();
    assert!(rule["device_id"].is_null(), "OSS-created rules are global");
}

// ── Mock hit analytics: the device axis applies to activity data too ──

#[tokio::test]
async fn mock_analytics_hide_global_rule_activity_from_agents() {
    let s = stack().await;

    // A global mock (owner) and the agent's own mock.
    let (_, global_body) = send(
        &s.app,
        json_request("POST", "/api/mocks", &s.jwt, mock_body("analytics global")),
    )
    .await;
    let global_id = global_body["id"].as_str().expect("id").to_string();
    let (_, own_body) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks",
            &s.agent_key,
            mock_body("analytics own"),
        ),
    )
    .await;
    let own_id = own_body["id"].as_str().expect("id").to_string();

    // Seed hit history for both rules (the analytics hook is
    // `MockManager::record_hit`; production fires it from the mock
    // match path).
    let hit_request = |url: &str| madhyamas_core::RequestData {
        method: "GET".parse().expect("method"),
        url: url.to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        headers: Default::default(),
        body: None,
        content_type: None,
        http_version: None,
    };
    s.mock_manager.record_hit(
        &global_id,
        &hit_request("https://example.com/a"),
        200,
        1,
        None,
    );
    s.mock_manager.record_hit(
        &global_id,
        &hit_request("https://example.com/b"),
        200,
        2,
        None,
    );
    s.mock_manager
        .record_hit(&own_id, &hit_request("https://example.com/c"), 200, 1, None);

    // The agent's aggregate analytics exclude the global rule entirely —
    // neither its id nor the URLs it intercepted are disclosed.
    let (status, analytics) = send(
        &s.app,
        json_request(
            "GET",
            "/api/mocks/analytics",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let records = analytics.as_array().expect("array");
    assert!(
        records.iter().all(|r| r["mock_id"] != global_id.as_str()),
        "global rule hit records must not reach agent principals"
    );
    assert!(
        records.iter().any(|r| r["mock_id"] == own_id.as_str()),
        "the agent's own rule records are visible"
    );

    // Per-rule stats and history for the global rule 404 for the agent,
    // 200 for the owner.
    for uri in [
        format!("/api/mocks/{global_id}/analytics"),
        format!("/api/mocks/{global_id}/history"),
    ] {
        let (status, _) = send(
            &s.app,
            json_request("GET", &uri, &s.agent_key, serde_json::Value::Null),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "agent GET {uri} must 404");
        let (status, body) = send(
            &s.app,
            json_request("GET", &uri, &s.jwt, serde_json::Value::Null),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "owner GET {uri}: {body}");
    }

    // Clearing hit history across all rules is owner/JWT-only.
    let (status, _) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks/history/clear",
            &s.agent_key,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "agent cannot clear all history"
    );
    let (status, _) = send(
        &s.app,
        json_request(
            "POST",
            "/api/mocks/history/clear",
            &s.jwt,
            serde_json::Value::Null,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
