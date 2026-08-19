# Madhyamas - Project Instructions

## Project Overview

HTTP/HTTPS debugging proxy in Rust with React web UI. Alternative to Charles Proxy/Fiddler.

- **Language**: Rust (backend), TypeScript/React (frontend)
- **Architecture**: Workspace with 6 crates (core, api, cli, mcp, plugin-sdk, main binary)
- **Binary**: Single unified `madhyamas` binary (proxy + web UI + MCP + CLI)
- **Web UI**: Embedded at compile time via `rust-embed` — no external files needed
- **License**: Dual MIT OR Apache-2.0

## Project Structure

```
madhyamas/
├── crates/
│   ├── madhyamas/             # Unified binary (subcommands: serve/mcp/cli)
│   ├── madhyamas-core/        # Core proxy engine, TLS, traffic storage, plugins
│   ├── madhyamas-api/         # REST/WebSocket API + embedded web assets (axum)
│   ├── madhyamas-cli/         # CLI library (re-exported by main binary)
│   ├── madhyamas-mcp/         # MCP server library (re-exported by main binary)
│   └── madhyamas-plugin-sdk/  # Guest SDK for writing WASM plugins
├── web/                       # React + TypeScript frontend (Vite)
├── docs/                      # Developer / reference documentation
├── docs-site/                 # End-user documentation (VitePress, GitHub Pages)
├── plugins/                   # Bundled example plugins + registry.json
├── skills/                    # Published AI skill package (@madhyamas/skill)
├── agents/                    # Specialized AI agent definitions (LLM + harness agnostic)
│   ├── agents/                # Canonical agent definitions (source of truth)
│   ├── references/            # Shared reference files (loaded on demand)
│   └── scripts/               # install.sh (fan out to harnesses) + validate.sh
├── docker/                    # Docker setup
└── Cargo.toml                 # Workspace configuration
```

## Unified Binary Usage

```bash
madhyamas                          # Start proxy + web UI (default subcommand)
madhyamas mcp                      # Run as MCP server (stdio transport)
madhyamas traffic list             # CLI: list captured traffic
madhyamas export har --output f.har # CLI: export HAR
madhyamas --help                   # See all 159 CLI subcommands
```

Full CLI reference: `skills/madhyamas/references/cli-commands.md`.
Full MCP tool reference: `skills/madhyamas/references/mcp-tools.md`.

## Core Technologies

**Backend**: axum, hyper, tokio, rustls, rcgen, rusqlite, serde, clap, tracing, rust-embed
**Frontend**: React 18, TypeScript, Vite, Tailwind CSS, shadcn/ui, TanStack Query

## Important Files & Modules

### Main Binary (`madhyamas`)
- `main.rs` - Unified entry point with subcommands (serve/mcp/cli)

### Core Crate (`madhyamas-core`)
- `lib.rs` - Public API exports, error types
- `access_control.rs` - IP allowlist (CIDR-based access control)
- `proxy/engine.rs` - Main proxy engine logic
- `tls/certificate.rs` - TLS certificate management
- `traffic/store.rs` - SQLite-based traffic storage
- `intercept/` - Intercept pipeline (see priority order below): `block_list`, `rewrite`, `mock`, `breakpoint`, `throttle`
- `scripting/` - JavaScript scripting system (boa_engine). See [docs/SCRIPTING.md](docs/SCRIPTING.md), [docs/SCRIPTING_API.md](docs/SCRIPTING_API.md), [docs/SCRIPTING_SECURITY.md](docs/SCRIPTING_SECURITY.md)
- `plugin/` - WASM plugin system (wasmtime). See [docs/PLUGINS.md](docs/PLUGINS.md), [docs/PLUGIN_DEVELOPMENT.md](docs/PLUGIN_DEVELOPMENT.md), [docs/PLUGIN_API.md](docs/PLUGIN_API.md), [docs/PLUGIN_SECURITY.md](docs/PLUGIN_SECURITY.md)
- `log_rotation.rs` - Rotating file logger (`LogHandle`, `RotatingFileWriter`)

