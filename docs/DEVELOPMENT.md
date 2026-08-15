# Development Guide

> **Last verified:** 2026-08-12 against Madhyamas `0.1.6`.

## Prerequisites

### Required Tools
- **Rust** 1.88 or later (see `Cargo.toml` `rust-version`)
- **Cargo** (comes with Rust)
- **Node.js** 18+ and npm
- **Git**
- **OpenSSL** (for HTTPS certificate generation)
  ```bash
  # macOS
  brew install openssl
  # Ubuntu/Debian
  sudo apt-get install libssl-dev
  # Fedora/RHEL
  sudo dnf install openssl-devel
  ```

### Optional Tools
- **Docker** (for containerized development)
- **SQLite CLI** (for database inspection)
- **Postman/Insomnia** (for API testing)
- **cargo-watch** (for backend auto-reload on file change)

## Setting Up Development Environment

### 1. Clone the Repository
```bash
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
```

### 2. Install Rust Dependencies
```bash
# Update Rust to latest stable
rustup update stable

# Install development tools
cargo install cargo-watch
cargo install cargo-edit
cargo install cargo-outdated
```

### 3. Build the Backend
```bash
# Build all crates
cargo build

# Build in release mode (optimized)
cargo build --release

# Build specific crate
cargo build -p madhyamas-core
```

### 4. Set Up Frontend
```bash
cd web
npm install
npm run dev
```

## Project Structure

```
madhyamas/
├── crates/
│   ├── madhyamas-core/       # Core library
│   │   ├── src/
│   │   │   ├── lib.rs         # Public API
│   │   │   ├── config.rs      # Configuration
│   │   │   ├── proxy/         # Proxy engine
│   │   │   ├── tls/           # TLS/certificate handling
│   │   │   ├── traffic/       # Traffic storage
│   │   │   ├── intercept/     # Breakpoints, mocks, rewrites
│   │   │   ├── websocket.rs   # WebSocket support
│   │   │   ├── grpc/          # gRPC support
│   │   │   ├── session.rs     # Session management
│   │   │   ├── replay.rs      # Request replay
│   │   │   ├── scripting/     # JS/TS scripting
│   │   │   └── plugin/        # Plugin system
│   │   ├── Cargo.toml
│   │   └── tests/
│   ├── madhyamas-api/        # API server
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── routes.rs           # Route definitions
│   │   │   ├── handlers.rs         # Core request handlers
│   │   │   ├── intercept_handlers.rs  # Breakpoint/mock/rewrite/throttle handlers
│   │   │   ├── phase3_handlers.rs     # gRPC/script/plugin handlers
│   │   │   ├── phase4_handlers.rs     # Enterprise/auth/audit handlers
│   │   │   ├── ws.rs                # WebSocket handler
│   │   │   ├── middleware.rs        # Auth middleware
│   │   │   ├── error.rs             # Error types
│   │   │   └── validation.rs        # Input validation
│   │   └── Cargo.toml
│   ├── madhyamas-mcp/        # MCP server library
│   │   ├── src/
│   │   └── Cargo.toml
│   └── madhyamas-cli/        # CLI
│       ├── src/
│       │   └── commands/     # CLI subcommands
│       └── Cargo.toml
├── web/                       # React frontend
│   ├── src/
│   ├── package.json
│   └── vite.config.ts
├── docs/                      # Documentation
├── Cargo.toml                 # Workspace config
└── README.md
```

## Development Workflow

### Running in Development Mode

#### Terminal 1: Backend
```bash
# Run with auto-reload on file changes
cargo watch -x run

# Or run manually
cargo run

# With debug logging
RUST_LOG=debug cargo run

# With specific log levels
RUST_LOG=madhyamas_core=debug,madhyamas_api=info cargo run
```

#### Terminal 2: Frontend
```bash
cd web
npm run dev
```

The application will be available at:
- **Proxy**: `http://localhost:8888`
- **Web UI**: `http://localhost:3000` (dev server)
- **API**: `http://localhost:3001`

### Running Tests

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p madhyamas-core

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run tests in parallel
cargo test -- --test-threads=4

# Run ignored tests
cargo test -- --ignored
```

### Code Quality

#### Formatting
```bash
# Format all code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check
```

#### Linting
```bash
# Run clippy on all targets
cargo clippy --all-targets --all-features

