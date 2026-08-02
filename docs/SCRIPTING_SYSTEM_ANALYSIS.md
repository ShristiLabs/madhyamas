# Scripting System Analysis

This document is a thorough analysis of the Madhyamas scripting system: what
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

The scripting system is a **scaffold with no execution engine**. The entire
infrastructure for storing, managing, and dispatching scripts exists —
metadata, hooks, contexts, API endpoints, CLI commands, MCP tools, and a web
UI code editor — but the `execute()` method returns a hardcoded
`"No JS runtime integrated"` error. No JavaScript engine is wired in, so
scripts never run.

The system is designed for **JavaScript/TypeScript** scripts that hook into
the proxy pipeline at defined points (`on_request`, `on_response`, etc.) to
inspect and modify HTTP traffic. The hook model, context structures, and
result types are well-designed and mirror what a real JS engine would need.

**Current status:** 🟡 Partial — infrastructure complete, execution missing.
**Recommended engine:** `boa_engine` (pure Rust, no native deps, sandboxed
by design) or `rquickjs` (QuickJS bindings, faster, small footprint).
**Estimated effort to make functional:** Medium (engine integration +
sandboxing + API binding).

---

## 2. What Exists Now

### 2.1 Core Module (`crates/madhyamas-core/src/scripting/`)

| Component | File | Lines | State |
|---|---|---|---|
| Module root | `mod.rs` | 12 | Exports `ScriptApi`, `ScriptContext`, `ScriptHook`, `ScriptResult`, `Script`, `ScriptConfig`, `ScriptExecution`, `ScriptRuntime`, `ScriptTemplates` |
| Runtime | `runtime.rs` | 473 | `ScriptRuntime` struct — in-memory `scripts: RwLock<HashMap<String, Script>>` + `history: RwLock<Vec<ScriptExecution>>`; CRUD methods (register, remove, get, toggle, update); `validate()` does brace/paren balance check only; **`execute()` returns error "No JS runtime integrated"**; `load_from_directory()` reads `.js`/`.ts` files; `export_scripts()`/`import_scripts()` JSON round-trip |
| Hooks | `hooks.rs` | 238 | `ScriptHook` enum (7 hooks: OnRequest, OnResponse, OnWebSocketMessage, OnGrpcMessage, OnTrafficStore, OnSessionStart, OnSessionEnd); `ScriptContext` with request/response/websocket/grpc/data fields; `RequestContext`/`ResponseContext` with `From<&RequestData>`/`From<&ResponseData>` conversions; `ScriptResult { modified, continue_, response, error, console, duration_ms }`; `ScriptResponse` with `to_response_data()` |
| Script API | `api.rs` | 256 | `ScriptApi` with `documentation()` (markdown API guide) and `builtin_functions()` (console.log, JSON.parse/stringify, base64.encode/decode, crypto.hash, url.parse/build); `URLComponents` with parse/build; **none of these functions are actually exposed to a running script** (no engine to register them with) |

### 2.2 Script Metadata (`runtime.rs:9-47`)

```rust
pub struct Script {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub source: String,          // JS/TS source code
    pub hooks: Vec<String>,      // which hooks this script subscribes to
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub priority: u32,           // lower = runs earlier (default 100)
}
```

### 2.3 Script Config (`runtime.rs:72-96`)

```rust
pub struct ScriptConfig {
    pub timeout_ms: u64,         // 5000 (NOT enforced)
    pub max_memory_bytes: usize, // 10MB (NOT enforced)
    pub enable_console: bool,    // true
    pub allow_network: bool,     // false (NOT enforced)
    pub allow_fs: bool,          // false (NOT enforced)
}
```

The doc comment explicitly states: "The `timeout_ms`, `max_memory_bytes`,
`allow_network`, and `allow_fs` limits are defined here for future use but
are NOT yet enforced."

### 2.4 Execution — The Gap (`runtime.rs:229-264`)

```rust
pub fn execute(&self, script_id: &str, _context: &super::ScriptContext) -> super::ScriptResult {
    let _script = match self.get_script(script_id) { ... };
    // No JS runtime is integrated — return an error result without
    // recording a fake execution in history.
    super::ScriptResult {
        modified: false,
        continue_: true,
        response: None,
        error: Some("No JS runtime integrated".to_string()),
        console: Vec::new(),
        duration_ms: 0,
    }
}
```

`execute_hook()` (line 267) calls `execute()` for each matching script, so
it also produces only error results. The pipeline calls `execute_hook()`
via the extension manager — so today, every enabled script logs an error on
every request but has no effect.

