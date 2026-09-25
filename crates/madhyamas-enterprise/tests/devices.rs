//! Integration tests for the public device-principal API (issue #104):
//! per-device credentials (`mdy_dev_...`), device CRUD/key lifecycle in
//! the SQLite store, audit roundtrips for the device event types, and
//! proxy-credential resolution to device principals.

use std::sync::Arc;

use madhyamas_core::{ProxyAuthValidator, ProxyCredentials};
use madhyamas_enterprise::auth::{generate_device_key, hash_api_key, is_device_key};
use madhyamas_enterprise::store::{
    DeviceKeyRecord, DeviceRecord, EnterpriseStore, SqliteEnterpriseStore,
};
use madhyamas_enterprise::{AuditEvent, AuditEventType, AuditFilter};
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

// ---- Credential shape ----

#[test]
fn test_device_key_generator_prefix_length_and_uniqueness() {
    let a = generate_device_key();
    let b = generate_device_key();
    assert!(a.starts_with("mdy_dev_"), "prefix must be mdy_dev_: {a}");
    assert_eq!(a.len(), "mdy_dev_".len() + 32, "32 hex chars of entropy");
    assert_ne!(a, b, "two generated keys must differ");
    assert!(
        a["mdy_dev_".len()..].chars().all(|c| c.is_ascii_hexdigit()),
        "key body must be hex"
    );
}

#[test]
fn test_is_device_key_classification() {
    assert!(is_device_key("mdy_dev_abcdef123"));
    assert!(
        is_device_key("  mdy_dev_padded  "),
        "leading/trailing space is trimmed"
    );
    assert!(!is_device_key("madhyamas_abcdef123"));
    assert!(
        !is_device_key("mdy_agent_abcdef123"),
        "agent keys are a later issue"
    );
    assert!(
        !is_device_key("mdy_dev"),
        "prefix without underscore is not a device key"
    );
    assert!(!is_device_key(""));
    assert!(!is_device_key("basic-password"));
}

// ---- REST rejection: device keys are connect-only ----

#[tokio::test]
async fn test_validate_api_key_rejects_device_keys() {
    let store = test_store().await;
    seed_user(&store).await;
    let mgr = test_manager().with_store(store);

    let key = generate_device_key();
    let err = mgr
        .validate_api_key(&key)
        .await
        .expect_err("device keys must never authenticate the REST API");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("connect"),
        "error should explain the connect-only scope: {msg}"
    );
    assert!(
        !msg.contains(&key),
        "key material must not leak into the error"
    );
}

// ---- validate_device_key ----

#[tokio::test]
async fn test_validate_device_key_resolves_device_and_stamps_last_seen() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Pixel").await;
    let (record, key) = seed_device_key(&store, &device.id).await;

    let auth = mgr.validate_device_key(&key).await.expect("validate");
    assert_eq!(auth.device_id, device.id);
    assert_eq!(auth.owner_user_id, uid);
    assert_eq!(auth.key_id, record.id);

    // The last_seen heartbeat is fire-and-forget; poll briefly for it.
    let mut seen = false;
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        if let Some(d) = store.get_device(&device.id).await.expect("get device") {
            if d.last_seen.is_some() {
                seen = true;
                break;
            }
        }
    }
    assert!(
        seen,
        "device last_seen must be stamped from proxy-auth events"
    );
}

#[tokio::test]
async fn test_validate_device_key_revoked_key_rejected() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Laptop").await;
    let (record, key) = seed_device_key(&store, &device.id).await;
    store
        .revoke_device_key(&record.id)
        .await
        .expect("revoke key");

    let err = mgr
        .validate_device_key(&key)
        .await
        .expect_err("revoked device key must be rejected");
    assert!(!err.to_string().contains(&key), "no key material in error");
}

#[tokio::test]
async fn test_validate_device_key_revoked_device_rejected() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Tablet").await;
    let (_record, key) = seed_device_key(&store, &device.id).await;
    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .expect("revoke keys");
    store
        .update_device_status(&device.id, "revoked")
        .await
        .expect("mark device revoked");

    assert!(mgr.validate_device_key(&key).await.is_err());
}

#[tokio::test]
async fn test_validate_device_key_unknown_rejected() {
    let store = test_store().await;
    seed_user(&store).await;
    let mgr = test_manager().with_store(store);
    assert!(mgr
        .validate_device_key(&generate_device_key())
        .await
        .is_err());
}

#[tokio::test]
async fn test_validate_device_key_requires_store() {
    let mgr = test_manager();
    let result = mgr
        .validate(&ProxyCredentials::ApiKey(generate_device_key()))
        .await;
    assert!(
        result.is_err(),
        "validation without a store must fail closed"
    );
}

