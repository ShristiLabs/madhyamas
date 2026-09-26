//! Public-API integration tests for the issue #107 feature-scope
//! taxonomy: route-to-scope enforcement for API keys, the JWT-only
//! exclusion list, whole-`/api` middleware coverage, legacy scope
//! reconciliation, and the `/auth/me` effective-scope report consumed by
//! the MCP server.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn;
use axum::routing::{get, post};
use axum::Router;
use madhyamas_api::{ApiAuthMiddleware, AppState};
use madhyamas_core::TrafficStore;
use madhyamas_enterprise::auth::hash_api_key;
use madhyamas_enterprise::middleware::PermissionState;
use madhyamas_enterprise::store::{ApiKeyRecord, AuthSession};
use madhyamas_enterprise::{
    create_enterprise_router, effective_scopes, hash_password, ApiKey, AuditLogger, AuthConfig,
    AuthManager, Permission, ResourceType,
};
use madhyamas_test_utils::enterprise::{seed_user, test_store};
use tower::ServiceExt;

/// Auth manager with strict auth ON — mirrors an `--enable-auth`
/// deployment (the middleware only enforces when `require_auth` is set).
fn strict_manager() -> AuthManager {
    AuthManager::new(AuthConfig {
        enabled: true,
        require_auth: true,
        jwt_secret: "test-secret-key-for-tests".to_string(),
        ..AuthConfig::default()
    })
}

/// Persist an API key with the given scope grants; returns the plaintext.
async fn seed_key(
    store: &Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    user_id: &str,
    scopes: &[&str],
) -> String {
    let api_key = ApiKey::generate(user_id, "scope-test-key");
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: user_id.to_string(),
        name: api_key.name.clone(),
        key_hash: hash_api_key(&api_key.key),
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: serde_json::to_string(scopes).unwrap(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_api_key(&record).await.expect("persist key");
    api_key.key
}

/// Mint a JWT for `user_id`/`role` and persist its auth session (the
/// middleware validates the session on every bearer request).
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

/// Boxed enterprise auth middleware — the exact wiring `main.rs` uses to
/// guard the whole `/api` nest while keeping madhyamas-api enterprise-free.
fn boxed_auth(
    auth: Arc<AuthManager>,
    store: Arc<dyn madhyamas_enterprise::EnterpriseStore>,
    audit: Arc<AuditLogger>,
) -> ApiAuthMiddleware {
    Arc::new(move |request, next| {
        let auth = auth.clone();
        let store = store.clone();
        let audit = audit.clone();
        Box::pin(madhyamas_enterprise::middleware::auth_middleware(
            axum::Extension(auth),
            axum::Extension(store),
            axum::Extension(audit),
            request,
            next,
        ))
    })
}

async fn ok_handler() -> &'static str {
    "ok"
}

/// Stub of the merged `/api` surface with one route per taxonomy class.
fn stub_api_router() -> Router<()> {
    Router::new()
        .route("/api/traffic", get(ok_handler))
        .route("/api/traffic/import/har", post(ok_handler))
        .route("/api/sessions", get(ok_handler).post(ok_handler))
        .route("/api/mocks", get(ok_handler).post(ok_handler))
        .route("/api/config", get(ok_handler).patch(ok_handler))
        .route("/api/replay", post(ok_handler))
        .route("/api/users", get(ok_handler))
        .route("/api/devices", get(ok_handler))
        .route("/api/auth/api-keys", get(ok_handler).post(ok_handler))
        .route("/api/mystery", get(ok_handler))
        // Public surface (would be handled by real handlers in production).
        .route("/api/auth/login", post(ok_handler))
        .route("/api/cert/ca", get(ok_handler))
        .route("/api/devices/enroll", post(ok_handler))
        .route("/api/ws", get(ok_handler))
}

/// Build a guarded stub router sharing one auth/store/audit triple.
/// Returns (app, audit, traffic:read key, mocks:write key, `*` key).
async fn guarded_stub() -> (Router, Arc<AuditLogger>, String, String, String) {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let manager = strict_manager().with_store(store.clone());
    let audit = Arc::new(AuditLogger::new(64));

    let traffic_read_key = seed_key(&store, &uid, &["traffic:read"]).await;
    let mocks_write_key = seed_key(&store, &uid, &["mocks:write"]).await;
    let star_key = seed_key(&store, &uid, &["*"]).await;

    let boxed = boxed_auth(Arc::new(manager), store, audit.clone());
    let app = stub_api_router().layer(from_fn(move |request, next| boxed(request, next)));
    (app, audit, traffic_read_key, mocks_write_key, star_key)
}

