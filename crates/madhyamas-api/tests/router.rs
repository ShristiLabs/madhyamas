//! Public-API integration tests for the log-config and secrets handlers,
//! migrated from the inline modules in src/handlers.rs.

use std::sync::Arc;

use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::Json;
use madhyamas_api::handlers::{
    delete_secret, get_log_status, list_secrets, set_secret, update_log_config,
    PatchDebugLogConfigRequest, PatchLogConfigRequest, SetSecretRequest,
};
use madhyamas_api::AppState;
use madhyamas_core::log_rotation::RotatingFileWriter;
use madhyamas_core::secrets::service::SecretService;
use madhyamas_core::{
    DebugLogConfig, DebugLogLevel, LogConfig, LogHandle, ProxyConfig, TrafficStore,
};
use madhyamas_test_utils::{tmpdir, MemStore};

async fn make_state() -> (AppState, tempfile::TempDir) {
    let store = TrafficStore::new(":memory:")
        .await
        .expect("in-memory store");
    let dir = tmpdir("log-tests");
    let writer = RotatingFileWriter::new(dir.path(), LogConfig::default()).unwrap();
    (
        AppState::new(store).with_log_handle(LogHandle::new(writer)),
        dir,
    )
}

async fn secrets_state() -> Arc<AppState> {
    let store = TrafficStore::new(":memory:")
        .await
        .expect("in-memory store");
    let svc = SecretService::new(Arc::new(MemStore::new())).unwrap();
    Arc::new(AppState::new(store).with_secrets(Arc::new(svc), true, vec!["authorization".into()]))
}

