//! Madhyamas enterprise crate.
//!
//! Holds all enterprise-specific code (authentication, RBAC, audit logging,
//! user management, enterprise API handlers/middleware/routes) licensed under
//! BSL-1.1. The crate depends on [`madhyamas_api`] (for trait abstractions and
//! [`AppState`]) and [`madhyamas_core`] (for shared non-enterprise types) but
//! nothing in those crates depends on this one.
//!
//! The three trait abstractions defined in [`madhyamas_api::auth`] —
//! [`AuthProvider`](madhyamas_api::auth::AuthProvider),
//! [`Authorizer`](madhyamas_api::auth::Authorizer), and
//! [`AuditSink`](madhyamas_api::auth::AuditSink) — are implemented here for
//! the concrete enterprise types ([`AuthManager`], [`RbacManager`],
//! [`AuditLogger`]).

// Beta clippy (2026-08 rollout) fires `double_must_use` on `#[async_trait]`
// store/handler signatures (macro-generated boxed future is `must_use`, and
// so is the returned `Result`). Silenced until the lint behavior stabilizes.
#![allow(clippy::double_must_use, clippy::manual_clamp)]

pub mod audit;
pub mod auth;
pub mod credentials;
pub mod enterprise_error;
pub mod handlers;
pub mod license;
pub mod middleware;
pub mod rbac;
pub mod redis_state;
pub mod router;
pub mod secrets;
pub mod security;
pub mod store;
pub mod user;

pub use audit::{AuditEvent, AuditEventType, AuditFilter, AuditLogger};
pub use auth::{ApiKey, ApiKeyAuth, AuthConfig, AuthManager, JwtClaims, RefreshTokenClaims, Scope};
pub use credentials::{hash_password, validate_password_complexity, verify_password};
pub use enterprise_error::EnterpriseError;
pub use license::{License, LicenseClaims, LicenseError, LicenseFile, LicenseVerifier};
pub use rbac::{Permission, RbacManager, Resource, ResourceType};
pub use redis_state::{
    InstanceInfo, InstanceMetrics, RedisState, RedisTrafficEvent, CHANNEL_CONFIG, CHANNEL_EVENTS,
    CHANNEL_INTERCEPT, CHANNEL_SEATS,
};
pub use router::create_enterprise_router;
pub use secrets::{EnterpriseSecretStore, SecretAuditAdapter};
pub use security::{is_private_ip, validate_callback_url};
pub use store::{ApiKeyRecord, AuditStats, AuthSession, UserUpdate};
pub use store::{EnterpriseStore, PostgresEnterpriseStore, SqliteEnterpriseStore, StoreError};
pub use user::{User, UserRole, UserStatus};

use madhyamas_api::auth::{AuditError, AuthError};
use std::sync::Arc;

/// Enterprise state — constructed by the main binary when enterprise features
/// are enabled.
///
/// Bundles the three core enterprise managers behind [`Arc`] so they can be
/// shared across async tasks and cloned into [`madhyamas_api::AppState`].
pub struct EnterpriseState {
    /// Authentication manager (JWT + API keys).
    pub auth: Arc<AuthManager>,
    /// RBAC manager (role → permission matrix).
    pub rbac: Arc<RbacManager>,
    /// Audit logger (in-memory ring buffer).
    pub audit: Arc<AuditLogger>,
    /// Persistent enterprise store (users, API keys, sessions, audit events).
    pub store: Option<Arc<dyn EnterpriseStore>>,
    /// Verified enterprise license, if one was provided and validated at
    /// startup. `None` means the binary is running in unlicensed enterprise
    /// mode (auth/RBAC/audit still functional; seat-count enforcement and
    /// feature gating arrive in later phases).
    pub license: Option<License>,
    /// Redis cross-instance state coordinator. `None` when `--redis-url` is
    /// not provided (single-instance mode). When set, pub/sub event
    /// broadcasting and license seat tracking are active.
    pub redis: Option<Arc<RedisState>>,
}

impl EnterpriseState {
    /// Construct a new [`EnterpriseState`] with the given [`AuthConfig`].
    pub fn new(config: AuthConfig) -> Self {
        Self {
            auth: Arc::new(AuthManager::new(config)),
            rbac: Arc::new(RbacManager::new()),
            audit: Arc::new(AuditLogger::default()),
            store: None,
            license: None,
            redis: None,
        }
    }

    /// Attach a persistent enterprise store.
    pub fn with_store(mut self, store: Arc<dyn EnterpriseStore>) -> Self {
        self.store = Some(store);
        self
    }

    /// Attach a verified license (or `None` for unlicensed enterprise mode).
    pub fn with_license(mut self, license: Option<License>) -> Self {
        self.license = license;
        self
    }

    /// Attach a Redis cross-instance state coordinator (or `None` for
    /// single-instance mode).
    pub fn with_redis(mut self, redis: Option<Arc<RedisState>>) -> Self {
        self.redis = redis;
        self
    }
}

// ---------------------------------------------------------------------------
// Error conversions: EnterpriseError → API-layer trait errors
// ---------------------------------------------------------------------------

impl From<EnterpriseError> for AuthError {
    fn from(err: EnterpriseError) -> Self {
        match err {
            EnterpriseError::AuthFailed { message } => AuthError::AuthFailed { message },
            EnterpriseError::TokenExpired => AuthError::TokenExpired,
            EnterpriseError::JwtError { message } => AuthError::JwtError { message },
            EnterpriseError::PermissionDenied { message } => {
                AuthError::PermissionDenied { message }
            }
            EnterpriseError::UserNotFound { id } => AuthError::UserNotFound { id },
            EnterpriseError::RoleNotFound { role } => AuthError::RoleNotFound { role },
            EnterpriseError::InvalidConfig { message } => AuthError::InvalidConfig { message },
            EnterpriseError::AuditError { message } => AuthError::AuthFailed { message },
        }
    }
}

impl From<EnterpriseError> for AuditError {
    fn from(err: EnterpriseError) -> Self {
        match err {
            EnterpriseError::AuditError { message } => AuditError::LogError { message },
            other => AuditError::LogError {
                message: other.to_string(),
            },
        }
    }
}
