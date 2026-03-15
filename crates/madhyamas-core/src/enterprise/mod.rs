//! Enterprise features module
//!
//! This module provides:
//! - Authentication (API keys, JWT)
//! - Audit logging
//! - Multi-user support
//! - Role-based access control

pub mod auth;
pub mod audit;
pub mod user;
pub mod rbac;
pub mod enterprise_error;

pub use auth::{AuthManager, AuthConfig, ApiKey, JwtClaims};
pub use audit::{AuditLogger, AuditEvent, AuditEventType, AuditFilter};
pub use user::{User, UserRole, UserStatus};
pub use rbac::{RbacManager, Permission, Resource, ResourceType};
pub use enterprise_error::EnterpriseError;
