//! Enterprise features module
//!
//! This module provides:
//! - Authentication (API keys, JWT)
//! - Audit logging
//! - Multi-user support
//! - Role-based access control

pub mod audit;
pub mod auth;
pub mod enterprise_error;
pub mod rbac;
pub mod user;

pub use audit::{AuditEvent, AuditEventType, AuditFilter, AuditLogger};
pub use auth::{ApiKey, AuthConfig, AuthManager, JwtClaims};
pub use enterprise_error::EnterpriseError;
pub use rbac::{Permission, RbacManager, Resource, ResourceType};
pub use user::{User, UserRole, UserStatus};