// ---- ProxyAuthValidator: device principal resolution ----

#[tokio::test]
async fn test_proxy_validator_device_api_key_resolves_device_principal() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Phone").await;
    let (record, key) = seed_device_key(&store, &device.id).await;

    let principal = mgr
        .validate(&ProxyCredentials::ApiKey(key.clone()))
        .await
        .expect("device key validates at CONNECT");
    assert_eq!(principal.device_id.as_deref(), Some(device.id.as_str()));
    assert_eq!(principal.api_key_id.as_deref(), Some(record.id.as_str()));
    assert!(
        principal.user_id.is_none(),
        "device principals must not fold into the owner's user identity"
    );
    assert!(principal.is_authenticated());
    assert!(
        !format!("{principal:?}").contains(&key),
        "no key material in debug output"
    );
}

#[tokio::test]
async fn test_proxy_validator_basic_auth_key_as_password_resolves_device() {
    // Manual-apply flow (iOS/OEM-Android proxy auth fields): arbitrary
    // username, device key as the password.
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Hari's Pixel").await;
    let (_record, key) = seed_device_key(&store, &device.id).await;

    let principal = mgr
        .validate(&ProxyCredentials::ProxyBasicAuth(format!("device:{key}")))
        .await
        .expect("basic auth with key as password resolves the device");
    assert_eq!(principal.device_id.as_deref(), Some(device.id.as_str()));
    assert!(principal.user_id.is_none());
}

#[tokio::test]
async fn test_proxy_validator_basic_auth_key_as_username_resolves_device() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Watch").await;
    let (_record, key) = seed_device_key(&store, &device.id).await;

    let principal = mgr
        .validate(&ProxyCredentials::ProxyBasicAuth(format!("{key}:anything")))
        .await
        .expect("basic auth with key as username resolves the device");
    assert_eq!(principal.device_id.as_deref(), Some(device.id.as_str()));
}

#[tokio::test]
async fn test_proxy_validator_bearer_device_key_resolves_device() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "TV").await;
    let (_record, key) = seed_device_key(&store, &device.id).await;

    let principal = mgr
        .validate(&ProxyCredentials::ProxyBearer(key))
        .await
        .expect("bearer device key resolves the device");
    assert_eq!(principal.device_id.as_deref(), Some(device.id.as_str()));
}

#[tokio::test]
async fn test_proxy_validator_user_api_key_carries_no_device_id() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let api_key = madhyamas_enterprise::ApiKey::generate(&uid, "user-key");
    let record = madhyamas_enterprise::ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: uid.clone(),
        name: api_key.name.clone(),
        key_hash: hash_api_key(&api_key.key),
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: "[]".to_string(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store
        .create_api_key(&record)
        .await
        .expect("persist api key");

    let principal = mgr
        .validate(&ProxyCredentials::ApiKey(api_key.key.clone()))
        .await
        .expect("user api key still validates");
    assert!(principal.device_id.is_none(), "user keys are not devices");
    assert_eq!(principal.user_id.as_deref(), Some(uid.as_str()));
}

#[tokio::test]
async fn test_proxy_validator_revoked_device_key_rejected_at_connect() {
    // The 407 source: after revocation the validator errors, which the
    // engine maps to ProxyAuthError::Invalid -> 407 in BOTH policy modes.
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let device = seed_device(&store, &uid, "Revoked-phone").await;
    let (_record, key) = seed_device_key(&store, &device.id).await;
    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .expect("revoke");

    assert!(mgr.validate(&ProxyCredentials::ApiKey(key)).await.is_err());
}

// ---- Store: device CRUD ----

#[tokio::test]
async fn test_device_crud_owner_scoping_and_delete() {
    let store = test_store().await;

    let a1 = seed_device(&store, "user-a", "A-phone").await;
    // Second device for the same owner with a later timestamp so the
    // newest-first ordering is observable.
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let a2 = seed_device(&store, "user-a", "A-laptop").await;
    let _b1 = seed_device(&store, "user-b", "B-phone").await;

    let got = store
        .get_device(&a1.id)
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.name, "A-phone");
    assert_eq!(got.owner_user_id, "user-a");
    assert_eq!(got.status, "active");
    assert!(got.last_seen.is_none());

    let list_a = store.list_devices("user-a").await.expect("list");
    assert_eq!(
        list_a.len(),
        2,
        "owner scoping: other users' devices hidden"
    );
    assert_eq!(list_a[0].id, a2.id, "newest first");
    assert_eq!(list_a[1].id, a1.id);

    let list_b = store.list_devices("user-b").await.expect("list");
    assert_eq!(list_b.len(), 1);

    store.delete_device(&a1.id).await.expect("delete");
    assert!(store.get_device(&a1.id).await.expect("get").is_none());
}

