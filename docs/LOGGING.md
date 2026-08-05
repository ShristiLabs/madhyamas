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
