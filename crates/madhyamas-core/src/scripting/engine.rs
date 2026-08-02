//! JavaScript engine integration backed by [`boa_engine`].
//!
//! [`JsEngine`] is a stateless facade around boa's ECMAScript engine.  A fresh
//! [`Context`] is created for every execution so scripts run in complete
//! isolation from one another (no shared globals, no leaked state).  The only
//! shared state is the script source code itself, which is owned by
//! [`super::ScriptRuntime`].
//!
//! # Security
//!
//! boa has no filesystem, network, or process access by default.  We do not
//! register any host functions that would expose those capabilities, so
//! scripts are sandboxed by construction.  See
//! [`docs/SCRIPTING_SECURITY.md`](../../../docs/SCRIPTING_SECURITY.md) for the
//! full threat model.

use base64::Engine as _;
use boa_engine::{
    js_string, object::ObjectInitializer, property::Attribute, Context, JsArgs, JsObject, JsResult,
    JsValue, NativeFunction, Source,
};
use std::collections::HashMap;
use std::time::Instant;

use super::api::URLComponents;
use super::hooks::{RequestContext, ResponseContext, ScriptContext, ScriptResponse};
use super::runtime::ScriptConfig;
use super::ScriptResult;

/// JavaScript preamble evaluated before every user script.
///
/// Sets up a `console` object whose `log` function appends stringified
/// arguments to a global `__console` array.  After execution the engine reads
/// the array back to collect console output.  Implementing `console` in JS
/// avoids the need for stateful native functions (boa's `NativeFunction` fn
/// pointers cannot capture mutable state).
const CONSOLE_PREAMBLE: &str = r#"
var __console = [];
var console = {
    log: function () {
        var parts = [];
        for (var i = 0; i < arguments.length; i++) {
            var a = arguments[i];
            if (a === null) { parts.push('null'); }
            else if (a === undefined) { parts.push('undefined'); }
            else if (typeof a === 'object') {
                try { parts.push(JSON.stringify(a)); }
                catch (e) { parts.push('[object]'); }
            }
            else { parts.push(String(a)); }
        }
        __console.push(parts.join(' '));
    }
};
"#;

/// Map a snake_case hook name (`on_request`) to the camelCase JS function name
/// (`onRequest`) that scripts are expected to define.
fn js_function_name(hook: &str) -> &str {
    match hook {
        "on_request" => "onRequest",
        "on_response" => "onResponse",
        "on_websocket_message" => "onWebSocketMessage",
        "on_grpc_message" => "onGrpcMessage",
        "on_traffic_store" => "onTrafficStore",
        "on_session_start" => "onSessionStart",
        "on_session_end" => "onSessionEnd",
        other => other,
    }
}

/// Stateless JavaScript execution engine.
pub struct JsEngine;

