# ProxyForge - Claude AI Context

## Project Overview

**ProxyForge** is a high-performance, open-source HTTP/HTTPS debugging proxy built in Rust with a modern React-based web UI. It's designed as a free, cross-platform alternative to commercial tools like Charles Proxy and Fiddler.

### Key Characteristics
- **Language**: Rust (backend), TypeScript/React (frontend)
- **Architecture**: Workspace with 3 crates (core, api, cli)
- **License**: Dual MIT OR Apache-2.0
- **Rust Version**: 1.75+
- **Status**: Active development

## Project Structure

```
proxyforge/
├── crates/
│   ├── proxyforge-core/      # Core proxy engine, TLS, traffic storage
│   ├── proxyforge-api/       # REST/WebSocket API server (axum)
│   └── proxyforge-cli/       # CLI entry point
├── web/                      # React + TypeScript frontend (Vite)
├── docs/                     # Documentation
│   ├── ARCHITECTURE.md       # System architecture
│   ├── API.md               # API reference
│   ├── GETTING_STARTED.md   # User guide
│   ├── DEVELOPMENT.md       # Development guide
│   ├── DEPLOYMENT.md        # Deployment guide
│   └── CONTRIBUTING.md      # Contribution guidelines
├── packaging/               # Distribution packages
├── docker/                  # Docker setup
├── Cargo.toml              # Workspace configuration
├── README.md               # Main documentation
└── PRD-ProxyForge.md       # Product requirements

```

## Core Technologies

### Backend Stack
- **axum** (0.7) - Web framework for API server
- **hyper** (1.2) - HTTP client/server implementation
- **tokio** (1.36) - Async runtime
- **rustls** (0.23) - TLS/SSL implementation
- **rcgen** (0.13) - Certificate generation
- **rusqlite** (0.31) - SQLite for traffic storage
- **serde** (1.0) - Serialization/deserialization
- **clap** (4.5) - CLI argument parsing
- **tracing** (0.1) - Structured logging

### Frontend Stack
- **React 18** - UI framework
- **TypeScript** - Type safety
- **Vite** - Build tool
- **Tailwind CSS** - Styling
- **shadcn/ui** - Component library
- **TanStack Query** - Data fetching
- **Zustand** - State management

## Key Features

### Traffic Interception
- HTTP/HTTPS proxy on port 8888 (configurable)
- Automatic TLS certificate generation for HTTPS interception
- Real-time traffic capture and display
- WebSocket traffic capture
- gRPC/Protocol Buffer support

### Traffic Manipulation
- **Breakpoints**: Pause requests/responses for inspection and modification
- **Mocking**: Serve custom responses instead of hitting real servers
- **Rewriting**: Automatically modify URLs, headers, or bodies
- **Throttling**: Simulate network conditions (3G, 4G, DSL presets)
- **Replay**: Re-execute captured requests with modifications

### Advanced Features
- **Scripting**: JavaScript/TypeScript hooks for automation
- **Plugin System**: Rust-based plugin architecture
- **Session Management**: Save/load debugging sessions
- **Export**: HAR format, cURL commands
- **Persistence**: SQLite-based traffic storage

## Architecture Patterns

### Rust Best Practices Applied
1. **Error Handling**: Uses `thiserror` for custom error types, `Result<T>` throughout
2. **Async/Await**: Tokio-based async runtime for high concurrency
3. **Ownership**: Leverages Rust's ownership system for memory safety
4. **Type Safety**: Strong typing with minimal `unwrap()` usage
5. **Trait-Based Design**: Extensible architecture via traits
6. **Zero-Cost Abstractions**: Performance-critical paths optimized

### Code Organization
- **Separation of Concerns**: Core logic, API, and CLI are separate crates
- **Shared Dependencies**: Workspace-level dependency management
- **Modular Design**: Each feature in its own module
- **Test Coverage**: Unit tests in each module

## Development Workflow

### Building
```bash
# Build all crates
cargo build --release

# Build specific crate
cargo build -p proxyforge-core

# Run with logging
RUST_LOG=debug cargo run
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p proxyforge-core

# Run with output
cargo test -- --nocapture
```

### Code Quality
```bash
# Format code
cargo fmt --all

# Lint with clippy
cargo clippy --all-targets --all-features

# Check without building
cargo check
```

## Important Files & Modules

### Core Crate (`proxyforge-core`)
- `lib.rs` - Public API exports, error types
- `proxy/engine.rs` - Main proxy engine logic
- `tls/certificate.rs` - TLS certificate management
- `traffic/store.rs` - SQLite-based traffic storage
- `intercept/` - Breakpoints, mocks, rewrites, throttling
- `websocket.rs` - WebSocket frame parsing
- `grpc/` - gRPC frame parsing and management
- `session.rs` - Session save/load
- `replay.rs` - Request replay functionality
- `scripting/` - JavaScript runtime integration
- `plugin/` - Plugin system

### API Crate (`proxyforge-api`)
- `lib.rs` - API server setup
- `routes.rs` - Route definitions
- `handlers/` - Request handlers
- `websocket.rs` - WebSocket real-time updates

### CLI Crate (`proxyforge-cli`)
- `main.rs` - Entry point, CLI argument parsing

## Common Tasks

