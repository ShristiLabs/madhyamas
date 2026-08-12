# Persistence Layer

> **Last verified:** 2026-08-12 against Madhyamas `0.1.6`.

## Overview

Madhyamas persists traffic, sessions, intercept rules, scripts, and plugins to
SQLite. The persistence layer is split across three stores:

- **Traffic store** — `crates/madhyamas-core/src/traffic/store.rs`
- **Intercept store** — `crates/madhyamas-core/src/persistence/intercept_store.rs`
- **Config store** — `crates/madhyamas-core/src/persistence/config_store.rs`

The `Persistable` trait (`persistence/mod.rs`) defines a common interface for
in-memory state that can be saved/loaded.

## SQLite Schema

```mermaid
erDiagram
    sessions ||--o{ requests : has
    sessions ||--o{ ws_connections : has
    requests ||--|| responses : has
    ws_connections ||--o{ ws_messages : has

    sessions {
        TEXT id PK
        TEXT name
        INTEGER created_at
        INTEGER updated_at
    }
    requests {
        TEXT id PK
        TEXT session_id FK
        TEXT method
        TEXT url
        TEXT host
        TEXT path
        TEXT headers
        BLOB body
        TEXT content_type
        INTEGER timestamp
        INTEGER modified
        TEXT notes
        INTEGER is_passthrough
        TEXT http_version
    }
    responses {
        TEXT request_id PK
        INTEGER status_code
        TEXT status_message
        TEXT headers
        BLOB body
        TEXT content_type
        INTEGER duration_ms
        TEXT http_version
    }
    ws_connections {
        TEXT id PK
        TEXT session_id FK
        TEXT url
        TEXT host
        TEXT path
        TEXT state
        TEXT request_headers
        TEXT response_headers
        TEXT subprotocol
        INTEGER created_at
        INTEGER closed_at
        INTEGER messages_sent
        INTEGER messages_received
        INTEGER bytes_sent
        INTEGER bytes_received
    }
    ws_messages {
        TEXT id PK
        TEXT connection_id FK
        TEXT direction
        TEXT message_type
        BLOB payload_raw
        TEXT payload_text
        INTEGER opcode
        INTEGER is_final
        INTEGER mask
        INTEGER timestamp
    }
    focus_hosts {
        TEXT id PK
        TEXT pattern
        INTEGER created_at
    }
```

### PRAGMA settings

- `synchronous=NORMAL` — safe with WAL mode, faster than FULL.
- `cache_size=-64000` — 64 MB page cache for read performance.

## Traffic Store (`traffic/store.rs`)

Tables: `sessions`, `requests`, `responses`, `ws_connections`, `ws_messages`,
`focus_hosts`. Relationships:

- `sessions` 1:N `requests`
- `requests` 1:1 `responses`
- `sessions` 1:N `ws_connections`
- `ws_connections` 1:N `ws_messages`

Indexes optimize the common query patterns: per-session lookups, URL/method
filtering, and timestamp ordering.

## Intercept Store (`persistence/intercept_store.rs`)

Stores the five intercept rule types plus the throttle profile. Each rule table
includes `enabled`, `priority`, `hit_count`, and timestamps.

| Table | Key columns | Notes |
|-------|-------------|-------|
| `mock_rules` | `id`, `name`, `condition` (JSON), `response_config` (JSON), `enabled`, `priority`, `hit_count`, `collection_id`, `tags` | Indexes on `enabled` and `priority`; schema migration for old 8-column format |
| `rewrite_rules` | `id`, `name`, `condition` (JSON), `direction` (JSON), `rewrites` (JSON), `enabled`, `priority`, `hit_count` | Indexes on `enabled` and `priority` |
| `breakpoint_rules` | `id`, `name`, `condition` (JSON), `direction` (JSON), `enabled`, `priority` | Index on `enabled` |
| `throttle_profile` | `id` (single-row, `CHECK (id = 1)`), `download_bps`, `upload_bps`, `latency_ms`, `jitter_ms`, `packet_loss_percent`, `enabled` | Singleton row |
| `block_list_entries` | `id`, `pattern`, `note`, `enabled`, `hit_count`, `status_code`, `response_body`, `content_type` | Index on `enabled` |

`export_all()` and `import_all()` serialize all rules to/from a single JSON
bundle (exposed via `/api/persistence/export` and `/api/persistence/import`).

## Config Store (`persistence/config_store.rs`)

A simple key-value store backed by a single table:

```sql
CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
)
```

Values are JSON-serialized. `PersistedConfig` is the typed wrapper for the
common config keys (proxy/api addresses, cert/data dirs, log level, theme,
window size, column widths, custom settings).

## `Persistable` Trait

```rust
pub trait Persistable {
    fn save(&self) -> Result<()>;
    fn load(&self) -> Result<()>;
    fn clear(&self) -> Result<()>;
    fn size(&self) -> usize;
}
```

Implemented by `ReplayManager`, `WsManager`, and `GrpcManager`. Note: the
current implementations are in-memory no-ops — these managers keep state in
memory for the lifetime of the process.

## Session Model (`session.rs`)

`SessionManager` wraps `TrafficStore` to provide session CRUD, export/import,
and presets:

- `SessionMetadata` / `SessionSummary` — session descriptors with request counts.
- `SessionExport` — versioned export format (currently `"1.0"`) containing the
  session metadata and all traffic entries.
- `SessionPreset` — built-in presets: "API Debugging", "Mobile App",
  "Performance Testing" (auto-clears entries older than 24h).

## Data Directory

All SQLite databases live under `~/.madhyamas/` (certs, logs, `traffic.db`,
plugins). The path can be overridden via configuration.

## See Also

- [API_CONFIG.md](API_CONFIG.md) — Persistence API endpoints
- [ARCHITECTURE.md](ARCHITECTURE.md) — System architecture
- [RECORDING_LIMITS.md](RECORDING_LIMITS.md) — Recording size limits and FIFO pruning
- [HAR_IMPORT.md](HAR_IMPORT.md) — HAR import/export
