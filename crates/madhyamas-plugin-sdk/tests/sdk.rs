//! Host-side integration tests for the plugin SDK wire types. These require
//! the `std` feature (host rlib build); WASM guest builds skip this file.

#![cfg(feature = "std")]

use madhyamas_plugin_sdk::Outcome;

#[test]
fn outcome_wire_format() {
    // The `continue_` field must serialize as `continue_` to match host.
    let o = Outcome::pass();
    let s = serde_json::to_string(&o).unwrap();
    assert!(s.contains("\"continue_\":true"), "got: {}", s);
}

#[test]
fn outcome_respond() {
    let o = Outcome::respond(403, "blocked");
    assert!(o.handled);
    assert!(!o.continue_);
    assert_eq!(o.custom_response.as_ref().unwrap().status_code, 403);
}
