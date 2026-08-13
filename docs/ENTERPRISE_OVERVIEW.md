# Enterprise Analysis Overview

> **Master document** for the Madhyamas enterprise tier analysis. This
> document provides the high-level architecture, two-tier model, and
> cross-references to detailed sub-documents.

The enterprise tier adds authentication, authorization, audit logging,
user management, and SSO to Madhyamas — features required by
organizations that need multi-user access control and compliance. This
analysis covers the full design: distribution model, crate
architecture, database strategy, licensing, auth/RBAC, and the
licensing server infrastructure.

## Sub-documents

| Document | Scope | Lines |
|---|---|---|
| **This document** | Two-tier model, crate architecture, database strategy overview, licensing overview, current state, what's missing, roadmap, risk analysis | ~1200 |
| [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md) | Full SaaS licensing server: account management, Stripe payments, license issuance/revocation, support tickets, admin dashboard, email notifications | ~1660 |
| [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) | Shared storage trait design: sync/async obstacle, trait definitions, dual SQLite/PostgreSQL backends, rusqlite → sqlx migration plan | ~720 |
| [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) | Proxy-side authentication, authorization (RBAC), and external identity provider integration (OIDC, header, LDAP, SAML) | ~320 |
| [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md) | Enterprise web UI: same-folder runtime-gated approach, tier detection, auth UI, admin panels, build/embedding | ~680 |
| [ENTERPRISE_CICD.md](ENTERPRISE_CICD.md) | CI/CD changes for two-tier builds: CI matrix, release workflow, Docker, licensing server pipeline, secrets | ~680 |
| [ENTERPRISE_MULTI_INSTANCE.md](ENTERPRISE_MULTI_INSTANCE.md) | Multi-instance deployment: load balancer routing, state sync (PostgreSQL + Redis), atomic config propagation, shared CA, license seat tracking, K8s manifests | ~1100 |
| [ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md) | Comprehensive performance and security analysis: threat model, 16 security gaps with remediations, 10 performance bottlenecks, 16 database optimizations for high-volume traffic (tiered storage, batching, indexing, partitioning), checklists | ~2200 |
| [ENTERPRISE_OSS_COMPARISON.md](ENTERPRISE_OSS_COMPARISON.md) | Side-by-side OSS vs Enterprise comparison: architecture, feature parity matrix (42 shared + 17 enterprise-only), build/distribution, database, deployment, security, performance, web UI, CLI/MCP, pricing, upgrade path, FAQ | ~1100 |
| [ENTERPRISE_AI_AGENTS.md](ENTERPRISE_AI_AGENTS.md) | AI agent integration for enterprise: gap analysis (MCP/CLI/API auth), MCP server changes (auth, HTTP transport, enterprise tools, dynamic resources, prompts, annotations), API key middleware, RBAC scopes, multi-instance agent access, agent workflows, security, implementation plan | ~1000 |
| [ENTERPRISE_CRATE_MIGRATION.md](ENTERPRISE_CRATE_MIGRATION.md) | Detailed migration analysis for extracting the madhyamas-enterprise crate: inventory of all enterprise code (859 lines in core, 742 in api), all 17 #[cfg] gates, dependency analysis, cross-crate reference map, trait abstractions, AppState changes, 6-phase migration plan, risk assessment | ~900 |
| [ENTERPRISE_IMPLEMENTATION_PLAN.md](ENTERPRISE_IMPLEMENTATION_PLAN.md) | Comprehensive implementation plan synthesizing all 12 analysis docs: 13 phases (0-12), dependency graph, critical path, per-phase steps with files and exit criteria, milestone summary, Gantt chart, effort estimates (194 dev-days / ~6mo with 2 devs), risk register, verification checklist | ~1100 |

---

## Table of Contents

