# Madhyamas

[![CI](https://github.com/ShristiLabs/madhyamas/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ShristiLabs/madhyamas/actions/workflows/ci.yml)
[![skills.sh](https://img.shields.io/badge/skills.sh-madhyamas-blue?logo=vercel&logoColor=white)](https://skills.sh/ShristiLabs/madhyamas)
[![npm](https://img.shields.io/badge/npm-%40madhyamas%2Fskill-blue?logo=npm&logoColor=white)](https://www.npmjs.com/package/@madhyamas/skill)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

A high-performance HTTP/HTTPS debugging proxy built in Rust with a modern
web UI. Capture, inspect, and manipulate traffic in real time — the open-source
alternative to Charles Proxy and Fiddler.

## Why Madhyamas

- **Fast** — Written in Rust. No JVM, no Electron, no Python runtime.
- **Cross-platform** — Single binary for Linux, macOS, and Windows.
- **Web UI** — Built-in React dashboard. No desktop app to install.
- **AI-ready** — Built-in MCP server for Claude, Windsurf, and other AI agents.
- **Two tiers** — Free OSS for individuals; Enterprise for teams and production.

## Quick Start

```bash
cargo build --release
./target/release/madhyamas
```

Point your browser or device proxy at `localhost:8888`, then open
`http://localhost:3001` for the dashboard.

For mobile devices, certificate installation, and advanced configuration, see
the [Getting Started Guide](https://shristilabs.github.io/madhyamas/getting-started/).

## Features

### Traffic Capture & Inspection

- **HTTP/HTTPS Interception** — Real-time capture with automatic TLS
  certificate generation
- **HTTP/2 & gRPC** — Upstream HTTP/2 with ALPN negotiation; gRPC frame
  parsing (experimental)
- **WebSocket Inspection** — Live WebSocket message capture and display
- **Waterfall Timeline** — Visual request timing chart with virtualized rows
- **JSON Viewer** — Syntax-highlighted Code and Tree views with JSONPath and
  JMESPath queries
- **Image Preview** — Inline rendering for PNG, JPEG, GIF, WebP, SVG, and more
- **Body Search** — Full-text search across request and response bodies
- **Compression Toggle** — On-demand gzip/deflate/brotli decompression
- **Copy as cURL** — Export any request as a cURL, HTTPie, fetch, or wget
  command
- **Size Tracking** — Accurate header + body sizes in list and detail views

### Traffic Manipulation

- **Breakpoints** — Pause requests or responses for inspection and editing
  before forwarding
- **Mock Responses** — Serve custom responses with single, sequence,
  conditional, and probabilistic mocks; collections, recording, import/export
- **Rewrite Rules** — Modify URLs, headers, and bodies with built-in templates
  (No Caching, Add CORS, etc.) or custom rules
- **Bandwidth Throttling** — Simulate 3G, 4G, DSL, or custom latency profiles
- **Request Replay** — Re-execute captured requests with modifications;
  batch replay with iterations, concurrency, and delay
- **Block List** — Domain and pattern blocking at the intercept layer
- **Focus Mode** — Visually highlight matching hosts without filtering

### Proxy Modes

- **HTTP/HTTPS Proxy** — Standard forward proxy on port 8888 with MITM
- **SOCKS5 Proxy** — Blind TCP tunneling on port 1080
- **Upstream Chaining** — Route outbound traffic through HTTP/HTTPS/SOCKS5
  upstream proxies with bypass lists

### Sessions & Export

- **Session Management** — Save and restore debugging sessions
- **HAR Export/Import** — Industry-standard HTTP Archive format
- **cURL Export** — Generate cURL commands for any captured request
- **Auto Save** — Periodic HAR/session backup with optional rotation
- **Mirror Tool** — Save response bodies to disk mirroring URL path structure
- **Recording Limits** — Configurable max requests, total size, and body size
  with FIFO pruning

### Scripting & Plugins

- **JavaScript Scripting** — Automate traffic manipulation with JS (ES6+)
  via boa_engine; 13 built-in templates; SQLite-persisted scripts
- **WASM Plugins** — Extend functionality with sandboxed WASM plugins
  (wasmtime); fuel-metered, Ed25519-signed, hot-reloadable

### AI Agent Integration

- **MCP Server** — Built-in Model Context Protocol server (stdio transport)
  for Claude Desktop, Windsurf, and other AI-powered tools
- **146 MCP Tools** — Full programmatic access to traffic, mocks, rewrites,
  breakpoints, sessions, and configuration
- **CLI** — 141 CLI subcommands for scripting and automation
- **REST API** — 148 REST endpoints for integration with external tools

### Security

- **IP Allowlist** — CIDR-based access control; loopback always allowed
- **Rate Limiting** — Opt-in API rate limiting with configurable RPS and burst
- **Security Headers** — X-Frame-Options, X-Content-Type-Options,
  Referrer-Policy
- **CORS Protection** — Safe-origin policy for the web UI
- **TLS Error Visibility** — Failed TLS handshakes (e.g., cert pinning) are
  recorded as 502 entries with diagnostic messages

### Enterprise

The enterprise tier is feature-gated behind the `enterprise` Cargo feature and
licensed under BSL-1.1. It adds team and production capabilities:

- **Authentication** — JWT (HMAC-SHA256) and API key auth with session
  management and token refresh
- **Role-Based Access Control** — Four roles (Admin, User, Viewer, ReadOnly)
  with per-resource permissions
- **Audit Logging** — Tamper-evident SHA-256 hash-chained audit trail,
  exportable for compliance
- **User Management** — Admin dashboard for user CRUD, role assignment, and
  API key lifecycle
- **Multi-Instance Clustering** — Horizontal scaling with PostgreSQL shared
  state, Redis pub/sub for real-time sync, and shared CA certificates
- **PostgreSQL Persistence** — Production-grade storage with advisory locks,
  tiered body storage, and PgBouncer connection pooling
- **License Management** — Ed25519-signed license files with seat tracking
  and instance heartbeats
- **Performance Monitoring** — Cluster-wide metrics, per-instance health, and
  alerting
- **Onboarding Wizard** — Guided first-run setup

See the [Enterprise Documentation](https://shristilabs.github.io/madhyamas/enterprise/)
for setup guides, configuration, and deployment instructions.

## Comparison

| Feature | Madhyamas | Charles | mitmproxy | Fiddler | Proxyman |
|---------|-----------|---------|-----------|---------|----------|
| Open Source | Yes | No | Yes | No | No |
| Free | Yes | No ($50) | Yes | Yes | Freemium |
| Cross-Platform | Yes | Yes | Yes | Windows | macOS |
| Web UI | Yes | No | Limited | No | No |
| Rust | Yes | No (Java) | No (Python) | No (.NET) | No (Swift) |
| gRPC | Yes (exp.) | No | Yes | No | No |
| WebSocket | Yes | Limited | Yes | Yes | Yes |
| Scripting | JS/TS (exp.) | No | Python | No | No |
| Plugins | WASM (exp.) | No | Yes | Yes | No |
| JSON Query | JSONPath + JMESPath | No | No | No | No |
| MCP / AI Agent | Yes | No | No | No | No |
| Auth / RBAC | Enterprise | No | No | No | No |
| Audit Logging | Enterprise | No | No | No | No |
| Multi-Instance | Enterprise | No | No | No | No |
| PostgreSQL | Enterprise | No | No | No | No |

## Documentation

Full documentation is hosted at
[shristilabs.github.io/madhyamas](https://shristilabs.github.io/madhyamas).

| Section | Description |
|---------|-------------|
| [Getting Started](https://shristilabs.github.io/madhyamas/getting-started/) | Installation, configuration, mobile setup |
| [Traffic Inspection](https://shristilabs.github.io/madhyamas/traffic-inspection/) | Viewing, filtering, exporting traffic |
| [Breakpoints](https://shristilabs.github.io/madhyamas/breakpoints/) | Pausing and modifying requests |
| [Mocks](https://shristilabs.github.io/madhyamas/mocks/) | Creating mock API responses |
| [Rewrites](https://shristilabs.github.io/madhyamas/rewrites/) | Modifying traffic on the fly |
| [Replay](https://shristilabs.github.io/madhyamas/replay/) | Re-executing captured requests |
| [Scripting](https://shristilabs.github.io/madhyamas/scripting/) | JavaScript automation |
| [Plugins](https://shristilabs.github.io/madhyamas/plugins/) | WASM plugin development |
| [Enterprise](https://shristilabs.github.io/madhyamas/enterprise/) | Auth, RBAC, audit, multi-instance, licensing |
| [REST API](https://shristilabs.github.io/madhyamas/rest-api/) | 148 endpoints reference |
| [MCP Integration](https://shristilabs.github.io/madhyamas/mcp/) | AI agent setup and tool reference |
| [CLI](https://shristilabs.github.io/madhyamas/cli/) | 141 CLI subcommands |
| [Migration from Charles](https://shristilabs.github.io/madhyamas/migration-from-charles/) | Switching from Charles Proxy |

## Enterprise Quick Start

```bash
# Build with enterprise features
cargo build --release --features enterprise

# Run with PostgreSQL and Redis
./target/release/madhyamas \
  --enterprise \
  --database-url postgres://user:pass@localhost/madhyamas \
  --redis-url redis://localhost:6379 \
  --jwt-secret your-secret-key
```

See the [Enterprise Getting Started Guide](https://shristilabs.github.io/madhyamas/enterprise/getting-started/)
for Docker Compose setup, license installation, and admin configuration.

## License

| Component | License |
|-----------|---------|
| OSS core (all crates except enterprise) | MIT OR Apache-2.0 |
| Enterprise crate (`madhyamas-enterprise`) | BSL-1.1 |

See [LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE), and
[crates/madhyamas-enterprise/LICENSE-BSL](crates/madhyamas-enterprise/LICENSE-BSL).

## Contributing

Contributions are welcome! Please read our contributing guidelines before
submitting PRs.

## Support

- **Issues**: [GitHub Issues](https://github.com/ShristiLabs/madhyamas/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ShristiLabs/madhyamas/discussions)
