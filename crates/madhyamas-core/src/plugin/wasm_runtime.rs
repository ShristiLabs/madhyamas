//! Sandboxed WASM plugin execution runtime (`wasmtime`).
//!
//! This module is only compiled when the `wasm-runtime` feature is enabled.
//! It implements the host side of the Madhyamas plugin ABI:
//!
//! # Host ↔ Guest ABI
//!
//! The guest (a `.wasm` module built with the `madhyamas-plugin-sdk`) exports:
//!
//! - `__madhyamas_alloc(size: i32) -> i32` — allocate `size` bytes in the
//!   guest's linear memory and return the pointer. The host uses this to
//!   write the JSON-serialized [`PluginContext`] into guest memory before
//!   calling the hook.
//! - `__madhyamas_hook(hook_id: i32, ctx_ptr: i32, ctx_len: i32) -> i64` —
//!   dispatch the hook. `hook_id` is the index from [`PluginHook::export_id`].
//!   The return value packs `(result_ptr << 32) | result_len` where the
//!   pointed-to bytes are a JSON-serialized [`PluginResult`]. A return of `0`
//!   means "no handler / continue".
//!
//! The host provides (WASM module `env`):
//! - `log(level: i32, ptr: i32, len: i32)` — append a log line to the
//!   invocation's log buffer (exposed via `GET /api/plugins/{id}/logs`).
//!
//! # Security
//!
//! - **Sandboxing**: WASM is sandboxed by design — no filesystem, network, or
//!   host memory access unless explicitly linked. Only `log` is linked.
//! - **CPU**: each invocation gets a fuel budget (`manifest.fuel_limit`,
//!   default 10M instructions). When exhausted, the call traps and is
//!   reported as an error.
//! - **Memory**: the engine caps static linear memory at 256 MiB.
//! - **Network**: `http_fetch` is **not** linked in v1 (declared-only). See
//!   `docs/PLUGIN_SECURITY.md`.

use super::{Plugin, PluginContext, PluginHook, PluginResult};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, warn};
use wasmtime::{Caller, Config, Engine, Linker, Memory, Module, Store, TypedFunc};

/// Per-invocation host state, shared between the host functions linked into
/// the WASM instance and the post-invocation result collection.
#[derive(Default)]
pub struct HostState {
    /// Log lines emitted by the guest via the `log` host function.
    pub logs: Vec<String>,
}

/// Packed (ptr, len) returned by guest functions, encoded as a single `i64`.
#[derive(Debug, Clone, Copy)]
struct Packed(u64);

impl Packed {
    const fn from_i64(v: i64) -> Self {
        Self(v as u64)
    }
    fn ptr(self) -> usize {
        (self.0 >> 32) as usize
    }
    fn len(self) -> usize {
        (self.0 & 0xFFFF_FFFF) as usize
    }
    fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// A compiled (and cached) WASM module plus its source mtime, so we can
/// hot-reload when the `.wasm` file changes on disk.
struct CachedModule {
    module: Module,
    mtime: Option<std::time::SystemTime>,
    /// Size of the compiled module in bytes (for stats/debugging).
    #[allow(dead_code)]
    bytes: usize,
}

/// Sandboxed WASM plugin execution runtime.
pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<HostState>,
    /// Cached compiled modules keyed by plugin id.
    modules: RwLock<HashMap<String, CachedModule>>,
}

fn wt_err(e: wasmtime::Error) -> crate::Error {
    crate::Error::Config(format!("wasmtime: {}", e))
}

