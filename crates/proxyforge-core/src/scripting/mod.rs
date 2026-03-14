//! Scripting system for ProxyForge
//!
//! This module provides JavaScript/TypeScript scripting capabilities for
//! custom traffic manipulation, filtering, and automation.

mod api;
mod hooks;
mod runtime;

pub use api::ScriptApi;
pub use hooks::{ScriptContext, ScriptHook, ScriptResult};
pub use runtime::{Script, ScriptConfig, ScriptExecution, ScriptRuntime, ScriptTemplates};
