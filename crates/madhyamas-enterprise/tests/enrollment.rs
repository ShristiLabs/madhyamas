//! Integration tests for the public QR-enrollment API (issue #106):
//! `mdy_enroll_...` token shape, REST/proxy rejection of enrollment
//! tokens, store lifecycle (single-use redemption, TTL, revocation
//! cascade, expired-row pruning), handler flows (issue/redeem including
//! the one-live-credential redeem semantics), audit roundtrips, and the
//! onboarding `device` step.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::{Extension, Json};
use madhyamas_api::AppState;
use madhyamas_core::{ProxyAuthValidator, ProxyCredentials, TrafficStore, WsManager};
use madhyamas_enterprise::auth::{
    generate_device_key, generate_enrollment_token, hash_api_key, is_enrollment_token,
};
use madhyamas_enterprise::handlers::{
    enroll_device, get_onboarding_status, issue_device_enrollment_token, EnrollDeviceRequest,
};
use madhyamas_enterprise::middleware::AuthUser;
use madhyamas_enterprise::store::{
    DeviceKeyRecord, DeviceRecord, EnrollmentTokenRecord, EnterpriseStore, SqliteEnterpriseStore,
};
use madhyamas_enterprise::{AuditEvent, AuditEventType, AuditFilter, AuditLogger};
use madhyamas_test_utils::enterprise::{seed_user, test_manager, test_store};

/// Register one device owned by `owner` and return its record.
async fn seed_device(store: &Arc<dyn EnterpriseStore>, owner: &str, name: &str) -> DeviceRecord {
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

/// Mint and persist a device key for `device_id`, returning the record
/// plus the plaintext (mirrors the handler's show-once minting).
async fn seed_device_key(
    store: &Arc<dyn EnterpriseStore>,
    device_id: &str,
) -> (DeviceKeyRecord, String) {
    let key = generate_device_key();
    let record = DeviceKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: device_id.to_string(),
        key_hash: hash_api_key(&key),
        key_prefix: key.chars().take(12).collect(),
        created_at: chrono::Utc::now().to_rfc3339(),
        revoked_at: None,
        last_used_at: None,
    };
    store.create_device_key(&record).await.expect("persist key");
    (record, key)
}

