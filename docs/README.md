# Madhyamas Documentation

Developer-facing reference documentation for Madhyamas, an open-source
HTTP/HTTPS debugging proxy built in Rust with an embedded React web UI.

For end-user documentation (with screenshots), see the
[User Guide (GitHub Pages)](https://shristilabs.github.io/madhyamas/).

## Quick Start

```bash
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
cargo build --release
./target/release/madhyamas
```

Open `http://localhost:3001` for the web UI. Configure your browser to use
`localhost:8888` as the proxy.

## Documentation Index

### Architecture & Internals

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System architecture, workspace layout, data flow, feature flags |
| [PROXY_FLOW.md](PROXY_FLOW.md) | End-to-end proxy flow with TLS interception detail |
| [INTERCEPT_PIPELINE.md](INTERCEPT_PIPELINE.md) | Intercept handler trait, priority pipeline (block list → rewrites → mocks → breakpoints → throttle) |
| [EXTENSION_SYSTEM.md](EXTENSION_SYSTEM.md) | Unified `Extension` trait abstracting scripts and plugins |
| [PERSISTENCE.md](PERSISTENCE.md) | SQLite schema, traffic/intercept/config stores, session model |
| [WEB_FRONTEND.md](WEB_FRONTEND.md) | Frontend architecture, state management, WebSocket client, build/embed flow |
| [PERFORMANCE.md](PERFORMANCE.md) | Memory tracking, metrics collector, alerting, connection pool |
| [ENTERPRISE.md](ENTERPRISE.md) | Auth (JWT + API keys), RBAC, audit logging, user management |

### API Reference

| Document | Description |
|----------|-------------|
| [API.md](API.md) | API index and conventions (filtering, pagination, WebSocket) |
| [API_TRAFFIC.md](API_TRAFFIC.md) | Traffic, sessions, export, certificate endpoints |
| [API_WEBSOCKET_GRPC.md](API_WEBSOCKET_GRPC.md) | WebSocket events, WS traffic inspection, gRPC inspection |
| [API_INTERCEPT.md](API_INTERCEPT.md) | Breakpoints, mocks, rewrites, throttle, block list, focus, replay |
| [API_SCRIPTS_PLUGINS.md](API_SCRIPTS_PLUGINS.md) | Scripts and plugins endpoints |
| [API_CONFIG.md](API_CONFIG.md) | Config, capture, auto save, mirror, logs, persistence, health |
| [API_ENTERPRISE.md](API_ENTERPRISE.md) | Auth, users, RBAC, audit, metrics, onboarding (feature-gated) |

### Features

| Document | Description |
|----------|-------------|
| [SOCKS_PROXY.md](SOCKS_PROXY.md) | SOCKS5 proxy listener |
| [UPSTREAM_PROXY.md](UPSTREAM_PROXY.md) | Upstream proxy chaining (HTTP/HTTPS/SOCKS5) |
| [ACCESS_CONTROL.md](ACCESS_CONTROL.md) | CIDR-based IP allowlist |
| [BLOCK_LIST.md](BLOCK_LIST.md) | Domain/pattern blocking |
| [FOCUS.md](FOCUS.md) | Visual emphasis of matching hosts |
| [RECORDING_LIMITS.md](RECORDING_LIMITS.md) | Recording size limits and FIFO pruning |
| [AUTO_SAVE.md](AUTO_SAVE.md) | Periodic HAR/session backup with rotation |
| [MIRROR.md](MIRROR.md) | Save response bodies to disk mirroring URL paths |
| [LOGGING.md](LOGGING.md) | Rotating file logger |
| [TIMELINE_VIEW.md](TIMELINE_VIEW.md) | Waterfall chart in the web UI |
| [EDIT_THEN_REPEAT.md](EDIT_THEN_REPEAT.md) | Modify saved requests before replay |
| [REPEAT_ADVANCED.md](REPEAT_ADVANCED.md) | Batch replay (iterations/concurrency/delay) |
| [HAR_IMPORT.md](HAR_IMPORT.md) | Import HAR files as sessions |
| [REWRITE_TEMPLATES.md](REWRITE_TEMPLATES.md) | Built-in rewrite rules (No Caching, Add CORS, etc.) |
| [MOCK_RESPONSES.md](MOCK_RESPONSES.md) | Mock response feature |
| [HTTP2_SUPPORT.md](HTTP2_SUPPORT.md) | HTTP/2 and gRPC traffic inspection |
| [ZSTD_SUPPORT.md](ZSTD_SUPPORT.md) | zstd compression support |

### Scripting & Plugins

| Document | Description |
|----------|-------------|
| [SCRIPTING.md](SCRIPTING.md) | JavaScript scripting system overview (boa_engine) |
| [SCRIPTING_API.md](SCRIPTING_API.md) | JavaScript API reference |
| [SCRIPTING_SECURITY.md](SCRIPTING_SECURITY.md) | Scripting sandbox model |
| [PLUGINS.md](PLUGINS.md) | WASM plugin system overview (wasmtime) |
| [PLUGIN_API.md](PLUGIN_API.md) | Plugin guest SDK API |
| [PLUGIN_DEVELOPMENT.md](PLUGIN_DEVELOPMENT.md) | Plugin development guide |
| [PLUGIN_SECURITY.md](PLUGIN_SECURITY.md) | Plugin security model (signing, fuel) |

### Development & DevOps

| Document | Description |
|----------|-------------|
| [DEVELOPMENT.md](DEVELOPMENT.md) | Dev environment setup, project structure, coding standards, local scripts, troubleshooting |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Docker, Kubernetes, cloud, package managers |
| [BUILD_OPTIMIZATION.md](BUILD_OPTIMIZATION.md) | CI/CD build performance optimization |
| [NETWORK_CONFIGURATION.md](NETWORK_CONFIGURATION.md) | Network setup and IP detection (consolidates IP_DETECTION.md) |
| [SECURITY.md](SECURITY.md) | Security considerations and best practices |

### AI Agent Integration

| Document | Description |
|----------|-------------|
| [MCP-INTEGRATION.md](MCP-INTEGRATION.md) | MCP server setup, Claude Desktop/Windsurf config |
| [TOOL_COVERAGE.md](TOOL_COVERAGE.md) | Feature coverage matrix across Web UI, MCP, and CLI |

### Getting Started & Guides

| Document | Description |
|----------|-------------|
| [GETTING_STARTED.md](GETTING_STARTED.md) | Installation, CLI options, env vars, mobile setup |
| [ANDROID_CERT_PINNING.md](ANDROID_CERT_PINNING.md) | Bypassing cert pinning on Android (Frida, APK patching, Magisk) |
| [CERT_PINNING_PLAIN_ENGLISH.md](CERT_PINNING_PLAIN_ENGLISH.md) | Non-technical cert pinning explanation for QA/PMs |

### Reference

| Document | Description |
|----------|-------------|
| [CHARLES_PROXY_FEATURE_COMPARISON.md](CHARLES_PROXY_FEATURE_COMPARISON.md) | Feature parity with Charles Proxy |
| [WEBSOCKET_MIGRATION.md](WEBSOCKET_MIGRATION.md) | WebSocket implementation migration notes |
| [TEMPLATE.md](TEMPLATE.md) | Canonical structure for new docs |

### Consolidated (redirect stubs)

These docs have been merged into the indicated target. The stub files remain
for link compatibility.

| Stub | Redirects to |
|------|-------------|
| [LOCAL_DEVELOPMENT.md](LOCAL_DEVELOPMENT.md) | [DEVELOPMENT.md](DEVELOPMENT.md) (Local Development Scripts + Troubleshooting sections) |
| [IP_DETECTION.md](IP_DETECTION.md) | [NETWORK_CONFIGURATION.md](NETWORK_CONFIGURATION.md) (How IP Detection Works section) |

## Documentation Conventions

- **Naming**: `UPPER_SNAKE_CASE.md` for reference docs (developer-facing).
- **Audience**: `docs/` explains *how it works* (developers);
  `docs-site/` explains *how to use it* (end users). Content is not duplicated
  between the two.
- **Diagrams**: Prefer mermaid diagrams wherever a visual aids understanding
  (architecture, flows, pipelines).
- **Template**: See [TEMPLATE.md](TEMPLATE.md) for the canonical doc structure.
- **No emojis** in prose, headings, or code.

## External Links

- [GitHub Repository](https://github.com/ShristiLabs/madhyamas)
- [Issue Tracker](https://github.com/ShristiLabs/madhyamas/issues)
- [Discussions](https://github.com/ShristiLabs/madhyamas/discussions)
- [Releases](https://github.com/ShristiLabs/madhyamas/releases)

## License

Documentation is licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
Code is dual-licensed under MIT OR Apache-2.0.