impl JsEngine {
    /// Execute a script's hook function against the given [`ScriptContext`].
    ///
    /// A fresh boa [`Context`] is created for each call, ensuring complete
    /// isolation between scripts.  The `timeout_ms` limit in `config` is
    /// enforced as a *soft* limit: boa does not support mid-execution
    /// preemption, so the script always runs to completion, but if it exceeds
    /// the configured timeout the result is replaced with a timeout error.
    /// (User-authored scripts are trusted per the trust model in
    /// `docs/SCRIPTING_SECURITY.md`.)
    pub fn execute(
        source: &str,
        hook: &str,
        context: &ScriptContext,
        config: &ScriptConfig,
    ) -> ScriptResult {
        let start = Instant::now();
        let mut ctx = Context::default();

        // 1. Register the console preamble and host utility globals.
        if let Err(e) = register_globals(&mut ctx, config) {
            return ScriptResult {
                error: Some(format!("Failed to initialise script globals: {e}")),
                ..Default::default()
            };
        }

        // 2. Parse and evaluate the user's script source (defines functions).
        if let Err(e) = ctx.eval(Source::from_bytes(source)) {
            return ScriptResult {
                error: Some(format!("Parse error: {e}")),
                ..Default::default()
            };
        }

        // 3. Look up the hook function (e.g. `onRequest`) on the global object.
        let hook_fn = js_function_name(hook);
        let global = ctx.global_object().clone();
        let hook_val = match global.get(js_string!(hook_fn), &mut ctx) {
            Ok(v) => v,
            Err(e) => {
                return ScriptResult {
                    error: Some(format!("Failed to look up hook '{hook_fn}': {e}")),
                    ..Default::default()
                };
            }
        };
        let hook_obj = match hook_val.as_object() {
            Some(o) if o.is_callable() => o,
            _ => {
                return ScriptResult {
                    error: Some(format!(
                        "Function '{hook_fn}' is not defined. Define `function {hook_fn}(...)` in your script."
                    )),
                    ..Default::default()
                };
            }
        };

        // 4. Build the JS argument objects for the hook, keeping references
        //    to the request/response objects so we can read back modifications.
        let (args, req_obj, resp_obj) = build_hook_args(hook, context, &mut ctx);

        // 5. Call the hook function.
        let call_result = hook_obj.call(&JsValue::undefined(), &args, &mut ctx);
        let duration_ms = start.elapsed().as_millis() as u64;

        // 6. Collect console output regardless of success/failure.
        let console = read_console(&mut ctx);

        // 7. Soft timeout check.
        if duration_ms > config.timeout_ms {
            return ScriptResult {
                error: Some(format!(
                    "Script timed out after {duration_ms}ms (limit: {}ms)",
                    config.timeout_ms
                )),
                console,
                duration_ms,
                ..Default::default()
            };
        }

        match call_result {
            Ok(retval) => {
                let mut result = parse_result(retval, console, duration_ms, &mut ctx);
                // 8. If the script reported modifications, read back the
                //    modified request/response objects from the JS context.
                if result.modified {
                    if let Some(req_obj) = &req_obj {
                        result.modified_request = read_request_obj(req_obj, &mut ctx);
                    }
                    if let Some(resp_obj) = &resp_obj {
                        result.modified_response = read_response_obj(resp_obj, &mut ctx);
                    }
                }
                result
            }
            Err(e) => ScriptResult {
                error: Some(format!("Runtime error: {e}")),
                console,
                duration_ms,
                ..Default::default()
            },
        }
    }

    /// Validate that a script's source parses without errors (without
    /// executing it).  Used by the API on create/update to reject malformed
    /// scripts early.
    pub fn validate(source: &str) -> Result<(), String> {
        let mut ctx = Context::default();
        let _ = register_globals(&mut ctx, &ScriptConfig::default());
        ctx.eval(Source::from_bytes(source))
            .map(|_| ())
            .map_err(|e| format!("Parse error: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Global registration
// ---------------------------------------------------------------------------

/// Register the console preamble and host utility globals (`base64`,
/// `crypto`, `url`) on the context.
fn register_globals(ctx: &mut Context, _config: &ScriptConfig) -> JsResult<()> {
    // Console preamble (defines `console` and `__console` in JS).
    ctx.eval(Source::from_bytes(CONSOLE_PREAMBLE))?;

    // base64.encode / base64.decode
    let base64_obj = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let input = args
                    .get_or_undefined(0)
                    .to_string(ctx)?
                    .to_std_string_escaped();
                let encoded = base64::engine::general_purpose::STANDARD.encode(&input);
                Ok(JsValue::String(js_string!(encoded.as_str())))
            }),
            js_string!("encode"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let input = args
                    .get_or_undefined(0)
                    .to_string(ctx)?
                    .to_std_string_escaped();
                match base64::engine::general_purpose::STANDARD.decode(&input) {
                    Ok(bytes) => Ok(JsValue::String(js_string!(
                        String::from_utf8_lossy(&bytes).as_ref()
                    ))),
                    Err(_) => Ok(JsValue::undefined()),
                }
            }),
            js_string!("decode"),
            0,
        )
        .build();
    ctx.register_global_property(js_string!("base64"), base64_obj, Attribute::all())?;

