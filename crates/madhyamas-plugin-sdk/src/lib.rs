//! Madhyamas Plugin SDK (guest side).
//!
//! This crate lets you write Madhyamas plugins in Rust that compile to a
//! `plugin.wasm` module loaded by the Madhyamas WASM runtime
//! (`madhyamas-core::plugin::WasmRuntime`).
//!
//! # Quick start
//!
//! ```ignore
//! use madhyamas_plugin_sdk::{register_plugin, Plugin, Context, Outcome};
//!
//! struct CorsHelper;
//!
//! impl Plugin for CorsHelper {
//!     fn on_response(&mut self, ctx: &mut Context) -> Outcome {
//!         if let Some(resp) = ctx.response_mut() {
//!             resp.headers
//!                 .insert("Access-Control-Allow-Origin".into(), "*".into());
//!         }
//!         Outcome::Modified
//!     }
//! }
//!
//! register_plugin!(CorsHelper);
//! ```
//!
//! Build with:
//!
//! ```text
//! cargo build --target wasm32-unknown-unknown --release
//! # -> target/wasm32-unknown-unknown/release/my_plugin.wasm
//! ```
//!
//! Rename the artifact to `plugin.wasm` and place it alongside a
//! `madhyamas-plugin.toml` manifest in your plugin directory.
//!
//! # ABI
//!
//! The SDK exports the two functions the host expects:
//! - `__madhyamas_alloc(size: i32) -> i32` — bump-allocate `size` bytes in
//!   linear memory and return the pointer.
//! - `__madhyamas_hook(hook_id: i32, ctx_ptr: i32, ctx_len: i32) -> i64` —
//!   deserialize the [`Context`], dispatch to the registered [`Plugin`]'s
//!   hook method, serialize the [`Outcome`], and return `(ptr << 32) | len`.
//!
//! The host provides `env.log(level, ptr, len)`; call it via [`log`].
//!
//! # Allocation
//!
//! On `wasm32-unknown-unknown` the SDK installs a bump allocator as the
//! global allocator. Each hook invocation runs in a fresh WASM instance, so
//! the bump pointer resetting per instance is acceptable (no free needed).
//! When compiled for a non-WASM target (e.g. host-side tests with the `std`
//! feature), the SDK uses the standard system allocator instead.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::String;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types — these mirror `madhyamas-core::plugin::hooks` and MUST stay
// wire-compatible (field names + types) with the host's serde format.
// ---------------------------------------------------------------------------

/// A request, as seen by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginRequest {
    pub method: String,
    pub url: String,
    pub host: String,
    pub path: String,
    pub headers: alloc::collections::BTreeMap<String, String>,
    pub body: Option<alloc::vec::Vec<u8>>,
    pub content_type: Option<String>,
}

/// A response, as seen by a plugin.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginResponse {
    pub status_code: u16,
    pub status_message: Option<String>,
    pub headers: alloc::collections::BTreeMap<String, String>,
    pub body: Option<alloc::vec::Vec<u8>>,
    pub content_type: Option<String>,
    pub duration_ms: u64,
}

/// The context passed to every hook. Wire-compatible with the host's
/// `PluginContext`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Context {
    pub plugin_id: String,
    pub request_id: Option<String>,
    pub session_id: Option<String>,
    pub hook: String,
    pub request: Option<PluginRequest>,
    pub response: Option<PluginResponse>,
    pub settings: alloc::collections::BTreeMap<String, serde_json::Value>,
    pub state: alloc::collections::BTreeMap<String, serde_json::Value>,
    pub timestamp: String,
}

impl Context {
    pub fn request(&self) -> Option<&PluginRequest> {
        self.request.as_ref()
    }
    pub fn request_mut(&mut self) -> Option<&mut PluginRequest> {
        self.request.as_mut()
    }
    pub fn response(&self) -> Option<&PluginResponse> {
        self.response.as_ref()
    }
    pub fn response_mut(&mut self) -> Option<&mut PluginResponse> {
        self.response.as_mut()
    }
    /// Get a setting by key (as a JSON value).
    pub fn setting(&self, key: &str) -> Option<&serde_json::Value> {
        self.settings.get(key)
    }
}

