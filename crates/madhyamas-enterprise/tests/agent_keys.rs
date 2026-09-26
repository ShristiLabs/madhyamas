//! Integration tests for the public device-derived agent-key API
//! (issue #108): `mdy_agent_...` shape, REST resolution to
//! `(user, device, scopes)`, proxy-CONNECT rejection (the inverse of the
//! #104 `mdy_dev_` REST rejection), store lifecycle, the mint/list/revoke
//! handlers with the #107 scope taxonomy and presets, the device
//! revoke/delete cascade, and the rotation-immunity guarantee
//! (referential binding — the security property the doc's derivation
//! table demands).

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::{Extension, Json};
use madhyamas_api::AppState;
use madhyamas_core::{ProxyAuthValidator, ProxyCredentials, TrafficStore, WsManager};
use madhyamas_enterprise::auth::{
    generate_agent_key, generate_device_key, hash_api_key, is_agent_key,
};
use madhyamas_enterprise::handlers::{
    agent_key_preset_scopes, create_agent_key, delete_device, list_agent_keys, revoke_agent_key,
    revoke_device, rotate_device_key, CreateAgentKeyRequest, AGENT_KEY_PRESETS, AGENT_KEY_TAXONOMY,
};
use madhyamas_enterprise::middleware::AuthUser;
use madhyamas_enterprise::store::{AgentKeyRecord, DeviceRecord, EnterpriseStore};
use madhyamas_enterprise::{AuditEventType, AuditFilter, AuditLogger};
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

/// Mint and persist an agent key for `device_id` with the given scopes,
/// returning the record plus the plaintext (mirrors the handler's
/// show-once minting).
async fn seed_agent_key(
    store: &Arc<dyn EnterpriseStore>,
    device: &DeviceRecord,
    scopes: &[&str],
) -> (AgentKeyRecord, String) {
    let key = generate_agent_key();
    let record = AgentKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        parent_device_id: device.id.clone(),
        owner_user_id: device.owner_user_id.clone(),
        name: "seed agent".to_string(),
        key_hash: hash_api_key(&key),
        key_prefix: key.chars().take(12).collect(),
        scopes: serde_json::to_string(scopes).expect("scopes json"),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        revoked_at: None,
        last_used_at: None,
    };
    store.create_agent_key(&record).await.expect("persist key");
    (record, key)
}

/// Build an AppState for direct handler invocation.
async fn make_state() -> Arc<AppState> {
    let tmp = tempfile::tempdir().expect("temp dir").keep();
    let db_path = tmp.join("test.db").to_string_lossy().to_string();
    let store = TrafficStore::new(db_path).await.expect("open store");
    Arc::new(AppState::new(store).with_ws_manager(Arc::new(WsManager::new())))
}

/// Seed a second, distinct user (the shared fixture seeds exactly one
/// "u-test" per store; username is UNIQUE).
async fn seed_other_user(store: &Arc<dyn EnterpriseStore>) -> String {
    let user = madhyamas_enterprise::User::new(
        "u-other".to_string(),
        format!("other-{}", uuid::Uuid::new_v4()),
        None,
        madhyamas_enterprise::UserRole::User,
        "other".to_string(),
        madhyamas_enterprise::UserStatus::Active,
    );
    let id = user.id.clone();
    store
        .create_user(&user, "$argon2id$stub")
        .await
        .expect("create other user");
    id
}

fn auth_user(user_id: &str, role: &str) -> AuthUser {
    AuthUser {
        claims: None,
        scopes: None,
        key_id: None,
        session_id: None,
        device_id: None,
        user_id: user_id.to_string(),
        role: role.to_string(),
    }
}

// ---- Credential shape ----

