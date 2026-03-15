# Madhyamas Documentation

Welcome to the Madhyamas documentation! This guide will help you find the information you need.

## 📚 Documentation Overview

### For Users

- **[Getting Started](GETTING_STARTED.md)** - Quick start guide for new users
  - Installation instructions
  - Basic configuration
  - First-time setup
  - Common use cases

- **[API Reference](API.md)** - Complete API documentation
  - REST endpoints
  - WebSocket events
  - Request/response formats
  - Query parameters

### For Developers

- **[Development Guide](DEVELOPMENT.md)** - Complete development setup
  - Environment setup
  - Project structure
  - Coding standards
  - Testing guidelines
  - Debugging tips

- **[Architecture](ARCHITECTURE.md)** - System architecture overview
  - Component design
  - Data flow
  - Technology stack
  - Performance characteristics

- **[Contributing](../CONTRIBUTING.md)** - How to contribute
  - Code of conduct
  - Development workflow
  - Pull request process
  - Community guidelines

### For DevOps

- **[Deployment Guide](DEPLOYMENT.md)** - Production deployment
  - Binary distribution
  - Docker deployment
  - Kubernetes setup
  - Cloud platforms (AWS, GCP, Azure)
  - Package managers
  - Monitoring and security

### For AI Assistants

- **[CLAUDE.md](../CLAUDE.md)** - AI assistant context
  - Project overview
  - Architecture patterns
  - Development guidelines
  - Common tasks
  - Troubleshooting

## 🚀 Quick Links

### Getting Started
```bash
# Install from source
git clone https://github.com/madhyamas/madhyamas.git
cd madhyamas
cargo build --release

# Run the proxy
./target/release/madhyamas

# Access web UI
open http://localhost:3001
```

### Common Tasks

