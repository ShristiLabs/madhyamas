//! Integration tests for the public RBAC API: role permission matrix.

use madhyamas_enterprise::{Permission, RbacManager, ResourceType, UserRole};

#[test]
fn test_admin_can_delete() {
    let rbac = RbacManager::new();
    assert!(rbac.has_permission(&UserRole::Admin, ResourceType::Traffic, Permission::Delete));
    assert!(rbac.has_permission(&UserRole::Admin, ResourceType::Mock, Permission::Delete));
    assert!(rbac.has_permission(&UserRole::Admin, ResourceType::Config, Permission::Write));
}

#[test]
fn test_viewer_cannot_delete() {
    let rbac = RbacManager::new();
    assert!(!rbac.has_permission(&UserRole::Viewer, ResourceType::Traffic, Permission::Delete));
    assert!(!rbac.has_permission(&UserRole::Viewer, ResourceType::Mock, Permission::Delete));
    // Viewer can read.
    assert!(rbac.has_permission(&UserRole::Viewer, ResourceType::Traffic, Permission::Read));
}

#[test]
fn test_readonly_cannot_write() {
    let rbac = RbacManager::new();
    assert!(!rbac.has_permission(
        &UserRole::ReadOnly,
        ResourceType::Traffic,
        Permission::Write
    ));
    assert!(!rbac.has_permission(&UserRole::ReadOnly, ResourceType::Mock, Permission::Write));
    // ReadOnly can read.
    assert!(rbac.has_permission(&UserRole::ReadOnly, ResourceType::Traffic, Permission::Read));
}