#[test]
fn test_agent_key_generator_prefix_length_and_uniqueness() {
    let a = generate_agent_key();
    let b = generate_agent_key();
    assert!(
        a.starts_with("mdy_agent_"),
        "prefix must be mdy_agent_: {a}"
    );
    assert_eq!(a.len(), "mdy_agent_".len() + 32, "32 hex chars of entropy");
    assert_ne!(a, b, "two generated keys must differ");
    assert!(
        a["mdy_agent_".len()..]
            .chars()
            .all(|c| c.is_ascii_hexdigit()),
        "key body must be hex"
    );
}

#[test]
fn test_is_agent_key_classification() {
    assert!(is_agent_key("mdy_agent_abcdef123"));
    assert!(
        is_agent_key("  mdy_agent_padded  "),
        "leading/trailing space is trimmed"
    );
    assert!(!is_agent_key("madhyamas_abcdef123"));
    assert!(!is_agent_key("mdy_dev_abcdef123"));
    assert!(!is_agent_key("mdy_enroll_abcdef123"));
    assert!(!is_agent_key("mdy_agent"));
    assert!(!is_agent_key(""));
}

// ---- REST validation: (user, device, scopes) resolution ----

#[tokio::test]
async fn test_validate_agent_key_resolves_owner_device_scopes_and_stamps_last_used() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Pixel").await;
    let (record, key) = seed_agent_key(&store, &device, &["traffic:read", "mocks:write"]).await;

    let auth = mgr.validate_api_key(&key).await.expect("validate");
    assert_eq!(auth.user_id, uid, "owner denormalized on the row");
    assert_eq!(
        auth.device_id.as_deref(),
        Some(device.id.as_str()),
        "agent keys resolve to their parent device"
    );
    assert_eq!(auth.key_id, record.id);
    // Scopes come back exactly as persisted (stored-order).
    assert_eq!(auth.scopes, vec!["traffic:read", "mocks:write"]);

    // last_used is fire-and-forget; poll briefly for it.
    let mut stamped = false;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        if let Some(row) = store
            .get_agent_key_by_hash(&hash_api_key(&key))
            .await
            .expect("lookup")
        {
            if row.last_used_at.is_some() {
                stamped = true;
                break;
            }
        }
    }
    assert!(stamped, "agent key last_used must be stamped on validation");
}

#[tokio::test]
async fn test_validate_agent_key_revoked_expired_and_unknown_rejected() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());
    let device = seed_device(&store, &uid, "Pixel").await;

    // Revoked.
    let (record, key) = seed_agent_key(&store, &device, &["traffic:read"]).await;
    store.revoke_agent_key(&record.id).await.expect("revoke");
    let err = mgr
        .validate_api_key(&key)
        .await
        .expect_err("revoked agent key must be rejected");
    assert!(
        err.to_string().to_lowercase().contains("revoked"),
        "error names revocation: {err}"
    );

    // Expired (backdated expiry).
    let key2 = generate_agent_key();
    let rec2 = AgentKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        parent_device_id: device.id.clone(),
        owner_user_id: uid.clone(),
        name: String::new(),
        key_hash: hash_api_key(&key2),
        key_prefix: key2.chars().take(12).collect(),
        scopes: r#"["traffic:read"]"#.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: Some((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339()),
        revoked_at: None,
        last_used_at: None,
    };
    store
        .create_agent_key(&rec2)
        .await
        .expect("persist expired");
    let err = mgr
        .validate_api_key(&key2)
        .await
        .expect_err("expired agent key must be rejected");
    assert!(
        err.to_string().to_lowercase().contains("expired"),
        "error names expiry: {err}"
    );

    // Unknown.
    let err = mgr
        .validate_api_key(&generate_agent_key())
        .await
        .expect_err("unknown agent key must be rejected");
    assert!(
        !err.to_string().contains("mdy_agent_"),
        "no key material in the error"
    );
}

