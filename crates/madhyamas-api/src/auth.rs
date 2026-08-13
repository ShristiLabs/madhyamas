//! Trait abstractions for authentication, authorization, and audit logging.
//!
//! These traits allow [`crate::AppState`] to hold optional trait objects
//! (`Option<Arc<dyn Trait + Send + Sync>>`) so the enterprise crate can plug
//! in concrete implementations without `madhyamas-api` depending on
//! enterprise code. In the simple/OSS tier the fields are `None`; in the
//! enterprise tier the main binary constructs implementations and injects
//! them via the [`crate::AppState`] builder methods.
//!
//! The trait method signatures mirror the existing concrete implementations
//! in `madhyamas-core::enterprise` (`AuthManager`, `RbacManager`,
//! `AuditLogger`) so Phase 1b can implement these traits with minimal
//! adapter code.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned by [`AuthProvider`] and [`Authorizer`] operations.
///
/// Mirrors the auth/permission variants of
/// `madhyamas_core::enterprise::EnterpriseError` so the enterprise crate can
/// map its existing errors with `From`.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum AuthError {
    /// Authentication failed (invalid credentials, unknown user, etc.).
    #[error("Authentication failed: {message}")]
    AuthFailed { message: String },

    /// Token has expired.
    #[error("Token expired")]
    TokenExpired,

    /// JWT creation or validation failed.
    #[error("JWT error: {message}")]
    JwtError { message: String },

    /// Authorization check denied access.
    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    /// Referenced user was not found.
    #[error("User not found: {id}")]
    UserNotFound { id: String },

    /// Referenced role was not found.
    #[error("Role not found: {role}")]
    RoleNotFound { role: String },

    /// Invalid configuration supplied to the provider.
    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },
}

/// Error returned by [`AuditSink`] operations.
#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum AuditError {
    /// The audit sink failed to persist or retrieve an event.
    #[error("Audit log error: {message}")]
    LogError { message: String },

    /// A requested event was not found.
    #[error("Audit event not found: {id}")]
    NotFound { id: String },
}

/// How a request was authenticated. Detected from the incoming request by
/// middleware and attached to the resulting [`Identity`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// Username/password login (issued a JWT).
    Password,
    /// `X-API-Key` header.
    ApiKey,
    /// `Authorization: Bearer <jwt>` header.
    Jwt,
    /// OpenID Connect / external IdP token.
    Oidc,
}

/// Authenticated user identity injected into request extensions by auth
/// middleware. Field names match [`madhyamas_core::enterprise::User`] and
/// [`madhyamas_core::enterprise::JwtClaims`] so the enterprise crate can
/// construct this from either source with no translation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// User ID (JWT `sub` or API key owner).
    pub user_id: String,
    /// Login name.
    pub username: String,
    /// Role name (e.g. `admin`, `user`, `viewer`, `readonly`).
    pub role: String,
    /// Email, when known.
    pub email: Option<String>,
    /// Display name, when known.
    pub display_name: Option<String>,
    /// API key ID, when authentication was via API key.
    pub api_key_id: Option<String>,
    /// Session ID, when authentication was via JWT with a session claim.
    pub session_id: Option<String>,
    /// Account status (e.g. `active`, `disabled`), when known.
    pub status: Option<String>,
    /// How the identity was established.
    pub method: AuthMethod,
}

/// Resource type a permission check applies to.
///
/// This is a superset of [`madhyamas_core::enterprise::ResourceType`]: the
/// original engine-level resources plus the enterprise-only resources
/// (`User`, `Audit`, `License`) that the RBAC matrix will govern once the
/// enterprise crate is extracted.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// Captured traffic entries.
    Traffic,
    /// Sessions.
    Session,
    /// Mock rules.
    Mock,
    /// Rewrite rules.
    Rewrite,
    /// Breakpoints.
    Breakpoint,
    /// Scripts.
    Script,
    /// Plugins.
    Plugin,
    /// Proxy/API configuration.
    Config,
    /// User accounts (enterprise).
    User,
    /// Audit log (enterprise).
    Audit,
    /// License management (enterprise).
    License,
}

/// Permission action a role may perform on a [`ResourceType`].
///
/// Mirrors [`madhyamas_core::enterprise::Permission`] with an added `Admin`
/// variant for enterprise-only administrative actions (user management,
/// license management).
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read/list/view a resource.
    Read,
    /// Create/update a resource.
    Write,
    /// Delete a resource.
    Delete,
    /// Execute a resource (scripts, plugins).
    Execute,
    /// Administrative action (enterprise-only: user/license/audit management).
    Admin,
}