/// Issue a request against `app` with optional credentials.
async fn send(
    app: &mut Router,
    method: &str,
    uri: &str,
    api_key: Option<&str>,
    bearer: Option<&str>,
) -> StatusCode {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(key) = api_key {
        builder = builder.header("X-API-Key", key);
    }
    if let Some(token) = bearer {
        builder = builder.header("Authorization", format!("Bearer {token}"));
    }
    let request = builder.body(Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    response.status()
}

#[tokio::test]
async fn traffic_read_key_reads_traffic_but_cannot_create_mocks() {
    let (mut app, _audit, traffic_key, _mocks_key, _star) = guarded_stub().await;
    assert_eq!(
        send(&mut app, "GET", "/api/traffic", Some(&traffic_key), None).await,
        StatusCode::OK
    );
    assert_eq!(
        send(&mut app, "POST", "/api/mocks", Some(&traffic_key), None).await,
        StatusCode::FORBIDDEN
    );
    // sessions listing is reachable via the legacy alias expansion.
    assert_eq!(
        send(&mut app, "GET", "/api/sessions", Some(&traffic_key), None).await,
        StatusCode::OK
    );
}

#[tokio::test]
async fn mocks_write_key_roundtrip_denies_traffic_read() {
    let (mut app, _audit, _traffic_key, mocks_key, _star) = guarded_stub().await;
    assert_eq!(
        send(&mut app, "POST", "/api/mocks", Some(&mocks_key), None).await,
        StatusCode::OK
    );
    assert_eq!(
        send(&mut app, "GET", "/api/mocks", Some(&mocks_key), None).await,
        StatusCode::FORBIDDEN,
        "mocks:write alone must not grant the read half"
    );
    assert_eq!(
        send(&mut app, "GET", "/api/traffic", Some(&mocks_key), None).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn unmapped_route_denies_api_key_by_default() {
    let (mut app, _audit, _t, _m, star_key) = guarded_stub().await;
    // Even `*` cannot reach an unmapped route: the fallback is JwtOnly.
    assert_eq!(
        send(&mut app, "GET", "/api/mystery", Some(&star_key), None).await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn star_key_reaches_feature_surface_but_not_excluded_routes() {
    let (mut app, _audit, _t, _m, star_key) = guarded_stub().await;
    for (method, uri) in [
        ("GET", "/api/traffic"),
        ("POST", "/api/mocks"),
        ("GET", "/api/config"),
        ("PATCH", "/api/config"),
        ("POST", "/api/replay"),
        ("POST", "/api/traffic/import/har"),
        ("GET", "/api/sessions"),
    ] {
        assert_eq!(
            send(&mut app, method, uri, Some(&star_key), None).await,
            StatusCode::OK,
            "`*` key should reach {method} {uri}"
        );
    }
    for (method, uri) in [
        ("GET", "/api/users"),
        ("POST", "/api/users"),
        ("GET", "/api/devices"),
        ("GET", "/api/auth/api-keys"),
        ("POST", "/api/auth/api-keys"),
        ("POST", "/api/sessions"),
    ] {
        assert_eq!(
            send(&mut app, method, uri, Some(&star_key), None).await,
            StatusCode::FORBIDDEN,
            "`*` key must be excluded from {method} {uri}"
        );
    }
}

#[tokio::test]
async fn legacy_traffic_grants_expand_to_taxonomy_equivalents() {
    // Pure-function contract of the reconciliation.
    assert_eq!(
        effective_scopes(&["traffic:read".to_string()]),
        vec!["traffic:read".to_string(), "sessions:read".to_string()]
    );
    assert_eq!(
        effective_scopes(&["traffic:write".to_string()]),
        vec!["traffic:write".to_string(), "traffic:export".to_string()]
    );
    assert_eq!(
        effective_scopes(&["traffic:read".to_string(), "traffic:write".to_string()]),
        vec![
            "traffic:read".to_string(),
            "traffic:write".to_string(),
            "sessions:read".to_string(),
            "traffic:export".to_string()
        ]
    );
    // `*` and unrelated grants pass through untouched.
    assert_eq!(effective_scopes(&["*".to_string()]), vec!["*".to_string()]);
    assert_eq!(
        effective_scopes(&["mocks:read".to_string()]),
        vec!["mocks:read".to_string()]
    );
    assert!(effective_scopes(&[]).is_empty());

    // Through the middleware: a legacy traffic:write key can still drive
    // the HAR round-trip (now gated on traffic:export)...
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let manager = Arc::new(strict_manager().with_store(store.clone()));
    let audit = Arc::new(AuditLogger::new(64));
    let legacy_write = seed_key(&store, &uid, &["traffic:write"]).await;
    // ...but a legacy resource-only grant (no permission half) matches
    // nothing in the split taxonomy — sanctioned loss, documented.
    let legacy_resource_only = seed_key(&store, &uid, &["mocks"]).await;
    let app = stub_api_router().layer(from_fn(move |request, next| {
        boxed_auth(manager.clone(), store.clone(), audit.clone())(request, next)
    }));
    let mut app = app;
    assert_eq!(
        send(
            &mut app,
            "POST",
            "/api/traffic/import/har",
            Some(&legacy_write),
            None
        )
        .await,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &mut app,
            "GET",
            "/api/mocks",
            Some(&legacy_resource_only),
            None
        )
        .await,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn unauthenticated_requests_rejected_across_api_nest() {
    let (mut app, _audit, _t, _m, _star) = guarded_stub().await;
    for (method, uri) in [
        ("GET", "/api/traffic"),
        ("GET", "/api/mocks"),
        ("POST", "/api/mocks"),
        ("GET", "/api/config"),
    ] {
        assert_eq!(
            send(&mut app, method, uri, None, None).await,
            StatusCode::UNAUTHORIZED,
            "unauthenticated {method} {uri} must 401 under --enable-auth"
        );
    }
    // Unknown keys are 401 (authentication failure), not 403.
    assert_eq!(
        send(
            &mut app,
            "GET",
            "/api/traffic",
            Some("madhyamas_bogus"),
            None
        )
        .await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn public_paths_bypass_authentication() {
    let (mut app, _audit, _t, _m, _star) = guarded_stub().await;
    for (method, uri) in [
        ("POST", "/api/auth/login"),
        ("GET", "/api/cert/ca"),
        ("POST", "/api/devices/enroll"),
        ("GET", "/api/ws"),
    ] {
        assert_eq!(
            send(&mut app, method, uri, None, None).await,
            StatusCode::OK,
            "{method} {uri} is public (in-handler auth for /ws)"
        );
    }
}

#[tokio::test]
async fn jwt_principal_passes_feature_routes_without_scope_gating() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let manager = strict_manager().with_store(store.clone());
    let audit = Arc::new(AuditLogger::new(64));
    let token = seed_jwt(&manager, &store, &uid, "admin").await;
    let boxed = boxed_auth(Arc::new(manager), store.clone(), audit.clone());
    let app = stub_api_router().layer(from_fn(move |request, next| boxed(request, next)));
    let mut app = app;
    // JWT authorization is role-based downstream; the route-scope map
    // does not gate bearer principals.
    assert_eq!(
        send(&mut app, "GET", "/api/traffic", None, Some(&token)).await,
        StatusCode::OK
    );
    assert_eq!(
        send(&mut app, "POST", "/api/mocks", None, Some(&token)).await,
        StatusCode::OK
    );
    // A garbage bearer token is still rejected.
    assert_eq!(
        send(&mut app, "GET", "/api/traffic", None, Some("garbage.token")).await,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn permission_middleware_rejects_keys_and_non_admin_jwt() {
    use axum::middleware::from_fn_with_state;
    // A permission-gated route shaped like the enterprise /users routes.
    let gated = Router::new().route(
        "/api/admin-only",
        get(ok_handler).layer(from_fn_with_state(
            PermissionState {
                rbac: Arc::new(madhyamas_enterprise::RbacManager::new()),
                resource_type: ResourceType::Config,
                permission: Permission::Write,
            },
            madhyamas_enterprise::middleware::require_permission_middleware,
        )),
    );

    let store = test_store().await;
    let uid = seed_user(&store).await;
    let manager = strict_manager().with_store(store.clone());
    let audit = Arc::new(AuditLogger::new(64));
    let admin_jwt = seed_jwt(&manager, &store, &uid, "admin").await;
    let user_jwt = seed_jwt(&manager, &store, &uid, "user").await;
    let star_key = seed_key(&store, &uid, &["*"]).await;
    let boxed = boxed_auth(Arc::new(manager), store, audit);
    let mut app = gated.layer(from_fn(move |request, next| boxed(request, next)));

    assert_eq!(
        send(&mut app, "GET", "/api/admin-only", None, Some(&admin_jwt)).await,
        StatusCode::OK,
        "admin JWT passes the RBAC gate (unchanged)"
    );
    assert_eq!(
        send(&mut app, "GET", "/api/admin-only", None, Some(&user_jwt)).await,
        StatusCode::FORBIDDEN,
        "non-admin JWT is still 403 (RBAC unchanged)"
    );
    assert_eq!(
        send(&mut app, "GET", "/api/admin-only", Some(&star_key), None).await,
        StatusCode::FORBIDDEN,
        "`*` API key must NOT pass the permission middleware (issue #107 closes the bypass)"
    );
}

#[tokio::test]
async fn api_key_polling_requests_do_not_flood_the_audit_log() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let manager = strict_manager().with_store(store.clone());
    let audit = Arc::new(AuditLogger::new(64));
    let traffic_key = seed_key(&store, &uid, &["traffic:read"]).await;
    let boxed = boxed_auth(Arc::new(manager), store, audit.clone());
    let app = stub_api_router().layer(from_fn(move |request, next| boxed(request, next)));
    let mut app = app;
    for _ in 0..3 {
        let _ = send(&mut app, "GET", "/api/traffic", Some(&traffic_key), None).await;
    }
    // Self-identity surface still audits key logins.
    let _ = send(&mut app, "GET", "/api/auth/me", Some(&traffic_key), None).await;
    let logins = audit
        .all_events()
        .into_iter()
        .filter(|e| e.description.contains("API key authenticated"))
        .count();
    assert_eq!(
        logins, 1,
        "key Login audit events fire only on the /auth surface, not per poll"
    );
}

#[tokio::test]
async fn auth_me_reports_effective_scopes_for_key_principal() {
    // Full stack: the real enterprise router under the boxed middleware,
    // exactly as the main binary mounts it.
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let manager = Arc::new(strict_manager().with_store(store.clone()));
    let audit = Arc::new(AuditLogger::new(64));
    let traffic_key = seed_key(&store, &uid, &["traffic:read"]).await;

    let api_state = Arc::new(AppState::new(
        TrafficStore::new(":memory:").await.expect("traffic store"),
    ));
    let enterprise_router =
        create_enterprise_router(store.clone(), manager.clone(), audit, None, None);
    let boxed = boxed_auth(manager, store, Arc::new(AuditLogger::new(64)));
    // Production shape: the enterprise router is nested under /api and
    // the auth middleware wraps the nest (as madhyamas-api does).
    let app = Router::new()
        .nest("/api", enterprise_router)
        .layer(from_fn(move |request, next| boxed(request, next)))
        .with_state(api_state);

    let request = Request::builder()
        .uri("/api/auth/me")
        .header("X-API-Key", &traffic_key)
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body["scopes"],
        serde_json::json!(["traffic:read", "sessions:read"]),
        "/auth/me must report the taxonomy-expanded (effective) scopes"
    );
}

#[tokio::test]
async fn login_is_public_and_mints_a_working_token_full_stack() {
    let store = test_store().await;
    // Seed a real (Argon2id) credential via the public store API.
    let user = madhyamas_enterprise::User::new(
        "u-login".to_string(),
        "loginuser".to_string(),
        None,
        madhyamas_enterprise::UserRole::Admin,
        "loginuser".to_string(),
        madhyamas_enterprise::UserStatus::Active,
    );
    store
        .create_user(
            &user,
            &hash_password("Sup3rSecret!pass").expect("hash password"),
        )
        .await
        .expect("seed login user");

    let manager = Arc::new(strict_manager().with_store(store.clone()));
    let audit = Arc::new(AuditLogger::new(64));
    let api_state = Arc::new(AppState::new(
        TrafficStore::new(":memory:").await.expect("traffic store"),
    ));
    let enterprise_router =
        create_enterprise_router(store.clone(), manager.clone(), audit, None, None);
    let boxed = boxed_auth(manager, store, Arc::new(AuditLogger::new(64)));
    // Production shape: the enterprise router is nested under /api and
    // the auth middleware wraps the nest (as madhyamas-api does).
    let app = Router::new()
        .nest("/api", enterprise_router)
        .layer(from_fn(move |request, next| boxed(request, next)))
        .with_state(api_state);
    let mut app = app;

    // Unauthenticated login on the public path.
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"username": "loginuser", "password": "Sup3rSecret!pass"})
                .to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let token = body["token"].as_str().expect("access token").to_string();

    // The minted JWT authenticates on a protected route.
    assert_eq!(
        send(&mut app, "GET", "/api/auth/me", None, Some(&token)).await,
        StatusCode::OK
    );
    // Bad password is 401.
    let request = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({"username": "loginuser", "password": "wrong"}).to_string(),
        ))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