#[tokio::test]
async fn test_validate_agent_key_parent_device_revoked_or_missing_rejected() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    // Device revoked (belt-and-braces beyond the cascade revoke).
    let device = seed_device(&store, &uid, "Zombie phone").await;
    let (_record, key) = seed_agent_key(&store, &device, &["traffic:read"]).await;
    store
        .update_device_status(&device.id, "revoked")
        .await
        .expect("revoke device");
    let err = mgr
        .validate_api_key(&key)
        .await
        .expect_err("agent key of a revoked device must be rejected");
    assert!(
        err.to_string().to_lowercase().contains("revoked"),
        "error names the device revocation: {err}"
    );

    // Parent device missing entirely (stale row).
    let ghost = DeviceRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: "Ghost".to_string(),
        owner_user_id: uid.clone(),
        install_uuid: None,
        mac_address: None,
        status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_seen: None,
    };
    let (_rec, key2) = seed_agent_key(&store, &ghost, &["traffic:read"]).await;
    mgr.validate_api_key(&key2)
        .await
        .expect_err("dangling agent key must be rejected");
}

#[tokio::test]
async fn test_user_key_validation_still_has_no_device_binding() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    // A plain user key (not device-derived) keeps device_id None.
    let key = madhyamas_enterprise::ApiKey::generate(&uid, "user key");
    let record = madhyamas_enterprise::store::ApiKeyRecord {
        id: key.id.clone(),
        user_id: uid.clone(),
        name: key.name.clone(),
        key_hash: hash_api_key(&key.key),
        key_prefix: key.key.chars().take(12).collect(),
        scopes: r#"["traffic:read"]"#.to_string(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_api_key(&record)
        .await
        .expect("persist user key");
    let auth = mgr
        .validate_api_key(&key.key)
        .await
        .expect("user key validates");
    assert_eq!(
        auth.device_id, None,
        "plain user keys must not gain a device binding"
    );
    assert_eq!(auth.user_id, uid);
}

// ---- Proxy-CONNECT rejection: the inverse of the #104 rule ----

#[tokio::test]
async fn test_agent_keys_rejected_on_every_proxy_auth_arm() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());
    let device = seed_device(&store, &uid, "Pixel").await;
    let (_record, key) = seed_agent_key(&store, &device, &["traffic:read"]).await;

    let creds = [
        ProxyCredentials::ApiKey(key.clone()),
        ProxyCredentials::ProxyBearer(key.clone()),
        ProxyCredentials::ProxyBasicAuth(format!("user:{key}")),
        ProxyCredentials::ProxyBasicAuth(format!("{key}:pw")),
    ];
    for cred in &creds {
        let err = mgr
            .validate(cred)
            .await
            .expect_err("agent keys must never authenticate proxy connections");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("device key"),
            "error should point at the device key: {msg}"
        );
        assert!(
            !msg.contains(&key),
            "key material must not leak into the error"
        );
    }
}

// ---- Store lifecycle ----

#[tokio::test]
async fn test_agent_key_store_crud_and_scoping() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let alpha = seed_device(&store, &uid, "Alpha").await;
    let beta = seed_device(&store, &uid, "Beta").await;

    let (a1, k1) = seed_agent_key(&store, &alpha, &["traffic:read"]).await;
    // Ensure a deterministic newest-first ordering gap.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let (a2, k2) = seed_agent_key(&store, &alpha, &["mocks:write"]).await;
    let (_b1, _bk) = seed_agent_key(&store, &beta, &["traffic:read"]).await;

    // Hash lookup.
    let fetched = store
        .get_agent_key_by_hash(&hash_api_key(&k1))
        .await
        .expect("lookup")
        .expect("found");
    assert_eq!(fetched.id, a1.id);

    // List is newest-first and scoped to the parent device.
    let listed = store.list_agent_keys(&alpha.id).await.expect("list");
    assert_eq!(listed.len(), 2, "beta's key must not appear");
    assert_eq!(listed[0].id, a2.id, "newest first");
    assert_eq!(listed[1].id, a1.id);

    // Single revoke: sibling untouched, other device untouched.
    store.revoke_agent_key(&a1.id).await.expect("revoke");
    let rows = store.list_agent_keys(&alpha.id).await.expect("list");
    assert!(rows
        .iter()
        .find(|r| r.id == a1.id)
        .unwrap()
        .revoked_at
        .is_some());
    assert!(rows
        .iter()
        .find(|r| r.id == a2.id)
        .unwrap()
        .revoked_at
        .is_none());
    let beta_rows = store.list_agent_keys(&beta.id).await.expect("list beta");
    assert!(beta_rows.iter().all(|r| r.revoked_at.is_none()));

    // Per-device cascade revoke.
    store
        .revoke_agent_keys_for_device(&beta.id)
        .await
        .expect("cascade");
    let beta_rows = store.list_agent_keys(&beta.id).await.expect("list beta");
    assert!(beta_rows.iter().all(|r| r.revoked_at.is_some()));
    // Revoke is idempotent (rows-affected guard).
    store
        .revoke_agent_keys_for_device(&beta.id)
        .await
        .expect("again");

    // last_used stamp.
    store
        .update_agent_key_last_used(&a2.id)
        .await
        .expect("stamp");
    let stamped = store
        .get_agent_key_by_hash(&hash_api_key(&k2))
        .await
        .expect("lookup")
        .expect("found");
    assert!(stamped.last_used_at.is_some());
}

