# Plugin System

Madhyamas supports a WASM-based plugin system that allows you to extend the
proxy with custom request/response interception, modification, logging, and
more. Plugins run in a sandboxed `wasmtime` runtime with fuel-metered CPU
limits and capped linear memory — they cannot access the filesystem, network,
or host memory unless explicitly granted.

## Quick Start

### Installing a Plugin

```bash
# From a URL
madhyamas plugins install https://example.com/my-plugin.zip

# From the registry
madhyamas plugins install --source registry my-plugin

# With checksum verification
madhyamas plugins install https://example.com/my-plugin.zip --checksum abc123...
```

### Managing Plugins

```bash
# List loaded plugins
madhyamas plugins list

# Enable/disable
madhyamas plugins enable my-plugin
madhyamas plugins disable my-plugin

# View stats
madhyamas plugins stats my-plugin

# View recent invocation logs
madhyamas plugins logs my-plugin --limit 20

# Reload all plugins from disk
madhyamas plugins reload

# Uninstall
madhyamas plugins uninstall my-plugin
```

### Plugin Settings

```bash
# Get settings schema
madhyamas plugins schema my-plugin

# Get current settings
madhyamas plugins get-settings my-plugin

# Update settings (pass JSON)
madhyamas plugins set-settings my-plugin --settings '{"key": "value"}'
```

### Registry

The plugin registry is backed by a **GitHub repository** containing a
`plugins/registry.json` catalog file. The default registry repo is
`shristilabs/madhyamas` (the same repo as the project itself — configurable).
Plugin packages are distributed as GitHub release assets attached to
the same repo's releases.

```bash
# List available plugins in the registry
madhyamas plugins registry

# Search the registry
madhyamas plugins search "cors"

# Show current registry config (repo, catalog URL, entry count)
madhyamas plugins registry-config

# Set the registry repo (e.g. "owner/repo" or "owner/repo@branch")
madhyamas plugins registry-config owner/repo@main

# Custom catalog path (owner/repo@branch:path/to/catalog.json)
madhyamas plugins registry-config owner/repo@dev:custom/catalog.json

# Force-refresh the registry cache
madhyamas plugins registry-refresh
```

#### Registry Catalog Format

The registry repo must contain a `plugins/registry.json` file (path is
configurable via the repo reference syntax):

```json
{
  "version": 1,
  "plugins": [
    {
      "manifest": {
        "id": "com.example.my-plugin",
        "name": "My Plugin",
        "version": "1.0.0",
        "description": "Does something useful",
        "main": "plugin.wasm",
        "hooks": ["on_request", "on_response"],
        "capabilities": ["intercept_request"],
        "dependencies": {},
        "enabled_by_default": false,
        "network": false,
        "max_memory_pages": 64,
        "fuel_limit": 10000000,
        "tags": ["example"],
        "panels": []
      },
      "download_url": "https://github.com/owner/repo/releases/download/v1.0/my-plugin.zip",
      "checksum": "sha256:abc123...",
      "downloads": 0,
      "rating": 5.0,
      "rating_count": 1,
      "tags": ["example"],
      "added_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

The `download_url` can point to:
- A GitHub release asset (recommended — stable URLs, CDN-backed)
- A raw file in the repo (`https://raw.githubusercontent.com/...`)
- Any HTTPS URL serving a zip file

The `checksum` is a SHA-256 hash of the zip file bytes (hex-encoded,
optionally prefixed with `sha256:`).

#### Submitting Plugins

To add a plugin to the registry:
1. Build your plugin and package it as a zip
2. Create a GitHub release with the zip as an asset
3. Fork the registry repo, add your plugin entry to `registry.json`
4. Open a PR to merge your entry into the catalog

### Plugin Signing (Ed25519)

```bash
# Generate a publisher keypair
madhyamas plugins gen-key
# Output:
#   public_key:  <hex>
#   secret_key:  <hex>

# Sign a plugin zip package
madhyamas plugins sign my-plugin.zip --secret-key <hex>
# Writes signature.sig alongside the zip

# Install a signed plugin (verification is automatic when the manifest
# declares a publisher_public_key)
madhyamas plugins install https://example.com/my-plugin.zip --checksum <sha256>
```

### Scaffolding New Plugins from Templates

```bash
# List available templates
madhyamas plugins templates

# Scaffold a new plugin from a template
madhyamas plugins new cors my-cors-plugin --output ./plugins

# Available templates: basic, cors, request-logger, domain-blocker, response-modifier
```

### Hot-Reload

When the `wasm-runtime` feature is enabled, the `HotReloader` watches plugin
directories for changes to `.wasm`, `.toml`, and `.json` files. When a file
changes, the affected plugins are automatically reloaded (with a 500ms
debounce to avoid reload storms during bulk writes). No manual `reload`
command is needed during development.

## Architecture

### Host Side (`madhyamas-core`)

- **`plugin/wasm_runtime.rs`** — `WasmRuntime`: the `wasmtime` engine, module
  cache, host ABI (`log` host function), fuel metering, and memory caps.
