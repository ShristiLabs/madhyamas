//! End-to-end integration test: load a compiled example WASM plugin and
//! execute a hook through the WasmRuntime.
//!
//! This test requires the `cors_helper.wasm` artifact built from
//! `madhyamas-plugin-sdk/examples/cors_helper.rs`. It is built automatically
//! when the `wasm-runtime` feature is enabled and the
//! `madhyamas-plugin-sdk` crate is a workspace member.
//!
//! If the WASM artifact is not present (e.g. the SDK examples haven't been
//! built), the test is skipped with a diagnostic message.

#![cfg(feature = "wasm-runtime")]

use madhyamas_core::plugin::{Plugin, PluginContext, PluginHook, PluginManifest, WasmRuntime};
use std::collections::HashMap;
use std::path::PathBuf;

fn find_wasm(name: &str) -> Option<PathBuf> {
    // Search the workspace target directory for the example artifact.
    let candidates = [
        format!(
            "target/wasm32-unknown-unknown/release/examples/{}.wasm",
            name
        ),
        format!("target/wasm32-unknown-unknown/debug/examples/{}.wasm", name),
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn make_plugin(id: &str, wasm_path: PathBuf) -> Plugin {
    let manifest = PluginManifest {
        id: id.into(),
        name: id.into(),
        version: "1.0.0".into(),
        description: None,
        author: None,
        homepage: None,
        repository: None,
        min_version: None,
        max_version: None,
        license: None,
        dependencies: HashMap::new(),
        hooks: vec!["on_response".into(), "on_load".into()],
        settings: None,
        enabled_by_default: true,
        capabilities: vec![],
        network: false,
        max_memory_pages: 64,
        fuel_limit: 10_000_000,
        timer_interval_seconds: None,
        publisher_public_key: None,
        panels: vec![],
        tags: vec![],
    };
    // The plugin path is the directory containing the wasm (we point directly
    // at the file; WasmRuntime looks for `<path>/plugin.wasm`).
    let dir = wasm_path.parent().unwrap().to_path_buf();
    let dest = dir.join("plugin.wasm");
    if !dest.exists() {
        std::fs::copy(&wasm_path, &dest).expect("copy wasm");
    }
    Plugin::from_manifest(manifest, &dir.to_string_lossy())
}

#[test]
fn cors_helper_adds_cors_headers() {
    let Some(wasm) = find_wasm("cors_helper") else {
        eprintln!("skipping: cors_helper.wasm not found (build with: cargo build --target wasm32-unknown-unknown --example cors_helper --release -p madhyamas-plugin-sdk)");
        return;
    };
    let rt = WasmRuntime::new().expect("runtime");
    let plugin = make_plugin("test.cors", wasm);

    let mut ctx = PluginContext::new("test.cors", PluginHook::OnResponse);
    ctx.response = Some(madhyamas_core::plugin::PluginResponse {
        status_code: 200,
        status_message: Some("OK".into()),
        headers: HashMap::new(),
        body: None,
        content_type: Some("text/html".into()),
        duration_ms: 42,
    });

    let result = rt.execute_hook(&plugin, PluginHook::OnResponse, &ctx);
    assert!(
        result.modified,
        "plugin should mark result as modified; logs: {:?}",
        result.logs
    );
    let resp = result
        .response
        .as_ref()
        .expect("plugin should return modified response");
    assert_eq!(
        resp.headers.get("Access-Control-Allow-Origin"),
        Some(&"*".to_string())
    );
}

#[test]
fn request_logger_passes_through() {
    let Some(wasm) = find_wasm("request_logger") else {
        eprintln!("skipping: request_logger.wasm not found");
        return;
    };
    let rt = WasmRuntime::new().expect("runtime");
    let plugin = make_plugin("test.logger", wasm);

    let mut ctx = PluginContext::new("test.logger", PluginHook::OnRequest);
    ctx.request = Some(madhyamas_core::plugin::PluginRequest {
        method: "GET".into(),
        url: "http://example.com/path".into(),
        host: "example.com".into(),
        path: "/path".into(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
    });

    let result = rt.execute_hook(&plugin, PluginHook::OnRequest, &ctx);
    // Logger should pass through (not modify) but should have logs.
    assert!(!result.handled, "logger should not handle");
    assert!(
        !result.logs.is_empty(),
        "logger should produce log output; got: {:?}",
        result.logs
    );
}
