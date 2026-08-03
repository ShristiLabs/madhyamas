# Plugin Development Guide

This guide walks you through writing a Madhyamas plugin in Rust, compiling it
to WASM, and packaging it for installation.

## Prerequisites

```bash
# Add the WASM target
rustup target add wasm32-unknown-unknown
```

## Quick Start: Scaffold from a Template

The fastest way to start a new plugin is to use a built-in template:

```bash
# List available templates
madhyamas plugins templates

# Scaffold a new plugin (e.g. from the CORS template)
madhyamas plugins new cors my-cors-plugin --output ./plugins

# Build it
cd plugins/my-cors-plugin
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/my_cors_plugin.wasm plugin.wasm

# Install it
madhyamas plugins install --source url file://$(pwd)
```

Available templates:
- **basic** — minimal plugin that logs every request
- **cors** — adds CORS headers to every response
- **request-logger** — logs method, host, and path of every request
- **domain-blocker** — blocks requests to configurable domains (with settings)
- **response-modifier** — modify response headers and body

## Manual Project Setup

Create a new crate that depends on `madhyamas-plugin-sdk`:

```toml
# Cargo.toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
madhyamas-plugin-sdk = { path = "../madhyamas/crates/madhyamas-plugin-sdk" }
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
```

## Your First Plugin

```rust
// src/lib.rs
#![no_std]
extern crate alloc;

use madhyamas_plugin_sdk::{log, log_level, register_plugin, Context, Outcome, Plugin};

#[derive(Default)]
struct MyPlugin;

impl Plugin for MyPlugin {
    fn on_load(&mut self, _ctx: &mut Context) -> Outcome {
        log(log_level::INFO, "my-plugin loaded!");
        Outcome::pass()
    }

    fn on_request(&mut self, ctx: &mut Context) -> Outcome {
        if let Some(req) = ctx.request() {
            log(log_level::INFO, &format!("Request: {} {}", req.method, req.url));
        }
        Outcome::pass()
    }

    fn on_response(&mut self, ctx: &mut Context) -> Outcome {
        if let Some(resp) = ctx.response_mut() {
            resp.headers.insert("X-Custom-Header".into(), "my-plugin".into());
        }
        Outcome::modified()
    }
}

register_plugin!(MyPlugin);
```

## Hooks

The `Plugin` trait has methods for each hook. Implement only the ones you need:

| Method | When | Common Use |
|--------|------|------------|
| `on_load` | Plugin loaded | Initialization |
| `on_enable` | Plugin enabled | Start resources |
| `on_disable` | Plugin disabled | Stop resources |
| `on_unload` | Plugin unloaded | Cleanup |
| `on_request` | Before sending request | Modify/block requests |
| `on_response` | After receiving response | Modify responses |
| `on_websocket` | WebSocket message | Inspect/modify WS |
| `on_grpc` | gRPC message | Inspect/modify gRPC |
| `on_settings_change` | Settings updated | React to config |
| `on_timer` | Timer interval fires | Periodic tasks |

## Outcomes

Return an `Outcome` from each hook:

```rust
// Pass through (no modification)
Outcome::pass()

// Mark as modified (changes to ctx.request/ctx.response are applied)
Outcome::modified()

// Short-circuit with a custom response
Outcome::respond(403, "Blocked!")

// Error (stops the chain)
Outcome::error("something went wrong")
```

## Settings

Declare settings in your manifest:

```toml
# madhyamas-plugin.toml
id = "my-plugin"
name = "My Plugin"
version = "1.0.0"
hooks = ["on_request"]

[settings]
[[settings.fields]]
key = "max_requests"
label = "Max Requests"
field_type = "number"
default = 100
description = "Maximum requests per minute"
```

Read settings in your plugin:

```rust
fn on_request(&mut self, ctx: &mut Context) -> Outcome {
    let max = ctx.setting("max_requests")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);
    // ...
    Outcome::pass()
}
```

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

The output is at `target/wasm32-unknown-unknown/release/my_plugin.wasm`.
Rename it to `plugin.wasm`.

## Packaging

Create a directory (or zip) with:

```
my-plugin/
├── madhyamas-plugin.toml   # Manifest
└── plugin.wasm             # Compiled WASM
```

### Manifest Format