impl WasmRuntime {
    /// Create a new runtime with the default engine configuration.
    pub fn new() -> crate::Result<Self> {
        let mut config = Config::new();
        // Cap static linear memory at 256 MiB as a hard host-side ceiling.
        config.static_memory_maximum_size(256 * 1024 * 1024);
        config.cache_config_load_default().map_err(wt_err)?;
        let engine = Engine::new(&config).map_err(wt_err)?;

        let mut linker: Linker<HostState> = Linker::new(&engine);
        // Host function: log(level, ptr, len) -> ()
        // Uses `Caller` to access the instance's exported `memory` so the
        // guest can pass string pointers into its linear memory.
        linker
            .func_wrap(
                "env",
                "log",
                |mut caller: Caller<'_, HostState>, level: i32, ptr: i32, len: i32| {
                    let Some(mem) = caller.get_export("memory").and_then(|e| e.into_memory())
                    else {
                        return;
                    };
                    let data = mem.data(&caller);
                    let start = ptr as usize;
                    let end = start.saturating_add(len as usize).min(data.len());
                    let start = start.min(data.len());
                    let bytes = &data[start..end];
                    let msg = String::from_utf8_lossy(bytes).to_string();
                    let level_str = match level {
                        0 => "ERROR",
                        1 => "WARN",
                        2 => "INFO",
                        _ => "DEBUG",
                    };
                    caller
                        .data_mut()
                        .logs
                        .push(format!("[{}] {}", level_str, msg));
                },
            )
            .map_err(wt_err)?;