// ---- Taxonomy and presets ----

#[test]
fn test_agent_key_presets_match_the_doc() {
    // read-only-agent: traffic:read + config:read.
    assert_eq!(
        agent_key_preset_scopes("read-only-agent").expect("preset"),
        vec!["traffic:read".to_string(), "config:read".to_string()]
    );
    // intercept-agent: read-only + mocks/rewrites/breakpoints/blocklist/throttle r/w.
    let intercept = agent_key_preset_scopes("intercept-agent").expect("preset");
    for scope in [
        "traffic:read",
        "config:read",
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
    ] {
        assert!(intercept.contains(&scope.to_string()), "missing {scope}");
    }
    assert_eq!(intercept.len(), 12, "no extra scopes");
    assert!(agent_key_preset_scopes("nope").is_none(), "unknown preset");
    assert_eq!(AGENT_KEY_PRESETS.len(), 2);
    // Every preset scope must be taxonomy-valid.
    for (_, scopes) in AGENT_KEY_PRESETS {
        for s in *scopes {
            assert!(
                AGENT_KEY_TAXONOMY.contains(s),
                "{s} must be in the taxonomy"
            );
        }
    }
    // The taxonomy is exactly the #107 set (no `*`).
    assert_eq!(AGENT_KEY_TAXONOMY.len(), 16);
    assert!(!AGENT_KEY_TAXONOMY.contains(&"*"));
}

// ---- Handlers: mint / list / revoke ----

#[tokio::test]
async fn test_mint_agent_key_happy_path_show_once_hash_at_rest_audit() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Pixel").await;

    let res = create_agent_key(
        State(state),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "user")),
        Path(device.id.clone()),
        Json(CreateAgentKeyRequest {
            name: Some("copilot".to_string()),
            preset: Some("read-only-agent".to_string()),
            scopes: vec!["mocks:read".to_string()],
            expires_in_days: Some(7),
        }),
    )
    .await
    .expect("mint")
    .0;

    // Show-once plaintext validates as an agent key with the union scopes.
    assert!(res.secret.starts_with("mdy_agent_"));
    let mgr = test_manager().with_store(store.clone());
    let auth = mgr.validate_api_key(&res.secret).await.expect("validate");
    assert_eq!(auth.device_id.as_deref(), Some(device.id.as_str()));
    assert_eq!(auth.user_id, uid);
    // preset ∪ explicit, sorted+deduped.
    assert_eq!(
        auth.scopes,
        vec!["config:read", "mocks:read", "traffic:read"]
    );
    assert_eq!(res.key.name, "copilot");
    assert!(res.key.expires_at.is_some(), "expiry honored");

    // Hash at rest: the stored row's hash is of the plaintext, and the
    // list response never carries the hash or the secret.
    let row = store
        .get_agent_key_by_hash(&hash_api_key(&res.secret))
        .await
        .expect("lookup")
        .expect("row");
    assert_eq!(row.key_hash, hash_api_key(&res.secret));
    assert_ne!(row.key_hash, res.secret);

    // Audit: ApiKeyCreated with parent device + agent kind, no secrets.
    let events = audit.query_in_memory(&AuditFilter {
        event_type: Some(AuditEventType::ApiKeyCreated),
        ..Default::default()
    });
    assert_eq!(events.len(), 1, "mint is audited");
    let serialized = serde_json::to_string(&events[0]).expect("serialize");
    assert!(
        !serialized.contains(&res.secret),
        "audit must never contain the agent key material"
    );
    assert_eq!(
        events[0]
            .metadata
            .get("parent_device_id")
            .and_then(|v| v.as_str()),
        Some(device.id.as_str())
    );
    assert_eq!(
        events[0].metadata.get("key_kind").and_then(|v| v.as_str()),
        Some("agent")
    );
}

