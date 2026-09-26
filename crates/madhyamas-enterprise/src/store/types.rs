//! Row and helper types for the enterprise store.
//!
//! These types map directly to the SQLite column layout so they can derive
//! [`sqlx::FromRow`] and be used with runtime `sqlx::query_as::<_, T>` calls.
//! The public enterprise domain types ([`crate::User`], [`crate::AuditEvent`])
//! are converted to/from these records inside [`super::sqlite::SqliteEnterpriseStore`].

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::audit::{AuditEvent, AuditEventType};
use crate::user::{User, UserRole, UserStatus};

/// Database row for the `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub display_name: String,
    pub password_hash: String,
    pub role: String,
    pub status: String,
    pub created_at: i64,
    pub last_login: Option<i64>,
    pub preferences: String,
}

impl From<UserRecord> for User {
    fn from(r: UserRecord) -> Self {
        let preferences: HashMap<String, serde_json::Value> =
            serde_json::from_str(&r.preferences).unwrap_or_default();
        Self {
            id: r.id,
            username: r.username,
            email: r.email,
            display_name: r.display_name,
            role: UserRole::from_label(&r.role),
            status: parse_status(&r.status),
            created_at: r.created_at,
            last_login: r.last_login,
            preferences,
        }
    }
}

impl From<&User> for UserRecord {
    fn from(u: &User) -> Self {
        let preferences = serde_json::to_string(&u.preferences).unwrap_or_else(|_| "{}".into());
        Self {
            id: u.id.clone(),
            username: u.username.clone(),
            email: u.email.clone(),
            display_name: u.display_name.clone(),
            password_hash: String::new(),
            role: u.role.as_label().to_string(),
            status: status_label(u.status),
            created_at: u.created_at,
            last_login: u.last_login,
            preferences,
        }
    }
}

fn status_label(status: UserStatus) -> String {
    match status {
        UserStatus::Active => "active".to_string(),
        UserStatus::Inactive => "inactive".to_string(),
        UserStatus::Suspended => "suspended".to_string(),
        UserStatus::PendingVerification => "pending_verification".to_string(),
    }
}

fn parse_status(label: &str) -> UserStatus {
    match label {
        "active" => UserStatus::Active,
        "inactive" => UserStatus::Inactive,
        "suspended" => UserStatus::Suspended,
        "pending_verification" => UserStatus::PendingVerification,
        _ => UserStatus::Active,
    }
}

/// Persisted API key record (distinct from the in-memory [`crate::ApiKey`]
/// which carries the plaintext key). The `key_hash` column stores a hash of
/// the plaintext key; `key_prefix` is a non-secret preview used for display.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKeyRecord {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub scopes: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

/// Persisted authentication session.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuthSession {
    pub id: String,
    pub user_id: String,
    pub jwt_jti: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_activity: String,
    pub revoked: bool,
}

/// Database row for the `devices` table (issue #104). A device is the
/// principal a per-device credential (`mdy_dev_...`) resolves to; its
/// `last_seen` is refreshed from proxy-auth events (device CONNECT).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub owner_user_id: String,
    pub install_uuid: Option<String>,
    pub mac_address: Option<String>,
    /// Lifecycle status: `active` or `revoked`. A revoked device's keys
    /// are deactivated and its credentials no longer authenticate.
    pub status: String,
    pub created_at: String,
    pub last_seen: Option<String>,
}

/// Database row for the `device_keys` table (issue #104). Each row is one
/// per-device credential; `key_hash` stores the SHA-256 of the plaintext
/// `mdy_dev_...` key and `key_prefix` a non-secret preview. Rotation
/// deactivates the old row (`revoked_at` set) and inserts a new one.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DeviceKeyRecord {
    pub id: String,
    pub device_id: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: String,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

/// Database row for the `device_enrollment_tokens` table (issue #106).
/// Each row is one short-lived, single-use enrollment token carried by
/// the onboarding QR code; `token_hash` stores the SHA-256 of the
/// plaintext `mdy_enroll_...` token and `token_prefix` a non-secret
/// preview. A token is redeemable only while unredeemed, unrevoked, and
/// younger than `expires_at` (15 minutes); redemption is an atomic
/// compare-and-set on `redeemed_at`.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EnrollmentTokenRecord {
    pub id: String,
    pub device_id: String,
    pub token_hash: String,
    pub token_prefix: String,
    pub created_at: String,
    pub expires_at: String,
    pub redeemed_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// Database row for the `agent_keys` table (issue #108). Each row is one
/// device-derived agent key: an API credential **referentially** bound
/// to a parent device (`parent_device_id` — the key material is
/// independent random, deliberately NOT derived from the device key, so
/// device-key rotation leaves agents working). `key_hash` stores the
/// SHA-256 of the plaintext `mdy_agent_...` key; `owner_user_id` is
/// denormalized from the device so validation resolves
/// `(user, device, scopes)` with a single row; `scopes` is a JSON array
/// of feature scopes from the issue #107 taxonomy. Revoking the parent
/// device cascades a revoke onto its agent keys; rotating the device
/// key does not.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AgentKeyRecord {
    pub id: String,
    /// The device this key is scoped to (data axis).
    pub parent_device_id: String,
    /// Denormalized device owner (capability/audit axis).
    pub owner_user_id: String,
    /// Optional human label shown in the device's agent list.
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    /// JSON array of feature scopes (issue #107 taxonomy).
    pub scopes: String,
    pub created_at: String,
    /// RFC 3339 expiry, `None` = never expires.
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub last_used_at: Option<String>,
}