### 2.5 Templates (`runtime.rs:367-473`)

Five templates are defined as `Script::new()` constructors with embedded JS
source:

| Template | Hook | Purpose |
|---|---|---|
| `log_requests` | `onRequest` | Console-log method, URL, headers |
| `add_cors` | `onResponse` | Add `Access-Control-Allow-*` headers |
| `block_domains` | `onRequest` | Return 403 for blocked domains |
| `modify_headers` | `onRequest` | Add `X-Madhyamas` and `X-Request-ID` headers |
| `mock_api` | `onRequest` | Return mock JSON for `/api/user/` paths |

These are valid JS using ES6+ (const, template literals, arrow functions).
They serve as both examples and test cases for the future engine.

### 2.6 Persistence

`ScriptRuntime` has **no `Persistable` impl** and no SQLite backing. Scripts
are stored only in memory (`RwLock<HashMap>`). On restart, all scripts are
lost unless `load_from_directory()` is called (which is not wired up in
`main.rs`). The `export_scripts()`/`import_scripts()` methods exist but are
not exposed via the API.

### 2.7 Extension Manager Integration (`extension.rs:240-369`)

`ScriptExtension` adapts `ScriptRuntime` to the unified `Extension` trait:
- Priority: 10 (runs before plugins at 20)
- `on_request()` / `on_response()` build a `ScriptContext` from the
  `ExtensionContext`, call `runtime.execute_hook()`, and aggregate results
- The adapter is registered in `main.rs:643-650` and the pipeline calls
  `extension_manager.on_request()` / `on_response()` at
  `pipeline.rs:303-305` and `pipeline.rs:429-431`

### 2.8 API Layer (`crates/madhyamas-api/src/phase3_handlers.rs:74-206`)

| Endpoint | Method | Handler | State |
|---|---|---|---|
| `/api/scripts` | GET | `get_scripts` | ✅ Returns all scripts |
| `/api/scripts` | POST | `create_script` | ✅ Creates script (with validation) |
| `/api/scripts/templates` | GET | `get_script_templates` | ✅ Returns 5 templates |
| `/api/scripts/config` | GET | `get_script_config` | ✅ Returns `ScriptConfig::default()` |
| `/api/scripts/{id}` | GET | `get_script` | ✅ Returns single script |
| `/api/scripts/{id}` | PUT | `update_script` | ✅ Updates source |
| `/api/scripts/{id}` | DELETE | `delete_script` | ✅ Removes script |
| `/api/scripts/{id}/toggle` | POST | `toggle_script` | ✅ Enables/disables |

All endpoints are feature-gated behind `#[cfg(feature = "scripting")]` and
registered in `routes.rs:352-364`. Input validation uses `validator` crate
(length constraints on name and source).

### 2.9 CLI (`crates/madhyamas-cli/src/commands/scripts.rs`)

| Command | State |
|---|---|
| `madhyamas scripts list` | ✅ |
| `madhyamas scripts create --name N --file F \| --inline S --hook H` | ✅ |
| `madhyamas scripts get <id>` | ✅ |
| `madhyamas scripts delete <id>` | ✅ |
| `madhyamas scripts toggle <id>` | ✅ (sends `{}` instead of `{enabled: bool}` — minor bug) |
| `madhyamas scripts templates` | ✅ |

### 2.10 MCP Tools (`crates/madhyamas-mcp/src/tools/scripts.rs`)

| Tool | State |
|---|---|
| `madhyamas_list_scripts` | ✅ |
| `madhyamas_create_script` | ✅ |
| `madhyamas_get_script` | ✅ |
| `madhyamas_update_script` | ✅ |
| `madhyamas_delete_script` | ✅ |
| `madhyamas_toggle_script` | ✅ |
| `madhyamas_get_script_templates` | ✅ |

Registered in `registry.rs:1079-1120`. All make HTTP calls to the REST API.

### 2.11 Web UI (`web/src/features/tools/ScriptsPanel.tsx`)

A full panel with:
- **Scripts tab**: list of scripts with enable/disable switch, edit (code
  editor), delete buttons; search filter
- **Templates tab**: list of 5 templates with "Use" button to create
- **Code editor**: `react-simple-code-editor` with Prism.js syntax
  highlighting (JavaScript, tomorrow theme)
- **API hooks**: `useScripts`, `useScriptTemplates`, `useCreateScript`,
  `useUpdateScript`, `useDeleteScript`, `useToggleScript` in
  `web/src/lib/api/phase3.ts:180-244`

