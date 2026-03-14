# ProxyForge Architecture

## Overview

ProxyForge is a Rust-based HTTP/HTTPS debugging proxy with a modern web-based UI.

## Core Components

### Backend (Rust)
- **proxyforge-core**: Core proxy logic, traffic storage, TLS handling
- **proxyforge-api**: REST API and WebSocket server
- **proxyforge-cli**: Command-line interface

### Frontend (React + TypeScript)
- Built with Vite, React 18, TypeScript, Tailwind CSS
- Real-time traffic updates via WebSocket
- Component library: shadcn/ui

## Data Flow

```
┌────────────────┐      ┌────────────────┐      ┌────────────────┐
│   Browser/     │      │   ProxyForge   │      │   Target       │
│   Mobile App  │─────▶│     Proxy     │─────▶│    Server     │
└────────────────┘      │   :8888      │      └────────────────┘
                              │
                              ▼
                     ┌────────────────┐
                     │   Web UI (React)   │
                     │   :3000/ws          │
                     └────────────────┘
```

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Proxy Engine | hyper, tokio | HTTP interception |
| TLS | rustls | Certificate management |
| API | axum | REST/WebSocket server |
| Storage | sqlite | Traffic persistence |
| Frontend | React,18 + User interface |

## Directory Structure

```
proxyforge/
├── crates/
│   ├── proxyforge-core/      # Core library
│   ├── proxyforge-api/       # API handlers
│   └── proxyforge-cli/       # CLI entry point
├── web/                     # React frontend
├── docs/                    # Documentation
└── tests/                   # Test suites
```

## Performance Consider

- **Memory**: < 500MB under normal load
- **Latency**: < 10ms proxy overhead
- **Concurrency**: 1000+ simultaneous connections
- **Throughput**: 10,000+ requests/second

## Security Model

- TLS 1.3+ for upstream connections
- mTLS for client connections
- Certificate pinning for CA trust
- Optional API key authentication
- Role-based access control (RBAC)

## Extension Points

1. **Scripts**: JavaScript hooks for traffic modification
2. **Plugins**: Rust plugins for custom protocols
3. **gRPC**: Protocol buffer introspection

## Deployment Options

1. **Docker**: Containerized deployment
2. **Binary**: Standalone executable
3. **Package**: Homebrew, AUR, Snap
