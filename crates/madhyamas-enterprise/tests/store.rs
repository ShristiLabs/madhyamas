//! Integration tests for the public PostgreSQL enterprise store API.
//!
//! These require a running PostgreSQL instance (override with
//! `MADHYAMAS_PG_TEST_URL`). They are marked `#[ignore]`; run explicitly:
//! `cargo test --all-features -- --ignored`.

use madhyamas_enterprise::store::{
    DeviceKeyRecord, DeviceRecord, EnterpriseStore, PostgresEnterpriseStore,
};
use madhyamas_enterprise::{
    ApiKeyRecord, AuditEvent, AuditEventType, AuditFilter, User, UserRole, UserStatus,
};

async fn make_store() -> PostgresEnterpriseStore {
    let url = std::env::var("MADHYAMAS_PG_TEST_URL")
        .unwrap_or_else(|_| "postgres://madhyamas:testpass@localhost:5432/madhyamas".to_string());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("failed to connect to PostgreSQL test instance");
    PostgresEnterpriseStore::new(pool)
        .await
        .expect("failed to create PostgresEnterpriseStore")
}

#[tokio::test]
#[ignore]
async fn test_pg_enterprise_user_crud() {
    let store = make_store().await;
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = User::new(
        user_id.clone(),
        format!("testuser_{}", &user_id[..8]),
        Some("test@example.com".to_string()),
        UserRole::Admin,
        "Test User".to_string(),
        UserStatus::Active,
    );
    store
        .create_user(&user, "hashed_password_123")
        .await
        .unwrap();

    let fetched = store.get_user(&user_id).await.unwrap().unwrap();
    assert_eq!(fetched.username, user.username);
    assert_eq!(fetched.role, UserRole::Admin);

    let by_username = store
        .get_user_by_username(&user.username)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_username.id, user_id);

    store.delete_user(&user_id).await.unwrap();
    assert!(store.get_user(&user_id).await.unwrap().is_none());
}

#[tokio::test]
#[ignore]
async fn test_pg_enterprise_audit_log() {
    let store = make_store().await;
    let mut event = AuditEvent::new(AuditEventType::Login, "User logged in");
    event.hash = Some("abc123".to_string());
    store.log_audit_event(&event).await.unwrap();

    let filter = AuditFilter {
        event_type: Some(AuditEventType::Login),
        ..Default::default()
    };
    let events = store.query_audit_events(&filter).await.unwrap();
    assert!(!events.is_empty());

    let latest = store.get_latest_audit_hash().await.unwrap();
    assert_eq!(latest, Some("abc123".to_string()));
}

#[tokio::test]
#[ignore]
async fn test_pg_enterprise_api_key() {
    let store = make_store().await;
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = User::new(
        user_id.clone(),
        format!("apikeyuser_{}", &user_id[..8]),
        None,
        UserRole::Viewer,
        "API Key User".to_string(),
        UserStatus::Active,
    );
    store.create_user(&user, "hash").await.unwrap();

    let key = ApiKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        user_id: user_id.clone(),
        name: "test-key".to_string(),
        key_hash: format!("hash_{}", uuid::Uuid::new_v4()),
        key_prefix: "mk_test".to_string(),
        scopes: "[]".to_string(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_api_key(&key).await.unwrap();

    let fetched = store
        .get_api_key_by_hash(&key.key_hash)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.name, "test-key");

    let keys = store.list_api_keys(&user_id).await.unwrap();
    assert!(!keys.is_empty());

    store.revoke_api_key(&key.id).await.unwrap();
    assert!(store
        .get_api_key_by_hash(&key.key_hash)
        .await
        .unwrap()
        .is_none());

    store.delete_user(&user_id).await.unwrap();
}