The UI is functional for CRUD but has no execution feedback — no console
output view, no execution history, no error display from script runs.

### 2.12 Feature Gate

`crates/madhyamas-core/Cargo.toml:11` — `default = ["grpc", "scripting",
"plugins", "enterprise"]`. The `scripting` feature is a bare flag (no
optional deps); it gates the module compilation but no JS engine is
depended upon.

---

## 3. Use Cases

### 3.1 Custom Traffic Manipulation

Users need to modify requests/responses in ways that the built-in rewrite
rules can't express — conditional logic, multi-step transformations,
stateful modifications across request/response pairs.

**Example:** "If the request is a POST to `/api/login` and the body
contains `test_user`, replace the response with a mock token. For all other
requests, add a trace header."

### 3.2 Dynamic Mocking

Unlike static mocks (which match a fixed URL pattern), scripts can generate
dynamic mock responses based on request body content, headers, or external
state.

**Example:** "For `/api/users/{id}`, parse the ID from the URL, look up the
user in a local JSON file, and return a realistic response. Return 404 if
not found."

### 3.3 Request/Response Logging and Metrics

Scripts can log structured data, compute custom metrics, or send traffic
data to external systems (when network access is allowed).

**Example:** "Log every API call with its duration to a structured JSON
log file. Alert if any request takes more than 5 seconds."

### 3.4 Security Testing

Scripts can inject vulnerabilities, strip security headers, or test for
specific attack patterns during security audits.

**Example:** "For all responses from `staging.example.com`, remove the
`Content-Security-Policy` header and add an `X-Test-Insecure` header to
verify the app degrades safely."

### 3.5 Protocol-Specific Handling

Scripts can handle protocols or content types that the built-in viewers
don't support — custom binary formats, proprietary encodings, or
industry-specific data formats.

**Example:** "For responses with `Content-Type: application/vnd.custom+json`,
decompress the body with a custom algorithm before storing."

### 3.6 Automation and CI/CD Integration

Scripts can automate multi-step debugging workflows — set up mocks, trigger
requests, verify responses, and tear down — all scriptable for CI.

**Example:** "On session start, load a set of mocks from a config file. On
each request, verify it matches expected patterns. On session end, export
a report."

### 3.7 AI Agent Orchestration

Via MCP, AI agents can create and manage scripts programmatically —
writing custom interception logic on the fly based on what they observe in
the traffic.

**Example:** An AI agent observes failing API calls, writes a script to
mock the failing endpoint with a known-good response, and re-runs the test.

---

## 4. What Needs to Be Built

### 4.1 Critical (Make Scripts Actually Run)

1. **Integrate a JavaScript engine** — parse and execute JS source code
2. **Bind the script API** — expose `request`, `response`, `context`,
   `console`, `JSON`, `base64`, `crypto`, `url` to the engine
3. **Implement hook dispatch** — call the right JS function (`onRequest`,
   `onResponse`) based on the hook type
4. **Apply script results** — when a script returns `{ modified: true }`,
   write the modifications back to the `RequestData`/`ResponseData`; when
   it returns `{ continue: false, response: {...} }`, short-circuit the
   pipeline with the custom response
5. **Enforce resource limits** — timeout, memory, no network, no filesystem
6. **Add persistence** — store scripts in SQLite so they survive restarts

### 4.2 Important (Usability)

7. **Execution history** — record real executions with duration, success,
   errors, console output
8. **Console output streaming** — show script console.log output in the web
   UI in real time
9. **Error reporting** — surface JS parse errors and runtime exceptions with
   line/column numbers
10. **Script testing/dry-run** — let users test a script against a captured
    request without affecting live traffic
11. **Hot-reload** — when a script's source is updated, re-compile it
    without restarting the proxy

### 4.3 Nice-to-Have (Enhancement)

12. **TypeScript support** — transpile TS to JS before execution
13. **Module system** — allow scripts to `import` shared modules
14. **Script marketplace** — share and download community scripts (via the
    plugin registry infrastructure)
15. **Debugging** — breakpoints, step-through, variable inspection

---

## 5. How to Implement It

### 5.1 Engine Selection

