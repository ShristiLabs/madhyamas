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

pub mod audit;
pub mod auth;
pub mod enterprise_error;
pub mod handlers;
pub mod middleware;
pub mod rbac;
pub mod router;
pub mod user;

pub use audit::{AuditEvent, AuditEventType, AuditFilter, AuditLogger};
pub use auth::{ApiKey, AuthConfig, AuthManager, JwtClaims};
pub use enterprise_error::EnterpriseError;
pub use rbac::{Permission, RbacManager, Resource, ResourceType};
pub use router::create_enterprise_router;
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
}

impl EnterpriseState {
    /// Construct a new [`EnterpriseState`] with the given [`AuthConfig`].
    pub fn new(config: AuthConfig) -> Self {
        Self {
            auth: Arc::new(AuthManager::new(config)),
            rbac: Arc::new(RbacManager::new()),
            audit: Arc::new(AuditLogger::default()),
        }
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
