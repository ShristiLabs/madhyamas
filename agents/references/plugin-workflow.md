# Plugin Engineering Workflow

End-to-end workflow for developing, testing, and documenting a new Madhyamas WASM
plugin. Load this reference before creating or modifying plugins.

## Architecture

- **Host runtime**: `wasmtime` with fuel metering, in `crates/madhyamas-core/src/plugin/`.
  - `wasm_runtime.rs` — `WasmRuntime`: engine, host ABI, module cache
  - `manager.rs` — `PluginManager`: lifecycle, hook dispatch, invocation logging
  - `persistence.rs` — `PluginPersistence`: SQLite state + invocation logs
  - `installer.rs` — `PluginInstaller`: download, checksum, Ed25519 signature verify, zip extract
  - `registry.rs` — `PluginRegistry`: local + remote catalog with search
  - `signing.rs` — Ed25519 key generation, signing, verification
  - `hot_reload.rs` — `HotReloader`: filesystem watcher (notify)
  - `event_bus.rs` — `PluginEventBus`: in-process pub/sub
  - `templates.rs` — `PluginTemplates`: built-in scaffolding (basic, cors, request-logger, domain-blocker, response-modifier)
  - `types.rs` — `PluginManifest`, `PluginCapability`, `PluginPanel`, `PluginSettingsSchema`
  - `hooks.rs` — `PluginHook` enum, `PluginContext`, `PluginResult`
- **Guest SDK**: `crates/madhyamas-plugin-sdk/` — `Plugin` trait, `register_plugin!` macro,
  `Context`/`Outcome` types, bump allocator, WASM entry points.
- **Examples**: `plugins/cors-helper/`, `plugins/domain-blocker/`, `plugins/request-logger/`
  and SDK examples in `crates/madhyamas-plugin-sdk/examples/`.
- **Catalog**: `plugins/registry.json` — local plugin index.

## Plugin Lifecycle

1. **Scaffold** from a template (`PluginTemplates::basic` / `cors` / etc.) or copy an
   existing example under `plugins/<name>/`.
2. **Implement** the `Plugin` trait in `lib.rs`; register with `register_plugin!(MyPlugin)`.
3. **Declare** capabilities in `Cargo.toml` and `manifest.json` (`PluginManifest`):
   `name`, `version`, `description`, `capabilities` (list of `PluginCapability`),
   optional `panels` (`PluginPanel` with `PluginPanelKind`) and `settings_schema`.
4. **Build** to WASM: `cargo build --release -p <plugin-name> --target wasm32-unknown-unknown`.
5. **Sign** the `.wasm` with Ed25519 (`crates/madhyamas-core/src/plugin/signing.rs`).
   The signature is required for installation via `PluginInstaller`.
6. **Package** as a zip containing the `.wasm` + `manifest.json` + signature file.
7. **Install** via `PluginInstaller::install_from_zip` or the API/CLI.
8. **Register** in `plugins/registry.json` so the catalog discovers it.
9. **Test** (see below).
10. **Document** (see below).

## Hooks

A plugin implements one or more hooks (`PluginHook` in `hooks.rs`):

- `on_request` — inspect/modify an outgoing request
- `on_response` — inspect/modify a response
- `on_connect` — observe connection establishment
- `on_traffic_recorded` — react to a stored traffic entry

Each hook receives a `PluginContext` and returns a `PluginResult`. See
`docs/PLUGIN_API.md` for the full ABI.

## Capabilities & Panels

- **Capabilities** gate what host APIs the plugin may call (principle of least privilege).
  Declare only what the plugin uses.
- **Panels** let a plugin render custom UI in the web app. A `PluginPanel` declares a
  `PluginPanelKind` (e.g. sidebar panel, detail tab) and an optional `settings_schema`
  (JSON-Schema-ish) for user-configurable options.

## Testing

Per project rules, do NOT write test cases unless explicitly asked. When asked:

1. **Unit tests** in the plugin crate (`#[cfg(test)] mod tests`) covering pure logic
   (header manipulation, URL matching, etc.) — no WASM needed.
2. **Host integration** via `PluginManager` in `madhyamas-core` tests: load the compiled
   `.wasm`, dispatch a synthetic `PluginContext`, assert the `PluginResult`.
3. **Fuel/timeout**: verify the plugin completes within fuel budget; a runaway plugin
   must be killed, not hang the proxy.
4. **Signature**: verify `signing.rs` round-trips (sign → verify → tamper → reject).
5. **Hot reload**: drop a new `.wasm` next to the installed one and confirm
   `HotReloader` picks it up without restart.

## Documentation Checklist

For every new or modified plugin, update:

- [ ] `docs/PLUGINS.md` — overview, install/usage, catalog entry
- [ ] `docs/PLUGIN_DEVELOPMENT.md` — development workflow, build commands, manifest schema
- [ ] `docs/PLUGIN_API.md` — host ABI, hook signatures, context/outcome fields
- [ ] `docs/PLUGIN_SECURITY.md` — capabilities, sandboxing, signing, fuel limits
- [ ] `docs-site/plugins.md` — end-user how-to (install, enable, configure, view panels)
- [ ] `plugins/registry.json` — catalog metadata (name, version, description, download URL, checksum)
- [ ] Plugin `README.md` (inside `plugins/<name>/`) — what it does, config, examples

## Security Rules (non-negotiable)

- Every distributable plugin MUST be Ed25519-signed; the installer rejects unsigned/tampered.
- Capabilities MUST be minimal. A plugin that only reads headers must NOT request write caps.
- Fuel metering MUST be on in production. Never ship a plugin that requires unlimited fuel.
- Never log secrets, cookies, or auth tokens from plugin output.
- A plugin MUST NOT be able to spawn processes, access the filesystem outside its sandbox,
  or make arbitrary network calls. The host ABI is the only surface.

## Common Pitfalls

- Forgetting to bump `version` in both `Cargo.toml` and `manifest.json` (installer checks).
- Declaring a capability but not using it (lint warning) — or using it without declaring
  (runtime denial).
- Returning a modified `PluginResult` that drops fields the host needs (e.g. clearing
  headers the proxy requires for routing).
- Panicking in WASM — convert to a `PluginResult::error` instead; panics abort the
  invocation and log, but should be avoided.
