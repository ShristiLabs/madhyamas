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
    use std::sync::Mutex;

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
