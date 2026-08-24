//! Integration tests for the public scripting API (feature: `scripting`).

#![cfg(feature = "scripting")]

use std::collections::HashMap;
use std::sync::Arc;

use madhyamas_core::scripting::{
    JsEngine, RequestContext, ResponseContext, Script, ScriptConfig, ScriptContext, ScriptHook,
    ScriptRuntime, ScriptTemplates,
};
use madhyamas_core::secrets::{Redactor, SecretService};
use madhyamas_test_utils::MemStore;

// ============================================================================
// JsEngine — execution
// ============================================================================

fn make_context() -> ScriptContext {
    let mut ctx = ScriptContext::new("req-1", "sess-1", ScriptHook::OnRequest);
    ctx.request = Some(make_request_context("http://example.com/api/test"));
    ctx
}

fn make_request_context(url: &str) -> RequestContext {
    RequestContext {
        method: "GET".to_string(),
        url: url.to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        headers: HashMap::new(),
        body: None,
        content_type: None,
        query: HashMap::new(),
    }
}

#[test]
fn execute_simple_on_request() {
    let source = r#"
function onRequest(request, context) {
    console.log("hello " + request.method);
    return { continue: true, modified: true };
}
"#;
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(result.modified);
    assert!(result.continue_);
    assert_eq!(result.console, vec!["hello GET".to_string()]);
}

#[test]
fn execute_block_request() {
    let source = r#"
function onRequest(request, context) {
    if (request.url.indexOf("ads") !== -1) {
        return {
            continue: false,
            response: {
                statusCode: 403,
                headers: { "Content-Type": "text/plain" },
                body: "Blocked"
            }
        };
    }
    return { continue: true };
}
"#;
    let mut ctx = make_context();
    ctx.request = Some(make_request_context("http://ads.example.com/banner"));
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(!result.continue_);
    let resp = result.response.expect("response should be set");
    assert_eq!(resp.status_code, 403);
    assert_eq!(resp.body, "Blocked");
    assert_eq!(resp.headers.get("Content-Type").unwrap(), "text/plain");
}

#[test]
fn execute_missing_hook_function() {
    let source = "var x = 1;";
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_some());
    assert!(result.error.as_ref().unwrap().contains("onRequest"));
}

#[test]
fn execute_parse_error() {
    let source = "function onRequest(";
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_some());
    assert!(result.error.as_ref().unwrap().contains("Parse error"));
}

#[test]
fn validate_rejects_invalid_source() {
    let result = JsEngine::validate("function (");
    assert!(result.is_err());
}

#[test]
fn validate_accepts_valid_source() {
    let result = JsEngine::validate("function onRequest() { return { continue: true }; }");
    assert!(result.is_ok());
}

#[test]
fn base64_encode_decode() {
    let source = r#"
function onRequest(request, context) {
    var encoded = base64.encode("hello");
    var decoded = base64.decode(encoded);
    console.log(encoded + " " + decoded);
    return { continue: true };
}
"#;
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert_eq!(result.console, vec!["aGVsbG8= hello".to_string()]);
}

#[test]
fn crypto_hash_sha256() {
    let source = r#"
function onRequest(request, context) {
    var h = crypto.hash("test");
    console.log(h);
    return { continue: true };
}
"#;
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    // SHA-256 of "test"
    assert_eq!(
        result.console,
        vec!["9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".to_string()]
    );
}

#[test]
fn url_parse_build() {
    let source = r#"
function onRequest(request, context) {
    var parts = url.parse("https://example.com:8080/api/users?id=42");
    console.log(parts.scheme + " " + parts.host + " " + parts.port + " " + parts.path);
    console.log(parts.query.id);
    return { continue: true };
}
"#;
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert_eq!(result.console.len(), 2);
    assert_eq!(result.console[0], "https example.com 8080 /api/users");
    assert_eq!(result.console[1], "42");
}

#[test]
fn json_parse_stringify_available() {
    let source = r#"
function onRequest(request, context) {
    var obj = JSON.parse('{"name":"test","value":42}');
    console.log(obj.name + " " + obj.value);
    console.log(JSON.stringify({ ok: true }));
    return { continue: true };
}
"#;
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert_eq!(result.console[0], "test 42");
    assert_eq!(result.console[1], "{\"ok\":true}");
}

