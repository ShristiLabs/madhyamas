---
title: Plugins
description: Extend Madhyamas with sandboxed WebAssembly plugins — install from the registry, manage, configure, sign, and scaffold new plugins with the madhyamas-plugin-sdk.
---

# Plugins

Plugins let you extend Madhyamas with custom logic compiled to **WebAssembly (WASM)**. They run in a sandboxed `wasmtime` runtime with strict CPU and memory limits, so you can add new interception, modification, or logging behavior without compromising the proxy's stability or security.

![Plugins View](/screenshots/plugins-view.png)

## How Plugins Work

A plugin is a `.wasm` module with a manifest that declares which hooks it subscribes to (e.g. `on_request`, `on_response`) and what capabilities it needs. When traffic flows through the proxy, the plugin manager dispatches matching events to each enabled plugin. Plugins can inspect and modify traffic, log activity, and expose their own settings and UI panels.

Plugins are sandboxed by design:

- **No filesystem, network, or host memory access** unless explicitly granted
- **CPU** is bounded by a fuel budget (default 10 million instructions per invocation)
- **Memory** is capped (linear memory limited to 256 MiB)
- **Packages are verified** with SHA-256 checksums on install, and optionally signed with Ed25519

## Installing a Plugin

### From the Registry

The plugin registry is a GitHub-backed catalog of plugins. Browse and install from it directly:

```bash
# List available plugins in the registry
madhyamas plugins registry

# Search the registry
madhyamas plugins search "cors"

# Install a plugin from the registry
madhyamas plugins install --source registry my-plugin
```

### From a URL

```bash
# Install from a direct URL
madhyamas plugins install https://example.com/my-plugin.zip

# With checksum verification
madhyamas plugins install https://example.com/my-plugin.zip --checksum abc123...
```

### From the Web UI

Open the **Plugins** view to browse the registry, install, enable, and configure plugins without leaving the browser.

## Managing Plugins

```bash
madhyamas plugins list                 # List loaded plugins
madhyamas plugins enable my-plugin     # Enable a plugin
madhyamas plugins disable my-plugin    # Disable a plugin
madhyamas plugins stats my-plugin      # View runtime statistics
madhyamas plugins logs my-plugin       # View recent invocation logs
madhyamas plugins reload               # Reload all plugins from disk
madhyamas plugins uninstall my-plugin  # Uninstall a plugin
```

Each plugin has a toggle switch in the web UI. Disabled plugins stay installed but don't process traffic.

## Plugin Settings

Plugins can declare a settings schema, so they expose configurable options that you can change at runtime:

```bash
madhyamas plugins schema my-plugin              # Get the settings schema
madhyamas plugins get-settings my-plugin        # Get current settings
madhyamas plugins set-settings my-plugin \
  --settings '{"key": "value"}'                 # Update settings
```

In the web UI, plugin settings appear in the Plugins panel when you select a plugin.

## The Plugin Registry

The registry is backed by a GitHub repository containing a `plugins/registry.json` catalog. The default registry is the Madhyamas project repo itself, but you can point it at any fork or private catalog:

```bash
madhyamas plugins registry-config                       # Show current config
madhyamas plugins registry-config owner/repo@main       # Set the registry repo
madhyamas plugins registry-refresh                      # Force-refresh the cache
```

### Submitting a Plugin

1. Build your plugin and package it as a zip
2. Create a GitHub release with the zip attached as an asset
3. Fork the registry repo and add your plugin entry to `registry.json`
4. Open a pull request to merge your entry into the catalog

## Hot Reload

During development, Madhyamas watches plugin directories for changes to `.wasm`, manifest, and settings files. When a file changes, the affected plugins reload automatically (with a short debounce to avoid reload storms). No manual reload command is needed while iterating on a plugin.

## Scaffolding New Plugins

If you want to write your own plugin, start from a built-in template:

```bash
madhyamas plugins templates                       # List available templates
madhyamas plugins new cors my-cors-plugin \
  --output ./plugins                              # Scaffold from the cors template
```

Available templates: `basic`, `cors`, `request-logger`, `domain-blocker`, `response-modifier`.

Plugins are written in Rust using the `madhyamas-plugin-sdk` crate and compiled to `plugin.wasm`. See the plugin development guide in the `docs/` directory for the full walkthrough.

## Plugin Signing

Plugin publishers can sign packages with Ed25519 so installers can verify authenticity:

```bash
madhyamas plugins gen-key                         # Generate a publisher keypair
madhyamas plugins sign my-plugin.zip \
  --secret-key <hex>                               # Sign a package (writes signature.sig)
```

When a plugin manifest declares a `publisher_public_key`, the installer automatically verifies the signature on install.

## Common Use Cases

### Adding Custom Interception Logic

Write a plugin when [scripts](./scripting) aren't enough — for example, when you need complex state, custom protocols, or reusable distributable logic.

### Sharing Reusable Tools

Publish a plugin to the registry so your team (or the community) can install a consistent debugging tool with a single command.

### Protocol-Specific Helpers

Build plugins that understand specific protocols (e.g. a GraphQL inspector, a protobuf decoder) and surface their own UI panels in the web UI.

### Team Standards

Distribute a plugin that enforces your team's debugging conventions — logging format, header injection, or traffic filtering — across every developer's proxy.

## See also

- [Scripting](./scripting) — sandboxed JavaScript for lighter-weight automation
- [Security Overview](./security) — plugin sandboxing and signing
- [CLI reference](./cli) — `madhyamas plugins` subcommands
- [REST API reference](./rest-api) — `/api/plugins` endpoints
- [Plugin development guide](https://github.com/ShristiLabs/madhyamas/blob/main/docs/PLUGIN_DEVELOPMENT.md) — writing your own plugin (developer docs)
