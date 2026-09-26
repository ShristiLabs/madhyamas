//! Enterprise storage trait and SQLite implementation.
//!
//! Defines the [`EnterpriseStore`] async trait (the storage abstraction for
//! users, API keys, auth sessions, and audit events) and a concrete
//! [`SqliteEnterpriseStore`] backed by [`sqlx::SqlitePool`]. The trait mirrors
//! the signature in `docs/ENTERPRISE_STORAGE_TRAITS.md` §1.10 and is the
//! pattern the core rusqlite → sqlx migration (Phase 2c) will follow.
//!
//! All SQL uses runtime `sqlx::query` / `sqlx::query_as::<_, T>` strings (not
//! the compile-time `query!` macro) so the crate builds without a database at
//! build time.

pub mod postgres;
pub mod sqlite;
pub mod types;

pub use postgres::PostgresEnterpriseStore;
pub use sqlite::SqliteEnterpriseStore;
pub use types::{
    AgentKeyRecord, ApiKeyRecord, AuditEventRecord, AuditStats, AuthSession, DeviceKeyRecord,
    DeviceRecord, EnrollmentTokenRecord, UserRecord, UserUpdate,
};

use async_trait::async_trait;

use crate::audit::{AuditEvent, AuditFilter};
use crate::user::User;

/// Error returned by [`EnterpriseStore`] implementations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for StoreError {
    fn from(err: serde_json::Error) -> Self {
        StoreError::Serialization(err.to_string())
    }
}

/// Convenience `Result` alias for store operations.
pub type Result<T> = std::result::Result<T, StoreError>;

/// Async storage trait for enterprise data (users, API keys, auth sessions,
/// audit events). Implemented by [`SqliteEnterpriseStore`]; a PostgreSQL
/// backend (`PgEnterpriseStore`) is deferred to Phase 5.
#[async_trait]
pub trait EnterpriseStore: Send + Sync {
    async fn create_user(&self, user: &User, password_hash: &str) -> Result<()>;
    async fn get_user(&self, id: &str) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    /// Fetch a user by username together with their stored password hash.
    /// Used by the login handler to verify credentials without exposing the
    /// `password_hash` column on the public [`User`] type.
    async fn get_user_credentials(&self, username: &str) -> Result<Option<(User, String)>>;
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn update_user(&self, id: &str, updates: &UserUpdate) -> Result<()>;
    async fn delete_user(&self, id: &str) -> Result<()>;

    async fn create_api_key(&self, key: &ApiKeyRecord) -> Result<()>;
    async fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>>;
    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>>;
    async fn revoke_api_key(&self, id: &str) -> Result<()>;
    async fn update_api_key_last_used(&self, id: &str) -> Result<()>;

    /// Register a device principal (issue #104).
    async fn create_device(&self, device: &DeviceRecord) -> Result<()>;
    /// Fetch a device by ID.
    async fn get_device(&self, id: &str) -> Result<Option<DeviceRecord>>;
    /// List devices owned by `owner_user_id`, newest first.
    async fn list_devices(&self, owner_user_id: &str) -> Result<Vec<DeviceRecord>>;
    /// Delete a device row (its keys should be revoked first).
    async fn delete_device(&self, id: &str) -> Result<()>;
    /// Update a device's lifecycle status (`active` / `revoked`).
    async fn update_device_status(&self, id: &str, status: &str) -> Result<()>;
    /// Stamp a device's `last_seen` to now — called from proxy-auth
    /// events (device CONNECT).
    async fn update_device_last_seen(&self, id: &str) -> Result<()>;
    /// Persist a per-device credential (hash + prefix; the plaintext is
    /// shown once at creation and never stored).
    async fn create_device_key(&self, key: &DeviceKeyRecord) -> Result<()>;
    /// Look up a device credential by its key hash.
    async fn get_device_key_by_hash(&self, hash: &str) -> Result<Option<DeviceKeyRecord>>;
    /// Deactivate a device credential (rotation/revocation).
    async fn revoke_device_key(&self, id: &str) -> Result<()>;
    /// Deactivate all of a device's credentials at once (device
    /// revocation/deletion).
    async fn revoke_device_keys_for_device(&self, device_id: &str) -> Result<()>;
    /// Stamp a device credential's `last_used_at` to now.
    async fn update_device_key_last_used(&self, id: &str) -> Result<()>;