| Engine | Pros | Cons | Recommendation |
|---|---|---|---|
| **`boa_engine`** | Pure Rust, no C deps, sandboxed by design, good ES2020+ support, actively developed | Slower than V8/QuickJS (~10x), no JIT | **Recommended for v1** — safest, simplest integration, no platform-specific builds |
| **`rquickjs`** (QuickJS) | Fast, small footprint, complete ES2020+ support, mature | C dependency (QuickJS), platform-specific builds | Good alternative if performance matters |
| **`deno_core`** (V8) | Fastest, most complete JS support, used by Deno | Very heavy dependency (V8), large binary size, complex build | Overkill for a debugging proxy |
| **`rune`** (Rune language) | Pure Rust, designed for embedding | Not JavaScript — users would need to learn Rune | Only if JS is not a requirement |

**Recommended: `boa_engine`** for v1. It's pure Rust (matches the project's
no-native-deps philosophy), sandboxed by design (no `fs`/`net` access
unless explicitly provided), and has good enough performance for
per-request script execution (scripts are small and fast). Switch to
`rquickjs` later if performance becomes a bottleneck.

### 5.2 Engine Integration Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Proxy Pipeline                     │
│  (pipeline.rs: run_request_hooks / run_response_hooks)│
└──────────────────┬──────────────────────────────────┘
                   │ ExtensionContext
                   ▼
┌─────────────────────────────────────────────────────┐
│              ExtensionManager (extension.rs)         │
│  dispatches to ScriptExtension (priority 10)         │
└──────────────────┬──────────────────────────────────┘
                   │ ScriptContext
                   ▼
┌─────────────────────────────────────────────────────┐
│              ScriptRuntime (runtime.rs)              │
│  execute_hook() → for each matching script:          │
│    1. Get/create compiled JsValue (cached)           │
│    2. Build JS context objects (request, response)   │
│    3. Call the hook function (onRequest/onResponse)  │
│    4. Parse return value → ScriptResult              │
│    5. Apply modifications to RequestData/ResponseData│
│    6. Record execution in history                    │
└──────────────────┬──────────────────────────────────┘
                   │ boa_engine
                   ▼
┌─────────────────────────────────────────────────────┐
│              boa_engine::Context                      │
│  - Registered globals: console, JSON, base64, etc.  │
│  - Compiled script cache (by script ID + source hash)│
│  - Resource limits (timeout via async, memory limit) │
└─────────────────────────────────────────────────────┘
```

### 5.3 Implementation Details

**Step 1: Add `boa_engine` dependency**

```toml
# crates/madhyamas-core/Cargo.toml
[dependencies]
boa_engine = "0.20"
```

**Step 2: Create a JS execution layer** (`scripting/engine.rs` — new file)

```rust
use boa_engine::{Context, JsValue, JsResult, Source, object::ObjectInitializer};
use std::time::{Duration, Instant};

pub struct JsEngine {
    /// Per-script compiled code cache (script_id → compiled AST)
    cache: RwLock<HashMap<String, CachedScript>>,
    /// Global API setup (console, JSON, etc.) — recreated per execution
    /// for isolation, or shared if scripts are trusted
}

struct CachedScript {
    bytecode: Vec<u8>,  // boa serialized bytecode
    source_hash: u64,   // recompile if source changes
}

