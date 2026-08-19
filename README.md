# Madhyamas

[![skills.sh](https://img.shields.io/badge/skills.sh-madhyamas-blue?logo=vercel&logoColor=white)](https://skills.sh/ShristiLabs/madhyamas)
[![npm](https://img.shields.io/badge/npm-%40madhyamas%2Fskill-blue?logo=npm&logoColor=white)](https://www.npmjs.com/package/@madhyamas/skill)
[![CI](https://github.com/ShristiLabs/madhyamas/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ShristiLabs/madhyamas/actions/workflows/ci.yml)

**Open Source HTTP/HTTPS Debugging Proxy**

A high-performance, cross-platform HTTP/HTTPS debugging proxy built in Rust with
a modern web-based UI. Madhyamas is the free, open-source alternative to tools
like Charles Proxy and Fiddler. It also offers an enterprise tier with
authentication, RBAC, audit logging, multi-instance clustering, and PostgreSQL
persistence for teams.

## Features

### Core Capabilities

- **HTTP/HTTPS Traffic Interception** — Capture and inspect all HTTP/HTTPS traffic in real-time
- **TLS/SSL Certificate Generation** — Automatic on-the-fly certificate generation for HTTPS interception
- **Traffic Inspection UI** — Modern React-based web UI for viewing and analyzing traffic
- **Request/Response Filtering** — Filter by URL, method, status code, host, content type, duration, headers, and cookies
- **Real-time WebSocket Updates** — Live traffic streaming via WebSocket (no polling required)
- **HTTP/2 Upstream Support** — Full HTTP/2 support for upstream connections with ALPN negotiation

### Traffic Inspection

- **Syntax-Highlighted JSON Viewer** — Prism.js-powered syntax highlighting for JSON bodies with Code and Tree views
- **JSON Prettify/Minify** — Toggle between prettified (2-space indent) and minified JSON output
- **JSONPath Queries** — Filter and extract JSON data using JSONPath expressions (e.g., `$.store.book[*].title`)
- **JMESPath Queries** — Query JSON data using JMESPath expressions (e.g., `store.book[*].title`)
- **Image Preview** — Automatic image rendering for image responses (PNG, JPEG, GIF, WebP, SVG, ICO, BMP, AVIF, TIFF) with download support
- **Compression Toggle** — Decompress gzip/deflate/brotli response bodies on demand with a toggle button
- **Base64 Body Decoding** — Automatic decoding of base64-prefixed binary bodies
- **Request/Response Size Tracking** — Accurate size computation (headers + body) displayed in traffic list and detail views
- **Body Search** — Full-text search within request and response bodies
- **Copy as cURL/HTTPie/fetch/wget** — Export any request as a command-line command

### Traffic Manipulation

- **Breakpoints** — Pause requests/responses for inspection and modification before forwarding
- **Response Mocking** — Serve custom responses instead of hitting real servers; supports collections, recording, import/export
- **URL/Header Rewriting** — Automatically modify traffic based on rules
- **Bandwidth Throttling** — Simulate slow network conditions (3G, 4G, DSL presets)
- **Request Replay** — Re-execute captured requests with modifications

### SSL/TLS Error Visibility

- **Failed TLS Handshake Recording** — CONNECT requests that fail TLS handshake (e.g., Android apps with certificate pinning) are recorded as 502 traffic entries so they're visible in the UI, with a clear error message explaining the cause

### Advanced Features

- **WebSocket Traffic Capture** — Inspect WebSocket messages in real-time
- **gRPC Support** _(Experimental)_ — Debug gRPC/Protocol Buffer traffic with frame parsing
- **JavaScript/TypeScript Scripting** _(Experimental)_ — Automate traffic manipulation with scripts
- **Plugin System** _(Experimental)_ — Extend functionality with custom Rust plugins
- **MCP Server** — AI agent integration via Model Context Protocol

> **Note:** Features marked Experimental have partial implementations and may be incomplete.

### Session Management

- **Session Save/Load** — Persist and restore debugging sessions
- **HAR Export** — Export traffic in HAR format for sharing
- **Session Import** — Import previously exported sessions
- **cURL Export** — Generate cURL commands for any request
- **Persistence Layer** — SQLite-based traffic storage with configurable retention

### Security & Configuration

- **Configurable Rate Limiting** — Opt-in API rate limiting (disabled by default) with configurable requests/sec and burst size
- **CORS Protection** — Safe-origin CORS policy for the web UI
- **Security Headers** — X-Frame-Options, X-Content-Type-Options, Referrer-Policy headers
- **Request Body Size Limit** — 10MB limit to prevent OOM from large payloads

### Enterprise Features

The enterprise tier (feature-gated behind the `enterprise` Cargo feature,
licensed under BSL-1.1) adds team-oriented capabilities:

- **Authentication** — JWT (HMAC-SHA256) and API key authentication with
  session management, token refresh, and configurable token expiry
- **Role-Based Access Control** — Four-role model (Admin, User, Viewer,
  ReadOnly) with fine-grained permissions per resource type
- **Audit Logging** — Tamper-evident audit trail with SHA-256 hash chaining,
  exportable for compliance (SOC 2, ISO 27001)
- **User Management** — Admin dashboard for user CRUD, role assignment,
  password reset, and API key lifecycle
- **Multi-Instance Clustering** — Horizontal scaling with PostgreSQL shared
  state, Redis pub/sub for real-time event sync, atomic config propagation,
  and shared CA certificates across instances
- **PostgreSQL Persistence** — Production-grade storage backend with
  advisory locks for concurrent pruning, tiered body storage, and
  connection pooling via PgBouncer
- **License Management** — Ed25519-signed license files with seat tracking,
  instance registration, and heartbeat-based liveness
- **Performance Monitoring** — Cluster-wide metrics, per-instance health
  checks, memory tracking, and alerting
- **Onboarding Wizard** — Guided first-run setup for new enterprise
  deployments

See the [Enterprise Documentation](https://shristilabs.github.io/madhyamas/enterprise/)
for setup guides, configuration reference, and deployment instructions.

## Comparison with Other Tools

| Feature            | Madhyamas | Charles Proxy | mitmproxy   | Fiddler   | Proxyman   |
| ------------------ | --------- | ------------- | ----------- | --------- | ---------- |
| **Open Source**    | Yes       | No            | Yes         | No        | No         |
| **Free**           | Yes       | No ($50)      | Yes         | Yes       | Freemium   |
| **Cross-Platform** | Yes       | Yes           | Yes         | Windows   | macOS      |
| **Web UI**         | Yes       | No            | Limited     | No        | No         |
| **Rust-Powered**   | Yes       | No (Java)     | No (Python) | No (.NET) | No (Swift) |
| **gRPC Support**   | Yes (Experimental) | No            | Yes         | No        | No         |
| **WebSocket**      | Yes       | Limited       | Yes         | Yes       | Yes        |
| **Scripting**      | Yes JS/TS (Experimental) | No            | Yes Python  | No        | No         |
| **Plugin System**  | Yes (Experimental) | No            | Yes         | Yes       | No         |
| **JSON Query**     | Yes JSONPath + JMESPath | No            | No          | No        | No         |
| **Image Preview**  | Yes       | Yes           | No          | Yes       | Yes        |
| **MCP/AI Agent**   | Yes       | No            | No          | No        | No         |
| **Auth/RBAC**      | Yes (Enterprise) | No            | No          | No        | No         |
| **Audit Logging**  | Yes (Enterprise) | No            | No          | No        | No         |
| **Multi-Instance** | Yes (Enterprise) | No            | No          | No        | No         |
| **PostgreSQL**     | Yes (Enterprise) | No            | No          | No        | No         |

## Quick Start

### OSS (Free, Open Source)

```bash
# Build and run
cargo build --release
./target/release/madhyamas

# Or run directly
cargo run --release
```

### Enterprise

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

Then configure your browser to use `localhost:8888` as the proxy and open
`http://localhost:3001` for the web UI.

See the [Getting Started Guide](docs/GETTING_STARTED.md) for detailed
installation, configuration, and mobile device setup instructions. For
enterprise deployment, see the
[Enterprise Getting Started Guide](https://shristilabs.github.io/madhyamas/enterprise/getting-started).

## Documentation

### End-User Documentation

Hosted on GitHub Pages: [shristilabs.github.io/madhyamas](https://shristilabs.github.io/madhyamas)

| Section | Description |
|---------|-------------|
| [Getting Started](docs/GETTING_STARTED.md) | Installation, configuration, CLI options, mobile setup, basic usage |
| [Enterprise Guide](https://shristilabs.github.io/madhyamas/enterprise/) | Auth, RBAC, audit logging, multi-instance deployment, licensing |
| [API Reference](docs/API.md) | REST API endpoints (148 paths), query parameters, WebSocket events |
| [MCP Integration](docs/MCP-INTEGRATION.md) | MCP server setup, Claude Desktop/Windsurf config, AI agent skills |
| [Deployment](docs/DEPLOYMENT.md) | Docker, production deployment |
| [Android Cert Pinning](docs/ANDROID_CERT_PINNING.md) | Bypassing certificate pinning on Android |

### Developer Documentation

| Document | Description |
|----------|-------------|
| [Development Guide](docs/DEVELOPMENT.md) | Dev environment setup, project structure, tech stack, git hooks |
| [Architecture](docs/ARCHITECTURE.md) | System architecture overview |
| [Enterprise Crate Guide](docs/ENTERPRISE_CRATE_GUIDE.md) | Enterprise crate structure, public API, key types, extension points |
| [Enterprise API Integration](docs/ENTERPRISE_API_INTEGRATION.md) | Trait abstractions, AppState injection, router merging, middleware |
| [Enterprise Startup Flow](docs/ENTERPRISE_STARTUP_FLOW.md) | 17-step initialization sequence, CLI flags, graceful shutdown |
| [Storage Backend Guide](docs/STORAGE_BACKEND_GUIDE.md) | Storage traits, schema design, implementation checklist for new backends |
| [Enterprise Testing](docs/ENTERPRISE_TESTING.md) | Unit, integration, multi-instance, and E2E testing guide |
| [Skills Package](skills/README.md) | AI agent skills package (146 MCP tools, 141 CLI commands, 148 REST endpoints) |

## License

- **OSS (core)**: Dual-licensed under MIT OR Apache-2.0
- **Enterprise crate** (`madhyamas-enterprise`): BSL-1.1 (Business Source License)

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for the OSS
license, and [crates/madhyamas-enterprise/LICENSE-BSL](crates/madhyamas-enterprise/LICENSE-BSL)
for the enterprise license.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## Support

- **Issues**: [GitHub Issues](https://github.com/ShristiLabs/madhyamas/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ShristiLabs/madhyamas/discussions)
