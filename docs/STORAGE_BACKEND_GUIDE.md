# Storage Backend Implementation Guide

> **Last verified:** 2025-01 against Madhyamas `0.1.6`.

A practical guide for implementing a new storage backend. Covers the six async
storage traits, schema design, multi-instance coordination via advisory locks
and shared state, an implementation checklist, testing strategy, and the
SQLite → PostgreSQL migration path.

Part of: [PERSISTENCE.md](PERSISTENCE.md), [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md),
[ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md).

---

## Overview

Madhyamas abstracts all persistence behind six async `#[async_trait]` traits,
each `Send + Sync` (held as `Arc<dyn Trait + Send + Sync>` on `AppState`).
The traits are backend-agnostic: the same trait is implemented by SQLite (OSS,
single-instance) and PostgreSQL (enterprise, multi-instance); a new backend
(e.g. MySQL) only needs to implement the same trait. The design follows
**Approach C** from [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md)
§1.2: a single async trait per store, `sqlx` as the only database library
(`SqlitePool`, `PgPool`, or `MySqlPool`).

### Trait hierarchy and existing implementations

```mermaid
graph TD
    subgraph "Core traits (madhyamas-core)"
        TSB[TrafficStoreBackend<br/>21 async + 16 sync]
        CSB[ConfigStoreBackend<br/>9 methods]
        ISB[InterceptStoreBackend<br/>16 methods]
        PSB[PluginStoreBackend<br/>8 — feature: plugins]
        SSB[ScriptStoreBackend<br/>8 — feature: scripting]
    end
    subgraph "Enterprise trait"
        ES[EnterpriseStore<br/>18 methods]
    end
    subgraph "SQLite (OSS)"
        ST[SqliteTrafficStore] & SC[SqliteConfigStore] & SI[SqliteInterceptStore]
        SP[SqlitePluginStore] & SS[SqliteScriptStore] & SE[SqliteEnterpriseStore]
    end
    subgraph "PostgreSQL (Enterprise)"
        PT[PostgresTrafficStore] & PC[PostgresConfigStore] & PI[PostgresInterceptStore]
        PP[PostgresPluginStore] & PS[PostgresScriptStore] & PE[PostgresEnterpriseStore]
    end
    TSB --> ST & PT
    CSB --> SC & PC
    ISB --> SI & PI
    PSB --> SP & PP
    SSB --> SS & PS
    ES --> SE & PE
```

PostgreSQL implementations live under `crates/madhyamas-core/src/storage/postgres/`
and `crates/madhyamas-enterprise/src/store/postgres.rs`; SQLite under
`crates/madhyamas-core/src/storage/sqlite/`, `traffic/store.rs`, and
`crates/madhyamas-enterprise/src/store/sqlite.rs`.

---

## Storage Traits

All traits are defined in `crates/madhyamas-core/src/storage/mod.rs` (core)
and `crates/madhyamas-enterprise/src/store/mod.rs` (enterprise). DB-backed
methods are `async fn`; in-memory config/broadcast methods are sync `fn`
(`RwLock`/`AtomicXxx`/`broadcast` only).

### TrafficStoreBackend

The largest trait: storage, retrieval, mutation, sessions, export/import,
focus hosts, real-time events, and in-memory capture config. Defined at
lines 57–132.