    // crypto.hash (SHA-256 hex digest)
    let crypto_obj = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let input = args
                    .get_or_undefined(0)
                    .to_string(ctx)?
                    .to_std_string_escaped();
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(input.as_bytes());
                let result = hasher.finalize();
                let hex: String = result.iter().map(|b| format!("{b:02x}")).collect();
                Ok(JsValue::String(js_string!(hex.as_str())))
            }),
            js_string!("hash"),
            0,
        )
        .build();
    ctx.register_global_property(js_string!("crypto"), crypto_obj, Attribute::all())?;

    // url.parse / url.build
    let url_obj = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let input = args
                    .get_or_undefined(0)
                    .to_string(ctx)?
                    .to_std_string_escaped();
                match URLComponents::parse(&input) {
                    Some(c) => {
                        let obj = build_url_components(&c, ctx)?;
                        Ok(JsValue::Object(obj))
                    }
                    None => Ok(JsValue::null()),
                }
            }),
            js_string!("parse"),
            0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let arg = args.get_or_undefined(0);
                let obj = match arg.as_object() {
                    Some(o) => o,
                    None => return Ok(JsValue::undefined()),
                };
                let scheme = get_string(obj, "scheme", ctx);
                let host = get_string(obj, "host", ctx);
                let port = obj
                    .get(js_string!("port"), ctx)
                    .ok()
                    .and_then(|v| v.as_number().map(|n| n as u16));
                let path = get_string(obj, "path", ctx);
                let fragment = get_string_option(obj, "fragment", ctx);

                let mut query = HashMap::new();
                if let Ok(qv) = obj.get(js_string!("query"), ctx) {
                    if let Some(qobj) = qv.as_object() {
                        if let Ok(keys) = qobj.own_property_keys(ctx) {
                            for key in keys {
                                let key_str = property_key_to_string(&key, ctx);
                                if let Ok(val) = qobj.get(key, ctx) {
                                    let val_str = val
                                        .to_string(ctx)
                                        .map(|s| s.to_std_string_escaped())
                                        .unwrap_or_default();
                                    query.insert(key_str, val_str);
                                }
                            }
                        }
                    }
                }

                let components = URLComponents {
                    scheme,
                    host,
                    port,
                    path,
                    query,
                    fragment,
                };
                Ok(JsValue::String(js_string!(components.build().as_str())))
            }),
            js_string!("build"),
            0,
        )
        .build();
    ctx.register_global_property(js_string!("url"), url_obj, Attribute::all())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// JS object construction
// ---------------------------------------------------------------------------

/// Build the argument array for a hook function call, returning the args
/// along with references to the request and response JS objects (so
/// modifications can be read back after execution).
fn build_hook_args(
    hook: &str,
    context: &ScriptContext,
    ctx: &mut Context,
) -> (Vec<JsValue>, Option<JsObject>, Option<JsObject>) {
    let request_val = context
        .request
        .as_ref()
        .map(|r| build_request_obj(r, ctx))
        .transpose()
        .ok()
        .flatten();
    let response_val = context
        .response
        .as_ref()
        .map(|r| build_response_obj(r, ctx))
        .transpose()
        .ok()
        .flatten();
    let context_obj = build_context_obj(context, ctx).ok();

    // Extract the JsObject references before moving the values into args.
    let req_obj = request_val.as_ref().and_then(|v| v.as_object()).cloned();
    let resp_obj = response_val.as_ref().and_then(|v| v.as_object()).cloned();

    let args = match hook {
        "on_response" => {
            let req = request_val.unwrap_or(JsValue::undefined());
            let resp = response_val.unwrap_or(JsValue::undefined());
            let ctx_val = context_obj.unwrap_or(JsValue::undefined());
            vec![req, resp, ctx_val]
        }
        "on_request" => {
            let req = request_val.unwrap_or(JsValue::undefined());
            let ctx_val = context_obj.unwrap_or(JsValue::undefined());
            vec![req, ctx_val]
        }
        _ => {
            let ctx_val = context_obj.unwrap_or(JsValue::undefined());
            vec![ctx_val]
        }
    };

    (args, req_obj, resp_obj)
}