/// Seed an enrollment-token row with explicit expiry/lifecycle stamps,
/// returning the record plus the plaintext token.
async fn seed_enrollment_token(
    store: &Arc<dyn EnterpriseStore>,
    device_id: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> (EnrollmentTokenRecord, String) {
    let token = generate_enrollment_token();
    let record = EnrollmentTokenRecord {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: device_id.to_string(),
        token_hash: hash_api_key(&token),
        token_prefix: token.chars().take(12).collect(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: expires_at.to_rfc3339(),
        redeemed_at: None,
        revoked_at: None,
    };
    store
        .create_enrollment_token(&record)
        .await
        .expect("persist token");
    (record, token)
}

/// Build an AppState for direct handler invocation (same shape as the
/// health-check handler tests).
async fn make_state() -> Arc<AppState> {
    let tmp = tempfile::tempdir().expect("temp dir").keep();
    let db_path = tmp.join("test.db").to_string_lossy().to_string();
    let store = TrafficStore::new(db_path).await.expect("open store");
    Arc::new(AppState::new(store).with_ws_manager(Arc::new(WsManager::new())))
}

fn auth_user(user_id: &str, role: &str) -> AuthUser {
    AuthUser {
        claims: None,
        scopes: None,
        key_id: None,
        session_id: None,
        user_id: user_id.to_string(),
        role: role.to_string(),
    }
}

// ---- Token shape ----

#[test]
fn test_enrollment_token_generator_prefix_length_and_uniqueness() {
    let a = generate_enrollment_token();
    let b = generate_enrollment_token();
    assert!(
        a.starts_with("mdy_enroll_"),
        "prefix must be mdy_enroll_: {a}"
    );
    assert_eq!(a.len(), "mdy_enroll_".len() + 32, "32 hex chars of entropy");
    assert_ne!(a, b, "two generated tokens must differ");
    assert!(
        a["mdy_enroll_".len()..]
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "token body must be hex"
    );
}

#[test]
fn test_is_enrollment_token_classification() {
    assert!(is_enrollment_token("mdy_enroll_abcdef123"));
    assert!(
        is_enrollment_token("  mdy_enroll_padded  "),
        "leading/trailing space is trimmed"
    );
    assert!(!is_enrollment_token("mdy_dev_abcdef123"));
    assert!(
        !is_enrollment_token("mdy_agent_abcdef123"),
        "agent keys are a later issue"
    );
    assert!(
        !is_enrollment_token("mdy_enroll"),
        "prefix without underscore is not an enrollment token"
    );
    assert!(!is_enrollment_token(""));
    assert!(!is_enrollment_token("some-password"));
}

// ---- Rejection: enrollment tokens are exchange-only ----

#[tokio::test]
async fn test_validate_api_key_rejects_enrollment_tokens() {
    let store = test_store().await;
    seed_user(&store).await;
    let mgr = test_manager().with_store(store);

    let token = generate_enrollment_token();
    let err = mgr
        .validate_api_key(&token)
        .await
        .expect_err("enrollment tokens must never authenticate the REST API");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("enrollment"),
        "error should name the credential type: {msg}"
    );
    assert!(
        !msg.contains(&token),
        "token material must not leak into the error"
    );
}

#[tokio::test]
async fn test_proxy_validator_rejects_enrollment_tokens_on_all_arms() {
    let store = test_store().await;
    seed_user(&store).await;
    let mgr = test_manager().with_store(store);
    let token = generate_enrollment_token();

    for creds in [
        ProxyCredentials::ApiKey(token.clone()),
        ProxyCredentials::ProxyBearer(token.clone()),
        ProxyCredentials::ProxyBasicAuth(format!("user:{token}")),
        ProxyCredentials::ProxyBasicAuth(format!("{token}:pass")),
    ] {
        let err = mgr
            .validate(&creds)
            .await
            .expect_err("enrollment tokens must not authenticate proxy connections on any arm");
        assert!(
            err.contains("redeem"),
            "error should point at redemption: {err}"
        );
        assert!(!err.contains(&token), "no token material in error");
    }
}

#[tokio::test]
async fn test_proxy_validator_rejects_enrollment_tokens_without_store() {
    let mgr = test_manager();
    assert!(
        mgr.validate(&ProxyCredentials::ApiKey(generate_enrollment_token()))
            .await
            .is_err(),
        "validation without a store must fail closed"
    );
}

// ---- Store: enrollment-token lifecycle ----

#[tokio::test]
async fn test_enrollment_token_crud_and_lookup() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "CRUD phone").await;
    let (record, token) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;

    let got = store
        .get_enrollment_token_by_hash(&record.token_hash)
        .await
        .expect("lookup")
        .expect("present");
    assert_eq!(got.id, record.id);
    assert_eq!(got.device_id, device.id);
    assert_eq!(got.token_prefix, token.chars().take(12).collect::<String>());
    assert!(got.redeemed_at.is_none());
    assert!(got.revoked_at.is_none());

    assert!(
        store
            .get_enrollment_token_by_hash(&hash_api_key("mdy_enroll_nosuchtoken"))
            .await
            .expect("lookup")
            .is_none(),
        "unknown token hashes resolve to None"
    );
}

#[tokio::test]
async fn test_enrollment_token_redeem_is_single_use() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "Single-use phone").await;
    let (record, _token) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;

    let now = chrono::Utc::now().to_rfc3339();
    let first = store
        .redeem_enrollment_token(&record.token_hash, &now)
        .await
        .expect("first redeem");
    assert!(first, "a fresh unexpired token redeems exactly once");

    let second = store
        .redeem_enrollment_token(&record.token_hash, &now)
        .await
        .expect("second redeem query");
    assert!(
        !second,
        "the same token must never redeem twice (single-use)"
    );

    let got = store
        .get_enrollment_token_by_hash(&record.token_hash)
        .await
        .expect("lookup")
        .expect("row kept for audit");
    assert!(got.redeemed_at.is_some(), "redemption is stamped");
}