```rust
#[async_trait]
pub trait TrafficStoreBackend: Send + Sync {
    // Storage
    async fn store_request(&self, entry: &TrafficEntry) -> Result<()>;
    async fn store_response(&self, request_id: &str, response: &ResponseData) -> Result<()>;
    // Query
    async fn get_traffic(&self, filter: &TrafficFilter) -> Result<Vec<TrafficEntry>>;
    async fn get_by_id(&self, id: &str) -> Result<Option<TrafficEntry>>;
    async fn get_entry_count(&self) -> Result<usize>;
    async fn get_capture_stats(&self) -> Result<CaptureStats>;
    async fn clear_traffic(&self) -> Result<()>;
    async fn delete_traffic(&self, ids: &[String]) -> Result<()>;
    async fn count(&self) -> Result<usize>;
    // Export / Import
    async fn export_har(&self, session_id: &str) -> Result<serde_json::Value>;
    async fn import_har(&self, har: &serde_json::Value, session_name: Option<&str>) -> Result<ImportResult>;
    // Sessions
    async fn list_sessions(&self) -> Result<Vec<Session>>;
    async fn create_session(&self, name: Option<&str>) -> Result<Session>;
    async fn switch_session(&self, session_id: &str) -> Result<()>;
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    async fn get_traffic_by_session(&self, session_id: &str) -> Result<Vec<TrafficEntry>>;
    // Focus hosts
    async fn add_focus_host(&self, pattern: &str) -> Result<FocusHost>;
    async fn remove_focus_host(&self, id: &str) -> Result<bool>;
    async fn list_focus_hosts(&self) -> Result<Vec<FocusHost>>;
    async fn clear_focus_hosts(&self) -> Result<()>;
    // Shared state (multi-instance coordination)
    async fn get_shared_state(&self, key: &str) -> Result<Option<String>>;
    async fn set_shared_state(&self, key: &str, value: &str) -> Result<()>;
    async fn sync_current_session(&self) -> Result<()>;
    // Health / flush
    async fn flush(&self) -> Result<()>;
    async fn ping(&self) -> Result<()>;
    // In-memory config / broadcast (sync)
    fn subscribe(&self) -> broadcast::Receiver<TrafficEvent>;
    fn event_sender(&self) -> broadcast::Sender<TrafficEvent>;
    fn current_session_id(&self) -> String;
    fn is_capture_enabled(&self) -> bool;
    fn set_capture_enabled(&self, enabled: bool);
    fn set_max_body_size(&self, max: usize);
    fn max_body_size(&self) -> usize;
    fn set_max_entries(&self, max: usize);
    fn max_entries(&self) -> usize;
    fn set_max_total_size_bytes(&self, max: usize);
    fn max_total_size_bytes(&self) -> usize;
    fn set_capture_request_bodies(&self, enabled: bool);
    fn capture_request_bodies(&self) -> bool;
    fn set_capture_response_bodies(&self, enabled: bool);
    fn capture_response_bodies(&self) -> bool;
    fn set_ignored_domains(&self, domains: Vec<String>);
    fn ignored_domains(&self) -> Vec<String>;
    fn set_mirror_writer(&self, writer: Arc<MirrorWriter>);
    fn mirror_writer(&self) -> Option<Arc<MirrorWriter>>;
}
```

The 21 async methods group into: **storage** (2), **query** (7), **export/import**
(2), **sessions** (5), **focus hosts** (4), **shared state** (3), and
**health/flush** (2). The 16 sync `fn` methods cover in-memory config
(`RwLock`/`AtomicXxx`/`broadcast` only — no pool access). Key notes:
`get_entry_count` reads `session_counters` (O(1)) with `COUNT(*)` fallback;
`switch_session` persists to shared state; `flush` drains the write batcher;
`ping` is used by `/health`.

### ConfigStoreBackend

Generic typed get/set over a `serde_json::Value` core, plus delete, load/save
of `PersistedConfig`, and export/import. Non-generic `get_value`/`set_value`
keep the trait object-safe; typed `get`/`set` defaults are bounded by
`Self: Sized`. Defined at lines 141–171.

```rust
#[async_trait]
pub trait ConfigStoreBackend: Send + Sync {
    async fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>>;
    async fn set_value(&self, key: &str, value: &serde_json::Value) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<bool>;
    async fn load_config(&self) -> Result<PersistedConfig>;
    async fn save_config(&self, config: &PersistedConfig) -> Result<()>;
    async fn export(&self) -> Result<String>;
    async fn import(&self, json: &str) -> Result<()>;
    // Typed defaults (bounded by Self: Sized, not object-safe):
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> Result<Option<T>> where Self: Sized { /* default */ }
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()> where Self: Sized { /* default */ }
}
```

