//! Unified extension trait.
//!
//! Both the scripting runtime ([`crate::scripting::ScriptRuntime`]) and the
//! plugin system ([`crate::plugin::PluginManager`]) expose `on_request` /
//! `on_response` hooks with nearly identical semantics.  This module
//! defines a single [`Extension`] trait that abstracts over both, plus an
//! [`ExtensionManager`] that runs all registered extensions in priority
//! order.
//!
//! The proxy pipeline calls [`ExtensionManager::on_request`] and
//! [`ExtensionManager::on_response`] instead of invoking the script runtime
//! and plugin manager separately.  Existing code that calls those managers
//! directly continues to work — the [`Extension`] implementations simply
//! delegate to them.

use parking_lot::RwLock;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Context & Result
// ---------------------------------------------------------------------------

/// Unified context passed to every extension hook.
///
/// This is a superset of the fields needed by both [`ScriptContext`] and
/// [`PluginContext`].  Adapter implementations in
/// [`ScriptExtension`] and [`PluginExtension`] project this struct onto
/// their internal types.
///
/// [`ScriptContext`]: crate::scripting::ScriptContext
/// [`PluginContext`]: crate::plugin::PluginContext
#[derive(Debug, Clone)]
pub struct ExtensionContext {
    /// Request ID (generated per pipeline invocation).
    pub request_id: String,
    /// Active capture session ID.
    pub session_id: String,
    /// Hook name (`"on_request"` or `"on_response"`).
    pub hook: &'static str,
    /// Request data — always present for `on_request`, present for
    /// `on_response` when the response corresponds to a captured request.
    pub request: Option<ExtensionRequest>,
    /// Response data — only present for `on_response`.
    pub response: Option<ExtensionResponse>,
    /// Free-form key/value bag shared across hooks for the same request.
    pub data: std::collections::HashMap<String, serde_json::Value>,
    /// Timestamp of hook invocation.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Request portion of an [`ExtensionContext`].
#[derive(Debug, Clone)]
pub struct ExtensionRequest {
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
}

/// Response portion of an [`ExtensionContext`].
#[derive(Debug, Clone)]
pub struct ExtensionResponse {
    pub status_code: u16,
    pub status_message: Option<String>,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub content_type: Option<String>,
    pub duration_ms: u64,
}

/// Result returned by an extension hook.
#[derive(Debug, Clone, Default)]
pub struct ExtensionResult {
    /// Whether the extension handled the request (e.g. short-circuited).
    pub handled: bool,
    /// Whether to continue invoking subsequent extensions.
    pub continue_chain: bool,
    /// Whether the extension modified the request/response.
    pub modified: bool,
    /// Error message, if the extension failed.
    pub error: Option<String>,
    /// Log lines produced by the extension.
    pub logs: Vec<String>,
}

impl ExtensionResult {
    /// No-op result: continue chain, no modification.
    pub fn pass() -> Self {
        Self::default()
    }

    /// Result indicating the extension modified the request/response.
    pub fn modified() -> Self {
        Self {
            modified: true,
            continue_chain: true,
            ..Default::default()
        }
    }