/// Device + device-key lifecycle on PostgreSQL (issue #104): register,
/// owner-scoped list, key hash lookup, revocation (single + cascade),
/// last-seen stamp, delete.
#[tokio::test]
#[ignore]
async fn test_pg_enterprise_devices() {
    let store = make_store().await;
    let owner = format!("owner_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);

    let device = DeviceRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: "PG test phone".to_string(),
        owner_user_id: owner.clone(),
        install_uuid: None,
        mac_address: None,
        status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_seen: None,
    };
    store.create_device(&device).await.unwrap();

    let got = store.get_device(&device.id).await.unwrap().unwrap();
    assert_eq!(got.name, "PG test phone");
    assert_eq!(got.owner_user_id, owner);

    let listed = store.list_devices(&owner).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, device.id);

    let key = DeviceKeyRecord {
        id: uuid::Uuid::new_v4().to_string(),
        device_id: device.id.clone(),
        key_hash: format!("devhash_{}", uuid::Uuid::new_v4()),
        key_prefix: "mdy_dev_".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        revoked_at: None,
        last_used_at: None,
    };
    store.create_device_key(&key).await.unwrap();

    let fetched_key = store
        .get_device_key_by_hash(&key.key_hash)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched_key.device_id, device.id);

    store.update_device_last_seen(&device.id).await.unwrap();
    assert!(store
        .get_device(&device.id)
        .await
        .unwrap()
        .unwrap()
        .last_seen
        .is_some());

    store
        .revoke_device_keys_for_device(&device.id)
        .await
        .unwrap();
    let revoked_key = store
        .get_device_key_by_hash(&key.key_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(revoked_key.revoked_at.is_some());

    store
        .update_device_status(&device.id, "revoked")
        .await
        .unwrap();
    assert_eq!(
        store.get_device(&device.id).await.unwrap().unwrap().status,
        "revoked"
    );

    store.delete_device(&device.id).await.unwrap();
    assert!(store.get_device(&device.id).await.unwrap().is_none());
}

/// Enrollment-token lifecycle on PostgreSQL (issue #106): issue (hashed at
/// rest), single-use redemption, TTL enforcement, revocation cascade, and
/// expired-row pruning.
#[tokio::test]
#[ignore]
async fn test_pg_enterprise_enrollment_tokens() {
    use madhyamas_enterprise::auth::{generate_enrollment_token, hash_api_key};
    use madhyamas_enterprise::store::EnrollmentTokenRecord;

    let store = make_store().await;
    let owner = format!("owner_{}", &uuid::Uuid::new_v4().simple().to_string()[..8]);
    let device = DeviceRecord {
        id: uuid::Uuid::new_v4().to_string(),
        name: "PG enroll phone".to_string(),
        owner_user_id: owner,
        install_uuid: None,
        mac_address: None,
        status: "active".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        last_seen: None,
    };
    store.create_device(&device).await.unwrap();

    let seed = |device_id: &str, expires_at: chrono::DateTime<chrono::Utc>| {
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
        (record, token)
    };

    // Valid token: redeems once, never twice.
    let (valid, _t) = seed(
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    );
    store.create_enrollment_token(&valid).await.unwrap();
    let now = chrono::Utc::now().to_rfc3339();
    assert!(store
        .redeem_enrollment_token(&valid.token_hash, &now)
        .await
        .unwrap());
    assert!(!store
        .redeem_enrollment_token(&valid.token_hash, &now)
        .await
        .unwrap());
    let fetched = store
        .get_enrollment_token_by_hash(&valid.token_hash)
        .await
        .unwrap()
        .unwrap();
    assert!(fetched.redeemed_at.is_some());
    assert_ne!(fetched.token_hash, _t, "hash at rest, not plaintext");

    // Expired token: rejected.
    let (expired, _t) = seed(
        &device.id,
        chrono::Utc::now() - chrono::Duration::minutes(1),
    );
    store.create_enrollment_token(&expired).await.unwrap();
    assert!(!store
        .redeem_enrollment_token(&expired.token_hash, &now)
        .await
        .unwrap());

    // Revocation cascade blocks redemption of an outstanding token.
    let (revoked, _t) = seed(
        &device.id,
        chrono::Utc::now() + chrono::Duration::minutes(15),
    );
    store.create_enrollment_token(&revoked).await.unwrap();
    store
        .revoke_enrollment_tokens_for_device(&device.id)
        .await
        .unwrap();
    assert!(!store
        .redeem_enrollment_token(&revoked.token_hash, &now)
        .await
        .unwrap());

    // Pruning removes only the expired row; future-expiry rows (redeemed
    // or revoked) stay.
    let removed = store
        .delete_expired_enrollment_tokens(&chrono::Utc::now().to_rfc3339())
        .await
        .unwrap();
    assert_eq!(removed, 1, "only the expired row is pruned");
}
