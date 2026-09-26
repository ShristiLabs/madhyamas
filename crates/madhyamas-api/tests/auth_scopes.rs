//! Integration tests for the public auth-scope helpers in
//! `madhyamas_api::auth` (issue #108): the `scope_grants` wildcard
//! matcher used by the in-handler WebSocket authentication path, and the
//! `DeviceScope` extension contract (OSS-inert: absent unless the
//! enterprise middleware inserted it).

use madhyamas_api::auth::{scope_grants, DeviceScope};

#[test]
fn scope_grants_exact_and_wildcard_matching() {
    let granted = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();

    assert!(scope_grants(&granted(&["traffic:read"]), "traffic:read"));
    assert!(
        scope_grants(&granted(&["*"]), "traffic:read"),
        "bare * grants all"
    );
    assert!(scope_grants(&granted(&["traffic:*"]), "traffic:read"));
    assert!(scope_grants(&granted(&["*:read"]), "traffic:read"));
    assert!(scope_grants(
        &granted(&["mocks:write", "traffic:read"]),
        "traffic:read"
    ));
    // Wrong resource, wrong permission, empty grants.
    assert!(!scope_grants(&granted(&["traffic:write"]), "traffic:read"));
    assert!(!scope_grants(&granted(&["mocks:read"]), "traffic:read"));
    assert!(!scope_grants(&granted(&[]), "traffic:read"));
    // Malformed entries never grant.
    assert!(!scope_grants(&granted(&["trafficread"]), "traffic:read"));
    assert!(!scope_grants(&granted(&["traffic:"]), "traffic:read"));
    assert!(!scope_grants(&granted(&["traffic:read"]), "trafficread"));
}

#[test]
fn device_scope_is_a_plain_inert_marker() {
    // The type carries only the binding; construction imposes no
    // enterprise dependency (the api crate stays OSS-clean).
    let scope = DeviceScope {
        device_id: "dev-x".to_string(),
    };
    assert_eq!(scope.device_id, "dev-x");
}