#[tokio::test]
async fn test_enrollment_token_concurrent_redeem_exactly_one_winner() {
    // The single-use guard is an atomic compare-and-set on redeemed_at;
    // racing redemptions must produce exactly one winner. The pool is
    // capped at one connection so SQLite locking never flakes the test —
    // the guard, not the lock, is what this asserts.
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(":memory:")
        .await
        .expect("pool");
    let store = SqliteEnterpriseStore::new(pool).await.expect("store");
    let store = Arc::new(store) as Arc<dyn EnterpriseStore>;
    let device = seed_device(&store, "u1", "Race phone").await;
    let (record, _token) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;

    let mut handles = Vec::new();
    for _ in 0..4 {
        let st = Arc::clone(&store);
        let hash = record.token_hash.clone();
        handles.push(tokio::spawn(async move {
            st.redeem_enrollment_token(&hash, &chrono::Utc::now().to_rfc3339())
                .await
        }));
    }
    let mut winners = 0;
    for h in handles {
        if h.await.expect("join").expect("redeem query") {
            winners += 1;
        }
    }
    assert_eq!(winners, 1, "exactly one concurrent redemption wins");
}

#[tokio::test]
async fn test_enrollment_token_ttl_enforced_at_redeem() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "TTL phone").await;

    // Expired token: expires_at in the past.
    let (expired, _t1) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() - chrono::Duration::minutes(1),
    )
    .await;
    let now = chrono::Utc::now().to_rfc3339();
    assert!(
        !store
            .redeem_enrollment_token(&expired.token_hash, &now)
            .await
            .expect("redeem query"),
        "expired tokens must not redeem"
    );

    // Unexpired token for the same device still redeems.
    let (valid, _t2) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;
    assert!(
        store
            .redeem_enrollment_token(&valid.token_hash, &now)
            .await
            .expect("redeem query"),
        "unexpired tokens redeem"
    );
}

#[tokio::test]
async fn test_enrollment_token_revoked_redeem_rejected_and_scoped() {
    let store = test_store().await;
    let d1 = seed_device(&store, "u1", "Revoked-token phone").await;
    let d2 = seed_device(&store, "u1", "Other phone").await;

    let (r1, _t1) = seed_enrollment_token(
        &store,
        &d1.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;
    let (r2, _t2) = seed_enrollment_token(
        &store,
        &d2.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;

    store
        .revoke_enrollment_tokens_for_device(&d1.id)
        .await
        .expect("cascade revoke");

    let now = chrono::Utc::now().to_rfc3339();
    assert!(
        !store
            .redeem_enrollment_token(&r1.token_hash, &now)
            .await
            .expect("q"),
        "revoked tokens must not redeem"
    );
    assert!(
        store
            .redeem_enrollment_token(&r2.token_hash, &now)
            .await
            .expect("q"),
        "another device's tokens are untouched by the cascade"
    );

    let row1 = store
        .get_enrollment_token_by_hash(&r1.token_hash)
        .await
        .expect("lookup")
        .expect("row kept");
    assert!(row1.revoked_at.is_some(), "revocation is stamped");
}

#[tokio::test]
async fn test_enrollment_token_delete_expired_counts() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "Prune phone").await;
    let past = chrono::Utc::now() - chrono::Duration::minutes(1);
    let future = chrono::Utc::now() + chrono::Duration::minutes(15);

    let (_expired, _t1) = seed_enrollment_token(&store, &device.id, past).await;
    // A redeemed-and-expired row also matches the prune predicate.
    let (redeemed_expired, _t2) = seed_enrollment_token(&store, &device.id, past).await;
    store
        .redeem_enrollment_token(&redeemed_expired.token_hash, &past.to_rfc3339())
        .await
        .expect("seed redeemed row");
    let (_valid, _t3) = seed_enrollment_token(&store, &device.id, future).await;

    let removed = store
        .delete_expired_enrollment_tokens(&chrono::Utc::now().to_rfc3339())
        .await
        .expect("prune");
    assert_eq!(removed, 2, "only the expired rows are pruned");
}

// ---- Audit roundtrip for the new event types ----

#[tokio::test]
async fn test_audit_enrollment_event_types_roundtrip() {
    let store = test_store().await;
    for (event_type, label) in [
        (
            AuditEventType::DeviceEnrollmentIssued,
            "device_enrollment_issued",
        ),
        (AuditEventType::DeviceEnrolled, "device_enrolled"),
    ] {
        let event = AuditEvent::new(event_type, "enrollment lifecycle");
        store.log_audit_event(&event).await.expect("log");

        let filter = AuditFilter {
            event_type: Some(event_type),
            ..Default::default()
        };
        let events = store.query_audit_events(&filter).await.expect("query");
        assert!(
            events.iter().any(|e| e.event_type == event_type),
            "roundtrip must preserve the {label} type"
        );
    }
}

