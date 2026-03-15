//! Enterprise error types

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Error, Serialize, Deserialize)]
pub enum EnterpriseError {
    #[error("Authentication failed: {message}")]
    AuthFailed { message: String },

    #[error("Token expired: {token}")]
    TokenExpired { token: String },

    #[error("JWT creation failed: {message}")]
    JwtError { message: String },

    #[error("Permission denied: {message}")]
    PermissionDenied { message: String },

    #[error("User not found: {id}")]
    UserNotFound { id: String },

    #[error("Audit log error: {message}")]
    AuditError { message: String },

    #[error("Role not found: {role}")]
    RoleNotFound { role: String },

    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },
}
