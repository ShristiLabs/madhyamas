# Development Guide

## Prerequisites

### Required Tools
- **Rust** 1.75 or later
- **Cargo** (comes with Rust)
- **Node.js** 18+ and npm
- **Git**

### Optional Tools
- **Docker** (for containerized development)
- **SQLite CLI** (for database inspection)
- **Postman/Insomnia** (for API testing)

## Setting Up Development Environment

### 1. Clone the Repository
```bash
git clone https://github.com/proxyforge/proxyforge.git
cd proxyforge
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
cargo build -p proxyforge-core
```

### 4. Set Up Frontend
```bash
cd web
npm install
npm run dev
```

## Project Structure

```
proxyforge/
├── crates/
│   ├── proxyforge-core/       # Core library
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
│   ├── proxyforge-api/        # API server
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── routes.rs      # Route definitions
│   │   │   └── handlers/      # Request handlers
│   │   └── Cargo.toml
│   └── proxyforge-cli/        # CLI
│       ├── src/
│       │   └── main.rs
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
RUST_LOG=proxyforge_core=debug,proxyforge_api=info cargo run
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
cargo test -p proxyforge-core

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

### 1. Core Feature in `proxyforge-core`

```bash
# Create new module
touch crates/proxyforge-core/src/my_feature.rs
```

```rust
// crates/proxyforge-core/src/my_feature.rs
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
// crates/proxyforge-core/src/lib.rs
pub mod my_feature;
pub use my_feature::MyFeature;
```

### 2. API Endpoint in `proxyforge-api`

```rust
// crates/proxyforge-api/src/handlers/my_feature_handlers.rs
use axum::{extract::State, Json};
use proxyforge_core::MyFeature;
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
// crates/proxyforge-api/src/routes.rs
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
// crates/proxyforge-core/src/traffic/store.rs
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
RUST_LOG=proxyforge_core::proxy=trace cargo run

# Use rust-lldb or rust-gdb
rust-lldb target/debug/proxyforge
```

### Frontend Debugging
- Use browser DevTools
- React DevTools extension
- Network tab for API calls
- Console for errors

### Database Inspection

```bash
# Open SQLite database
sqlite3 ~/.proxyforge/traffic.db

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
cargo flamegraph --bin proxyforge

# Open flamegraph.svg in browser
```

### Memory Profiling
```bash
# Use valgrind
valgrind --tool=massif target/release/proxyforge

# Or heaptrack
heaptrack target/release/proxyforge
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
cargo run -- --db-path /tmp/proxyforge.db
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
strip target/release/proxyforge

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