#[tokio::test]
async fn test_mint_agent_key_scope_validation() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Pixel").await;

    let cases: Vec<CreateAgentKeyRequest> = vec![
        // No scopes at all.
        CreateAgentKeyRequest {
            name: None,
            preset: None,
            scopes: vec![],
            expires_in_days: None,
        },
        // Wildcard rejected for agent keys.
        CreateAgentKeyRequest {
            name: None,
            preset: None,
            scopes: vec!["*".to_string()],
            expires_in_days: None,
        },
        // Unknown scope.
        CreateAgentKeyRequest {
            name: None,
            preset: None,
            scopes: vec!["intercept:all".to_string()],
            expires_in_days: None,
        },
        // Unknown preset.
        CreateAgentKeyRequest {
            name: None,
            preset: Some("admin-agent".to_string()),
            scopes: vec![],
            expires_in_days: None,
        },
    ];
    for req in cases {
        let status = create_agent_key(
            State(state.clone()),
            Extension(store.clone()),
            Extension(audit.clone()),
            Extension(auth_user(&uid, "user")),
            Path(device.id.clone()),
            Json(req),
        )
        .await
        .expect_err("invalid mint must be rejected");
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn test_mint_agent_key_ownership_and_device_state() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let other = seed_other_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Pixel").await;

    // Non-owner non-admin: 403.
    let status = create_agent_key(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&other, "user")),
        Path(device.id.clone()),
        Json(CreateAgentKeyRequest {
            name: None,
            preset: Some("read-only-agent".to_string()),
            scopes: vec![],
            expires_in_days: None,
        }),
    )
    .await
    .expect_err("stranger must not mint");
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);

    // Admin may mint for another user's device.
    let _ = create_agent_key(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&other, "admin")),
        Path(device.id.clone()),
        Json(CreateAgentKeyRequest {
            name: None,
            preset: Some("read-only-agent".to_string()),
            scopes: vec![],
            expires_in_days: None,
        }),
    )
    .await
    .expect("admin may mint for any device");

    // Unknown device: 404.
    let status = create_agent_key(
        State(state.clone()),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "user")),
        Path("no-such-device".to_string()),
        Json(CreateAgentKeyRequest {
            name: None,
            preset: Some("read-only-agent".to_string()),
            scopes: vec![],
            expires_in_days: None,
        }),
    )
    .await
    .expect_err("unknown device");
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);

    // Revoked device: 409.
    store
        .update_device_status(&device.id, "revoked")
        .await
        .expect("revoke device");
    let status = create_agent_key(
        State(state),
        Extension(store),
        Extension(audit),
        Extension(auth_user(&uid, "user")),
        Path(device.id),
        Json(CreateAgentKeyRequest {
            name: None,
            preset: Some("read-only-agent".to_string()),
            scopes: vec![],
            expires_in_days: None,
        }),
    )
    .await
    .expect_err("revoked device");
    assert_eq!(status, axum::http::StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_list_agent_keys_metadata_only_and_ownership() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let other = seed_other_user(&store).await;
    let state = make_state().await;
    let device = seed_device(&store, &uid, "Pixel").await;
    let (_a1, _k1) = seed_agent_key(&store, &device, &["traffic:read"]).await;
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let (_a2, k2) = seed_agent_key(&store, &device, &["mocks:write"]).await;

    let listed = list_agent_keys(
        State(state.clone()),
        Extension(store.clone()),
        Extension(auth_user(&uid, "user")),
        Path(device.id.clone()),
    )
    .await
    .expect("list")
    .0;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].id, _a2.id, "newest first");
    assert_eq!(listed[0].scopes, vec!["mocks:write"]);
    assert_eq!(listed[0].status, "active");
    // Metadata only: the serialized response must carry neither the hash
    // nor the plaintext of any key.
    let serialized = serde_json::to_string(&listed).expect("serialize");
    assert!(!serialized.contains("key_hash"), "no hash column");
    assert!(!serialized.contains(&k2), "no plaintext");

    // Stranger: 403. Unknown device: 404.
    let status = list_agent_keys(
        State(state.clone()),
        Extension(store.clone()),
        Extension(auth_user(&other, "user")),
        Path(device.id.clone()),
    )
    .await
    .expect_err("stranger may not list");
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    let status = list_agent_keys(
        State(state),
        Extension(store),
        Extension(auth_user(&uid, "user")),
        Path("no-such-device".to_string()),
    )
    .await
    .expect_err("unknown device");
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_revoke_agent_key_leaves_device_and_siblings_alone() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let other = seed_other_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Pixel").await;
    let (dk, dev_key) = {
        let key = generate_device_key();
        let record = madhyamas_enterprise::store::DeviceKeyRecord {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: device.id.clone(),
            key_hash: hash_api_key(&key),
            key_prefix: key.chars().take(12).collect(),
            created_at: chrono::Utc::now().to_rfc3339(),
            revoked_at: None,
            last_used_at: None,
        };
        store
            .create_device_key(&record)
            .await
            .expect("persist device key");
        (record, key)
    };
    let (a1, k1) = seed_agent_key(&store, &device, &["traffic:read"]).await;
    let (_a2, k2) = seed_agent_key(&store, &device, &["mocks:write"]).await;
    let beta = seed_device(&store, &uid, "Beta").await;
    let (b1, _bk) = seed_agent_key(&store, &beta, &["traffic:read"]).await;

    let mgr = test_manager().with_store(store.clone());

    revoke_agent_key(
        State(state),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "user")),
        Path((device.id.clone(), a1.id.clone())),
    )
    .await
    .expect("revoke");

    // The revoked key is dead; device key + sibling + other device alive.
    mgr.validate_api_key(&k1)
        .await
        .expect_err("revoked agent key is dead");
    mgr.validate_api_key(&k2).await.expect("sibling survives");
    mgr.validate_device_key(&dev_key)
        .await
        .expect("device key survives");

    // Audit: ApiKeyRevoked with the parent device, no secrets.
    let events = audit.query_in_memory(&AuditFilter {
        event_type: Some(AuditEventType::ApiKeyRevoked),
        ..Default::default()
    });
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0]
            .metadata
            .get("parent_device_id")
            .and_then(|v| v.as_str()),
        Some(device.id.as_str())
    );

    // Wrong device in the path: 404 (never someone else's key).
    let status = revoke_agent_key(
        State(make_state().await),
        Extension(store.clone()),
        Extension(audit.clone()),
        Extension(auth_user(&uid, "user")),
        Path((device.id.clone(), b1.id.clone())),
    )
    .await
    .expect_err("key of another device");
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);

    // Stranger: 403.
    let status = revoke_agent_key(
        State(make_state().await),
        Extension(store.clone()),
        Extension(audit),
        Extension(auth_user(&other, "user")),
        Path((device.id.clone(), a1.id)),
    )
    .await
    .expect_err("stranger");
    assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
    let _ = dk;
}