# Treat warnings as errors
cargo clippy --all-targets --all-features -- -D warnings

# Fix clippy suggestions automatically
cargo clippy --fix
```

#### Type Checking
```bash
# Check without building
cargo check

# Check all targets
cargo check --all-targets
```

## Coding Standards

### Rust Style Guide

#### Error Handling
```rust
// ✅ Good: Use Result and ? operator
pub fn process_request(&self, req: &Request) -> Result<Response> {
    let data = self.parse_request(req)?;
    let result = self.handle_data(data)?;
    Ok(result)
}

// ❌ Bad: Using unwrap() in production code
pub fn process_request(&self, req: &Request) -> Response {
    let data = self.parse_request(req).unwrap();
    self.handle_data(data).unwrap()
}
```

#### Async/Await
```rust
// ✅ Good: Async functions with proper error handling
pub async fn fetch_data(&self, url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

// Use tokio::spawn for concurrent tasks
tokio::spawn(async move {
    // Background task
});
```

#### Ownership and Borrowing
```rust
// ✅ Good: Borrow when possible
pub fn process(&self, data: &[u8]) -> Result<String> {
    String::from_utf8(data.to_vec())
        .map_err(|e| Error::Encoding(e.to_string()))
}

// ❌ Bad: Unnecessary cloning
pub fn process(&self, data: Vec<u8>) -> Result<String> {
    String::from_utf8(data.clone())
        .map_err(|e| Error::Encoding(e.to_string()))
}
```

#### Documentation
```rust
/// Processes an HTTP request through the proxy
///
/// # Arguments
/// * `request` - The incoming HTTP request
/// * `config` - Proxy configuration
///
/// # Returns
/// * `Ok(Response)` - The processed response
/// * `Err(Error)` - If processing fails
///
/// # Examples
/// ```
/// let response = engine.process_request(&request, &config)?;
/// ```
pub async fn process_request(
    &self,
    request: Request,
    config: &ProxyConfig,
) -> Result<Response> {
    // Implementation
}
```

### Module Organization

```rust
// lib.rs - Public API exports
pub mod config;
pub mod proxy;
pub mod tls;
pub mod traffic;

pub use config::ProxyConfig;
pub use proxy::ProxyEngine;
pub use tls::CertificateManager;

// Error types
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TLS error: {0}")]
    Tls(String),
}

pub type Result<T> = std::result::Result<T, Error>;
```

## Adding New Features

### 1. Core Feature in `madhyamas-core`

```bash
# Create new module
touch crates/madhyamas-core/src/my_feature.rs
```

```rust
// crates/madhyamas-core/src/my_feature.rs
use crate::{Error, Result};

/// My feature implementation
pub struct MyFeature {
    config: MyConfig,
}

impl MyFeature {
    pub fn new(config: MyConfig) -> Self {
        Self { config }
    }
    
    pub async fn process(&self, input: &str) -> Result<String> {
        // Implementation
        Ok(input.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_my_feature() {
        let feature = MyFeature::new(MyConfig::default());
        // Test implementation
    }
}
```

```rust
// crates/madhyamas-core/src/lib.rs
pub mod my_feature;
pub use my_feature::MyFeature;
```

### 2. API Endpoint in `madhyamas-api`

```rust
// crates/madhyamas-api/src/handlers/my_feature_handlers.rs
use axum::{extract::State, Json};
use madhyamas_core::MyFeature;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct MyRequest {
    input: String,
}

#[derive(Serialize)]
pub struct MyResponse {
    output: String,
}

pub async fn handle_my_feature(
    State(feature): State<MyFeature>,
    Json(req): Json<MyRequest>,
) -> Result<Json<MyResponse>, StatusCode> {
    let output = feature.process(&req.input)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    Ok(Json(MyResponse { output }))
}
```

```rust
// crates/madhyamas-api/src/routes.rs
use axum::routing::post;

pub fn create_router() -> Router {
    Router::new()
        .route("/api/my-feature", post(handle_my_feature))
        // ... other routes
}
```

### 3. Frontend Integration

```typescript
// web/src/api/myFeature.ts
export interface MyRequest {
  input: string;
}

export interface MyResponse {
  output: string;
}

export async function processMyFeature(input: string): Promise<MyResponse> {
  const response = await fetch('/api/my-feature', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ input }),
  });
  return response.json();
}
```

## Database Changes

### Modifying Schema

```rust
// crates/madhyamas-core/src/traffic/store.rs
fn create_tables(&self) -> Result<()> {
    let conn = self.conn.lock();
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS my_table (
            id TEXT PRIMARY KEY,
            data TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        
        CREATE INDEX IF NOT EXISTS idx_my_table_created 
        ON my_table(created_at);
        "#
    ).map_err(Error::Database)?;
    
    Ok(())
}
```

### Migration Strategy
1. Add new columns with `ALTER TABLE`
2. Provide default values for existing rows
3. Update queries to use new schema
4. Increment schema version in config

## Debugging

### Backend Debugging

```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Specific module logging
RUST_LOG=madhyamas_core::proxy=trace cargo run