### Plugin SDK Crate (`madhyamas-plugin-sdk`)
- `lib.rs` - Guest SDK: `Plugin` trait, `register_plugin!` macro, `Context`/`Outcome` types

Example plugins live in `plugins/` (not `crates/madhyamas-plugin-sdk/examples/`):
`cors-helper`, `request-logger`, `domain-blocker`. See
[docs/PLUGIN_DEVELOPMENT.md](docs/PLUGIN_DEVELOPMENT.md).

### API Crate (`madhyamas-api`)
- `lib.rs` - API server setup
- `embedded_assets.rs` - rust-embed web UI serving (compiled into binary)
- `routes.rs` - Route definitions
- `handlers.rs`, `intercept_handlers.rs`, `tools_handlers.rs`, `enterprise_handlers.rs` - Request handlers
- `ws.rs` - WebSocket connection handler
- `middleware.rs` - Auth middleware
- `error.rs`, `validation.rs` - API error types, input validation

### CLI Crate (`madhyamas-cli`)
- `lib.rs` - Exports `Commands` enum and `ApiClient`
- `commands/` - CLI subcommands (traffic, mocks, breakpoints, etc.)

### MCP Crate (`madhyamas-mcp`)
- `lib.rs` - Exports `McpServer` and `McpConfig`
- `server.rs` - MCP server (stdio transport)
- `tools/` - MCP tools for AI agent integration

### Specialized AI Agents (`agents/`)
Project-local, LLM-agnostic, harness-agnostic agent definitions that make common
activities more efficient than a general-purpose agent. Authored once; `install.sh`
fans out to every supported harness as both subagent profiles and slash-command skills.

See [agents/README.md](agents/README.md) for the full guide. Install/validate:
```bash
bash agents/scripts/install.sh              # install to all harnesses
bash agents/scripts/install.sh claude       # only Claude Code
bash agents/scripts/validate.sh             # validate canonical source
```

Harness output dirs (`.agents/`, `.claude/agents/`, `.devin/agents/`, etc.) are
gitignored — only `agents/` is tracked. The `agents/` package teaches AI agents
how to *develop* Madhyamas; the `skills/madhyamas/` package teaches AI agents
how to *use* Madhyamas as a debugging proxy. They do not overlap.

## Development Workflow

```bash
# Docker (recommended for deployment)
./startup.sh           # Build and start with Docker Compose
./stop.sh              # Stop containers

# Local development
./startup-local.sh                    # Enterprise: Docker multi-instance stack
                                      # (PostgreSQL + Redis + 2x Madhyamas + nginx LB)
./startup-local.sh --tier oss         # OSS: local binary (SQLite, single instance)
./startup-local.sh --clean            # Clean rebuild (current tier)
./stop-local.sh                       # Stop all instances (local + Docker)
./stop-local.sh --tier oss            # Stop OSS local binary only
./stop-local.sh --tier enterprise     # Stop enterprise Docker stack + local binary

# Manual commands
cargo build --release -p madhyamas   # Build unified binary
RUST_LOG=debug cargo run --bin madhyamas
cargo test
cargo fmt --all && cargo clippy --all-targets --all-features

# Frontend (must build before Rust — assets are embedded at compile time)
cd web && npm run build
```

### Private git dependency (`licensing-core`)

