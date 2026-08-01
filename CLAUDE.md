# Madhyamas - Claude AI Context

## Project Overview

HTTP/HTTPS debugging proxy in Rust with React web UI. Alternative to Charles Proxy/Fiddler.

- **Language**: Rust (backend), TypeScript/React (frontend)
- **Architecture**: Workspace with 5 crates (core, api, cli, mcp, main binary)
- **Binary**: Single unified `madhyamas` binary (proxy + web UI + MCP + CLI)
- **Web UI**: Embedded at compile time via `rust-embed` — no external files needed
- **License**: Dual MIT OR Apache-2.0

## Project Structure

```
madhyamas/
├── crates/
│   ├── madhyamas/          # Unified binary (subcommands: serve/mcp/cli)
│   ├── madhyamas-core/     # Core proxy engine, TLS, traffic storage
│   ├── madhyamas-api/      # REST/WebSocket API + embedded web assets (axum)
│   ├── madhyamas-cli/      # CLI library (re-exported by main binary)
│   └── madhyamas-mcp/      # MCP server library (re-exported by main binary)
├── web/                    # React + TypeScript frontend (Vite)
├── docs/                   # Documentation
├── docker/                 # Docker setup
└── Cargo.toml              # Workspace configuration
```

## Unified Binary Usage

```bash
# Start proxy server with web UI (default)
madhyamas
# or: madhyamas serve

# Run as MCP server (stdio transport)
madhyamas mcp

# CLI commands
madhyamas traffic list
madhyamas mocks list
madhyamas breakpoints list
madhyamas sessions list
madhyamas throttle get
madhyamas rewrites list
madhyamas grpc status
madhyamas scripts list
madhyamas plugins list
madhyamas export har --output traffic.har
madhyamas --help  # See all commands
```

## Core Technologies

**Backend**: axum, hyper, tokio, rustls, rcgen, rusqlite, serde, clap, tracing, rust-embed
**Frontend**: React 18, TypeScript, Vite, Tailwind CSS, shadcn/ui, TanStack Query, Zustand

## Important Files & Modules

### Main Binary (`madhyamas`)
- `main.rs` - Unified entry point with subcommands (serve/mcp/cli)

### Core Crate (`madhyamas-core`)
- `lib.rs` - Public API exports, error types
- `proxy/engine.rs` - Main proxy engine logic
- `tls/certificate.rs` - TLS certificate management
- `traffic/store.rs` - SQLite-based traffic storage
- `intercept/` - Breakpoints, mocks, rewrites, throttling

### API Crate (`madhyamas-api`)
- `lib.rs` - API server setup
- `embedded_assets.rs` - rust-embed web UI serving (compiled into binary)
- `routes.rs` - Route definitions
- `handlers.rs` - Core request handlers (traffic, sessions, export, cert, ws, config, capture)
- `intercept_handlers.rs` - Interception handlers (breakpoints, mocks, rewrites, throttle, replay)
- `phase3_handlers.rs` - Phase 3 handlers (gRPC, scripts, plugins)
- `phase4_handlers.rs` - Phase 4 enterprise handlers (auth, users, RBAC, audit, onboarding)
- `ws.rs` - WebSocket connection handler
- `middleware.rs` - Auth middleware
- `error.rs` - API error types
- `validation.rs` - Input validation

### CLI Crate (`madhyamas-cli`)
- `lib.rs` - Exports `Commands` enum and `ApiClient`
- `commands/` - CLI subcommands (traffic, mocks, breakpoints, etc.)

### MCP Crate (`madhyamas-mcp`)
- `lib.rs` - Exports `McpServer` and `McpConfig`
- `server.rs` - MCP server (stdio transport)
- `tools/` - MCP tools for AI agent integration

## Development Workflow

```bash
# Docker (recommended for deployment)
./startup.sh           # Build and start with Docker Compose
./stop.sh              # Stop containers

# Local development (runs directly on host)
./startup-local.sh     # Build and run locally
./startup-local.sh --clean  # Clean rebuild
./stop-local.sh        # Stop local instance

# Manual commands
cargo build --release -p madhyamas   # Build unified binary
RUST_LOG=debug cargo run --bin madhyamas
cargo test
cargo fmt --all && cargo clippy --all-targets --all-features

# Frontend (must build before Rust — assets are embedded at compile time)
cd web && npm run build
```

## Configuration

**CLI Flags**: `--proxy-port`, `--api-port`, `--host`, `--public-ip`, `--verbose`, `--no-https`, `--enable-socks`, `--socks-port`, `--socks-username`, `--socks-password`, `--upstream-proxy-enabled`, `--upstream-proxy`, `--upstream-protocol`, `--upstream-auth`, `--upstream-no-proxy`

