---
title: CLI Reference
description: Drive Madhyamas from the command line — 128 subcommands across traffic, mocks, breakpoints, replay, sessions, scripts, plugins, config, export, and more, with JSON output for scripting.
---

# CLI Reference

The `madhyamas` binary is a single unified executable that contains the proxy, web UI, MCP server, and CLI. The CLI subcommands talk to a running proxy instance over its REST API, so you can inspect traffic, create mocks, manage sessions, and export data from scripts, CI pipelines, or terminals without opening the browser.

## Prerequisites

- Madhyamas installed and on your `PATH` (`madhyamas --version` works)
- The proxy running (`madhyamas serve`) and reachable at the API URL
- For remote proxies, set `--api-url` or `MADHYAMAS_API_URL`

## Global Flags

```
madhyamas [GLOBAL FLAGS] <command> [subcommand] [options]

Global Flags:
  --api-url <URL>     API server URL [default: http://127.0.0.1:3001]
                       [env: MADHYAMAS_API_URL]
  -v, --verbose        Enable verbose logging
  -h, --help           Print help
  -V, --version        Print version
```

Most subcommands support `--json` for machine-readable JSON output, which is useful for piping into `jq` or other tools.

## Subcommand Groups

The CLI is organized into 20 command groups, mirroring the features available in the web UI.

| Group | Purpose | Key subcommands |
|-------|---------|-----------------|
| `traffic` | Inspect captured traffic | `list`, `get`, `search`, `count`, `clear` |
| `mocks` | Manage mock responses | `list`, `create`, `delete`, `toggle`, `update`, `create-advanced`, `analytics`, `history`, `export`, `import`, `recording`, `collections`, `templates` |
| `breakpoints` | Manage breakpoints | `list`, `get`, `create`, `delete`, `paused` |
| `sessions` | Manage sessions | `list`, `create`, `delete`, `switch`, `export` |
| `replay` | Replay saved requests | `run`, `run-advanced`, `save`, `list`, `delete`, `export`, `history` |
| `config` | Get and update config | `get`, `update` |
| `capture` | Toggle capture mode | `status`, `toggle`, `enable`, `disable` |
| `throttle` | Network throttling | `get`, `set`, `enable`, `disable`, `presets` |
| `rewrites` | Manage rewrite rules | `list`, `create`, `update`, `delete`, `toggle`, `batch-toggle`, `templates` |
| `grpc` | gRPC inspection | `connections`, `streams`, `frames`, `stats`, `clear` |
| `scripts` | Manage JS scripts | `list`, `create`, `delete`, `toggle`, `validate`, `test`, `history`, `templates` |
| `plugins` | Manage WASM plugins | `list`, `enable`, `disable`, `install`, `uninstall`, `reload`, `stats`, `logs`, `schema`, `get-settings`, `set-settings`, `registry`, `search`, `registry-config`, `registry-refresh`, `templates`, `new`, `gen-key`, `sign` |
| `export` | Export traffic | `har`, `curl` |
| `autosave` | Auto Save config | `get`, `update`, `snapshot` |
| `blocklist` | Block list management | `list`, `add`, `remove`, `toggle`, `clear` |
| `focus` | Focus hosts | `list`, `add`, `remove`, `clear` |
| `logs` | Log rotation | `get`, `rotate`, `level` |
| `mirror` | Mirror tool | `get`, `update`, `trigger` |
| `wstraffic` | WebSocket traffic | `connections`, `messages`, `stats`, `clear` |
| `serve` | Start the proxy | (default subcommand) |
| `mcp` | Run as MCP server | (stdio transport) |

Run `madhyamas --help` to see every subcommand, and `madhyamas <group> --help` for options of a specific group.

## Common Tasks

### Inspect traffic

```bash
# List the 20 most recent requests
madhyamas traffic list --limit 20

# Filter POSTs that returned 500
madhyamas traffic list --method POST --status 500 --json | jq .

# Search bodies for an auth token
madhyamas traffic search "authorization"

# Get full headers and body of one entry
madhyamas traffic get <id>
```

### Manage mocks

```bash
# Create a simple mock
madhyamas mocks create \
  --url-pattern "*/api/users/*" \
  --status-code 200 \
  --content-type "application/json" \
  --body '{"id":1,"name":"Alice"}'

# Toggle a mock off
madhyamas mocks toggle <mock-id> --enabled false

# Export all mocks to JSON
madhyamas mocks export --output mocks.json
```

### Replay requests

```bash
# Replay a saved request as-is
madhyamas replay run <saved-request-id>

# Batch replay: 100 iterations, 10 concurrent, 50ms delay
madhyamas replay run-advanced <id> \
  --iterations 100 --concurrency 10 --delay-ms 50

# Replay with a modified URL and extra header
madhyamas replay run <id> \
  --url https://staging.example.com/api \
  --header "Authorization: Bearer token"
```

### Sessions

```bash
# Create and switch to a new session
madhyamas sessions create --name "debug-auth"
madhyamas sessions switch <session-id>

# Export the current session as HAR
madhyamas sessions export <session-id> --format har --output auth.har
```

### Export traffic

```bash
# Export everything as HAR
madhyamas export har --output traffic.har

# Get a cURL command for a specific request
madhyamas export curl <traffic-id>
```

### Configuration

```bash
# View current config
madhyamas config get

# Enable HTTPS interception
madhyamas config update --intercept-https true

# Increase the in-memory request limit
madhyamas config update --max-requests 50000
```

### Scripts and plugins

```bash
# Validate a script without enabling it
madhyamas scripts validate --file ./my-script.js

# Test a script against a sample request
madhyamas scripts test --file ./my-script.js --hook on_request

# Install a plugin from the registry
madhyamas plugins install --source registry cors-helper
```

## Using the CLI in CI and Scripts

Because every subcommand supports `--json`, the CLI is script-friendly. A typical CI snippet:

```bash
# Start the proxy in the background
madhyamas serve --host 0.0.0.0 &
PROXY_PID=$!

# Wait for it to be ready
until curl -sf http://localhost:3001/api/health >/dev/null; do sleep 0.5; done

# Run your test suite (configured to use localhost:8888 as its proxy)
./run-tests.sh

# Export any 5xx responses as HAR for artifact upload
madhyamas traffic list --status 500 --json | jq -r '.[].id' | while read id; do
  madhyamas export curl "$id"
done

# Clean up
kill $PROXY_PID
```

## See also

- [Getting Started](./getting-started) — installation and first run
- [Configuration](./configuration) — startup flags and environment variables
- [MCP & AI Agents](./mcp) — drive Madhyamas from an LLM via MCP
- [REST API reference](./rest-api) — the HTTP API the CLI calls under the hood
- [Troubleshooting](./troubleshooting) — common CLI and proxy issues
- [Enterprise CLI & MCP Tools](./enterprise/cli-mcp) — enterprise commands (users, audit, license, auth, API keys)