/// Database row for the `audit_events` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditEventRecord {
    pub id: String,
    pub event_type: String,
    pub timestamp: String,
    pub user_id: Option<String>,
    pub api_key_id: Option<String>,
    pub client_ip: Option<String>,
    pub description: String,
    pub metadata: String,
    pub prev_hash: Option<String>,
    /// This event's own hash (SHA-256 of canonical fields + prev_hash).
    /// Populated by [`crate::AuditLogger`] before persistence.
    #[sqlx(default)]
    pub hash: Option<String>,
}

impl From<AuditEventRecord> for AuditEvent {
    fn from(r: AuditEventRecord) -> Self {
        let timestamp = DateTime::parse_from_rfc3339(&r.timestamp)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let metadata: HashMap<String, serde_json::Value> =
            serde_json::from_str(&r.metadata).unwrap_or_default();
        Self {
            id: r.id,
            event_type: parse_event_type(&r.event_type),
            timestamp,
            user_id: r.user_id,
            api_key_id: r.api_key_id,
            client_ip: r.client_ip,
            description: r.description,
            metadata,
            prev_hash: r.prev_hash,
            hash: r.hash,
        }
    }
}

impl From<&AuditEvent> for AuditEventRecord {
    fn from(e: &AuditEvent) -> Self {
        let metadata = serde_json::to_string(&e.metadata).unwrap_or_else(|_| "{}".into());
        Self {
            id: e.id.clone(),
            event_type: event_type_label(e.event_type),
            timestamp: e.timestamp.to_rfc3339(),
            user_id: e.user_id.clone(),
            api_key_id: e.api_key_id.clone(),
            client_ip: e.client_ip.clone(),
            description: e.description.clone(),
            metadata,
            prev_hash: e.prev_hash.clone(),
            hash: e.hash.clone(),
        }
    }
}

fn event_type_label(t: AuditEventType) -> String {
    match t {
        AuditEventType::Login => "login".to_string(),
        AuditEventType::Logout => "logout".to_string(),
        AuditEventType::ApiKeyCreated => "api_key_created".to_string(),
        AuditEventType::ApiKeyRevoked => "api_key_revoked".to_string(),
        AuditEventType::DeviceRegistered => "device_registered".to_string(),
        AuditEventType::DeviceKeyRotated => "device_key_rotated".to_string(),
        AuditEventType::DeviceRevoked => "device_revoked".to_string(),
        AuditEventType::DeviceEnrollmentIssued => "device_enrollment_issued".to_string(),
        AuditEventType::DeviceEnrolled => "device_enrolled".to_string(),
        AuditEventType::TrafficExported => "traffic_exported".to_string(),
        AuditEventType::SessionCreated => "session_created".to_string(),
        AuditEventType::SessionDeleted => "session_deleted".to_string(),
        AuditEventType::MockCreated => "mock_created".to_string(),
        AuditEventType::MockDeleted => "mock_deleted".to_string(),
        AuditEventType::BreakpointCreated => "breakpoint_created".to_string(),
        AuditEventType::BreakpointDeleted => "breakpoint_deleted".to_string(),
        AuditEventType::ConfigChanged => "config_changed".to_string(),
        AuditEventType::Custom => "custom".to_string(),
    }
}

fn parse_event_type(label: &str) -> AuditEventType {
    match label {
        "login" => AuditEventType::Login,
        "logout" => AuditEventType::Logout,
        "api_key_created" => AuditEventType::ApiKeyCreated,
        "api_key_revoked" => AuditEventType::ApiKeyRevoked,
        "device_registered" => AuditEventType::DeviceRegistered,
        "device_key_rotated" => AuditEventType::DeviceKeyRotated,
        "device_revoked" => AuditEventType::DeviceRevoked,
        "device_enrollment_issued" => AuditEventType::DeviceEnrollmentIssued,
        "device_enrolled" => AuditEventType::DeviceEnrolled,
        "traffic_exported" => AuditEventType::TrafficExported,
        "session_created" => AuditEventType::SessionCreated,
        "session_deleted" => AuditEventType::SessionDeleted,
        "mock_created" => AuditEventType::MockCreated,
        "mock_deleted" => AuditEventType::MockDeleted,
        "breakpoint_created" => AuditEventType::BreakpointCreated,
        "breakpoint_deleted" => AuditEventType::BreakpointDeleted,
        "config_changed" => AuditEventType::ConfigChanged,
        _ => AuditEventType::Custom,
    }
}

/// Partial user update applied by [`super::EnterpriseStore::update_user`].
/// Only fields set to `Some` are written; `None` fields are left unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserUpdate {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
    pub role: Option<String>,
    pub status: Option<String>,
    pub preferences: Option<String>,
    pub last_login: Option<i64>,
}

/// Aggregate audit statistics returned by [`super::EnterpriseStore::get_audit_stats`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: i64,
    pub events_by_type: HashMap<String, i64>,
    pub events_today: i64,
    pub unique_users: i64,
}
