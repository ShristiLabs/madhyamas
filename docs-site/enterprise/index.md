---
title: Enterprise Overview
description: Madhyamas Enterprise adds authentication, RBAC, audit logging, user management, licensing, and multi-instance deployment for team and production environments.
---

# Enterprise Overview

Madhyamas Enterprise extends the open-source debugging proxy with authentication, role-based access control, audit logging, user management, performance monitoring, licensing, and multi-instance deployment. These features are designed for teams, production environments, and compliance-sensitive deployments.

![Enterprise login screen](/screenshots/enterprise-login.png)

## When to Use Enterprise

| Scenario | Why Enterprise |
|----------|---------------|
| **Team deployments** | Multiple developers share one proxy instance with per-user accountability |
| **Production/shared debugging** | Proxy is exposed beyond loopback and needs access control |
| **Compliance environments** | Audit trails of who exported traffic or changed configuration |
| **CI/CD integration** | Long-lived API keys for automated pipelines |
| **Multi-instance scaling** | Load-balanced instances sharing a single database and CA |
| **Enterprise SSO** | Integrate with Okta, Auth0, Keycloak, Active Directory via OIDC/LDAP/SAML |

For single-developer, loopback-only usage, the OSS tier is sufficient — the proxy works without auth by default.

## OSS vs Enterprise Feature Matrix

| Feature | OSS | Enterprise |
|---------|-----|-----------|
| HTTP/HTTPS proxy & TLS interception | ✅ | ✅ |
| Traffic recording & sessions | ✅ | ✅ |
| Breakpoints, mocks, rewrites, throttle | ✅ | ✅ |
| Block list & focus hosts | ✅ | ✅ |
| Scripting (JavaScript) & plugins (WASM) | ✅ | ✅ |
| gRPC, WebSocket, SOCKS5, upstream proxy | ✅ | ✅ |
| HAR import/export, auto save, mirror | ✅ | ✅ |
| CLI (159 subcommands) & MCP (146 tools) | ✅ | ✅ |
| SQLite storage | ✅ | ✅ |
| PostgreSQL storage | — | ✅ |
| JWT + API key authentication | — | ✅ |
| User management (CRUD, roles) | — | ✅ |
| Role-based access control (RBAC) | — | ✅ |
| Audit logging (hash-chained, persistent) | — | ✅ |
| Performance metrics dashboard | — | ✅ |
| License verification (Ed25519) | — | ✅ |
| Multi-instance deployment (Redis pub/sub) | — | ✅ |
| Shared CA certificate | — | ✅ |
| License seat coordination | — | ✅ |
| SSO (OIDC, LDAP, SAML, header-based) | — | ✅ |
| MFA (TOTP) | — | ✅ |
| Cluster metrics & instance registry | — | ✅ |
| Configuration import/export | — | ✅ |
| Enterprise admin panels (web UI) | — | ✅ |
| Enterprise CLI commands | — | ✅ |
| Enterprise MCP tools | — | ✅ |

## How to Enable Enterprise

Enterprise is enabled at **build time** via the `enterprise` Cargo feature (included by default) and at **runtime** via CLI flags.

### Build

```bash
# Enterprise (default)
cargo build --release -p madhyamas

# OSS (no enterprise features)
cargo build --release --no-default-features -p madhyamas
```

### Runtime

```bash
madhyamas \
  --database-url postgres://madhyamas:password@localhost:5432/madhyamas \
  --enable-auth \
  --jwt-secret your-secret-key \
  --admin-username admin \
  --admin-password your-secure-password \
  --license-file /path/to/license.json
```

See [Getting Started with Enterprise](./getting-started) for a complete setup guide.

## Enterprise Documentation Sections

| Section | Audience | Description |
|---------|----------|-------------|
| [Getting Started](./getting-started) | Operators | First-run setup, database, admin bootstrap, license installation |
| [Authentication](./authentication) | Admins, Developers | JWT, API keys, SSO (OIDC/LDAP/SAML), MFA, proxy auth |
| [User Management](./user-management) | Admins | Creating, editing, deleting users; role assignment; password reset |
| [Role-Based Access Control](./rbac) | Admins | Roles, permissions, resource types, enforcement |
| [Audit Logging](./audit-logging) | Admins, Compliance | Event types, hash chain, querying, export, compliance |
| [Performance & Monitoring](./monitoring) | Admins, Operators | Metrics dashboard, cluster metrics, health checks |
| [Licensing](./licensing) | Admins, Operators | License installation, seat management, renewal, pricing |
| [Multi-Instance Deployment](./deployment) | Operators, DevOps | Docker Compose, nginx, PostgreSQL, Redis, K8s, shared CA |
| [Configuration](./configuration) | Operators | All enterprise CLI flags, env vars, production checklist |
| [CLI & MCP Tools](./cli-mcp) | Developers, Admins | Enterprise CLI commands, MCP tools, authenticated agent access |

## Admin Panels

The web UI includes six admin panels accessible from the navigation rail:

| Panel | Purpose |
|-------|---------|
| **Users** | Manage user accounts, roles, and status |
| **Audit Log** | View, filter, and export audit events |
| **Metrics** | Real-time performance and cluster metrics |
| **License** | View license details and seat usage |
| **API Keys** | Create, manage, and revoke API keys |
| **Instances** | View active instances in a multi-instance cluster |

![Enterprise user management panel](/screenshots/enterprise-users-panel.png)

## See Also

- [Getting Started with Enterprise](./getting-started) — first-run setup guide
- [Configuration](./configuration) — all CLI flags and environment variables
- [Security Overview](../security) — security model and best practices
- [REST API Reference](../rest-api) — full endpoint reference