    /// Persist an enrollment token (hash + prefix; the plaintext rides in
    /// the QR payload and is never stored) (issue #106).
    async fn create_enrollment_token(&self, token: &EnrollmentTokenRecord) -> Result<()>;
    /// Look up an enrollment token row by its token hash (used to resolve
    /// the device after a successful redemption).
    async fn get_enrollment_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<EnrollmentTokenRecord>>;
    /// Atomically redeem an enrollment token: stamps `redeemed_at` only
    /// when the token is still unredeemed, unrevoked, and unexpired at
    /// `now` (RFC 3339). Returns `true` when this call performed the
    /// redemption — `false` means the token was already used, revoked,
    /// expired, or unknown (single-use enforcement, issue #106).
    async fn redeem_enrollment_token(&self, token_hash: &str, now: &str) -> Result<bool>;
    /// Deactivate all of a device's outstanding enrollment tokens (device
    /// revocation/deletion cascade).
    async fn revoke_enrollment_tokens_for_device(&self, device_id: &str) -> Result<()>;
    /// Delete enrollment tokens that expired before `now` (RFC 3339);
    /// returns the number of rows removed (opportunistic cleanup on
    /// issuance, keeps the table bounded).
    async fn delete_expired_enrollment_tokens(&self, now: &str) -> Result<u64>;

    /// Persist a device-derived agent key (hash + prefix; the plaintext
    /// is shown once at creation and never stored) (issue #108).
    async fn create_agent_key(&self, key: &AgentKeyRecord) -> Result<()>;
    /// Look up an agent key row by its key hash (validation path).
    async fn get_agent_key_by_hash(&self, key_hash: &str) -> Result<Option<AgentKeyRecord>>;
    /// List a device's agent keys, newest first (metadata only — the
    /// caller must never return the hash).
    async fn list_agent_keys(&self, parent_device_id: &str) -> Result<Vec<AgentKeyRecord>>;
    /// Deactivate one agent key (single-agent revoke; leaves the device
    /// and sibling agents untouched).
    async fn revoke_agent_key(&self, id: &str) -> Result<()>;
    /// Deactivate all of a device's agent keys at once (device
    /// revocation/deletion cascade). Device-key ROTATION must NOT call
    /// this — the binding is referential, not key-material.
    async fn revoke_agent_keys_for_device(&self, parent_device_id: &str) -> Result<()>;
    /// Stamp an agent key's `last_used_at` to now.
    async fn update_agent_key_last_used(&self, id: &str) -> Result<()>;

    async fn create_session(&self, session: &AuthSession) -> Result<()>;
    async fn get_session(&self, id: &str) -> Result<Option<AuthSession>>;
    async fn revoke_session(&self, id: &str) -> Result<()>;
    async fn cleanup_expired_sessions(&self) -> Result<()>;
    /// Update the `last_activity` timestamp of a session to the current time.
    /// Used by the auth middleware to track idle timeout (Phase 4b.8).
    async fn update_session_activity(&self, session_id: &str) -> Result<()>;

    async fn log_audit_event(&self, event: &AuditEvent) -> Result<()>;
    async fn query_audit_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>>;
    async fn get_audit_stats(&self) -> Result<AuditStats>;
    async fn clear_audit_events(&self) -> Result<()>;
    /// Fetch the `hash` of the most recently logged audit event, or `None`
    /// if the table is empty. Used by the hash-chain computation in
    /// [`crate::AuditLogger`].
    async fn get_latest_audit_hash(&self) -> Result<Option<String>>;

    /// Persist a secret (create or overwrite). `nonce`/`ciphertext` are the
    /// AES-256-GCM sealed hex pair produced by the keystore helpers; the
    /// plaintext never reaches the store (issue #87).
    async fn set_secret(&self, name: &str, nonce: &str, ciphertext: &str) -> Result<()>;
    /// Delete a secret; returns whether it existed.
    async fn delete_secret(&self, name: &str) -> Result<bool>;
    /// List all secrets as (name, nonce, ciphertext) triples (still sealed).
    async fn list_secrets(&self) -> Result<Vec<(String, String, String)>>;
}