// ---- Cascade lifecycle ----

#[tokio::test]
async fn test_device_revoke_and_delete_cascade_to_agent_keys() {
    // Revoke.
    {
        let store = test_store().await;
        let uid = seed_user(&store).await;
        let state = make_state().await;
        let audit = Arc::new(AuditLogger::new(64));
        let device = seed_device(&store, &uid, "Pixel").await;
        let (_a, key) = seed_agent_key(&store, &device, &["traffic:read"]).await;
        let mgr = test_manager().with_store(store.clone());
        mgr.validate_api_key(&key)
            .await
            .expect("valid before revoke");

        revoke_device(
            State(state),
            Extension(store),
            Extension(audit),
            Extension(auth_user(&uid, "user")),
            Path(device.id),
        )
        .await
        .expect("revoke device");

        mgr.validate_api_key(&key)
            .await
            .expect_err("device revocation must kill its agent keys");
    }
    // Delete.
    {
        let store = test_store().await;
        let uid = seed_user(&store).await;
        let state = make_state().await;
        let audit = Arc::new(AuditLogger::new(64));
        let device = seed_device(&store, &uid, "Pixel").await;
        let (_a, key) = seed_agent_key(&store, &device, &["traffic:read"]).await;
        let mgr = test_manager().with_store(store.clone());

        delete_device(
            State(state),
            Extension(store),
            Extension(audit),
            Extension(auth_user(&uid, "user")),
            Path(device.id.clone()),
        )
        .await
        .expect("delete device");

        mgr.validate_api_key(&key)
            .await
            .expect_err("device deletion must kill its agent keys");
    }
}