#[tokio::test]
async fn test_device_optional_metadata_persisted() {
    let pool = sqlx::SqlitePool::connect(":memory:").await.expect("pool");
    let store = SqliteEnterpriseStore::new(pool).await.expect("store");
    let device = DeviceRecord {
        id: "dev-meta".to_string(),
        name: "Named".to_string(),
        owner_user_id: "u1".to_string(),
        install_uuid: Some("install-uuid-1".to_string()),
        mac_address: Some("aa:bb:cc:dd:ee:ff".to_string()),
        status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_seen: None,
    };
    store.create_device(&device).await.expect("create");
    let got = store
        .get_device("dev-meta")
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.install_uuid.as_deref(), Some("install-uuid-1"));
    assert_eq!(got.mac_address.as_deref(), Some("aa:bb:cc:dd:ee:ff"));
}

#[tokio::test]
async fn test_device_status_and_last_seen_updates() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "D").await;

    store
        .update_device_last_seen(&device.id)
        .await
        .expect("stamp last_seen");
    let got = store
        .get_device(&device.id)
        .await
        .expect("get")
        .expect("present");
    assert!(got.last_seen.is_some());

    store
        .update_device_status(&device.id, "revoked")
        .await
        .expect("update status");
    let got = store
        .get_device(&device.id)
        .await
        .expect("get")
        .expect("present");
    assert_eq!(got.status, "revoked");
}

// ---- Store: device-key lifecycle ----

#[tokio::test]
async fn test_device_key_lookup_revoke_and_last_used() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "D").await;
    let (record, key) = seed_device_key(&store, &device.id).await;

    let got = store
        .get_device_key_by_hash(&hash_api_key(&key))
        .await
        .expect("lookup")
        .expect("present");
    assert_eq!(got.id, record.id);
    assert_eq!(got.device_id, device.id);
    assert!(got.revoked_at.is_none());

    store
        .update_device_key_last_used(&record.id)
        .await
        .expect("stamp last_used");
    let got = store
        .get_device_key_by_hash(&hash_api_key(&key))
        .await
        .expect("lookup")
        .expect("present");
    assert!(got.last_used_at.is_some());

    store.revoke_device_key(&record.id).await.expect("revoke");
    let got = store
        .get_device_key_by_hash(&hash_api_key(&key))
        .await
        .expect("lookup")
        .expect("row kept for audit");
    assert!(
        got.revoked_at.is_some(),
        "revoked keys stay queryable but flagged"
    );
}

#[tokio::test]
async fn test_revoke_device_keys_for_device_revokes_all_active() {
    let store = test_store().await;
    let device = seed_device(&store, "u1", "D").await;
    let (k1, _key1) = seed_device_key(&store, &device.id).await;
    let (k2, _key2) = seed_device_key(&store, &device.id).await;
    // A key of another device must be untouched.
    let other = seed_device(&store, "u2", "Other").await;
    let (ko, _keyo) = seed_device_key(&store, &other.id).await;
    // Pre-revoked key keeps its original revocation timestamp.
    store.revoke_device_key(&k1.id).await.expect("revoke first");
    let pre = store
        .get_device_key_by_hash(&k1.key_hash)
        .await
        .expect("lookup")
        .expect("present")
        .revoked_at
        .clone();

    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .expect("revoke all");

    let k1_after = store
        .get_device_key_by_hash(&k1.key_hash)
        .await
        .expect("lookup")
        .expect("present");
    assert_eq!(
        k1_after.revoked_at, pre,
        "already-revoked key not re-stamped"
    );
    for kh in [&k2.key_hash] {
        let row = store
            .get_device_key_by_hash(kh)
            .await
            .expect("lookup")
            .expect("present");
        assert!(
            row.revoked_at.is_some(),
            "active keys of the device are revoked"
        );
    }
    let ko_after = store
        .get_device_key_by_hash(&ko.key_hash)
        .await
        .expect("lookup")
        .expect("present");
    assert!(
        ko_after.revoked_at.is_none(),
        "other devices' keys untouched"
    );
}

// ---- Audit roundtrip for the new device event types ----

#[tokio::test]
async fn test_audit_device_event_types_roundtrip() {
    let store = test_store().await;
    for (event_type, label) in [
        (AuditEventType::DeviceRegistered, "device_registered"),
        (AuditEventType::DeviceKeyRotated, "device_key_rotated"),
        (AuditEventType::DeviceRevoked, "device_revoked"),
    ] {
        let event = AuditEvent::new(event_type, "device lifecycle");
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