/// Build a JS object representing a [`RequestContext`].
fn build_request_obj(req: &RequestContext, ctx: &mut Context) -> JsResult<JsValue> {
    let headers = build_string_map(&req.headers, ctx)?;
    let query = build_string_map(&req.query, ctx)?;

    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("method"),
            js_string!(req.method.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("url"),
            js_string!(req.url.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("host"),
            js_string!(req.host.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("path"),
            js_string!(req.path.as_str()),
            Attribute::all(),
        )
        .property(js_string!("headers"), headers, Attribute::all())
        .property(js_string!("query"), query, Attribute::all())
        .build();

    let body_val = req
        .body
        .as_ref()
        .map(|b| JsValue::String(js_string!(b.as_str())))
        .unwrap_or(JsValue::null());
    obj.set(js_string!("body"), body_val, false, ctx)?;

    let ct_val = req
        .content_type
        .as_ref()
        .map(|c| JsValue::String(js_string!(c.as_str())))
        .unwrap_or(JsValue::null());
    obj.set(js_string!("contentType"), ct_val, false, ctx)?;

    Ok(JsValue::Object(obj))
}

/// Build a JS object representing a [`ResponseContext`].
fn build_response_obj(resp: &ResponseContext, ctx: &mut Context) -> JsResult<JsValue> {
    let headers = build_string_map(&resp.headers, ctx)?;

    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("statusCode"),
            JsValue::Integer(resp.status_code as i32),
            Attribute::all(),
        )
        .property(js_string!("headers"), headers, Attribute::all())
        .property(
            js_string!("durationMs"),
            JsValue::Integer(resp.duration_ms as i32),
            Attribute::all(),
        )
        .build();

    let body_val = resp
        .body
        .as_ref()
        .map(|b| JsValue::String(js_string!(b.as_str())))
        .unwrap_or(JsValue::null());
    obj.set(js_string!("body"), body_val, false, ctx)?;

    let ct_val = resp
        .content_type
        .as_ref()
        .map(|c| JsValue::String(js_string!(c.as_str())))
        .unwrap_or(JsValue::null());
    obj.set(js_string!("contentType"), ct_val, false, ctx)?;

    if let Some(ref msg) = resp.status_message {
        obj.set(
            js_string!("statusMessage"),
            JsValue::String(js_string!(msg.as_str())),
            false,
            ctx,
        )?;
    }

    Ok(JsValue::Object(obj))
}

/// Build a JS object representing the script execution [`ScriptContext`].
fn build_context_obj(context: &ScriptContext, ctx: &mut Context) -> JsResult<JsValue> {
    let data_obj = JsObject::with_object_proto(ctx.intrinsics());
    for (k, v) in &context.data {
        let js_val = serde_json_value_to_js(v, ctx)?;
        data_obj.set(js_string!(k.as_str()), js_val, false, ctx)?;
    }

    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("requestId"),
            js_string!(context.request_id.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("sessionId"),
            js_string!(context.session_id.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("hook"),
            js_string!(context.hook.as_str()),
            Attribute::all(),
        )
        .property(js_string!("data"), data_obj, Attribute::all())
        .build();

    Ok(JsValue::Object(obj))
}

/// Build a JS object from a `HashMap<String, String>`.
fn build_string_map(map: &HashMap<String, String>, ctx: &mut Context) -> JsResult<JsObject> {
    let obj = JsObject::with_object_proto(ctx.intrinsics());
    for (k, v) in map {
        obj.set(
            js_string!(k.as_str()),
            JsValue::String(js_string!(v.as_str())),
            false,
            ctx,
        )?;
    }
    Ok(obj)
}

/// Build a JS object from [`URLComponents`].
fn build_url_components(c: &URLComponents, ctx: &mut Context) -> JsResult<JsObject> {
    let query = build_string_map(&c.query, ctx)?;
    let obj = ObjectInitializer::new(ctx)
        .property(
            js_string!("scheme"),
            js_string!(c.scheme.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("host"),
            js_string!(c.host.as_str()),
            Attribute::all(),
        )
        .property(
            js_string!("port"),
            c.port
                .map(|p| JsValue::Integer(p as i32))
                .unwrap_or(JsValue::null()),
            Attribute::all(),
        )
        .property(
            js_string!("path"),
            js_string!(c.path.as_str()),
            Attribute::all(),
        )
        .property(js_string!("query"), query, Attribute::all())
        .build();
    if let Some(ref frag) = c.fragment {
        obj.set(
            js_string!("fragment"),
            JsValue::String(js_string!(frag.as_str())),
            false,
            ctx,
        )?;
    }
    Ok(obj)
}

/// Convert a [`serde_json::Value`] to a [`JsValue`] by serialising to JSON and
/// evaluating the parenthesised expression.  This produces proper JS objects
/// and arrays without needing direct access to `JsArray` constructors.
fn serde_json_value_to_js(v: &serde_json::Value, ctx: &mut Context) -> JsResult<JsValue> {
    let json_str = serde_json::to_string(v).unwrap_or_else(|_| "null".to_string());
    let source = format!("({json_str})");
    ctx.eval(Source::from_bytes(&source))
}

// ---------------------------------------------------------------------------
// Result parsing
// ---------------------------------------------------------------------------

/// Parse the JS return value into a [`ScriptResult`].
fn parse_result(
    retval: JsValue,
    console: Vec<String>,
    duration_ms: u64,
    ctx: &mut Context,
) -> ScriptResult {
    let mut result = ScriptResult {
        console,
        duration_ms,
        ..Default::default()
    };

    let obj = match retval.as_object() {
        Some(o) => o,
        None => {
            // Script returned a non-object (e.g. undefined).  Treat as
            // continue with no modification.
            return result;
        }
    };

    // continue (default true)
    result.continue_ = obj
        .get(js_string!("continue"), ctx)
        .ok()
        .and_then(|v| v.as_boolean())
        .unwrap_or(true);

    // modified (default false)
    result.modified = obj
        .get(js_string!("modified"), ctx)
        .ok()
        .and_then(|v| v.as_boolean())
        .unwrap_or(false);

    // response (only meaningful when continue is false)
    if !result.continue_ {
        if let Ok(response_val) = obj.get(js_string!("response"), ctx) {
            if let Some(resp_obj) = response_val.as_object() {
                result.response = parse_response(resp_obj, ctx);
            }
        }
    }

    result
}

/// Parse a JS response object into a [`ScriptResponse`].
fn parse_response(obj: &JsObject, ctx: &mut Context) -> Option<ScriptResponse> {
    let status_code = obj
        .get(js_string!("statusCode"), ctx)
        .ok()
        .and_then(|v| v.as_number().map(|n| n as u16))
        .unwrap_or(200);

    let mut headers = HashMap::new();
    if let Ok(headers_val) = obj.get(js_string!("headers"), ctx) {
        if let Some(headers_obj) = headers_val.as_object() {
            if let Ok(keys) = headers_obj.own_property_keys(ctx) {
                for key in keys {
                    let key_str = property_key_to_string(&key, ctx);
                    if let Ok(val) = headers_obj.get(key, ctx) {
                        let val_str = val
                            .to_string(ctx)
                            .map(|s| s.to_std_string_escaped())
                            .unwrap_or_default();
                        headers.insert(key_str, val_str);
                    }
                }
            }
        }
    }

    let body = obj
        .get(js_string!("body"), ctx)
        .ok()
        .and_then(|v| {
            if v.is_null() || v.is_undefined() {
                Some(String::new())
            } else {
                v.to_string(ctx).ok().map(|s| s.to_std_string_escaped())
            }
        })
        .unwrap_or_default();

    Some(ScriptResponse {
        status_code,
        headers,
        body,
    })
}

// ---------------------------------------------------------------------------
// Console output
// ---------------------------------------------------------------------------

/// Read the `__console` array from the context and return its entries.
fn read_console(ctx: &mut Context) -> Vec<String> {
    let global = ctx.global_object().clone();
    let console_val = match global.get(js_string!("__console"), ctx) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let arr = match console_val.as_object() {
        Some(o) => o,
        None => return Vec::new(),
    };

    let length = arr
        .get(js_string!("length"), ctx)
        .ok()
        .and_then(|v| v.as_number().map(|n| n as usize))
        .unwrap_or(0);

    let mut out = Vec::with_capacity(length);
    for i in 0..length {
        if let Ok(elem) = arr.get(i as u32, ctx) {
            let s = elem
                .to_string(ctx)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_default();
            out.push(s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Read-back (modified objects → Rust)
// ---------------------------------------------------------------------------

/// Read a JS request object back into a [`RequestContext`].
fn read_request_obj(obj: &JsObject, ctx: &mut Context) -> Option<RequestContext> {
    let method = get_string(obj, "method", ctx);
    let url = get_string(obj, "url", ctx);
    let host = get_string(obj, "host", ctx);
    let path = get_string(obj, "path", ctx);
    let headers = read_string_map(obj, "headers", ctx);
    let query = read_string_map(obj, "query", ctx);
    let body = get_string_option(obj, "body", ctx);
    let content_type = get_string_option(obj, "contentType", ctx);

    Some(RequestContext {
        method,
        url,
        host,
        path,
        headers,
        body,
        content_type,
        query,
    })
}

/// Read a JS response object back into a [`ResponseContext`].
fn read_response_obj(obj: &JsObject, ctx: &mut Context) -> Option<ResponseContext> {
    let status_code = obj
        .get(js_string!("statusCode"), ctx)
        .ok()
        .and_then(|v| v.as_number().map(|n| n as u16))
        .unwrap_or(200);
    let headers = read_string_map(obj, "headers", ctx);
    let body = get_string_option(obj, "body", ctx);
    let content_type = get_string_option(obj, "contentType", ctx);
    let status_message = get_string_option(obj, "statusMessage", ctx);
    let duration_ms = obj
        .get(js_string!("durationMs"), ctx)
        .ok()
        .and_then(|v| v.as_number().map(|n| n as u64))
        .unwrap_or(0);

    Some(ResponseContext {
        status_code,
        status_message,
        headers,
        body,
        content_type,
        duration_ms,
    })
}

/// Read a JS object property (a map of string→string) into a `HashMap`.
fn read_string_map(obj: &JsObject, key: &str, ctx: &mut Context) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(val) = obj.get(js_string!(key), ctx) {
        if let Some(map_obj) = val.as_object() {
            if let Ok(keys) = map_obj.own_property_keys(ctx) {
                for key in keys {
                    let key_str = property_key_to_string(&key, ctx);
                    if let Ok(val) = map_obj.get(key, ctx) {
                        let val_str = val
                            .to_string(ctx)
                            .map(|s| s.to_std_string_escaped())
                            .unwrap_or_default();
                        map.insert(key_str, val_str);
                    }
                }
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get a string property from a JS object (defaults to empty string).
fn get_string(obj: &JsObject, key: &str, ctx: &mut Context) -> String {
    obj.get(js_string!(key), ctx)
        .ok()
        .and_then(|v| {
            if v.is_null() || v.is_undefined() {
                None
            } else {
                v.to_string(ctx).ok().map(|s| s.to_std_string_escaped())
            }
        })
        .unwrap_or_default()
}

/// Get an optional string property from a JS object.
fn get_string_option(obj: &JsObject, key: &str, ctx: &mut Context) -> Option<String> {
    obj.get(js_string!(key), ctx).ok().and_then(|v| {
        if v.is_null() || v.is_undefined() {
            None
        } else {
            v.to_string(ctx).ok().map(|s| s.to_std_string_escaped())
        }
    })
}

/// Convert a [`boa_engine::property::PropertyKey`] to a Rust `String`.
fn property_key_to_string(key: &boa_engine::property::PropertyKey, ctx: &mut Context) -> String {
    use boa_engine::property::PropertyKey;
    match key {
        PropertyKey::String(s) => s.to_std_string_escaped(),
        PropertyKey::Index(i) => i.get().to_string(),
        PropertyKey::Symbol(_) => {
            let val = JsValue::from(key.clone());
            val.to_string(ctx)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|_| "[Symbol]".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::hooks::ScriptHook;
    use super::*;

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
        let script = super::super::runtime::ScriptTemplates::log_requests();
        let ctx = make_context();
        let result =
            JsEngine::execute(&script.source, "on_request", &ctx, &ScriptConfig::default());
        assert!(result.error.is_none(), "error: {:?}", result.error);
        assert!(result.continue_);
    }

    #[test]
    fn template_add_cors_executes() {
        let script = super::super::runtime::ScriptTemplates::add_cors();
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
        let script = super::super::runtime::ScriptTemplates::mock_api();
        let mut ctx = make_context();
        ctx.request = Some(make_request_context("http://example.com/api/user/123"));
        let result =
            JsEngine::execute(&script.source, "on_request", &ctx, &ScriptConfig::default());
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
        let script = super::super::runtime::ScriptTemplates::add_cors();
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
}
