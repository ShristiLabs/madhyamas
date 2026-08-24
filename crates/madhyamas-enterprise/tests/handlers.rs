//! Integration tests for the public enterprise handler API: health-check
//! request/response flows with and without Redis.

use std::sync::Arc;

use axum::extract::State;
use axum::Extension;
use madhyamas_api::AppState;
use madhyamas_core::{TrafficStore, WsManager};
use madhyamas_enterprise::handlers::get_health_check;
use madhyamas_enterprise::{AuthConfig, AuthManager, RedisState};

async fn make_state() -> Arc<AppState> {
    // Keep the temp dir alive for the store's lifetime: dropping it would
    // unlink the SQLite file out from under the pool and flake ping-based
    // tests (seen on linux CI) when a connection is re-opened.
    let tmp = tempfile::tempdir().expect("temp dir").keep();
    let db_path = tmp.join("test.db").to_string_lossy().to_string();
    let store = TrafficStore::new(db_path).await.expect("open store");
    Arc::new(AppState::new(store).with_ws_manager(Arc::new(WsManager::new())))
}

#[tokio::test]
async fn test_health_check_without_redis() {
    let state = make_state().await;
    let auth = Arc::new(AuthManager::new(AuthConfig::default()));
    let resp = get_health_check(
        State(state.clone()),
        Extension(None),
        Extension(None),
        Extension(auth),
    )
    .await
    .0;
    let deps = resp.dependencies.expect("dependencies");
    assert_eq!(deps.redis, "not_configured");
    assert_eq!(deps.license, "not_configured");
    assert_eq!(deps.database, "ok");
    assert_eq!(resp.status, Some("ok".to_string()));
}

#[tokio::test]
#[ignore = "requires redis at redis://localhost:6379"]
async fn test_health_check_with_redis() {
    let state = make_state().await;
    let rs = Arc::new(
        RedisState::new("redis://localhost:6379", "test-health".to_string())
            .await
            .expect("connect redis"),
    );
    let auth = Arc::new(AuthManager::new(AuthConfig::default()));
    let resp = get_health_check(
        State(state.clone()),
        Extension(None),
        Extension(Some(rs)),
        Extension(auth),
    )
    .await
    .0;
    let deps = resp.dependencies.expect("dependencies");
    assert_eq!(deps.redis, "ok");
    assert_eq!(resp.status, Some("ok".to_string()));
}
