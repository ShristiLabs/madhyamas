//! Plugin system for Madhyamas
//!
//! This module provides a plugin architecture for extending Madhyamas
//! with custom functionality.
//!
//! # Current status
//!
//! What works today:
//! - **Manifest discovery & parsing** — `madhyamas-plugin.toml` / `.json`
//!   manifests are discovered from plugin search directories and parsed into
//!   [`PluginManifest`] values.
//! - **Tilde expansion** — plugin search paths starting with `~` are expanded
//!   to the user's home directory via [`dirs::home_dir`].
//! - **Registry refresh** — [`PluginRegistry::refresh`] re-scans local plugin
//!   directories and re-populates the in-memory catalog (plus built-ins).
//! - **Manager refresh** — [`PluginManager::refresh`] re-scans plugin
//!   directories, reloading manifests and preserving user settings.
//! - **Dependency version constraints** — plugin dependencies are validated
//!   against semver [`semver::VersionReq`] constraints.
//!
//! What is **not** implemented yet (clearly marked with `TODO`/doc comments):
//! - **Plugin code execution** — there is no runtime (WASM via `wasmtime`,
//!   dynamic lib via `libloading`, or embedded scripting) to actually invoke
//!   plugin hook handlers. See [`PluginManager::execute_hook`].
//! - **Remote registry fetch** — [`PluginRegistry::refresh`] does not yet
//!   perform an HTTP fetch from the configured registry URL; it works
//!   offline using built-in + local plugins only.

mod hooks;
mod manager;
mod registry;
mod types;

pub use hooks::{PluginContext, PluginHook, PluginRequest, PluginResponse, PluginResult};
pub use manager::PluginManager;
pub use registry::PluginRegistry;
pub use types::{Plugin, PluginCapability, PluginError, PluginManifest, PluginState, PluginStats};