/// THE rotation-immunity guarantee (issue #108 acceptance): rotating the
/// device key must NOT disturb its agent keys — the binding is the
/// device row, not the key material.
#[tokio::test]
async fn test_device_key_rotation_leaves_agent_keys_working() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let state = make_state().await;
    let audit = Arc::new(AuditLogger::new(64));
    let device = seed_device(&store, &uid, "Pixel").await;

    // Old device key + one agent key.
    let old_dev_key = generate_device_key();
    store
        .create_device_key(&madhyamas_enterprise::store::DeviceKeyRecord {
            id: uuid::Uuid::new_v4().to_string(),
            device_id: device.id.clone(),
            key_hash: hash_api_key(&old_dev_key),
            key_prefix: old_dev_key.chars().take(12).collect(),
            created_at: chrono::Utc::now().to_rfc3339(),
            revoked_at: None,
            last_used_at: None,
        })
        .await
        .expect("persist device key");
    let (agent, agent_key) = seed_agent_key(&store, &device, &["traffic:read"]).await;

    let mgr = test_manager().with_store(store.clone());
    mgr.validate_api_key(&agent_key)
        .await
        .expect("valid before");

    let rotated = rotate_device_key(
        State(state),
        Extension(store.clone()),
        Extension(audit),
        Extension(auth_user(&uid, "user")),
        Path(device.id.clone()),
    )
    .await
    .expect("rotate")
    .0;
    assert!(rotated.key.starts_with("mdy_dev_"));
    assert_ne!(rotated.key, old_dev_key, "a fresh device key was minted");

    // The agent key STILL validates — same row, no revocation.
    let auth = mgr
        .validate_api_key(&agent_key)
        .await
        .expect("agent keys survive device-key rotation (referential binding)");
    assert_eq!(auth.device_id.as_deref(), Some(device.id.as_str()));
    let rows = store.list_agent_keys(&device.id).await.expect("list");
    assert!(
        rows.iter()
            .find(|r| r.id == agent.id)
            .unwrap()
            .revoked_at
            .is_none(),
        "rotation must not revoke agent rows"
    );

    // And the old device key is properly dead while the agent lives.
    mgr.validate_device_key(&old_dev_key)
        .await
        .expect_err("old device key is rotated out");
    mgr.validate_device_key(&rotated.key)
        .await
        .expect("new device key works");
}
