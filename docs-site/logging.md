---
title: Logging & Log Rotation
description: Configure Madhyamas log file rotation by time or size, inspect log status, rotate on demand, and tune log levels — prevents unbounded log growth on long-running deployments.
---

# Logging & Log Rotation

Madhyamas writes log events to both **stdout** (for `tail` and `docker logs`) and a **rotating file** at `<log_path>/madhyamas.log` (default `~/.madhyamas/logs/madhyamas.log`). The binary manages its own rotation, so you don't need an external log-rotation daemon.

## Why Rotation?

Without rotation, a long-running proxy can produce an unbounded log file (100 GB+ has been observed on busy deployments). The built-in `RotatingFileWriter` caps file size and prunes old archives automatically, so disk usage stays bounded.

## Configuration

Log rotation is configured via the `log_config` section of the proxy config, the `PATCH /api/logs` endpoint, the `madhyamas logs config` CLI subcommand, or the `madhyamas_update_log_config` MCP tool.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `true` | Master switch. When `false`, logs go to stdout only. |
| `rotation` | object | `{"mode": "daily"}` | Rotation strategy (see below) |
| `max_files` | int | `7` | Max archived files to keep (oldest pruned first) |
| `max_file_size_mb` | int | `100` | Hard per-file size cap (MB) — safety net for time-based modes |
| `json_format` | bool | `false` | Write structured JSON instead of human-readable text (restart required) |

### Rotation Modes

The `rotation` field is a tagged enum:

```json
{"mode": "never"}                      // no time/size rotation (not recommended)
{"mode": "hourly"}                     // rotate at the top of each hour
{"mode": "daily"}                      // rotate at midnight local time (default)
{"mode": "size", "size_mb": 50}        // rotate when the file exceeds 50 MB
```

Even with time-based rotation (`hourly`/`daily`), the `max_file_size_mb` cap is enforced as a safety net — a single file that exceeds the cap is rotated immediately, so a file can never grow unbounded between scheduled rotations.

## On-Demand Rotation

Rotate the current log file immediately, regardless of the configured mode:

```bash
# REST API
curl -X POST http://localhost:3001/api/logs/rotate -H 'Content-Type: application/json' -d '{}'

# CLI
madhyamas logs rotate
```

The current `madhyamas.log` is renamed to `madhyamas.log.<YYYY-MM-DD_HH-MM-SS>` and a fresh file is opened. Archived files are pruned to `max_files`.

## Inspecting Status

```bash
# REST API
curl http://localhost:3001/api/logs

# CLI
madhyamas logs status
```

Returns the current config, the active log file path and size, and the list of archived (rotated) files with their sizes and modification times.

## Updating Configuration at Runtime

```bash
# CLI: switch to size-based rotation, 50 MB per file, keep 3 archives
madhyamas logs config --rotation size --size-mb 50 --max-files 3

# REST API
curl -X PATCH http://localhost:3001/api/logs \
  -H 'Content-Type: application/json' \
  -d '{"rotation":{"mode":"size","size_mb":50},"max_files":3}'
```

Changes to `max_files` and `max_file_size_mb` take effect immediately. Changes to `rotation` mode and `json_format` take effect on the next restart (the tracing subscriber layer is installed once at startup). All changes are persisted to `~/.madhyamas/config.json` and survive restarts.

## Log Levels

Control verbosity with the `RUST_LOG` environment variable:

| Value | What you see |
|-------|--------------|
| `error` | Only errors |
| `warn` | Warnings and errors |
| `info` | Info, warnings, errors (default) |
| `debug` | Debug, info, warnings, errors |
| `trace` | Everything, including very verbose internal events |

```bash
RUST_LOG=debug madhyamas serve
```

For targeted verbosity (e.g. only the proxy engine):

```bash
RUST_LOG=madhyamas_core::proxy=debug,info madhyamas serve
```

## How It Works

- A custom `RotatingFileWriter` (in `madhyamas-core/src/log_rotation.rs`) implements `std::io::Write` and is wrapped as a `tracing_subscriber` `MakeWriter` so it can be used as a `fmt` layer alongside the stdout layer.
- A background Tokio task wakes every 60 seconds to perform time-based rotation (hourly/daily) and prune archived files.
- Size-based rotation is checked on every write (per-event), so a burst of logs that exceeds the cap triggers rotation immediately.
- The `LogHandle` is stored in the API `AppState`, enabling on-demand rotation and config updates from HTTP handlers.

## MCP Mode

In [MCP](./mcp) mode, logs go to **stderr only** — writing to stdout would corrupt the JSON-RPC stream that the AI agent reads. File rotation still works if enabled.

## See also

- [Configuration](./configuration) — `RUST_LOG` and other environment variables
- [REST API reference](./rest-api) — `/api/logs` endpoints
- [CLI reference](./cli) — `madhyamas logs` subcommands
- [MCP & AI Agents](./mcp) — `madhyamas_get_log_status`, `madhyamas_update_log_config`, `madhyamas_rotate_logs`
- [Performance](https://github.com/ShristiLabs/madhyamas/blob/main/docs/PERFORMANCE.md) — memory tracking and metrics
