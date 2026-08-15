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
//!   directories and re-populates the in-memory catalog (plus built-ins), and
//!   (when a registry URL is configured and reachable) fetches the remote
//!   catalog.
//! - **Manager refresh** — [`PluginManager::refresh`] re-scans plugin
//!   directories, reloading manifests and preserving user settings.
//! - **Dependency version constraints** — plugin dependencies are validated
//!   against semver [`semver::VersionReq`] constraints.
//! - **WASM code execution** — when the `wasm-runtime` feature is enabled,
//!   [`PluginManager::execute_hook`] dispatches to the plugin's `plugin.wasm`
//!   module via a sandboxed `wasmtime` runtime (see [`WasmRuntime`]). Fuel
//!   metering bounds CPU; manifest-only plugins (no `.wasm`) are no-ops.
//! - **Persistence** — plugin enabled state, settings, and an invocation
//!   audit log are stored in SQLite via [`PluginPersistence`].
//! - **Install / uninstall** — [`PluginInstaller`] downloads a plugin zip
//!   from a URL or registry entry, verifies its SHA-256 checksum, extracts
//!   it to the plugin directory, and loads it. Ed25519 signature
//!   verification is supported when a publisher public key is configured.
//!
//! What is **not** implemented yet:
//! - **Network access for plugins** — the `http_fetch` host function is not
//!   linked in v1 (the `Network` capability is declared-only). See
//!   `docs/PLUGIN_SECURITY.md`.

mod event_bus;
mod hooks;
mod hot_reload;
mod installer;
mod manager;
mod persistence;
mod registry;
mod signing;
mod templates;
mod types;

#[cfg(feature = "wasm-runtime")]
mod wasm_runtime;

pub use event_bus::PluginEventBus;
pub use hooks::{PluginContext, PluginHook, PluginRequest, PluginResponse, PluginResult};
#[cfg(feature = "wasm-runtime")]
pub use hot_reload::HotReloader;
pub use installer::{InstallResult, InstallSource, PluginInstaller};
pub use manager::PluginManager;
pub use persistence::{PluginInvocationRow, PluginStateRow};
pub use registry::PluginRegistry;
pub use signing::{
    bytes_to_hex, generate_keypair, hex_to_bytes, sign_package, verify_package, PluginKeypair,
};
pub use templates::{PluginTemplate, PluginTemplates, TemplateId};
pub use types::{
    Plugin, PluginCapability, PluginError, PluginManifest, PluginPanel, PluginPanelContent,
    PluginPanelKind, PluginSettingField, PluginSettingType, PluginSettingsSchema, PluginState,
    PluginStats,
};

#[cfg(feature = "wasm-runtime")]
pub use wasm_runtime::WasmRuntime;
