# Log File Rotation

Madhyamas writes log events to both **stdout** (for `tail`/`docker logs`) and
a **rotating file** at `<log_path>/madhyamas.log` (default
`~/.madhyamas/logs/madhyamas.log`).

## Why rotation?

Previously, the `startup-local.sh` script redirected stdout into a single
`madhyamas.log` file with no rotation, size cap, or cleanup. On long-running
deployments this file grew without bound (100GB+ was observed). The binary now
manages its own rotating log files, so the shell redirect was removed.

## Configuration

Log rotation is configured via the `log_config` section of the proxy config
(`~/.madhyamas/config.json`), the `PATCH /api/logs` endpoint, the
`madhyamas logs config` CLI subcommand, or the `madhyamas_update_log_config`
MCP tool.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Master switch. When `false`, logs go to stdout only. |
| `rotation` | object | `{"mode": "daily"}` | Rotation strategy (see below). |
| `max_files` | int | `7` | Max archived files to keep (oldest pruned first). |
| `max_file_size_mb` | int | `100` | Hard per-file size cap (MB). Safety net for time-based modes. |
| `json_format` | bool | `false` | Write structured JSON instead of human-readable text (restart required). |
| `async_writing` | bool | `true` | Decouple file writes from emitting threads via a dedicated writer thread (restart required). |
| `async_mode` | string | `"lossless"` | Overflow policy: `lossless` (block producers when the buffer is full) or `lossy` (drop and count). Runtime-toggleable. |
| `async_buffer_size` | int | `8192` | Bounded async buffer capacity, in events (restart required). |

### Rotation modes

The `rotation` field is a tagged enum:

```json
{"mode": "never"}     // no time/size rotation (not recommended)
{"mode": "hourly"}    // rotate at the top of each hour
{"mode": "daily"}     // rotate at midnight local time (default)
{"mode": "size", "size_mb": 50}  // rotate when the file exceeds 50 MB
```

Even with time-based rotation (`hourly`/`daily`), the `max_file_size_mb` cap
is enforced as a safety net — a single file that exceeds the cap is rotated
immediately, so a file can never grow unbounded between scheduled rotations.

## Asynchronous file writing

With `async_writing: true` (the default), the file layer does not touch the
disk on the thread that emits a log event. Events are enqueued into a
bounded buffer drained by a single dedicated writer thread that owns the
write side of the rotating file (implemented in
`crates/madhyamas-core/src/async_log.rs`):

- **No mutex-guarded file I/O on request threads** — proxy request latency
  is decoupled from disk I/O latency spikes; concurrent events no longer
  serialize through the file mutex.
- **Overflow policy** (`async_mode`):
  - `lossless` (default): producers park until the buffer has space
    (bounded backpressure). No events are ever lost.
  - `lossy`: events are dropped when the buffer is full and a
    dropped-event counter is incremented.
- **Shutdown flush**: the `LogHandle` holds the writer guard for the
  process lifetime; on graceful shutdown (SIGINT/SIGTERM) all buffered
  events are drained and flushed before the process exits.
- **Rotation safety**: size-based rotation happens on the writer thread,
  and time-based rotation/pruning continue on the existing background task
  against the same shared file state, so no lines are lost or duplicated
  across rotation boundaries.
- **stdout stays synchronous** — only the file layer is asynchronous.

Buffer state is visible in `GET /api/logs` under `async`:

```json
{
  "async": {
    "enabled": true,
    "mode": "lossless",
    "buffer_size": 8192,
    "buffer_depth": 0,
    "high_water": 14,
    "dropped_events": 0,
    "written_events": 21034
  }
}
```

`async_mode` can be switched at runtime (`PATCH /api/logs` with
`{"async_mode": "lossy"}`); `async_writing` and `async_buffer_size` are
applied at startup (persisted changes take effect on the next restart).
CLI flags for startup: `--log-sync`, `--log-async-mode <lossless|lossy>`,
`--log-buffer-size <n>`; runtime CLI:
`madhyamas logs config --async-mode lossy`.

## Structured log schema

With `json_format: true`, the file layer emits one JSON object per event
with event fields flattened to the top level. Proxied-traffic events
(target `madhyamas::debug_log`) conform to this stable schema:

| Field | Type | Present | Description |
|-------|------|---------|-------------|
| `timestamp` | string | always | RFC 3339 event time. |
| `level` | string | always | `TRACE`/`DEBUG`/`INFO`/`WARN`/`ERROR`. |
| `target` | string | always | `tracing` target (e.g. `madhyamas::debug_log`). |
| `request_id` | string | proxy events | Correlation id; all events for one request share it. |
| `connection_id` | string | proxy events | Correlation id of the client connection. |
| `method` | string | proxy events | HTTP method. |
| `host` | string | proxy events | Destination host. |
| `path` | string | proxy events | URL path. |
| `status` | number | response events | HTTP status code. |
| `duration_ms` | number | response events | Upstream/response latency in milliseconds. |
| `source` | string | response events | `upstream`, `mocked`, `blocked`, `script`, `breakpoint response`, `error`. |
| `rule_hit` | string/null | proxy events | Name of the matched intercept rule (e.g. mock), when known. |
| `direction` | string | proxy events | `request` or `response`. |
| `headers`/`body` | string | headers/full levels | Redacted per `debug_logging.redact_headers`. |

Request correlation is also carried by tracing spans: each proxied request
runs inside a `proxy_request` span with `request_id`/`connection_id` fields
(one `connection_id` per client connection), so any event emitted during
request processing — including events from other modules — is correlated
through the span list.