#[test]
fn template_log_requests_executes() {
    let script = ScriptTemplates::log_requests();
    let ctx = make_context();
    let result = JsEngine::execute(&script.source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(result.continue_);
}

#[test]
fn template_add_cors_executes() {
    let script = ScriptTemplates::add_cors();
    let mut ctx = make_context();
    ctx.request = Some(make_request_context("http://example.com"));
    ctx.response = Some(ResponseContext {
        status_code: 200,
        status_message: None,
        headers: HashMap::new(),
        body: None,
        content_type: None,
        duration_ms: 10,
    });
    let result = JsEngine::execute(
        &script.source,
        "on_response",
        &ctx,
        &ScriptConfig::default(),
    );
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(result.modified);
}

#[test]
fn template_mock_api_executes() {
    let script = ScriptTemplates::mock_api();
    let mut ctx = make_context();
    ctx.request = Some(make_request_context("http://example.com/api/user/123"));
    let result = JsEngine::execute(&script.source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(!result.continue_);
    let resp = result.response.expect("response should be set");
    assert_eq!(resp.status_code, 200);
    assert!(resp.body.contains("Mock User"));
}

#[test]
fn modified_request_headers_read_back() {
    let source = r#"
function onRequest(request, context) {
    request.headers['X-Custom'] = 'injected';
    request.headers['X-Request-ID'] = context.requestId;
    return { continue: true, modified: true };
}
"#;
    let ctx = make_context();
    let result = JsEngine::execute(source, "on_request", &ctx, &ScriptConfig::default());
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(result.modified);
    let modified = result
        .modified_request
        .expect("modified_request should be set");
    assert_eq!(modified.headers.get("X-Custom").unwrap(), "injected");
    assert_eq!(modified.headers.get("X-Request-ID").unwrap(), "req-1");
}

#[test]
fn modified_response_headers_read_back() {
    let script = ScriptTemplates::add_cors();
    let mut ctx = make_context();
    ctx.response = Some(ResponseContext {
        status_code: 200,
        status_message: None,
        headers: HashMap::new(),
        body: None,
        content_type: None,
        duration_ms: 5,
    });
    let result = JsEngine::execute(
        &script.source,
        "on_response",
        &ctx,
        &ScriptConfig::default(),
    );
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(result.modified);
    let modified = result
        .modified_response
        .expect("modified_response should be set");
    assert_eq!(
        modified.headers.get("Access-Control-Allow-Origin").unwrap(),
        "*"
    );
}

// ============================================================================
// ScriptRuntime — secret substitution into executing scripts (#87)
// ============================================================================

fn svc() -> Arc<SecretService> {
    let store = MemStore::new();
    let s = SecretService::new(Arc::new(store)).unwrap();
    s.set("api_token", "tok-abc123").unwrap();
    Arc::new(s)
}

#[test]
fn execute_substitutes_granted_secret_into_running_script() {
    // A script that logs the substituted token; the execution history
    // (what script traces are built from) must contain the substituted
    // value — the redaction pass at the API layer is what protects it
    // from leaking (tested in secrets::redaction).
    let runtime = ScriptRuntime::new(ScriptConfig::default());
    runtime.with_secrets(svc());
    let mut script = Script::new(
        "tok".into(),
        "function onRequest(ctx) { console.log('${SECRET:api_token}'); }".into(),
    );
    script.hooks = vec!["on_request".into()];
    script.secret_grants = vec!["api_token".into()];
    runtime.register_script(script.clone());
    let ctx = ScriptContext::new("test-req", "test-sess", ScriptHook::OnRequest);
    let result = runtime.execute(&script.id, &ctx);
    assert!(result.error.is_none(), "error: {:?}", result.error);
    assert!(result.console.iter().any(|l| l.contains("tok-abc123")));
    // Redaction: the trace line must redact via the shared redactor.
    let redactor = Redactor::with_defaults(vec!["tok-abc123".to_string()]);
    let mut lines = result.console.clone();
    redactor.redact_lines(&mut lines);
    assert!(lines.iter().all(|l| !l.contains("tok-abc123")));
}
