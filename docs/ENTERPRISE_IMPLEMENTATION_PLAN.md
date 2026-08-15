# Enterprise Implementation Plan

> **All 13 phases (0–12) are complete.** This document is retained as a
> historical record of the implementation plan. For the current state of
> the enterprise crate, see [ENTERPRISE_CRATE_GUIDE.md](ENTERPRISE_CRATE_GUIDE.md)
> and [ENTERPRISE_STARTUP_FLOW.md](ENTERPRISE_STARTUP_FLOW.md).

> Part of: [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md)

This document synthesizes the findings from all 12 enterprise
analysis documents into a single, ordered, actionable implementation
plan. It covers every work stream — crate extraction, storage
migration, auth/RBAC, licensing, web UI, AI agent integration,
multi-instance, CI/CD, security hardening, and database optimization
— with dependencies, milestones, and verification criteria.

---

## Table of Contents

1. [Guiding Principles](#1-guiding-principles)
2. [Work Stream Summary](#2-work-stream-summary)
3. [Dependency Graph](#3-dependency-graph)
4. [Phase 0: Preparation](#4-phase-0-preparation)
5. [Phase 1: Crate Extraction](#5-phase-1-crate-extraction)
6. [Phase 2: Storage Migration (rusqlite → sqlx)](#6-phase-2-storage-migration-rusqlite--sqlx)
7. [Phase 3: License Verification](#7-phase-3-license-verification)
8. [Phase 4: Auth, RBAC, and Audit](#8-phase-4-auth-rbac-and-audit)
9. [Phase 5: PostgreSQL Backends](#9-phase-5-postgresql-backends)
10. [Phase 6: Multi-Instance Infrastructure](#10-phase-6-multi-instance-infrastructure)
11. [Phase 7: Web UI Enterprise Features](#11-phase-7-web-ui-enterprise-features)
12. [Phase 8: AI Agent Integration](#12-phase-8-ai-agent-integration)
13. [Phase 9: Security Hardening](#13-phase-9-security-hardening)
14. [Phase 10: Database Optimization](#14-phase-10-database-optimization)
15. [Phase 11: CI/CD Two-Tier Pipeline](#15-phase-11-cicd-two-tier-pipeline)
16. [Phase 12: Licensing Server](#16-phase-12-licensing-server)
17. [Milestone Summary](#17-milestone-summary)
18. [Effort Estimates](#18-effort-estimates)
19. [Risk Register](#19-risk-register)
20. [Verification Checklist](#20-verification-checklist)

---

## 1. Guiding Principles

1. **Every phase produces a working build.** No long-lived branches
   with broken intermediate states. Each phase ends with `cargo build
   --release` passing for both OSS and enterprise configurations.

2. **OSS tier is never broken.** The default build (`cargo build -p
   madhyamas --no-default-features`) must produce a functional
   localhost proxy with zero enterprise code. This is verified at
   every phase boundary.

3. **Smallest blast radius first.** Migrate the smallest, most
   isolated components before the largest. Within storage migration,
   ConfigStore (7 refs) before TrafficStore (35 refs). Within crate
   extraction, trait abstractions before code deletion.

4. **Stubs before real implementations.** Where a phase requires new
   functionality (e.g., API key validation), first wire the route with
   a stub, then implement the real logic. This keeps the build green
   and allows incremental testing.

5. **One repo, two builds.** OSS and enterprise live in the same
   repository. The `enterprise` feature flag on the main binary pulls
   in the `madhyamas-enterprise` crate. No separate repos, no
   forks.

6. **Different licenses, same repo.** OSS crates remain MIT OR
   Apache-2.0. The `madhyamas-enterprise` crate uses BSL 1.1. The
   default build is pure MIT/Apache; the enterprise build links BSL
   code.

---

## 2. Work Stream Summary

| ID | Work stream | Doc | Effort | Phases |
|---|---|---|---|---|
| WS-1 | Crate extraction | [ENTERPRISE_CRATE_MIGRATION.md](ENTERPRISE_CRATE_MIGRATION.md) | Medium-large | 1 |
| WS-2 | Storage migration (rusqlite → sqlx) | [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) | Large | 2, 5 |
| WS-3 | License verification | [ENTERPRISE_OVERVIEW.md §4](ENTERPRISE_OVERVIEW.md) | Medium | 3 |
| WS-4 | Auth, RBAC, audit | [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md) | Large | 4 |
| WS-5 | Multi-instance | [ENTERPRISE_MULTI_INSTANCE.md](ENTERPRISE_MULTI_INSTANCE.md) | Large | 6 |
| WS-6 | Web UI | [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md) | Large | 7 |
| WS-7 | AI agent integration | [ENTERPRISE_AI_AGENTS.md](ENTERPRISE_AI_AGENTS.md) | Medium | 8 |
| WS-8 | Security hardening | [ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md) | Medium | 9 |
| WS-9 | Database optimization | [ENTERPRISE_PERF_SECURITY.md §6](ENTERPRISE_PERF_SECURITY.md) | Medium | 10 |
| WS-10 | CI/CD | [ENTERPRISE_CICD.md](ENTERPRISE_CICD.md) | Medium | 11 |
| WS-11 | Licensing server | [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md) | Large | 12 |

---

## 3. Dependency Graph

```mermaid
graph TD
    P0["Phase 0<br/>Preparation"]
    P1["Phase 1<br/>Crate Extraction"]
    P2["Phase 2<br/>Storage: rusqlite → sqlx"]
    P3["Phase 3<br/>License Verification"]
    P4["Phase 4<br/>Auth, RBAC, Audit"]
    P5["Phase 5<br/>PostgreSQL Backends"]
    P6["Phase 6<br/>Multi-Instance"]
    P7["Phase 7<br/>Web UI Enterprise"]
    P8["Phase 8<br/>AI Agent Integration"]
    P9["Phase 9<br/>Security Hardening"]
    P10["Phase 10<br/>Database Optimization"]
    P11["Phase 11<br/>CI/CD Two-Tier"]
    P12["Phase 12<br/>Licensing Server"]

    P0 --> P1
    P1 --> P2
    P1 --> P3
    P1 --> P4
    P2 --> P5
    P3 --> P4
    P4 --> P5
    P4 --> P7
    P4 --> P8
    P5 --> P6
    P6 --> P9
    P6 --> P10
    P7 --> P9
    P8 --> P9
    P1 --> P11
    P3 --> P12
    P4 --> P12

    style P0 fill:#4a4,stroke:#333
    style P1 fill:#4a4,stroke:#333
    style P2 fill:#fa4,stroke:#333
    style P3 fill:#4a4,stroke:#333
    style P4 fill:#fa4,stroke:#333
    style P5 fill:#f44,stroke:#333
    style P6 fill:#f44,stroke:#333
    style P7 fill:#fa4,stroke:#333
    style P8 fill:#4a4,stroke:#333
    style P9 fill:#fa4,stroke:#333
    style P10 fill:#f44,stroke:#333
    style P11 fill:#4a4,stroke:#333
    style P12 fill:#f44,stroke:#333
```

### Critical path

```
Phase 0 → Phase 1 → Phase 2 → Phase 5 → Phase 6 → Phase 10
```

This is the longest chain: crate extraction enables storage migration,
which enables PostgreSQL, which enables multi-instance, which enables
database optimization. Every other work stream can proceed in parallel
once its dependencies are met.

### Parallelizable work

| Can run in parallel | After phase |
|---|---|
| Phase 3 (license) + Phase 4 (auth) | Phase 1 |
| Phase 7 (web UI) + Phase 8 (AI agents) | Phase 4 |
| Phase 9 (security) + Phase 10 (DB optimization) | Phase 6 |
| Phase 11 (CI/CD) + Phase 12 (licensing server) | Phase 1 / Phase 3 |

---

## 4. Phase 0: Preparation

**Goal:** Verify the current state is clean and establish baselines.

**Source:** [ENTERPRISE_CRATE_MIGRATION.md §10](ENTERPRISE_CRATE_MIGRATION.md#10-migration-steps-ordered)

### Steps

| Step | Action | Verification |
|---|---|---|
| 0.1 | Run `cargo build --release` (default features, includes enterprise) | Builds successfully |
| 0.2 | Run `cargo build --release --no-default-features` (OSS only) | Builds successfully |
| 0.3 | Run `cargo test --all-features` | All tests pass |
| 0.4 | Run `cargo clippy --all-targets --all-features` | No warnings |
| 0.5 | Record binary sizes for both builds | Baseline for comparison |
| 0.6 | Run `cargo tree -p madhyamas --features enterprise` and record deps | Baseline for dep audit |
| 0.7 | Verify `strings target/release/madhyamas \| grep -c enterprise` | Count enterprise strings in binary |

### Exit criteria

- Both builds pass
- All tests pass
- Clippy is clean
- Baselines recorded

---

## 5. Phase 1: Crate Extraction

**Goal:** Extract all enterprise code into a separate
`madhyamas-enterprise` crate. Remove all 17 `#[cfg]` gates from
`madhyamas-core` and `madhyamas-api`. Replace with trait abstractions
and `Option<Arc<dyn Trait>>` fields on `AppState`.

**Source:** [ENTERPRISE_CRATE_MIGRATION.md](ENTERPRISE_CRATE_MIGRATION.md)

### Sub-phases

#### 1a: Create trait abstractions in `madhyamas-api`

| Step | Action | Files |
|---|---|---|
| 1a.1 | Create `madhyamas-api/src/auth.rs` with `AuthProvider`, `Authorizer`, `AuditSink` traits, `Identity`, `AuthMethod` types | New file |
| 1a.2 | Add `auth_provider`, `authorizer`, `audit_sink` fields to `AppState` as `Option<Arc<dyn Trait>>` | `lib.rs` |
| 1a.3 | Add builder methods: `with_auth_provider()`, `with_authorizer()`, `with_audit_sink()` | `lib.rs` |
| 1a.4 | Export traits and types from `lib.rs` | `lib.rs` |
| 1a.5 | Verify build compiles | — |

#### 1b: Create `madhyamas-enterprise` crate

| Step | Action | Files |
|---|---|---|
| 1b.1 | Create `crates/madhyamas-enterprise/Cargo.toml` (BSL-1.1 license, `publish = false`) | New file |
| 1b.2 | Add `crates/madhyamas-enterprise` to workspace `members` | `Cargo.toml` |
| 1b.3 | Create `src/lib.rs` with module declarations and `EnterpriseState` struct | New file |
| 1b.4 | Copy `enterprise_error.rs` → `src/error.rs` | New file |
| 1b.5 | Copy `auth.rs` from core → `src/auth.rs` | New file |
| 1b.6 | Copy `rbac.rs` from core → `src/rbac.rs` | New file |
| 1b.7 | Copy `audit.rs` from core → `src/audit.rs` | New file |
| 1b.8 | Copy `user.rs` from core → `src/user.rs` | New file |
| 1b.9 | Copy `enterprise_handlers.rs` from api → `src/handlers.rs`, update imports | New file |
| 1b.10 | Copy `middleware.rs` from api → `src/middleware.rs`, update imports | New file |
| 1b.11 | Extract enterprise route block from `routes.rs` → `src/router.rs` | New file |
| 1b.12 | Implement `AuthProvider` for `AuthManager` | `auth.rs` |
| 1b.13 | Implement `Authorizer` for `RbacManager` | `rbac.rs` |
| 1b.14 | Implement `AuditSink` for `AuditLogger` | `audit.rs` |
| 1b.15 | Verify enterprise crate compiles standalone (`cargo build -p madhyamas-enterprise`) | — |

#### 1c: Remove enterprise from `madhyamas-core`

| Step | Action | Files |
|---|---|---|
| 1c.1 | Delete `crates/madhyamas-core/src/enterprise/` directory | 6 files deleted |
| 1c.2 | Remove `#[cfg(feature = "enterprise")] pub mod enterprise;` | `lib.rs` |
| 1c.3 | Remove `Enterprise` variant from `Error` enum | `lib.rs` |
| 1c.4 | Remove `enterprise` feature from `Cargo.toml` | `Cargo.toml` |
| 1c.5 | Remove `jsonwebtoken` from `[dependencies]` | `Cargo.toml` |
| 1c.6 | Verify `cargo build -p madhyamas-core` compiles | — |

#### 1d: Remove enterprise from `madhyamas-api`

| Step | Action | Files |
|---|---|---|
| 1d.1 | Delete `crates/madhyamas-api/src/enterprise_handlers.rs` | 1 file deleted |
| 1d.2 | Delete `crates/madhyamas-api/src/middleware.rs` | 1 file deleted |
| 1d.3 | Remove enterprise imports from `routes.rs` (3 `use` statements) | `routes.rs` |
| 1d.4 | Remove `create_routes_with_enterprise()` function | `routes.rs` |
| 1d.5 | Remove enterprise route block (lines 468-560) | `routes.rs` |
| 1d.6 | Remove all 6 `#[cfg]` gates from `lib.rs` | `lib.rs` |
| 1d.7 | Remove `enterprise` feature from `Cargo.toml` | `Cargo.toml` |
| 1d.8 | Verify `cargo build -p madhyamas-api` compiles | — |

#### 1e: Wire enterprise crate into main binary

| Step | Action | Files |
|---|---|---|
| 1e.1 | Add `madhyamas-enterprise` as optional dependency | `madhyamas/Cargo.toml` |
| 1e.2 | Change `enterprise` feature to `["dep:madhyamas-enterprise"]` | `madhyamas/Cargo.toml` |
| 1e.3 | Add `#[cfg(feature = "enterprise")]` block in `main.rs` to construct `EnterpriseState` | `main.rs` |
| 1e.4 | Inject enterprise trait impls into `AppState` via builder methods | `main.rs` |
| 1e.5 | Merge enterprise router with core router | `main.rs` |
| 1e.6 | Add CLI flags: `--enable-auth`, `--jwt-secret`, `--license-file` | `main.rs` |
| 1e.7 | Verify enterprise build: `cargo build --release` | — |
| 1e.8 | Verify OSS build: `cargo build --release --no-default-features` | — |

### Exit criteria

- `#[cfg(feature = "enterprise")]` count: 17 → ~5 (only in `main.rs`)
- `madhyamas-core` has zero enterprise code
- `madhyamas-api` has zero enterprise code (only trait definitions)
- `madhyamas-enterprise` crate compiles standalone
- OSS build has no enterprise code (`strings` check)
- Enterprise build starts and mounts enterprise routes

---

## 6. Phase 2: Storage Migration (rusqlite → sqlx)

**Goal:** Migrate all storage from `rusqlite` (sync, `Mutex<Connection>`)
to `sqlx` (async, connection pooling). This is a prerequisite for
PostgreSQL support and multi-instance.

**Source:** [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md)

### Sub-phases

#### 2a: Enterprise store (new code, no migration)

| Step | Action |
|---|---|
| 2a.1 | Create `madhyamas-enterprise/src/store/` with `EnterpriseStore` trait |
| 2a.2 | Implement `SqliteEnterpriseStore` using `sqlx::SqlitePool` |
| 2a.3 | Wire enterprise handlers to use the store (replace in-memory stubs) |
| 2a.4 | Verify enterprise build compiles and handlers return real data |

#### 2b: Define core storage traits

| Step | Action |
|---|---|
| 2b.1 | Create `madhyamas-core/src/storage/mod.rs` with trait definitions: `TrafficStoreBackend`, `ConfigStoreBackend`, `InterceptStoreBackend`, `PluginStoreBackend`, `ScriptStoreBackend` |
| 2b.2 | Define async methods matching current sync APIs |
| 2b.3 | Verify build compiles (traits exist but nothing uses them yet) |

#### 2c: Migrate SQLite stores (smallest first)

Migrate one store at a time. Each sub-step produces a working build.

| Order | Store | rusqlite refs | Lines | Steps |
|---|---|---|---|---|
| 1 | ConfigStore | 7 | ~220 | Create `SqliteConfigStore`, update `AppState`, update callers to `.await`, remove old |
| 2 | InterceptStore | 22 | ~600 | Same pattern |
| 3 | PluginStore | 13 | ~350 | Same pattern |
| 4 | ScriptStore | 20 | ~500 | Same pattern |
| 5 | TrafficStore | 35 | ~1700 | Create `SqliteTrafficStore`, update 15+ proxy call sites, update 30+ API handler call sites, update `SessionManager`, remove old |

#### 2d: Remove rusqlite

| Step | Action |
|---|---|
| 2d.1 | Remove `rusqlite` from all `Cargo.toml` files |
| 2d.2 | Verify OSS build compiles with `sqlx::SqlitePool` only |
| 2d.3 | Verify enterprise build compiles |
| 2d.4 | Run all tests |

### Exit criteria

- Zero `rusqlite` references in codebase
- All stores use `sqlx::SqlitePool` (async)
- `AppState` holds `Arc<dyn StoreBackend>` trait objects
- OSS build works identically to before (same SQLite file, same behavior)
- All tests pass

---

## 7. Phase 3: License Verification

**Goal:** Implement Ed25519 license verification so the enterprise
binary can validate a license file at startup.

**Source:** [ENTERPRISE_OVERVIEW.md §4](ENTERPRISE_OVERVIEW.md), [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md)

### Steps

| Step | Action | Files |
|---|---|---|
| 3.1 | Add `ed25519-dalek` dependency to `madhyamas-enterprise` | `Cargo.toml` |
| 3.2 | Create `madhyamas-enterprise/src/license.rs` with `License`, `LicenseClaims`, `LicenseVerifier` types | New file |
| 3.3 | Implement license file format: JSON payload + Ed25519 signature (base64) | `license.rs` |
| 3.4 | Embed Ed25519 public key at compile time (from env var or file) | `license.rs` |
| 3.5 | Implement `LicenseVerifier::verify(file_path)` — parse, verify signature, check expiry, check instance ID | `license.rs` |
| 3.6 | Add `--license-file` CLI flag to main binary | `main.rs` |
| 3.7 | At startup: if `--license-file` is provided, verify it; fail fast if invalid | `main.rs` |
| 3.8 | Add license info endpoint: `GET /api/license` (returns plan, seats, expiry) | `handlers.rs` |
| 3.9 | Add license health check: include license status in `GET /api/health/detailed` | `handlers.rs` |
| 3.10 | Write tests: valid license, expired license, tampered license, wrong key | `license.rs` |

### License file format

```json
{
  "license_id": "lic_abc123",
  "customer": "Acme Corp",
  "plan": "enterprise",
  "seats": 50,
  "instance_id": "inst_xyz789",
  "issued_at": "2026-01-01T00:00:00Z",
  "expires_at": "2027-01-01T00:00:00Z",
  "features": ["auth", "rbac", "audit", "multi_instance", "oidc"],
  "signature": "base64_ed25519_signature_of_canonical_json"
}
```

### Exit criteria

- Enterprise binary refuses to start without a valid license
- License expiry is checked at startup and periodically
- License info is available via API
- Tampered/expired licenses are rejected with clear error messages

---

## 8. Phase 4: Auth, RBAC, and Audit

**Goal:** Replace in-memory stubs with real, persistent implementations
backed by SQLite (Phase 4) and PostgreSQL (Phase 5). Implement
password hashing, JWT with proper validation, API key management with
scopes, RBAC enforcement on all routes, and audit logging.

**Source:** [ENTERPRISE_AUTH_RBAC.md](ENTERPRISE_AUTH_RBAC.md), [ENTERPRISE_AI_AGENTS.md §5-7](ENTERPRISE_AI_AGENTS.md)

### Sub-phases

#### 4a: User management with persistence

| Step | Action |
|---|---|
| 4a.1 | Add `argon2` dependency to enterprise crate |
| 4a.2 | Create `users` table migration (id, username, email, password_hash, role, status, created_at, last_login, preferences) |
| 4a.3 | Implement `UserManager` with SQLite persistence (create, get, list, update, delete) |
| 4a.4 | Implement password hashing with Argon2id |
| 4a.5 | Implement bootstrap: first-run creates admin user from CLI flag or interactive prompt |
| 4a.6 | Wire user management handlers (replace 501 stubs) |
| 4a.7 | Write tests: create user, authenticate, update role, delete user |

#### 4b: JWT authentication with proper validation

| Step | Action |
|---|---|
| 4b.1 | Fix JWT validation: explicitly set `algorithms: [HS256]` to prevent algorithm confusion (security gap #2) |
| 4b.2 | Add clock skew tolerance (±60 seconds, security gap #1) |
| 4b.3 | Implement `POST /api/auth/login` — validate credentials, return JWT |
| 4b.4 | Implement `POST /api/auth/logout` — invalidate session |
| 4b.5 | Implement `GET /api/auth/me` — return current user from JWT |
| 4b.6 | Implement `POST /api/auth/validate` — validate token, return claims |
| 4b.7 | Implement refresh token flow (long-lived refresh token → short-lived access token) |
| 4b.8 | Add session idle timeout (30min, security gap #15) |
| 4b.9 | Write tests: login success, login failure, token refresh, expired token, revoked session |

#### 4c: API key management with scopes

| Step | Action |
|---|---|
| 4c.1 | Create `api_keys` table migration (id, user_id, name, key_hash, key_prefix, scopes, expires_at, last_used_at, created_at) |
| 4c.2 | Implement `AuthManager::validate_api_key()` — hash input, lookup, check expiry, check scope |
| 4c.3 | Implement `POST /api/auth/api-keys` — create key, return plaintext once |
| 4c.4 | Implement `GET /api/auth/api-keys` — list keys (show prefix only, not full key) |
| 4c.5 | Implement `DELETE /api/auth/api-keys/{id}` — revoke key |
| 4c.6 | Add `X-API-Key` header branch to `auth_middleware` (security gap #16, AI agent gap G3) |
| 4c.7 | Add `?api_key=` query param branch to `auth_middleware` |
| 4c.8 | Implement scope format: `"<resource>:<permission>"` (e.g., `traffic:read`, `mocks:write`, `*` for admin) |
| 4c.9 | Enforce scopes in auth middleware: check required scope per route |
| 4c.10 | Write tests: create key, use key, expired key, insufficient scope, revoke key |

#### 4d: RBAC enforcement

| Step | Action |
|---|---|
| 4d.1 | Wire `require_permission_middleware` to enterprise routes |
| 4d.2 | Define route → permission mapping (e.g., `GET /api/traffic` → `traffic:read`, `POST /api/mocks` → `mocks:write`) |
| 4d.3 | Apply permission middleware to core routes (traffic, mocks, rewrites, etc.) when enterprise is enabled |
| 4d.4 | Implement `GET /api/rbac/roles` — list roles and their permissions |
| 4d.5 | Implement `GET /api/rbac/permissions` — list all permissions |
| 4d.6 | Implement `POST /api/rbac/check` — check if user has permission |
| 4d.7 | Write tests: admin can delete, viewer cannot delete, read-only cannot write |

#### 4e: Audit logging with persistence

| Step | Action |
|---|---|
| 4e.1 | Create `audit_events` table migration (id, event_type, timestamp, user_id, api_key_id, client_ip, description, metadata) |
| 4e.2 | Implement `AuditLogger` with SQLite persistence (replace in-memory ring buffer) |
| 4e.3 | Wire audit logging into auth middleware (log every authenticated request) |
| 4e.4 | Wire audit logging into mutation handlers (mock create/delete, config change, user create/delete) |
| 4e.5 | Implement `GET /api/audit` — query with filters (user, type, time range, pagination) |
| 4e.6 | Implement `GET /api/audit/stats` — aggregate statistics |
| 4e.7 | Implement `GET /api/audit/export` — export as JSON or CSV |
| 4e.8 | Implement `DELETE /api/audit/clear` — clear old events (admin only) |
| 4e.9 | Add audit log integrity: hash chain (each event includes hash of previous event, security gap #13) |
| 4e.10 | Write tests: log event, query events, filter by user, export, hash chain verification |

### Exit criteria

- Users can log in with username/password and receive JWT
- API keys can be created, used, scoped, and revoked
- RBAC is enforced on all routes (admin/user/viewer/read-only roles)
- All authenticated and mutation actions are audited
- Audit log has integrity protection (hash chain)
- All auth handlers return real data (no 501s)

---

## 9. Phase 5: PostgreSQL Backends

**Goal:** Implement PostgreSQL backends for all stores. The enterprise
binary selects PostgreSQL or SQLite at startup based on configuration.

**Source:** [ENTERPRISE_STORAGE_TRAITS.md Phase D](ENTERPRISE_STORAGE_TRAITS.md), [ENTERPRISE_PERF_SECURITY.md §6](ENTERPRISE_PERF_SECURITY.md)

### Steps

| Step | Action |
|---|---|
| 5.1 | Add `sqlx` PostgreSQL feature to enterprise crate |
| 5.2 | Create `PostgresTrafficStore` implementing `TrafficStoreBackend` |
| 5.3 | Create `PostgresConfigStore` implementing `ConfigStoreBackend` |
| 5.4 | Create `PostgresInterceptStore` implementing `InterceptStoreBackend` |
| 5.5 | Create `PostgresPluginStore` implementing `PluginStoreBackend` |
| 5.6 | Create `PostgresScriptStore` implementing `ScriptStoreBackend` |
| 5.7 | Create `PostgresEnterpriseStore` for users, audit, API keys |
| 5.8 | Create optimized PostgreSQL schema (tiered body storage, GIN/BRIN/trigram indexes, partitioning) per [ENTERPRISE_PERF_SECURITY.md §6](ENTERPRISE_PERF_SECURITY.md) |
| 5.9 | Add `--database-url` CLI flag (e.g., `postgres://user:pass@host:5432/madhyamas`) |
| 5.10 | At startup: if `--database-url` starts with `postgres://`, use Pg backends; if `sqlite://`, use SQLite backends |
| 5.11 | Run migrations on startup (with advisory lock to prevent race, multi-instance issue #10) |
| 5.12 | Write tests: Pg traffic store CRUD, Pg config store, Pg enterprise store |

### PostgreSQL schema highlights

From [ENTERPRISE_PERF_SECURITY.md §6](ENTERPRISE_PERF_SECURITY.md):

| Optimization | Phase | Impact |
|---|---|---|
| Tiered body storage (inline/TOAST/S3) | DB-1 | Prevents table bloat |
| zstd body compression | DB-1 | 5-10x storage reduction |
| GIN index on headers (JSONB) | DB-1 | Header filter: O(n) → O(log n) |
| Trigram index on URL/path | DB-1 | Substring search: O(log n) |
| BRIN index on timestamp | DB-1 | 10x smaller than B-tree |
| Write batching (100 entries / 500ms) | DB-2 | 100x fewer round-trips |
| Cursor-based pagination | DB-2 | O(1) regardless of page depth |
| Session counter (eliminate COUNT(*)) | DB-2 | Eliminates 1000 queries/sec |
| Table partitioning (weekly) | DB-3 | Fast pruning, parallel scan |
| PgBouncer | DB-3 | Supports 50+ instances |
| Read replicas | DB-4 | Offload read traffic |

### Exit criteria

- Enterprise binary can use PostgreSQL as its database
- All stores work with both SQLite and PostgreSQL
- PostgreSQL schema includes optimized indexes
- Startup migrations work with advisory lock
- Performance: 1000+ req/sec traffic capture with PostgreSQL

---

## 10. Phase 6: Multi-Instance Infrastructure

**Goal:** Enable multiple Madhyamas instances behind a load balancer
with shared state (PostgreSQL + Redis).

**Source:** [ENTERPRISE_MULTI_INSTANCE.md](ENTERPRISE_MULTI_INSTANCE.md)

### Steps

#### 6a: Redis for cross-instance state

| Step | Action |
|---|---|
| 6a.1 | Add `redis` (or `fred`) dependency to enterprise crate |
| 6a.2 | Implement Redis pub/sub for WebSocket event broadcasting (issue #2) |
| 6a.3 | Implement Redis-based config propagation: `PATCH /api/config` writes to PG + publishes config update event (issue #6) |
| 6a.4 | Implement Redis-based intercept rule sync: mock/rewrite/breakpoint/throttle changes propagate to all instances (issue #3) |
| 6a.5 | Add Redis auth and TLS support (security gap #10) |
| 6a.6 | Add `--redis-url` CLI flag |
| 6a.7 | Write tests: config change propagates, intercept rule propagates, WS event broadcasts |

#### 6b: Shared CA certificate

| Step | Action |
|---|---|
| 6b.1 | Add `--ca-cert-file` and `--ca-key-file` CLI flags |
| 6b.2 | At startup: if CA files exist, load them; if not, generate and optionally write to shared volume (issue #5) |
| 6b.3 | Document CA sharing via Kubernetes Secret or shared volume |
| 6b.4 | Write tests: two instances share same CA, intercepted TLS works across both |

#### 6c: License seat coordination

| Step | Action |
|---|---|
| 6c.1 | Implement Redis-based seat tracking: instance registers on startup, heartbeat every 60s, deregisters on shutdown (issue #9) |
| 6c.2 | At startup: check current active seats against license limit; fail if exceeded |
| 6c.3 | Implement graceful seat release on SIGTERM (issue #15) |
| 6c.4 | Write tests: seat registration, seat limit enforcement, seat release on shutdown |

#### 6d: Load balancer support

| Step | Action |
|---|---|
| 6d.1 | Add configurable base path (`--base-path /madhyamas`) for context-path routing (issue #16) |
| 6d.2 | Update Vite `base` config for frontend builds |
| 6d.3 | Update API client to use configurable base path |
| 6d.4 | Add WebSocket sticky session support documentation |
| 6d.5 | Add `GET /api/health/detailed` with dependency checks (DB, Redis, license) (issue #14) |
| 6d.6 | Implement graceful shutdown: SIGTERM handler closes WS connections, releases seats, flushes audit log (issue #15) |
| 6d.7 | Create Kubernetes manifests (Deployment, Service, Ingress, ConfigMap, Secret) |
| 6d.8 | Create Docker Compose for multi-instance local testing |
| 6d.9 | Write tests: health check with downed Redis, graceful shutdown sequence |

#### 6e: Cluster metrics

| Step | Action |
|---|---|
| 6e.1 | Implement `InstanceRegistry` using Redis (instance ID, address, heartbeat, metrics) |
| 6e.2 | Implement `GET /api/metrics/cluster` — aggregate metrics from all instances |
| 6e.3 | Implement `GET /api/instances` — list active instances (admin only) |
| 6e.4 | Write tests: instance registration, cluster metrics aggregation |

### Exit criteria

- Two instances behind a load balancer share traffic data
- Config changes on one instance propagate to all
- WebSocket events from any instance reach clients on any instance
- License seat count is coordinated across instances
- Graceful shutdown releases seats and closes connections
- Health check verifies DB, Redis, and license dependencies

---

## 11. Phase 7: Web UI Enterprise Features

**Goal:** Add login page, user menu, admin panels, and tier-aware UI
to the web frontend. Uses same-folder, runtime-gated approach.

**Source:** [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md)

### Sub-phases

#### 7a: Tier detection + auth infrastructure

| Step | Action |
|---|---|
| 7a.1 | Add `GET /api/health/detailed` response with `tier: "enterprise" \| "community"` |
| 7a.2 | Create `web/src/contexts/TierContext.tsx` — fetches tier on app load |
| 7a.3 | Create `web/src/features/auth/AuthContext.tsx` — manages JWT, login/logout, current user |
| 7a.4 | Create `web/src/features/auth/api.ts` — API client for auth endpoints |
| 7a.5 | Create `web/src/features/auth/ProtectedApp.tsx` — wraps app, redirects to login if not authenticated |
| 7a.6 | Create `web/src/features/auth/LoginPage.tsx` — username/password form |
| 7a.7 | Add API client interceptor: attach `Authorization: Bearer` header to all requests |
| 7a.8 | Add API client interceptor: on 401, redirect to login |
| 7a.9 | Write tests: tier detection, login flow, 401 redirect |

#### 7b: Shell changes

| Step | Action |
|---|---|
| 7b.1 | Add `UserMenu` component to `AppHeader` (shows username, role, logout) |
| 7b.2 | Add admin nav items (Users, Audit, Metrics, License) — visible only to admin role |
| 7b.3 | Add enterprise badge to header (shows "Enterprise" when tier is enterprise) |
| 7b.4 | Add session timeout warning (5 min before idle timeout) |
| 7b.5 | Write tests: user menu renders, admin nav hidden for non-admin, badge shows correct tier |

#### 7c: Admin panels

| Step | Action |
|---|---|
| 7c.1 | Create `web/src/features/admin/UsersPanel.tsx` — user list, create, edit, delete |
| 7c.2 | Create `web/src/features/admin/AuditPanel.tsx` — audit event list with filters, export |
| 7c.3 | Create `web/src/features/admin/MetricsPanel.tsx` — performance metrics dashboard |
| 7c.4 | Create `web/src/features/admin/LicensePanel.tsx` — license info, seats, expiry |
| 7c.5 | Create `web/src/features/admin/ApiKeysPanel.tsx` — API key management |
| 7c.6 | Create `web/src/features/admin/InstancesPanel.tsx` — multi-instance overview |
| 7c.7 | Add route definitions for admin panels (lazy-loaded, admin-only) |
| 7c.8 | Write tests: each panel renders, CRUD operations work, permission denied for non-admin |

#### 7d: Build configuration

| Step | Action |
|---|---|
| 7d.1 | Configure Vite to build enterprise JS chunks separately (lazy-loaded) |
| 7d.2 | Ensure enterprise JS chunks don't leak feature names (security gap #7) |
| 7d.3 | Add `MADHYAMAS_TIER` env var for build-time hints (optional, runtime detection is primary) |
| 7d.4 | Verify OSS build: enterprise chunks not loaded, no enterprise UI visible |
| 7d.5 | Verify enterprise build: login page shown, admin panels accessible |

### Exit criteria

- OSS build shows no enterprise UI (no login, no admin panels)
- Enterprise build shows login page, redirects unauthenticated users
- Admin panels work (users, audit, metrics, license, API keys, instances)
- Enterprise JS chunks are lazy-loaded and don't leak in OSS build
- Session timeout warning works

---

## 12. Phase 8: AI Agent Integration

**Goal:** Enable AI agents (Claude, Cursor, Devin, etc.) to
authenticate and use Madhyamas in enterprise deployments.

**Source:** [ENTERPRISE_AI_AGENTS.md](ENTERPRISE_AI_AGENTS.md)

### Sub-phases

#### 8a: MCP server auth (critical)

| Step | Action |
|---|---|
| 8a.1 | Add `McpAuth` enum to `McpConfig` (ApiKey, Jwt, Oidc variants) |
| 8a.2 | Add `auth_headers()` method to `McpConfig` |
| 8a.3 | Update `McpTool::execute()` signature to accept `auth_headers: &[(String, String)]` |
| 8a.4 | Update all 135 MCP tool implementations to inject auth headers on every HTTP request |
| 8a.5 | Add `MADHYAMAS_API_KEY` env var support to `madhyamas mcp` command |
| 8a.6 | Write tests: MCP tool call with API key auth, MCP tool call with JWT auth |

#### 8b: CLI auth

| Step | Action |
|---|---|
| 8b.1 | Add `CliAuth` enum to CLI `ApiClient` |
| 8b.2 | Add `--api-key` and `--token` CLI flags |
| 8b.3 | Add `MADHYAMAS_API_KEY` and `MADHYAMAS_TOKEN` env var support |
| 8b.4 | Update all CLI HTTP methods (get, post, put, delete) to inject auth headers |
| 8b.5 | Write tests: CLI command with API key, CLI command without auth (OSS mode) |

#### 8c: Enterprise MCP tools

| Step | Action |
|---|---|
| 8c.1 | Implement tier detection in MCP server (call `/api/health/detailed` at startup) |
| 8c.2 | Conditionally register enterprise MCP tools when tier is enterprise |
| 8c.3 | Implement `madhyamas_list_users`, `madhyamas_create_user`, `madhyamas_delete_user`, `madhyamas_update_user_role` |
| 8c.4 | Implement `madhyamas_get_audit_events`, `madhyamas_export_audit` |
| 8c.5 | Implement `madhyamas_get_license_info`, `madhyamas_get_metrics`, `madhyamas_get_health` |
| 8c.6 | Implement `madhyamas_export_config`, `madhyamas_import_config` |
| 8c.7 | Update skill package (`skills/madhyamas/references/mcp-tools.md`) with enterprise tools |
| 8c.8 | Write tests: enterprise tool registration, enterprise tool execution with auth |

#### 8d: MCP protocol enhancements

| Step | Action |
|---|---|
| 8d.1 | Add Streamable HTTP transport to MCP server (`--transport http --port 3002`) |
| 8d.2 | Add tool annotations (readOnly, destructive, idempotent, required_permission) to all 135+ tools |
| 8d.3 | Add dynamic MCP resources (`madhyamas://session/{id}`, `madhyamas://traffic/{id}`, `madhyamas://mock/{id}`) |
| 8d.4 | Add 6 debugging prompts (debug-4xx, debug-5xx, find-auth-issues, mock-missing-endpoint, compare-staging-prod, audit-trail) |
| 8d.5 | Write tests: HTTP transport, dynamic resource reading, prompt generation |

#### 8e: Enterprise CLI commands

| Step | Action |
|---|---|
| 8e.1 | Implement `madhyamas users list/create/delete/update-role` |
| 8e.2 | Implement `madhyamas audit list/export/stats` |
| 8e.3 | Implement `madhyamas license info` |
| 8e.4 | Implement `madhyamas auth login/logout/api-keys` |
| 8e.5 | Update skill package (`skills/madhyamas/references/cli-commands.md`) with enterprise commands |
| 8e.6 | Write tests: each enterprise CLI command |

### Exit criteria

- MCP server sends auth headers on every tool call (no more 401)
- CLI sends auth headers on every command
- 11 enterprise MCP tools available when connected to enterprise proxy
- Streamable HTTP transport works for remote agents
- Tool annotations tell agents which tools are read-only/destructive
- Dynamic resources and prompts are available
- Enterprise CLI commands work

---

## 13. Phase 9: Security Hardening

**Goal:** Address all 16 security gaps identified in the performance
and security analysis.

**Source:** [ENTERPRISE_PERF_SECURITY.md §3](ENTERPRISE_PERF_SECURITY.md)

### Steps (ordered by severity)

| Step | Gap | Severity | Action |
|---|---|---|---|
| 9.1 | #6 WebSocket no auth | **High** | Add JWT validation on WS upgrade handshake; reject unauthenticated WS connections in enterprise mode |
| 9.2 | #10 Redis no auth/TLS | **High** | Require Redis password; support TLS connection to Redis; document `--redis-url rediss://:password@host:6379` |
| 9.3 | #3 Missing CSP headers | **High** | Add `Content-Security-Policy` header to all API responses; restrict script-src, connect-src, img-src |
| 9.4 | #1 JWT clock skew | Medium | Add ±60s clock skew tolerance to JWT validation (done in Phase 4b.2) |
| 9.5 | #2 JWT algorithm confusion | Medium | Explicitly set `algorithms: [HS256]` in JWT validation (done in Phase 4b.1) |
| 9.6 | #5 Proxy listener no auth | Medium | Add optional proxy auth: require API key or JWT for proxy CONNECT requests in enterprise mode |
| 9.7 | #8 SSRF for SSO callbacks | Medium | Validate OIDC callback URLs against allowlist; block private IP ranges |
| 9.8 | #13 Audit log integrity | Medium | Implement hash chain in audit log (done in Phase 4e.9) |
| 9.9 | #14 No password complexity | Medium | Enforce password policy: min 12 chars, 1 uppercase, 1 lowercase, 1 digit, 1 special |
| 9.10 | #15 No session idle timeout | Medium | Implement 30min idle timeout with 5min warning (done in Phase 4b.8, 7b.4) |
| 9.11 | #16 No API key scopes | Medium | Implement scoped API keys (done in Phase 4c.8) |
| 9.12 | #4 CSRF if cookie auth | Low | Add CSRF token for cookie-based auth (if added in future) |
| 9.13 | #7 Enterprise JS leak | Low | Lazy-load enterprise chunks, strip names in production build (done in Phase 7d.2) |
| 9.14 | #9 License replay | Low | Include instance ID in license; check instance ID matches on verification |
| 9.15 | #11 PG connection string exposure | Low | Read from env var or file; don't log connection string; use `--database-url-file` for file-based |
| 9.16 | #12 Enterprise binary public keys | None | Documented as non-issue (public keys are meant to be public) |

### Exit criteria

- WebSocket requires auth in enterprise mode
- Redis connection uses auth and TLS
- CSP headers on all responses
- Password complexity enforced
- Session idle timeout works
- API key scopes enforced
- Audit log has hash chain integrity
- All 16 security gaps addressed or documented as non-issues

---

## 14. Phase 10: Database Optimization

**Goal:** Optimize PostgreSQL for high-volume traffic capture
(1000+ req/sec). Implement tiered body storage, write batching,
optimized indexes, and partitioning.

**Source:** [ENTERPRISE_PERF_SECURITY.md §6](ENTERPRISE_PERF_SECURITY.md)

### Sub-phases

#### 10a: Schema and indexing (DB-1)

| Step | Action |
|---|---|
| 10a.1 | Create optimized `traffic_entries` table with tiered body storage (inline for <4KB, TOAST for 4KB-1MB, S3 for >1MB) |
| 10a.2 | Add zstd compression for body storage |
| 10a.3 | Create GIN index on `headers` (JSONB) |
| 10a.4 | Create trigram index on `url` and `path` |
| 10a.5 | Create BRIN index on `timestamp` |
| 10a.6 | Tune autovacuum settings for high-write tables |
| 10a.7 | Implement S3 body storage for large bodies (>1MB) |
| 10a.8 | Write benchmark: insert 100K entries, measure throughput |

#### 10b: Query optimization (DB-2)

| Step | Action |
|---|---|
| 10b.1 | Implement write batching: buffer 100 entries or 500ms, whichever comes first |
| 10b.2 | Implement session counter table (eliminate `COUNT(*)` on traffic) |
| 10b.3 | Implement cursor-based pagination (replace OFFSET) |
| 10b.4 | Implement lazy body loading (don't load body in list view) |
| 10b.5 | Implement WebSocket message batching (buffer 100 messages or 100ms) |
| 10b.6 | Write benchmark: list 10K entries with cursor pagination, measure latency |

#### 10c: Scale (DB-3)

| Step | Action |
|---|---|
| 10c.1 | Implement weekly table partitioning on `traffic_entries` |
| 10c.2 | Add `pg_partman` for automatic partition creation and retention |
| 10c.3 | Document PgBouncer configuration for connection pooling |
| 10c.4 | Write benchmark: 1M entries across 4 weekly partitions, measure query performance |

#### 10d: High availability (DB-4)

| Step | Action |
|---|---|
| 10d.1 | Document read replica setup |
| 10d.2 | Implement read/write split in store layer (writes to primary, reads to replica) |
| 10d.3 | Document PostgreSQL HA setup (Patroni or Stolon) |
| 10d.4 | Write failover test: primary goes down, reads continue from replica |

### Exit criteria

- 1000+ req/sec sustained traffic capture with PostgreSQL
- List view loads in <100ms with 1M entries
- Body storage uses 5-10x less space (zstd compression)
- Table partitioning enables efficient retention
- PgBouncer supports 50+ concurrent instances

---

## 15. Phase 11: CI/CD Two-Tier Pipeline

**Goal:** Update CI/CD to build both OSS and enterprise binaries,
publish OSS to crates.io, and publish enterprise to private registry.

**Source:** [ENTERPRISE_CICD.md](ENTERPRISE_CICD.md)

### Steps

| Step | Action |
|---|---|
| 11.1 | Add enterprise matrix dimension to `ci.yml` (build with and without `enterprise` feature) |
| 11.2 | Add enterprise-specific test job (auth, RBAC, audit, license tests) |
| 11.3 | Add enterprise Docker image build (separate tag: `madhyamas-enterprise:latest`) |
| 11.4 | Add cross-compile matrix for both tiers (Linux, macOS, Windows) |
| 11.5 | Add security audit job for enterprise dependencies (BSL-licensed crate) |
| 11.6 | Update release workflow: publish OSS to crates.io, publish enterprise to GitHub Releases (private) |
| 11.7 | Add SBOM generation for both tiers |
| 11.8 | Add license check: verify OSS build contains no BSL code |
| 11.9 | Document release process in `docs/ENTERPRISE_CICD.md` |
| 11.10 | Verify: PR CI runs both tiers, release creates both artifacts |

### Exit criteria

- CI builds both OSS and enterprise on every PR
- Enterprise tests run in CI
- Release workflow publishes both artifacts
- OSS artifact contains no BSL code (verified by CI)

---

## 16. Phase 12: Licensing Server

**Goal:** Build a separate licensing server for customer registration,
payment processing (Stripe), license issuance, and seat tracking.

**Source:** [ENTERPRISE_LICENSING_SERVER.md](ENTERPRISE_LICENSING_SERVER.md)

### Sub-phases

#### 12a: Core licensing server

| Step | Action |
|---|---|
| 12a.1 | Create `licensing-server/` workspace member |
| 12a.2 | Implement database schema (accounts, customers, licenses, seats, audit) |
| 12a.3 | Implement Ed25519 license signing (using `ed25519-dalek`) |
| 12a.4 | Implement license issuance API (create license, sign, return file) |
| 12a.5 | Implement license verification API (proxy binary calls this) |
| 12a.6 | Implement seat tracking API (register/deregister/heartbeat) |
| 12a.7 | Write tests: license issuance, verification, seat tracking |

#### 12b: Customer portal

| Step | Action |
|---|---|
| 12b.1 | Create React frontend for customer portal (login, license management, billing) |
| 12b.2 | Implement customer registration and authentication |
| 12b.3 | Implement license dashboard (view active licenses, seats, usage) |
| 12b.4 | Implement team management (invite users, assign roles) |
| 12b.5 | Write tests: registration, login, license view |

#### 12c: Stripe integration

| Step | Action |
|---|---|
| 12c.1 | Integrate Stripe Checkout for subscription signup |
| 12c.2 | Implement webhook handler for Stripe events (subscription created, payment failed, etc.) |
| 12c.3 | Implement automatic license creation on successful payment |
| 12c.4 | Implement license suspension on payment failure |
| 12c.5 | Implement billing portal (view invoices, update payment method) |
| 12c.6 | Write tests: checkout flow, webhook handling, license creation from payment |

#### 12d: Admin portal

| Step | Action |
|---|---|
| 12d.1 | Implement admin authentication (separate from customer auth) |
| 12d.2 | Implement customer management (list, search, suspend) |
| 12d.3 | Implement license management (create, revoke, extend) |
| 12d.4 | Implement revenue dashboard (MRR, churn, active licenses) |
| 12d.5 | Write tests: admin login, customer management, license operations |

#### 12e: Deployment

| Step | Action |
|---|---|
| 12e.1 | Create Dockerfile for licensing server |
| 12e.2 | Create docker-compose.yml for local development (server + PostgreSQL + Redis) |
| 12e.3 | Create Kubernetes manifests for production deployment |
| 12e.4 | Document Ed25519 key management (generate, store in secrets manager, rotate) |
| 12e.5 | Document backup and disaster recovery |
| 12e.6 | Write deployment guide |

### Exit criteria

- Licensing server issues and verifies Ed25519-signed licenses
- Customers can register, subscribe via Stripe, and receive licenses
- Seat tracking works with multiple proxy instances
- Admin portal manages customers and licenses
- Deployment is documented and reproducible

---

## 17. Milestone Summary

```mermaid
gantt
    title Enterprise Implementation Timeline
    dateFormat YYYY-MM-DD
    axisFormat %b %d

    section Foundation
    Phase 0: Preparation              :p0, 2026-01-01, 2d
    Phase 1: Crate Extraction         :p1, after p0, 10d

    section Storage
    Phase 2: rusqlite → sqlx          :p2, after p1, 25d
    Phase 5: PostgreSQL Backends      :p5, after p2, 14d

    section Auth
    Phase 3: License Verification     :p3, after p1, 7d
    Phase 4: Auth, RBAC, Audit        :p4, after p3, 20d

    section Scale
    Phase 6: Multi-Instance           :p6, after p5, 14d
    Phase 10: DB Optimization         :p10, after p6, 14d

    section UX
    Phase 7: Web UI Enterprise        :p7, after p4, 18d
    Phase 8: AI Agent Integration     :p8, after p4, 14d

    section Hardening
    Phase 9: Security Hardening       :p9, after p7, 10d

    section Delivery
    Phase 11: CI/CD Two-Tier          :p11, after p1, 7d
    Phase 12: Licensing Server        :p12, after p3, 30d
```

### Milestones

| Milestone | Phases complete | What's deliverable |
|---|---|---|
| **M1: Clean separation** | 0, 1 | Enterprise code in separate crate; OSS build has zero enterprise code |
| **M2: Async storage** | 2 | All stores use sqlx; rusqlite removed; OSS works identically |
| **M3: Licensed enterprise** | 3, 4 | Enterprise binary requires license; auth/RBAC/audit work with SQLite |
| **M4: PostgreSQL enterprise** | 5, 6 | Enterprise binary supports PostgreSQL; multi-instance works |
| **M5: Full enterprise UX** | 7, 8 | Web UI has login/admin panels; AI agents can authenticate |
| **M6: Production-ready** | 9, 10, 11 | Security hardened; DB optimized; CI/CD publishes both tiers |
| **M7: Commercial launch** | 12 | Licensing server operational; customers can self-serve |

---

## 18. Effort Estimates

Effort is expressed in **developer-days** (1 developer, 8h/day).
These are rough estimates for planning purposes, not commitments.

| Phase | Effort (dev-days) | Notes |
|---|---|---|
| 0: Preparation | 1 | Verify builds, record baselines |
| 1: Crate Extraction | 10 | 1a (2d) + 1b (4d) + 1c (1d) + 1d (1d) + 1e (2d) |
| 2: Storage Migration | 25 | 2a (3d) + 2b (2d) + 2c (18d) + 2d (2d); TrafficStore is 10d alone |
| 3: License Verification | 7 | Ed25519, license format, startup check |
| 4: Auth, RBAC, Audit | 20 | 4a (4d) + 4b (5d) + 4c (4d) + 4d (3d) + 4e (4d) |
| 5: PostgreSQL Backends | 14 | 5 stores + schema + migrations |
| 6: Multi-Instance | 14 | 6a (4d) + 6b (2d) + 6c (2d) + 6d (4d) + 6e (2d) |
| 7: Web UI Enterprise | 18 | 7a (5d) + 7b (3d) + 7c (8d) + 7d (2d) |
| 8: AI Agent Integration | 14 | 8a (3d) + 8b (1d) + 8c (4d) + 8d (4d) + 8e (2d) |
| 9: Security Hardening | 10 | 16 gaps, most are small fixes |
| 10: Database Optimization | 14 | 10a (4d) + 10b (4d) + 10c (3d) + 10d (3d) |
| 11: CI/CD Two-Tier | 7 | Matrix, Docker, release workflow |
| 12: Licensing Server | 30 | 12a (7d) + 12b (7d) + 12c (6d) + 12d (5d) + 12e (5d) |
| **Total** | **194 dev-days** | ~10 months for 1 developer |

### With 2 developers

| Track | Developer 1 | Developer 2 |
|---|---|---|
| Backend | Phases 0, 1, 2, 3, 4, 5, 6, 10 | — |
| Frontend + Infra | — | Phases 7, 8, 9, 11 |
| Licensing server | — | Phase 12 (after Phase 3) |

**Estimated timeline with 2 developers: ~6 months.**

### With 3 developers

Add a third developer focused on Phase 12 (licensing server) in
parallel from Phase 3 onward.

**Estimated timeline with 3 developers: ~5 months.**

---

## 19. Risk Register

| # | Risk | Probability | Impact | Mitigation |
|---|---|---|---|---|
| R1 | Phase 2 (storage migration) breaks existing OSS behavior | Medium | High | Migrate smallest store first; verify each sub-step with tests; keep old code until new is verified |
| R2 | Phase 1 (crate extraction) introduces circular dependencies | Low | High | Enterprise crate depends on api+core, not vice versa; trait abstractions prevent cycles |
| R3 | Phase 5 (PostgreSQL) performance is worse than SQLite for small deployments | Medium | Medium | Benchmark before switching; keep SQLite as default for small deployments |
| R4 | Phase 6 (multi-instance) introduces race conditions in config sync | Medium | High | Use Redis atomic operations; test with 3+ instances; chaos testing |
| R5 | Phase 4 (auth) has security vulnerabilities | Medium | Critical | External security audit before production; follow OWASP guidelines; use Argon2id not bcrypt |
| R6 | Phase 12 (licensing server) Stripe integration has edge cases | High | Medium | Use Stripe Checkout (not custom form); test webhooks with Stripe CLI; handle all event types |
| R7 | Phase 8 (AI agent) MCP protocol changes break compatibility | Low | Medium | Pin MCP protocol version; test with Claude Desktop, Cursor, and Windsurf |
| R8 | Phase 10 (DB optimization) partitioning complicates queries | Medium | Medium | Use pg_partman for management; test all query patterns with partitioned tables |
| R9 | Ed25519 key compromise | Low | Critical | Store private key in HSM or secrets manager; rotate keys; have revocation plan |
| R10 | OSS community rejects BSL license for enterprise crate | Medium | Medium | BSL only applies to enterprise crate; OSS crates remain MIT/Apache; communicate clearly |

---

## 20. Verification Checklist

### After every phase

- [ ] `cargo build --release` (default features) passes
- [ ] `cargo build --release --no-default-features` (OSS) passes
- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets --all-features` is clean
- [ ] OSS binary has no enterprise code (`strings` check)
- [ ] No new `#[cfg]` gates added to `madhyamas-core` or `madhyamas-api`

### After Phase 1 (Crate Extraction)

- [ ] `madhyamas-enterprise` crate compiles standalone
- [ ] `madhyamas-core` has zero `enterprise` references
- [ ] `madhyamas-api` has zero `enterprise` references (only trait defs)
- [ ] `#[cfg]` gate count: 17 → ≤5 (only in `main.rs`)
- [ ] `jsonwebtoken` not in `madhyamas-core/Cargo.toml`
- [ ] Enterprise binary starts and mounts `/api/auth/*`, `/api/users/*`, `/api/audit/*` routes

### After Phase 2 (Storage Migration)

- [ ] Zero `rusqlite` references in codebase
- [ ] All stores use `sqlx::SqlitePool`
- [ ] `AppState` holds `Arc<dyn StoreBackend>` trait objects
- [ ] OSS behavior identical (same SQLite file, same query results)

### After Phase 4 (Auth, RBAC, Audit)

- [ ] Login with username/password returns JWT
- [ ] API key authentication works (`X-API-Key` header)
- [ ] RBAC denies unauthorized actions (viewer cannot delete)
- [ ] Audit log records all authenticated and mutation actions
- [ ] No handler returns 501 Not Implemented

### After Phase 6 (Multi-Instance)

- [ ] Two instances share traffic data via PostgreSQL
- [ ] Config change on instance A propagates to instance B within 1 second
- [ ] WebSocket event from instance A reaches client connected to instance B
- [ ] License seat count is accurate across instances
- [ ] Graceful shutdown releases seats and closes connections

### After Phase 9 (Security Hardening)

- [ ] WebSocket requires auth in enterprise mode
- [ ] Redis connection uses TLS and auth
- [ ] CSP headers present on all responses
- [ ] Password complexity enforced
- [ ] Session idle timeout works (30min)
- [ ] Audit log hash chain verifies

### Final (before production)

- [ ] External security audit completed
- [ ] Load test: 1000+ req/sec with PostgreSQL
- [ ] Load test: 50+ concurrent instances with PgBouncer
- [ ] Failover test: primary DB goes down, reads continue
- [ ] Disaster recovery test: restore from backup
- [ ] Documentation complete and reviewed
- [ ] SBOM generated for both tiers
- [ ] License review: OSS build contains no BSL code

---

## See Also

- [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md) — Master document
- [Enterprise Crate Migration](ENTERPRISE_CRATE_MIGRATION.md) — Phase 1 details
- [Enterprise Storage Traits](ENTERPRISE_STORAGE_TRAITS.md) — Phase 2, 5 details
- [Enterprise Auth, RBAC, and IdP](ENTERPRISE_AUTH_RBAC.md) — Phase 4 details
- [Enterprise Multi-Instance](ENTERPRISE_MULTI_INSTANCE.md) — Phase 6 details
- [Enterprise Web UI](ENTERPRISE_WEB_UI.md) — Phase 7 details
- [Enterprise AI Agent Integration](ENTERPRISE_AI_AGENTS.md) — Phase 8 details
- [Enterprise Performance & Security](ENTERPRISE_PERF_SECURITY.md) — Phase 9, 10 details
- [Enterprise CI/CD](ENTERPRISE_CICD.md) — Phase 11 details
- [Enterprise Licensing Server](ENTERPRISE_LICENSING_SERVER.md) — Phase 12 details
- [Enterprise OSS Comparison](ENTERPRISE_OSS_COMPARISON.md) — Feature parity matrix