/// Invoke a handler and return (status, JSON body).
async fn respond(resp: Response) -> (axum::http::StatusCode, serde_json::Value) {
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn update_log_config_applies_full_debug_logging_section() {
    let (state, _dir) = make_state().await;
    let state = Arc::new(state);
    let req: PatchLogConfigRequest = serde_json::from_str(
        r#"{"debug_logging": {
            "enabled": true,
            "level": "full",
            "host_filter": ["*.example.com"],
            "redact_headers": ["X-Secret"],
            "redact_bodies": true
        }}"#,
    )
    .unwrap();

    let (status, body) = respond(
        update_log_config(State(state), Json(req))
            .await
            .into_response(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let d = &body["debug_logging"];
    assert_eq!(d["enabled"], true);
    assert_eq!(d["level"], "full");
    assert_eq!(d["host_filter"], serde_json::json!(["*.example.com"]));
    assert_eq!(d["redact_headers"], serde_json::json!(["X-Secret"]));
    assert_eq!(d["redact_bodies"], true);
}

#[tokio::test]
async fn update_log_config_partial_debug_logging_keeps_defaults() {
    let (state, _dir) = make_state().await;
    let state = Arc::new(state);
    let req: PatchLogConfigRequest =
        serde_json::from_str(r#"{"debug_logging": {"enabled": true}}"#).unwrap();

    let (status, body) = respond(
        update_log_config(State(state), Json(req))
            .await
            .into_response(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let d = &body["debug_logging"];
    // Untouched fields keep the defaults.
    assert_eq!(d["level"], "summary");
    assert_eq!(d["host_filter"], serde_json::Value::Null);
    assert_eq!(
        d["redact_headers"],
        serde_json::json!(["Authorization", "Cookie", "Set-Cookie"])
    );
    assert_eq!(d["redact_bodies"], false);
}

#[tokio::test]
async fn update_log_config_rejects_invalid_debug_level() {
    let (state, _dir) = make_state().await;
    let state = Arc::new(state);
    let req: PatchLogConfigRequest =
        serde_json::from_str(r#"{"debug_logging": {"enabled": true, "level": "verbose"}}"#)
            .unwrap();

    let (status, body) = respond(
        update_log_config(State(state), Json(req))
            .await
            .into_response(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().unwrap().contains("verbose"));
}

#[tokio::test]
async fn update_log_config_normalizes_empty_host_filter_to_null() {
    let (state, _dir) = make_state().await;
    let state = Arc::new(state);
    let req: PatchLogConfigRequest =
        serde_json::from_str(r#"{"debug_logging": {"enabled": true, "host_filter": []}}"#).unwrap();

    let (status, body) = respond(
        update_log_config(State(state), Json(req))
            .await
            .into_response(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(
        body["debug_logging"]["host_filter"],
        serde_json::Value::Null
    );
}

#[tokio::test]
async fn update_log_config_without_debug_logging_section_is_noop() {
    let (state, _dir) = make_state().await;
    let state = Arc::new(state);
    let req: PatchLogConfigRequest = serde_json::from_str(r#"{"max_files": 3}"#).unwrap();

    let (status, body) = respond(
        update_log_config(State(state), Json(req))
            .await
            .into_response(),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["debug_logging"]["enabled"], false);
}

#[tokio::test]
async fn get_log_status_includes_debug_logging_section() {
    let store = TrafficStore::new(":memory:")
        .await
        .expect("in-memory store");
    let dir = tmpdir("log-tests");
    let writer = RotatingFileWriter::new(dir.path(), LogConfig::default()).unwrap();
    let cfg = ProxyConfig {
        debug_logging: DebugLogConfig {
            enabled: true,
            level: DebugLogLevel::Headers,
            host_filter: Some(vec!["api.example.com".to_string()]),
            redact_headers: vec!["Authorization".to_string()],
            redact_bodies: true,
        },
        ..ProxyConfig::default()
    };
    let state = Arc::new(
        AppState::new(store)
            .with_log_handle(LogHandle::new(writer))
            .with_proxy_config(Arc::new(parking_lot::RwLock::new(cfg))),
    );

    let (status, body) = respond(get_log_status(State(state)).await.into_response()).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["debug_logging"]["enabled"], true);
    assert_eq!(body["debug_logging"]["level"], "headers");
    assert_eq!(
        body["debug_logging"]["host_filter"],
        serde_json::json!(["api.example.com"])
    );
    assert_eq!(body["debug_logging"]["redact_bodies"], true);
}

#[test]
fn patch_debug_log_request_deserializes_partial_payloads() {
    let req: PatchDebugLogConfigRequest = serde_json::from_str(r#"{"level": "headers"}"#).unwrap();
    assert_eq!(req.level.as_deref(), Some("headers"));
    assert_eq!(req.enabled, None);
    assert_eq!(req.host_filter, None);
    assert_eq!(req.redact_headers, None);
    assert_eq!(req.redact_bodies, None);
}

#[tokio::test]
async fn secrets_api_never_returns_plaintext_values() {
    let state = secrets_state().await;
    // Set a secret via the handler.
    let req: SetSecretRequest =
        serde_json::from_str(r#"{"value": "super-secret-plaintext"}"#).unwrap();
    let resp = set_secret(
        axum::extract::State(state.clone()),
        axum::extract::Path("api_token".to_string()),
        axum::Json(req),
    )
    .await;
    let (status, body) = respond(resp.into_response()).await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);
    assert!(
        !body.to_string().contains("super-secret-plaintext"),
        "set response must not echo the value"
    );

    // Listing returns names only.
    let resp = list_secrets(axum::extract::State(state.clone())).await;
    let (status, body) = respond(resp.into_response()).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["names"][0], "api_token");
    assert!(
        !body.to_string().contains("super-secret-plaintext"),
        "list response must never include values"
    );

    // Redaction: the state redactor must scrub the secret value and
    // configured headers from captured traffic JSON.
    let redactor = state.redactor().unwrap();
    let mut entry = serde_json::json!({
        "request": {
            "headers": { "Authorization": "Bearer super-secret-plaintext" },
            "body": "token=super-secret-plaintext"
        }
    });
    redactor.redact_json(&mut entry);
    assert!(!entry.to_string().contains("super-secret-plaintext"));
    assert_eq!(entry["request"]["headers"]["Authorization"], "[REDACTED]");
}

#[tokio::test]
async fn secrets_api_delete_and_disabled() {
    let state = secrets_state().await;
    let req: SetSecretRequest = serde_json::from_str(r#"{"value": "v"}"#).unwrap();
    let _ = set_secret(
        axum::extract::State(state.clone()),
        axum::extract::Path("s1".to_string()),
        axum::Json(req),
    )
    .await;
    let resp = delete_secret(
        axum::extract::State(state.clone()),
        axum::extract::Path("s1".to_string()),
    )
    .await;
    assert_eq!(
        resp.into_response().status(),
        axum::http::StatusCode::NO_CONTENT
    );

    // Without the secrets subsystem: 404.
    let store = TrafficStore::new(":memory:").await.unwrap();
    let plain = Arc::new(AppState::new(store));
    let resp = list_secrets(axum::extract::State(plain)).await;
    assert_eq!(
        resp.into_response().status(),
        axum::http::StatusCode::NOT_FOUND
    );
}

// ── issue #105: device_id filter param + real session list ────────────────

mod device_traffic_api {
    use super::*;
    use axum::extract::Query;
    use madhyamas_api::handlers::{get_sessions, get_traffic, TrafficQuery};
    use madhyamas_core::traffic::{HttpMethod, RequestData, TrafficEntry};

    fn none_query() -> TrafficQuery {
        TrafficQuery {
            url: None,
            method: None,
            status_code: None,
            limit: None,
            offset: None,
            search: None,
            file_type: None,
            header: None,
            cookie: None,
            is_passthrough: None,
            host: None,
            cursor: None,
            include_bodies: None,
            device_id: None,
        }
    }

    fn entry(session_id: &str, id: &str, path: &str, device_id: Option<&str>) -> TrafficEntry {
        let request = RequestData {
            method: HttpMethod::Get,
            url: format!("https://device.example{path}"),
            host: "device.example".to_string(),
            path: path.to_string(),
            headers: std::collections::HashMap::new(),
            body: None,
            content_type: None,
            http_version: Some("HTTP/1.1".to_string()),
        };
        let mut e = TrafficEntry::new(session_id, request);
        e.id = id.to_string();
        e.device_id = device_id.map(str::to_string);
        e
    }

    async fn traffic_state() -> Arc<AppState> {
        let store = TrafficStore::new(":memory:").await.unwrap();
        Arc::new(AppState::new(store))
    }

    /// `?device_id=` maps into the filter and scopes the response to that
    /// device's entries; the entries carry `device_id` for the web client.
    #[tokio::test]
    async fn get_traffic_device_id_param_scopes_response() {
        let state = traffic_state().await;

        // Global (unattributed) entry + two device entries, exactly as the
        // entry-construction points stamp them.
        let global = store_entry(&state, entry("default-session", "g1", "/global", None)).await;
        let alpha_session = state
            .traffic_store
            .session_for_device(Some("dev-alpha"), Some("Alpha Phone"))
            .await;
        let alpha = store_entry(
            &state,
            entry(&alpha_session, "a1", "/alpha", Some("dev-alpha")),
        )
        .await;
        let beta_session = state
            .traffic_store
            .session_for_device(Some("dev-beta"), Some("Beta Tablet"))
            .await;
        let _beta = store_entry(
            &state,
            entry(&beta_session, "b1", "/beta", Some("dev-beta")),
        )
        .await;
        assert_eq!(alpha_session, "device-dev-alpha");
        assert_ne!(alpha_session, beta_session);
        let _ = (global, alpha);

        let mut q = none_query();
        q.device_id = Some("dev-alpha".to_string());
        let (status, body) = respond(
            get_traffic(State(state.clone()), Query(q), None)
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(
            body.as_array().map(Vec::len),
            Some(1),
            "only the alpha entry: {body}"
        );
        assert_eq!(body[0]["id"], "a1");
        assert_eq!(body[0]["device_id"], "dev-alpha");
        assert_eq!(body[0]["session_id"], "device-dev-alpha");
    }

    /// Without the param the handler keeps its pre-#105 global current
    /// session scope (OSS behavior unchanged; device rows live elsewhere).
    #[tokio::test]
    async fn get_traffic_without_device_id_keeps_global_scope() {
        let state = traffic_state().await;
        store_entry(&state, entry("default-session", "g1", "/global", None)).await;
        let alpha_session = state
            .traffic_store
            .session_for_device(Some("dev-alpha"), None)
            .await;
        store_entry(
            &state,
            entry(&alpha_session, "a1", "/alpha", Some("dev-alpha")),
        )
        .await;

        let (status, body) = respond(
            get_traffic(State(state.clone()), Query(none_query()), None)
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body.as_array().map(Vec::len), Some(1));
        assert_eq!(body[0]["id"], "g1");
        assert_eq!(body[0]["device_id"], serde_json::Value::Null);
    }

    /// Issue #105: `get_sessions` returns the real persisted sessions —
    /// including device sessions auto-created by `session_for_device` —
    /// with the response shape the web client expects
    /// ({id, name, created_at, updated_at}), not a fabricated default.
    #[tokio::test]
    async fn get_sessions_returns_real_rows_including_device_sessions() {
        let state = traffic_state().await;
        state
            .traffic_store
            .session_for_device(Some("dev-alpha"), Some("Alpha Phone"))
            .await;
        // Persist an entry so the row is exercised through the normal path
        // (the ensure itself already upserts the session row).
        let sessions_before = state.traffic_store.list_sessions().await.unwrap();
        assert!(
            sessions_before.iter().any(|s| s.id == "device-dev-alpha"),
            "device session row exists before the call"
        );

        let (status, body) = respond(
            get_sessions(State(state.clone()), None)
                .await
                .into_response(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK);
        let rows = body.as_array().expect("sessions array");
        let alpha = rows
            .iter()
            .find(|s| s["id"] == "device-dev-alpha")
            .expect("device session listed");
        assert_eq!(alpha["name"], "Device: Alpha Phone");
        for key in ["id", "name", "created_at", "updated_at"] {
            assert!(
                alpha.get(key).is_some(),
                "session row carries `{key}` for the web client"
            );
        }
    }

    async fn store_entry(state: &Arc<AppState>, e: TrafficEntry) -> TrafficEntry {
        state.traffic_store.store_request(&e).await.unwrap();
        e
    }
}

mod api_auth_middleware {
    //! Issue #107: the optional boxed auth middleware applied to the whole
    //! `/api` nest. These tests use a STUB boxed middleware (never the
    //! enterprise one) so madhyamas-api stays enterprise-free; they verify
    //! invocation, path normalization (nest-stripped form, base-path safe),
    //! and the OSS default of no middleware.

    use std::sync::{Arc, Mutex};

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use madhyamas_api::{create_router, ApiAuthMiddleware, AppState, RateLimitConfig};
    use madhyamas_core::TrafficStore;
    use tower::ServiceExt;

    async fn make_app(base_path: &str, api_auth: Option<ApiAuthMiddleware>) -> axum::Router {
        let store = TrafficStore::new(":memory:").await.expect("store");
        create_router(
            AppState::new(store),
            RateLimitConfig::default(),
            None,
            base_path,
            api_auth,
        )
    }

    /// Stub boxed middleware recording the request path it observes, then
    /// passing through — same shape as the enterprise handoff in main.rs.
    fn recording_middleware(seen: Arc<Mutex<Vec<String>>>) -> ApiAuthMiddleware {
        Arc::new(move |request, next| {
            let seen = seen.clone();
            Box::pin(async move {
                {
                    let mut guard = seen.lock().unwrap();
                    guard.push(request.uri().path().to_string());
                }
                next.run(request).await
            })
        })
    }

    async fn status(app: &axum::Router, uri: &str) -> StatusCode {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        app.clone()
            .oneshot(request)
            .await
            .expect("request served")
            .status()
    }

    #[tokio::test]
    async fn api_auth_runs_on_api_routes_with_nest_stripped_path() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let app = make_app("/", Some(recording_middleware(seen.clone()))).await;
        assert_eq!(status(&app, "/api/traffic").await, StatusCode::OK);
        // The middleware lives inside the /api nest: it must observe the
        // /api-stripped path (the form the enterprise route-scope map
        // matches on), not the full request path.
        assert_eq!(*seen.lock().unwrap(), vec!["/traffic".to_string()]);
    }

    #[tokio::test]
    async fn api_auth_skips_top_level_health() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let app = make_router_with_auth(seen.clone()).await;
        assert_eq!(status(&app, "/health").await, StatusCode::OK);
        assert!(seen.lock().unwrap().is_empty());
    }

    async fn make_router_with_auth(seen: Arc<Mutex<Vec<String>>>) -> axum::Router {
        make_app("/", Some(recording_middleware(seen))).await
    }

    #[tokio::test]
    async fn api_auth_with_base_path_strips_base_and_api_prefixes() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let app = make_app("/madhyamas", Some(recording_middleware(seen.clone()))).await;
        assert_eq!(
            status(&app, "/madhyamas/api/traffic").await,
            StatusCode::OK,
            "base-path deployment must keep serving /api routes"
        );
        // Both the base path AND the /api nest prefix are stripped before
        // the middleware observes the path — the enterprise route-scope
        // map sees the same shape on base-path and root deployments.
        assert_eq!(*seen.lock().unwrap(), vec!["/traffic".to_string()]);
        // Top-level health under the base path also bypasses the nest.
        assert_eq!(status(&app, "/madhyamas/health").await, StatusCode::OK);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn api_auth_none_keeps_oss_behavior_unauthenticated() {
        // OSS build shape: no middleware handed over, /api serves openly.
        let app = make_app("/", None).await;
        assert_eq!(status(&app, "/api/traffic").await, StatusCode::OK);
    }
}
