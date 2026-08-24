//! Integration tests for the public plugin API (feature: `plugins`).
//!
//! Tests that exercise the WASM execution engine specifically are
//! additionally gated on `wasm-runtime`.

#![cfg(feature = "plugins")]

#[cfg(feature = "wasm-runtime")]
use std::collections::HashMap;

use madhyamas_core::plugin::{
    bytes_to_hex, generate_keypair, hex_to_bytes, sign_package, verify_package, PluginEventBus,
    PluginTemplates, TemplateId,
};
#[cfg(feature = "wasm-runtime")]
use madhyamas_core::plugin::{Plugin, PluginContext, PluginHook, PluginManifest, WasmRuntime};
use madhyamas_test_utils::tmpdir;

// ============================================================================
// PluginTemplates — scaffolding
// ============================================================================

#[test]
fn test_template_ids_roundtrip() {
    for id in TemplateId::all() {
        let s = id.as_str();
        assert_eq!(TemplateId::from_id(s), Some(id));
    }
}

#[test]
fn test_template_from_id_invalid() {
    assert_eq!(TemplateId::from_id("nonexistent"), None);
}

#[test]
fn test_scaffold_basic() {
    let tmp = tmpdir("plugin-scaffold");
    let name = "test-basic-plugin";
    PluginTemplates::scaffold(&TemplateId::Basic, name, tmp.path()).unwrap();

    let plugin_dir = tmp.path().join(name);
    assert!(plugin_dir.join("Cargo.toml").exists());
    assert!(plugin_dir.join("src/lib.rs").exists());
    assert!(plugin_dir.join("madhyamas-plugin.toml").exists());
    assert!(plugin_dir.join("README.md").exists());
    assert!(plugin_dir.join(".gitignore").exists());

    // Verify Cargo.toml content.
    let cargo = std::fs::read_to_string(plugin_dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains(name));

    // Verify lib.rs has the struct and register_plugin!.
    let lib = std::fs::read_to_string(plugin_dir.join("src/lib.rs")).unwrap();
    assert!(lib.contains("register_plugin!"));
    assert!(lib.contains("TestBasicPlugin"));

    // Verify manifest.
    let manifest = std::fs::read_to_string(plugin_dir.join("madhyamas-plugin.toml")).unwrap();
    assert!(manifest.contains(&format!("id = \"{}\"", name)));
    assert!(manifest.contains("on_request"));
}

#[test]
fn test_scaffold_domain_blocker_has_settings() {
    let tmp = tmpdir("plugin-scaffold");
    let name = "test-blocker";
    PluginTemplates::scaffold(&TemplateId::DomainBlocker, name, tmp.path()).unwrap();

    let manifest =
        std::fs::read_to_string(tmp.path().join(name).join("madhyamas-plugin.toml")).unwrap();
    assert!(manifest.contains("blocked_domains"));
    assert!(manifest.contains("[[settings.fields]]"));
}

#[test]
fn test_scaffold_refuses_existing_dir() {
    let tmp = tmpdir("plugin-scaffold");
    let name = "existing";
    std::fs::create_dir(tmp.path().join(name)).unwrap();
    let result = PluginTemplates::scaffold(&TemplateId::Basic, name, tmp.path());
    assert!(result.is_err());
}

// ============================================================================
// PluginEventBus — publish/subscribe
// ============================================================================

#[test]
fn test_subscribe_and_publish() {
    use std::sync::{Arc, Mutex as StdMutex};

    let bus = PluginEventBus::new();
    let received = Arc::new(StdMutex::new(Vec::new()));
    let received_clone = received.clone();
    let _id = bus.subscribe("test", move |event| {
        received_clone
            .lock()
            .unwrap()
            .push(event.as_str().unwrap().to_string());
    });
    bus.publish("test", serde_json::json!("hello"));
    bus.publish("test", serde_json::json!("world"));
    assert_eq!(*received.lock().unwrap(), vec!["hello", "world"]);
}

#[test]
fn test_unsubscribe() {
    use std::sync::{Arc, Mutex as StdMutex};

    let bus = PluginEventBus::new();
    let received = Arc::new(StdMutex::new(0));
    let received_clone = received.clone();
    let id = bus.subscribe("test", move |_event| {
        *received_clone.lock().unwrap() += 1;
    });
    bus.publish("test", serde_json::json!(1));
    assert_eq!(*received.lock().unwrap(), 1);
    assert!(bus.unsubscribe("test", id));
    bus.publish("test", serde_json::json!(2));
    assert_eq!(*received.lock().unwrap(), 1); // no new events
}