| Task | Documentation |
|------|---------------|
| Install Madhyamas | [Getting Started](GETTING_STARTED.md#installation) |
| Configure proxy settings | [Getting Started](GETTING_STARTED.md#configuration) |
| Set up breakpoints | [Getting Started](GETTING_STARTED.md#set-breakpoints) |
| Export traffic as HAR | [API Reference](API.md#export) |
| Deploy to production | [Deployment Guide](DEPLOYMENT.md) |
| Contribute code | [Contributing](../CONTRIBUTING.md) |
| Report a bug | [GitHub Issues](https://github.com/madhyamas/madhyamas/issues) |

## 📖 Documentation Structure

```
docs/
├── README.md              # This file - documentation index
├── GETTING_STARTED.md     # User quick start guide
├── API.md                 # API reference
├── ARCHITECTURE.md        # System architecture
├── DEVELOPMENT.md         # Developer guide
└── DEPLOYMENT.md          # Deployment guide

Root directory:
├── CLAUDE.md              # AI assistant context
├── CONTRIBUTING.md        # Contribution guidelines
├── README.md              # Project overview
└── PRD-Madhyamas.md      # Product requirements
```

## 🎯 Documentation by Role

### I'm a User
1. Start with [Getting Started](GETTING_STARTED.md)
2. Learn about features in [README](../README.md)
3. Explore [API Reference](API.md) for automation

### I'm a Developer
1. Read [Development Guide](DEVELOPMENT.md)
2. Understand [Architecture](ARCHITECTURE.md)
3. Follow [Contributing Guidelines](../CONTRIBUTING.md)
4. Check [CLAUDE.md](../CLAUDE.md) for AI assistance

### I'm a DevOps Engineer
1. Review [Deployment Guide](DEPLOYMENT.md)
2. Check [Architecture](ARCHITECTURE.md) for infrastructure needs
3. Refer to [API Reference](API.md) for health checks

### I'm an AI Assistant
1. Start with [CLAUDE.md](../CLAUDE.md)
2. Reference [Development Guide](DEVELOPMENT.md) for coding standards
3. Use [Architecture](ARCHITECTURE.md) for system understanding

## 🔍 Finding Information

### By Topic

**Installation & Setup**
- [Getting Started - Installation](GETTING_STARTED.md#installation)
- [Deployment - Binary Distribution](DEPLOYMENT.md#binary-distribution)
- [Deployment - Docker](DEPLOYMENT.md#docker-deployment)

**Configuration**
- [Getting Started - Configuration](GETTING_STARTED.md#configuration)
- [Deployment - Production Configuration](DEPLOYMENT.md#production-configuration)

**Features**
- [README - Features](../README.md#features)
- [Getting Started - Basic Usage](GETTING_STARTED.md#basic-usage)
- [API Reference](API.md)

**Development**
- [Development - Setup](DEVELOPMENT.md#setting-up-development-environment)
- [Development - Coding Standards](DEVELOPMENT.md#coding-standards)
- [Development - Testing](DEVELOPMENT.md#running-tests)

**Architecture**
- [Architecture - Overview](ARCHITECTURE.md#overview)
- [Architecture - Data Flow](ARCHITECTURE.md#data-flow)
- [Architecture - Technology Stack](ARCHITECTURE.md#technology-stack)

**Deployment**
- [Deployment - Docker](DEPLOYMENT.md#docker-deployment)
- [Deployment - Kubernetes](DEPLOYMENT.md#kubernetes-deployment)
- [Deployment - Cloud Platforms](DEPLOYMENT.md#cloud-platform-deployments)

**Contributing**
- [Contributing - Getting Started](../CONTRIBUTING.md#getting-started)
- [Contributing - Pull Request Process](../CONTRIBUTING.md#pull-request-process)
- [Contributing - Coding Standards](../CONTRIBUTING.md#coding-standards)

## 🛠️ Technical Reference

### Core Technologies
- **Backend**: Rust 1.75+, Tokio, Axum, Hyper
- **Frontend**: React 18, TypeScript, Vite, Tailwind CSS
- **Storage**: SQLite (rusqlite)
- **TLS**: rustls, rcgen

### Key Modules
- `madhyamas-core` - Core proxy engine
- `madhyamas-api` - REST/WebSocket API
- `madhyamas-cli` - Command-line interface

### API Endpoints
See [API Reference](API.md) for complete list

### Configuration Files
- `~/.madhyamas/config.toml` - User configuration
- `Cargo.toml` - Rust workspace configuration
- `web/package.json` - Frontend dependencies

## 📝 Additional Resources

### External Links
- [GitHub Repository](https://github.com/madhyamas/madhyamas)
- [Issue Tracker](https://github.com/madhyamas/madhyamas/issues)
- [Discussions](https://github.com/madhyamas/madhyamas/discussions)
- [Releases](https://github.com/madhyamas/madhyamas/releases)

### Learning Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Documentation](https://docs.rs/axum/)
- [React Documentation](https://react.dev/)

### Related Projects
- [mitmproxy](https://mitmproxy.org/) - Python-based proxy
- [Charles Proxy](https://www.charlesproxy.com/) - Commercial alternative
- [Fiddler](https://www.telerik.com/fiddler) - Windows proxy tool

## 🤝 Community & Support

### Getting Help
1. Check this documentation
2. Search [GitHub Issues](https://github.com/madhyamas/madhyamas/issues)
3. Ask in [GitHub Discussions](https://github.com/madhyamas/madhyamas/discussions)
4. Join our Discord community

### Reporting Issues
- **Bugs**: [GitHub Issues](https://github.com/madhyamas/madhyamas/issues/new?template=bug_report.md)
- **Features**: [GitHub Issues](https://github.com/madhyamas/madhyamas/issues/new?template=feature_request.md)
- **Security**: Email security@madhyamas.dev

### Contributing
See [Contributing Guidelines](../CONTRIBUTING.md) for:
- Code of conduct
- Development workflow
- Pull request process
- Coding standards

## 📊 Documentation Status

| Document | Status | Last Updated |
|----------|--------|--------------|
| README.md | ✅ Complete | 2026-03-14 |
| CLAUDE.md | ✅ Complete | 2026-03-14 |
| CONTRIBUTING.md | ✅ Complete | 2026-03-14 |
| docs/GETTING_STARTED.md | ✅ Complete | 2026-03-14 |
| docs/API.md | ✅ Complete | 2026-03-14 |
| docs/ARCHITECTURE.md | ✅ Complete | 2026-03-14 |
| docs/DEVELOPMENT.md | ✅ Complete | 2026-03-14 |
| docs/DEPLOYMENT.md | ✅ Complete | 2026-03-14 |

## 🔄 Keeping Documentation Updated

When making changes to Madhyamas:

1. **Code Changes**: Update relevant technical documentation
2. **API Changes**: Update [API.md](API.md)
3. **New Features**: Update [README.md](../README.md) and [GETTING_STARTED.md](GETTING_STARTED.md)
4. **Architecture Changes**: Update [ARCHITECTURE.md](ARCHITECTURE.md)
5. **Deployment Changes**: Update [DEPLOYMENT.md](DEPLOYMENT.md)

## 📄 License

Documentation is licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).

Code is dual-licensed under MIT OR Apache-2.0.

---

**Need help?** Open an issue or start a discussion on GitHub!