### Boundary with the enterprise audit trail

These structured logs are **operational** observability output: they answer
"what is the proxy doing and why is a request slow/failing", rotate locally,
and are safe to ship to log aggregators (ELK, Loki, Datadog) with redaction
already applied. The **enterprise audit event trail**
(`crates/madhyamas-enterprise/src/store/postgres.rs` and the SQLite audit
store) is a separate, tamper-evident record of security-relevant admin
actions (logins, user/role changes, config changes) stored in the database
with hash-chaining. The two systems complement each other and do not
duplicate content: operational logs cover proxied traffic and engine
diagnostics; the audit trail covers administrative actions and
authentication events. Do not feed one into the other.

## On-demand rotation

You can rotate the current log file immediately at any time, regardless of the
configured rotation mode:

**API:**
```bash
curl -X POST http://localhost:3001/api/logs/rotate -H 'Content-Type: application/json' -d '{}'
```

**CLI:**
```bash
madhyamas logs rotate
```

**MCP:**
```
madhyamas_rotate_logs
```

The current `madhyamas.log` is renamed to `madhyamas.log.<YYYY-MM-DD_HH-MM-SS>`
and a fresh file is opened. Archived files are pruned to `max_files`.

## Inspecting status

**API:** `GET /api/logs`
**CLI:** `madhyamas logs status`
**MCP:** `madhyamas_get_log_status`

Returns the current config, the active log file path and size, and the list of
archived (rotated) files with their sizes and modification times.

## Updating configuration at runtime

**API:** `PATCH /api/logs`
**CLI:** `madhyamas logs config --rotation size --size-mb 50 --max-files 3`
**MCP:** `madhyamas_update_log_config`

Changes to `max_files` and `max_file_size_mb` take effect immediately. Changes
to `rotation` mode and `json_format` take effect on the next restart (the
tracing subscriber layer is installed once at startup). All changes are
persisted to `~/.madhyamas/config.json` and survive restarts.

## Proxied-traffic debug logging

Besides the engine's own diagnostics, Madhyamas can log each proxied
request/response as structured events into the same main rotated log / stdout
(no separate debug file). Events use the dedicated `tracing` target
`madhyamas::debug_log` and are gated in code by the runtime setting, so
toggling takes effect without a restart (new connections pick up the config
snapshot from the shared proxy config). This is the proxy's diagnostic
logging — distinct from the traffic capture shown in the web UI.

Configured via the `debug_logging` section of the proxy config, the
`debug_logging` object in `PATCH /api/logs`, or the web UI settings dialog
(Debug Logging tab):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Master switch (runtime-toggleable, no restart). |
| `level` | string | `"summary"` | Verbosity: `summary`, `headers`, or `full`. |
| `host_filter` | string[] | `null` | Host patterns to log (empty/omitted = all hosts). |
| `redact_headers` | string[] | `["Authorization", "Cookie", "Set-Cookie"]` | Headers replaced with `[REDACTED]` (case-insensitive). |
| `redact_bodies` | bool | `false` | Never log body content (size placeholder only). |

Verbosity levels:

- `summary` — one event per request/response with method, host, path,
  status, duration, and source (`upstream`, `mocked`, `blocked`, `script`,
  `breakpoint response`, `error`).
- `headers` — summary fields plus all headers (sensitive headers redacted).
- `full` — headers plus bodies, size-capped at the traffic-capture
  `max_body_size`. Compressed bodies (gzip, deflate, br, zstd) are
  decompressed first; non-text binaries (images, protobuf, ...) are replaced
  with a `[binary body: N bytes, content-type ...]` placeholder.

Host filter patterns reuse the traffic-capture matcher: exact hosts, suffix
domains (`example.com` matches `api.example.com`), wildcard subdomains
(`*.example.com`), and globs (`*api*`).

Example:

```bash
curl -X PATCH http://localhost:3001/api/logs \
  -H 'Content-Type: application/json' \
  -d '{"debug_logging": {"enabled": true, "level": "headers", "host_filter": ["api.example.com"]}}'
```

`GET /api/logs` includes the current `debug_logging` section in its status
response. Implementation lives in `madhyamas-core/src/debug_log.rs`, with
hook points in `madhyamas-core/src/proxy/pipeline.rs` (request entry,
upstream response, short-circuit responses, and upstream errors).

## How it works

- A custom `RotatingFileWriter` (in `madhyamas-core/src/log_rotation.rs`)
  implements `std::io::Write` and is wrapped as a `tracing_subscriber`
  `MakeWriter` so it can be used as a `fmt` layer alongside the stdout layer.
- A background `tokio` task wakes every 60 seconds to perform time-based
  rotation (hourly/daily) and prune archived files.
- Size-based rotation is checked on every write (per-event), so a burst of
  logs that exceeds the cap triggers rotation immediately.
- The `LogHandle` is stored in the API `AppState`, enabling on-demand
  rotation and config updates from HTTP handlers.

## Backward compatibility

- The existing `log_path` config field is reused as the log **directory**.
- Old config files without a `log_config` section deserialize with defaults
  (enabled, daily rotation, 7 files, 100 MB cap).
- MCP mode continues to log to stderr only (to avoid corrupting stdio
  JSON-RPC).
- The `startup-local.sh` script no longer redirects stdout to an unbounded
  file; it captures only stderr (`madhyamas.stderr.log`) for crash diagnostics.
