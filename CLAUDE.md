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
madhyamas traffic import-har /path/to/traffic.har --name "Imported" --switch
madhyamas replay run-advanced <id> --iterations 100 --concurrency 10 --delay-ms 50
madhyamas focus list
madhyamas focus add "*.example.com"
madhyamas mirror status
madhyamas mirror start
madhyamas mirror config --output-dir /tmp/mirror --host-filter "*.example.com"
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
- `access_control.rs` - IP allowlist (CIDR-based access control)
- `proxy/engine.rs` - Main proxy engine logic
- `tls/certificate.rs` - TLS certificate management
- `traffic/store.rs` - SQLite-based traffic storage
- `intercept/` - Breakpoints, mocks, rewrites, throttling, block list
  - `rewrite.rs` - Rewrite rules + `RewriteTemplates` (No Caching, Block Cookies, Add CORS, HTTP→HTTPS, Add Auth, Remove Security Headers). See [docs/REWRITE_TEMPLATES.md](docs/REWRITE_TEMPLATES.md)
- `scripting/` - JavaScript scripting system (boa_engine runtime)
  - `engine.rs` - `JsEngine`: boa_engine integration, global registration (console, base64, crypto, url), result parsing, modified object read-back
  - `runtime.rs` - `ScriptRuntime`: script manager, execution, history, persistence, `ScriptTemplates` (Log Requests, Add CORS, Block Domains, Modify Headers, Mock API, Inject Latency, Rewrite URL, Inject Auth Token, Modify JSON Response, Override Status Code, Cache Buster, Strip Response Headers, Conditional Mock)
  - `hooks.rs` - `ScriptHook` enum, `ScriptContext`, `RequestContext`, `ResponseContext`, `ScriptResult`
  - `persistence.rs` - `ScriptPersistence`: SQLite storage for scripts and execution history
  - `api.rs` - `ScriptApi` documentation, `URLComponents` helper
  - See [docs/SCRIPTING.md](docs/SCRIPTING.md), [docs/SCRIPTING_API.md](docs/SCRIPTING_API.md), [docs/SCRIPTING_SECURITY.md](docs/SCRIPTING_SECURITY.md)

### API Crate (`madhyamas-api`)
- `lib.rs` - API server setup
- `embedded_assets.rs` - rust-embed web UI serving (compiled into binary)
- `routes.rs` - Route definitions
- `handlers.rs` - Core request handlers (traffic, sessions, export, cert, ws, config, capture)
- `intercept_handlers.rs` - Interception handlers (breakpoints, mocks, rewrites, throttle, replay)
- `tools_handlers.rs` - Tools handlers (gRPC, scripts, plugins)
- `enterprise_handlers.rs` - Enterprise handlers (auth, users, RBAC, audit, onboarding)
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

**CLI Flags**: `--proxy-port`, `--api-port`, `--host`, `--public-ip`, `--verbose`, `--no-https`, `--enable-socks`, `--socks-port`, `--socks-username`, `--socks-password`, `--upstream-proxy-enabled`, `--upstream-proxy`, `--upstream-protocol`, `--upstream-auth`, `--upstream-no-proxy`, `--allowed-ip` (repeatable)

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
- `MADHYAMAS_ALLOWED_IPS` - Comma-separated IP/CIDR allowlist (e.g. `192.168.1.0/24,10.0.0.5`)

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

**Access Control (IP Allowlist)**: When `--allowed-ip` is provided (or
`allowed_ips` is set via the API/config file), only connections from the
listed IP addresses or CIDR ranges are accepted. Loopback (`127.0.0.1`,
`::1`) is always allowed. An empty list (the default) allows all
connections. API updates via `PATCH /api/config` take effect immediately
for new connections. See [docs/ACCESS_CONTROL.md](docs/ACCESS_CONTROL.md).

**Block List**: Domain/pattern-based request blocking. When a block list
entry's pattern matches a request's host, the proxy returns a configurable
response (default `403 Forbidden`) instead of forwarding upstream. Runs at
priority 5 in the intercept pipeline (before rewrites, mocks, breakpoints,
throttle). Supports exact domains, wildcard subdomains (`*.example.com`),
and glob patterns (`*ads*`). Managed via `GET/POST/PUT/DELETE /api/blocklist`.
Entries persist to SQLite and survive restarts. See
[docs/BLOCK_LIST.md](docs/BLOCK_LIST.md).