#[test]
fn test_multiple_subscribers() {
    use std::sync::{Arc, Mutex as StdMutex};

    let bus = PluginEventBus::new();
    let count = Arc::new(StdMutex::new(0));
    let count_clone = count.clone();
    let _id1 = bus.subscribe("topic", move |_| {
        *count_clone.lock().unwrap() += 1;
    });
    let count_clone2 = count.clone();
    let _id2 = bus.subscribe("topic", move |_| {
        *count_clone2.lock().unwrap() += 1;
    });
    bus.publish("topic", serde_json::json!(null));
    assert_eq!(*count.lock().unwrap(), 2);
}

#[test]
fn test_no_subscribers_silent() {
    let bus = PluginEventBus::new();
    // Should not panic.
    bus.publish("nobody", serde_json::json!("data"));
}

#[test]
fn test_subscriber_count_and_topics() {
    let bus = PluginEventBus::new();
    let _id1 = bus.subscribe("a", |_| {});
    let _id2 = bus.subscribe("a", |_| {});
    let _id3 = bus.subscribe("b", |_| {});
    assert_eq!(bus.subscriber_count("a"), 2);
    assert_eq!(bus.subscriber_count("b"), 1);
    assert_eq!(bus.subscriber_count("c"), 0);
    let mut topics = bus.topics();
    topics.sort();
    assert_eq!(topics, vec!["a", "b"]);
}

// ============================================================================
// Signing — Ed25519 package signatures
// ============================================================================

#[test]
fn test_keypair_sign_verify() {
    let kp = generate_keypair();
    let message = b"hello, plugin world!";
    let sig = sign_package(message, &kp.secret_key).unwrap();
    verify_package(message, &sig, &kp.public_key).unwrap();
}

#[test]
fn test_verify_rejects_tampered() {
    let kp = generate_keypair();
    let message = b"original message";
    let sig = sign_package(message, &kp.secret_key).unwrap();
    // Tamper with the message.
    let tampered = b"tampered message";
    assert!(verify_package(tampered, &sig, &kp.public_key).is_err());
}

#[test]
fn test_hex_roundtrip() {
    let bytes = [0u8, 1, 2, 3, 255, 128, 64];
    let hex = bytes_to_hex(&bytes);
    assert_eq!(hex, "00010203ff8040");
    let decoded: [u8; 7] = hex_to_bytes(&hex).unwrap();
    assert_eq!(decoded, bytes);
}

// ============================================================================
// WasmRuntime — sandboxed plugin execution (feature: `wasm-runtime`)
// ============================================================================

// Requires the wasmtime-backed runtime; the `plugins`-only build has no
// WASM execution engine.
#[cfg(feature = "wasm-runtime")]
#[test]
fn runtime_constructs() {
    // Constructing the runtime should succeed (engine + linker).
    let rt = WasmRuntime::new();
    assert!(rt.is_ok(), "WasmRuntime::new failed: {:?}", rt.err());
    let rt = rt.unwrap();
    assert_eq!(rt.cached_count(), 0);
}

// Requires the wasmtime-backed runtime to dispatch hook calls.
#[cfg(feature = "wasm-runtime")]
#[test]
fn manifest_only_plugin_returns_continue() {
    let rt = WasmRuntime::new().unwrap();
    // A plugin with no plugin.wasm should return cont() with a log.
    let manifest = PluginManifest {
        id: "test.noop".into(),
        name: "Noop".into(),
        version: "1.0.0".into(),
        description: None,
        author: None,
        homepage: None,
        repository: None,
        min_version: None,
        max_version: None,
        license: None,
        dependencies: HashMap::new(),
        hooks: vec!["on_request".into()],
        settings: None,
        enabled_by_default: false,
        capabilities: vec![],
        network: false,
        max_memory_pages: 64,
        fuel_limit: 1_000_000,
        timer_interval_seconds: None,
        publisher_public_key: None,
        env_grants: vec![],
        secret_grants: vec![],
        panels: vec![],
        tags: vec![],
    };
    let plugin = Plugin::from_manifest(manifest, "/nonexistent/path");
    let ctx = PluginContext::new("test.noop", PluginHook::OnRequest);
    let result = rt.execute_hook(&plugin, PluginHook::OnRequest, &ctx);
    assert!(!result.handled);
    assert!(result.continue_);
    assert!(!result.logs.is_empty());
}
