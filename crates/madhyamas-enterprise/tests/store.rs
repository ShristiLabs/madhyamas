//! Integration tests for the public PostgreSQL enterprise store API.
//!
//! These require a running PostgreSQL instance (override with
//! `MADHYAMAS_PG_TEST_URL`). They are marked `#[ignore]`; run explicitly:
//! `cargo test --all-features -- --ignored`.

use madhyamas_enterprise::store::{EnterpriseStore, PostgresEnterpriseStore};
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
