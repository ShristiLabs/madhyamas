# Recording Size Limits

Madhyamas enforces configurable limits on how much traffic is recorded to
prevent unbounded memory and disk usage during long debugging sessions.
When limits are exceeded, the oldest entries are automatically pruned
(FIFO — first in, first out).

## Why Recording Limits Matter

Without limits, a long-running proxy session capturing high-traffic
applications can accumulate gigabytes of data, leading to:

- Slow web UI performance (large database queries)
- High memory usage from SQLite page cache
- Disk space exhaustion
- Degraded proxy throughput

Recording limits ensure Madhyamas stays responsive regardless of traffic
volume.

## Configuration Options

All options can be set via the web UI (Config → Capture tab), the REST API
(`PATCH /api/config`), or the config file (`~/.madhyamas/config.json`).

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_requests` | `usize` | 10,000 | Maximum number of traffic entries to keep. When exceeded, oldest entries are pruned. Also exposed as `max_entries` in capture stats. |
| `max_total_size_mb` | `Option<usize>` | `None` (unlimited) | Maximum total size of all stored request/response bodies in megabytes. When exceeded, oldest entries are pruned until under the limit. Set to `null` to disable. |
| `max_body_size` | `usize` | 20 MB (20,971,520) | Maximum size of a single request or response body. Bodies larger than this are truncated before storage. |
| `capture_request_bodies` | `bool` | `true` | Whether to store request bodies. When `false`, request headers and metadata are still recorded but bodies are omitted. |
| `capture_response_bodies` | `bool` | `true` | Whether to store response bodies. When `false`, response headers and metadata are still recorded but bodies are omitted. |
| `ignored_domains` | `Vec<String>` | `[]` (empty) | Domains whose traffic should not be recorded at all. Supports exact match, suffix match, and wildcard patterns. |

## How Pruning Works

Pruning uses a **FIFO (first in, first out)** strategy — the oldest entries
(by timestamp) are deleted first.

### Entry Count Limit (`max_requests`)

- Checked on **every** `store_request()` call (cheap `COUNT(*)` query).
- If the entry count exceeds `max_requests`, the surplus oldest entries
  are pruned immediately.
- Set `max_requests` to `0` to disable the entry-count limit.

### Total Size Limit (`max_total_size_mb`)

- Checked **every 100 inserts** (expensive `SUM(LENGTH(body))` query).
- If the total body size exceeds the limit, oldest entries are pruned
  one by one until the total is under the limit.
- Set `max_total_size_mb` to `null` to disable the total-size limit.

### Pruning Events

When entries are pruned, a `TrafficEvent::Deleted` WebSocket event is
emitted with the list of pruned entry IDs. This allows the web UI to
remove the pruned entries from the traffic list in real time without
a full refresh.

## Ignored Domains

The `ignored_domains` setting prevents traffic from specified hosts from
being recorded at all. This is useful for filtering out noise from
analytics, telemetry, or CDN domains.

### Matching Rules

Matching is **case-insensitive** and supports three patterns:

| Pattern | Example | Matches |
|---------|---------|---------|
| Exact hostname | `example.com` | `example.com` |
| Suffix match | `example.com` | `example.com`, `api.example.com`, `www.example.com` |
| Wildcard subdomain | `*.example.com` | `api.example.com`, `www.example.com` (not `example.com` itself) |

### Example

```
ignored_domains:
  - "*.google-analytics.com"
  - "*.doubleclick.net"
  - "telemetry.example.com"
```

## Recording Quota Indicator

The web UI header displays a recording quota indicator showing the current
entry count versus the `max_entries` limit (e.g. `1,234/10,000`). The
indicator turns amber when usage exceeds 80% of the limit. Hovering over
the indicator shows the total recording size versus the size limit (when
configured).

The quota indicator polls `GET /api/capture/stats` every 5 seconds.

## API Usage

### Get Current Stats

```bash
curl http://127.0.0.1:3001/api/capture/stats
```

Response:
```json
{
  "entry_count": 1234,
  "max_entries": 10000,
  "total_size_bytes": 52428800,
  "max_total_size_bytes": 0,
  "max_body_size": 20971520,
  "capture_enabled": true,
  "capture_request_bodies": true,
  "capture_response_bodies": true,
  "ignored_domains": []
}
```

### Update Limits

```bash
# Set max entries to 5,000 and total size to 500 MB
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"max_requests": 5000, "max_total_size_mb": 500}'

# Disable response body capture
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"capture_response_bodies": false}'

# Add ignored domains
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"ignored_domains": ["*.google-analytics.com", "telemetry.example.com"]}'

# Disable total size limit (set to null)
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"max_total_size_mb": null}'
```

### Get Full Config

```bash
curl http://127.0.0.1:3001/api/config
```

The response includes all recording limit fields.

## Web UI Configuration

1. Click the **Config** button in the header.
2. Navigate to the **Capture** tab.
3. Adjust the settings:
   - **Capture Request Bodies** — toggle on/off
   - **Capture Response Bodies** — toggle on/off
   - **Max Body Size** — slider (16 KB to 4096 KB)
   - **Max Total Recording Size** — input in MB (empty = unlimited)
   - **Ignored Domains** — textarea, one pattern per line
4. Click **Save Changes**.

Changes take effect immediately — no restart required.

## Config File

Recording limits can also be set in `~/.madhyamas/config.json`:

```json
{
  "max_requests": 5000,
  "max_body_size": 10485760,
  "max_total_size_mb": 500,
  "capture_request_bodies": true,
  "capture_response_bodies": true,
  "ignored_domains": ["*.google-analytics.com"]
}
```

Config file values are loaded at startup and persist across restarts.
Runtime changes made via the API or web UI are saved back to the config
file automatically.
