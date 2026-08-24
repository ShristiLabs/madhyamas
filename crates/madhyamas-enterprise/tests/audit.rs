//! Integration tests for the public audit API: log/query, hash-chain
//! integrity, and tamper detection against the SQLite store.

use std::sync::Arc;

use madhyamas_enterprise::store::EnterpriseStore;
use madhyamas_enterprise::{
    AuditEvent, AuditEventType, AuditFilter, AuditLogger, SqliteEnterpriseStore,
};
use madhyamas_test_utils::enterprise::test_store;

#[tokio::test]
async fn test_log_and_query() {
    let store = test_store().await;
    let logger = AuditLogger::default().with_store(store.clone());

    logger.log(AuditEvent::new(AuditEventType::Login, "user logged in").with_user("u1"));
    logger.log(AuditEvent::new(AuditEventType::Logout, "user logged out").with_user("u1"));
    // Give fire-and-forget spawns time to persist.
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let events = logger
        .query(&AuditFilter {
            limit: Some(100),
            ..Default::default()
        })
        .await;
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_filter_by_user() {
    let store = test_store().await;
    let logger = AuditLogger::default().with_store(store.clone());

    logger.log(AuditEvent::new(AuditEventType::Login, "user1 login").with_user("user1"));
    logger.log(AuditEvent::new(AuditEventType::Login, "user2 login").with_user("user2"));
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let events = logger
        .query(&AuditFilter {
            user_id: Some("user1".to_string()),
            limit: Some(100),
            ..Default::default()
        })
        .await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].user_id.as_deref(), Some("user1"));
}

#[tokio::test]
async fn test_hash_chain() {
    let store = test_store().await;
    let logger = AuditLogger::default().with_store(store.clone());

    logger.log(AuditEvent::new(AuditEventType::Login, "event 1"));
    logger.log(AuditEvent::new(AuditEventType::Logout, "event 2"));
    logger.log(AuditEvent::new(AuditEventType::Custom, "event 3"));
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let ok = logger.verify_hash_chain().await.expect("verify");
    assert!(ok, "hash chain should be intact");
}

#[tokio::test]
async fn test_hash_chain_tamper_detection() {
    let pool = sqlx::SqlitePool::connect(":memory:")
        .await
        .expect("open in-memory pool");
    let store = Arc::new(
        SqliteEnterpriseStore::new(pool.clone())
            .await
            .expect("init store"),
    ) as Arc<dyn EnterpriseStore>;
    let logger = AuditLogger::default().with_store(store.clone());

    logger.log(AuditEvent::new(AuditEventType::Login, "event 1"));
    logger.log(AuditEvent::new(AuditEventType::Logout, "event 2"));
    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    // Tamper: directly modify an event's description in the DB, breaking
    // its hash without updating the chain.
    sqlx::query("UPDATE audit_events SET description = 'tampered' WHERE description = 'event 1'")
        .execute(&pool)
        .await
        .expect("tamper");

    let ok = logger.verify_hash_chain().await.expect("verify");
    assert!(!ok, "tampered chain should be detected");
}