    /// Result that stops the chain with an error.
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            error: Some(msg.into()),
            continue_chain: false,
            ..Default::default()
        }
    }

    /// Result that short-circuits the chain (extension handled the request).
    pub fn handled() -> Self {
        Self {
            handled: true,
            continue_chain: false,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Extension trait
// ---------------------------------------------------------------------------

/// A unified traffic extension.
///
/// Implementors provide request/response hooks and a priority value.
/// Lower priority numbers run first.  The [`ExtensionManager`] sorts
/// registered extensions by priority before each invocation.
pub trait Extension: Send + Sync {
    /// Human-readable name (for logging / debugging).
    fn name(&self) -> &str;

    /// Priority — lower values run first.  Default is `0`.
    fn priority(&self) -> i32 {
        0
    }

    /// Whether the extension is currently enabled.
    fn enabled(&self) -> bool {
        true
    }

    /// Called before a request is forwarded to the upstream server.
    fn on_request(&self, _ctx: &mut ExtensionContext) -> ExtensionResult {
        ExtensionResult::pass()
    }

    /// Called after a response is received from the upstream server.
    fn on_response(&self, _ctx: &mut ExtensionContext) -> ExtensionResult {
        ExtensionResult::pass()
    }
}

// ---------------------------------------------------------------------------
// Extension manager
// ---------------------------------------------------------------------------

/// Manages a collection of [`Extension`] trait objects and invokes them in
/// priority order.
pub struct ExtensionManager {
    extensions: RwLock<Vec<Arc<dyn Extension>>>,
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionManager {
    /// Create an empty manager.
    pub fn new() -> Self {
        Self {
            extensions: RwLock::new(Vec::new()),
        }
    }

    /// Register a new extension.
    pub fn register(&self, ext: Arc<dyn Extension>) {
        self.extensions.write().push(ext);
        // Keep sorted by priority so iteration is in execution order.
        self.extensions.write().sort_by_key(|e| e.priority());
    }

    /// Run `on_request` on all enabled extensions in priority order.
    ///
    /// Returns `true` if any extension handled the request (short-circuit).
    pub fn on_request(&self, ctx: &mut ExtensionContext) -> bool {
        let exts = self.extensions.read();
        for ext in exts.iter() {
            if !ext.enabled() {
                continue;
            }
            let result = ext.on_request(ctx);
            if let Some(err) = &result.error {
                tracing::warn!("Extension {} on_request error: {}", ext.name(), err);
            }
            if result.handled || !result.continue_chain {
                return result.handled;
            }
        }
        false
    }

    /// Run `on_response` on all enabled extensions in priority order.
    pub fn on_response(&self, ctx: &mut ExtensionContext) {
        let exts = self.extensions.read();
        for ext in exts.iter() {
            if !ext.enabled() {
                continue;
            }
            let result = ext.on_response(ctx);
            if let Some(err) = &result.error {
                tracing::warn!("Extension {} on_response error: {}", ext.name(), err);
            }
            if !result.continue_chain {
                break;
            }
        }
    }

    /// Number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.read().len()
    }

    /// Whether the manager has no extensions.
    pub fn is_empty(&self) -> bool {
        self.extensions.read().is_empty()
    }
}

// ---------------------------------------------------------------------------
// Adapters for existing systems
// ---------------------------------------------------------------------------

#[cfg(feature = "scripting")]
mod script_adapter {
    use super::*;
    use crate::scripting::{ScriptContext, ScriptHook, ScriptRuntime};

    /// Adapter that wraps a [`ScriptRuntime`] as an [`Extension`].
    pub struct ScriptExtension {
        runtime: Arc<ScriptRuntime>,
    }

    impl ScriptExtension {
        pub fn new(runtime: Arc<ScriptRuntime>) -> Self {
            Self { runtime }
        }
    }

    impl Extension for ScriptExtension {
        fn name(&self) -> &str {
            "scripting"
        }

        fn priority(&self) -> i32 {
            10
        }

        fn enabled(&self) -> bool {
            // ScriptRuntime has no global enable flag; individual scripts
            // are filtered by their own `enabled` field inside
            // `get_scripts_for_hook`.
            true
        }

        fn on_request(&self, ctx: &mut ExtensionContext) -> ExtensionResult {
            let script_ctx = build_script_context(ctx, "on_request");
            let results = self
                .runtime
                .execute_hook(ScriptHook::OnRequest.as_str(), &mut script_ctx.clone());
            let mut modified = false;
            let mut logs = Vec::new();
            let mut errors = Vec::new();
            let mut handled = false;
            // The per-script on_error policy (StopChain/Continue) is
            // enforced inside `execute_hook`, which stops running
            // subsequent scripts when a script errors with StopChain.
            // The results vector only contains scripts that actually ran.
            // Script errors do NOT stop the extension chain (plugins, etc.)
            // — only the script chain.
            for r in results {
                if r.modified {
                    modified = true;
                    // Apply modified request fields back to the extension context.
                    if let Some(req) = r.modified_request {
                        apply_request_modifications(ctx, &req);
                    }
                }
                if !r.continue_ {
                    // Script short-circuited — write the custom response and
                    // stop the chain.
                    if let Some(resp) = r.response {
                        ctx.response = Some(ExtensionResponse {
                            status_code: resp.status_code,
                            status_message: None,
                            headers: resp.headers,
                            body: Some(resp.body.into_bytes()),
                            content_type: None,
                            duration_ms: 0,
                        });
                        handled = true;
                    }
                }
                if let Some(e) = r.error {
                    errors.push(e);
                }
                logs.extend(r.console);
            }
            ExtensionResult {
                modified,
                logs,
                error: errors.into_iter().next(),
                continue_chain: !handled,
                handled,
            }
        }

        fn on_response(&self, ctx: &mut ExtensionContext) -> ExtensionResult {
            let script_ctx = build_script_context(ctx, "on_response");
            let results = self
                .runtime
                .execute_hook(ScriptHook::OnResponse.as_str(), &mut script_ctx.clone());
            let mut modified = false;
            let mut logs = Vec::new();
            let mut errors = Vec::new();
            // Per-script on_error is enforced inside `execute_hook`.
            for r in results {
                if r.modified {
                    modified = true;
                    // Apply modified response fields back to the extension context.
                    if let Some(resp) = r.modified_response {
                        apply_response_modifications(ctx, &resp);
                    }
                }
                if let Some(e) = r.error {
                    errors.push(e);
                }
                logs.extend(r.console);
            }
            ExtensionResult {
                modified,
                logs,
                error: errors.into_iter().next(),
                continue_chain: true,
                handled: false,
            }
        }
    }

    fn build_script_context(ctx: &ExtensionContext, hook: &str) -> ScriptContext {
        use crate::scripting::{RequestContext, ResponseContext};

        let request = ctx.request.as_ref().map(|r| RequestContext {
            method: r.method.clone(),
            url: r.url.clone(),
            host: r.host.clone(),
            path: r.path.clone(),
            headers: r.headers.clone(),
            body: r
                .body
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).to_string()),
            content_type: r.content_type.clone(),
            query: Default::default(),
        });

        let response = ctx.response.as_ref().map(|r| ResponseContext {
            status_code: r.status_code,
            status_message: r.status_message.clone(),
            headers: r.headers.clone(),
            body: r
                .body
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).to_string()),
            content_type: r.content_type.clone(),
            duration_ms: r.duration_ms,
        });

        ScriptContext {
            request_id: ctx.request_id.clone(),
            session_id: ctx.session_id.clone(),
            request,
            response,
            websocket: None,
            grpc: None,
            data: ctx.data.clone(),
            hook: hook.to_string(),
            timestamp: ctx.timestamp,
        }
    }

    /// Apply modified request fields from a script back to the
    /// [`ExtensionContext`].
    fn apply_request_modifications(
        ctx: &mut ExtensionContext,
        req: &crate::scripting::RequestContext,
    ) {
        if let Some(ext_req) = ctx.request.as_mut() {
            ext_req.method = req.method.clone();
            ext_req.url = req.url.clone();
            ext_req.host = req.host.clone();
            ext_req.path = req.path.clone();
            ext_req.headers = req.headers.clone();
            if let Some(ref body) = req.body {
                ext_req.body = Some(body.as_bytes().to_vec());
            }
            ext_req.content_type = req.content_type.clone();
        }
    }

    /// Apply modified response fields from a script back to the
    /// [`ExtensionContext`].
    fn apply_response_modifications(
        ctx: &mut ExtensionContext,
        resp: &crate::scripting::ResponseContext,
    ) {
        if let Some(ext_resp) = ctx.response.as_mut() {
            ext_resp.status_code = resp.status_code;
            ext_resp.status_message = resp.status_message.clone();
            ext_resp.headers = resp.headers.clone();
            if let Some(ref body) = resp.body {
                ext_resp.body = Some(body.as_bytes().to_vec());
            }
            ext_resp.content_type = resp.content_type.clone();
            ext_resp.duration_ms = resp.duration_ms;
        }
    }
}

