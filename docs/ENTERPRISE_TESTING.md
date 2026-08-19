# Enterprise Testing Guide

> Part of: [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md)
>
> Last verified: 2025-01

This document describes the testing strategy for Madhyamas enterprise
features — from unit tests through integration tests against PostgreSQL
and Redis, multi-instance verification via Docker Compose, and end-to-end
web UI testing with Playwright.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Test Infrastructure](#2-test-infrastructure)
3. [Unit Tests](#3-unit-tests)
4. [Integration Tests](#4-integration-tests)
5. [Multi-Instance Tests](#5-multi-instance-tests)
6. [End-to-End Tests](#6-end-to-end-tests)
7. [Running Tests](#7-running-tests)
8. [Test Database Setup](#8-test-database-setup)
9. [CI/CD](#9-cicd)

---

## 1. Overview

The enterprise test suite follows a classic test pyramid: unit tests form
the broad base (fast, no external dependencies), integration tests verify
module interactions against real PostgreSQL and Redis, and end-to-end
tests exercise the full Docker Compose stack through a browser.

```mermaid
graph TD
    subgraph "Test Pyramid"
        E2E["E2E Tests<br/>(Playwright + Docker Compose)"]
        INT["Integration Tests<br/>(PostgreSQL + Redis, #[ignore])"]
        UNIT["Unit Tests<br/>(in-memory SQLite, pure logic)"]
    end
    UNIT --> INT --> E2E
    style UNIT fill:#4CAF50,color:#fff
    style INT fill:#FF9800,color:#fff
    style E2E fill:#f44336,color:#fff
```

| Layer | Scope | External deps | Run command |
|---|---|---|---|
| Unit | `madhyamas-enterprise` crate modules | None (in-memory SQLite) | `cargo test -p madhyamas-enterprise` |
| Integration | PostgreSQL + Redis backed tests | PostgreSQL, Redis | `cargo test -- --ignored` |
| Multi-instance | Docker Compose stack (2 instances) | Docker | `./startup-local.sh` + manual checks |
| E2E | Playwright browser automation | Running enterprise stack | `node scripts/capture-enterprise-screenshots.mjs` |

---

## 2. Test Infrastructure

### 2.1 PostgreSQL test container

PostgreSQL integration tests connect via `MADHYAMAS_PG_TEST_URL` (default:
`postgres://madhyamas:testpass@localhost:5432/madhyamas`). The test helper
in `crates/madhyamas-core/src/storage/postgres/traffic.rs` creates a fresh
`PostgresTrafficStore` per test, running DDL for tables and optimized
indexes (GIN, BRIN, trigram) inside an advisory-locked transaction.

### 2.2 Redis test container

Redis integration tests connect to `redis://localhost:6379` (hardcoded in
`crates/madhyamas-enterprise/src/redis_state.rs`). Each test generates a
unique instance ID (`test-<uuid>`) to avoid collisions. Tests cover
pub/sub, seat coordination, and cluster metrics aggregation.

### 2.3 Docker Compose multi-instance stack

The full multi-instance stack is defined in
`docker/docker-compose.multi.yml` and started by `./startup-local.sh`:

```mermaid
graph TD
    LB["nginx Load Balancer<br/>:14000 (round-robin)"]
    INST1["Madhyamas Instance 1<br/>:14001"]
    INST2["Madhyamas Instance 2<br/>:14002"]
    PG["PostgreSQL 16<br/>:15432 (shared storage)"]
    REDIS["Redis 7<br/>:16379 (pub/sub + seats)"]
    LB --> INST1
    LB --> INST2
    INST1 --> PG
    INST2 --> PG
    INST1 --> REDIS
    INST2 --> REDIS
    INST1 -->|"proxy :8888"| CLIENTS["Client apps"]
    INST2 -->|"proxy :8888"| CLIENTS
```

| Service | Container | Host port | Purpose |
|---|---|---|---|
| nginx | `nginx` | 14000 | Round-robin load balancer |
| Instance 1 | `madhyamas-1` | 14001 | API + Web UI + proxy |
| Instance 2 | `madhyamas-2` | 14002 | API + Web UI + proxy |
| PostgreSQL | `postgres` | 15432 | Shared traffic + enterprise storage |
| Redis | `redis` | 16379 | Pub/sub, seat coordination, metrics |

---

## 3. Unit Tests

Unit tests live in `#[cfg(test)] mod tests` blocks within each source
file. They use in-memory SQLite for store-backed tests and require no
external services.

### 3.1 Auth tests (`auth.rs`, lines 644–851)

| Test | What it verifies |
|---|---|
| `test_jwt_generate_and_validate` | JWT access token generation and validation round-trip |
| `test_jwt_expired_rejected` | Tokens expired beyond the 60s leeway are rejected |
| `test_jwt_wrong_secret_rejected` | Tokens signed with a different secret fail validation |
| `test_refresh_token_flow` | Access + refresh token pair shares session ID; typ enforcement |
| `test_api_key_create_and_validate` | API key creation, SHA-256 hashing, store-backed validation |
| `test_api_key_expired` | Expired API keys are rejected during validation |
| `test_api_key_revoked` | Revoked API keys are rejected during validation |
| `test_scope_matching` | Scope wildcard matching (`*:*`, `traffic:*`, `*:read`) |

Helpers: `test_manager()`, `test_store()` (in-memory SQLite), `seed_user()`.

### 3.2 RBAC tests (`rbac.rs`, lines 278–311)

| Test | What it verifies |
|---|---|
| `test_admin_can_delete` | Admin role has Delete on Traffic, Mock; Write on Config |
| `test_viewer_cannot_delete` | Viewer role cannot Delete but can Read |
| `test_readonly_cannot_write` | ReadOnly role cannot Write but can Read |

Exercises the static permission matrix in `RbacManager::new()` defining
role → `(ResourceType, Permission)` mappings for Admin, User, Viewer, ReadOnly.

### 3.3 Audit tests (`audit.rs`, lines 451–547)

| Test | What it verifies |
|---|---|
| `test_log_and_query` | Events are logged and queryable via the persistent store |
| `test_filter_by_user` | Audit filter by `user_id` returns only matching events |
| `test_hash_chain` | SHA-256 hash chain is intact across multiple events |
| `test_hash_chain_tamper_detection` | Direct DB modification breaks the chain and is detected |

Verifies the tamper-evident audit log: each event's `prev_hash` links to
the previous event's `hash`; `verify_hash_chain()` detects modifications.

### 3.4 License tests (`license.rs`, lines 310–497)

| Test | What it verifies |
|---|---|
| `test_valid_license` | Valid Ed25519-signed license verifies successfully |
| `test_expired_license` | Licenses past `expires_at` return `LicenseError::Expired` |
| `test_tampered_license` | Flipped signature byte returns `InvalidSignature` |
| `test_wrong_key` | Signature verified with a different public key fails |
| `test_canonical_json_stable` | Canonical JSON is deterministic regardless of key order |
| `test_verify_from_file` | License file read from disk verifies correctly |
| `test_verify_missing_file` | Missing file returns `LicenseError::NotFound` |
| `test_instance_id_match` | Matching `instance_id` passes replay prevention check |
| `test_instance_id_mismatch` | Mismatched `instance_id` returns `InstanceMismatch` |
| `test_instance_id_not_set_accepts_any` | Without expected instance ID, any is accepted |

Helpers: `make_signed_license()` (fresh Ed25519 keypair + signed claims),
`sample_claims()` (standard test claims).

### 3.5 Redis state tests (`redis_state.rs`, lines 417–669)

All `#[ignore]` — require Redis at `redis://localhost:6379`.

| Test | What it verifies |
|---|---|
| `test_redis_connect_ping` | Connection and PING round-trip |
| `test_redis_publish_subscribe` | Pub/sub message delivery across channels |
| `test_redis_config_propagation` | Config change notification propagation |
| `test_seat_registration` | Instance registration and active count |
| `test_seat_limit_enforcement` | Multiple instances registered and counted |
| `test_seat_release` | Deregistration removes instance from active set |
| `test_instance_registration_with_metrics` | Metrics snapshot stored and updated |
| `test_cluster_metrics_aggregation` | Cluster-wide metrics aggregation across instances |

### 3.6 Additional unit tests

- **Credentials** (`credentials.rs`): Argon2id hashing/verification, malformed
  hash handling, password complexity (length, uppercase, lowercase, digit,
  special character).
- **Security** (`security.rs`): OIDC callback URL validation (HTTPS-only,
  rejects private/loopback IPs), private IP detection for IPv4/IPv6.
- **Handlers** (`handlers.rs`): Health check with and without Redis.

### 3.7 Running unit tests

```bash
cargo test -p madhyamas-enterprise              # all enterprise unit tests
cargo test -p madhyamas-enterprise auth::tests  # specific module
cargo test -p madhyamas-enterprise license::tests
```

---

## 4. Integration Tests

Integration tests require external services (PostgreSQL and/or Redis),
marked `#[ignore]` so they skip during normal `cargo test`.

### 4.1 PostgreSQL traffic store tests

**File:** `crates/madhyamas-core/src/storage/postgres/traffic.rs` (lines 1874–2165)

| Test | What it verifies |
|---|---|
| `test_pg_traffic_store_request_response` | Request/response storage and retrieval |
| `test_pg_traffic_store_sessions` | Session creation and listing |
| `test_pg_traffic_store_focus_hosts` | Focus host CRUD with unique constraint |
| `test_pg_traffic_store_har_import` | HAR file import into PostgreSQL |
| `test_pg_tiered_body_storage` | Bodies ≥4KB stored in `traffic_bodies` with zstd |
| `test_pg_session_counter` | O(1) session counter table vs COUNT(*) fallback |
| `test_pg_cursor_pagination` | Keyset pagination via `(timestamp, id)` cursor |
| `test_pg_lazy_body_loading` | List view omits body columns (lazy loading) |
| `test_pg_flush` | Write batcher flush with no pending writes |

### 4.2 PostgreSQL enterprise store tests

**File:** `crates/madhyamas-enterprise/src/store/postgres.rs` (lines 534+)

| Test | What it verifies |
|---|---|
| `test_pg_enterprise_user_crud` | User create/read/update/delete lifecycle |
| `test_pg_enterprise_audit_log` | Audit event persistence and hash chain queries |
| `test_pg_enterprise_api_key` | API key storage, lookup by hash, revocation |

### 4.3 Test database setup

```mermaid
graph LR
    ENV["MADHYAMAS_PG_TEST_URL"] --> POOL["PgPool (max 5 conns)"]
    POOL --> STORE["PostgresTrafficStore::new()"]
    STORE -->|"advisory lock"| DDL["Tables + GIN/BRIN/trigram indexes"]
    DDL --> TESTS["#[ignore] tests"]
```

### 4.4 Running integration tests

```bash
# Start test containers (see §8)
docker run -d --name madhyamas-pg-test \
  -e POSTGRES_DB=madhyamas -e POSTGRES_USER=madhyamas -e POSTGRES_PASSWORD=testpass \
  -p 5432:5432 postgres:16-alpine
docker run -d --name madhyamas-redis-test -p 6379:6379 redis:7-alpine

# Run all ignored tests (PostgreSQL + Redis)
cargo test --all-features -- --ignored

# Run only PostgreSQL or Redis tests
cargo test -p madhyamas-core -- --ignored test_pg_
cargo test -p madhyamas-enterprise -- --ignored test_redis_
```

---

## 5. Multi-Instance Tests

Verifies that two Madhyamas enterprise instances sharing the same
PostgreSQL and Redis correctly synchronize state.

### 5.1 Starting the Docker Compose stack

```bash
./startup-local.sh                    # start full multi-instance stack
./startup-local.sh --clean            # clean rebuild
LB_PORT=80 INSTANCE1_API_PORT=8081 INSTANCE2_API_PORT=8082 ./startup-local.sh
```

The script builds the Docker image, starts all 5 services, and polls
health endpoints until all instances return HTTP 200 from `/health`.

### 5.2 Verifying event propagation

Traffic events captured by one instance should be visible to WebSocket
clients on the other instance via Redis pub/sub (`madhyamas:events`).

```bash
wscat -c ws://localhost:14001/ws              # connect to instance 1
curl -x http://localhost:14002 http://example.com  # proxy through instance 2
# Verify the traffic event appears in instance 1's WebSocket stream
```

### 5.3 Verifying session sync

Both instances share PostgreSQL and use a deterministic default session
ID (`default-session`), so traffic captured by either appears in the
same session.

```bash
curl http://localhost:14001/api/sessions  # should match instance 2
curl http://localhost:14002/api/sessions
```

### 5.4 Verifying seat coordination

Seat counts are tracked via a Redis sorted set with 120s heartbeat TTL.

```bash
curl http://localhost:14001/api/metrics | jq .instances  # expect 2
docker compose -f docker/docker-compose.multi.yml stop madhyamas-2
sleep 130  # wait for heartbeat TTL
curl http://localhost:14001/api/metrics | jq .instances  # expect 1
```

### 5.5 Verifying shared CA

Both instances share the TLS CA via the PostgreSQL `instance_state` table.

```bash
curl http://localhost:14001/api/cert/ca -o ca1.pem
curl http://localhost:14002/api/cert/ca -o ca2.pem
diff ca1.pem ca2.pem  # should be identical
```

---

## 6. End-to-End Tests

### 6.1 Playwright screenshot capture

**File:** `scripts/capture-enterprise-screenshots.mjs`

Uses Playwright (headless Chromium) to capture documentation screenshots of
the enterprise web UI. Requires the enterprise stack running via
`./startup-local.sh`.

```bash
./startup-local.sh                    # start enterprise stack on :14000
cd web && npm install && npx playwright install chromium && cd ..
node scripts/capture-enterprise-screenshots.mjs
```

Screenshots saved to `docs-site/public/screenshots/`:

| Screenshot | What it shows |
|---|---|
| `enterprise-login.png` | Login page before authentication |
| `enterprise-user-menu.png` | User dropdown menu after login |
| `enterprise-users-panel.png` | User management admin panel |
| `enterprise-audit-panel.png` | Audit log viewer |
| `enterprise-metrics-panel.png` | System metrics dashboard |
| `enterprise-license-panel.png` | License details panel |
| `enterprise-apikeys-panel.png` | API key management panel |
| `enterprise-instances-panel.png` | Multi-instance overview panel |

Login credentials: `admin` / `testpass123` (defaults from `startup-local.sh`).

### 6.2 Web UI auth flow testing

The script exercises the full auth flow: navigate to
`http://localhost:14000`, verify the login page renders with `#username`
and `#password` inputs, fill credentials, submit, and verify redirect to
the dashboard with the username shown in the user menu.

### 6.3 Admin panel interaction testing

After login, the script clicks NavRail buttons by `aria-label` to
navigate to each admin panel and captures a screenshot. This verifies
enterprise-only buttons (Users, Audit Log, Metrics, License, API Keys,
Instances) render and each panel loads without errors.

---

## 7. Running Tests

### 7.1 Quick reference

```bash
# OSS tests (no enterprise features)
cargo test --no-default-features

# Enterprise unit tests (no external deps)
cargo test -p madhyamas-enterprise

# All unit tests with enterprise features
cargo test --all-features

# Enterprise integration tests (requires PostgreSQL + Redis, see §8)
cargo test --all-features -- --ignored

# Multi-instance verification (requires Docker)
./startup-local.sh  # then run manual checks (see §5)

# Web UI tests
# (none yet — web/package.json defines no `test` script; CI runs
#  typecheck + lint + build only)

# Screenshot capture (requires running enterprise stack)
node scripts/capture-enterprise-screenshots.mjs
```

### 7.2 Using cargo-nextest

CI uses `cargo-nextest` for faster, parallel test execution:

```bash
cargo install cargo-nextest
cargo nextest run -p madhyamas-enterprise
cargo nextest run --all-features --run-ignored all  # includes #[ignore]
```

### 7.3 Clippy and formatting

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
```

---

## 8. Test Database Setup

### 8.1 Docker commands for test containers

```bash
# PostgreSQL test container (port 5432)
docker run -d --name madhyamas-pg-test \
  -e POSTGRES_DB=madhyamas -e POSTGRES_USER=madhyamas -e POSTGRES_PASSWORD=testpass \
  -p 5432:5432 postgres:16-alpine

# Redis test container (port 6379)
docker run -d --name madhyamas-redis-test -p 6379:6379 redis:7-alpine
```

### 8.2 Environment variables

| Variable | Default | Used by |
|---|---|---|
| `MADHYAMAS_PG_TEST_URL` | `postgres://madhyamas:testpass@localhost:5432/madhyamas` | PostgreSQL traffic + enterprise store tests |
| Redis URL (hardcoded) | `redis://localhost:6379` | Redis state + handler tests |
| `MADHYAMAS_LICENSE_PUBLIC_KEY` | Compiled-in dev key | License verification (tests generate own keypairs) |

### 8.3 Cleanup procedures

```bash
docker stop madhyamas-pg-test madhyamas-redis-test
docker rm madhyamas-pg-test madhyamas-redis-test
./stop-local.sh --tier enterprise
# Or: docker compose -f docker/docker-compose.multi.yml down -v --remove-orphans
```

---

## 9. CI/CD

### 9.1 How tests are run in CI

Tests run in the `rust-checks` job in `.github/workflows/ci.yml` with a two-tier matrix:

```mermaid
graph TD
    FE["Build Frontend"] -->|artifact| RC["Rust Checks"]
    RC --> M1["enterprise<br/>cargo nextest run --all-features"]
    RC --> M2["oss<br/>cargo nextest run --no-default-features"]
    M1 --> SA["Security Audit + OSS BSL check"]
    M2 --> SA
    FE --> DB["Docker Build Test (oss + enterprise)"]
    FE --> CB["Cross-compile 8 targets × 2 tiers"]
```

### 9.2 Test matrix

| Dimension | Values |
|---|---|
| OS | `ubuntu-latest`, `macos-latest`, `windows-latest` |
| Rust | `stable` (all OSes), `beta` (ubuntu only) |
| Tier | `enterprise` (all features), `oss` (no default features) |

Each cell runs: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, `cargo nextest run`.

### 9.3 PostgreSQL service container

> CI currently runs only non-ignored tests. PostgreSQL and Redis
> integration tests (`#[ignore]`) are not yet wired into CI. To add them,
> a service container block would be added to `rust-checks`:

```yaml
services:
  postgres:
    image: postgres:16-alpine
    env: {POSTGRES_DB: madhyamas, POSTGRES_USER: madhyamas, POSTGRES_PASSWORD: testpass}
    ports: ["5432:5432"]
    options: --health-cmd "pg_isready -U madhyamas" --health-interval 5s --health-retries 5
  redis:
    image: redis:7-alpine
    ports: ["6379:6379"]
    options: --health-cmd "redis-cli ping" --health-interval 5s --health-retries 5
```

Then: `cargo nextest run --all-features --run-ignored all`.

### 9.4 OSS/enterprise separation

CI enforces that the OSS build does not pull in the BSL-licensed
`madhyamas-enterprise` crate via `cargo tree --no-default-features | grep`.