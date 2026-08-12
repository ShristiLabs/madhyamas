---
title: Replay
description: Re-execute captured requests in Madhyamas — single replay, edit-then-repeat with diff-based modifications, and batch replay with iterations, concurrency, and latency stats.
---

# Replay

Replay lets you **re-execute previously captured requests** — either one at a time or as a saved sequence. This is invaluable for reproducing issues, testing the same request against different servers, or automating repetitive API testing.

![Replay View](/screenshots/replay-view.png)

## How Replay Works

When you replay a request, Madhyamas sends it to the server again using the original method, URL, headers, and body. The response is captured as a new traffic entry, so you can compare it with the original.

## Replaying a Single Request

### From the Traffic View

1. Right-click any traffic entry in the traffic list
2. Select **Replay** from the context menu
3. The request is sent immediately
4. The new response appears as a new traffic entry

### From the Replay View

1. Navigate to the **Replay** view
2. Select a previously saved request from the list
3. Click **Replay** to execute it
4. View the response in the detail panel

## Saving Requests for Replay

Instead of searching through traffic history, you can save specific requests for easy replay:

1. Right-click a traffic entry → **Save for Replay**
2. Give it a name and optional description
3. It appears in the Replay view's saved list

Saved requests persist across restarts, so you can build a library of commonly tested API calls.

## Modifying Before Replay (Edit-then-Repeat)

Before replaying, you can modify a saved request and replay it with your changes. The editor is **diff-based** — it compares your edited values against the original and sends only the fields that changed, so the original request is preserved and only the edited parts are overridden.

### From the Web UI

1. In the **Saved Requests** list, click **Edit & Replay** next to a saved request
2. The **Request Editor** dialog opens, pre-filled with the saved request's method, URL, headers, and body
3. Modify any fields:
   - **Method** — dropdown selector (GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS)
   - **URL** — text input
   - **Headers** — key-value editor with add/remove rows
   - **Content-Type** — selector that auto-detects from headers
   - **Body** — monospace textarea
4. Click **Replay with Changes**. The editor sends only the changed fields as modifications.
5. The replay result dialog shows the response status, headers, and body.

The existing **Replay** button remains for quick no-edit replay.

### From the CLI

The `madhyamas replay run` command accepts modification flags:

```bash
# Simple replay (no modifications)
madhyamas replay run <saved-request-id>

# Override URL and method
madhyamas replay run <id> --url https://staging.example.com/api --method POST

# Add/replace headers (repeatable)
madhyamas replay run <id> --header "Authorization: Bearer token" --header "X-Custom: value"

# Override body from text or a file
madhyamas replay run <id> --body '{"key":"value"}'
madhyamas replay run <id> --body-file ./payload.json

# Follow redirects
madhyamas replay run <id> --follow-redirects
```

| Flag | Description |
|------|-------------|
| `--url <URL>` | Override the request URL |
| `--method <METHOD>` | Override the HTTP method |
| `--header "Key: Value"` | Add/replace a header (repeatable) |
| `--body <TEXT>` | New request body (raw text) |
| `--body-file <PATH>` | Read request body from a file |
| `--follow-redirects` | Follow 3xx redirect responses |

This is useful for:
- Testing different parameter values
- Adding or removing headers
- Changing the request body
- Pointing to a different server

## Repeat Advanced (Batch Replay)

Repeat Advanced replays a saved request **multiple times** with configurable **iterations**, **concurrency**, and an optional **inter-request delay**, then returns aggregate statistics (success/failure counts and latency percentiles). This is useful for basic load testing, performance benchmarking, and verifying endpoint stability under concurrent load.

### From the Web UI

1. Open the **Replay** panel
2. Find a saved request and click the **Advanced** button
3. Adjust the **Iterations** slider (1–1000 in the UI)
4. Adjust the **Concurrency** slider (1–100)
5. Optionally toggle **Delay between requests** and enter a delay in milliseconds
6. Click **Run Batch**. A results summary appears with success/failure counts and a latency statistics table (min/avg/max/p95).

### From the CLI

```bash
# Replay a saved request 100 times, 10 concurrent, 50ms delay
madhyamas replay run-advanced <id> \
  --iterations 100 \
  --concurrency 10 \
  --delay-ms 50

# With modifications (same flags as replay run)
madhyamas replay run-advanced <id> \
  --iterations 50 --concurrency 5 \
  --url https://staging.example.com/api/users \
  --header "Authorization: Bearer token"
```

| Flag | Description | Default |
|------|-------------|---------|
| `--iterations` | Total number of requests to send (max 10,000) | 1 |
| `--concurrency` | Simultaneous in-flight requests (max 100) | 1 |
| `--delay-ms` | Delay between requests in milliseconds | (none) |

### Result Statistics

The batch result reports:

| Field | Description |
|-------|-------------|
| **Total** | Total number of requests sent |
| **Succeeded** | Requests that completed without error |
| **Failed** | Requests that errored |
| **Min / Avg / Max** | Latency of successful requests |
| **P95** | 95th percentile latency of successful requests |

::: tip
To prevent accidental denial-of-service against upstream servers, iterations are capped at 10,000 and concurrency at 100.
:::

## Replay History

Every replay execution is recorded in the **Replay History** tab. Each entry shows:

- The original request that was replayed
- The timestamp of the replay
- The response status code and timing
- The full response details

This lets you compare results across multiple replays — for example, to see if a server's response has changed over time.

## Common Use Cases

### Reproducing a Bug

Capture the exact request that caused a bug, save it, and replay it after each code change to verify the fix works.

### API Regression Testing

Save a set of key API requests and replay them after deploying a new version to verify the responses haven't changed.

### Performance Comparison

Replay the same request multiple times and compare response times to track performance trends.

### Testing Different Environments

Save a request, then modify the URL to point to staging or production, and replay to compare responses across environments.

## See also

- [Breakpoints](./breakpoints) — modify requests interactively
- [Mocks](./mocks) — return fake responses without replaying
- [Throttling](./throttling) — replay under simulated network conditions
- [CLI reference](./cli) — `madhyamas replay` subcommands
- [REST API reference](./rest-api) — `/api/replay` endpoints