impl JsEngine {
    pub fn execute(
        &self,
        source: &str,
        hook_fn: &str,       // "onRequest" or "onResponse"
        context: &ScriptContext,
        config: &ScriptConfig,
    ) -> ScriptResult {
        let mut ctx = Context::default();

        // 1. Register globals (console, JSON, base64, crypto, url)
        self.register_globals(&mut ctx, config);

        // 2. Parse and evaluate the script source (defines functions)
        let result = ctx.eval(Source::from_bytes(source));
        if let Err(e) = result {
            return ScriptResult {
                error: Some(format!("Parse error: {}", e)),
                ..Default::default()
            };
        }

        // 3. Get the hook function (e.g., global.onRequest)
        let hook = ctx.globals().get(hook_fn.as_bytes().into())?;
        if !hook.is_function() {
            return ScriptResult {
                error: Some(format!("Function '{}' not defined", hook_fn)),
                ..Default::default()
            };
        }

        // 4. Build JS request/response objects from ScriptContext
        let js_request = self.build_js_request(&context, &mut ctx);
        let js_context = self.build_js_context(&context, &mut ctx);

        // 5. Call the hook function with timeout
        let start = Instant::now();
        let call_result = hook.call(&JsValue::undefined, &[js_request, js_context]);
        let duration_ms = start.elapsed().as_millis() as u64;

        // 6. Parse the return value → ScriptResult
        self.parse_result(call_result, duration_ms, &mut ctx)
    }
}
```

**Step 3: Register the script API globals**

```rust
fn register_globals(&self, ctx: &mut Context, config: &ScriptConfig) {
    // console.log
    let console = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn(|_, args, ctx| {
                let msg = args.get(0).map(|v| v.to_string(ctx)).unwrap_or_default();
                // Store in a per-execution console buffer
                ctx.insert_log(msg.to_std_string_escaped());
                Ok(JsValue::undefined())
            }),
            "log", 0,
        )
        .build();
    ctx.globals().insert("console", console);

    // JSON.parse / JSON.stringify are built into boa
    // base64, crypto, url — register as native functions
    // Only register if config allows (e.g., skip fetch if !allow_network)
}
```

**Step 4: Wire into `ScriptRuntime::execute()`**

Replace the placeholder in `runtime.rs:238-264` with a call to `JsEngine::execute()`.

**Step 5: Apply results back to the pipeline**

In the `ScriptExtension` adapter (`extension.rs:272-296`), when a script
returns `modified: true`, write the changes back to the `ExtensionContext`'s
request/response fields. When a script returns `continue_: false` with a
`response`, set `ExtensionResult.handled = true` so the pipeline
short-circuits.

### 5.4 Hook Function Mapping

| ScriptHook | JS Function Name | Arguments | Called When |
|---|---|---|---|
| `OnRequest` | `onRequest` | `(request, context)` | Before forwarding to upstream |
| `OnResponse` | `onResponse` | `(request, response, context)` | After receiving response |
| `OnWebSocketMessage` | `onWebSocketMessage` | `(message, context)` | On WS frame |
| `OnGrpcMessage` | `onGrpcMessage` | `(message, context)` | On gRPC frame |
| `OnTrafficStore` | `onTrafficStore` | `(entry, context)` | Before storing traffic |
| `OnSessionStart` | `onSessionStart` | `(session, context)` | On new session |
| `OnSessionEnd` | `onSessionEnd` | `(session, context)` | On session close |

### 5.5 Result Contract

Scripts return a JS object:
```javascript
{
    continue: true|false,   // false = stop pipeline, return response
    modified: true|false,   // true = request/response was changed
    response: {             // only when continue is false
        statusCode: 200,
        headers: { "Content-Type": "application/json" },
        body: "..."
    }
}
```

The Rust side parses this into `ScriptResult` and the extension adapter
applies it to the `ExtensionContext`.

---

## 6. How to Keep It Secure

Script execution is the highest-risk feature in the proxy — untrusted JS
code running inside the process. Security must be designed in from the
start.

### 6.1 Sandboxing (Defense in Depth)

| Layer | Mechanism | Implementation |
|---|---|---|
| **No filesystem** | `boa_engine` has no FS access by default; don't register any FS functions | Don't call `ctx.register_global_callable("readFile", ...)` |
| **No network** | `boa_engine` has no network access by default; don't register `fetch`/`XMLHttpRequest` | Only register `fetch` if `config.allow_network` is true, and even then restrict to allowlisted domains |
| **No process access** | No `process`, `require`, `import`, `child_process` globals | Don't register them |
| **Memory limit** | `boa_engine` doesn't have a built-in memory limit; use a watchdog task that kills execution after `max_memory_bytes` | Check `ctx.memory_usage()` periodically (boa exposes allocator stats) |
| **Time limit** | Run execution in a `tokio::task::spawn_blocking` with a timeout | `tokio::time::timeout(Duration::from_millis(config.timeout_ms), handle)` |
| **Isolation** | Create a fresh `boa_engine::Context` per execution (no shared state between scripts) | Don't cache `Context` — cache only compiled bytecode |

### 6.2 Resource Limits Enforcement

```rust
pub fn execute_with_limits(
    source: &str,
    hook_fn: &str,
    context: &ScriptContext,
    config: &ScriptConfig,
) -> ScriptResult {
    // Run in a blocking thread with a timeout
    let timeout = Duration::from_millis(config.timeout_ms);
    let handle = tokio::task::spawn_blocking(move || {
        let mut ctx = Context::default();
        ctx.set_max_instructions(config.max_instructions);  // boa instruction limit
        // ... register globals, execute ...
    });

    match tokio::time::timeout(timeout, handle).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => ScriptResult { error: Some("Script panicked".into()), .. },
        Err(_) => ScriptResult { error: Some(format!("Script timed out after {}ms", config.timeout_ms)), .. },
    }
}
```

### 6.3 Script Validation

- **Source size limit**: Reject scripts larger than 100KB (prevents
  resource exhaustion from huge source)
- **Syntax validation**: Use `boa_engine` to parse (not execute) the script
  on create/update — reject if parse fails
- **Hook function check**: Verify the script defines at least one known
  hook function (`onRequest`, `onResponse`, etc.)
- **No `eval` / `Function` constructor**: Disable `eval` and `Function`
  constructor in the engine to prevent dynamic code generation

### 6.4 Trust Model

| Script Source | Trust Level | Restrictions |
|---|---|---|
| User-created via web UI | Trusted (local user) | Default limits (5s timeout, 10MB, no network) |
| Loaded from directory | Trusted (local filesystem) | Default limits |
| Imported from JSON | Semi-trusted | Default limits + syntax validation |
| Downloaded from marketplace | Untrusted | Stricter limits (1s timeout, 1MB, sandboxed) + user confirmation |

### 6.5 Audit Logging

Log every script execution with:
- Script ID, name, hook
- Duration, success/error
- Whether the request/response was modified
- Console output

This is stored in `ScriptExecution` records (already defined in
`runtime.rs:50-64`) and exposed via the API.

---

## 7. How to Enhance It

### 7.1 TypeScript Support

Use `swc` or `tsc` (via `dprint`/`deno_ast`) to transpile TypeScript to
JavaScript before passing to the engine. The `load_from_directory()`
method already reads `.ts` files; add a transpilation step.

```rust
fn transpile_ts(source: &str) -> Result<String, String> {
    // Use swc_core to parse TS and emit JS
    // Cache the transpiled output by source hash
}
```

### 7.2 Script Module System

Allow scripts to import shared modules:
```javascript
import { corsHeaders } from 'madhyamas:utils';
```

Implement a custom module resolver that loads modules from:
- `~/.madhyamas/scripts/lib/` (user libraries)
- Built-in modules (`madhyamas:utils`, `madhyamas:crypto`, `madhyamas:http`)

### 7.3 Script Marketplace

Leverage the existing `PluginRegistry` infrastructure to serve scripts:
- Upload/download scripts via the registry
- Versioning, checksums, ratings (already in `RegistryEntry`)
- Search by tags (already supported)

### 7.4 Debugging Support

- **Source maps**: When transpiling TS, generate source maps for error
  reporting
- **Step debugger**: Use `boa_engine`'s debugger hooks to implement
  breakpoints and step-through in the web UI
- **Variable inspection**: Expose the engine's scope chain to the web UI

### 7.5 Performance Optimizations

- **Bytecode caching**: Compile scripts once, cache bytecode by source hash,
  reuse across executions (boa supports bytecode serialization)
- **Context pooling**: Instead of creating a fresh `Context` per execution,
  pool and reset them (reduce allocation overhead)
- **JIT compilation**: If `boa_engine` is too slow, switch to `rquickjs`
  which has a JIT compiler

### 7.6 Advanced Hooks

- `on_connect`: Called when a new TCP connection is accepted (before TLS)
- `on_tls_handshake`: Called during TLS negotiation
- `on_websocket_connect`: Called on WS upgrade
- `on_error`: Called when a request fails (timeout, connection reset)
- `on_match`: Called when a URL matches a pattern (for custom filtering)

### 7.7 Script-to-Script Communication

Allow scripts to share state via a per-request key-value store (already
partially supported via `ScriptContext.data`). Extend to support:
- Cross-request state (with TTL)
- Pub/sub between scripts
- Shared variables scoped to a session

---

## 8. Backend Changes

### 8.1 New Files

| File | Purpose |
|---|---|
| `crates/madhyamas-core/src/scripting/engine.rs` | JS engine integration (`JsEngine` struct, `execute()`, global registration, result parsing) |
| `crates/madhyamas-core/src/scripting/persistence.rs` | SQLite-backed script storage (replaces in-memory HashMap) |

### 8.2 Modified Files

| File | Change |
|---|---|
| `crates/madhyamas-core/Cargo.toml` | Add `boa_engine = "0.20"` dependency under the `scripting` feature |
| `crates/madhyamas-core/src/scripting/mod.rs` | Add `mod engine;` and `mod persistence;`; export `JsEngine` |
| `crates/madhyamas-core/src/scripting/runtime.rs` | Replace `execute()` placeholder (line 238) with `JsEngine::execute()` call; add `Persistable` impl for SQLite storage; add bytecode cache; enforce resource limits; record real `ScriptExecution` in history |
| `crates/madhyamas-core/src/scripting/hooks.rs` | Add `From<ScriptResult>` for `ExtensionResult` to apply modifications back; add `on_connect`, `on_error` hooks |
| `crates/madhyamas-core/src/scripting/api.rs` | Implement actual `base64.encode/decode`, `crypto.hash`, `url.parse/build` as `boa_engine` native functions (currently just documentation strings) |
| `crates/madhyamas-core/src/extension.rs` | In `ScriptExtension::on_request/on_response` (line 272-322), apply script modifications back to `ExtensionContext` (write modified headers/body back); handle `handled`/short-circuit |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | In `run_request_hooks` (line 1197), when `ExtensionResult.handled` is true, short-circuit the pipeline with the custom response |
| `crates/madhyamas-api/src/phase3_handlers.rs` | Add `GET /api/scripts/{id}/history` (execution history); add `POST /api/scripts/{id}/test` (dry-run against a captured request); add `GET /api/scripts/{id}/console` (console output) |
| `crates/madhyamas-api/src/routes.rs` | Register the new script endpoints |
| `crates/madhyamas-cli/src/commands/scripts.rs` | Add `madhyamas scripts history <id>`, `madhyamas scripts test <id> --request <traffic_id>`, `madhyamas scripts validate --file F`; fix `toggle` command to send `{enabled: bool}` instead of `{}` |
| `crates/madhyamas-mcp/src/tools/scripts.rs` | Add `madhyamas_test_script`, `madhyamas_get_script_history` tools |
| `crates/madhyamas-mcp/src/tools/registry.rs` | Register the new MCP tools |
| `crates/madhyamas/src/main.rs` | Call `script_runtime.load_from_directory()` on startup if `~/.madhyamas/scripts/` exists; load persisted scripts from SQLite |

### 8.3 Database Schema

Add a `scripts` table to the SQLite database:

```sql
CREATE TABLE IF NOT EXISTS scripts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    source TEXT NOT NULL,
    hooks TEXT NOT NULL,        -- JSON array
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 100,
    created_at TEXT NOT NULL,
    modified_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS script_executions (
    id TEXT PRIMARY KEY,
    script_id TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    success INTEGER NOT NULL,
    error TEXT,
    console TEXT,               -- JSON array of strings
    timestamp TEXT NOT NULL,
    FOREIGN KEY (script_id) REFERENCES scripts(id) ON DELETE CASCADE
);
```

### 8.4 Config Changes

Add to `ProxyConfig` (`config.rs`):
```rust
pub struct ScriptingConfig {
    pub enabled: bool,
    pub timeout_ms: u64,
    pub max_memory_bytes: usize,
    pub max_instructions: u64,
    pub allow_network: bool,
    pub allow_fs: bool,
    pub script_dirs: Vec<String>,  // directories to load scripts from
}
```

Expose via `GET/PATCH /api/config` and CLI flags
(`--script-timeout`, `--script-max-memory`, `--allow-script-network`).

---

## 9. Frontend Changes

### 9.1 ScriptsPanel Enhancements (`web/src/features/tools/ScriptsPanel.tsx`)

| Enhancement | Description |
|---|---|
| **Console output panel** | Below the code editor, show a console panel with `console.log` output from the last execution. Use a terminal-style component. |
| **Execution history** | A "History" tab showing recent executions (timestamp, duration, success/error, console output) for the selected script. |
| **Error display** | When a script has a parse or runtime error, show it inline in the editor with line/column (use a gutter marker or inline highlight). |
| **Dry-run / test** | A "Test" button that opens a dialog to select a captured traffic entry, runs the script against it, and shows the result (modified request/response, console output) without affecting live traffic. |
| **Hook selector** | A multi-select for choosing which hooks a script subscribes to (currently a free-text field in the API; the UI should present the 7 known hooks as checkboxes). |
| **Priority field** | A numeric input for script priority (lower = runs earlier). |
| **Script status indicator** | Show a green/red dot next to each script indicating whether the last execution succeeded or failed. |
| **Save & Run** | A button that saves the script and immediately triggers a test execution against the most recent traffic entry. |

### 9.2 New Components

| Component | File | Purpose |
|---|---|---|
| `ScriptConsole` | `web/src/features/tools/ScriptConsole.tsx` | Terminal-style console output viewer |
| `ScriptHistory` | `web/src/features/tools/ScriptHistory.tsx` | Execution history list with expandable details |
| `ScriptTestDialog` | `web/src/features/tools/ScriptTestDialog.tsx` | Dialog for selecting a traffic entry and dry-running a script |
| `ScriptErrorGutter` | `web/src/features/tools/ScriptErrorGutter.tsx` | Code editor gutter marker for syntax/runtime errors |

### 9.3 API Hooks (`web/src/lib/api/phase3.ts`)

Add:
```typescript
export function useScriptHistory(id: string) { ... }       // GET /scripts/{id}/history
export function useTestScript() { ... }                    // POST /scripts/{id}/test
export function useScriptConfig() { ... }                  // GET/PATCH /scripts/config (resource limits)
```

### 9.4 Types

Update `Script` interface to include `priority` and `last_execution`:
```typescript
export interface Script {
  id: string;
  name: string;
  source: string;
  description?: string;
  enabled: boolean;
  hooks: string[];
  priority: number;
  created_at: string;
  modified_at: string;
  last_execution?: {
    success: boolean;
    error?: string;
    duration_ms: number;
    timestamp: string;
  };
}