**Environment Variables**:
- `RUST_LOG` - Logging level (trace/debug/info/warn/error)
- `MADHYAMAS_HOST` - Bind host (default: 127.0.0.1)
- `MADHYAMAS_API_PORT` - API port (default: 3001)
- `MADHYAMAS_PROXY_PORT` - Proxy port (default: 8888)
- `MADHYAMAS_PUBLIC_IP` - Public IP shown to users for remote access
- `MADHYAMAS_API_URL` - API URL for CLI/MCP modes (default: http://127.0.0.1:3001)
- `MADHYAMAS_WEB_DIR` - Override web asset directory (dev only; defaults to embedded)
- `MADHYAMAS_ENABLE_SOCKS` - Enable the SOCKS5 listener (default: false)
- `MADHYAMAS_SOCKS_PORT` - SOCKS5 listener port (default: 1080)
- `MADHYAMAS_SOCKS_USERNAME` - SOCKS5 username/password auth username (optional)
- `MADHYAMAS_SOCKS_PASSWORD` - SOCKS5 username/password auth password (optional)
- `MADHYAMAS_UPSTREAM_PROXY_ENABLED` - Enable upstream proxy chaining (default: false)
- `MADHYAMAS_UPSTREAM_PROXY` - Upstream proxy host:port (e.g. `corp-proxy:8080`)
- `MADHYAMAS_UPSTREAM_PROTOCOL` - Upstream proxy protocol: http/https/socks5 (default: http)
- `MADHYAMAS_UPSTREAM_AUTH` - Upstream proxy auth as `username:password`
- `MADHYAMAS_UPSTREAM_NO_PROXY` - Comma-separated bypass list (e.g. `localhost,127.0.0.0/8`)

**SOCKS5 Proxy**: When `--enable-socks` is set, Madhyamas also listens on the
SOCKS5 port (default `1080`) as a blind TCP tunnel (RFC 1928/1929). SOCKS
connections are recorded as passthrough traffic entries (`http_version:
SOCKS5`). HTTPS cannot be MITM-intercepted via SOCKS — use the HTTP proxy port
with `CONNECT` for that. See [docs/SOCKS_PROXY.md](docs/SOCKS_PROXY.md).

**Upstream Proxy Chaining**: When `--upstream-proxy-enabled` is set, all
outbound traffic is routed through the configured upstream proxy (HTTP
CONNECT, HTTPS, or SOCKS5). A bypass list (`--upstream-no-proxy`) excludes
specified hosts/CIDRs from the upstream proxy. See
[docs/UPSTREAM_PROXY.md](docs/UPSTREAM_PROXY.md).

**Data Directory**: `~/.madhyamas/` (certs, logs, traffic.db)

**API Endpoints** (all under `/api` prefix):

| Category | Endpoints |
|----------|-----------|
| Traffic | `GET /traffic`, `GET /traffic/{id}`, `POST /traffic/clear`, `GET /traffic/count` |
| Sessions | `GET /sessions`, `POST /sessions`, `GET/DELETE /sessions/{id}`, `GET /sessions/{id}/export`, `POST /sessions/{id}/switch`, `POST /sessions/import` |
| Export | `GET /export/har`, `GET /export/curl/{id}` |
| Certificate | `GET /cert/ca` |
| WebSocket | `GET /ws` (real-time traffic updates) |
| Config | `GET /config`, `PATCH /config` |
| Capture | `GET /capture`, `POST /capture/toggle` |
| Breakpoints | `GET/POST /breakpoints`, `GET/DELETE /breakpoints/{id}`, `GET /breakpoints/paused`, `POST /breakpoints/paused/{id}/resume` |
| Mocks | `GET/POST /mocks`, `GET/PUT/DELETE /mocks/{id}`, `POST /mocks/{id}/toggle`, collections, recording, import/export |
| Rewrites | `GET/POST /rewrites`, `GET/DELETE /rewrites/{id}`, `POST /rewrites/{id}/toggle` |
| Throttle | `GET/POST /throttle`, `POST /throttle/enabled`, `GET /throttle/presets` |
| Replay | `GET/POST /replay/saved`, `POST /replay/execute/{id}`, `GET /replay/history` |
| gRPC | `GET /grpc/connections`, `GET /grpc/streams`, `GET /grpc/frames`, `GET /grpc/stats` |
| Scripts | `GET/POST /scripts`, `GET/PUT/DELETE /scripts/{id}`, `POST /scripts/{id}/toggle` |
| Plugins | `GET /plugins`, `POST /plugins/{id}/enable`, `POST /plugins/{id}/disable`, `POST /plugins/reload` |
| Health | `GET /health` |

> **Phase 4 (Enterprise, conditionally enabled):** `/metrics`, `/auth/*`, `/users`, `/rbac/*`, `/audit/*`, `/onboarding/*`

## AI Assistant Guidelines

### Rust Best Practices
1. Use `thiserror` for custom errors, `Result<T>` throughout
2. Avoid `unwrap()` in production - use `?` operator
3. All I/O operations async with Tokio
4. Strong typing with minimal panics
5. Add tests for new functionality

### Adding Features
1. Add module in `madhyamas-core/src/`
2. Implement with proper error handling
3. Add tests in the module
4. Expose public API in `lib.rs`
5. Add API endpoints in `madhyamas-api`
6. Update documentation

### Adding Dependencies
```toml
# In workspace Cargo.toml [workspace.dependencies]
new-dep = { version = "1.0", features = ["feature"] }

# In crate Cargo.toml
[dependencies]
new-dep.workspace = true
```

### Debugging
- `RUST_LOG=debug cargo run --bin madhyamas -- --verbose`
- `cargo test -- --nocapture` for test output
- Check database schema matches code expectations

### Common Issues
- **Certificate Errors**: Install CA cert in system trust store
- **Port Conflicts**: Check ports 8888/3001 available
- **Database Locked**: Only one instance at a time
- **Web UI not updating**: Rebuild frontend (`cd web && npm run build`) then rebuild Rust

## Resources

- [README.md](README.md) - Quick start
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [docs/API.md](docs/API.md) - API reference
