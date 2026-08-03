# Recording Limits

Recording Limits let you **bound how much traffic Madhyamas records** to prevent unbounded memory and disk usage during long debugging sessions. When limits are exceeded, the oldest entries are automatically pruned (FIFO — first in, first out).

## Why Recording Limits Matter

Without limits, a long-running proxy session capturing a high-traffic app can accumulate gigabytes of data, leading to:

- Slow web UI performance (large database queries)
- High memory usage
- Disk space exhaustion
- Degraded proxy throughput

Recording limits keep Madhyamas responsive regardless of traffic volume.

## Configuration Options

All options can be set via the web UI (Config → Capture tab), the REST API, or the config file. Changes take effect immediately — no restart required.

| Option | Default | Description |
|--------|---------|-------------|
| **Max Requests** | 10,000 | Maximum number of traffic entries to keep. When exceeded, oldest entries are pruned. |
| **Max Total Size** | Unlimited | Maximum total size of all stored bodies in MB. When exceeded, oldest entries are pruned. Set to empty/null to disable. |
| **Max Body Size** | 20 MB | Maximum size of a single request or response body. Larger bodies are truncated before storage. |
| **Capture Request Bodies** | `true` | Whether to store request bodies. When off, request headers and metadata are still recorded. |
| **Capture Response Bodies** | `true` | Whether to store response bodies. When off, response headers and metadata are still recorded. |
| **Ignored Domains** | (empty) | Domains whose traffic should not be recorded at all. Supports exact match, suffix match, and wildcard patterns. |

## How Pruning Works

Pruning uses a **FIFO (first in, first out)** strategy — the oldest entries (by timestamp) are deleted first.

- **Entry count limit** (`max_requests`): checked on every insert. If the count exceeds the limit, the surplus oldest entries are pruned immediately. Set to `0` to disable.
- **Total size limit** (`max_total_size_mb`): checked periodically. If the total body size exceeds the limit, oldest entries are pruned one by one until under the limit. Set to `null` to disable.

When entries are pruned, the web UI removes them from the traffic list in real time — no manual refresh needed.

## Ignored Domains

The `ignored_domains` setting prevents traffic from specified hosts from being recorded at all. This is useful for filtering out noise from analytics, telemetry, or CDN domains.

Matching is case-insensitive and supports three patterns:

| Pattern | Example | Matches |
|---------|---------|---------|
| Exact hostname | `example.com` | `example.com` |
| Suffix match | `example.com` | `example.com`, `api.example.com`, `www.example.com` |
| Wildcard subdomain | `*.example.com` | `api.example.com`, `www.example.com` (not `example.com` itself) |

Example:

```
*.google-analytics.com
*.doubleclick.net
telemetry.example.com
```

## Recording Quota Indicator

The web UI header displays a recording quota indicator showing the current entry count versus the limit (e.g. `1,234/10,000`). The indicator turns amber when usage exceeds 80% of the limit. Hovering over it shows the total recording size versus the size limit (when configured).

## From the Web UI

1. Click the **Config** button in the header
2. Navigate to the **Capture** tab
3. Adjust the settings:
   - **Capture Request Bodies** — toggle on/off
   - **Capture Response Bodies** — toggle on/off
   - **Max Body Size** — slider
   - **Max Total Recording Size** — input in MB (empty = unlimited)
   - **Ignored Domains** — textarea, one pattern per line
4. Click **Save Changes**

## From the CLI / API

```bash
# View current stats
curl http://127.0.0.1:3001/api/capture/stats

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
```

## Common Use Cases

### Long-Running Captures

Set a `max_total_size_mb` so an overnight capture can't fill your disk — old entries are pruned automatically as new traffic arrives.

### Reducing Noise

Add analytics and telemetry domains to `ignored_domains` so they never appear in your traffic list, keeping it focused on the requests you actually care about.

### Saving Memory on Low-Power Machines

Lower `max_requests` and disable request body capture to reduce memory and disk usage on laptops or VMs with limited resources.

### Body-Only Debugging

If you only care about response bodies (e.g. debugging API payloads), disable request body capture to halve the storage overhead while keeping full response inspection.