`madhyamas-enterprise` depends on `licensing-core` from the private
[ShristiLabs/licensing](https://github.com/ShristiLabs/licensing) repo
(pinned to a `licensing-core-vX.Y.Z` tag). Local development with a sibling
checkout: copy `.cargo/config.toml.example` to `.cargo/config.toml`
(gitignored) to redirect the dependency to `../licensing/crates/licensing-core`
and fetch private repos over SSH. CI needs read credentials for the private
repo. Bump the tag pin in the root `Cargo.toml` to pick up licensing-core
changes.

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
- `MADHYAMAS_ENABLE_SOCKS` / `MADHYAMAS_SOCKS_PORT` / `MADHYAMAS_SOCKS_USERNAME` / `MADHYAMAS_SOCKS_PASSWORD` - SOCKS5 listener
- `MADHYAMAS_UPSTREAM_PROXY_ENABLED` / `MADHYAMAS_UPSTREAM_PROXY` / `MADHYAMAS_UPSTREAM_PROTOCOL` / `MADHYAMAS_UPSTREAM_AUTH` / `MADHYAMAS_UPSTREAM_NO_PROXY` - Upstream proxy chaining
- `MADHYAMAS_ALLOWED_IPS` - Comma-separated IP/CIDR allowlist

**Data Directory**: `~/.madhyamas/` (certs, logs, traffic.db, plugins/)

### Feature Documentation Index

Each feature has a dedicated doc page. Read the relevant one before working on that area.

| Feature | Doc | Summary |
|---|---|---|
| SOCKS5 proxy | [docs/SOCKS_PROXY.md](docs/SOCKS_PROXY.md) | Blind TCP tunnel on port 1080; HTTPS MITM not supported via SOCKS |
| Upstream proxy chaining | [docs/UPSTREAM_PROXY.md](docs/UPSTREAM_PROXY.md) | Route all outbound via HTTP/HTTPS/SOCKS5 upstream; bypass list |
| Access control (IP allowlist) | [docs/ACCESS_CONTROL.md](docs/ACCESS_CONTROL.md) | CIDR allowlist; loopback always allowed; live via `PATCH /api/config` |
| Block list | [docs/BLOCK_LIST.md](docs/BLOCK_LIST.md) | Domain/pattern blocking at intercept priority 5 |
| Focus | [docs/FOCUS.md](docs/FOCUS.md) | Visual emphasis of matching hosts (not a filter); persists to SQLite |
| Recording size limits | [docs/RECORDING_LIMITS.md](docs/RECORDING_LIMITS.md) | `max_requests`, `max_total_size_mb`, `max_body_size`, FIFO pruning |
| Auto Save | [docs/AUTO_SAVE.md](docs/AUTO_SAVE.md) | Periodic HAR/session backup + optional rotation |
| Mirror tool | [docs/MIRROR.md](docs/MIRROR.md) | Save response bodies to disk mirroring URL path structure |
| Log file rotation | [docs/LOGGING.md](docs/LOGGING.md) | Time/size/on-demand rotation; `RotatingFileWriter` in `log_rotation.rs` |
| Timeline view (waterfall) | [docs/TIMELINE_VIEW.md](docs/TIMELINE_VIEW.md) | Waterfall chart in web UI; virtualized rows |
| Edit-then-Repeat | [docs/EDIT_THEN_REPEAT.md](docs/EDIT_THEN_REPEAT.md) | Modify saved requests before replay (UI/CLI/MCP) |
| Repeat Advanced (batch) | [docs/REPEAT_ADVANCED.md](docs/REPEAT_ADVANCED.md) | Iterations/concurrency/delay; capped at 10k iters / 100 concurrency |
| Scripting system | [docs/SCRIPTING.md](docs/SCRIPTING.md) | JS (ES6+) via boa_engine; sandboxed; 13 templates; SQLite-persisted |
| Plugin system | [docs/PLUGINS.md](docs/PLUGINS.md) | WASM (wasmtime); fuel-metered; Ed25519-signed; hot-reload; 5 templates |
| gRPC | [docs/HTTP2_SUPPORT.md](docs/HTTP2_SUPPORT.md) | gRPC traffic inspection |
| HAR import | [docs/HAR_IMPORT.md](docs/HAR_IMPORT.md) | Import HAR files as sessions |
| Rewrite templates | [docs/REWRITE_TEMPLATES.md](docs/REWRITE_TEMPLATES.md) | Built-in rewrite rules (No Caching, Add CORS, etc.) |
| Mock responses | [docs/MOCK_RESPONSES.md](docs/MOCK_RESPONSES.md) | Single/sequence/conditional/probabilistic mocks; collections; recording |
| Intercept pipeline | [docs/INTERCEPT_PIPELINE.md](docs/INTERCEPT_PIPELINE.md) | `InterceptHandler` trait, 5 handlers, priority execution order |
| Extension system | [docs/EXTENSION_SYSTEM.md](docs/EXTENSION_SYSTEM.md) | Unified `Extension` trait abstracting scripts (prio 10) and plugins (prio 20) |
| Persistence layer | [docs/PERSISTENCE.md](docs/PERSISTENCE.md) | SQLite schema, traffic/intercept/config stores, session model |
| Web frontend | [docs/WEB_FRONTEND.md](docs/WEB_FRONTEND.md) | React architecture, TanStack Query, WebSocket client, build/embed flow |
| Performance | [docs/PERFORMANCE.md](docs/PERFORMANCE.md) | Memory tracking, metrics collector, alerting, connection pool |
| Enterprise (current internals) | [docs/ENTERPRISE.md](docs/ENTERPRISE.md) | Auth (JWT + API keys), RBAC, audit logging (PostgreSQL hash chain), user management |
| Enterprise crate guide | [docs/ENTERPRISE_CRATE_GUIDE.md](docs/ENTERPRISE_CRATE_GUIDE.md) | Enterprise crate structure, public API, key types, extension points, Mermaid diagrams |
| Enterprise API integration | [docs/ENTERPRISE_API_INTEGRATION.md](docs/ENTERPRISE_API_INTEGRATION.md) | AuthProvider/Authorizer/AuditSink traits, AppState injection, router merging, middleware, Mermaid sequence diagrams |
| Enterprise startup flow | [docs/ENTERPRISE_STARTUP_FLOW.md](docs/ENTERPRISE_STARTUP_FLOW.md) | 17-step initialization sequence, CLI flags, error handling, graceful shutdown, Mermaid flowchart + sequence diagrams |
| Storage backend guide | [docs/STORAGE_BACKEND_GUIDE.md](docs/STORAGE_BACKEND_GUIDE.md) | Storage trait reference, implementation checklist for new backends, schema design, testing, migration, Mermaid ER diagrams |
| Enterprise testing | [docs/ENTERPRISE_TESTING.md](docs/ENTERPRISE_TESTING.md) | Enterprise testing guide: unit, integration, multi-instance, Playwright E2E, CI/CD, Mermaid diagrams |
| Enterprise analysis (overview) | [docs/ENTERPRISE_OVERVIEW.md](docs/ENTERPRISE_OVERVIEW.md) | Two-tier model, crate architecture, database strategy, licensing overview, roadmap (historical — written pre-implementation) |
| Enterprise licensing server | [docs/ENTERPRISE_LICENSING_SERVER.md](docs/ENTERPRISE_LICENSING_SERVER.md) | Pointer: the licensing server now lives in the private ShristiLabs/licensing repo (multi-product platform); shared `licensing-core` crate keeps signing/verification compatible |
| Enterprise storage traits | [docs/ENTERPRISE_STORAGE_TRAITS.md](docs/ENTERPRISE_STORAGE_TRAITS.md) | Shared async storage traits, rusqlite → sqlx migration, SQLite + PostgreSQL backends |
| Enterprise auth/RBAC/IdP | [docs/ENTERPRISE_AUTH_RBAC.md](docs/ENTERPRISE_AUTH_RBAC.md) | Authentication modes, RBAC model, OIDC/header/LDAP/SAML integration |
| Enterprise web UI | [docs/ENTERPRISE_WEB_UI.md](docs/ENTERPRISE_WEB_UI.md) | Same-folder runtime-gated approach, tier detection, auth UI, admin panels, build/embedding |
| Enterprise CI/CD | [docs/ENTERPRISE_CICD.md](docs/ENTERPRISE_CICD.md) | Two-tier CI matrix, release workflow, Docker build-args, licensing server pipeline, secrets |
| Enterprise multi-instance | [docs/ENTERPRISE_MULTI_INSTANCE.md](docs/ENTERPRISE_MULTI_INSTANCE.md) | LB routing (context path/subdomain), PostgreSQL+Redis state sync, atomic config propagation, shared CA, license seat tracking, K8s manifests |
| Enterprise perf & security | [docs/ENTERPRISE_PERF_SECURITY.md](docs/ENTERPRISE_PERF_SECURITY.md) | Threat model, 16 security gaps, 10 perf bottlenecks, 16 database optimizations (tiered body storage, write batching, GIN/BRIN/trigram indexes, partitioning, cursor pagination, PgBouncer, read replicas), checklists |
| OSS vs Enterprise comparison | [docs/ENTERPRISE_OSS_COMPARISON.md](docs/ENTERPRISE_OSS_COMPARISON.md) | Side-by-side comparison: architecture, feature parity matrix (42 shared + 17 enterprise-only), build/distribution, database, deployment, security, performance, web UI, CLI/MCP, pricing, upgrade path, FAQ |
| Enterprise AI agent integration | [docs/ENTERPRISE_AI_AGENTS.md](docs/ENTERPRISE_AI_AGENTS.md) | Gap analysis (MCP/CLI/API auth broken for enterprise), MCP server changes (auth config, HTTP transport, enterprise tools, dynamic resources, prompts, annotations), API key middleware, RBAC scopes, multi-instance agent access, agent workflows, security, implementation plan |
| Enterprise crate migration | [docs/ENTERPRISE_CRATE_MIGRATION.md](docs/ENTERPRISE_CRATE_MIGRATION.md) | Detailed migration analysis for extracting madhyamas-enterprise crate: inventory of all enterprise code (859 lines in core, 742 in api), all 17 #[cfg] gates, dependency analysis, cross-crate reference map, trait abstractions (AuthProvider/Authorizer/AuditSink), AppState changes, 6-phase migration plan, risk assessment |
| Enterprise implementation plan | [docs/ENTERPRISE_IMPLEMENTATION_PLAN.md](docs/ENTERPRISE_IMPLEMENTATION_PLAN.md) | Comprehensive implementation plan synthesizing all 12 analysis docs: 13 phases (0-12), dependency graph, critical path, per-phase steps with files and exit criteria, milestone summary (M1-M7), Gantt chart, effort estimates (194 dev-days / ~6mo with 2 devs), risk register (10 risks), verification checklist |

### Intercept Pipeline Priority Order

Handlers run in ascending priority order. Add new intercept features at the
correct priority, not at the end blindly. See
[docs/INTERCEPT_PIPELINE.md](docs/INTERCEPT_PIPELINE.md) for full detail.

| Priority | Handler | Effect |
|---|---|---|
| 5 | Block list | Returns blocked response; never reaches upstream |
| 10 | Rewrites | Modifies request before subsequent handlers see it |
| 20 | Mocks | Short-circuits with a mock response |
| 30 | Breakpoints | Prompts user (only for non-mocked traffic) |
| 40 | Throttle | Applies latency right before forwarding |

### API Endpoints

All endpoints are under the `/api` prefix. The API reference is split by
domain — see [docs/API.md](docs/API.md) for the index:
- [API_TRAFFIC.md](docs/API_TRAFFIC.md) — traffic, sessions, export, cert
- [API_WEBSOCKET_GRPC.md](docs/API_WEBSOCKET_GRPC.md) — WebSocket events, WS/gRPC inspection
- [API_INTERCEPT.md](docs/API_INTERCEPT.md) — breakpoints, mocks, rewrites, throttle, block list, focus, replay
- [API_SCRIPTS_PLUGINS.md](docs/API_SCRIPTS_PLUGINS.md) — scripts and plugins
- [API_CONFIG.md](docs/API_CONFIG.md) — config, capture, auto save, mirror, logs, persistence, health
- [API_ENTERPRISE.md](docs/API_ENTERPRISE.md) — auth, users, RBAC, audit, metrics, onboarding

Real-time traffic updates via WebSocket at `GET /ws`. Health check at `GET /health`.

> **Phase 4 (Enterprise, conditionally enabled):** `/metrics`, `/auth/*`, `/users`, `/rbac/*`, `/audit/*`, `/onboarding/*`

## AI Assistant Guidelines

### Specialized Agents (prefer over general-purpose)
This repo ships specialized agents under `agents/` that are more efficient
than a general-purpose agent for specific activities. Prefer dispatching them
(via subagent profiles or `/agent-name` slash commands after
`bash agents/scripts/install.sh`):

| Activity | Agent | Scope |
|---|---|---|
| End-user docs (`docs-site/`) | `docs-site-author` | VitePress pages, nav, screenshots, SEO |
| Developer docs (`docs/`) | `docs-author` | API contracts, architecture, plugin/scripting refs |
| Feature development | `developer` | `crates/` + `web/` end-to-end implementation |
| Code review | `reviewer` | Read-only git diff review (correctness, security, style, perf) |
| Plugin work | `plugin-engineer` | WASM plugin build, test, sign, package, document |
| MCP/CLI/skill sync | `ai-agent-tooling` | Keep MCP tools, CLI subcommands, skill package in sync |

Each agent has a fixed output format so its result can be consumed by the
next agent in a pipeline (e.g. `developer` → `reviewer` → `docs-author` →
`docs-site-author` → `ai-agent-tooling`).

### Git Commits

- **No AI/harness attribution.** Never add `Co-Authored-By:` trailers for
  Claude or any AI harness, and never append "Generated with Claude Code" (or
  similar) lines to commit messages. The commit author and committer must be
  the user — git is already configured with their identity; do not run
  `git config` to change it.
- **Conventional Commits.** Match the repo's existing style — prefix the
  subject with `feat:`, `fix:`, `chore:`, `release:`, `docs:`, `refactor:`,
  or `test:`.
- Keep the subject under 70 characters in imperative mood ("add", not
  "added"). Reference issues/PRs as `owner/repo#123` in the body, not the
  subject.
- Only commit when explicitly asked. Stage specific files by name, never
  `git add -A` / `git add .`.
- Do not amend, force-push, rewrite history, or pass `--no-verify` unless the
  user explicitly asks for that exact action.

### Rust Best Practices
1. Use `thiserror` for custom errors, `Result<T>` throughout
2. Avoid `unwrap()` in production - use `?` operator
3. All I/O operations async with Tokio
4. Strong typing with minimal panics
5. Do not create tests unless explicitly asked (per global rules)

### Adding Features
1. Add module in `madhyamas-core/src/`
2. Implement with proper error handling
3. Expose public API in `lib.rs`
4. Add API endpoints in `madhyamas-api`
5. If AI-agent-facing, add MCP tool (`madhyamas-mcp/src/tools/`) and CLI subcommand (`madhyamas-cli/src/commands/`)
6. Update documentation — dispatch the `docs-author` agent for `docs/` and the
   `docs-site-author` agent for `docs-site/`; if the feature adds an MCP tool or
   CLI subcommand, dispatch the `ai-agent-tooling` agent to sync the skill
   package. See `agents/references/ai-agent-tooling-workflow.md` for the full
   sync checklist.

### Documentation Guidelines
- `docs/` — developer-facing reference (UPPER_SNAKE_CASE.md). Explains *how it works*.
- `docs-site/` — end-user guide (VitePress, lowercase-hyphenated). Explains *how to use it*.
- Do not duplicate content verbatim between the two.
- **Prefer mermaid diagrams** wherever a visual aids understanding (architecture, flows, pipelines).
- No emojis in prose, headings, or code.
- See `agents/references/docs-site-structure.md` for docs-site authoring rules.

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
- [docs/API.md](docs/API.md) - Full API reference (99 endpoints across 78 paths)
- [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) - Development guide
- [agents/README.md](agents/README.md) - Specialized AI agents (design, install, validate, adding new agents)
- [skills/README.md](skills/README.md) - Published AI skill package (@madhyamas/skill)
