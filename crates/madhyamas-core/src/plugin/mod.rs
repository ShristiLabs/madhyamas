//! Plugin system for Madhyamas
//!
//! This module provides a plugin architecture for extending Madhyamas
//! with custom functionality written in Rust (as dynamic libraries).

mod hooks;
mod manager;
mod registry;
mod types;

pub use hooks::{PluginContext, PluginHook, PluginResult};
pub use manager::PluginManager;
pub use registry::PluginRegistry;
pub use types::{Plugin, PluginCapability, PluginError, PluginManifest, PluginState, PluginStats};