**Focus**: Highlight traffic from specific hosts in the traffic view. Unlike
a filter (which hides non-matching traffic), focus visually emphasizes
matching rows (yellow left border, star icon, bold domain) while dimming the
rest. A "Show only focused" toggle switches to focus-only mode (hides
non-matching traffic). Patterns support exact hostnames, wildcard subdomains
(`*.example.com`), and globs (`*api*`). Right-click a traffic row for a
quick "Focus this host" action. Managed via `GET/POST/DELETE /api/focus`,
the `madhyamas focus` CLI subcommand, and `madhyamas_focus_*` MCP tools.
Focus hosts persist to SQLite (`focus_hosts` table) and survive restarts.
See [docs/FOCUS.md](docs/FOCUS.md).

**Recording Size Limits**: Traffic recording is bounded by configurable
limits to prevent unbounded memory/disk usage. `max_requests` (default
10,000) caps the number of stored entries; `max_total_size_mb` (default
unlimited) caps the total body size; `max_body_size` (default 20 MB)
truncates individual bodies. When limits are exceeded, oldest entries are
pruned automatically (FIFO). Granular body capture toggles
(`capture_request_bodies`, `capture_response_bodies`) and an ignore list
(`ignored_domains`) reduce storage further. The web UI header shows a
quota indicator (entry count vs limit). Configured via Config → Capture
tab, `PATCH /api/config`, or config file. Stats via
`GET /api/capture/stats`. See
[docs/RECORDING_LIMITS.md](docs/RECORDING_LIMITS.md).

**Auto Save**: Periodic session backup and rotation for disaster recovery.
Traffic is stored in SQLite in real time, so Auto Save is a backup
mechanism — not the primary store. When enabled, a background task
(`tokio::time::interval`) periodically exports the current session as HAR
or Session format to a backup directory. Old backups are pruned
automatically (keep last N). Optional session rotation starts a new session
after N requests or M minutes. Config fields: `auto_save.enabled`,
`auto_save.interval_seconds` (default 300), `auto_save.export_format`
(`"har"` or `"session"`), `auto_save.output_dir` (default
`~/.madhyamas/backups`), `auto_save.max_backups` (default 10),
`auto_save.rotate_after_requests`, `auto_save.rotate_after_minutes`.
Configured via Config → Auto Save tab, `GET/PATCH /api/autosave`,
`POST /api/autosave/snapshot` (manual snapshot), or the `madhyamas autosave`
CLI subcommand. See [docs/AUTO_SAVE.md](docs/AUTO_SAVE.md).

**Mirror Tool**: Save response bodies to disk following the URL path
structure (`output_dir/host/path/content`). A `.meta.json` sidecar is
written alongside each file with request/response metadata. When enabled,
captured responses are written to disk asynchronously (via `tokio::spawn`)
after being stored in the database. Passthrough traffic is skipped (no
captured body). Config fields: `mirror.enabled` (default `false`),
`mirror.output_dir` (default `~/.madhyamas/mirror`),
`mirror.host_filter` (optional list of host patterns; `null` = all hosts),
`mirror.save_request_bodies` (default `false`). Host filter patterns
support exact hostnames, wildcard subdomains (`*.example.com`), and globs
(`*api*`). Configured via the Mirror panel in the tools sidebar,
`GET /api/mirror`, `POST /api/mirror/toggle`, `PATCH /api/mirror/config`,
or the `madhyamas mirror` CLI subcommand. See
[docs/MIRROR.md](docs/MIRROR.md).

**Timeline View (Waterfall)**: The web UI traffic panel has a toggle to switch
between the list (table) view and a waterfall timeline view. The timeline shows
each request as a horizontal bar positioned by start time and sized by duration,
color-coded by status code (2xx green, 3xx blue, 4xx orange, 5xx red, pending
gray). Hover shows a tooltip with method/host/path/status/duration; click
selects the entry and opens the detail panel. Rows are virtualized with
`@tanstack/react-virtual` for large datasets. The detail panel's Timing tab
also includes a mini waterfall bar visualizing the request's duration. See
[docs/TIMELINE_VIEW.md](docs/TIMELINE_VIEW.md).

**Edit-then-Repeat**: Saved requests can be modified before replaying via
the web UI's "Edit & Replay" button (opens a `RequestEditor` dialog that
diffs changes against the original and sends only modified fields as
`RequestModifications`), CLI flags (`madhyamas replay run <id> --url`,
`--method`, `--header`, `--body`, `--body-file`, `--follow-redirects`), or
the MCP `madhyamas_replay_request` tool's `modifications` parameter. See
[docs/EDIT_THEN_REPEAT.md](docs/EDIT_THEN_REPEAT.md).

**Repeat Advanced (Batch Replay)**: Saved requests can be replayed multiple
times with configurable concurrency, iterations, and inter-request delay via
the web UI's "Advanced" button (iterations slider, concurrency slider,
optional delay), the CLI (`madhyamas replay run-advanced <id> --iterations
N --concurrency N --delay-ms N`), the API (`POST /replay/execute/{id}/batch`),
or the MCP `madhyamas_replay_advanced` tool. Returns aggregate statistics
(success/failure counts, min/avg/max/p95 latency). Safety limits: iterations
capped at 10,000 and concurrency at 100. See
[docs/REPEAT_ADVANCED.md](docs/REPEAT_ADVANCED.md).

