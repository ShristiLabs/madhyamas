//! Enterprise secrets storage (#87).
//!
//! Secret values live in the enterprise store (PostgreSQL/SQLite) as
//! AES-256-GCM sealed blobs — the plaintext never reaches the database.
//! Management is RBAC-governed (the `/api/secrets` routes are scope-gated
//! to `config:*` — admin-only in the default role matrix) and every set /
//! delete / grant is forwarded to the enterprise audit trail.

use crate::audit::{AuditEvent, AuditEventType, AuditLogger};
use crate::store::{EnterpriseStore, Result as StoreResult};
use madhyamas_core::secrets::keystore::{resolve_key, seal, unseal};
use madhyamas_core::secrets::service::{
    SecretAuditEvent, SecretAuditSink, SecretStore as CoreSecretStore,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Run a future from a possibly-async context. Handlers run on the
/// multi-threaded tokio runtime, so `block_in_place` + `block_on` is safe;
/// on a non-async caller (no runtime) the future cannot be awaited and the
/// caller's error path applies.
fn block_on_maybe<F: std::future::Future>(fut: F) -> Option<F::Output> {
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::try_current()
            .ok()
            .map(|h| h.block_on(fut))
    })
}

/// Enterprise-backed secret store: sealed values in the enterprise store,
/// master key resolved via the same key-management precedence as the OSS
/// keystore (env var / key file / generated key file).
pub struct EnterpriseSecretStore {
    store: Arc<dyn EnterpriseStore>,
    key: Vec<u8>,
}

impl EnterpriseSecretStore {
    pub fn new(store: Arc<dyn EnterpriseStore>, data_dir: &Path) -> StoreResult<Self> {
        let key = resolve_key(data_dir)
            .map_err(|e| crate::store::StoreError::Serialization(format!("secrets key: {}", e)))?;
        Ok(Self { store, key })
    }
}

impl CoreSecretStore for EnterpriseSecretStore {
    fn load_all(&self) -> madhyamas_core::Result<HashMap<String, String>> {
        let rows: Vec<(String, String, String)> = block_on_maybe(self.store.list_secrets())
            .ok_or_else(|| madhyamas_core::Error::Config("no tokio runtime".into()))?
            .map_err(|e| madhyamas_core::Error::Config(e.to_string()))?;
        let mut out = HashMap::new();
        for (name, nonce, ct) in rows {
            out.insert(name, unseal(&self.key, &nonce, &ct)?);
        }
        Ok(out)
    }

    fn set(&self, name: &str, value: &str) -> madhyamas_core::Result<()> {
        let (nonce, ct) = seal(&self.key, value)?;
        block_on_maybe(self.store.set_secret(name, &nonce, &ct))
            .ok_or_else(|| madhyamas_core::Error::Config("no tokio runtime".into()))?
            .map_err(|e| madhyamas_core::Error::Config(e.to_string()))?;
        Ok(())
    }

    fn delete(&self, name: &str) -> madhyamas_core::Result<bool> {
        block_on_maybe(self.store.delete_secret(name))
            .ok_or_else(|| madhyamas_core::Error::Config("no tokio runtime".into()))?
            .map_err(|e| madhyamas_core::Error::Config(e.to_string()))
    }
}

/// Forwards secret audit events to the enterprise audit trail.
pub struct SecretAuditAdapter {
    logger: Arc<AuditLogger>,
}

impl SecretAuditAdapter {
    pub fn new(logger: Arc<AuditLogger>) -> Self {
        Self { logger }
    }
}

impl SecretAuditSink for SecretAuditAdapter {
    fn record(&self, event: SecretAuditEvent) {
        let mut audit = AuditEvent::new(
            AuditEventType::Custom,
            format!("secret {}: {}", event.action, event.name),
        )
        .with_metadata("action", serde_json::json!(event.action))
        .with_metadata("secret_name", serde_json::json!(event.name))
        .with_metadata("actor", serde_json::json!(event.actor));
        // The value is never part of the audit record — only names/actions.
        if let Some(user) = event.actor.strip_prefix("user:") {
            audit = audit.with_user(user.to_string());
        }
        self.logger.log(audit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use std::sync::Mutex;

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
        let filter = crate::audit::AuditFilter::default();
        let events = logger.query_in_memory(&filter);
        assert!(events.iter().any(|e| e.description.contains("api_token")));
    }

    #[test]
    fn seal_unseal_shared_with_oss_keystore() {
        let key: Vec<u8> = (0u8..32).collect();
        let (n, c) = seal(&key, "enterprise-value").unwrap();
        assert_eq!(unseal(&key, &n, &c).unwrap(), "enterprise-value");
    }

    /// The store impl itself is exercised against SQLite in the store
    /// integration tests; here we only sanity-check the trait object is
    /// constructible with a mock-free path (block_on_maybe without a
    /// runtime returns None instead of panicking).
    #[test]
    fn block_on_maybe_outside_runtime_is_none() {
        let called = Arc::new(Mutex::new(false));
        let c2 = called.clone();
        let fut = async move {
            *c2.lock().unwrap() = true;
        };
        assert!(block_on_maybe(fut).is_none());
        assert!(!*called.lock().unwrap());
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use crate::store::sqlite::SqliteEnterpriseStore;
    use std::str::FromStr;

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
}