### InterceptStoreBackend

Mocks, rewrites, breakpoints, throttle, blocklist persistence, bulk clear, and
export/import. Defined at lines 175–199.

```rust
#[async_trait]
pub trait InterceptStoreBackend: Send + Sync {
    async fn save_mock_rule(&self, rule: &MockRule) -> Result<()>;
    async fn load_mock_rules(&self) -> Result<Vec<MockRule>>;
    async fn delete_mock_rule(&self, id: &str) -> Result<bool>;
    async fn increment_mock_hit_count(&self, id: &str) -> Result<()>;
    async fn save_rewrite_rule(&self, rule: &RewriteRule) -> Result<()>;
    async fn load_rewrite_rules(&self) -> Result<Vec<RewriteRule>>;
    async fn delete_rewrite_rule(&self, id: &str) -> Result<bool>;
    async fn save_breakpoint_rule(&self, rule: &BreakpointRule) -> Result<()>;
    async fn load_breakpoint_rules(&self) -> Result<Vec<BreakpointRule>>;
    async fn delete_breakpoint_rule(&self, id: &str) -> Result<bool>;
    async fn save_throttle_profile(&self, profile: &ThrottleProfile, enabled: bool) -> Result<()>;
    async fn load_throttle_profile(&self) -> Result<Option<(ThrottleProfile, bool)>>;
    async fn save_block_list_entry(&self, entry: &BlockListEntry) -> Result<()>;
    async fn load_block_list_entries(&self) -> Result<Vec<BlockListEntry>>;
    async fn delete_block_list_entry(&self, id: &str) -> Result<bool>;
    async fn increment_block_list_hit_count(&self, id: &str) -> Result<()>;
    async fn clear_block_list_entries(&self) -> Result<()>;
    async fn clear_mock_rules(&self) -> Result<()>;
    async fn clear_rewrite_rules(&self) -> Result<()>;
    async fn clear_breakpoint_rules(&self) -> Result<()>;
    async fn export_all(&self) -> Result<String>;
    async fn import_all(&self, json: &str) -> Result<()>;
}
```

### PluginStoreBackend (feature-gated: `plugins`)

Plugin registry state and invocation audit log. Available only when the
`plugins` feature is enabled. Defined at lines 203–223.

```rust
#[cfg(feature = "plugins")]
#[async_trait]
pub trait PluginStoreBackend: Send + Sync {
    async fn save_state(&self, plugin_id: &str, enabled: bool, settings: &HashMap<String, serde_json::Value>) -> Result<()>;
    async fn mark_installed(&self, plugin_id: &str) -> Result<()>;
    async fn remove_state(&self, plugin_id: &str) -> Result<()>;
    async fn load_state(&self, plugin_id: &str) -> Result<Option<PluginStateRow>>;
    async fn load_all_states(&self) -> Result<Vec<PluginStateRow>>;
    async fn record_invocation(&self, row: &PluginInvocationRow) -> Result<()>;
    async fn list_invocations(&self, plugin_id: &str, limit: u32) -> Result<Vec<PluginInvocationRow>>;
    async fn prune_invocations(&self, keep: u32) -> Result<()>;
}
```

### ScriptStoreBackend (feature-gated: `scripting`)

Script definitions and execution history. Available only when the `scripting`
feature is enabled. Defined at lines 227–242.

```rust
#[cfg(feature = "scripting")]
#[async_trait]
pub trait ScriptStoreBackend: Send + Sync {
    async fn save_script(&self, script: &Script) -> Result<()>;
    async fn load_scripts(&self) -> Result<Vec<Script>>;
    async fn delete_script(&self, id: &str) -> Result<bool>;
    async fn save_execution(&self, exec: &ScriptExecution) -> Result<()>;
    async fn load_all_executions(&self, limit: usize) -> Result<Vec<ScriptExecution>>;
    async fn load_executions(&self, script_id: &str, limit: usize) -> Result<Vec<ScriptExecution>>;
    async fn load_executions_by_traffic(&self, traffic_entry_id: &str, limit: usize) -> Result<Vec<ScriptExecution>>;
    async fn clear_executions(&self, script_id: Option<&str>) -> Result<()>;
}
```