// ---- Handlers: token issuance ----

#[tokio::test]
async fn test_issue_enrollment_token_returns_token_with_fifteen_minute_ttl() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Issue phone").await;

    let resp = issue_device_enrollment_token(
        State(state),
        Extension(store.clone()),
        Extension(audit),
        Extension(auth_user(&uid, "admin")),
        Path(device.id.clone()),
    )
    .await
    .expect("issue")
    .0;

    assert_eq!(resp.device.id, device.id);
    assert!(
        is_enrollment_token(&resp.token),
        "issued plaintext carries the enrollment prefix"
    );
    let expires =
        chrono::DateTime::parse_from_rfc3339(&resp.expires_at).expect("expires_at is RFC 3339");
    let ttl = expires.timestamp() - chrono::Utc::now().timestamp();
    assert!(
        (14 * 60..=16 * 60).contains(&ttl),
        "TTL must be ~15 minutes, got {ttl}s"
    );

    // The token is persisted hashed: the row must NOT contain the plaintext.
    let row = store
        .get_enrollment_token_by_hash(&hash_api_key(&resp.token))
        .await
        .expect("lookup")
        .expect("token persisted by hash");
    assert_ne!(row.token_hash, resp.token, "hash at rest, not plaintext");
}

#[tokio::test]
async fn test_issue_enrollment_token_ownership_enforced() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Owned phone").await;

    // Unknown device: 404.
    let status = issue_device_enrollment_token(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "admin")),
        Path("no-such-device".to_string()),
    )
    .await
    .expect_err("unknown device");
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);

    // Non-owner, non-admin: 403.
    let status = issue_device_enrollment_token(
        State(state),
        Extension(store),
        Extension(audit),
        Extension(auth_user("someone-else", "user")),
        Path(device.id),
    )
    .await
    .expect_err("not the owner");
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_issue_enrollment_token_revoked_device_conflict() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Revoked phone").await;
    store
        .update_device_status(&device.id, "revoked")
        .await
        .expect("revoke device");

    let status = issue_device_enrollment_token(
        State(state),
        Extension(store),
        Extension(audit),
        Extension(auth_user(&uid, "admin")),
        Path(device.id),
    )
    .await
    .expect_err("revoked devices cannot enroll");
    assert_eq!(status, axum::http::StatusCode::CONFLICT);
}

// ---- Handlers: redemption ----

