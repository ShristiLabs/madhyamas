//! Role-Based access control module

use std::collections::{HashMap, HashSet};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::user::UserRole;

/// Resource type
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ResourceType {
    Traffic,
    Session,
    Mock,
    Rewrite,
    Breakpoint,
    Script,
    Plugin,
    Config,
}

/// Permission action
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum Permission {
    Read,
    Write,
    Delete,
    Execute,
}

/// Resource definition
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Resource {
    /// Resource type
    pub resource_type: ResourceType,
    /// Resource ID
    pub id: String,
    /// Resource name
    pub name: String,
}

impl Resource {
    /// Create a new resource
    pub fn new(id: String, name: String, resource_type: ResourceType) -> Self {
        Self {
            resource_type,
            id,
            name,
        }
    }
}

/// Role-based access control manager
#[derive(Debug)]
pub struct RbacManager {
    /// Role permissions mapping
    role_permissions: RwLock<HashMap<UserRole, HashSet<(ResourceType, Permission)>>>,
}

impl Default for RbacManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RbacManager {
    /// Create a new RBAC manager
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        // Admin has all permissions
        let admin_perms: HashSet<(ResourceType, Permission)> = [
            (ResourceType::Traffic, Permission::Read),
            (ResourceType::Traffic, Permission::Write),
            (ResourceType::Traffic, Permission::Delete),
            (ResourceType::Session, Permission::Read),
            (ResourceType::Session, Permission::Write),
            (ResourceType::Session, Permission::Delete),
            (ResourceType::Mock, Permission::Read),
            (ResourceType::Mock, Permission::Write),
            (ResourceType::Mock, Permission::Delete),
            (ResourceType::Rewrite, Permission::Read),
            (ResourceType::Rewrite, Permission::Write),
            (ResourceType::Rewrite, Permission::Delete),
            (ResourceType::Breakpoint, Permission::Read),
            (ResourceType::Breakpoint, Permission::Write),
            (ResourceType::Breakpoint, Permission::Delete),
            (ResourceType::Script, Permission::Read),
            (ResourceType::Script, Permission::Write),
            (ResourceType::Script, Permission::Delete),
            (ResourceType::Script, Permission::Execute),
            (ResourceType::Plugin, Permission::Read),
            (ResourceType::Plugin, Permission::Execute),
            (ResourceType::Config, Permission::Read),
            (ResourceType::Config, Permission::Write),
        ]
        .iter()
        .cloned()
        .collect();
        role_permissions.insert(UserRole::Admin, admin_perms);

        // User has read/write permissions
        let user_perms: HashSet<(ResourceType, Permission)> = [
            (ResourceType::Traffic, Permission::Read),
            (ResourceType::Traffic, Permission::Write),
            (ResourceType::Session, Permission::Read),
            (ResourceType::Session, Permission::Write),
            (ResourceType::Mock, Permission::Read),
            (ResourceType::Mock, Permission::Write),
            (ResourceType::Rewrite, Permission::Read),
            (ResourceType::Rewrite, Permission::Write),
            (ResourceType::Breakpoint, Permission::Read),
            (ResourceType::Breakpoint, Permission::Write),
            (ResourceType::Script, Permission::Read),
            (ResourceType::Script, Permission::Execute),
        ]
        .iter()
        .cloned()
        .collect();
        role_permissions.insert(UserRole::User, user_perms);

        // Viewer has read-only permissions
        let viewer_perms: HashSet<(ResourceType, Permission)> = [
            (ResourceType::Traffic, Permission::Read),
            (ResourceType::Session, Permission::Read),
            (ResourceType::Mock, Permission::Read),
            (ResourceType::Rewrite, Permission::Read),
            (ResourceType::Breakpoint, Permission::Read),
            (ResourceType::Script, Permission::Read),
            (ResourceType::Plugin, Permission::Read),
        ]
        .iter()
        .cloned()
        .collect();
        role_permissions.insert(UserRole::Viewer, viewer_perms.clone());
        role_permissions.insert(UserRole::ReadOnly, viewer_perms);

        Self {
            role_permissions: RwLock::new(role_permissions),
        }
    }

    /// Check if a role has permission for a resource
    pub fn has_permission(
        &self,
        role: &UserRole,
        resource_type: ResourceType,
        permission: Permission,
    ) -> bool {
        self.role_permissions
            .read()
            .get(role)
            .map(|perms| perms.contains(&(resource_type, permission)))
            .unwrap_or(false)
    }

    /// Get all permissions for a role
    pub fn get_permissions(&self, role: &UserRole) -> HashSet<(ResourceType, Permission)> {
        self.role_permissions
            .read()
            .get(role)
            .cloned()
            .unwrap_or_default()
    }

    /// Grant permission to a role
    pub fn grant_permission(
        &self,
        role: UserRole,
        resource_type: ResourceType,
        permission: Permission,
    ) {
        self.role_permissions
            .write()
            .entry(role)
            .or_default()
            .insert((resource_type, permission));
    }

    /// Revoke permission from a role
    pub fn revoke_permission(
        &self,
        role: &UserRole,
        resource_type: ResourceType,
        permission: Permission,
    ) {
        if let Some(perms) = self.role_permissions.write().get_mut(role) {
            perms.remove(&(resource_type, permission));
        }
    }
}