# Use rust-lldb or rust-gdb
rust-lldb target/debug/madhyamas
```

### Frontend Debugging
- Use browser DevTools
- React DevTools extension
- Network tab for API calls
- Console for errors

### Database Inspection

```bash
# Open SQLite database
sqlite3 ~/.madhyamas/traffic.db

# List tables
.tables

# View schema
.schema requests

# Query data
SELECT * FROM requests LIMIT 10;
```

## Performance Profiling

### CPU Profiling
```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --bin madhyamas

# Open flamegraph.svg in browser
```

### Memory Profiling
```bash
# Use valgrind
valgrind --tool=massif target/release/madhyamas

# Or heaptrack
heaptrack target/release/madhyamas
```

### Benchmarking
```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench --bench my_benchmark
```

## Common Issues

### Build Errors

**Issue**: `error: linker 'cc' not found`
```bash
# macOS
xcode-select --install

# Ubuntu/Debian
sudo apt install build-essential

# Fedora
sudo dnf install gcc
```

**Issue**: OpenSSL errors
```bash
# macOS
brew install openssl
export OPENSSL_DIR=/usr/local/opt/openssl

# Ubuntu/Debian
sudo apt install libssl-dev pkg-config
```

### Runtime Errors

**Issue**: Port already in use
```bash
# Find process using port
lsof -i :8888
kill -9 <PID>
```

**Issue**: Database locked
```bash
# Only one instance can access SQLite
# Stop other instances or use different database path
cargo run -- --db-path /tmp/madhyamas.db
```

## CI/CD

### GitHub Actions
The project uses GitHub Actions for:
- Running tests on push/PR
- Linting with clippy
- Formatting checks
- Building release binaries
- Publishing Docker images

### Local CI Simulation
```bash
# Run all checks locally
./scripts/ci-check.sh
```

## Release Process

### Version Bumping
```bash
# Update version in Cargo.toml files
cargo set-version 0.2.0

# Commit version bump
git commit -am "chore: bump version to 0.2.0"
git tag v0.2.0
git push origin main --tags
```

### Building Release Binaries
```bash
# Build optimized binary
cargo build --release

# Strip symbols for smaller binary
strip target/release/madhyamas