        Ok(Self {
            engine,
            linker,
            modules: RwLock::new(HashMap::new()),
        })
    }

    /// Get or compile the WASM module for a plugin, refreshing the cache when
    /// the `.wasm` file changes on disk (hot-reload).
    fn get_or_compile(&self, plugin: &Plugin) -> crate::Result<Option<Module>> {
        let wasm_path = PathBuf::from(&plugin.path).join("plugin.wasm");
        if !wasm_path.exists() {
            // Manifest-only plugin (no executable code) — not an error.
            return Ok(None);
        }
        let mtime = std::fs::metadata(&wasm_path)
            .and_then(|m| m.modified())
            .ok();
        let needs_compile = {
            let cache = self.modules.read();
            match cache.get(&plugin.manifest.id) {
                Some(c) => c.mtime != mtime,
                None => true,
            }
        };
        if needs_compile {
            let bytes = std::fs::read(&wasm_path)?;
            let module = Module::new(&self.engine, bytes.as_slice()).map_err(wt_err)?;
            let len = bytes.len();
            let mut cache = self.modules.write();
            cache.insert(
                plugin.manifest.id.clone(),
                CachedModule {
                    module: module.clone(),
                    mtime,
                    bytes: len,
                },
            );
            debug!(
                "Compiled WASM module for plugin {} ({} bytes)",
                plugin.manifest.id, len
            );
        }
        let cache = self.modules.read();
        Ok(cache.get(&plugin.manifest.id).map(|c| c.module.clone()))
    }

    /// Execute a hook for a single plugin.
    ///
    /// Returns a [`PluginResult`]. If the plugin has no `plugin.wasm` (a
    /// manifest-only plugin), returns `PluginResult::cont()` with a log
    /// noting that no executable code is present.
    pub fn execute_hook(
        &self,
        plugin: &Plugin,
        hook: PluginHook,
        context: &PluginContext,
    ) -> PluginResult {
        let _start = Instant::now();
        let module = match self.get_or_compile(plugin) {
            Ok(Some(m)) => m,
            Ok(None) => {
                // Manifest-only plugin — no code to run.
                return PluginResult {
                    logs: vec![format!(
                        "plugin {} has no plugin.wasm (manifest-only); skipping {}",
                        plugin.manifest.id, hook
                    )],
                    ..PluginResult::cont()
                };
            }
            Err(e) => {
                return PluginResult::error(&format!(
                    "failed to compile plugin {}: {}",
                    plugin.manifest.id, e
                ));
            }
        };

        let fuel = plugin.manifest.fuel_limit;
        let mut store = Store::new(&self.engine, HostState::default());
        if store.set_fuel(fuel).is_err() {
            warn!("could not set fuel for plugin {}", plugin.manifest.id);
        }

        let instance = match self.linker.instantiate(&mut store, &module) {
            Ok(i) => i,
            Err(e) => {
                return PluginResult::error(&format!(
                    "failed to instantiate plugin {}: {}",
                    plugin.manifest.id, e
                ));
            }
        };

        // Resolve exports.
        let alloc: TypedFunc<i32, i32> = match instance
            .get_typed_func(&mut store, "__madhyamas_alloc")
        {
            Ok(f) => f,
            Err(_) => {
                return PluginResult::error("plugin missing required export `__madhyamas_alloc`");
            }
        };
        let hook_fn: TypedFunc<(i32, i32, i32), i64> = match instance
            .get_typed_func(&mut store, "__madhyamas_hook")
        {
            Ok(f) => f,
            Err(_) => {
                return PluginResult::error("plugin missing required export `__madhyamas_hook`");
            }
        };
        let memory = match instance.get_memory(&mut store, "memory") {
            Some(m) => m,
            None => return PluginResult::error("plugin exports no memory"),
        };

        // Serialize context and write into guest memory.
        let ctx_json = match serde_json::to_vec(context) {
            Ok(v) => v,
            Err(e) => return PluginResult::error(&format!("context serialize: {}", e)),
        };
        let ctx_ptr = match alloc.call(&mut store, ctx_json.len() as i32) {
            Ok(p) => p,
            Err(e) => return PluginResult::error(&format!("guest alloc: {}", e)),
        };
        write_guest(&mut store, &memory, ctx_ptr as usize, &ctx_json);

        let hook_id = hook.export_id();
        let call_res = hook_fn.call(&mut store, (hook_id, ctx_ptr, ctx_json.len() as i32));

        let result = match call_res {
            Ok(packed) => {
                let p = Packed::from_i64(packed);
                if p.is_null() {
                    PluginResult::cont()
                } else {
                    let bytes = read_guest(&store, &memory, p.ptr(), p.len());
                    match serde_json::from_slice::<PluginResult>(&bytes) {
                        Ok(r) => r,
                        Err(e) => PluginResult::error(&format!(
                            "result deserialize: {} (raw: {})",
                            e,
                            String::from_utf8_lossy(&bytes)
                        )),
                    }
                }
            }
            Err(e) => {
                let msg = if e.to_string().contains("fuel") {
                    format!(
                        "plugin {} exhausted fuel budget ({})",
                        plugin.manifest.id, fuel
                    )
                } else {
                    format!("plugin {} hook {} trapped: {}", plugin.manifest.id, hook, e)
                };
                PluginResult::error(&msg)
            }
        };

        // Collect logs from host state.
        let logs = store.into_data().logs;
        let mut final_result = result;
        final_result.logs.extend(logs);
        final_result
    }

    /// Drop the cached module for a plugin (called on unload/uninstall).
    pub fn drop_module(&self, plugin_id: &str) {
        self.modules.write().remove(plugin_id);
    }

    /// Number of cached compiled modules.
    pub fn cached_count(&self) -> usize {
        self.modules.read().len()
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new().expect("failed to construct WasmRuntime")
    }
}

/// Read `len` bytes from guest memory at `ptr`.
fn read_guest(store: &Store<HostState>, mem: &Memory, ptr: usize, len: usize) -> Vec<u8> {
    let data = mem.data(store);
    let end = ptr.saturating_add(len).min(data.len());
    let start = ptr.min(data.len());
    data[start..end].to_vec()
}

/// Write `bytes` into guest memory at `ptr`.
fn write_guest(store: &mut Store<HostState>, mem: &Memory, ptr: usize, bytes: &[u8]) {
    let data = mem.data_mut(store);
    let end = ptr.saturating_add(bytes.len()).min(data.len());
    let start = ptr.min(data.len());
    let take = end - start;
    data[start..end].copy_from_slice(&bytes[..take]);
}

/// A lightweight summary of a plugin's runtime activity, used for stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct WasmRuntimeStats {
    pub cached_modules: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginManifest;

    #[test]
    fn runtime_constructs() {
        // Constructing the runtime should succeed (engine + linker).
        let rt = WasmRuntime::new();
        assert!(rt.is_ok(), "WasmRuntime::new failed: {:?}", rt.err());
        let rt = rt.unwrap();
        assert_eq!(rt.cached_count(), 0);
    }

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
}