/// Audit event type, mirroring
/// [`madhyamas_core::enterprise::AuditEventType`] plus an `Admin` variant
/// for enterprise administrative actions.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// User login.
    Login,
    /// User logout.
    Logout,
    /// API key created.
    ApiKeyCreated,
    /// API key revoked.
    ApiKeyRevoked,
    /// Traffic exported.
    TrafficExported,
    /// Session created.
    SessionCreated,
    /// Session deleted.
    SessionDeleted,
    /// Mock rule created.
    MockCreated,
    /// Mock rule deleted.
    MockDeleted,
    /// Breakpoint created.
    BreakpointCreated,
    /// Breakpoint deleted.
    BreakpointDeleted,
    /// Configuration changed.
    ConfigChanged,
    /// Administrative action (user/license/role management).
    Admin,
    /// Custom event.
    Custom,
}

/// Audit event record. Field names match
/// [`madhyamas_core::enterprise::AuditEvent`] so the enterprise crate can
/// convert between them with `From`/`Into`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID.
    pub id: String,
    /// Event type.
    pub event_type: AuditEventType,
    /// Timestamp (UTC).
    pub timestamp: DateTime<Utc>,
    /// User who performed the action, if authenticated.
    pub user_id: Option<String>,
    /// API key used, if applicable.
    pub api_key_id: Option<String>,
    /// Client IP address, if known.
    pub client_ip: Option<String>,
    /// Human-readable description.
    pub description: String,
    /// Additional structured metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Filter for querying audit events via [`AuditSink::query_events`].
///
/// Mirrors [`madhyamas_core::enterprise::AuditFilter`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditFilter {
    /// Filter by event type.
    pub event_type: Option<AuditEventType>,
    /// Filter by user ID.
    pub user_id: Option<String>,
    /// Filter by start time (inclusive).
    pub start_time: Option<DateTime<Utc>>,
    /// Filter by end time (inclusive).
    pub end_time: Option<DateTime<Utc>>,
    /// Maximum number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

/// Authentication provider trait.
///
/// Enterprise crate implements this with JWT + API key + OIDC backed by
/// `AuthManager`. The simple tier leaves the `AppState` field as `None`
/// (no authentication enforced).
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Validate a JWT bearer token and return the authenticated identity.
    async fn validate_token(&self, token: &str) -> Result<Identity, AuthError>;

    /// Validate an API key and return the authenticated identity.
    async fn validate_api_key(&self, key: &str) -> Result<Identity, AuthError>;

    /// Authenticate a user with username/password and return a JWT.
    async fn authenticate_password(
        &self,
        username: &str,
        password: &str,
    ) -> Result<String, AuthError>;

    /// Generate a JWT for a user (post-authentication or token refresh).
    async fn generate_token(&self, user_id: &str, role: &str) -> Result<String, AuthError>;

    /// Create a new API key for a user. Returns the full key value (shown
    /// once to the caller).
    async fn create_api_key(&self, user_id: &str, name: &str) -> Result<String, AuthError>;

    /// Revoke an API key by its ID.
    async fn revoke_api_key(&self, key_id: &str) -> Result<(), AuthError>;
}

/// Authorization checker trait.
///
/// Enterprise crate implements this with the RBAC matrix backed by
/// `RbacManager`. The simple tier leaves the `AppState` field as `None`
/// (allow-all when no authorizer is configured).
pub trait Authorizer: Send + Sync {
    /// Check if a role has a permission for a resource type. Returns `true`
    /// when allowed.
    fn has_permission(&self, role: &str, resource: ResourceType, permission: Permission) -> bool;

    /// Like [`Authorizer::has_permission`] but returns an [`AuthError`]
    /// (with a descriptive message) when denied, so handlers can convert it
    /// directly into a `403` response.
    fn check_permission(
        &self,
        role: &str,
        resource: ResourceType,
        permission: Permission,
    ) -> Result<(), AuthError> {
        if self.has_permission(role, resource, permission) {
            Ok(())
        } else {
            Err(AuthError::PermissionDenied {
                message: format!("Role '{role}' lacks {:?} on {:?}", permission, resource),
            })
        }
    }

    /// Resolve a role string into a canonical role name, or `None` if the
    /// role is unknown. Used by handlers that need to list/normalize roles.
    fn get_user_role(&self, user_id: &str) -> Option<String>;

    /// List all known role names.
    fn list_roles(&self) -> Vec<String>;
}

/// Audit sink trait.
///
/// Enterprise crate implements this with persistent storage (PostgreSQL in
/// multi-instance, SQLite otherwise) backed by `AuditLogger`. The simple
/// tier leaves the `AppState` field as `None` (audit events are dropped).
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Persist an audit event.
    async fn log_event(&self, event: AuditEvent) -> Result<(), AuditError>;

    /// Query audit events matching the given filter.
    async fn query_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>, AuditError>;

    /// Export audit events matching the given filter as a serialized
    /// payload (e.g. JSON/CSV). The returned bytes are format-specific; the
    /// `content_type` hint indicates the serialization format.
    async fn export_events(&self, filter: &AuditFilter) -> Result<Vec<u8>, AuditError>;
}