- **`plugin/manager.rs`** — `PluginManager`: plugin lifecycle (load/enable/
  disable/unload), hook dispatch, invocation logging, timer scheduling.
- **`plugin/persistence.rs`** — `PluginPersistence`: SQLite storage for
  plugin state (enabled flag, settings) and invocation audit logs.
- **`plugin/installer.rs`** — `PluginInstaller`: download, checksum verify,
  Ed25519 signature verification, zip extract, and install/uninstall.
- **`plugin/registry.rs`** — `PluginRegistry`: GitHub-backed plugin catalog
  (fetches `registry.json` from a GitHub repo via `raw.githubusercontent.com`)
  + local plugin discovery. Configurable repo via CLI/API.
- **`plugin/signing.rs`** — Ed25519 key generation, signing, and verification
  utilities for plugin publishers.
- **`plugin/hot_reload.rs`** — `HotReloader`: filesystem watcher (via
  `notify`) that auto-reloads plugins on `.wasm`/manifest changes.
- **`plugin/event_bus.rs`** — `PluginEventBus`: in-process pub/sub for
  inter-plugin communication.
- **`plugin/templates.rs`** — `PluginTemplates`: built-in project scaffolding
  templates (basic, cors, request-logger, domain-blocker, response-modifier).
- **`plugin/types.rs`** — `PluginManifest`, `PluginCapability`,
  `PluginPanel`, `PluginPanelKind`, etc.
- **`plugin/hooks.rs`** — `PluginHook` enum, `PluginContext`, `PluginResult`.

### Guest Side (`madhyamas-plugin-sdk`)

The `madhyamas-plugin-sdk` crate provides the types and macro for writing
plugins in Rust that compile to `plugin.wasm`.

See [PLUGIN_DEVELOPMENT.md](PLUGIN_DEVELOPMENT.md) for a guide.

## Security

- **Sandboxing**: WASM plugins are sandboxed by design. No filesystem,
  network, or host memory access unless explicitly linked.
- **CPU**: each invocation gets a fuel budget (default 10M instructions,
  configurable via `fuel_limit` in the manifest).
- **Memory**: linear memory capped at 256 MiB (host-side ceiling).
- **Network**: `http_fetch` is **not** linked in v1 (the `Network`
  capability is declared-only).
- **Checksums**: plugin packages are verified with SHA-256 on install.
- **Signing**: Ed25519 signature verification is fully implemented. When a
  plugin manifest declares a `publisher_public_key`, the installer verifies
  a `signature.sig` file (detached Ed25519 signature over the zip bytes)
  against the declared public key. Use `madhyamas plugins gen-key` and
  `madhyamas plugins sign` to sign plugins.

See [PLUGIN_SECURITY.md](PLUGIN_SECURITY.md) for details.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/plugins` | List all plugins |
| GET | `/api/plugins/{id}` | Get a plugin |
| POST | `/api/plugins/{id}/enable` | Enable a plugin |
| POST | `/api/plugins/{id}/disable` | Disable a plugin |
| GET | `/api/plugins/{id}/stats` | Get plugin stats |
| POST | `/api/plugins/reload` | Reload all plugins |
| POST | `/api/plugins/install` | Install a plugin |
| DELETE | `/api/plugins/{id}/uninstall` | Uninstall a plugin |
| GET | `/api/plugins/{id}/settings` | Get plugin settings |
| PUT | `/api/plugins/{id}/settings` | Update plugin settings |
| GET | `/api/plugins/{id}/schema` | Get settings schema |
| GET | `/api/plugins/{id}/panels` | Get declarative UI panels |
| GET | `/api/plugins/{id}/logs` | Get invocation logs |
| GET | `/api/plugins/registry` | List registry entries |
| GET | `/api/plugins/registry/search?q=` | Search registry |
| GET | `/api/plugins/registry/{id}` | Get registry entry |
| GET | `/api/plugins/registry/config` | Get registry config (repo, URL) |
| PUT | `/api/plugins/registry/config` | Set registry repo |
| POST | `/api/plugins/registry/refresh` | Force-refresh registry cache |
| GET | `/api/plugins/templates` | List plugin templates |
| POST | `/api/plugins/scaffold` | Scaffold a new plugin from a template |

## MCP Tools

- `madhyamas_list_plugins` — list loaded plugins
- `madhyamas_get_plugin` — get plugin details
- `madhyamas_enable_plugin` / `madhyamas_disable_plugin`
- `madhyamas_get_plugin_stats` — runtime statistics
- `madhyamas_reload_plugins` — reload from disk
- `madhyamas_install_plugin` — install from URL or registry
- `madhyamas_uninstall_plugin` — uninstall
- `madhyamas_search_registry` / `madhyamas_list_registry`
- `madhyamas_get_plugin_schema` — settings schema
- `madhyamas_get_plugin_settings` / `madhyamas_update_plugin_settings`
- `madhyamas_get_plugin_logs` — invocation logs