export interface ScriptExecution {
  script_id: string;
  duration_ms: number;
  success: boolean;
  error?: string;
  console: string[];
  timestamp: string;
}
```

---

## 10. Documentation Changes

### 10.1 New Documents

| Document | Content |
|---|---|
| `docs/SCRIPTING.md` | End-user guide: how to create scripts, available hooks, the request/response/context objects, the script API (console, JSON, base64, crypto, url), examples, troubleshooting |
| `docs/SCRIPTING_API.md` | Complete API reference for the script runtime — every function, object, and property available to scripts, with signatures and examples |
| `docs/SCRIPTING_SECURITY.md` | Security model: sandboxing, resource limits, trust levels, what scripts can and cannot do, best practices for untrusted scripts |

### 10.2 Updated Documents

| Document | Change |
|---|---|
| `CLAUDE.md` | Update the scripting section: note that scripts are now functional; add `ScriptingConfig` to the config section; add new API endpoints to the table; add new CLI commands |
| `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` | Change Scripting row from 🟡 to ✅ |
| `docs/ARCHITECTURE.md` | Add the JS engine to the architecture diagram; describe the script execution flow |
| `docs/PROXY_FLOW.md` | Add scripting hooks to the request/response flow diagram |
| `.claude/skills/madhyamas/SKILL.md` | Add scripting workflow: how to create, test, and debug scripts via the web UI, CLI, and MCP |

### 10.3 In-UI Documentation

- Add a "Help" button in the ScriptsPanel that opens a modal with the
  script API documentation (currently in `api.rs:11-137` as markdown —
  render it in the UI)
- Add inline examples in the template cards (show a preview of the source
  code before creating)

---

## 11. Implementation Phases

### Phase 1: Make Scripts Run (Critical)

**Goal:** Scripts execute and can modify traffic.

| Task | Effort |
|---|---|
| Add `boa_engine` dependency | Small |
| Create `scripting/engine.rs` with `JsEngine::execute()` | Medium |
| Register globals (console, JSON, base64, crypto, url) | Medium |
| Wire `JsEngine` into `ScriptRuntime::execute()` | Small |
| Apply script results back to `ExtensionContext` | Small |
| Handle short-circuit (custom response) in pipeline | Small |
| Enforce timeout via `tokio::time::timeout` | Small |
| Add SQLite persistence for scripts | Medium |
| Test with the 5 built-in templates | Small |

**Estimated effort:** 3-5 days. **Impact:** High — makes the entire
scripting system functional.

### Phase 2: Usability (Important)

**Goal:** Users can debug and monitor script execution.

| Task | Effort |
|---|---|
| Record real `ScriptExecution` in history | Small |
| Add `GET /api/scripts/{id}/history` endpoint | Small |
| Add console output streaming to web UI | Medium |
| Add error display with line/column in editor | Medium |
| Add dry-run / test endpoint and UI | Medium |
| Add hook selector checkboxes in UI | Small |
| Fix CLI `toggle` command bug | Trivial |

**Estimated effort:** 2-3 days. **Impact:** Medium — makes scripts
debuggable.

### Phase 3: Enhancement (Nice-to-Have)

**Goal:** Advanced features for power users.

| Task | Effort |
|---|---|
| TypeScript transpilation via `swc` | Medium |
| Bytecode caching for performance | Medium |
| Script module system | Hard |
| Script marketplace (via plugin registry) | Medium |
| Debugging (breakpoints, step-through) | Hard |
| Advanced hooks (on_connect, on_error) | Small |
| Memory limit enforcement | Medium |

**Estimated effort:** 1-2 weeks. **Impact:** Medium — nice-to-have features
that expand the user base.

---

*Generated 2026-08-01. Based on codebase analysis as of this date.*