#[tokio::test]
async fn test_enroll_redeems_and_returns_working_device_key() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Enroll phone").await;
    // The create-time show-once key — must die at redemption.
    let (_create_rec, create_key) = seed_device_key(&store, &device.id).await;

    let issued = issue_device_enrollment_token(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "admin")),
        Path(device.id.clone()),
    )
    .await
    .expect("issue")
    .0;

    let redeemed = enroll_device(
        State(state),
        Extension(store.clone()),
        Extension(audit),
        Json(EnrollDeviceRequest {
            token: issued.token.clone(),
        }),
    )
    .await
    .expect("redeem")
    .0;
    assert_eq!(redeemed.device.id, device.id);
    assert!(
        redeemed.key.starts_with("mdy_dev_"),
        "redemption returns a real device credential"
    );
    assert_ne!(redeemed.key, create_key, "a FRESH key is minted");

    // The redeemed key authenticates a CONNECT (the device-auth path).
    let auth = mgr
        .validate_device_key(&redeemed.key)
        .await
        .expect("the redeemed key must authenticate exactly like a created device key");
    assert_eq!(auth.device_id, device.id);
    assert_eq!(auth.device_name, "Enroll phone");

    // One live credential: the create-time key is retired by redemption.
    assert!(
        mgr.validate_device_key(&create_key).await.is_err(),
        "the create-time show-once key must stop authenticating after redemption"
    );

    // Single-use: the same token cannot be redeemed again.
    let status = enroll_device(
        State(make_state().await),
        Extension(store),
        Extension(Arc::new(AuditLogger::new(64))),
        Json(EnrollDeviceRequest {
            token: issued.token,
        }),
    )
    .await
    .expect_err("single-use");
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_enroll_rejects_bad_shape_unknown_and_expired() {
    let store = test_store().await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));

    // Not an enrollment token at all: 400.
    let status = enroll_device(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Json(EnrollDeviceRequest {
            token: "not-a-token".to_string(),
        }),
    )
    .await
    .expect_err("bad shape");
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);

    // Well-formed but unknown: 401.
    let status = enroll_device(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Json(EnrollDeviceRequest {
            token: generate_enrollment_token(),
        }),
    )
    .await
    .expect_err("unknown token");
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);

    // Expired: 401 (seeded with a past expiry).
    let uid = seed_user(&store).await;
    let device = seed_device(&store, &uid, "Expired phone").await;
    let (_rec, expired_token) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() - chrono::Duration::minutes(1),
    )
    .await;
    let status = enroll_device(
        State(state),
        Extension(store),
        Extension(audit),
        Json(EnrollDeviceRequest {
            token: expired_token,
        }),
    )
    .await
    .expect_err("expired token");
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_enroll_revoked_device_rejected() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Dead phone").await;
    let (_rec, token) = seed_enrollment_token(
        &store,
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    )
    .await;
    // Belt-and-braces path: token not cascade-revoked, but the device is.
    store
        .update_device_status(&device.id, "revoked")
        .await
        .expect("revoke device");

    let status = enroll_device(
        State(state),
        Extension(store),
        Extension(audit),
        Json(EnrollDeviceRequest { token }),
    )
    .await
    .expect_err("revoked device must not enroll");
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

// ---- Audit: issue + redeem recorded without secret material ----

#[tokio::test]
async fn test_enrollment_audit_events_recorded_without_secret_material() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    // No store attached: query the synchronous in-memory ring so the
    // fire-and-forget persistence cannot race the assertions.
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Audit phone").await;

    let issued = issue_device_enrollment_token(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "admin")),
        Path(device.id.clone()),
    )
    .await
    .expect("issue")
    .0;
    let redeemed = enroll_device(
        State(state),
        Extension(store),
        Extension(audit.clone()),
        Json(EnrollDeviceRequest {
            token: issued.token.clone(),
        }),
    )
    .await
    .expect("redeem")
    .0;

    let issued_events = audit.query_in_memory(&AuditFilter {
        event_type: Some(AuditEventType::DeviceEnrollmentIssued),
        ..Default::default()
    });
    assert_eq!(issued_events.len(), 1, "issuance is audited");
    let enrolled_events = audit.query_in_memory(&AuditFilter {
        event_type: Some(AuditEventType::DeviceEnrolled),
        ..Default::default()
    });
    assert_eq!(enrolled_events.len(), 1, "redemption is audited");

    for event in issued_events.iter().chain(enrolled_events.iter()) {
        let serialized = serde_json::to_string(event).expect("serialize");
        assert!(
            !serialized.contains(&issued.token),
            "audit must never contain the enrollment token"
        );
        assert!(
            !serialized.contains(&redeemed.key),
            "audit must never contain the device key"
        );
        assert!(
            event
                .metadata
                .get("device_id")
                .is_some_and(|v| v.as_str() == Some(device.id.as_str())),
            "device_id rides in metadata"
        );
    }
    // The redeem event attributes the owner (the only principal the
    // public endpoint can derive).
    assert_eq!(enrolled_events[0].user_id.as_deref(), Some(uid.as_str()));
}

// ---- Onboarding: the enterprise device step ----

#[tokio::test]
async fn test_onboarding_steps_include_device_step() {
    let state = make_state().await;
    let status = get_onboarding_status(State(state)).await;
    let ids: Vec<&str> = status.steps.iter().map(|s| s.id.as_str()).collect();
    let device_pos = ids
        .iter()
        .position(|id| *id == "device")
        .expect("device step present");
    assert_eq!(
        ids.get(device_pos.wrapping_sub(1)),
        Some(&"proxy"),
        "device step follows the proxy step"
    );
    assert!(status.steps[device_pos].optional, "device step is optional");
    assert_eq!(status.total_steps as usize, status.steps.len());
}