#[cfg(feature = "scripting")]
pub use script_adapter::ScriptExtension;

#[cfg(feature = "plugins")]
mod plugin_adapter {
    use super::*;
    use crate::plugin::{PluginContext, PluginHook, PluginManager, PluginRequest, PluginResponse};

    /// Adapter that wraps a [`PluginManager`] as an [`Extension`].
    pub struct PluginExtension {
        manager: Arc<PluginManager>,
    }

    impl PluginExtension {
        pub fn new(manager: Arc<PluginManager>) -> Self {
            Self { manager }
        }
    }

    impl Extension for PluginExtension {
        fn name(&self) -> &str {
            "plugins"
        }

        fn priority(&self) -> i32 {
            20
        }

        fn enabled(&self) -> bool {
            self.manager.is_enabled()
        }

        fn on_request(&self, ctx: &mut ExtensionContext) -> ExtensionResult {
            let pctx = build_plugin_context(ctx, "on_request");
            let results = self.manager.execute_hook(PluginHook::OnRequest, pctx);
            let mut modified = false;
            let mut logs = Vec::new();
            let mut errors = Vec::new();
            for (_id, r) in results {
                if r.modified {
                    modified = true;
                }
                if let Some(e) = r.error {
                    errors.push(e);
                }
                logs.extend(r.logs);
            }
            ExtensionResult {
                modified,
                logs,
                error: errors.into_iter().next(),
                continue_chain: true,
                handled: false,
            }
        }

