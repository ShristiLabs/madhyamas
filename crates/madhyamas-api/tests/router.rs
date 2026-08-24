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