### EnterpriseStore

Async storage for users, API keys, auth sessions, and audit events. Defined
in `crates/madhyamas-enterprise/src/store/mod.rs` (lines 50–84). The 18
methods: users (7), API keys (4), sessions (4), audit (4) + `get_latest_audit_hash`.

```rust
#[async_trait]
pub trait EnterpriseStore: Send + Sync {
    // Users
    async fn create_user(&self, user: &User, password_hash: &str) -> Result<()>;
    async fn get_user(&self, id: &str) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>>;
    async fn get_user_credentials(&self, username: &str) -> Result<Option<(User, String)>>;
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn update_user(&self, id: &str, updates: &UserUpdate) -> Result<()>;
    async fn delete_user(&self, id: &str) -> Result<()>;
    // API keys
    async fn create_api_key(&self, key: &ApiKeyRecord) -> Result<()>;
    async fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>>;
    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>>;
    async fn revoke_api_key(&self, id: &str) -> Result<()>;
    async fn update_api_key_last_used(&self, id: &str) -> Result<()>;
    // Auth sessions
    async fn create_session(&self, session: &AuthSession) -> Result<()>;
    async fn get_session(&self, id: &str) -> Result<Option<AuthSession>>;
    async fn revoke_session(&self, id: &str) -> Result<()>;
    async fn cleanup_expired_sessions(&self) -> Result<()>;
    async fn update_session_activity(&self, session_id: &str) -> Result<()>;
    // Audit
    async fn log_audit_event(&self, event: &AuditEvent) -> Result<()>;
    async fn query_audit_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>>;
    async fn get_audit_stats(&self) -> Result<AuditStats>;
    async fn clear_audit_events(&self) -> Result<()>;
    async fn get_latest_audit_hash(&self) -> Result<Option<String>>;
}
```

`EnterpriseStore` returns `store::Result<T>` (alias for
`Result<T, StoreError>`), distinct from core `crate::Result`. Map driver
errors into `StoreError::Database` / `NotFound` / `Serialization`.

---

## Schema Design

The schema is identical between SQLite and PostgreSQL (modulo placeholder
syntax and type names). PostgreSQL adds optimized indexes
(GIN/BRIN/trigram) and a tiered body storage table that SQLite does not use.

### Core traffic tables

```mermaid
erDiagram
    sessions ||--o{ requests : has
    sessions ||--|| session_counters : tracks
    requests ||--|| responses : has
    requests ||--o{ traffic_bodies : stores
    responses ||--o{ traffic_bodies : stores

    sessions {
        TEXT id PK
        TEXT name
        BIGINT created_at
        BIGINT updated_at
    }
    requests {
        TEXT id PK
        TEXT session_id FK
        TEXT method
        TEXT url
        TEXT host
        TEXT headers
        BYTEA body
        BIGINT timestamp
        BOOLEAN is_passthrough
    }
    responses {
        TEXT request_id PK
        INTEGER status_code
        TEXT headers
        BYTEA body
        BIGINT duration_ms
    }
    session_counters {
        TEXT session_id PK
        INTEGER entry_count
    }
    instance_state {
        TEXT key PK
        TEXT value
        BIGINT updated_at
    }
    focus_hosts {
        TEXT id PK
        TEXT pattern UNIQUE
        BIGINT created_at
    }
    traffic_bodies {
        TEXT id PK
        TEXT entry_id FK
        BYTEA body
        BIGINT size
        BOOLEAN compressed
        TEXT storage_type
    }
```

The schema also includes `ws_connections` and `ws_messages` tables for
WebSocket traffic inspection (same columns in both backends).

### Enterprise tables