        fn on_response(&self, ctx: &mut ExtensionContext) -> ExtensionResult {
            let pctx = build_plugin_context(ctx, "on_response");
            let results = self.manager.execute_hook(PluginHook::OnResponse, pctx);
            let mut modified = false;
            let mut logs = Vec::new();
            let mut errors = Vec::new();
            for (_id, r) in results {
                if r.modified {
                    modified = true;
                }
                if let Some(e) = r.error {
                    errors.push(e);
                }
                logs.extend(r.logs);
            }
            ExtensionResult {
                modified,
                logs,
                error: errors.into_iter().next(),
                continue_chain: true,
                handled: false,
            }
        }
    }

    fn build_plugin_context(ctx: &ExtensionContext, hook: &str) -> PluginContext {
        let request = ctx.request.as_ref().map(|r| PluginRequest {
            method: r.method.clone(),
            url: r.url.clone(),
            host: r.host.clone(),
            path: r.path.clone(),
            headers: r.headers.clone(),
            body: r.body.clone(),
            content_type: r.content_type.clone(),
        });

        let response = ctx.response.as_ref().map(|r| PluginResponse {
            status_code: r.status_code,
            status_message: r.status_message.clone(),
            headers: r.headers.clone(),
            body: r.body.clone(),
            content_type: r.content_type.clone(),
            duration_ms: r.duration_ms,
        });

        PluginContext {
            plugin_id: String::new(),
            request_id: Some(ctx.request_id.clone()),
            session_id: Some(ctx.session_id.clone()),
            hook: hook.to_string(),
            request,
            response,
            settings: Default::default(),
            state: ctx.data.clone(),
            timestamp: ctx.timestamp,
        }
    }
}

#[cfg(feature = "plugins")]
pub use plugin_adapter::PluginExtension;
