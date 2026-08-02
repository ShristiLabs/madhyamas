# Plugin System Analysis

This document is a thorough analysis of the Madhyamas plugin system: what
exists today, the use cases it will serve, what needs to be built, how to
implement it, how to keep it secure, how to enhance it, and what changes are
needed across the backend, frontend, and documentation.

> All file paths are relative to the repository root
> (`madhyamas/`).

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [What Exists Now](#2-what-exists-now)
3. [Use Cases](#3-use-cases)
4. [What Needs to Be Built](#4-what-needs-to-be-built)
5. [How to Implement It](#5-how-to-implement-it)
6. [How to Keep It Secure](#6-how-to-keep-it-secure)
7. [How to Enhance It](#7-how-to-enhance-it)
8. [Backend Changes](#8-backend-changes)
9. [Frontend Changes](#9-frontend-changes)
10. [Documentation Changes](#10-documentation-changes)
11. [Implementation Phases](#11-implementation-phases)

---

## 1. Executive Summary

The plugin system is a **well-structured scaffold with no code execution**.
The infrastructure for discovering, loading, parsing, and managing plugin
manifests is complete and functional — including semver dependency
validation, local directory scanning, tilde expansion, settings
preservation across reloads, and a registry with built-in plugins. But the
`execute_plugin_hook()` method is a documented no-op that returns
`PluginResult::cont()` for every hook invocation. No runtime (WASM, dynamic
library, or embedded scripting) exists to actually run plugin code.

The system is designed for **packaged, distributable plugins** with a
manifest (`madhyamas-plugin.toml`/`.json`), semver versioning, dependency
chains, a settings schema for UI generation, and capability flags. This is
a more structured extensibility model than scripts (which are single-file
JS), intended for reusable, shareable, versioned extensions.

**Current status:** 🟡 Partial — manifest lifecycle complete, execution
missing.
**Recommended runtime:** WASM via `wasmtime` (sandboxed, portable,
language-agnostic) for v1; consider a Rust native plugin ABI for
high-performance use cases later.
**Estimated effort to make functional:** Hard (WASM runtime + host ABI +
guest SDK + security model).

---

## 2. What Exists Now

### 2.1 Core Module (`crates/madhyamas-core/src/plugin/`)

| Component | File | Lines | State |
|---|---|---|---|
| Module root | `mod.rs` | 37 | Exports `PluginContext`, `PluginHook`, `PluginRequest`, `PluginResponse`, `PluginResult`, `PluginManager`, `PluginRegistry`, `Plugin`, `PluginCapability`, `PluginError`, `PluginManifest`, `PluginState`, `PluginStats`. Clear doc comment documenting what works and what doesn't. |
| Types | `types.rs` | 216 | `PluginManifest` (id, name, version, description, author, homepage, repository, min/max_version, license, dependencies, hooks, settings schema, enabled_by_default); `PluginSettingsSchema` with `PluginSettingField` (8 field types: String, Number, Boolean, Select, MultiSelect, Color, Url, Path, Json); `Plugin` (manifest + state + settings + path + loaded_at + error); `PluginState` enum (Loaded, Enabled, Running, Disabled, Error, Unloading); `PluginError` enum (7 variants); `PluginStats` (invocations, total_time_ms, errors, last_invoked); `PluginCapability` enum (8 capabilities) |
| Manager | `manager.rs` | 439 | `PluginManager` — plugin discovery, loading (TOML + JSON manifests), enable/disable, dependency checking with semver, settings management, stats tracking, refresh/reload; **`execute_plugin_hook()` is a no-op** (line 355-396) |
| Registry | `registry.rs` | 440 | `PluginRegistry` — local + built-in plugin catalog with search, list, list_by_capability, get_popular, get_top_rated; 3 built-in plugins (CORS Helper, Request Logger, API Mock Helper); **remote registry fetch is NOT implemented** (line 138-148, documented TODO) |
| Hooks | `hooks.rs` | 234 | `PluginHook` enum (10 hooks: OnLoad, OnEnable, OnDisable, OnUnload, OnRequest, OnResponse, OnWebSocket, OnGrpc, OnSettingsChange, OnTimer); `PluginContext` (plugin_id, request_id, session_id, hook, request, response, settings, state, timestamp); `PluginRequest`/`PluginResponse` with `From<&RequestData>`/`From<&ResponseData>`; `PluginResult` (handled, continue_, modified, request, response, error, logs, custom_response) with constructors (cont, modified, error, respond) |

### 2.2 What Actually Works

| Feature | Status | Location |
|---|---|---|
| Manifest discovery (TOML + JSON) | ✅ | `manager.rs:82-105` (`discover_plugins`) |
| Manifest parsing | ✅ | `manager.rs:108-152` (`load_plugin`) |
| Semver version validation | ✅ | `manager.rs:132-138` — validates plugin version is valid semver |
| Dependency checking with semver constraints | ✅ | `manager.rs:251-289` (`check_dependencies`) — uses `semver::VersionReq` |
| Plugin enable/disable | ✅ | `manager.rs:214-244` |
| Plugin unload | ✅ | `manager.rs:201-211` |
| Plugin refresh/reload | ✅ | `manager.rs:161-198` — re-scans dirs, preserves settings across reloads |
| Settings management | ✅ | `manager.rs:399-409` (`update_settings`) |
| Stats tracking (invocations, time, errors) | ✅ | `manager.rs:362-396` — but stats only record no-op invocations |
| Tilde expansion in plugin paths | ✅ | `manager.rs:51-66` (`expand_tilde`) |
| Registry: built-in plugins | ✅ | `registry.rs:209-324` — 3 built-in plugins |
| Registry: local directory scanning | ✅ | `registry.rs:156-206` (`scan_local_dirs`) |
| Registry: search by name/description/tags | ✅ | `registry.rs:327-352` |
| Registry: list by capability | ✅ | `registry.rs:373-387` |
| Registry: popular / top-rated sorting | ✅ | `registry.rs:390-415` |

### 2.3 What Does NOT Work

| Feature | Status | Location | Note |
|---|---|---|---|
| **Plugin code execution** | ❌ | `manager.rs:355-396` | `execute_plugin_hook()` returns `PluginResult::cont()` — documented TODO with candidate approaches (wasmtime, libloading, embedded scripting) |
| **Remote registry fetch** | ❌ | `registry.rs:138-148` | `refresh()` only scans local dirs + built-ins; HTTP fetch from `registry.madhyamas.dev` is a documented TODO |
| **Plugin download/install** | ❌ | — | No code to download a plugin from a URL, verify checksum, and install to a local directory |
| **Plugin settings UI generation** | ❌ | — | `PluginSettingsSchema` and `PluginSettingField` types exist but the web UI doesn't render settings forms from them |
| **Plugin lifecycle hooks** | ❌ | — | `OnLoad`, `OnEnable`, `OnDisable`, `OnUnload` are defined but never called |
| **Plugin timer hook** | ❌ | — | `OnTimer` is defined but no timer scheduling exists |
| **Persistence** | ❌ | — | Plugin settings and enabled state are in-memory only; lost on restart |

### 2.4 Plugin Manifest Format

```toml
# madhyamas-plugin.toml
id = "my-company.api-inspector"
name = "API Inspector"
version = "1.2.0"
description = "Deep inspection of REST API responses"
author = "Jane Developer"
homepage = "https://github.com/jane/api-inspector"
repository = "https://github.com/jane/api-inspector"
license = "MIT"
min_version = "0.5.0"
enabled_by_default = false

[dependencies]
"my-company.core" = "^1.0"

hooks = ["on_request", "on_response"]

[settings]
[[settings.fields]]
key = "inspect_depth"
label = "Inspection Depth"
description = "How deep to inspect nested JSON"
field_type = "select"
default = "shallow"
required = true
options = ["shallow", "medium", "deep"]
```

### 2.5 Built-in Plugins (`registry.rs:209-324`)

| Plugin ID | Name | Hook | Capabilities | Default |
|---|---|---|---|---|
| `madhyamas.cors-helper` | CORS Helper | `on_response` | InterceptResponse | Enabled |
| `madhyamas.request-logger` | Request Logger | `on_request` | InterceptRequest | Disabled |
| `madhyamas.mock-helper` | API Mock Helper | `on_request` | InterceptRequest, UiPanel | Disabled |

These are **manifest-only** — they have no executable code. They appear in
the registry but do nothing when enabled.

### 2.6 Extension Manager Integration (`extension.rs:371-481`)

`PluginExtension` adapts `PluginManager` to the unified `Extension` trait:
- Priority: 20 (runs after scripting at 10)
- `enabled()` checks `manager.is_enabled()` (global on/off)
- `on_request()` / `on_response()` build a `PluginContext`, call
  `manager.execute_hook()`, and aggregate results
- Registered in `main.rs:643-650`; pipeline dispatches via
  `extension_manager` at `pipeline.rs:303-305` and `pipeline.rs:429-431`

### 2.7 API Layer (`crates/madhyamas-api/src/phase3_handlers.rs:208-295`)

| Endpoint | Method | Handler | State |
|---|---|---|---|
| `/api/plugins` | GET | `get_plugins` | ✅ Returns all loaded plugins |
| `/api/plugins/{id}` | GET | `get_plugin` | ✅ Returns single plugin |
| `/api/plugins/{id}/enable` | POST | `enable_plugin` | ✅ Enables plugin (checks dependencies) |
| `/api/plugins/{id}/disable` | POST | `disable_plugin` | ✅ Disables plugin |
| `/api/plugins/{id}/stats` | GET | `get_plugin_stats` | ✅ Returns invocation stats |
| `/api/plugins/reload` | POST | `reload_plugins` | ✅ Re-scans and reloads all plugins |

Feature-gated behind `#[cfg(feature = "plugins")]`, registered in
`routes.rs:367-380`. **Missing endpoints**: install, uninstall, search
registry, get settings schema, update settings, get/set global enable.

### 2.8 CLI (`crates/madhyamas-cli/src/commands/plugins.rs`)

| Command | State |
|---|---|
| `madhyamas plugins list` | ✅ |
| `madhyamas plugins get <id>` | ✅ |
| `madhyamas plugins enable <id>` | ✅ |
| `madhyamas plugins disable <id>` | ✅ |
| `madhyamas plugins stats <id>` | ✅ |
| `madhyamas plugins reload` | ✅ |

**Missing**: `install`, `uninstall`, `search`, `settings get/set`,
`registry list`.

### 2.9 MCP Tools (`crates/madhyamas-mcp/src/tools/plugins.rs`)

| Tool | State |
|---|---|
| `madhyamas_list_plugins` | ✅ |
| `madhyamas_get_plugin` | ✅ |
| `madhyamas_enable_plugin` | ✅ |
| `madhyamas_disable_plugin` | ✅ |
| `madhyamas_get_plugin_stats` | ✅ |
| `madhyamas_reload_plugins` | ✅ |

Registered in `registry.rs:1187-1260`. **Missing**: install, search, settings.

### 2.10 Web UI (`web/src/features/tools/PluginsPanel.tsx`)

A panel with:
- **Plugin list**: searchable list with expand/collapse, state badge
  (enabled/disabled/error), enable/disable toggle, version display
- **Plugin detail** (expanded): description, ID, author, hooks, stats
  (invocations, errors, avg duration)
- **Reload button**: re-scans plugin directories
- **API hooks**: `usePlugins`, `usePlugin`, `usePluginStats`,
  `useEnablePlugin`, `useDisablePlugin`, `useReloadPlugins` in
  `web/src/lib/api/phase3.ts:248-311`

**Missing**: settings UI (form generation from schema), registry browser,
install/uninstall, plugin code viewer, error detail display.

### 2.11 Feature Gate

`crates/madhyamas-core/Cargo.toml:11` — `default = ["grpc", "scripting",
"plugins", "enterprise"]`. The `plugins` feature is a bare flag; no runtime
dependency (`wasmtime`, `libloading`, etc.) is included.

### 2.12 Plugin Directories

Scanned in this order (`manager.rs:41`):
1. `./plugins` (current working directory)
2. `~/.madhyamas/plugins` (user home directory)

Additional directories can be added via `add_plugin_dir()` (line 73).

---

## 3. Use Cases

### 3.1 Reusable, Distributable Extensions

Unlike scripts (single-file, user-specific), plugins are packaged,
versioned, and distributable. A plugin can be developed once and shared
across teams or the community.

**Example:** A "GraphQL Inspector" plugin that parses GraphQL queries,
validates them against a schema, and highlights N+1 query patterns.
Distributed via the plugin registry, installable with one command.

### 3.2 Language-Agnostic Extensibility (WASM)

If plugins run as WASM modules, they can be written in any language that
compiles to WASM — Rust, C/C++, Go, AssemblyScript, Python (via Pyodide).
This lowers the barrier to contribution compared to a Rust-only or
JS-only plugin model.

**Example:** A team that writes Go writes a plugin in Go that compiles to
WASM and integrates with Madhyamas without learning Rust or JavaScript.

### 3.3 UI Panel Extensions

Plugins with the `UiPanel` capability can add custom panels to the web UI
— visualizations, specialized viewers, or workflow tools that integrate
with the traffic data.

**Example:** A "Performance Waterfall" plugin that adds a custom waterfall
chart panel with flame-graph detail, computed from captured traffic.

### 3.4 Custom Export/Import Formats

Plugins with `ExportFormat`/`ImportFormat` capabilities can add new
export/import formats beyond the built-in HAR and cURL.

**Example:** A "Postman Collection" plugin that exports captured traffic as
a Postman collection JSON, or a "JMeter" plugin that exports as a JMX test
plan.

### 3.5 Theme Customization

Plugins with the `Theme` capability can provide custom CSS themes for the
web UI.

**Example:** A "Dark Pro" theme plugin that overrides the default color
scheme with a higher-contrast dark theme.

### 3.6 Protocol Decoders

Plugins with `InterceptRequest`/`InterceptResponse` capabilities can
decode proprietary or industry-specific protocols that Madhyamas doesn't
support natively.

**Example:** A "Protobuf Decoder" plugin that loads `.proto` descriptors
and decodes gRPC messages with full field names and types (complementing
the built-in schema-less decoder).

### 3.7 Integration with External Systems

Plugins can integrate Madhyamas with external systems — CI servers, issue
trackers, logging platforms, security scanners.

**Example:** A "JIRA Integration" plugin that, when a 500 error is
captured, automatically creates a JIRA ticket with the request/response
details attached.

### 3.8 Enterprise-Specific Customization

Organizations can develop internal plugins for company-specific debugging
workflows, compliance checks, or security policies that aren't suitable
for the public registry.

**Example:** An "ACME Corp Security Policy" plugin that blocks requests to
external domains and logs violations to the corporate SIEM.

---

## 4. What Needs to Be Built

### 4.1 Critical (Make Plugins Actually Run)

1. **Choose and integrate a plugin runtime** — WASM (`wasmtime`) is
   recommended for sandboxing and language agnosticism
2. **Define the host ABI** — the interface between Madhyamas (host) and
   plugin (guest): what functions the host exposes, what functions the
   guest must export, how data is passed (shared memory, message passing)
3. **Create a guest SDK** — a library (in Rust, and optionally other
   languages) that plugin authors use to implement hooks without dealing
   with WASM ABI details
4. **Implement hook dispatch** — load the plugin's WASM module, find the
   exported hook function, call it with the context data, parse the result
5. **Apply plugin results** — write modifications back to
   `RequestData`/`ResponseData`; handle short-circuit responses
6. **Implement lifecycle hooks** — call `on_load`, `on_enable`,
   `on_disable`, `on_unload` at the appropriate times
7. **Add persistence** — store plugin enabled state and settings in SQLite

### 4.2 Important (Distribution and Management)

8. **Remote registry fetch** — implement HTTP fetch from
   `registry.madhyamas.dev` (or a configurable URL) with checksum
   verification
9. **Plugin install/uninstall** — download a plugin package, verify
   checksum, extract to the plugin directory, load it
10. **Settings UI generation** — render a settings form from
    `PluginSettingsSchema` in the web UI
11. **Plugin settings persistence** — store settings in SQLite, restore on
    reload
12. **Plugin error handling** — surface plugin errors (load failure,
    execution error, dependency missing) in the web UI with actionable
    messages

### 4.3 Nice-to-Have (Enhancement)

13. **UI panel rendering** — render plugin-provided UI panels in the web UI
    (via an iframe or a custom React component protocol)
14. **Plugin sandboxing with resource limits** — CPU time, memory, network
    access controls per plugin
15. **Plugin signing / verification** — verify plugin signatures from
    trusted publishers
16. **Plugin development toolkit** — scaffolding, testing, and packaging
    tools for plugin authors
17. **Hot-reload** — reload a plugin's WASM module without restarting the
    proxy
18. **Timer hook scheduling** — implement the `OnTimer` hook with
    configurable intervals per plugin

---

## 5. How to Implement It

### 5.1 Runtime Selection

| Runtime | Pros | Cons | Recommendation |
|---|---|---|---|
| **`wasmtime`** (WASM) | Sandboxed by design, language-agnostic, portable, mature, WASI support, resource limits (fuel/gas) | Heavy dependency (~10MB), adds build complexity, indirect function calls (slower than native) | **Recommended for v1** — best security story, broadest language support |
| **`libloading`** (dynamic lib) | Fastest (native calls), no overhead | Unsafe (can crash the process), platform-specific (`.so`/`.dylib`/`.dll`), no sandboxing, ABI stability issues | Only for trusted, first-party plugins |
| **Embedded scripting** (`boa`, `rune`, `mlua`) | Simple, sandboxed, no native deps | Limited to one language, slower than WASM, less structured than a plugin model | Overlaps with the scripting system; not recommended for plugins |
| **`extism`** (WASM plug-in framework) | Built on wasmtime, provides a plug-in SDK, handles ABI, multi-language SDKs | Additional abstraction layer, less control | Good alternative to raw wasmtime — consider for faster development |

**Recommended: `wasmtime`** (or `extism` for a higher-level API). WASM
provides the strongest sandboxing story (critical for running third-party
code), supports plugins written in any WASM-targeting language, and has
built-in resource limiting (fuel/gas metering).

### 5.2 Host ABI Design

The host (Madhyamas) exposes functions that plugins can call, and plugins
export functions that the host calls. This is the ABI.

**Host functions (available to plugins):**
```rust
// Exposed to WASM guests via wasmtime::Linker
fn log(level: u32, message: ptr, len: u32)          // Logging
fn get_request_header(key: ptr, klen: u32) -> ptr   // Read request header
fn set_request_header(key: ptr, klen: u32, val: ptr, vlen: u32)  // Set header
fn get_request_body() -> ptr                        // Read request body
fn set_request_body(ptr, len: u32)                  // Set request body
fn get_response_header(key: ptr, klen: u32) -> ptr  // Read response header
fn set_response_header(key: ptr, klen: u32, ...)    // Set response header
fn get_response_body() -> ptr                       // Read response body
fn set_response_body(ptr, len: u32)                 // Set response body
fn get_setting(key: ptr, klen: u32) -> ptr          // Read plugin setting
fn http_fetch(url: ptr, len: u32) -> i32            // Network access (if allowed)
```

**Guest functions (exported by plugins):**
```rust
// Exported by the WASM module, called by the host
fn on_request(request_id: ptr, len: u32) -> u32     // Returns: 0=continue, 1=modified, 2=handled
fn on_response(request_id: ptr, len: u32) -> u32
fn on_load() -> u32
fn on_enable() -> u32
fn on_disable() -> u32
fn on_unload() -> u32
```

**Data passing:** Use shared linear memory (WASM's default memory model).
The host writes data into the guest's linear memory, calls the function,
and reads the result back. For complex data, use a JSON serialization
layer (host serializes to JSON, writes to guest memory, guest deserializes).

### 5.3 Guest SDK

Create a Rust crate `madhyamas-plugin-sdk` that wraps the raw WASM ABI:

```rust
// In a plugin's Rust code:
use madhyamas_plugin_sdk::{Plugin, RequestContext, ResponseContext, Result};

#[madhyamas_plugin::plugin]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn on_request(&self, ctx: &mut RequestContext) -> Result {
        ctx.set_header("X-Plugin-Active", "true")?;
        Ok(Result::Modified)
    }
}
```

The SDK macro generates the WASM exports and handles memory management.
Plugin authors write idiomatic Rust without touching WASM ABI.

### 5.4 Plugin Package Format

A plugin is a directory containing:
```
my-plugin/
├── madhyamas-plugin.toml    # Manifest
├── plugin.wasm              # Compiled WASM module
├── README.md                # Documentation
├── LICENSE                  # License file
└── settings.schema.json     # Settings schema (optional, can be in manifest)
```

For distribution, the directory is zipped:
`my-plugin-1.0.0.zip`. The registry stores the zip with a SHA-256 checksum.

### 5.5 Execution Flow

```
┌─────────────────────────────────────────────────────┐
│                   Proxy Pipeline                     │
│  (pipeline.rs: run_request_hooks / run_response_hooks)│
└──────────────────┬──────────────────────────────────┘
                   │ ExtensionContext
                   ▼
┌─────────────────────────────────────────────────────┐
│              ExtensionManager (extension.rs)         │
│  dispatches to PluginExtension (priority 20)         │
└──────────────────┬──────────────────────────────────┘
                   │ PluginContext
                   ▼
┌─────────────────────────────────────────────────────┐
│              PluginManager (manager.rs)              │
│  execute_hook() → for each matching plugin:          │
│    1. Get/create WasmInstance (cached per plugin)    │
│    2. Serialize context to JSON                      │
│    3. Write JSON to guest linear memory              │
│    4. Call exported hook function (on_request)       │
│    5. Read result from guest memory                  │
│    6. Deserialize result → PluginResult              │
│    7. Apply modifications to RequestData/ResponseData│
│    8. Update stats                                   │
└──────────────────┬──────────────────────────────────┘
                   │ wasmtime
                   ▼
┌─────────────────────────────────────────────────────┐
│              wasmtime::Instance                       │
│  - Sandboxed WASM execution                          │
│  - Fuel/gas metering (CPU limit)                     │
│  - No host access unless explicitly linked           │
│  - Fresh instance per hook call (or pooled)          │
└─────────────────────────────────────────────────────┘
```

### 5.6 Implementation Details

**Step 1: Add dependencies**

```toml
# crates/madhyamas-core/Cargo.toml
[dependencies]
wasmtime = "26"       # or extism = "1"
```

**Step 2: Create WASM runtime** (`plugin/wasm_runtime.rs` — new file)

```rust
use wasmtime::{Engine, Instance, Linker, Module, Store, Memory};

pub struct WasmRuntime {
    engine: Engine,
    linker: Linker<HostState>,
    /// Cached compiled modules (plugin_id → Module)
    modules: RwLock<HashMap<String, Module>>,
}

pub struct HostState {
    /// Per-invocation state (request data, settings, etc.)
    request_data: Option<Vec<u8>>,    // JSON-serialized context
    response_data: Option<Vec<u8>>,
    settings: HashMap<String, serde_json::Value>,
    logs: Vec<String>,
}

impl WasmRuntime {
    pub fn execute_hook(
        &self,
        plugin: &Plugin,
        hook: PluginHook,
        context: &PluginContext,
    ) -> PluginResult {
        // 1. Get or compile the WASM module
        let module = self.get_or_compile(&plugin.manifest.id, &plugin.path)?;

        // 2. Create a store with fuel (CPU limit)
        let mut store = Store::new(&self.engine, HostState::from(context));
        store.set_fuel(10_000_000)?;  // ~10M instructions

        // 3. Instantiate with linked host functions
        let instance = self.linker.instantiate(&mut store, &module)?;

        // 4. Get the hook function
        let func = instance.get_typed_func::<u32, u32>(&mut store, hook.export_name())?;

        // 5. Write context JSON to guest memory
        let ctx_json = serde_json::to_vec(context)?;
        let ptr = self.write_to_guest(&mut store, &instance, &ctx_json)?;

        // 6. Call the function
        let result = func.call(&mut store, ptr)?;

        // 7. Read result from guest memory
        let result_json = self.read_from_guest(&mut store, &instance, result)?;
        let plugin_result: PluginResult = serde_json::from_slice(&result_json)?;

        // 8. Collect logs from host state
        plugin_result.logs = store.into_data().logs;

        plugin_result
    }
}
```

**Step 3: Wire into `PluginManager::execute_plugin_hook()`**

Replace the no-op in `manager.rs:355-396` with a call to
`WasmRuntime::execute_hook()`.

**Step 4: Apply results in `PluginExtension`**

In `extension.rs:400-446`, when a plugin returns `modified: true`, write
the changes back to `ExtensionContext`. When `handled: true`, short-circuit.

### 5.7 Plugin Hook Export Names

| PluginHook | WASM Export Name | Called When |
|---|---|---|
| `OnLoad` | `on_load` | Plugin first loaded |
| `OnEnable` | `on_enable` | Plugin enabled |
| `OnDisable` | `on_disable` | Plugin disabled |
| `OnUnload` | `on_unload` | Plugin unloaded |
| `OnRequest` | `on_request` | Before forwarding to upstream |
| `OnResponse` | `on_response` | After receiving response |
| `OnWebSocket` | `on_websocket` | On WebSocket message |
| `OnGrpc` | `on_grpc` | On gRPC message |
| `OnSettingsChange` | `on_settings_change` | When settings are updated |
| `OnTimer` | `on_timer` | Periodic (configurable interval) |

---

## 6. How to Keep It Secure

Plugin execution is even higher-risk than scripting — plugins are
distributed code from potentially untrusted sources. Security is paramount.

### 6.1 WASM Sandboxing (Primary Defense)

WASM is sandboxed by design:
- **No host access** — a WASM module cannot access the filesystem, network,
  or host memory unless the host explicitly provides it via the linker
- **No syscalls** — WASM has no syscall interface; all I/O goes through
  host-provided functions
- **Linear memory isolation** — each WASM instance has its own linear
  memory; it cannot read or write host memory directly

```rust
// Only link the functions you want to expose:
let mut linker = Linker::new(&engine);
linker.define("env", "log", Func::wrap(...))?;
linker.define("env", "get_request_header", Func::wrap(...))?;
// Do NOT link: fs_read, fs_write, network_connect, process_spawn
```

### 6.2 Resource Limits

| Resource | Mechanism | Implementation |
|---|---|---|
| **CPU time** | wasmtime fuel/gas metering | `store.set_fuel(10_000_000)` — when fuel runs out, execution traps |
| **Memory** | WASM linear memory limit | `Memory::new(store, Limits::new(1, Some(64)))` — max 64 pages (4MB) |
| **Execution time** | `tokio::time::timeout` | Wrap `func.call()` in a timeout; kill the instance if exceeded |
| **Network** | Only link `http_fetch` if the plugin's manifest declares `network: true` | Check manifest before linking |
| **Filesystem** | Never link FS functions | Plugins cannot read/write files |

### 6.3 Plugin Verification

| Check | When | Implementation |
|---|---|---|
| **Checksum verification** | On install/download | Compare SHA-256 of the downloaded zip against the registry's `checksum` field |
| **Signature verification** | On install (future) | Verify plugin is signed by a trusted publisher using Ed25519 |
| **Manifest validation** | On load | Validate semver, required fields, hook names are known |
| **Capability enforcement** | On load + execution | Only allow hooks declared in the manifest; reject calls to undeclared hooks |
| **Dependency audit** | On enable | Recursively check dependencies for version conflicts and cycles |

### 6.4 Trust Model

| Plugin Source | Trust Level | Restrictions |
|---|---|---|
| Built-in (ships with Madhyamas) | Fully trusted | No fuel limit, full host access |
| Local filesystem (`~/.madhyamas/plugins/`) | Trusted (local admin) | Fuel limit, no network unless declared |
| Remote registry (verified publisher) | Semi-trusted | Fuel limit, no network unless declared, signature verified |
| Remote registry (unverified) | Untrusted | Strict fuel limit, no network, no FS, user confirmation required |
| Custom registry URL | Depends on registry | Configurable per-registry trust level |

### 6.5 Capability-Based Security

The manifest declares which capabilities a plugin uses. The host enforces
these at link time:

```rust
// Only link network functions if the manifest declares network capability
if manifest.capabilities.contains(&PluginCapability::Network) {
    linker.define("env", "http_fetch", Func::wrap(...))?;
}
// Otherwise, http_fetch is not linked → calling it traps (security error)
```

### 6.6 Instance Isolation

- **Fresh instance per hook call** — no shared state between invocations
  (safest, but has overhead)
- **Pooled instances per plugin** — reuse instances across calls for the
  same plugin, reset state between calls (better performance, acceptable
  risk for trusted plugins)
- **Shared instance per plugin** — persistent state across calls (only for
  fully trusted built-in plugins; needed for `OnTimer` and stateful plugins)

### 6.7 Audit Logging

Log every plugin invocation with:
- Plugin ID, hook, timestamp
- Duration, fuel consumed, success/error
- Whether the request/response was modified
- Logs produced by the plugin

Stored in `PluginStats` (already defined in `types.rs:184-194`) and
exposed via `GET /api/plugins/{id}/stats`.

---

## 7. How to Enhance It

### 7.1 UI Panel Plugins

Plugins with `UiPanel` capability can render custom UI:
- **iframe approach**: Plugin serves an HTML page; Madhyamas embeds it in
  an iframe with postMessage communication
- **React component approach**: Plugin provides a WASM-compiled React
  component (via `react-wasm`); Madhyamas renders it
- **Declarative approach**: Plugin declares a UI schema (like settings
  schema); Madhyamas renders it with built-in components

**Recommended: Declarative approach for v1** (simplest, safest); iframe
approach for v2 (more flexible).

### 7.2 Plugin Development Toolkit

Create a `madhyamas-plugin-cli` tool for plugin authors:
```bash
madhyamas-plugin new my-plugin --lang rust
madhyamas-plugin build
madhyamas-plugin test --traffic-file capture.har
madhyamas-plugin package
madhyamas-plugin publish
```

This scaffolds a plugin project, compiles to WASM, tests against captured
traffic, packages as a zip, and publishes to the registry.

### 7.3 Plugin Signing

Implement Ed25519 signature verification:
- Plugin authors generate a keypair and sign their plugin package
- The registry stores the public key and signature
- On install, Madhyamas verifies the signature against a trusted publisher
  list
- Untrusted plugins require explicit user confirmation

### 7.4 Plugin Hot-Reload

When a plugin's WASM file changes on disk:
1. Watch the plugin directory with `notify` crate
2. Recompile the WASM module
3. Swap the cached `Module` in the `modules` map
4. New hook calls use the new module; existing calls finish with the old

### 7.5 Plugin Communication

Allow plugins to communicate with each other via:
- **Event bus**: Plugins publish/subscribe to named events
- **Shared state**: A per-session key-value store accessible to all plugins
- **Plugin-to-plugin calls**: One plugin can call another's exported
  function (with capability enforcement)

### 7.6 Remote Registry

Implement the remote registry fetch (`registry.rs:138-148` TODO):
- `GET https://registry.madhyamas.dev/plugins` — list plugins
- `GET https://registry.madhyamas.dev/plugins/{id}` — get plugin metadata
- `GET https://registry.madhyamas.dev/plugins/{id}/download` — download zip
- Pagination, search, filtering by capability/tags
- Client-side caching with TTL (already implemented in `PluginRegistry`)

### 7.7 Plugin Templates

Provide starter plugin templates (like script templates):
- "CORS Helper" plugin (Rust → WASM)
- "Request Logger" plugin
- "Custom Export Format" plugin
- "UI Panel" plugin

These give plugin authors a working starting point.

---

## 8. Backend Changes

### 8.1 New Files

| File | Purpose |
|---|---|
| `crates/madhyamas-core/src/plugin/wasm_runtime.rs` | WASM execution runtime (`WasmRuntime`, host functions, instance management) |
| `crates/madhyamas-core/src/plugin/persistence.rs` | SQLite-backed plugin state and settings storage |
| `crates/madhyamas-core/src/plugin/installer.rs` | Plugin download, checksum verification, extraction |
| `crates/madhyamas-plugin-sdk/` | **New crate** — guest SDK for plugin authors (Rust) |
| `crates/madhyamas-plugin-sdk/macros/` | Procedural macros (`#[plugin]`, `#[hook]`) |

### 8.2 Modified Files

| File | Change |
|---|---|
| `crates/madhyamas-core/Cargo.toml` | Add `wasmtime = "26"` (or `extism = "1"`) under the `plugins` feature; add `notify = "6"` for hot-reload |
| `crates/madhyamas-core/src/plugin/mod.rs` | Add `mod wasm_runtime;`, `mod persistence;`, `mod installer;`; export `WasmRuntime` |
| `crates/madhyamas-core/src/plugin/manager.rs` | Replace `execute_plugin_hook()` no-op (line 355) with `WasmRuntime::execute_hook()`; add `WasmRuntime` field to `PluginManager`; call lifecycle hooks (`on_load`, `on_enable`, etc.); add `install_plugin()`, `uninstall_plugin()` methods; add persistence (load/save enabled state + settings) |
| `crates/madhyamas-core/src/plugin/registry.rs` | Implement remote registry fetch (line 138-148 TODO) using `reqwest`; add `download_plugin()` method; add checksum verification |
| `crates/madhyamas-core/src/plugin/types.rs` | Add `capabilities: Vec<PluginCapability>` to `PluginManifest` (currently only in `RegistryEntry`); add `network: bool`, `max_memory_pages: u32`, `fuel_limit: u64` to manifest for resource limits |
| `crates/madhyamas-core/src/plugin/hooks.rs` | Add `export_name()` method to `PluginHook` mapping to WASM export names; add `OnTimer` interval field to manifest |
| `crates/madhyamas-core/src/extension.rs` | In `PluginExtension::on_request/on_response` (line 400-446), apply plugin modifications back to `ExtensionContext`; handle `handled`/short-circuit |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | In `run_request_hooks` (line 1226), when `ExtensionResult.handled` is true, short-circuit with custom response |
| `crates/madhyamas-api/src/phase3_handlers.rs` | Add `POST /api/plugins/install` (install from URL or registry), `DELETE /api/plugins/{id}` (uninstall), `GET /api/plugins/registry/search` (search remote registry), `GET /api/plugins/{id}/settings` + `PUT /api/plugins/{id}/settings` (settings CRUD), `GET /api/plugins/{id}/schema` (settings schema for UI generation) |
| `crates/madhyamas-api/src/routes.rs` | Register the new plugin endpoints |
| `crates/madhyamas-cli/src/commands/plugins.rs` | Add `madhyamas plugins install <id>`, `madhyamas plugins uninstall <id>`, `madhyamas plugins search <query>`, `madhyamas plugins settings get <id>`, `madhyamas plugins settings set <id> <key> <value>` |
| `crates/madhyamas-mcp/src/tools/plugins.rs` | Add `madhyamas_install_plugin`, `madhyamas_uninstall_plugin`, `madhyamas_search_plugins`, `madhyamas_get_plugin_settings`, `madhyamas_update_plugin_settings` |
| `crates/madhyamas-mcp/src/tools/registry.rs` | Register the new MCP tools |
| `crates/madhyamas/src/main.rs` | Load persisted plugin state on startup; call `on_load` lifecycle hook for each enabled plugin; call `on_unload` on shutdown |

### 8.3 Database Schema

```sql
CREATE TABLE IF NOT EXISTS plugin_state (
    plugin_id TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 0,
    settings TEXT,                    -- JSON
    installed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_invocations (
    id TEXT PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    hook TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    fuel_consumed INTEGER,
    success INTEGER NOT NULL,
    error TEXT,
    logs TEXT,                        -- JSON array
    modified INTEGER NOT NULL DEFAULT 0,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (plugin_id) REFERENCES plugin_state(plugin_id) ON DELETE CASCADE
);
```

### 8.4 Config Changes

Add to `ProxyConfig`:
```rust
pub struct PluginsConfig {
    pub enabled: bool,
    pub plugin_dirs: Vec<String>,
    pub registry_url: Option<String>,       // remote registry URL
    pub default_fuel_limit: u64,            // default WASM fuel
    pub default_memory_pages: u32,          // default max memory pages
    pub allow_unverified_plugins: bool,     // allow unsigned plugins
    pub auto_update: bool,                  // auto-update plugins from registry
}
```

Expose via `GET/PATCH /api/config` and CLI flags
(`--plugin-dir`, `--registry-url`, `--plugin-fuel-limit`).

### 8.5 Workspace Changes

Add the plugin SDK crate to the workspace:
```toml
# Cargo.toml (workspace)
[workspace.members]
members = [
    "crates/madhyamas",
    "crates/madhyamas-core",
    "crates/madhyamas-api",
    "crates/madhyamas-cli",
    "crates/madhyamas-mcp",
    "crates/madhyamas-plugin-sdk",    # NEW
]
```

---

## 9. Frontend Changes

### 9.1 PluginsPanel Enhancements (`web/src/features/tools/PluginsPanel.tsx`)

| Enhancement | Description |
|---|---|
| **Settings panel** | When a plugin is expanded and has a `settings` schema, render a form with the appropriate input types (text, number, boolean, select, color picker, etc.). Save via `PUT /api/plugins/{id}/settings`. |
| **Registry browser** | A new tab "Registry" that searches the remote registry, shows plugin cards with description, rating, download count, and an "Install" button. |
| **Install/Uninstall** | Install button in the registry browser; uninstall button in the plugin detail view (with confirmation dialog). |
| **Error detail** | When a plugin is in `error` state, show the full error message with a "Retry" button. |
| **Plugin logs** | A "Logs" tab in the expanded view showing recent invocation logs (hook, duration, success/error, plugin log output). |
| **Capability badges** | Show capability badges (Intercept, UI Panel, Export, etc.) in the plugin detail view. |
| **Plugin README** | If the plugin includes a README, render it (markdown) in the expanded view. |
| **Fuel/memory indicator** | Show fuel consumed and memory used in the stats section. |

### 9.2 New Components

| Component | File | Purpose |
|---|---|---|
| `PluginSettingsForm` | `web/src/features/tools/PluginSettingsForm.tsx` | Dynamic form generated from `PluginSettingsSchema` |
| `PluginRegistryBrowser` | `web/src/features/tools/PluginRegistryBrowser.tsx` | Search and install plugins from the remote registry |
| `PluginInstallDialog` | `web/src/features/tools/PluginInstallDialog.tsx` | Confirmation dialog for installing a plugin (shows checksum, publisher, capabilities) |
| `PluginLogs` | `web/src/features/tools/PluginLogs.tsx` | Invocation log viewer for a plugin |

### 9.3 API Hooks (`web/src/lib/api/phase3.ts`)

Add:
```typescript
export function usePluginSettings(id: string) { ... }       // GET /plugins/{id}/settings
export function useUpdatePluginSettings() { ... }            // PUT /plugins/{id}/settings
export function usePluginSchema(id: string) { ... }          // GET /plugins/{id}/schema
export function useRegistrySearch(query: string) { ... }     // GET /plugins/registry/search
export function useInstallPlugin() { ... }                   // POST /plugins/install
export function useUninstallPlugin() { ... }                 // DELETE /plugins/{id}
```

### 9.4 Types

Update `Plugin` and add new types:
```typescript
export interface Plugin {
  manifest: PluginManifest;
  state: 'loaded' | 'enabled' | 'running' | 'disabled' | 'error' | 'unloading';
  settings: Record<string, unknown>;
  path: string;
  loaded_at: string;
  error?: string;
}

export interface PluginSettingsSchema {
  fields: PluginSettingField[];
}

export interface PluginSettingField {
  key: string;
  label: string;
  description?: string;
  field_type: 'string' | 'number' | 'boolean' | 'select' | 'multi_select' | 'color' | 'url' | 'path' | 'json';
  default?: unknown;
  required: boolean;
  options?: string[];
}

export interface RegistryEntry {
  manifest: PluginManifest;
  download_url: string;
  checksum: string;
  downloads: number;
  rating: number;
  rating_count: number;
  capabilities: string[];
  tags: string[];
  added_at: string;
  updated_at: string;
}
```

---

## 10. Documentation Changes

### 10.1 New Documents

| Document | Content |
|---|---|
| `docs/PLUGINS.md` | End-user guide: how to install, enable, configure, and uninstall plugins; the plugin panel; the registry browser; troubleshooting |
| `docs/PLUGIN_DEVELOPMENT.md` | Developer guide: plugin structure, manifest format, hooks, the guest SDK, building to WASM, testing, packaging, publishing to the registry |
| `docs/PLUGIN_API.md` | Complete API reference for plugin authors — every host function, hook, data type, with signatures and examples |
| `docs/PLUGIN_SECURITY.md` | Security model: WASM sandboxing, resource limits, capability enforcement, trust levels, signing, what plugins can and cannot do |

### 10.2 Updated Documents

| Document | Change |
|---|---|
| `CLAUDE.md` | Update the plugin section: note that plugins are now functional; add `PluginsConfig` to the config section; add new API endpoints; add new CLI commands; add plugin SDK crate to the project structure |
| `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` | Change Plugin system row from 🟡 to ✅ |
| `docs/ARCHITECTURE.md` | Add the WASM runtime to the architecture diagram; describe the plugin execution flow |
| `docs/PROXY_FLOW.md` | Add plugin hooks to the request/response flow diagram |
| `.claude/skills/madhyamas/SKILL.md` | Add plugin workflow: how to install, configure, and develop plugins |

### 10.3 Plugin SDK Documentation

The `madhyamas-plugin-sdk` crate should include:
- `README.md` — quick start, "hello world" plugin
- `docs/` — detailed guides for each hook type, the host API, testing
- Examples directory with 3-5 sample plugins (CORS, logger, mock, custom
  export, UI panel)

### 10.4 In-UI Documentation

- Add a "Develop Plugins" link in the PluginsPanel that opens the plugin
  development guide
- Show the manifest format and available hooks in a help modal
- Display the plugin's README (if included) in the expanded view

---

## 11. Implementation Phases

### Phase 1: Make Plugins Run (Critical)

**Goal:** WASM plugins execute and can modify traffic.

| Task | Effort |
|---|---|
| Add `wasmtime` dependency | Small |
| Create `plugin/wasm_runtime.rs` with `WasmRuntime` | Hard |
| Define and implement host ABI (linker functions) | Hard |
| Create `madhyamas-plugin-sdk` crate with basic API | Medium |
| Wire `WasmRuntime` into `PluginManager::execute_plugin_hook()` | Medium |
| Apply plugin results back to `ExtensionContext` | Small |
| Handle short-circuit (custom response) in pipeline | Small |
| Implement lifecycle hooks (on_load, on_enable, etc.) | Small |
| Add SQLite persistence for plugin state | Medium |
| Write 2-3 example plugins (CORS, logger) | Medium |
| Test with example plugins | Small |

**Estimated effort:** 1-2 weeks. **Impact:** High — makes the plugin
system functional and enables third-party extensibility.

### Phase 2: Distribution and Management (Important)

**Goal:** Users can discover, install, and configure plugins.

| Task | Effort |
|---|---|
| Implement remote registry fetch | Medium |
| Plugin install/uninstall with checksum verification | Medium |
| Settings UI generation from schema | Medium |
| Plugin settings persistence | Small |
| Registry browser in web UI | Medium |
| Install/uninstall in web UI | Small |
| CLI install/uninstall/search commands | Small |
| MCP install/search tools | Small |
| Plugin error handling in UI | Small |

**Estimated effort:** 1 week. **Impact:** Medium — makes plugins
discoverable and configurable.

### Phase 3: Enhancement (Nice-to-Have)

**Goal:** Advanced features for a thriving plugin ecosystem.

| Task | Effort |
|---|---|
| UI panel rendering (declarative approach) | Hard |
| Plugin signing (Ed25519) | Medium |
| Plugin development CLI (`madhyamas-plugin`) | Medium |
| Hot-reload via `notify` crate | Medium |
| Timer hook scheduling | Small |
| Plugin communication (event bus) | Medium |
| Plugin templates / scaffolding | Small |
| Multi-language SDKs (Go, AssemblyScript) | Hard |

**Estimated effort:** 2-4 weeks. **Impact:** Medium — builds a plugin
ecosystem; not needed for initial adoption.

---

## Scripting vs. Plugins: When to Use Which

| Aspect | Scripting | Plugins |
|---|---|---|
| **Format** | Single JS/TS file | Packaged directory (manifest + WASM) |
| **Language** | JavaScript/TypeScript | Any WASM-targeting language |
| **Distribution** | Copy-paste, JSON import | Registry download, zip install |
| **Versioning** | No (source only) | Semver, dependencies |
| **Sandboxing** | `boa_engine` (no FS/net by default) | WASM (strongest sandbox) |
| **Performance** | Interpreted (slower) | Compiled WASM (faster) |
| **Use case** | Quick, personal, ad-hoc manipulation | Reusable, shared, structured extensions |
| **Settings** | None (hardcoded in source) | Schema-driven, UI-generated |
| **UI panels** | No | Yes (UiPanel capability) |
| **Target user** | Individual developer | Teams, community, enterprise |

**Recommendation:** Keep both systems. Scripts are the "quick hack" path
for individual users; plugins are the "proper extension" path for reusable,
shareable, production-quality extensions. The unified `ExtensionManager`
already dispatches to both in priority order (scripts at 10, plugins at
20), so they coexist cleanly.

---

*Generated 2026-08-01. Based on codebase analysis as of this date.*
