//! Integration tests for the public enterprise secrets API: the audit
//! adapter, seal/unsealed storage, and the SQLite-backed store round-trip.

use std::str::FromStr;
use std::sync::Arc;

use madhyamas_core::secrets::keystore::{seal, unseal};
use madhyamas_core::secrets::service::{SecretAuditEvent, SecretAuditSink};
use madhyamas_enterprise::store::sqlite::SqliteEnterpriseStore;
use madhyamas_enterprise::store::EnterpriseStore;
use madhyamas_enterprise::{AuditFilter, AuditLogger, SecretAuditAdapter};

#[test]
fn audit_adapter_records_names_not_values() {
    let logger = Arc::new(AuditLogger::new(16));
    let adapter = SecretAuditAdapter::new(logger.clone());
    adapter.record(SecretAuditEvent {
        action: "secret_set".into(),
        name: "api_token".into(),
        actor: "api".into(),
    });
    // AuditLogger keeps an in-memory ring buffer; query it back.
    let filter = AuditFilter::default();
    let events = logger.query_in_memory(&filter);
    assert!(events.iter().any(|e| e.description.contains("api_token")));
}

#[test]
fn seal_unseal_shared_with_oss_keystore() {
    let key: Vec<u8> = (0u8..32).collect();
    let (n, c) = seal(&key, "enterprise-value").unwrap();
    assert_eq!(unseal(&key, &n, &c).unwrap(), "enterprise-value");
}

#[tokio::test]
async fn enterprise_store_secret_round_trip() {
    let opts = sqlx::sqlite::SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .create_if_missing(true);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .unwrap();
    let store = SqliteEnterpriseStore::new(pool).await.unwrap();
    let store: Arc<dyn EnterpriseStore> = Arc::new(store);

    // Values are stored sealed — never plaintext.
    let (nonce, ct) = seal(&(0u8..32).collect::<Vec<u8>>(), "super-secret-value").unwrap();
    assert!(!ct.contains("super-secret-value"));
    store.set_secret("api_token", &nonce, &ct).await.unwrap();
    // Overwrite.
    let (n2, c2) = seal(&(0u8..32).collect::<Vec<u8>>(), "rotated").unwrap();
    store.set_secret("api_token", &n2, &c2).await.unwrap();
    let listed = store.list_secrets().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].0, "api_token");
    assert!(!listed[0].2.contains("rotated"));
    assert!(store.delete_secret("api_token").await.unwrap());
    assert!(!store.delete_secret("api_token").await.unwrap());
    assert!(store.list_secrets().await.unwrap().is_empty());
}
