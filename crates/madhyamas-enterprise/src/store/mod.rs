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

pub mod sqlite;
pub mod types;

pub use sqlite::SqliteEnterpriseStore;
pub use types::{ApiKeyRecord, AuditEventRecord, AuditStats, AuthSession, UserRecord, UserUpdate};

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
    async fn create_user(&self, user: &User) -> Result<()>;
    async fn get_user(&self, id: &str) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn update_user(&self, id: &str, updates: &UserUpdate) -> Result<()>;
    async fn delete_user(&self, id: &str) -> Result<()>;

    async fn create_api_key(&self, key: &ApiKeyRecord) -> Result<()>;
    async fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>>;
    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>>;
    async fn revoke_api_key(&self, id: &str) -> Result<()>;
    async fn update_api_key_last_used(&self, id: &str) -> Result<()>;

    async fn create_session(&self, session: &AuthSession) -> Result<()>;
    async fn get_session(&self, id: &str) -> Result<Option<AuthSession>>;
    async fn revoke_session(&self, id: &str) -> Result<()>;
    async fn cleanup_expired_sessions(&self) -> Result<()>;

    async fn log_audit_event(&self, event: &AuditEvent) -> Result<()>;
    async fn query_audit_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>>;
    async fn get_audit_stats(&self) -> Result<AuditStats>;
    async fn clear_audit_events(&self) -> Result<()>;
}