# Cross-compile for other platforms
cargo install cross
cross build --target x86_64-unknown-linux-musl --release
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum/)
- [React Documentation](https://react.dev/)
- [Project README](../README.md)
- [Architecture Guide](ARCHITECTURE.md)

## Project Structure

```
madhyamas/
├── Cargo.toml                 # Workspace configuration
├── crates/
│   ├── madhyamas/             # Unified binary (proxy + web UI + MCP + CLI)
│   ├── madhyamas-core/        # Core proxy engine (Rust)
│   ├── madhyamas-api/         # REST/WebSocket API + embedded web assets (Rust)
│   ├── madhyamas-cli/         # CLI library (re-exported by main binary)
│   └── madhyamas-mcp/         # MCP server library (re-exported by main binary)
├── web/                       # React frontend (embedded at compile time)
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
├── android/                   # Android VPN companion app (Kotlin)
├── docs/                      # Documentation
├── docker/                    # Docker setup
├── skills/                    # AI agent skills package (multi-harness)
│   └── madhyamas/             # SKILL.md + references + build scripts
└── README.md
```

## Technology Stack

### Backend (Rust)

- **axum** — Web framework
- **hyper** — HTTP server/client
- **tokio** — Async runtime
- **rustls** — TLS implementation
- **rcgen** — Certificate generation
- **rusqlite** — SQLite storage
- **clap** — CLI framework
- **reqwest** — HTTP client for upstream requests (gzip/deflate/brotli support)
- **tower-governor** — Rate limiting (opt-in)

### Frontend (React)

- **React 18** — UI framework
- **TypeScript** — Type safety
- **Vite** — Build tool
- **Tailwind CSS** — Styling
- **shadcn/ui** — UI components
- **TanStack Query** — Data fetching
- **Zustand** — State management
- **Prism.js** — Syntax highlighting for JSON viewer
- **react-json-view-lite** — Collapsible JSON tree view
- **jsonpath-plus** — JSONPath query engine
- **jmespath** — JMESPath query engine

## Development Workflow

The web UI is embedded into the Rust binary at compile time via `rust-embed`. For development:

1. Run the web UI dev server: `cd web && npm run dev`
2. Run the Rust backend: `cargo run -- --verbose`
3. The backend serves the web UI at `http://localhost:3001`
4. For production builds, always build the web UI first (`cd web && npm run build`), then rebuild the Rust binary

## Git Hooks (Pre-commit Checks)

A pre-commit hook is provided to catch formatting and clippy issues before they reach CI. To install:

```bash
./hooks/install.sh
```

This installs a `pre-commit` hook that runs:

- **`cargo fmt --all -- --check`** — fails if any Rust file is not formatted
- **`cargo clippy --all-targets --all-features -- -D warnings`** — fails on any clippy warning
- **`npm run lint`** — fails on frontend lint issues (when web files are changed)

The hook only runs when `.rs` files or frontend config files are staged. To bypass temporarily:

```bash
git commit --no-verify
```

## Local Development Scripts

For convenience, startup scripts are provided to build and run Madhyamas
locally without Docker.

### Start / Stop

```bash
./startup-local.sh           # Build web + Rust, start in background
./startup-local.sh --clean   # Clean rebuild (removes web/dist, node_modules, target)
./stop-local.sh              # Stop the local instance
```

### Data Directory

All Madhyamas data lives under `~/.madhyamas/`:

```
~/.madhyamas/
├── certs/           # SSL certificates
├── logs/            # Log files (madhyamas.log)
├── traffic.db       # Traffic database (SQLite)
└── madhyamas.pid    # Process ID file
```

View logs in real-time:

```bash
tail -f ~/.madhyamas/logs/madhyamas.log
```

To reset all data:

```bash
rm -rf ~/.madhyamas/
```

### Troubleshooting

**Port already in use:**

```bash
lsof -i :3001          # find the process
kill -9 <PID>          # or use different ports:
export MADHYAMAS_API_PORT=3002
export MADHYAMAS_PROXY_PORT=8889
./startup-local.sh
```

**Build errors:**

```bash
./startup-local.sh --clean   # or manually: cargo clean && rm -rf web/dist web/node_modules
```

**Process won't stop:**

```bash
pkill -9 -f madhyamas
rm ~/.madhyamas/madhyamas.pid
```

### Local vs Docker

| Feature | Local | Docker |
|---------|-------|--------|
| Setup | Requires Rust + Node.js | Only requires Docker |
| Build time | Faster (incremental) | Slower (full rebuild) |
| Hot reload | Yes (with cargo-watch) | No |
| Isolation | No | Yes |
| Best for | Development | Production / testing |

## Enterprise Development

Enterprise features require PostgreSQL and (optionally) Redis. The fastest
way to get a development environment is the multi-instance Docker Compose stack.

### Prerequisites

- **PostgreSQL 16+** — for enterprise storage (users, audit, API keys, sessions)
- **Redis 7+** — for multi-instance pub/sub and license seat coordination
- Docker and Docker Compose (for the stack)

### Build Configurations

```bash
# Enterprise build (default — includes all enterprise features)
cargo build --release -p madhyamas

# OSS build (no enterprise code compiled)
cargo build --release --no-default-features -p madhyamas

# Run enterprise tests
cargo test -p madhyamas-enterprise

# Run OSS tests (no PostgreSQL needed)
cargo test --no-default-features -p madhyamas-core --lib
```

### Starting the Enterprise Stack

```bash
# Multi-instance stack (2 instances + PostgreSQL + Redis + nginx)
./startup-local.sh --tier enterprise

# Services:
#   nginx LB:    http://localhost:14000 (API), http://localhost:8888 (proxy)
#   Instance 1:  http://localhost:14001
#   Instance 2:  http://localhost:14002
#   PostgreSQL:  localhost:15432
#   Redis:       localhost:16379

# Default admin: admin / testpass123

# Stop the stack
./stop-local.sh --tier enterprise
```

### Manual PostgreSQL + Redis Setup

```bash
# Start PostgreSQL
docker run -d --name madhyamas-pg \
  -e POSTGRES_USER=madhyamas \
  -e POSTGRES_PASSWORD=madhyamas \
  -e POSTGRES_DB=madhyamas \
  -p 5432:5432 postgres:16

# Start Redis
docker run -d --name madhyamas-redis -p 6379:6379 redis:7

# Run with enterprise features
cargo run --bin madhyamas -- \
  --database-url postgres://madhyamas:madhyamas@localhost:5432/madhyamas \
  --redis-url redis://localhost:6379 \
  --enable-auth \
  --jwt-secret dev-secret \
  --admin-username admin \
  --admin-password testpass123
```

### Enterprise Development Workflow

```mermaid
flowchart LR
    FE["cd web && npm run build"] --> BE["cargo build -p madhyamas"]
    BE --> STACK["./startup-local.sh --tier enterprise"]
    STACK --> TEST["cargo test -p madhyamas-enterprise"]
    TEST --> SHOT["node scripts/capture-enterprise-screenshots.mjs"]
```

1. Build the frontend (`cd web && npm run build`) — assets are embedded at compile time
2. Build the Rust binary (`cargo build -p madhyamas`)
3. Start the enterprise stack (`./startup-local.sh --tier enterprise`)
4. Run enterprise tests (`cargo test -p madhyamas-enterprise`)
5. Capture screenshots for docs (`node scripts/capture-enterprise-screenshots.mjs`)

### Testing Enterprise Features

See [ENTERPRISE_TESTING.md](ENTERPRISE_TESTING.md) for the full testing guide,
including unit tests, integration tests with PostgreSQL, multi-instance
verification, and Playwright E2E tests.

### Key Enterprise Source Files

| Area | File | Purpose |
|------|------|---------|
| Enterprise crate | `crates/madhyamas-enterprise/src/` | All enterprise modules |
| API traits | `crates/madhyamas-api/src/auth.rs` | AuthProvider, Authorizer, AuditSink traits |
| Startup flow | `crates/madhyamas/src/main.rs` (lines 1290-1830) | Enterprise initialization |
| PostgreSQL traffic | `crates/madhyamas-core/src/storage/postgres/traffic.rs` | PostgreSQL TrafficStoreBackend |
| Enterprise store | `crates/madhyamas-enterprise/src/store/postgres.rs` | PostgreSQL EnterpriseStore |
| MCP enterprise tools | `crates/madhyamas-mcp/src/tools/enterprise.rs` | 11 enterprise MCP tools |
| CLI enterprise commands | `crates/madhyamas-cli/src/commands/enterprise.rs` | Enterprise CLI subcommands |
| Web admin panels | `web/src/features/admin/` | 6 admin panel components |
| Web auth | `web/src/features/auth/` | Auth context, login, protected app |
| Docker stack | `docker/docker-compose.multi.yml` | Multi-instance Compose file |

## See Also

- [ARCHITECTURE.md](ARCHITECTURE.md) — System architecture
- [WEB_FRONTEND.md](WEB_FRONTEND.md) — Frontend architecture and build flow
- [DEPLOYMENT.md](DEPLOYMENT.md) — Docker and production deployment
- [NETWORK_CONFIGURATION.md](NETWORK_CONFIGURATION.md) — Network setup
- [GETTING_STARTED.md](GETTING_STARTED.md) — User-facing getting started guide
- [ENTERPRISE_CRATE_GUIDE.md](ENTERPRISE_CRATE_GUIDE.md) — Enterprise crate developer guide
- [ENTERPRISE_STARTUP_FLOW.md](ENTERPRISE_STARTUP_FLOW.md) — Enterprise startup sequence
- [ENTERPRISE_TESTING.md](ENTERPRISE_TESTING.md) — Enterprise testing guide
- [STORAGE_BACKEND_GUIDE.md](STORAGE_BACKEND_GUIDE.md) — Storage backend implementation
