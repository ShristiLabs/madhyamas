# Madhyamas Documentation

## Documentation Index

### For Users

| Document | Description |
|----------|-------------|
| [Getting Started](GETTING_STARTED.md) | Installation, configuration, CLI options, environment variables, mobile device setup, basic usage |
| [API Reference](API.md) | REST API endpoints, query parameters, WebSocket events |
| [MCP Integration & AI Agent Skills](MCP-INTEGRATION.md) | MCP server setup, Claude Desktop/Windsurf config, AI agent skills installation |
| [Android Cert Pinning](ANDROID_CERT_PINNING.md) | Bypassing certificate pinning on Android (Frida, APK patching, Magisk) |
| [Cert Pinning (Plain English)](CERT_PINNING_PLAIN_ENGLISH.md) | Non-technical explanation of certificate pinning for QA/testers/PMs |

### For Developers

| Document | Description |
|----------|-------------|
| [Development Guide](DEVELOPMENT.md) | Dev environment setup, project structure, tech stack, git hooks, coding standards |
| [Architecture](ARCHITECTURE.md) | System architecture overview, component design, data flow |
| [Proxy Flow](PROXY_FLOW.md) | End-to-end proxy flow with diagrams, TLS interception, request pipeline |
| [Tool Coverage](TOOL_COVERAGE.md) | Feature coverage matrix across Web UI, MCP, and CLI |
| [Build Optimization](BUILD_OPTIMIZATION.md) | CI/CD build performance optimization strategies and status |

### For DevOps

| Document | Description |
|----------|-------------|
| [Deployment Guide](DEPLOYMENT.md) | Docker, Kubernetes, cloud platform deployment, package managers |
| [Local Development](LOCAL_DEVELOPMENT.md) | Local dev setup with startup scripts |
| [Network Configuration](NETWORK_CONFIGURATION.md) | Network setup and IP detection |
| [IP Detection](IP_DETECTION.md) | Public IP detection for remote access |

### For AI Assistants

| Document | Description |
|----------|-------------|
| [CLAUDE.md](../CLAUDE.md) | AI assistant context — project overview, architecture, coding guidelines |
| [Skills Package](../skills/README.md) | AI agent skills (67 MCP tools, 58 CLI commands, 130+ API endpoints) |

### Other

| Document | Description |
|----------|-------------|
| [WebSocket Migration](websocket-migration.md) | WebSocket implementation migration notes |
| [Mock Responses](MOCK_RESPONSES.md) | Mock response feature documentation |
| [Web UI Redesign Plan](WEB_NEXT_REDESIGN_PLAN.md) | Completed web UI redesign reference |
| [Security](SECURITY.md) | Security considerations and best practices |

## Quick Start

```bash
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
cargo build --release
./target/release/madhyamas
```

Open `http://localhost:3001` for the web UI. Configure your browser to use `localhost:8888` as the proxy.

## External Links

- [GitHub Repository](https://github.com/ShristiLabs/madhyamas)
- [Issue Tracker](https://github.com/ShristiLabs/madhyamas/issues)
- [Discussions](https://github.com/ShristiLabs/madhyamas/discussions)
- [Releases](https://github.com/ShristiLabs/madhyamas/releases)

## License

Documentation is licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Code is dual-licensed under MIT OR Apache-2.0.
