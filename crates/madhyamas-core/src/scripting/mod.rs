//! Scripting system for Madhyamas
//!
//! This module provides JavaScript/TypeScript scripting capabilities for
//! custom traffic manipulation, filtering, and automation.  Scripts are
//! executed by an embedded [`boa_engine`] runtime (see [`engine::JsEngine`]).
//!
//! # Hooks
//!
//! Scripts subscribe to one or more hooks (`on_request`, `on_response`, …)
//! and define a matching JS function (`onRequest`, `onResponse`, …).  The
//! runtime calls the function with the current request/response and a context
//! object, and applies the returned modifications back to the proxy pipeline.

mod api;
mod engine;
mod hooks;
mod persistence;
mod runtime;

pub use api::ScriptApi;
pub use engine::JsEngine;
pub use hooks::{RequestContext, ResponseContext, ScriptContext, ScriptHook, ScriptResult};
pub use runtime::{
    Script, ScriptConfig, ScriptErrorPolicy, ScriptExecution, ScriptMatch, ScriptRuntime,
    ScriptTemplates, UpdateScriptFields,
};