1. [Two-Tier Distribution Model](#1-two-tier-distribution-model)
2. [Crate Architecture: Separate madhyamas-enterprise Crate](#2-crate-architecture-separate-madhyamas-enterprise-crate)
3. [Database Strategy: SQLite and PostgreSQL](#3-database-strategy-sqlite-and-postgresql)
4. [Registration, Attestation, and Licensing](#4-registration-attestation-and-licensing)
5. [Current State: What Is Built](#5-current-state-what-is-built)
6. [What Is Missing](#6-what-is-missing)
7. [Web UI Strategy](#7-web-ui-strategy)
8. [Plugin vs Flag vs Crate Architecture](#8-plugin-vs-flag-vs-crate-architecture)
9. [Implementation Roadmap](#9-implementation-roadmap)
10. [Risk Analysis](#10-risk-analysis)

---

## 1. Two-Tier Distribution Model

Madhyamas is distributed in two tiers. The **Simple** tier is the default
open-source build with no enterprise code. The **Enterprise** tier is a
separate build that includes enterprise features, activated only when a
valid license is presented at startup.

### Design principles

- **Simple tier must be genuinely simple.** No enterprise code, no
  auth middleware, no login screen, no license checks. A solo developer
  running `cargo build` or downloading the OSS binary gets a clean,
  unauthenticated debugging proxy identical to today's experience.
- **Enterprise tier requires registration.** Organizations cannot
  accidentally or silently enable enterprise features. A valid license
  issued through a registration process must be present at startup.
- **One codebase, two builds.** Both tiers are built from the same
  source tree. Enterprise code lives in a separate `madhyamas-enterprise`
  crate (see [Section 2](#2-crate-architecture-separate-madhyamas-enterprise-crate)),
  not scattered behind `#[cfg]` annotations in core and API crates.
  The main binary conditionally depends on this crate. A runtime license
  check gates activation even when enterprise code is compiled in.
- **No telemetry in either tier.** The Simple tier has no phone-home.
  The Enterprise tier verifies the license offline by default; an
  optional online revocation check is opt-in only.

### Tier comparison

| Aspect | Simple (OSS) | Enterprise |
|---|---|---|
| Cargo feature | `--no-default-features` (or default without `enterprise`) | `--features enterprise` |
| Enterprise code compiled | No | Yes |
| License required | No | Yes (Ed25519-signed license file) |
| Authentication | None (local trust) | JWT + API keys, or external IdP |
| RBAC | Not available | Enforced per-route |
| Audit logging | Not available | Persisted to PostgreSQL (or SQLite) |
| User management | Not available | Full CRUD, password hashing |
| Multi-user | Single implicit user | Named users with roles |
| Web UI | No login page | Login, user menu, admin panels |
| Metrics dashboard | Basic (existing) | Full (wired to PerformanceMonitor) |
| SSO/OIDC | Not available | Optional, via config |
| Support | Community | Priority (via registration) |

### Cargo feature changes required

Currently `enterprise` is in the **default** feature set of all three
crates, and enterprise code is intermingled with core/API code behind
`#[cfg(feature = "enterprise")]` annotations (17+ sites across
`madhyamas-core` and `madhyamas-api`). This must change so the default
build is the Simple tier with zero enterprise code.

The proposed approach is a **separate `madhyamas-enterprise` crate**
(see [Section 2](#2-crate-architecture-separate-madhyamas-enterprise-crate))
rather than feature-gating within existing crates. This eliminates all
`#[cfg(feature = "enterprise")]` annotations from `madhyamas-core` and
`madhyamas-api` entirely.

```toml
# crates/madhyamas-core/Cargo.toml  (CURRENT — needs change)
default = ["grpc", "scripting", "plugins", "wasm-runtime", "enterprise"]

# PROPOSED — enterprise removed entirely from core
default = ["grpc", "scripting", "plugins", "wasm-runtime"]
# No enterprise feature at all in core. Enterprise code moves to
# the separate madhyamas-enterprise crate.

# crates/madhyamas-api/Cargo.toml  (CURRENT — needs change)
default = ["grpc", "scripting", "plugins", "enterprise", "embedded-assets"]

# PROPOSED — enterprise removed entirely from api
default = ["grpc", "scripting", "plugins", "embedded-assets"]
# No enterprise feature. Auth trait definitions stay in api (not
# gated). Enterprise implementations move to madhyamas-enterprise.

# crates/madhyamas/Cargo.toml  (CURRENT — needs change)
default = ["grpc", "scripting", "plugins", "wasm-runtime", "enterprise"]

# PROPOSED — enterprise is an optional dependency on the new crate
default = ["grpc", "scripting", "plugins", "wasm-runtime"]
enterprise = ["dep:madhyamas-enterprise"]
```

The `enterprise` feature in the main binary crate simply adds
`madhyamas-enterprise` as a dependency. Core and API crates have no
enterprise feature at all.

### Build commands

```bash
# Simple tier (default — what most users build)
# madhyamas-enterprise crate is not compiled; zero enterprise code in binary
cargo build --release -p madhyamas

# Enterprise tier (organizations, CI for enterprise releases)
# madhyamas-enterprise crate is compiled and linked
cargo build --release -p madhyamas --features enterprise

# Verify which tier a binary supports
madhyamas --version
# Simple:  "madhyamas 0.1.6 (community)"
# Enterprise: "madhyamas 0.1.6 (enterprise — license required)"
```

### Distribution

- **Simple binary:** Published to GitHub Releases, `cargo install
  madhyamas`, Homebrew. No license, no registration, no restrictions.
  The `madhyamas-enterprise` crate is not a dependency — it is not
  compiled, not linked, not present in the binary at all.
- **Enterprise binary:** Published to a separate release channel (or
  built by the organization from source with `--features enterprise`).
  Requires a license file placed at `~/.madhyamas/license.json` or
  specified via `--license-file`. The `madhyamas-enterprise` crate can
  be published to a private registry, distributed as source in the repo,
  or kept out of `crates.io` entirely.

---

## 2. Crate Architecture: Separate madhyamas-enterprise Crate

### 2.1 Analysis: current approach vs separate crate

The current codebase uses `#[cfg(feature = "enterprise")]` annotations
scattered across `madhyamas-core` and `madhyamas-api`. An audit found
**17 annotation sites** across these crates:

| File | `#[cfg]` sites | What's gated |
|---|---|---|
| `madhyamas-core/src/enterprise/mod.rs` | 1 | Module existence |
| `madhyamas-core/src/enterprise/auth.rs` | 1 | `AuthManager`, `AuthConfig`, `JwtClaims`, `ApiKey` |
| `madhyamas-core/src/enterprise/rbac.rs` | 1 | `RbacManager`, `UserRole`, `ResourceType`, `Permission` |
| `madhyamas-core/src/enterprise/audit.rs` | 1 | `AuditLogger`, `AuditEvent`, `AuditFilter` |
| `madhyamas-core/src/enterprise/user.rs` | 1 | `User`, `UserManager` |
| `madhyamas-core/src/lib.rs` | 1 | `pub mod enterprise` export |
| `madhyamas-api/src/middleware.rs` | 2 | `auth_middleware`, `require_permission_middleware` |
| `madhyamas-api/src/routes.rs` | 2 | `create_routes_with_enterprise()` |
| `madhyamas-api/src/enterprise_handlers.rs` | 1 | All enterprise handlers (30+ stubs) |
| `madhyamas-api/src/lib.rs` | 2 | `AppState.auth_service`, `with_auth_service()` |
| `madhyamas-api/src/auth.rs` | 1 | `AuthUser` extractor |
| `madhyamas-cli/src/commands/enterprise.rs` | 1 | Enterprise CLI subcommands |
| `madhyamas-cli/src/lib.rs` | 1 | Enterprise command enum variant |
| `madhyamas-mcp/src/tools/enterprise.rs` | 1 | Enterprise MCP tools |

**Verdict: Yes, a separate crate is the right approach.** The
annotations are spread across 14 files in 4 crates. This is not a
clean feature gate — it's a cross-cutting concern that has leaked into
every layer. A separate crate provides a structural guarantee that the
simple build has zero enterprise code.

### 2.2 Pros and cons of a separate crate

| Aspect | `#[cfg]` annotations (current) | Separate crate (proposed) |
|---|---|---|
| Simple build isolation | Convention only — `#[cfg]` can be missed | Structural — enterprise code is not compiled |
| Dependency isolation | Enterprise deps (`argon2`, `openidconnect`) are in core/api Cargo.toml | Enterprise deps are only in `madhyamas-enterprise/Cargo.toml` |
| License isolation | All code is MIT/Apache (enterprise code too) | Enterprise crate can carry a different license (BSL, commercial) |
| Code clarity | Enterprise code is interleaved with core code | Enterprise code is in its own directory tree |
| Build complexity | One workspace, features toggle code | One additional crate in the workspace |
| Testing | Must test with and without feature | Simple build tests don't need enterprise at all |
| `AppState` complexity | `#[cfg]` fields on `AppState` | `Option<Arc<dyn Trait>>` fields (clean, no cfg) |

### 2.3 Proposed crate structure

```
crates/
├── madhyamas/                    # Main binary
├── madhyamas-core/               # Core proxy engine (no enterprise code)
├── madhyamas-api/                # REST/WebSocket API (traits only, no enterprise impl)
├── madhyamas-cli/                # CLI library
├── madhyamas-mcp/                # MCP server library
├── madhyamas-plugin-sdk/         # Plugin SDK
└── madhyamas-enterprise/         # NEW: Enterprise features (separate crate)
    ├── Cargo.toml
    ├── src/
    │   ├── lib.rs                # Public API: EnterpriseState, init_enterprise()
    │   ├── license.rs            # License verification (Ed25519)
    │   ├── auth/                 # AuthManager, JWT, API keys, argon2
    │   ├── rbac/                 # RbacManager, role/permission matrix
    │   ├── audit/                # AuditLogger, audit event persistence
    │   ├── user/                 # UserManager, user CRUD
    │   ├── store/                # EnterpriseStore trait + Pg/SQLite impls
    │   ├── handlers/             # Enterprise API handlers (auth, users, rbac, audit)
    │   ├── middleware/           # auth_middleware, require_permission_middleware
    │   ├── router.rs             # Enterprise router (merged with core router at startup)
    │   └── oidc/                 # OIDC/SSO integration (optional feature)
    └── migrations/               # SQL migrations for enterprise tables
```

### 2.4 Trait abstractions

The enterprise crate implements traits defined in `madhyamas-api`.
These traits are **not enterprise-specific** — they are generic
interfaces that the simple tier can also implement (as no-ops or mocks):

```rust
// crates/madhyamas-api/src/auth.rs (NEW — always present, not gated)

use async_trait::async_trait;
use std::sync::Arc;

/// Authentication provider — implemented by madhyamas-enterprise.
/// Simple tier sets this to None.
#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<Identity>;
    async fn validate_api_key(&self, key: &str) -> Result<Identity>;
    async fn issue_token(&self, identity: &Identity) -> Result<String>;
    async fn revoke_token(&self, token: &str) -> Result<()>;
}

/// Authorization provider — implemented by madhyamas-enterprise.
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn check_permission(
        &self,
        identity: &Identity,
        resource: ResourceType,
        permission: Permission,
    ) -> Result<bool>;
}

/// Audit sink — implemented by madhyamas-enterprise.
/// Simple tier sets this to None (audit calls are no-ops).
#[async_trait]
pub trait AuditSink: Send + Sync {
    async fn log(&self, event: AuditEvent) -> Result<()>;
    async fn query(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>>;
}

/// Authenticated identity (extracted from JWT or API key).
pub struct Identity {
    pub user_id: String,
    pub username: String,
    pub role: UserRole,
    pub auth_method: AuthMethod,
}
```

### 2.5 AppState changes

```rust
// crates/madhyamas-api/src/lib.rs (PROPOSED)

pub struct AppState {
    // Core stores (always present) — see ENTERPRISE_STORAGE_TRAITS.md
    pub traffic_store: Arc<dyn TrafficStoreBackend>,
    pub config_store: Option<Arc<dyn ConfigStoreBackend>>,
    pub intercept_store: Option<Arc<dyn InterceptStoreBackend>>,

    // Enterprise providers (None in simple tier, Some in enterprise)
    pub auth_provider: Option<Arc<dyn AuthProvider>>,
    pub authorizer: Option<Arc<dyn Authorizer>>,
    pub audit_sink: Option<Arc<dyn AuditSink>>,

    // ... other fields unchanged (session_manager, ws_manager, etc.)
}
```

No `#[cfg]` annotations. The simple tier constructs `AppState` with
`None` for enterprise fields. The enterprise tier constructs them with
real implementations.

### 2.6 Integration in the main binary

```rust
// crates/madhyamas/src/main.rs

#[cfg(feature = "enterprise")]
async fn build_app_state(/* ... */) -> AppState {
    // Verify license
    let license = madhyamas_enterprise::license::verify(&license_path)
        .unwrap_or_else(|e| {
            eprintln!("License verification failed: {}", e);
            std::process::exit(1);
        });

    // Initialize enterprise state (database, auth, rbac, audit)
    let enterprise = madhyamas_enterprise::init_enterprise(&license, &db_config).await;

    // Build AppState with enterprise providers
    AppState::new(traffic_store)
        .with_auth_provider(enterprise.auth_provider)
        .with_authorizer(enterprise.authorizer)
        .with_audit_sink(enterprise.audit_sink)
}

#[cfg(not(feature = "enterprise"))]
async fn build_app_state(/* ... */) -> AppState {
    // Simple tier — no enterprise providers
    AppState::new(traffic_store)
}
```

### 2.7 Dependency graph

```mermaid
graph TD
    CORE["madhyamas-core<br/>(proxy engine, storage traits)"]
    API["madhyamas-api<br/>(REST API, auth traits)"]
    CLI["madhyamas-cli"]
    MCP["madhyamas-mcp"]
    SDK["madhyamas-plugin-sdk"]
    MAIN["madhyamas<br/>(main binary)"]
    ENT["madhyamas-enterprise<br/>(auth, rbac, audit, license)"]

    MAIN --> CORE
    MAIN --> API
    MAIN --> CLI
    MAIN --> MCP
    MAIN -.->|"--features enterprise"| ENT
    API --> CORE
    CLI --> API
    MCP --> API
    ENT --> API
    ENT --> CORE

    style ENT stroke-dasharray: 5 5
```

The dashed line from `madhyamas` to `madhyamas-enterprise` indicates
it's an optional dependency, only enabled with `--features enterprise`.

### 2.8 What moves out of existing crates

| From | To | What |
|---|---|---|
| `madhyamas-core/src/enterprise/` | `madhyamas-enterprise/src/auth/`, `rbac/`, `audit/`, `user/` | All enterprise types and logic |
| `madhyamas-api/src/enterprise_handlers.rs` | `madhyamas-enterprise/src/handlers/` | All 30+ enterprise handler stubs |
| `madhyamas-api/src/middleware.rs` (enterprise parts) | `madhyamas-enterprise/src/middleware/` | `auth_middleware`, `require_permission_middleware` |
| `madhyamas-api/src/routes.rs` (enterprise parts) | `madhyamas-enterprise/src/router.rs` | `create_routes_with_enterprise()` |
| `madhyamas-cli/src/commands/enterprise.rs` | `madhyamas-enterprise/src/cli.rs` | Enterprise CLI subcommands |
| `madhyamas-mcp/src/tools/enterprise.rs` | `madhyamas-enterprise/src/mcp.rs` | Enterprise MCP tools |

### 2.9 What stays in existing crates

| Stays in | What | Why |
|---|---|---|
| `madhyamas-api/src/auth.rs` | `AuthProvider`, `Authorizer`, `AuditSink` traits, `Identity`, `AuthMethod` | These are generic interfaces, not enterprise-specific. Simple tier can implement them too (as no-ops). |
| `madhyamas-api/src/lib.rs` | `AppState` with `Option<Arc<dyn trait>>` fields | The `Option` handles the presence/absence of enterprise without `#[cfg]`. |
| `madhyamas-core/src/storage/` | Storage traits (`TrafficStoreBackend`, etc.) | These are core abstractions, not enterprise-specific. See [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md). |

### 2.10 License considerations for the crate

The `madhyamas-enterprise` crate can carry a **different license** from
the rest of the codebase:

| File | License |
|---|---|
| `crates/madhyamas-core/` | MIT OR Apache-2.0 (unchanged) |
| `crates/madhyamas-api/` | MIT OR Apache-2.0 (unchanged) |
| `crates/madhyamas-enterprise/` | BSL 1.1 or proprietary (new) |
| `crates/madhyamas/` (main binary) | MIT OR Apache-2.0 (the `enterprise` feature pulls in BSL-licensed code, but the default build is pure MIT/Apache) |

This means the **default build** (`cargo build -p madhyamas`) produces
a binary licensed under MIT OR Apache-2.0 with zero BSL code. The
enterprise build produces a binary that includes BSL-licensed code
from `madhyamas-enterprise`.

---

## 3. Database Strategy: SQLite and PostgreSQL

### 3.1 Current state

All persistence in Madhyamas uses `rusqlite` (synchronous SQLite
bindings). There are 8 files with `rusqlite` usage totaling **103
references** across ~3,970 lines of code. The stores are:

| Store | File | rusqlite refs | Lines |
|---|---|---|---|
| Traffic | `traffic/store.rs` | 35 | ~1700 |
| Intercept | `persistence/intercept_store.rs` | 22 | ~600 |
| Scripting | `scripting/persistence.rs` | 20 | ~500 |
| Plugin | `plugin/persistence.rs` | 13 | ~350 |
| Config | `persistence/config_store.rs` | 7 | ~220 |
| Session | `session.rs` | 1 | ~200 |
| Scripting runtime | `scripting/runtime.rs` | 2 | ~400 |
| Lib init | `lib.rs` | 3 | — |

### 3.2 Why PostgreSQL for enterprise

- **Concurrent writes:** SQLite serializes all writes through a single
  global lock. PostgreSQL uses MVCC for concurrent read/write.
- **Network-accessible:** PostgreSQL is a server — multiple proxy
  instances can share one database. SQLite is a file — only one
  process can write at a time.
- **Audit at scale:** Enterprise audit tables can grow to millions of
  rows. PostgreSQL handles this with partitioning, partial indexes, and
  GIN indexes for JSONB queries.
- **Enterprise standard:** PostgreSQL is the expected database for
  enterprise software. SQLite is seen as "embedded" or "toy."
- **Backup and replication:** PostgreSQL has streaming replication,
  point-in-time recovery, and managed backup. SQLite requires file
  copying (which can corrupt if the database is being written to).

### 3.3 Proposed database tiering

| Tier | Database | Library | Stores |
|---|---|---|---|
| Simple | SQLite | `sqlx::SqlitePool` | traffic, config, intercept, plugins, scripts, sessions |
| Enterprise (default) | PostgreSQL | `sqlx::PgPool` | users, api_keys, auth_sessions, audit_events, license_cache + optionally traffic/config/intercept |
| Enterprise (small) | SQLite | `sqlx::SqlitePool` | All stores in one SQLite file (single-instance, small team) |

### 3.4 Why sqlx

- **Async-native:** Built on tokio, integrates with axum's async
  handlers. No `spawn_blocking` needed.
- **Multi-backend:** Supports both PostgreSQL and SQLite with the same
  API. The same trait can be implemented for both backends.
- **Compile-time checking:** `sqlx::query!` macro validates SQL at
  compile time against the database schema.
- **Migrations:** Built-in migration system (`sqlx::migrate!`).
- **No ORM overhead:** Direct SQL with type-safe row mapping. No
  magic, no N+1 problems.
- **Connection pooling:** Built-in `Pool` type with configurable
  min/max connections, idle timeout, acquire timeout.

### 3.5 What uses which database

```mermaid
graph TD
    subgraph "Simple tier (sqlx::SqlitePool)"
        S_TRAFFIC["traffic (SQLite)<br/>SqliteTrafficStore"]
        S_CONFIG["config (SQLite)<br/>SqliteConfigStore"]
        S_INTERCEPT["intercept rules (SQLite)<br/>SqliteInterceptStore"]
        S_PLUGINS["plugins (SQLite)<br/>SqlitePluginStore"]
        S_SCRIPTS["scripts (SQLite)<br/>SqliteScriptStore"]
    end

    subgraph "Enterprise tier (sqlx::PgPool or sqlx::SqlitePool)"
        E_USERS["users<br/>PgEnterpriseStore / SqliteEnterpriseStore"]
        E_KEYS["api_keys<br/>PgEnterpriseStore / SqliteEnterpriseStore"]
        E_AUTH_SESSIONS["auth_sessions<br/>PgEnterpriseStore / SqliteEnterpriseStore"]
        E_AUDIT["audit_events<br/>PgEnterpriseStore / SqliteEnterpriseStore"]

        E_TRAFFIC["traffic (PostgreSQL or SQLite)<br/>PostgresTrafficStore / SqliteTrafficStore"]
        E_CONFIG["config (PostgreSQL or SQLite)<br/>PostgresConfigStore / SqliteConfigStore"]
    end

    S_TRAFFIC -.->|"same trait, different backend"| E_TRAFFIC
    S_CONFIG -.->|"same trait, different backend"| E_CONFIG
```

### 3.6 Shared storage trait design

> **Full detail:** [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md)

The core stores (traffic, config, intercept, plugins, scripts) need
**shared async traits** so both SQLite and PostgreSQL backends can
implement the same interface. This requires migrating all stores from
`rusqlite` (sync) to `sqlx` (async).

Key decisions:
- **Async traits** (`#[async_trait]`) — required for `sqlx` compatibility
- **`sqlx` only** — eliminates `rusqlite` entirely; both backends use `sqlx`
- **Separate implementations** per backend (not `sqlx::Any`) — SQLite and PostgreSQL have different type systems (TEXT vs UUID, INTEGER vs TIMESTAMPTZ)
- **Shared query helpers** reduce SQL duplication between backends

Traits defined in `madhyamas-core/src/storage/`:
- `TrafficStoreBackend` — 41 methods (largest)
- `ConfigStoreBackend` — 8 methods
- `InterceptStoreBackend` — 19 methods
- `PluginStoreBackend` — ~10 methods
- `ScriptStoreBackend` — ~15 methods

Plus `EnterpriseStore` trait in `madhyamas-enterprise/src/store/` for
users, API keys, auth sessions, and audit events.

### 3.7 Migration approach

> **Full detail:** [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md#2-migration-approach)

The migration from `rusqlite` to `sqlx` is phased smallest-store-first:

| Phase | Scope | Effort |
|---|---|---|
| A | Enterprise store (new code, no migration) | Medium |
| B | Define core storage traits (compile-time only) | Small |
| C | Migrate stores to `sqlx::SqlitePool` (ConfigStore → InterceptStore → PluginStore → ScriptStore → TrafficStore) | Large |
| D | Implement PostgreSQL backends (enterprise feature) | Medium |
| E | Remove `rusqlite` dependency | Trivial |

### 3.8 Database configuration

After migration, both tiers use `sqlx` with a configurable backend:

```toml
# ~/.madhyamas/enterprise.toml (enterprise tier)
[database]
backend = "postgres"  # postgres | sqlite
url = "postgres://madhyamas:password@db.internal:5432/madhyamas"
max_connections = 10
```

```bash
# Environment variables (both tiers)
MADHYAMAS_DB_BACKEND=postgres    # sqlite | postgres
MADHYAMAS_DB_URL=postgres://user:pass@host:5432/madhyamas
```

### 3.9 Connection pooling

`sqlx::PgPool` provides built-in connection pooling. For high-scale
deployments, external `PgBouncer` can be used in front of PostgreSQL
for connection multiplexing.

### 3.10 Schema migrations

Use `sqlx::migrate!` macro for schema management. At startup, pending
migrations run automatically. This supports schema evolution across
versions.

### 3.11 PostgreSQL-specific schema optimizations

The PostgreSQL schema (see [Appendix below](#appendix-d-postgresql-schema-for-enterprise-store))
uses native types: `UUID`, `JSONB`, `TIMESTAMPTZ`, `BOOLEAN`, `CHECK`
constraints, `GIN` indexes for JSONB, partial indexes, and optional
table partitioning for audit events.

### 3.12 Multi-instance considerations

When multiple proxy instances share a PostgreSQL database:

| Concern | Solution |
|---|---|
| Concurrent traffic writes | PostgreSQL MVCC handles concurrent inserts |
| Session state | Shared `auth_sessions` table — all instances see revocations |
| Audit deduplication | Each audit event has a UUID; idempotent inserts |
| Traffic store contention | Each instance writes its own entries; shared reads for cross-instance visibility |

---

## 4. Registration, Attestation, and Licensing

> **Full licensing server design:** [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md)

The enterprise tier requires proof of entitlement before features
activate. This section covers the **proxy-side** license verification.
The **server-side** licensing infrastructure (account management,
payments, license issuance, support tickets) is detailed in the
licensing server document.

### 4.1 License model

An **Ed25519-signed license file** is the primary mechanism. This
reuses the existing `ed25519-dalek` dependency and the signing/verification
utilities already built for the plugin system.

#### License file format

```json
{
  "payload": {
    "license_id": "uuid",
    "license_type": "enterprise",
    "organization": "Acme Corp",
    "contact_email": "admin@acme.com",
    "issued_at": "2026-08-12T00:00:00Z",
    "expires_at": "2027-08-12T00:00:00Z",
    "max_users": 50,
    "features": ["auth", "rbac", "audit", "sso", "mfa"],
    "fingerprint": "sha256-of-organization-domain-or-install-id"
  },
  "signature": "hex-encoded-64-byte-ed25519-signature-over-canonical-payload"
}
```

#### License payload fields

| Field | Type | Description |
|---|---|---|
| `license_id` | UUID | Unique license identifier (for revocation tracking) |
| `license_type` | string | `"enterprise"`, `"enterprise-trial"`, `"enterprise-academic"` |
| `organization` | string | Registered organization name |
| `contact_email` | string | Admin contact (for revocation notices) |
| `issued_at` | ISO 8601 | Issue timestamp |
| `expires_at` | ISO 8601 | Expiry timestamp (`null` for perpetual) |
| `max_users` | u32 | Maximum named users (0 = unlimited) |
| `features` | string[] | Enabled feature flags (allows tiered entitlement) |
| `fingerprint` | string | Binding token (see 4.3) |
| `issuer` | string | `"madhyamas-license-authority"` |
| `issuer_key_id` | string | Which signing key was used (for key rotation) |

#### Verification process

```mermaid
flowchart TD
    A[Startup] --> B{Enterprise feature\ncompiled?}
    B -->|No| C[Simple mode — done]
    B -->|Yes| D{License file\npresent?}
    D -->|No| E[Refuse to start with\n--enable-auth, or\nstart in simple mode]
    D -->|Yes| F[Parse license JSON]
    F --> G[Verify Ed25519 signature\nagainst embedded public key]
    G -->|Invalid| H[Reject: invalid license]
    G -->|Valid| I{Expired?}
    I -->|Yes| J[Reject: license expired]
    I -->|No| K{Feature flags\ninclude 'auth'?}
    K -->|No| L[Enterprise routes\nnot mounted]
    K -->|Yes| M{Optional online\nrevocation check?}
    M -->|Enabled & reachable| N[Check license_id\nagainst revocation list]
    N -->|Revoked| O[Reject: license revoked]
    N -->|Not revoked| P[Activate enterprise]
    M -->|Disabled or unreachable| P
    P --> Q[Mount enterprise routes\nConstruct AuthManager\nEnable RBAC + audit]
```

### 4.2 Registration process

Registration is handled by the **licensing server** at
`madhyamas.ai/register`. See [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md)
for the full account management, payment, and license issuance flow.

Summary:
1. Organization registers at the portal (org name, admin email, password)
2. Selects a pricing plan and completes payment via Stripe
3. Licensing server generates and signs a license file (Ed25519)
4. License file is available for download in the portal + sent via email
5. Organization places license at `~/.madhyamas/license.json`
6. Proxy binary verifies signature, checks expiry, activates enterprise

For **trial licenses**, the portal issues 30-day licenses automatically.
For **manual licenses** (air-gapped, government, custom contracts), a
Madhyamas admin issues licenses via the admin dashboard.

### 4.3 Fingerprint binding (optional)

- **Soft binding:** `fingerprint` is a hash of the organization's
  domain. The binary logs a warning if it doesn't match but does not
  refuse to start.
- **Hard binding:** The binary refuses to start if the fingerprint
  doesn't match the local machine. Requires generating a machine ID at
  registration time.

**Recommendation:** Start with soft binding. Hard binding adds
operational friction (VM migrations, hardware changes).

### 4.4 Online revocation check (optional)

The proxy binary can optionally check license revocation status against
the licensing server:

- `--license-revocation-url https://madhyamas.ai/api/v1/license/verify`
- At startup, the binary checks the license ID against the revocation API
- If revoked, the license is rejected
- If the endpoint is unreachable, the license is accepted (fail-open,
  configurable to fail-closed via `--license-strict-revocation`)
- Check result is cached for 24 hours

**Off by default.** Air-gapped environments never need it. See
[ENTERPRISE_LICENSING_SERVER.md §7](ENTERPRISE_LICENSING_SERVER.md#7-license-verification-api)
for the API design.

### 4.5 Trial licenses

A `license_type: "enterprise-trial"` license has a short `expires_at`
(30 days). The binary enforces expiry at startup. The licensing server
sends warning emails 3 days before trial expiry.

### 4.6 What happens without a valid license

| Scenario | Behavior |
|---|---|
| Simple build (no `enterprise` feature) | Always starts in simple mode. No license check. |
| Enterprise build, no `--enable-auth`, no license | Starts in simple mode. Enterprise code is compiled but dormant. |
| Enterprise build, `--enable-auth`, no license | Refuses to start. Prints: "Enterprise features require a valid license. Register at madhyamas.ai/register." |
| Enterprise build, `--enable-auth`, invalid license | Refuses to start. Prints: "License verification failed: [reason]." |
| Enterprise build, `--enable-auth`, expired license | Refuses to start. Prints: "License expired on [date]. Contact admin@madhyamas.ai to renew." |
| License valid but `features` array doesn't include `"sso"` | Enterprise auth/RBAC/audit activate, but SSO endpoints return 501. |

### 4.7 License CLI subcommands

```bash
madhyamas license verify --file ~/.madhyamas/license.json
# Verifies signature and prints license details or error

madhyamas license info
# Prints current license status (if loaded)

madhyamas license fingerprint
# Prints this machine's fingerprint (for hard binding registration)
```

---

## 5. Current State: What Is Built

The enterprise layer exists as a **feature-gated scaffold**. The types
and middleware are real and functional; the handlers, persistence, and
runtime wiring are not.

### 5.1 Implemented and functional

| Component | Location | Status |
|---|---|---|
| `AuthManager` (JWT issue/validate, API key gen) | `madhyamas-core/src/enterprise/auth.rs` | Works in-memory |
| `RbacManager` (role → permission matrix) | `madhyamas-core/src/enterprise/rbac.rs` | Works in-memory |
| `AuditLogger` (ring buffer + in-memory) | `madhyamas-core/src/enterprise/audit.rs` | Works in-memory |
| `UserManager` (in-memory user store) | `madhyamas-core/src/enterprise/user.rs` | Works in-memory |
| `auth_middleware` (JWT validation) | `madhyamas-api/src/middleware.rs` | Functional |
| `require_permission_middleware` (RBAC check) | `madhyamas-api/src/middleware.rs` | Functional |
| `AuthUser` extractor (extracts JWT claims) | `madhyamas-api/src/auth.rs` | Functional |

### 5.2 Scaffolded but not working

| Component | Location | Issue |
|---|---|---|
| Enterprise API handlers (30+ endpoints) | `madhyamas-api/src/enterprise_handlers.rs` | All return `501 Not Implemented` or empty responses |
| `create_routes_with_enterprise()` | `madhyamas-api/src/routes.rs` | Defined but never called from `main.rs` |
| Enterprise CLI subcommands | `madhyamas-cli/src/commands/enterprise.rs` | Defined but stubbed |
| Enterprise MCP tools | `madhyamas-mcp/src/tools/enterprise.rs` | Defined but stubbed |

### 5.3 Architecture (current)

```mermaid
graph TD
    subgraph "madhyamas-core"
        AUTH["AuthManager<br/>(in-memory)"]
        RBAC["RbacManager<br/>(in-memory)"]
        AUDIT["AuditLogger<br/>(in-memory ring buffer)"]
        USER["UserManager<br/>(in-memory)"]
    end

    subgraph "madhyamas-api"
        MW["auth_middleware<br/>require_permission_middleware"]
        HANDLERS["enterprise_handlers.rs<br/>(30+ stubs)"]
        ROUTES["create_routes_with_enterprise()<br/>(never called)"]
        STATE["AppState.auth_service<br/>(#[cfg] gated)"]
    end

    subgraph "madhyamas (main binary)"
        MAIN["main.rs<br/>(does not call enterprise path)"]
    end

    AUTH --> MW
    RBAC --> MW
    MW --> HANDLERS
    HANDLERS --> ROUTES
    ROUTES -.->|"never called"| MAIN
    STATE -.->|"#[cfg] gated"| AUTH
```

**Key problem:** The enterprise code is compiled into the default build
(enterprise is in default features) but is never activated. It adds
binary bloat and dependency weight without providing value. The
separate crate approach (Section 2) fixes this.

---

## 6. What Is Missing

### 6.1 Must-have for a usable enterprise tier

1. **Separate `madhyamas-enterprise` crate** — extract all enterprise
  code. Define `AuthProvider`, `Authorizer`, `AuditSink` traits in
  `madhyamas-api`. See [Section 2](#2-crate-architecture-separate-madhyamas-enterprise-crate).

2. **License verification module** — new
  `crates/madhyamas-enterprise/src/license.rs`. Ed25519 verification,
  expiry check, feature flags. See [Section 4](#4-registration-attestation-and-licensing).

3. **Licensing server** — full SaaS platform for account management,
  Stripe payments, license issuance, support tickets. See
  [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md).

4. **Enterprise database layer** — `crates/madhyamas-enterprise/src/store/`
  with PostgreSQL backend via `sqlx`. See [Section 3](#3-database-strategy-sqlite-and-postgresql).

5. **Shared storage traits for core stores** — async traits in
  `madhyamas-core/src/storage/`. Migrate `rusqlite` → `sqlx`. See
  [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md).

6. **Real handler implementations** — wire enterprise handlers to
  managers + store. See [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md).

7. **Password hashing** — `argon2` dependency in enterprise crate.

8. **Wire enterprise into `main.rs`** — construct `EnterpriseState`,
  inject trait implementations, merge routers, apply middleware.

9. **CLI/env configuration** — `--enable-auth`, `--jwt-secret`,
  `--license-file`, `--db-backend`, `--db-url`, `--admin-username`,
  `--admin-password`, `--auth-mode`.

10. **Audit emission at mutation sites** — call `AuditSink::log` from
    intercept handlers and API handlers. No-op when `audit_sink` is
    `None` (simple tier).

11. **API-key middleware branch** — check `X-API-Key` header when no
    JWT is present.

12. **Token refresh endpoint** — `POST /auth/refresh`.

13. **Wire `MetricsCollector` to real `PerformanceMonitor`**.

### 6.2 Should-have for real enterprise use

14. **Per-user rate limiting** — key on JWT `sub` claim, not just IP.
15. **Secret management** — reject default JWT secret in production.
16. **Session revocation** — denylist check in auth middleware.
17. **MFA (TOTP)** — `totp-rs` dependency, setup/verify endpoints.
18. **SSO / external IdP** — OIDC, header-based, LDAP. See
    [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md).
19. **Optional: traffic store PostgreSQL backend** — shared traffic
    storage across instances. See
    [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md).

### 6.3 Gap summary table

| Area | Built | Stub | Missing |
|---|---|---|---|
| Separate enterprise crate | — | — | New `madhyamas-enterprise` crate + trait abstractions |
| License verification | — | — | New `license.rs` module in enterprise crate |
| Licensing server | — | — | Full SaaS: accounts, Stripe, tickets, admin. See [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md) |
| Enterprise database (PostgreSQL) | — | — | New `store/` module with `sqlx` + PostgreSQL |
| Shared storage traits (core stores) | — | — | Async traits; migrate `rusqlite` → `sqlx`. See [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) |
| Auth types (JWT, API key) | Yes (in core) | — | Move to enterprise crate; API-key branch in middleware |
| RBAC types + check | Yes (in core) | — | Move to enterprise crate; per-route enforcement |
| Audit types + logger | Yes (in core) | — | Move to enterprise crate; PostgreSQL persistence |
| User types | Yes (in core) | — | Move to enterprise crate; store, password hashing |
| Enterprise handlers | — | All 30+ (in api) | Move to enterprise crate; real implementations |
| Routes mounted at runtime | — | — | `main.rs` must construct enterprise router and merge |
| CLI/env config | — | — | `--enable-auth`, `--license-file`, `--jwt-secret`, `--db-backend`, `--db-url` |
| Metrics/performance | — | Stubs | Wire to existing `PerformanceMonitor` |
| Web UI (login/admin/audit) | — | — | New `auth` + `admin` frontend features. See [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md) |
| SSO (OIDC/SAML/LDAP) | — | — | OIDC recommended first. See [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) |
| MFA | — | — | TOTP (optional) |
| Plugin-based integrations | — | — | IdP/audit-sink adapters (optional) |

---

## 7. Web UI Strategy

> **Full detail:** [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md)

The enterprise tier requires significant web UI additions: login page,
user management, audit log viewer, metrics dashboard, license info
panel, and user menu in the header. This section addresses where that
code lives and how it's gated.

### 7.1 The question: same folder or separate?

Three options were analyzed:

| Option | Description | Verdict |
|---|---|---|
| A: Separate `web-enterprise/` folder | Completely separate React app, own package.json, own build | Rejected — duplicates all infrastructure (shadcn/ui, API client, hooks, tailwind), divergent styling, two build pipelines |
| B: Same `web/` folder, build-time gated | `VITE_ENTERPRISE=true` env var controls which features are built; two builds | Rejected — requires CI coordination, risk of build mismatch |
| **C: Same `web/` folder, runtime-gated** | Single build. Frontend calls `/api/health/detailed` at startup to learn tier. Enterprise features lazy-loaded only if tier is enterprise. | **Recommended** |

### 7.2 Why same folder, runtime-gated (Option C)

1. **Bundle size impact is negligible.** Enterprise features are
   lazy-loaded via `React.lazy(() => import(...))`. Vite splits them
   into separate chunks. In the simple tier, these chunks are embedded
   but never fetched by the browser.
2. **Enterprise JS in the OSS repo is not a security concern.** The
   enterprise *backend* code is in a separate Rust crate
   (`madhyamas-enterprise`) not compiled in the simple build. The
   enterprise *frontend* JS calls API endpoints that don't exist in
   the simple tier (they return 404). No features can be activated.
3. **Shared infrastructure is a major benefit.** Enterprise UI needs
   the same shadcn/ui components, API client patterns, TanStack Query
   hooks, and Tailwind theme. Duplicating these would be wasteful.
4. **The licensing server portal is already separate.** The licensing
   server (`madhyamas.ai`) has its own React app — that's the right
   separation (customer admin vs. developer using the proxy). The
   proxy's enterprise UI is for the same audience as the OSS UI.
5. **Single build is simpler.** One `npm run build`, one `web/dist/`,
   one `rust-embed` config. No CI coordination. No mismatch risk.

### 7.3 How runtime gating works

```mermaid
flowchart TD
    A[App loads] --> B[GET /api/health/detailed]
    B --> C{tier?}
    C -->|community| D[Render OSS UI<br/>no auth, no admin]
    C -->|enterprise| E{auth_required?}
    E -->|No| F[Render enterprise UI<br/>with admin features]
    E -->|Yes| G{has valid JWT?}
    G -->|No| H[Render LoginPage]
    G -->|Yes| F
    D --> I[Enterprise chunks<br/>never loaded]
    F --> J[Lazy-load admin panels<br/>UsersPanel, AuditLogViewer, etc.]
```

### 7.4 What UI changes are needed

| Area | Change | Files |
|---|---|---|
| **Tier detection** | New `/api/health/detailed` endpoint; frontend fetches tier info at startup | `web/src/lib/tier.ts` (new), backend handler |
| **Auth infrastructure** | AuthContext (user state, login, logout), LoginPage, ProtectedApp wrapper, UserMenu in header | `web/src/features/auth/` (new, 5-6 files) |
| **Shell changes** | AppHeader shows UserMenu + license badge; NavRail adds admin nav items; App.tsx wraps in AuthProvider | `App.tsx`, `AppHeader.tsx`, `NavRail.tsx` (modified) |
| **Admin panels** | UsersPanel, AuditLogViewer, MetricsDashboard, LicensePanel | `web/src/features/admin/` (new, 4-5 files) |
| **API client** | Auth token injection (Authorization header), 401 handling, new auth.ts and admin.ts modules | `client.ts` (modified), `auth.ts` + `admin.ts` (new) |
| **SSO** | SSO button on login page (if OIDC), OIDC callback handler | `SsoRedirect.tsx` (new) |
| **MFA (optional)** | TOTP setup dialog, MFA step in login flow | `MfaSetupDialog.tsx` (new) |

### 7.5 What stays unchanged

All OSS feature panels (`traffic/`, `tools/`, `sessions/`, `cert/`,
`config/`, `onboarding/`), all `components/ui/` shadcn primitives, all
`hooks/`, and all `types/` remain unchanged. Enterprise features are
additive — they don't modify existing feature panels.

### 7.6 Build and embedding

No changes to the build pipeline. The same `npm run build` →
`web/dist/` → `rust-embed` flow works for both tiers. Enterprise
features are in separate Vite chunks (via `React.lazy`) that are
embedded but never loaded in the simple tier.

---

## 8. Plugin vs Flag vs Crate Architecture

This section evaluates three mechanisms for isolating enterprise
functionality: Cargo features (flag-based), WASM plugins, and a separate
crate. The analysis in [Section 2](#2-crate-architecture-separate-madhyamas-enterprise-crate)
already established that a separate crate is the best primary mechanism.
This section contextualizes that decision against the alternatives and
defines the role of each.

### 7.1 Crate-based (primary mechanism — recommended)

A separate `madhyamas-enterprise` crate is the primary isolation
mechanism. See [Section 2](#2-crate-architecture-separate-madhyamas-enterprise-crate).
Summary of why this is superior to pure flag-based gating:

- **Structural guarantee:** The simple build cannot reference enterprise
  code. No `#[cfg]` leakage across 17+ sites in core and API.
- **Dependency isolation:** `argon2`, `openidconnect`, `ldap3`,
  `totp-rs`, `sqlx/postgres` are not in the simple build's dependency
  tree.
- **License isolation:** The enterprise crate can carry a different
  license (BSL, commercial) while core stays MIT/Apache.
- **Trait-based extensibility:** The `AuthProvider`, `Authorizer`,
  `AuditSink` traits in `madhyamas-api` enable mocking in tests and
  future alternative implementations.

### 7.2 Flag-based (supplementary — for within-crate toggles)

Cargo features are still useful **within** the enterprise crate for
optional sub-features:

| Feature | Controls | Default |
|---|---|---|
| `postgres` | PostgreSQL backend via `sqlx` | Yes |
| `sqlite` | SQLite backend via `sqlx` (small enterprise) | No |
| `oidc` | OIDC/SSO integration | Yes |
| `ldap` | LDAP bind authentication | No |
| `mfa` | TOTP MFA support | No |

### 7.3 Plugin-based (complement, not replacement)

The existing WASM plugin/extension system hosts **integration adapters**
rather than core auth logic. Plugins run in the intercept pipeline
(priority 10-40) and operate on traffic, not on API requests to the
admin surface.

| Plugin type | What it does | Why it fits plugins |
|---|---|---|
| OIDC connector | Handles IdP redirect/callback, group mapping | I/O-bound, swappable per deployment |
| Audit sink | Forwards `AuditEvent` to Splunk/ELK/S3 | I/O-bound, org-specific destination |
| Custom role mapper | Maps exotic IdP group layouts to roles | Org-specific logic, not core |
| MFA provider | SMS/push notification MFA | External service integration |

### 7.4 Recommended three-layer split

```mermaid
graph TD
    subgraph "Layer 1: madhyamas-api (traits, always present)"
        TRAITS["AuthProvider trait\nAuthorizer trait\nAuditSink trait"]
        STATE["AppState\nOption<Arc<dyn AuthProvider>>"]
    end

    subgraph "Layer 2: madhyamas-enterprise (crate, license-gated)"
        LICENSE["License Verification\n(Ed25519)"]
        AUTH["Auth Manager\n(JWT + API keys + argon2)"]
        RBAC["RBAC Manager\n(role/permission matrix)"]
        AUDIT["Audit Logger\n(PostgreSQL/SQLite via sqlx)"]
        MW["Auth Middleware\n(JWT/API-key validation)"]
        PERM["Permission Middleware\n(RBAC enforcement)"]
        STORE["Enterprise Store\n(users, keys, sessions, audit)"]
        ROUTER["Enterprise Router\n(auth, users, rbac, audit endpoints)"]
    end

    subgraph "Layer 3: Plugins (optional, sandboxed, swappable)"
        OIDC_P["OIDC Connector Plugin"]
        AUDIT_SINK["Audit Sink Plugin\n(Splunk/ELK/S3)"]
        ROLE_MAP["Custom Role Mapper Plugin"]
        MFA_P["MFA Provider Plugin"]
    end

    subgraph "Config-driven mode selection"
        MODE["auth.mode = local | oidc | header | ldap"]
    end

    TRAITS --> STATE
    LICENSE --> AUTH
    MODE --> AUTH
    AUTH --> MW
    RBAC --> PERM
    MW --> PERM
    MW --> ROUTER
    STORE --> AUTH
    STORE --> AUDIT
    AUDIT --> AUDIT_SINK
    OIDC_P --> AUTH
    ROLE_MAP --> RBAC
    MFA_P --> AUTH
    AUTH -.->|"implements"| TRAITS
```

- **Layer 1 — API traits (always present):** `AuthProvider`,
  `Authorizer`, `AuditSink` traits in `madhyamas-api`. `AppState` holds
  `Option<Arc<dyn trait>>`. Simple tier sets them to `None`.
- **Layer 2 — Enterprise crate (license-gated):** License verification,
  password store, JWT, RBAC, audit persistence, middleware, handlers.
  Implements Layer 1 traits. Activated only with `--features enterprise`
  and a valid license.
- **Layer 3 — Plugins (optional):** External IdP connectors, audit
  forwarding, custom role mapping, MFA providers. Loaded via the
  existing plugin manager.

---

## 9. Implementation Roadmap

The roadmap covers both the **proxy binary** enterprise tier and the
**licensing server**. They are independent projects that can proceed
in parallel.

### 9.1 Proxy binary roadmap

| Phase | Goal | Effort | Detail |
|---|---|---|---|
| Phase 0 | Crate extraction + simple tier separation | Medium-large | [Section 2](#2-crate-architecture-separate-madhyamas-enterprise-crate) |
| Phase 1 | License verification (Ed25519) | Medium | [Section 4](#4-registration-attestation-and-licensing) |
| Phase 2 | Enterprise database (PostgreSQL via sqlx) | Medium | [Section 3](#3-database-strategy-sqlite-and-postgresql) |
| Phase 2a | Core storage migration (rusqlite → sqlx) | Large | [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) |
| Phase 3 | Auth/RBAC/audit implementation | Large | [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) |
| Phase 4 | RBAC enforcement | Medium | [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) |
| Phase 5 | Web UI (login, admin, audit) | Large | [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md) |
| Phase 6 | External IdP (OIDC, header, LDAP) | Medium | [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) |
| Phase 7 | Hardening (MFA, session revocation, refresh tokens) | Medium | — |

### 9.2a Web UI roadmap

| Phase | Goal | Effort | Detail |
|---|---|---|---|
| Phase UI-1 | Tier detection + auth infrastructure | Medium | [ENTERPRISE_WEB_UI.md §11](ENTERPRISE_WEB_UI.md#11-implementation-phases) |
| Phase UI-2 | Shell changes (UserMenu, nav items, badges) | Small | [ENTERPRISE_WEB_UI.md §11](ENTERPRISE_WEB_UI.md#11-implementation-phases) |
| Phase UI-3 | Admin panels (users, audit, metrics, license) | Large | [ENTERPRISE_WEB_UI.md §11](ENTERPRISE_WEB_UI.md#11-implementation-phases) |
| Phase UI-4 | SSO integration (OIDC callback) | Small | [ENTERPRISE_WEB_UI.md §11](ENTERPRISE_WEB_UI.md#11-implementation-phases) |
| Phase UI-5 | MFA (optional) | Small | [ENTERPRISE_WEB_UI.md §11](ENTERPRISE_WEB_UI.md#11-implementation-phases) |
| Phase UI-6 | Onboarding integration | Small | [ENTERPRISE_WEB_UI.md §11](ENTERPRISE_WEB_UI.md#11-implementation-phases) |

### 9.2 Licensing server roadmap

| Phase | Goal | Effort | Detail |
|---|---|---|---|
| Phase L1 | Core license server (signing, accounts, manual issuance) | Large | [ENTERPRISE_LICENSING_SERVER.md §15](ENTERPRISE_LICENSING_SERVER.md#15-implementation-roadmap) |
| Phase L2 | Stripe integration (self-service payments) | Medium | [ENTERPRISE_LICENSING_SERVER.md §5](ENTERPRISE_LICENSING_SERVER.md#5-payment-processing-stripe) |
| Phase L3 | Email notifications | Small-medium | [ENTERPRISE_LICENSING_SERVER.md §10](ENTERPRISE_LICENSING_SERVER.md#10-email-notifications) |
| Phase L4 | Support ticket system | Medium | [ENTERPRISE_LICENSING_SERVER.md §8](ENTERPRISE_LICENSING_SERVER.md#8-support-ticket-system) |
| Phase L5 | Admin dashboard + analytics | Medium | [ENTERPRISE_LICENSING_SERVER.md §9](ENTERPRISE_LICENSING_SERVER.md#9-admin-dashboard) |
| Phase L6 | Hardening + scale | Medium | [ENTERPRISE_LICENSING_SERVER.md §16](ENTERPRISE_LICENSING_SERVER.md#16-risk-analysis) |

### 9.3 Combined roadmap

```mermaid
gantt
    title Enterprise Implementation (Proxy + Licensing Server)
    dateFormat YYYY-MM-DD
    axisFormat %b %d

    section Proxy — Foundation
    Phase 0: Crate extraction              :p0, 2026-08-13, 7d
    Phase 1: License verification           :p1, after p0, 5d

    section Proxy — Database
    Phase 2: Enterprise DB (sqlx+Pg)        :p2, after p1, 7d
    Phase 2a: Core storage migration        :p2a, after p2, 25d

    section Proxy — Core
    Phase 3: Auth/RBAC/audit impl           :p3, after p2, 14d
    Phase 4: RBAC enforcement               :p4, after p3, 5d

    section Proxy — UI + Integration
    Phase UI-1: Tier detection + auth      :u1, after p4, 5d
    Phase UI-2: Shell changes              :u2, after u1, 3d
    Phase UI-3: Admin panels               :u3, after u2, 10d
    Phase 6: External IdP (OIDC)            :p6, after u3, 7d
    Phase UI-4: SSO integration            :u4, after p6, 3d
    Phase 7: Hardening                      :p7, after u4, 10d

    section Licensing Server
    Phase L1: Core license server           :l1, 2026-08-13, 21d
    Phase L2: Stripe integration            :l2, after l1, 14d
    Phase L3: Email notifications           :l3, after l2, 7d
    Phase L4: Support tickets               :l4, after l3, 10d
    Phase L5: Admin dashboard               :l5, after l4, 10d
    Phase L6: Hardening + scale             :l6, after l5, 14d
```

### 9.4 Dependency chain

```
Proxy binary:
Phase 0 (crate extraction)
  └→ Phase 1 (license verification)
       └→ Phase 2 (enterprise database)
            ├→ Phase 2a (core storage migration: rusqlite → sqlx)
            └→ Phase 3 (auth/RBAC/audit impl)
                 ├→ Phase 4 (RBAC enforcement)
                 │    └→ Phase UI-1 (tier detection + auth UI)
                 │         └→ Phase UI-2 (shell changes)
                 │              └→ Phase UI-3 (admin panels)
                 │                   └→ Phase 6 (external IdP)
                 │                        └→ Phase UI-4 (SSO integration)
                 │                             └→ Phase 7 (hardening)
                 └→ Phase 7 (hardening — parallel with UI)

Licensing server (independent of proxy binary):
Phase L1 (core server)
  └→ Phase L2 (Stripe)
       └→ Phase L3 (email)
            └→ Phase L4 (support tickets)
                 └→ Phase L5 (admin dashboard)
                      └→ Phase L6 (hardening)
```

The licensing server can launch before the proxy binary's enterprise
tier is complete. Manual license issuance (Phase L1) is enough to
support early customers. The only contract between them is the license
file format (Ed25519-signed JSON) and the optional revocation API.

---

## 10. Risk Analysis

> **Note:** For a comprehensive performance and security analysis with
> threat model, 16 security gaps, 10 performance bottlenecks, and
> pre-launch checklists, see
> [ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md). The
> risk tables below are a summary; the dedicated document has full
> remediations with code examples.

### 10.1 Security risks

| Risk | Mitigation |
|---|---|
| License private key compromise | Rotate keys, embed multiple public keys, support `issuer_key_id` for rotation. Store in Secrets Manager/Vault. See [ENTERPRISE_LICENSING_SERVER.md §13](ENTERPRISE_LICENSING_SERVER.md#13-security). |
| JWT secret leakage | Require from env/file, reject default in production, never log |
| Password database compromise | argon2id hashing (memory-hard, GPU-resistant) |
| API key sharing | Track `last_used`, support expiry, audit key creation/revocation |
| Middleware bypass | Apply `auth_middleware` as a layer on the entire `/api` router, not per-route. Public paths are explicitly listed. |
| License file sharing (soft binding) | Log fingerprint mismatch as warning. Hard binding available if needed. Optional attestation endpoint for abuse detection. |
| XSS in web UI steals JWT | Store in `sessionStorage` (not `localStorage`), use `HttpOnly` cookies as alternative, CSP headers |
| Stripe API key compromise | Store in Secrets Manager. Use restricted keys. Rotate on personnel turnover. |
| WebSocket has no authentication | Add JWT validation on WS upgrade (query param token). See [PERF_SECURITY §3.6](ENTERPRISE_PERF_SECURITY.md#36-websocket-has-no-authentication). |
| Redis pub/sub unauthenticated | Redis password + TLS + K8s NetworkPolicy. See [PERF_SECURITY §3.10](ENTERPRISE_PERF_SECURITY.md#310-redis-has-no-authentication-or-tls-by-default). |
| Missing CSP headers on proxy UI | Add Content-Security-Policy + security headers to embedded asset responses. See [PERF_SECURITY §3.3](ENTERPRISE_PERF_SECURITY.md#33-missing-csp-headers-on-proxy-web-ui). |
| JWT clock skew / algorithm confusion | Add ±60s tolerance, pin `alg: HS256`, validate `iat`. See [PERF_SECURITY §3.1-3.2](ENTERPRISE_PERF_SECURITY.md#31-jwt-clock-skew-and-validation-gaps). |
| Audit log tampering | Hash-chained append-only table with triggers. See [PERF_SECURITY §3.13](ENTERPRISE_PERF_SECURITY.md#313-audit-log-integrity). |
| No password complexity / session idle timeout | Enforce 12+ char complexity, refresh token rotation with 15min access / 8h refresh. See [PERF_SECURITY §3.14-3.15](ENTERPRISE_PERF_SECURITY.md#314-no-password-complexity-enforcement). |

### 10.2 Operational risks

| Risk | Mitigation |
|---|---|
| License expiry causes outage | Warning banner 30 days before expiry, CLI `license info` command, email notification at registration. See [ENTERPRISE_LICENSING_SERVER.md §10](ENTERPRISE_LICENSING_SERVER.md#10-email-notifications). |
| Enterprise startup fails without license | Clear error message with registration URL. Fallback to simple mode if `--enable-auth` not set. |
| PostgreSQL connection failure | Connection retry with backoff at startup. Health check endpoint reports DB status. Fallback to SQLite if configured. |
| PostgreSQL connection pool exhaustion | Configurable `max_connections` per instance. External PgBouncer for high-instance-count deployments. |
| Database migration failure on upgrade | `sqlx::migrate!` runs in a transaction. Test migrations on staging. Backup before upgrade. |
| rusqlite → sqlx migration breaks behavior | Migrate stores one at a time, smallest first. Each sub-phase produces a working build. See [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md#2-migration-approach). |
| Sync-to-async caller migration introduces deadlocks | All store callers are already in async context. Adding `.await` is mechanical. Test each store migration independently. |
| Stripe outage | Stripe has 99.99% uptime. Webhook retries for 3 days. Manual license issuance as fallback. |
| Licensing server outage | Does not affect running proxy instances (offline license verification). Only affects new registrations and revocation checks. |
| Enterprise UI code leaks feature details in simple build | Enterprise JS chunks are embedded but never loaded. They call API endpoints that return 404 in simple tier. No features can be activated. See [ENTERPRISE_WEB_UI.md §3](ENTERPRISE_WEB_UI.md#3-recommended-approach-same-folder-runtime-gated). |
| JWT stored in sessionStorage is stolen via XSS | React auto-escaping prevents XSS in our UI. CSP headers add defense-in-depth. Consider HttpOnly cookies as alternative. See [ENTERPRISE_WEB_UI.md §8](ENTERPRISE_WEB_UI.md#8-api-client-changes). |
| Tier detection endpoint (`/api/health/detailed`) is unavailable | Frontend falls back to `tier: "community"` if the endpoint returns an error. App renders in simple mode. |
| SQLite single-writer lock under high traffic | Migrate to PostgreSQL with PgPool (MVCC). See [PERF_SECURITY §5.1](ENTERPRISE_PERF_SECURITY.md#51-sqlite-mutexconnection--single-writer). |
| Synchronous audit logging adds latency | Batch inserts with background flusher (1s / 100 events). See [PERF_SECURITY §5.3](ENTERPRISE_PERF_SECURITY.md#53-audit-logging--synchronous-insert-per-request). |
| WebSocket broadcast fan-out memory | Use `Arc<TrafficEvent>` in broadcast channel. See [PERF_SECURITY §5.4](ENTERPRISE_PERF_SECURITY.md#54-websocket-broadcast--fan-out-cost). |
| Traffic body memory pressure | Stream + compress bodies, lazy load on demand. See [PERF_SECURITY §5.8](ENTERPRISE_PERF_SECURITY.md#58-memory-usage--traffic-capture-with-bodies). |
| Naive PG port doesn't scale (COUNT(*), LIKE, no batching) | 16 database optimizations: tiered body storage, write batching, GIN/BRIN/trigram indexes, cursor pagination, partitioning. See [PERF_SECURITY §6](ENTERPRISE_PERF_SECURITY.md#6-database-optimization-for-high-volume-traffic). |
| PostgreSQL table bloat from large bodies (up to 20MB each) | Tiered storage: inline (≤1KB) / TOAST (≤100KB) / S3 (>100KB) + zstd compression. See [PERF_SECURITY §6.3-6.4](ENTERPRISE_PERF_SECURITY.md#63-tiered-storage-architecture). |
| PostgreSQL connection exhaustion (20+ instances) | PgBouncer external pooler (1000 clients → 25 server connections). See [PERF_SECURITY §6.11](ENTERPRISE_PERF_SECURITY.md#611-connection-pooling-with-pgbouncer). |

### 10.3 Adoption risks

| Risk | Mitigation |
|---|---|
| OSS users accidentally build enterprise | Enterprise is not in default features. Enterprise code is in a separate crate — not even compiled. |
| Enterprise users find license process too heavy | Self-serve trial portal for 30-day licenses. Stripe Checkout for instant purchase. |
| PostgreSQL requirement deters small enterprise | SQLite backend available as fallback (`--db-backend sqlite`). No external database needed for single-instance. |
| Separate crate adds build complexity | One additional `cargo build --features enterprise` flag. CI can build both tiers. |
| Pricing too low for sustainability | Start with current tiers. Monitor MRR and costs. Adjust based on customer feedback. |
| Customer churn | Track churn rate. Exit survey on cancellation. Engage with customers showing reduced license attestations. |

---

## Appendix A: New Dependencies

### Rust dependencies (shared — both tiers)

| Dependency | Purpose | Phase | Already in workspace? |
|---|---|---|---|
| `sqlx` | Async PostgreSQL + SQLite driver, connection pooling, migrations | Phase 2 / 2a | No — add to `[workspace.dependencies]` |
| `async-trait` | Trait definitions for auth and storage traits | Phase 0 / 2a | Yes |
| `uuid` | UUID generation for PostgreSQL store IDs | Phase 2 | Check — may already be transitive |
| `ed25519-dalek` | License signing/verification + plugin signing | Phase 1 | Yes |
| `jsonwebtoken` | JWT issue/validate (HMAC-SHA256) | Phase 3 | Yes |
| `reqwest` | Online license revocation check | Phase 1 | Yes |

### Rust dependencies (enterprise crate only — not in simple build)

| Dependency | Purpose | Phase |
|---|---|---|
| `argon2` | Password hashing (argon2id) | Phase 3 |
| `totp-rs` | MFA TOTP code generation/verification | Phase 7 |
| `openidconnect` | OIDC/OAuth2 client library | Phase 6 |
| `ldap3` | LDAP bind authentication | Phase 7 |

### Dependencies removed after Phase 2a

| Dependency | Reason |
|---|---|
| `rusqlite` | Replaced by `sqlx::SqlitePool` across all core stores. |

### Licensing server dependencies (separate workspace)

| Dependency | Purpose |
|---|---|
| `axum` | HTTP server (same as Madhyamas API) |
| `sqlx` | PostgreSQL driver (same as enterprise tier) |
| `stripe-rust` or custom client | Stripe API integration |
| `aws-sdk-ses` or `postmark` | Transactional email |
| `redis` | Session storage, rate limiting, revocation cache |
| `argon2` | Password hashing (portal users) |
| `totp-rs` | Admin MFA |
| `ed25519-dalek` | License signing (shared with proxy binary) |

### Frontend dependencies (enterprise UI)

| Dependency | Purpose | Phase |
|---|---|---|
| `recharts` | Metrics dashboard charts | Phase 5 |

### Dependency isolation benefit

With the separate `madhyamas-enterprise` crate, the simple build's
dependency tree does **not** include any of: `argon2`, `openidconnect`,
`ldap3`, `totp-rs`. After Phase 2a, `rusqlite` is also eliminated —
both tiers use `sqlx`. The `postgres` feature on `sqlx` is only enabled
in the enterprise build.

---

## Appendix B: Configuration File

Proposed `~/.madhyamas/enterprise.toml` (enterprise tier only):

```toml
# Enterprise configuration
# Only loaded when --enable-auth is set and a valid license is present.

[database]
backend = "postgres"  # postgres | sqlite
url = "postgres://madhyamas:password@db.internal:5432/madhyamas"
max_connections = 10
min_connections = 2
connect_timeout_secs = 10
idle_timeout_secs = 600
acquire_timeout_secs = 5

# For SQLite (small enterprise, single-instance):
# backend = "sqlite"
# url = "~/.madhyamas/madhyamas.db"

[auth]
mode = "local"  # local | oidc | header | ldap
jwt_secret_file = "~/.madhyamas/jwt-secret"
jwt_expiration_secs = 3600
refresh_interval_secs = 300
api_key_header = "X-API-Key"
require_auth = true

[auth.bootstrap]
admin_username = "admin"
# admin_password from MADHYAMAS_ADMIN_PASSWORD env (not stored in file)

# OIDC mode configuration (when mode = "oidc")
[oidc]
issuer_url = "https://your-idp.example.com"
client_id = "madhyamas"
client_secret_env = "MADHYAMAS_OIDC_CLIENT_SECRET"
scopes = ["openid", "profile", "email", "groups"]
redirect_uri = "http://localhost:3001/api/auth/oidc/callback"

[oidc.group_mapping]
"madhyamas-admins" = "admin"
"madhyamas-users" = "user"

# Header mode configuration (when mode = "header")
[header_auth]
header_user = "X-Forwarded-User"
header_groups = "X-Forwarded-Groups"
trusted_proxies = ["127.0.0.1/8", "10.0.0.0/8"]

# LDAP mode configuration (when mode = "ldap")
[ldap]
url = "ldaps://dc.example.com"
bind_dn_template = "{user}@example.com"
group_base_dn = "OU=Groups,DC=example,DC=com"
group_attribute = "memberOf"

# License configuration
[license]
file = "~/.madhyamas/license.json"
revocation_url = ""  # optional, leave empty for offline-only
attest_url = ""     # optional, leave empty for no attestation
strict_revocation = false  # fail-open if revocation server unreachable
```

---

## Appendix C: SQLite Schema for Enterprise Store (Fallback)

> Used when `database.backend = "sqlite"`. For the default PostgreSQL
> backend, see [Appendix D](#appendix-d-postgresql-schema-for-enterprise-store).

```sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT,
    display_name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'readonly',
    status TEXT NOT NULL DEFAULT 'active',
    password_hash TEXT,
    mfa_secret TEXT,
    preferences TEXT,
    created_at INTEGER NOT NULL,
    last_login INTEGER
);

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_used INTEGER
);

CREATE TABLE IF NOT EXISTS auth_sessions (
    session_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    client_ip TEXT,
    user_agent TEXT
);

CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    user_id TEXT,
    api_key_id TEXT,
    client_ip TEXT,
    description TEXT NOT NULL,
    metadata TEXT
);

CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_events_user_id ON audit_events(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_type ON audit_events(event_type);

CREATE TABLE IF NOT EXISTS role_permission_overrides (
    role TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    permission TEXT NOT NULL,
    granted INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (role, resource_type, permission)
);
```

API keys are stored as **SHA-256 hashes**. The raw key is shown to the
user once on creation and never again.

---

## Appendix D: PostgreSQL Schema for Enterprise Store

> Default schema for enterprise deployments. Uses PostgreSQL-native
> types. Full schema and migration files in
> [ENTERPRISE_LICENSING_SERVER.md §11](ENTERPRISE_LICENSING_SERVER.md#11-database-schema)
> (licensing server) and below (proxy-side enterprise store).

```sql
-- Users
CREATE TABLE IF NOT EXISTS users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username      TEXT UNIQUE NOT NULL,
    email         TEXT,
    display_name  TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'readonly'
                    CHECK (role IN ('admin', 'user', 'viewer', 'readonly')),
    status        TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'inactive', 'suspended', 'pending_verification')),
    password_hash TEXT,
    mfa_secret    TEXT,
    preferences   JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login    TIMESTAMPTZ
);

-- API keys
CREATE TABLE IF NOT EXISTS api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_hash    TEXT UNIQUE NOT NULL,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ,
    is_active   BOOLEAN NOT NULL DEFAULT true,
    last_used   TIMESTAMPTZ
);

-- Auth sessions
CREATE TABLE IF NOT EXISTS auth_sessions (
    session_id  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ NOT NULL,
    revoked     BOOLEAN NOT NULL DEFAULT false,
    client_ip   TEXT,
    user_agent  TEXT
);

-- Audit events
CREATE TABLE IF NOT EXISTS audit_events (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type  TEXT NOT NULL,
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT now(),
    user_id     UUID REFERENCES users(id) ON DELETE SET NULL,
    api_key_id  UUID REFERENCES api_keys(id) ON DELETE SET NULL,
    client_ip   TEXT,
    description TEXT NOT NULL,
    metadata    JSONB NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX idx_audit_events_timestamp ON audit_events(timestamp DESC);
CREATE INDEX idx_audit_events_user_id ON audit_events(user_id, timestamp DESC);
CREATE INDEX idx_audit_events_type ON audit_events(event_type, timestamp DESC);
CREATE INDEX idx_audit_events_metadata_gin ON audit_events USING GIN (metadata);

-- Role permission overrides
CREATE TABLE IF NOT EXISTS role_permission_overrides (
    role            TEXT NOT NULL,
    resource_type   TEXT NOT NULL,
    permission      TEXT NOT NULL,
    granted         BOOLEAN NOT NULL DEFAULT true,
    PRIMARY KEY (role, resource_type, permission)
);

-- License cache (optional)
CREATE TABLE IF NOT EXISTS license_cache (
    license_id      TEXT PRIMARY KEY,
    organization    TEXT NOT NULL,
    license_type    TEXT NOT NULL,
    verified_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ,
    features        JSONB NOT NULL DEFAULT '[]'::jsonb,
    raw_license     JSONB NOT NULL
);
```

---

## See Also

- [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md) — Full SaaS licensing server design
- [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) — Shared storage traits + multi-instance traits (EventBus, ConfigSync, InstanceRegistry, HealthProbe, CaProvider, Reloadable)
- [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) — Authentication, authorization, and IdP integration
- [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md) — Enterprise web UI design (same-folder, runtime-gated)
- [ENTERPRISE_CICD.md](ENTERPRISE_CICD.md) — CI/CD changes for two-tier builds
- [ENTERPRISE_MULTI_INSTANCE.md](ENTERPRISE_MULTI_INSTANCE.md) — Multi-instance deployment (LB routing, state sync, config propagation, K8s)
- [ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md) — Performance & security analysis (threat model, 16 gaps, 10 bottlenecks, checklists)
- [ENTERPRISE_OSS_COMPARISON.md](ENTERPRISE_OSS_COMPARISON.md) — OSS vs Enterprise comparison (feature parity, architecture, pricing, upgrade path)
- [ENTERPRISE_AI_AGENTS.md](ENTERPRISE_AI_AGENTS.md) — AI agent integration for enterprise (MCP/CLI/API auth gaps, enterprise MCP tools, RBAC scopes, implementation plan)
- [ENTERPRISE_CRATE_MIGRATION.md](ENTERPRISE_CRATE_MIGRATION.md) — Detailed migration analysis for extracting the madhyamas-enterprise crate (inventory, #[cfg] gates, dependency analysis, 6-phase plan)
- [ENTERPRISE_IMPLEMENTATION_PLAN.md](ENTERPRISE_IMPLEMENTATION_PLAN.md) — Comprehensive implementation plan synthesizing all analysis docs (13 phases, dependency graph, milestones, effort estimates, risk register)
- [ENTERPRISE.md](ENTERPRISE.md) — Current enterprise feature internals (pre-refactor)
- [API_ENTERPRISE.md](API_ENTERPRISE.md) — Enterprise API endpoint reference
- [WEB_FRONTEND.md](WEB_FRONTEND.md) — Current web frontend architecture (OSS)
- [PERFORMANCE.md](PERFORMANCE.md) — Performance monitoring (wired to enterprise metrics)
- [PERSISTENCE.md](PERSISTENCE.md) — SQLite schema and store architecture (simple tier)
- [PLUGINS.md](PLUGINS.md) — Plugin system (reused Ed25519 crypto for licenses)
- [PLUGIN_SECURITY.md](PLUGIN_SECURITY.md) — Plugin signing (same crypto as license signing)
- [ACCESS_CONTROL.md](ACCESS_CONTROL.md) — IP allowlist (complements auth middleware)
