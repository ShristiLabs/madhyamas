//! Shared test fixtures for the Madhyamas workspace.
//!
//! Consumed as a dev-dependency by integration tests under each crate's
//! `tests/` directory. Two rules keep this crate safe for both build tiers:
//!
//! * `publish = false` — never shipped.
//! * Enterprise (BSL-1.1) fixtures live behind the non-default `enterprise`
//!   feature so the OSS build graph never pulls `madhyamas-enterprise`
//!   (enforced by CI's `cargo tree` check).
//!
//! Per-crate fixtures that build only one crate's types belong in that
//! crate's `tests/common/mod.rs`, not here.

use std::collections::HashMap;
use std::sync::Arc;

use madhyamas_core::secrets::SecretStore;
use madhyamas_core::traffic::TrafficStore;

/// Deterministic 32-byte AES key for keystore/crypto tests.
pub fn test_key() -> Vec<u8> {
    (0u8..32).collect()
}

/// Auto-cleaned temp directory tagged with the calling test group's name.
pub fn tmpdir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("madhyamas-{tag}-"))
        .tempdir()
        .expect("create temp dir")
}

/// In-memory [`SecretStore`] for redaction/substitution/keystore tests.
///
/// Canonical replacement for the per-crate `MemStore` duplicates that lived
/// in madhyamas-core (scripting runtime), madhyamas-api, and
/// madhyamas-enterprise test modules.
pub struct MemStore(std::sync::Mutex<HashMap<String, String>>);

impl MemStore {
    pub fn new() -> Self {
        Self(std::sync::Mutex::new(HashMap::new()))
    }
}

impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MemStore {
    fn load_all(&self) -> madhyamas_core::Result<HashMap<String, String>> {
        Ok(self.0.lock().unwrap().clone())
    }

    fn set(&self, name: &str, value: &str) -> madhyamas_core::Result<()> {
        self.0.lock().unwrap().insert(name.into(), value.into());
        Ok(())
    }

    fn delete(&self, name: &str) -> madhyamas_core::Result<bool> {
        Ok(self.0.lock().unwrap().remove(name).is_some())
    }
}

/// In-memory traffic store shared by session/persistence/auto-save tests.
pub async fn in_memory_traffic_store() -> Arc<TrafficStore> {
    TrafficStore::in_memory()
        .await
        .expect("failed to create in-memory store")
}

/// Spawn a minimal mock HTTP server that captures the raw request text
/// from every connection and returns a JSON `[]` body. Returns the bound
/// URL and an mpsc receiver of raw request texts. Handles multiple
/// connections (e.g. tier detection followed by the test request).
pub async fn spawn_mock_server() -> (String, tokio::sync::mpsc::Receiver<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(16);
    let url = format!("http://{}", addr);

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => break,
            };
            let tx = tx.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let request_text = String::from_utf8_lossy(&buf[..n]).to_string();
                let _ = tx.try_send(request_text);
                let body =
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n[]";
                let _ = socket.write_all(body).await;
                let _ = socket.flush().await;
            });
        }
    });

    (url, rx)
}

/// Fixtures building enterprise (BSL-1.1) types. Feature-gated so the OSS
/// build never resolves `madhyamas-enterprise` through this crate.
#[cfg(feature = "enterprise")]
pub mod enterprise {
    use std::sync::Arc;

    use madhyamas_enterprise::{
        AuthConfig, AuthManager, EnterpriseStore, SqliteEnterpriseStore, User, UserRole, UserStatus,
    };

    /// Auth manager with a deterministic test JWT secret.
    pub fn test_manager() -> AuthManager {
        AuthManager::new(AuthConfig {
            enabled: true,
            jwt_secret: "test-secret-key-for-tests".to_string(),
            jwt_expiration_secs: 3600,
            refresh_token_secs: 7 * 24 * 3600,
            ..AuthConfig::default()
        })
    }

    /// In-memory SQLite enterprise store (users, API keys, audit events).
    pub async fn test_store() -> Arc<dyn EnterpriseStore> {
        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .expect("open in-memory pool");
        Arc::new(SqliteEnterpriseStore::new(pool).await.expect("init store"))
    }

    /// Seed one active admin user; returns its id.
    pub async fn seed_user(store: &Arc<dyn EnterpriseStore>) -> String {
        let user = User::new(
            "u-test".to_string(),
            "testuser".to_string(),
            None,
            UserRole::Admin,
            "testuser".to_string(),
            UserStatus::Active,
        );
        store
            .create_user(&user, "$argon2id$stub")
            .await
            .expect("create user");
        user.id
    }
}