**Scripting System**: JavaScript (ES6+) scripting via embedded `boa_engine`
runtime.  Scripts subscribe to hooks (`on_request`, `on_response`, etc.) and
can modify requests/responses, block requests, mock responses, add headers,
and log traffic.  Sandboxed by construction — no filesystem, network, or
process access.  Scripts persisted to SQLite and survive restarts.  Features:
test dialog (dry-run), syntax validation, execution history with console
output, 13 built-in templates (Log Requests, Add CORS, Block Domains, Modify
Headers, Mock API, Inject Latency, Rewrite URL, Inject Auth Token, Modify JSON
Response, Override Status Code, Cache Buster, Strip Response Headers,
Conditional Mock).  Built-in JS APIs: `console.log`, `JSON.parse/stringify`,
`base64.encode/decode`, `crypto.hash` (SHA-256), `url.parse/build`.
Configured via the Scripts panel in the tools sidebar, `GET/POST/PUT/DELETE
/api/scripts`, `POST /api/scripts/test`, `POST /api/scripts/validate`,
`GET /api/scripts/{id}/history`, the `madhyamas scripts` CLI subcommand, or
MCP tools (`madhyamas_list_scripts`, `madhyamas_create_script`,
`madhyamas_test_script`, `madhyamas_validate_script`,
`madhyamas_get_script_history`).  See
[docs/SCRIPTING.md](docs/SCRIPTING.md), [docs/SCRIPTING_API.md](docs/SCRIPTING_API.md),
[docs/SCRIPTING_SECURITY.md](docs/SCRIPTING_SECURITY.md).

**Data Directory**: `~/.madhyamas/` (certs, logs, traffic.db)

**API Endpoints** (all under `/api` prefix):

| Category | Endpoints |
|----------|-----------|
| Traffic | `GET /traffic`, `GET /traffic/{id}` (supports `?decompressed=true` for on-demand gzip/deflate/brotli/zstd decompression), `POST /traffic/clear`, `GET /traffic/count`, `POST /traffic/import/har` |
| Sessions | `GET /sessions`, `POST /sessions`, `GET/DELETE /sessions/{id}`, `GET /sessions/{id}/export`, `POST /sessions/{id}/switch`, `POST /sessions/import` |
| Export | `GET /export/har`, `GET /export/curl/{id}` |
| Certificate | `GET /cert/ca` |
| WebSocket | `GET /ws` (real-time traffic updates) |
| Config | `GET /config`, `PATCH /config` |
| Auto Save | `GET /autosave`, `PATCH /autosave`, `POST /autosave/snapshot` |
| Capture | `GET /capture`, `POST /capture/toggle`, `GET /capture/stats` |
| Breakpoints | `GET/POST /breakpoints`, `GET/DELETE /breakpoints/{id}`, `GET /breakpoints/paused`, `POST /breakpoints/paused/{id}/resume` |
| Mocks | `GET/POST /mocks`, `GET/PUT/DELETE /mocks/{id}`, `POST /mocks/{id}/toggle`, collections, recording, import/export |
| Rewrites | `GET/POST /rewrites`, `GET /rewrites/templates`, `GET/PUT/DELETE /rewrites/{id}`, `POST /rewrites/{id}/toggle`, `POST /rewrites/batch-toggle` |
| Throttle | `GET/POST /throttle`, `POST /throttle/enabled`, `GET /throttle/presets` |
| Replay | `GET/POST /replay/saved`, `POST /replay/execute/{id}`, `POST /replay/execute/{id}/batch`, `GET /replay/history` |
| Block List | `GET/POST /blocklist`, `GET /blocklist/stats`, `GET/PUT/DELETE /blocklist/{id}`, `POST /blocklist/{id}/toggle` |
| Focus | `GET/POST /focus`, `DELETE /focus/{id}`, `DELETE /focus` (clear all) |
| Mirror | `GET /mirror`, `POST /mirror/toggle`, `PATCH /mirror/config` |
| gRPC | `GET /grpc/connections`, `GET /grpc/streams`, `GET /grpc/frames`, `GET /grpc/stats` |
| Scripts | `GET/POST /scripts`, `GET/PUT/DELETE /scripts/{id}`, `POST /scripts/{id}/toggle`, `GET /scripts/templates`, `GET/PUT /scripts/config`, `GET /scripts/history`, `POST /scripts/test`, `POST /scripts/validate`, `GET/DELETE /scripts/{id}/history` |
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
