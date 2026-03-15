# Madhyamas - Claude AI Context

## Project Overview

HTTP/HTTPS debugging proxy in Rust with React web UI. Alternative to Charles Proxy/Fiddler.

- **Language**: Rust (backend), TypeScript/React (frontend)
- **Architecture**: Workspace with 4 crates (core, api, cli, mcp)
- **License**: Dual MIT OR Apache-2.0

## Project Structure

```
madhyamas/
├── crates/
│   ├── madhyamas-core/      # Core proxy engine, TLS, traffic storage
│   ├── madhyamas-api/       # REST/WebSocket API server (axum)
│   ├── madhyamas-cli/       # CLI entry point
│   └── madhyamas-mcp/       # MCP server for AI agent integration
├── web/                      # React + TypeScript frontend (Vite)
├── docs/                     # Documentation
├── docker/                   # Docker setup
└── Cargo.toml               # Workspace configuration
```

## Core Technologies

**Backend**: axum, hyper, tokio, rustls, rcgen, rusqlite, serde, clap, tracing
**Frontend**: React 18, TypeScript, Vite, Tailwind CSS, shadcn/ui, TanStack Query, Zustand

## Important Files & Modules

### Core Crate (`madhyamas-core`)
- `lib.rs` - Public API exports, error types
- `proxy/engine.rs` - Main proxy engine logic
- `tls/certificate.rs` - TLS certificate management
- `traffic/store.rs` - SQLite-based traffic storage
- `intercept/` - Breakpoints, mocks, rewrites, throttling

### API Crate (`madhyamas-api`)
- `lib.rs` - API server setup
- `routes.rs` - Route definitions
- `handlers/` - Request handlers

### CLI Crate (`madhyamas-cli`)
- `main.rs` - Entry point, CLI argument parsing

### MCP Crate (`madhyamas-mcp`)
- `main.rs` - MCP server entry point (stdio transport)
- `lib.rs` - MCP tools for AI agent integration with Madhyamas API

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
cargo build --release
RUST_LOG=debug cargo run
cargo test
cargo fmt --all && cargo clippy --all-targets --all-features

# Frontend
cd web && npm run build
```

## Configuration

**CLI Flags**: `--proxy-port`, `--api-port`, `--host`, `--public-ip`

**Environment Variables**:
- `RUST_LOG` - Logging level (trace/debug/info/warn/error)
- `MADHYAMAS_HOST` - Bind host (default: 0.0.0.0)
- `MADHYAMAS_API_PORT` - API port (default: 3001)
- `MADHYAMAS_PROXY_PORT` - Proxy port (default: 8888)
- `MADHYAMAS_PUBLIC_IP` - Public IP shown to users for remote access

**Data Directory**: `~/.madhyamas/` (certs, logs, traffic.db)

**API Endpoints**:
- `GET /api/config` - Returns config with detected/host IP for display

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
- `RUST_LOG=debug cargo run -- --verbose`
- `cargo test -- --nocapture` for test output
- Check database schema matches code expectations

### Common Issues
- **Certificate Errors**: Install CA cert in system trust store
- **Port Conflicts**: Check ports 8888/3001 available
- **Database Locked**: Only one instance at a time

## Resources

- [README.md](README.md) - Quick start
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [docs/API.md](docs/API.md) - API reference
