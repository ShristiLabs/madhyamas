//! User management module

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// User role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    User,
    Viewer,
    ReadOnly,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::Admin
    }
}

impl UserRole {
    /// Check if role has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, &UserRole::Admin)
    }

    /// Check if role can modify traffic
    pub fn can_modify_traffic(&self) -> bool {
        matches!(self, &UserRole::Admin)
    }
    /// Check if role can manage users
    pub fn can_manage_users(&self) -> bool {
        matches!(self, &UserRole::Admin)
    }
}

/// User status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserStatus {
    Active,
    Inactive,
    Suspended,
    PendingVerification,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// User
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: String,
    /// Username
    pub username: String,
    /// Email
    pub email: Option<String>,
    /// Display name
    pub display_name: String,
    /// Role
    pub role: UserRole,
    /// Status
    pub status: UserStatus,
    /// Created at
    pub created_at: i64,
    /// Last login time
    pub last_login: Option<i64>,
    /// Preferences (JSON)
    pub preferences: HashMap<String, serde_json::Value>,
}

impl User {
    /// Create a new user
    pub fn new(
        id: String,
        username: String,
        email: Option<String>,
        role: UserRole,
        display_name: String,
        status: UserStatus,
    ) -> Self {
        Self {
            id,
            username,
            email,
            display_name,
            role,
            status,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            last_login: None,
            preferences: HashMap::new(),
        }
    }

    /// Create admin user
    pub fn create_admin(username: String, email: String) -> User {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            email: Some(email),
            display_name: "Admin".to_string(),
            role: UserRole::Admin,
            status: UserStatus::Active,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            last_login: None,
            preferences: HashMap::new(),
        }
    }

    /// Create viewer user
    pub fn create_viewer(username: String, email: String) -> User {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            username,
            email: Some(email),
            display_name: "Viewer".to_string(),
            role: UserRole::Viewer,
            status: UserStatus::Active,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            last_login: None,
            preferences: HashMap::new(),
        }
    }
}