```toml
id = "com.example.my-plugin"       # Unique plugin id
name = "My Plugin"                  # Display name
version = "1.0.0"                   # Semver
description = "Does cool things"
author = "Your Name"
license = "MIT"
hooks = ["on_request", "on_response"]
enabled_by_default = false

# Capabilities (declared, not enforced in v1)
capabilities = ["intercept_request", "intercept_response"]

# Resource limits
max_memory_pages = 64               # 64 * 64KiB = 4 MiB
fuel_limit = 10000000              # 10M WASM instructions per invocation
timer_interval_seconds = 30        # Fire on_timer every 30s (optional)

# Dependencies (other plugins, with semver constraints)
[dependencies]
"com.example.other" = "^1.0"

# Settings schema (for UI generation)
[[settings.fields]]
key = "api_key"
label = "API Key"
field_type = "text"
required = true

[[settings.fields]]
key = "timeout"
label = "Timeout (ms)"
field_type = "number"
default = 5000
```

## Example Plugins

The SDK includes three example plugins in `crates/madhyamas-plugin-sdk/examples/`:

- **`cors_helper.rs`** — adds CORS headers to every response
- **`request_logger.rs`** — logs every request's method and host
- **`domain_blocker.rs`** — blocks requests to configurable domains (settings + short-circuit)

Build them with:

```bash
cargo build --target wasm32-unknown-unknown --example cors_helper --release -p madhyamas-plugin-sdk
cargo build --target wasm32-unknown-unknown --example request_logger --release -p madhyamas-plugin-sdk
cargo build --target wasm32-unknown-unknown --example domain_blocker --release -p madhyamas-plugin-sdk
```

## Installation

Place the plugin directory in `~/.madhyamas/plugins/` or use the installer:

```bash
# From a zip URL
madhyamas plugins install https://example.com/my-plugin.zip

# From the registry
madhyamas plugins install --source registry com.example.my-plugin
```

## Signing Your Plugin (Ed25519)

To establish trust with users, sign your plugin packages with an Ed25519
keypair. When a manifest declares a `publisher_public_key`, the installer
verifies the signature before installing.

```bash
# 1. Generate a keypair (do this once and store the secret key securely)
madhyamas plugins gen-key
# Output:
#   public_key:  <hex 64 chars>
#   secret_key:  <hex 64 chars>

# 2. Add the public key to your manifest (madhyamas-plugin.toml)
# publisher_public_key = "<hex public key>"

# 3. Package your plugin as a zip
cd ~/.madhyamas/plugins/my-plugin
zip -r ../my-plugin.zip .

# 4. Sign the zip
madhyamas plugins sign ../my-plugin.zip --secret-key <hex secret key>
# Writes signature.sig alongside the zip

# 5. Include signature.sig in the final zip package
zip ../my-plugin.zip signature.sig
```

When a user installs `my-plugin.zip`, the installer:
1. Extracts the zip
2. Reads `publisher_public_key` from the manifest
3. Reads `signature.sig` from the extracted files
4. Verifies the Ed25519 signature against the raw zip bytes
5. Aborts installation if verification fails

## Declarative UI Panels

Plugins can declare UI panels in their manifest that the web UI renders
automatically. This allows plugins to provide custom UI without shipping
JavaScript.

```toml
# madhyamas-plugin.toml
[[panels]]
id = "overview"
title = "Plugin Overview"
kind = "markdown"
order = 0

[panels.content]
markdown = """
## About this plugin

This plugin adds CORS headers to all responses.

### Configuration

Use the Settings tab to configure allowed origins.
"""

[[panels]]
id = "dashboard"
title = "Statistics"
kind = "stats"
icon = "bar-chart"
order = 1

[[panels]]
id = "custom-widget"
title = "Custom Widget"
kind = "widget"
order = 2

[panels.content]
html = "<div id='root'></div>"
script = "document.getElementById('root').innerText = 'Hello from plugin!';"
```

Supported panel kinds:
- **`markdown`** — static markdown content (rendered as read-only docs)
- **`settings`** — auto-generated settings form from the plugin's settings schema
- **`logs`** — live invocation log table
- **`stats`** — plugin statistics dashboard
- **`widget`** — custom HTML/JS rendered in a sandboxed iframe

## Hot-Reload During Development

When the `wasm-runtime` feature is enabled, the `HotReloader` watches plugin
directories for changes to `.wasm`, `.toml`, and `.json` files. During
development, simply rebuild your plugin and copy the new `.wasm` file into
the plugin directory — the plugin will be automatically reloaded within
500ms. No manual `reload` command is needed.

## Inter-Plugin Communication (Event Bus)

Plugins can communicate with each other via the in-process event bus
(`PluginEventBus`). This enables loosely-coupled pub/sub communication
without direct dependencies.

> **Note**: The event bus is currently accessible from the host side. A
> WASM host function for plugins to publish/subscribe is planned for a
> future release.

## Debugging

View invocation logs:

```bash
madhyamas plugins logs my-plugin --limit 50
```

Logs include:
- Host-side log lines (from the `log()` function)
- Duration, fuel consumed, success/error status
- Whether the request/response was modified
