//! Secret service: in-memory cache over a [`SecretStore`], with an audit
//! hook for the enterprise tier.

use super::redaction::Redactor;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Persistent backend for secret storage (OSS: [`FileKeystore`];
/// enterprise: the enterprise store behind RBAC).
pub trait SecretStore: Send + Sync {
    /// Load all name -> plaintext pairs (called once at startup).
    fn load_all(&self) -> crate::Result<HashMap<String, String>>;
    /// Persist a secret (create or overwrite).
    fn set(&self, name: &str, value: &str) -> crate::Result<()>;
    /// Delete a secret; returns whether it existed.
    fn delete(&self, name: &str) -> crate::Result<bool>;
}

/// Audit event emitted for secret management and access. The enterprise
/// tier forwards these to its audit trail; the OSS tier has no sink.
#[derive(Debug, Clone)]
pub struct SecretAuditEvent {
    /// `secret_set` | `secret_delete` | `secret_granted`
    pub action: String,
    /// Secret name involved (never the value).
    pub name: String,
    /// Who triggered it: a plugin id, script id, or `"api"`.
    pub actor: String,
}

/// Sink for secret audit events (implemented by the enterprise tier).
pub trait SecretAuditSink: Send + Sync {
    fn record(&self, event: SecretAuditEvent);
}

/// In-memory secret service used for substitution, redaction, and the
/// management API.
///
/// Values are cached in memory (substitution is on the plugin/script
/// execution path and must not hit disk or the network per lookup); the
/// backing store is authoritative at rest.
pub struct SecretService {
    store: RwLock<Arc<dyn SecretStore>>,
    values: RwLock<HashMap<String, String>>,
    audit: RwLock<Option<Arc<dyn SecretAuditSink>>>,
}

impl SecretService {
    /// Load all secrets from the backing store into memory.
    pub fn new(store: Arc<dyn SecretStore>) -> crate::Result<Self> {
        let values = store.load_all()?;
        Ok(Self {
            store: RwLock::new(store),
            values: RwLock::new(values),
            audit: RwLock::new(None),
        })
    }

    /// Replace the backing store at runtime (the enterprise tier swaps the
    /// OSS file keystore for the enterprise store once it is constructed)
    /// and reload all values from it. On failure the previous store and
    /// values are kept.
    pub fn swap_store(&self, store: Arc<dyn SecretStore>) -> crate::Result<()> {
        let values = store.load_all()?;
        *self.store.write() = store;
        *self.values.write() = values;
        Ok(())
    }

    /// Attach (or replace) the audit sink.
    pub fn set_audit_sink(&self, sink: Arc<dyn SecretAuditSink>) {
        *self.audit.write() = Some(sink);
    }

    /// Attach an audit sink (enterprise tier).
    pub fn with_audit_sink(self, sink: Arc<dyn SecretAuditSink>) -> Self {
        *self.audit.write() = Some(sink);
        self
    }

    fn emit(&self, action: &str, name: &str, actor: &str) {
        if let Some(sink) = self.audit.read().as_ref() {
            sink.record(SecretAuditEvent {
                action: action.to_string(),
                name: name.to_string(),
                actor: actor.to_string(),
            });
        }
    }

    /// Sorted secret names (safe to expose; values never leave).
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.values.read().keys().cloned().collect();
        names.sort();
        names
    }

    /// Look up a plaintext value. This is the substitution/read path —
    /// callers must have verified the name is granted to the consumer.
    /// Every read emits a `secret_granted` audit event when a sink is
    /// attached.
    pub fn get(&self, name: &str, consumer: &str) -> Option<String> {
        let v = self.values.read().get(name).cloned();
        if v.is_some() {
            self.emit("secret_granted", name, consumer);
        }
        v
    }

    /// Create or update a secret (management API, admin-gated upstream).
    /// Emits a `secret_set` audit event; the value is never included.
    pub fn set(&self, name: &str, value: &str) -> crate::Result<()> {
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(crate::Error::Config(
                "secret names must be non-empty and contain only [A-Za-z0-9_-]".into(),
            ));
        }
        self.store.read().clone().set(name, value)?;
        self.values
            .write()
            .insert(name.to_string(), value.to_string());
        self.emit("secret_set", name, "api");
        Ok(())
    }

    /// Delete a secret. Emits a `secret_delete` audit event on success.
    pub fn delete(&self, name: &str) -> bool {
        match self.store.read().clone().delete(name) {
            Ok(true) => {
                self.values.write().remove(name);
                self.emit("secret_delete", name, "api");
                true
            }
            _ => false,
        }
    }

    /// Build a [`Redactor`] over the current secret values plus the given
    /// configured header patterns.
    pub fn redactor(&self, header_patterns: &[String]) -> Redactor {
        let values: Vec<String> = self.values.read().values().cloned().collect();
        Redactor::new(header_patterns, values)
    }

    /// Snapshot of all current secret values (used to build the shared
    /// redactor's value-matching set).
    pub fn value_snapshot(&self) -> Vec<String> {
        self.values.read().values().cloned().collect()
    }

    /// Whether the service holds any secrets (used to skip redaction work
    /// when the store is empty).
    pub fn is_empty(&self) -> bool {
        self.values.read().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MemStore {
        entries: Mutex<HashMap<String, String>>,
        fail: bool,
    }

    impl MemStore {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                entries: Mutex::new(HashMap::new()),
                fail: false,
            })
        }
    }

    impl SecretStore for MemStore {
        fn load_all(&self) -> crate::Result<HashMap<String, String>> {
            Ok(self.entries.lock().unwrap().clone())
        }
        fn set(&self, name: &str, value: &str) -> crate::Result<()> {
            if self.fail {
                return Err(crate::Error::Config("store down".into()));
            }
            self.entries
                .lock()
                .unwrap()
                .insert(name.to_string(), value.to_string());
            Ok(())
        }
        fn delete(&self, name: &str) -> crate::Result<bool> {
            Ok(self.entries.lock().unwrap().remove(name).is_some())
        }
    }

    #[derive(Default)]
    struct AuditCapture {
        events: Mutex<Vec<SecretAuditEvent>>,
    }

    impl SecretAuditSink for AuditCapture {
        fn record(&self, e: SecretAuditEvent) {
            self.events.lock().unwrap().push(e);
        }
    }

    #[test]
    fn set_get_delete_lifecycle() {
        let store = MemStore::new();
        let svc = SecretService::new(store.clone()).unwrap();
        assert!(svc.is_empty());
        svc.set("api_token", "tok").unwrap();
        assert_eq!(svc.names(), vec!["api_token".to_string()]);
        assert_eq!(svc.get("api_token", "plugin-a").as_deref(), Some("tok"));
        assert!(svc.get("missing", "plugin-a").is_none());
        assert!(svc.delete("api_token"));
        assert!(!svc.delete("api_token"));
        assert!(svc.names().is_empty());
        // Persisted to the backing store too.
        assert!(store.entries.lock().unwrap().is_empty());
    }

    #[test]
    fn invalid_names_rejected() {
        let svc = SecretService::new(MemStore::new()).unwrap();
        assert!(svc.set("", "v").is_err());
        assert!(svc.set("bad name", "v").is_err());
        assert!(svc.set("ok_name-1", "v").is_ok());
    }

    #[test]
    fn audit_events_emitted() {
        let audit = Arc::new(AuditCapture::default());
        let svc = SecretService::new(MemStore::new())
            .unwrap()
            .with_audit_sink(audit.clone());
        svc.set("s", "value-1234").unwrap();
        svc.get("s", "plugin-b").unwrap();
        svc.delete("s");
        let events = audit.events.lock().unwrap();
        let actions: Vec<&str> = events.iter().map(|e| e.action.as_str()).collect();
        assert_eq!(
            actions,
            vec!["secret_set", "secret_granted", "secret_delete"]
        );
        assert!(events
            .iter()
            .all(|e| !format!("{:?}", e).contains("value-1234")));
        assert_eq!(events[1].actor, "plugin-b");
    }

    #[test]
    fn failed_read_of_missing_secret_no_audit() {
        let audit = Arc::new(AuditCapture::default());
        let svc = SecretService::new(MemStore::new())
            .unwrap()
            .with_audit_sink(audit.clone());
        svc.get("nope", "plugin");
        assert!(audit.events.lock().unwrap().is_empty());
    }

    #[test]
    fn redactor_built_from_values() {
        let svc = SecretService::new(MemStore::new()).unwrap();
        svc.set("t", "abcdef").unwrap();
        let r = svc.redactor(&[]);
        assert_eq!(r.redact_text("x abcdef y"), "x [REDACTED] y");
    }

    #[test]
    fn loads_existing_from_store() {
        let store = MemStore::new();
        store.set("pre", "existing").unwrap();
        let svc = SecretService::new(store).unwrap();
        assert_eq!(svc.get("pre", "t").as_deref(), Some("existing"));
    }
}