```mermaid
erDiagram
    users ||--o{ api_keys : owns
    users ||--o{ auth_sessions : has
    users ||--o{ audit_events : performs

    users {
        TEXT id PK
        TEXT username UNIQUE
        TEXT password_hash
        TEXT role
        TEXT status
        BIGINT created_at
    }
    api_keys {
        TEXT id PK
        TEXT user_id FK
        TEXT key_hash UNIQUE
        TEXT scopes
        TEXT expires_at
    }
    auth_sessions {
        TEXT id PK
        TEXT user_id FK
        TEXT jwt_jti
        TEXT expires_at
        BOOLEAN revoked
    }
    audit_events {
        TEXT id PK
        TEXT event_type
        TEXT timestamp
        TEXT user_id
        TEXT prev_hash
        TEXT hash
    }
```

`audit_events` forms a hash chain: each row's `hash` is computed over the
row's contents plus the preceding row's `prev_hash`, maintained under an
advisory lock (see below).

### Index recommendations

The PostgreSQL backend creates B-tree indexes on `session_id`, `url`, `method`,
`timestamp` (core), plus GIN (`gin_trgm_ops`) on `headers`/`url`/`path` for
fuzzy search, BRIN on `timestamp` for range scans, a UNIQUE index on
`focus_hosts.pattern` (race guard), and B-tree on `traffic_bodies.entry_id`,
`ws_connections(session_id, state)`, `ws_messages(connection_id, timestamp)`.
A new backend should replicate the B-tree indexes at minimum. For MySQL,
consider fulltext indexes on `url`/`headers` and a regular index on `timestamp`.

### Body storage strategy

Bodies use a tiered strategy (Phase 10a.1/10a.2): **inline** (`'inline'`) for
bodies < `INLINE_THRESHOLD` (4 KB) in `requests.body`/`responses.body`;
**toast** (`'toast'`) for bodies ≥ 4 KB in `traffic_bodies` (optionally
zstd-compressed, inline `body` = `NULL`); **S3** (`'s3'`, documented only). SQLite
stores all bodies inline. A new backend should at minimum support inline
storage; tiered is an optimization for high-volume deployments.

---

## Multi-Instance Support

When multiple proxy instances share a single database (enterprise PostgreSQL),
three mechanisms prevent races: **advisory locks** serialize DDL and pruning;
the **shared state table** (`instance_state`) propagates the current session
ID; and **session sync** lets each instance pull the latest session.

### Advisory locks

PostgreSQL transaction-scoped advisory locks (`pg_advisory_xact_lock`) use
three distinct keys so different operations don't block each other:

| Lock key | Mnemonic | Purpose |
|---|---|---|
| `0x4D414448` | "MADH" | DDL / schema initialization (all stores) |
| `0x4D414449` | "MADI" | Entry-count and total-size pruning |
| `0x4D414450` | "MADP" | Audit event hash-chain insertion |

All three are transaction-scoped, released on commit/rollback, and never block
normal read/write operations.

```mermaid
flowchart TD
    P1["Instance A: enforce_entry_limit"] --> P2["BEGIN + pg_advisory_xact_lock 0x4D414449"]
    P2 --> P3["DELETE ... RETURNING oldest N"]
    P3 --> P4["UPDATE session_counters — COMMIT"]
    Q1["Instance B: enforce_entry_limit"] --> Q2["BEGIN + lock — Blocked"]
    P4 --> Q2
    Q2 --> Q3["Sees updated counter — prunes 0"]
```