### Adding a New Feature
1. Add module in `proxyforge-core/src/`
2. Implement core logic with proper error handling
3. Add tests in the module
4. Expose public API in `lib.rs`
5. Add API endpoints in `proxyforge-api`
6. Update documentation

### Adding Dependencies
```toml
# In Cargo.toml workspace.dependencies section
new-dep = { version = "1.0", features = ["feature"] }

# Then reference in crate Cargo.toml
[dependencies]
new-dep.workspace = true
```

### Database Schema Changes
- Modify `traffic/store.rs` `create_tables()` method
- Add migration logic for existing databases
- Update related structs and queries

## Configuration

### Runtime Configuration
- **Proxy Port**: Default 8888, configurable via `--proxy-port`
- **API Port**: Default 3001, configurable via `--api-port`
- **Data Directory**: `~/.proxyforge/` by default
- **Certificate Path**: `~/.proxyforge/certs/`
- **Database**: `~/.proxyforge/traffic.db`

### Environment Variables
- `RUST_LOG` - Logging level (trace, debug, info, warn, error)
- `PROXYFORGE_CONFIG` - Custom config file path

## Performance Considerations

### Memory Management
- Traffic entries limited to 10,000 by default (configurable)
- Streaming for large request/response bodies
- Connection pooling for database access
- Efficient WebSocket frame buffering

### Concurrency
- Tokio async runtime handles 1000+ concurrent connections
- Lock-free data structures where possible
- RwLock for shared state (parking_lot for performance)
- Channel-based communication between components

## Security Model

### TLS/SSL
- Generates self-signed CA certificate on first run
- On-the-fly certificate generation for intercepted domains
- TLS 1.3 support for upstream connections
- Certificate pinning options

### Authentication (Enterprise Features)
- Optional API key authentication
- Role-based access control (RBAC)
- Audit logging for sensitive operations

## Testing Strategy

### Unit Tests
- Each module has inline tests
- Mock external dependencies
- Test error conditions

### Integration Tests
- End-to-end API tests
- Proxy functionality tests
- Database persistence tests

### Performance Tests
- Benchmark critical paths
- Load testing with multiple concurrent connections
- Memory profiling

## Deployment

### Docker
```bash
docker build -t proxyforge .
docker run -p 8888:8888 -p 3001:3001 proxyforge
```

### Binary Distribution
- Cross-compilation for Linux, macOS, Windows
- Packaged with web UI assets
- Installer scripts for each platform

## Troubleshooting

### Common Issues
1. **Certificate Errors**: Ensure CA cert is installed in system trust store
2. **Port Conflicts**: Check if ports 8888/3001 are available
3. **Database Locked**: Only one instance can access SQLite DB at a time
4. **Memory Usage**: Reduce max_requests setting if memory is constrained

### Debug Mode
```bash
RUST_LOG=debug cargo run -- --verbose
```

## Contributing Guidelines

### Code Style
- Follow Rust standard formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy`)
- Add tests for new features
- Update documentation

### Pull Request Process
1. Fork the repository
2. Create feature branch
3. Make changes with tests
4. Run `cargo test` and `cargo clippy`
5. Submit PR with clear description

## Roadmap

### Phase 1 (Current)
- ✅ Core proxy functionality
- ✅ Basic traffic interception
- ✅ Web UI
- ✅ Breakpoints, mocks, rewrites
- ✅ Session management

### Phase 2 (In Progress)
- 🔄 Enhanced gRPC support
- 🔄 JavaScript scripting runtime
- 🔄 Plugin system
- 🔄 Performance optimizations

### Phase 3 (Planned)
- 📋 Team collaboration features
- 📋 Cloud sync
- 📋 Advanced analytics
- 📋 Mobile app support

## Resources

### Documentation
- [README.md](README.md) - Quick start guide
- [ARCHITECTURE.md](docs/ARCHITECTURE.md) - System architecture
- [API.md](docs/API.md) - API reference
- [PRD-ProxyForge.md](PRD-ProxyForge.md) - Product requirements

### External Links
- Rust Book: https://doc.rust-lang.org/book/
- Tokio Docs: https://tokio.rs/
- Axum Docs: https://docs.rs/axum/
- React Docs: https://react.dev/

## AI Assistant Guidelines

When working on this project:

1. **Follow Rust Best Practices**: Use proper error handling, avoid `unwrap()` in production code, prefer `Result<T>` and `?` operator
2. **Maintain Type Safety**: Leverage Rust's type system, use `thiserror` for errors
3. **Async/Await**: All I/O operations should be async with Tokio
4. **Testing**: Add tests for new functionality
5. **Documentation**: Update docs when adding features
6. **Code Quality**: Run `cargo fmt` and `cargo clippy` before committing
7. **Performance**: Consider memory usage and concurrency patterns
8. **Security**: Be mindful of TLS/certificate handling and data validation

### When Adding Features
- Check existing patterns in similar modules
- Reuse workspace dependencies when possible
- Add proper error variants to `Error` enum in `lib.rs`
- Expose public API thoughtfully
- Consider backward compatibility

### When Debugging
- Check logs with `RUST_LOG=debug`
- Use `cargo test -- --nocapture` for test output
- Profile with `cargo flamegraph` for performance issues
- Verify database schema matches code expectations

### When Refactoring
- Ensure all tests pass
- Check for breaking API changes
- Update documentation
- Consider migration path for users