/// The outcome of a hook. Wire-compatible with the host's `PluginResult`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Outcome {
    pub handled: bool,
    /// `continue_` (trailing underscore) matches the host field name.
    #[serde(rename = "continue_")]
    pub continue_: bool,
    pub modified: bool,
    pub request: Option<PluginRequest>,
    pub response: Option<PluginResponse>,
    pub error: Option<String>,
    pub logs: alloc::vec::Vec<String>,
    pub custom_response: Option<PluginResponse>,
}

impl Outcome {
    /// No-op: continue the chain, no modification.
    pub fn pass() -> Self {
        Self {
            continue_: true,
            ..Default::default()
        }
    }

    /// The plugin modified the request/response.
    pub fn modified() -> Self {
        Self {
            continue_: true,
            modified: true,
            ..Default::default()
        }
    }

    /// The plugin handled the request; return `custom_response` and stop.
    pub fn respond(status: u16, body: &str) -> Self {
        let mut headers = alloc::collections::BTreeMap::new();
        headers.insert("Content-Type".into(), "text/plain".into());
        Self {
            handled: true,
            continue_: false,
            custom_response: Some(PluginResponse {
                status_code: status,
                headers,
                body: Some(body.as_bytes().to_vec()),
                content_type: Some("text/plain".into()),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// The plugin errored; stop the chain with an error message.
    pub fn error(msg: &str) -> Self {
        Self {
            handled: true,
            continue_: false,
            error: Some(msg.into()),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Hook ids — must match `madhyamas-core::plugin::PluginHook::export_id`.
// ---------------------------------------------------------------------------

pub const HOOK_ON_LOAD: i32 = 0;
pub const HOOK_ON_ENABLE: i32 = 1;
pub const HOOK_ON_DISABLE: i32 = 2;
pub const HOOK_ON_UNLOAD: i32 = 3;
pub const HOOK_ON_REQUEST: i32 = 4;
pub const HOOK_ON_RESPONSE: i32 = 5;
pub const HOOK_ON_WEBSOCKET: i32 = 6;
pub const HOOK_ON_GRPC: i32 = 7;
pub const HOOK_ON_SETTINGS_CHANGE: i32 = 8;
pub const HOOK_ON_TIMER: i32 = 9;

// ---------------------------------------------------------------------------
// Plugin trait
// ---------------------------------------------------------------------------

/// A Madhyamas plugin. Implement the hooks you care about; the rest are
/// no-ops that return [`Outcome::pass`].
pub trait Plugin {
    fn on_load(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_enable(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_disable(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_unload(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_request(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_response(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_websocket(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_grpc(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_settings_change(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
    fn on_timer(&mut self, _ctx: &mut Context) -> Outcome {
        Outcome::pass()
    }
}

/// Dispatch a hook id to the plugin's trait method.
#[allow(dead_code)]
fn dispatch(plugin: &mut dyn Plugin, hook_id: i32, ctx: &mut Context) -> Outcome {
    match hook_id {
        HOOK_ON_LOAD => plugin.on_load(ctx),
        HOOK_ON_ENABLE => plugin.on_enable(ctx),
        HOOK_ON_DISABLE => plugin.on_disable(ctx),
        HOOK_ON_UNLOAD => plugin.on_unload(ctx),
        HOOK_ON_REQUEST => plugin.on_request(ctx),
        HOOK_ON_RESPONSE => plugin.on_response(ctx),
        HOOK_ON_WEBSOCKET => plugin.on_websocket(ctx),
        HOOK_ON_GRPC => plugin.on_grpc(ctx),
        HOOK_ON_SETTINGS_CHANGE => plugin.on_settings_change(ctx),
        HOOK_ON_TIMER => plugin.on_timer(ctx),
        _ => Outcome::pass(),
    }
}

// ---------------------------------------------------------------------------
// Host imports + logging
// ---------------------------------------------------------------------------

/// Log levels for [`log`].
pub mod log_level {
    pub const ERROR: i32 = 0;
    pub const WARN: i32 = 1;
    pub const INFO: i32 = 2;
    pub const DEBUG: i32 = 3;
}

/// Emit a log line to the host (visible in `GET /api/plugins/{id}/logs`).
pub fn log(level: i32, message: &str) {
    host_log(level, message.as_ptr(), message.len());
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
    #[link_name = "log"]
    fn _host_log(level: i32, ptr: *const u8, len: i32);
}

#[cfg(target_arch = "wasm32")]
fn host_log(level: i32, ptr: *const u8, len: usize) {
    unsafe { _host_log(level, ptr, len as i32) }
}

#[cfg(not(target_arch = "wasm32"))]
fn host_log(level: i32, ptr: *const u8, len: usize) {
    // Host-side fallback (tests): no-op without std.
    let _ = (level, ptr, len);
}

// ---------------------------------------------------------------------------
// Bump allocator (wasm32) + global allocator wiring
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod bump_alloc {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static HEAP_PTR: AtomicUsize = AtomicUsize::new(0);

    /// Initialize the heap pointer to the end of the current linear memory
    /// (above data/bss/stack). Idempotent.
    unsafe fn ensure_init() -> usize {
        let mut start = HEAP_PTR.load(Ordering::Acquire);
        if start == 0 {
            let pages = core::arch::wasm32::memory_size(0);
            start = pages * 65536;
            HEAP_PTR.store(start, Ordering::Release);
        }
        start
    }

    /// Bump-allocate `size` bytes with `align` alignment, growing memory as
    /// needed. Returns a pointer (offset into linear memory).
    pub unsafe fn bump(size: usize, align: usize) -> *mut u8 {
        let _ = ensure_init();
        loop {
            let cur = HEAP_PTR.load(Ordering::Acquire);
            // Align cur up.
            let aligned = (cur + align - 1) & !(align - 1);
            let next = aligned + size;
            let pages = core::arch::wasm32::memory_size(0);
            let mem_bytes = pages * 65536;
            if next > mem_bytes {
                // Need to grow memory. Grow by enough pages to fit.
                let needed = next - mem_bytes;
                let pages_needed = (needed + 65535) / 65536;
                let prev = core::arch::wasm32::memory_grow(0, pages_needed);
                if prev == usize::MAX {
                    // Out of memory.
                    return core::ptr::null_mut();
                }
                // Retry CAS.
                continue;
            }
            if HEAP_PTR
                .compare_exchange(cur, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return aligned as *mut u8;
            }
            // Lost a race; retry.
        }
    }

    /// A simple global allocator backed by the bump allocator. Never frees
    /// (acceptable: each hook runs in a fresh instance).
    pub struct BumpAllocator;

    unsafe impl core::alloc::GlobalAlloc for BumpAllocator {
        unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
            let ptr = bump(layout.size(), layout.align());
            if ptr.is_null() {
                core::arch::wasm32::unreachable();
            }
            ptr
        }
        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
            // Bump allocator: no-op.
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[global_allocator]
static ALLOCATOR: bump_alloc::BumpAllocator = bump_alloc::BumpAllocator;

// A panic handler is required on `wasm32-unknown-unknown` without std.
// We simply trap; the host reports a trapped invocation as an error.
#[cfg(all(target_arch = "wasm32", not(feature = "std")))]
#[panic_handler]
fn on_panic(_info: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable();
}

// ---------------------------------------------------------------------------
// register_plugin! macro + WASM entry point helpers
// ---------------------------------------------------------------------------

/// Register a plugin type as the WASM module's plugin.
///
/// This macro generates the two WASM entry points (`__madhyamas_alloc` and
/// `__madhyamas_hook`) that the host runtime expects. The plugin instance is
/// lazily constructed on the first hook invocation.
///
/// ```ignore
/// struct MyPlugin;
/// impl Plugin for MyPlugin { /* ... */ }
/// register_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! register_plugin {
    ($ty:ident) => {
        static mut PLUGIN: core::option::Option<&'static mut dyn $crate::Plugin> =
            core::option::Option::None;

        fn ensure_plugin() -> &'static mut dyn $crate::Plugin {
            unsafe {
                if PLUGIN.is_none() {
                    let boxed: alloc::boxed::Box<dyn $crate::Plugin> =
                        alloc::boxed::Box::new($ty::default());
                    PLUGIN = core::option::Option::Some(alloc::boxed::Box::leak(boxed));
                }
                match PLUGIN {
                    core::option::Option::Some(ref mut p) => *p,
                    core::option::Option::None => core::hint::unreachable_unchecked(),
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn __madhyamas_alloc(size: i32) -> i32 {
            $crate::__alloc(size)
        }

        #[no_mangle]
        pub extern "C" fn __madhyamas_hook(hook_id: i32, ctx_ptr: i32, ctx_len: i32) -> i64 {
            let plugin = ensure_plugin();
            $crate::__dispatch_hook(hook_id, ctx_ptr, ctx_len, plugin)
        }
    };
}

/// Bump-allocate `size` bytes in linear memory. Called by the generated
/// `__madhyamas_alloc` export.
#[cfg(target_arch = "wasm32")]
pub fn __alloc(size: i32) -> i32 {
    unsafe {
        let ptr = bump_alloc::bump(size as usize, 1);
        if ptr.is_null() {
            return 0;
        }
        ptr as i32
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn __alloc(_size: i32) -> i32 {
    0
}

/// Read the context from linear memory, dispatch to the plugin, write the
/// result back, and return the packed (ptr, len). Called by the generated
/// `__madhyamas_hook` export.
#[cfg(target_arch = "wasm32")]
pub fn __dispatch_hook(hook_id: i32, ctx_ptr: i32, ctx_len: i32, plugin: &mut dyn Plugin) -> i64 {
    unsafe {
        let ctx_bytes = core::slice::from_raw_parts(ctx_ptr as *const u8, ctx_len as usize);
        let mut ctx: Context = match serde_json::from_slice(ctx_bytes) {
            Ok(c) => c,
            Err(e) => {
                let msg = alloc::format!("context deserialize: {}", e);
                let outcome = Outcome::error(&msg);
                return write_outcome(&outcome);
            }
        };

        let outcome = dispatch(plugin, hook_id, &mut ctx);
        write_outcome(&outcome)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn __dispatch_hook(
    _hook_id: i32,
    _ctx_ptr: i32,
    _ctx_len: i32,
    _plugin: &mut dyn Plugin,
) -> i64 {
    0
}

/// Serialize an outcome, allocate space for it, copy it into linear memory,
/// and return the packed (ptr, len).
#[cfg(target_arch = "wasm32")]
fn write_outcome(outcome: &Outcome) -> i64 {
    let bytes = match serde_json::to_vec(outcome) {
        Ok(b) => b,
        Err(_) => return 0,
    };
    let len = bytes.len();
    let ptr = unsafe { bump_alloc::bump(len, 1) };
    if ptr.is_null() {
        return 0;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
    }
    pack(ptr as usize, len)
}

/// Pack a (ptr, len) into an i64 return value.
#[inline]
#[allow(dead_code)]
fn pack(ptr: usize, len: usize) -> i64 {
    ((ptr as u64) << 32 | (len as u64 & 0xFFFF_FFFF)) as i64
}

// ---------------------------------------------------------------------------
// Tests (host-side, with the `std` feature)
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    struct Cors;
    impl Plugin for Cors {
        fn on_response(&mut self, ctx: &mut Context) -> Outcome {
            if let Some(resp) = ctx.response_mut() {
                resp.headers
                    .insert("Access-Control-Allow-Origin".into(), "*".into());
            }
            Outcome::modified()
        }
    }

    #[test]
    fn dispatch_modifies_response() {
        let mut ctx = Context {
            response: Some(PluginResponse::default()),
            ..Default::default()
        };
        let mut p = Cors;
        let out = dispatch(&mut p, HOOK_ON_RESPONSE, &mut ctx);
        assert!(out.modified);
        assert_eq!(
            ctx.response
                .as_ref()
                .unwrap()
                .headers
                .get("Access-Control-Allow-Origin"),
            Some(&"*".to_string())
        );
    }

    #[test]
    fn outcome_wire_format() {
        // The `continue_` field must serialize as `continue_` to match host.
        let o = Outcome::pass();
        let s = serde_json::to_string(&o).unwrap();
        assert!(s.contains("\"continue_\":true"), "got: {}", s);
    }

    #[test]
    fn outcome_respond() {
        let o = Outcome::respond(403, "blocked");
        assert!(o.handled);
        assert!(!o.continue_);
        assert_eq!(o.custom_response.as_ref().unwrap().status_code, 403);
    }
}
