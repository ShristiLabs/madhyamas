# Repeat Advanced (Batch Replay)

> **Last verified:** 2026-08-12 against Madhyamas `0.1.6`.

Repeat Advanced is the Madhyamas equivalent of Charles Proxy's "Repeat
Advanced" tool. It replays a saved request multiple times with configurable
**iterations**, **concurrency**, and an optional **inter-request delay**,
then returns aggregate statistics (success/failure counts and latency
percentiles). This is useful for basic load testing, performance
benchmarking, and verifying endpoint stability under concurrent load.

## How It Works

All iterations use the same `RequestModifications` (if any). Concurrency is
controlled with `futures::stream::buffer_unordered`, which keeps up to N
requests in flight at once. An optional `tokio::time::sleep` delay is applied
before each dispatch (skipping the first).

The result is a `ReplayBatchResult` containing:

| Field | Description |
|-------|-------------|
| `total` | Total number of requests sent |
| `succeeded` | Number of requests that completed without error |
| `failed` | Number of requests that errored |
| `min_ms` | Minimum latency (successful requests only) |
| `avg_ms` | Average latency (successful requests only) |
| `max_ms` | Maximum latency (successful requests only) |
| `p95_ms` | 95th percentile latency (successful requests only) |
| `results` | Individual `ReplayResult` entries (in completion order) |

## Safety Limits

To prevent accidental denial-of-service against upstream servers:

- **Iterations**: capped at **10,000**
- **Concurrency**: capped at **100**
- Zero values are normalized to 1

These limits are enforced by `ReplayBatchConfig::clamp_to_limits()`.

## Web UI

1. Open the **Replay** panel in the web UI.
2. Find a saved request and click the **Advanced** button.
3. Adjust the **Iterations** slider (1–1000 in the UI; the API supports up to
   10,000).
4. Adjust the **Concurrency** slider (1–100).
5. Optionally toggle **Delay between requests** and enter a delay in
   milliseconds.
6. Click **Run Batch**. A results summary appears with success/failure counts
   and a latency statistics table (min/avg/max/p95).

## CLI

```bash
# Replay a saved request 100 times, 10 concurrent, 50ms delay
madhyamas replay run-advanced <id> \
  --iterations 100 \
  --concurrency 10 \
  --delay-ms 50

# With modifications (same flags as `replay run`)
madhyamas replay run-advanced <id> \
  --iterations 50 \
  --concurrency 5 \
  --url https://staging.example.com/api/users \
  --header "Authorization: Bearer token" \
  --json
```

Flags:

| Flag | Description | Default |
|------|-------------|---------|
| `--iterations` | Total number of requests to send (max 10,000) | 1 |
| `--concurrency` | Simultaneous in-flight requests (max 100) | 1 |
| `--delay-ms` | Delay between requests in milliseconds | (none) |
| `--url` | Override the request URL | — |
| `--method` | Override the HTTP method | — |
| `--header` | Header to add/replace (repeatable). Format: `Key: Value` | — |
| `--body` | New request body (raw text) | — |
| `--body-file` | Read request body from a file | — |
| `--follow-redirects` | Follow 3xx redirect responses | false |
| `--json` | Output full result as JSON | false |

## MCP

AI agents can use the `madhyamas_replay_advanced` MCP tool:

```json
{
  "id": "<saved-request-id>",
  "iterations": 100,
  "concurrency": 10,
  "delay_ms": 50,
  "modifications": {
    "url": "https://staging.example.com/api/users",
    "headers": { "Authorization": "Bearer token" }
  }
}
```

The tool returns a formatted summary with success/failure counts and latency
statistics.

## API (curl)

```bash
curl -X POST http://127.0.0.1:3001/api/replay/execute/<id>/batch \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "iterations": 100,
      "concurrency": 10,
      "delay_ms": 50
    },
    "modifications": {
      "url": "https://staging.example.com/api/users"
    }
  }'
```

Response:

```json
{
  "saved_request_id": "<id>",
  "results": [ ... ],
  "total": 100,
  "succeeded": 98,
  "failed": 2,
  "min_ms": 12,
  "max_ms": 340,
  "avg_ms": 45,
  "p95_ms": 120,
  "started_at": "2025-01-01T00:00:00Z",
  "finished_at": "2025-01-01T00:00:05Z"
}
```

## Use Cases

- **Basic load testing**: Send N requests with concurrency C to measure how an
  endpoint handles concurrent load.
- **Performance benchmarking**: Collect latency statistics (min/avg/max/p95)
  over many requests to characterize endpoint performance.
- **Stability verification**: Run repeated requests to check for intermittent
  failures or flaky behaviour.
- **Rate-limit probing**: Use the delay parameter to space out requests and
  observe rate-limiting responses.

## Implementation

- **Core**: `crates/madhyamas-core/src/replay.rs` — `ReplayBatchConfig`,
  `ReplayBatchResult`, and `ReplayManager::replay_batch()`.
- **API**: `crates/madhyamas-api/src/intercept_handlers.rs` —
  `replay_request_batch` handler at `POST /api/replay/execute/{id}/batch`.
- **CLI**: `crates/madhyamas-cli/src/commands/replay.rs` — `RunAdvanced`
  subcommand.
- **MCP**: `crates/madhyamas-mcp/src/tools/replay.rs` —
  `replay_request_advanced()` function; `madhyamas_replay_advanced` tool.
- **Web UI**: `web/src/features/tools/ReplayPanel.tsx` — Advanced dialog with
  sliders and results summary.