The same pattern applies to the DDL and audit locks. Without the prune lock,
two instances could both observe the counter at `N+1` over the limit, both
call `prune_oldest(N)`, and delete up to `2N` entries (race condition #4). The
lock makes read-then-delete atomic across instances.

### Shared state and session sync

The `instance_state` table (`key TEXT PK, value TEXT, updated_at BIGINT`) is a
cross-instance key/value store. `set_shared_state` upserts; `get_shared_state`
reads by key. The primary use is the `current_session_id` key: when one
instance switches sessions, it writes the new ID here. Other instances call
`sync_current_session` periodically to pull the latest value and update their
in-memory `current_session_id` (`Mutex<String>`). This is the only
cross-instance state actively synced; all other coordination uses locking at
the point of mutation.

---

## Implementation Checklist

Step-by-step for adding a new backend (e.g. MySQL) for all six traits:

1. **Add the driver.** Add `sqlx` with the `mysql` feature to `madhyamas-core`
   and `madhyamas-enterprise` `Cargo.toml`. Gate behind a `mysql` Cargo feature.

2. **Create the module skeleton.** Add a `mysql/` directory under the
   storage module with the following files:

   ```text
   storage/mysql/
   ├── mod.rs
   ├── traffic.rs
   ├── config.rs
   ├── intercept.rs
   ├── plugin.rs
   └── script.rs
   ```

   Also add an enterprise MySQL store. Re-export from `storage/mod.rs`
   behind the feature flag.

3. **Define the schema DDL.** Translate `SCHEMA_CORE_STMTS` to MySQL syntax
   (`BYTEA` → `LONGBLOB`; `BOOLEAN` → `TINYINT(1)`; IDs are app-generated
   UUIDs, no `AUTO_INCREMENT`). Keep `CREATE TABLE IF NOT EXISTS`.

4. **Implement `MysqlTrafficStore::new`.** Build a `sqlx::MySqlPool`, run DDL,
   call `ensure_session` with `"default-session"`. Use `INSERT IGNORE` /
   `INSERT ... ON DUPLICATE KEY UPDATE` instead of `ON CONFLICT DO NOTHING`.

5. **Implement the 21 async methods.** Translate from `storage/postgres/traffic.rs`,
   replacing `$N` with `?`. Key cases: `get_entry_count` reads `session_counters`
   first (fallback `COUNT(*)`); `enforce_entry_limit` uses `SELECT ... FOR UPDATE`
   or `GET_LOCK()` (no advisory locks); `set_shared_state` uses `ON DUPLICATE KEY
   UPDATE`; `add_focus_host` relies on `UNIQUE(pattern)` + `INSERT IGNORE`.

6. **Implement the 16 sync in-memory methods.** Identical across backends —
   copy from `traffic/store.rs` (`AtomicBool`/`AtomicUsize`/`RwLock`/`broadcast`).

7. **Implement `ConfigStoreBackend`, `InterceptStoreBackend`,
   `PluginStoreBackend`, `ScriptStoreBackend`.** Translate from PostgreSQL —
   straightforward CRUD behind the appropriate feature flags.

8. **Implement `EnterpriseStore`.** Translate `store/postgres.rs`. The critical
   method is `log_audit_event`: use `GET_LOCK('audit_chain', ...)` or
   `SELECT ... FOR UPDATE` to serialize read-last-hash + insert and keep the
   hash chain consistent.

9. **Wire up in the main binary.** Construct the MySQL stores when the DB URL
   starts with `mysql://`, held as `Arc<dyn Trait + Send + Sync>` on `AppState`.
   Write tests (see [Testing Guide](#testing-guide)) and update docs
   ([ENTERPRISE_OSS_COMPARISON.md](ENTERPRISE_OSS_COMPARISON.md),
   [PERSISTENCE.md](PERSISTENCE.md)).

### Advisory-lock equivalents

PostgreSQL advisory locks are the cleanest serialization primitive. Backends
without them must emulate: MySQL uses `GET_LOCK()`/`RELEASE_LOCK()` or
`SELECT ... FOR UPDATE`; SQLite uses `BEGIN IMMEDIATE`; CockroachDB supports
`pg_advisory_xact_lock()` natively.

---

## Testing Guide

### Running tests against a specific backend

Tests are gated by env vars: SQLite in-memory always runs; PostgreSQL tests
are `#[ignore]` and run only when `MADHYAMAS_PG_TEST_URL` is set.

```bash
docker run -d --name pg-test -e POSTGRES_USER=madhyamas \
  -e POSTGRES_PASSWORD=testpass -e POSTGRES_DB=madhyamas -p 5432:5432 postgres:16
MADHYAMAS_PG_TEST_URL=postgres://madhyamas:testpass@localhost:5432/madhyamas \
  cargo test -p madhyamas-enterprise -- --ignored
```

For a new backend, follow the same pattern (e.g. `MADHYAMAS_MYSQL_TEST_URL`).
Each test starts from a clean schema: connect to a dedicated test database,
construct via `::new(pool)` (idempotent DDL), clean up rows at the end. For
SQLite, `TrafficStore::in_memory()` creates an isolated in-memory database per
test with `max_connections(1)`.

### Multi-instance test scenarios

Construct two store instances over the same database and exercise concurrent
operations: (1) **schema init** — both call `create_tables()`; assert no
"duplicate key" errors, tables exist once; (2) **pruning** — set
`max_entries(10)` on both, insert 20 interleaved; assert exactly 10 remain
(race condition #4); (3) **session switch** — `store_a.switch_session("s2")`,
`store_b.sync_current_session()`; assert `store_b.current_session_id() == "s2"`;
(4) **audit chain** — log 50 events concurrently; assert chain unbroken. A new
backend must pass these using its lock mechanism (`GET_LOCK` for MySQL,
`BEGIN IMMEDIATE` for SQLite).

---

## Migration Guide

### SQLite to PostgreSQL

The schema is identical (modulo type names); the trait abstraction means no
application code changes — only store construction changes.

1. **Provision PostgreSQL** and enable `pg_trgm`:
   ```sql
   CREATE USER madhyamas WITH PASSWORD '...';
   CREATE DATABASE madhyamas OWNER madhyamas;
   \c madhyamas
   CREATE EXTENSION IF NOT EXISTS pg_trgm;
   ```
2. **Start Madhyamas with the PostgreSQL URL** — constructors run idempotent
   DDL, creating all tables and indexes automatically.
3. **Export from SQLite, import into PostgreSQL:**
   ```bash
   # Export
   madhyamas export har --session default-session --output traffic.har
   curl http://127.0.0.1:3001/api/intercept/export > intercept.json
   curl http://127.0.0.1:3001/api/config/export > config.json
   # Import (into the new PostgreSQL-backed instance)
   madhyamas import har --input traffic.har --session "Migrated Session"
   curl -X POST http://127.0.0.1:3001/api/intercept/import \
     -H 'Content-Type: application/json' -d @intercept.json
   curl -X POST http://127.0.0.1:3001/api/config/import \
     -H 'Content-Type: application/json' -d @config.json
   ```
4. **Migrate enterprise data** (users, API keys, audit) — no built-in export;
   use `pg_loader` or a custom script. Preserve the audit hash chain by
   inserting events in timestamp order.
5. **Verify** — compare entry counts, run `GET /health`, confirm the WebSocket
   event stream is live.

### Data export/import notes

- **HAR** (`export_har`) covers one session — repeat per session. Bodies are
  inline (base64); for large recordings, prefer a direct DB-level migration.
- **Intercept** (`export_all`) is a single JSON blob with all rules.
- **Config** (`export`) is the full `PersistedConfig` as JSON.
- **Session counters**: recompute after import:
  ```sql
  INSERT INTO session_counters (session_id, entry_count)
  SELECT session_id, COUNT(*) FROM requests GROUP BY session_id
  ON CONFLICT (session_id) DO UPDATE SET entry_count = EXCLUDED.entry_count;
  ```

---

**See Also:** [PERSISTENCE.md](PERSISTENCE.md) · [ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) · [ENTERPRISE_PERF_SECURITY.md](ENTERPRISE_PERF_SECURITY.md) §6 · [ENTERPRISE_MULTI_INSTANCE.md](ENTERPRISE_MULTI_INSTANCE.md) · [PGBOUNCER.md](PGBOUNCER.md)
