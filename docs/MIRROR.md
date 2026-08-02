# Mirror Tool

The Mirror tool saves response bodies to disk following the URL path
structure, creating a browsable site mirror. This is the equivalent of
Charles Proxy's "Mirror" / "Save Responses" feature.

## Overview

When enabled, the proxy writes each captured response body to disk as a
file whose path mirrors the request URL:

```
URL: https://api.example.com/v1/users/123
→ output_dir/api.example.com/v1/users/123/index.json

URL: https://cdn.example.com/assets/img/logo.png
→ output_dir/cdn.example.com/assets/img/logo.png
```

A `.meta.json` sidecar is written alongside each body file containing
request/response metadata (URL, method, status code, headers, timestamp,
duration).

## Path Mapping Rules

| URL | Filesystem path |
|-----|-----------------|
| `https://example.com/` | `output_dir/example.com/index.html` |
| `https://example.com/page` | `output_dir/example.com/page/index.html` |
| `https://example.com/page.html` | `output_dir/example.com/page.html` |
| `https://api.example.com/v1/users/` | `output_dir/api.example.com/v1/users/index.json` |
| `https://cdn.example.com/assets/img/logo.png` | `output_dir/cdn.example.com/assets/img/logo.png` |

- **Host** becomes the top-level directory.
- **URL path** maps directly to a filesystem path.
- Paths ending with `/` or having no file extension are saved as
  `index.html` (or `index.json` based on content-type).
- **Query strings** are stripped from the filename and stored in the
  metadata sidecar to keep filenames clean.
- Path components are **sanitized** — `..`, path separators, and null
  bytes are removed to prevent directory traversal.

## Metadata Sidecar

Each mirrored response has a `.meta.json` sidecar:

```json
{
  "url": "https://api.example.com/v1/users/123",
  "method": "GET",
  "status_code": 200,
  "headers": { "content-type": "application/json" },
  "timestamp": "2026-08-01T12:00:00Z",
  "duration_ms": 145,
  "truncated": false
}
```

The `truncated` field indicates whether the response body was truncated
before mirroring (bodies larger than `max_body_size` are truncated).

## Host Filtering

The `host_filter` option restricts mirroring to specific hosts. Patterns
support:

- **Exact hostname**: `api.example.com`
- **Wildcard subdomain**: `*.example.com` (matches `api.example.com`)
- **Glob**: `*api*` (matches any host containing `api`)

When `host_filter` is empty or `null`, all hosts are mirrored.

## Configuration

### Config Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | `bool` | `false` | Master switch |
| `output_dir` | `string` | `~/.madhyamas/mirror` | Directory for mirrored files |
| `host_filter` | `string[] \| null` | `null` | Host patterns to mirror (null = all) |
| `save_request_bodies` | `bool` | `false` | Also save request bodies as `.request` files |

### Web UI

Navigate to **Mirror** in the tools sidebar (or the nav rail). Toggle the
enable switch, set the output directory and host filter, and click
**Save Config**. Statistics (files written, bytes written) update
automatically every 5 seconds.

### CLI

```bash
# Show mirror status and statistics
madhyamas mirror status

# Start mirroring
madhyamas mirror start

# Stop mirroring
madhyamas mirror stop

# Update configuration
madhyamas mirror config --output-dir /tmp/mirror --host-filter "*.example.com" --save-request-bodies

# Clear host filter (mirror all hosts)
madhyamas mirror config --host-filter none
```

### API

```bash
# Get mirror status
curl http://127.0.0.1:3001/api/mirror

# Toggle mirroring on
curl -X POST http://127.0.0.1:3001/api/mirror/toggle \
  -H "Content-Type: application/json" \
  -d '{"enabled": true}'

# Update configuration
curl -X PATCH http://127.0.0.1:3001/api/mirror/config \
  -H "Content-Type: application/json" \
  -d '{
    "output_dir": "/tmp/mirror",
    "host_filter": ["*.example.com", "api.test.com"],
    "save_request_bodies": true
  }'

# Clear host filter
curl -X PATCH http://127.0.0.1:3001/api/mirror/config \
  -H "Content-Type: application/json" \
  -d '{"host_filter": null}'
```

### MCP Tools

- `madhyamas_get_mirror_status` — Get current mirror status and statistics
- `madhyamas_toggle_mirror` — Toggle mirroring on/off
- `madhyamas_update_mirror_config` — Update mirror configuration

## Behavior

- **Async writes**: Mirror writes are performed on background tasks
  (`tokio::spawn`) to avoid blocking the proxy pipeline.
- **Passthrough traffic**: Passthrough (SSL-tunneled) entries have no
  captured body and are not mirrored.
- **Overwrite by default**: Each new response for the same URL overwrites
  the previous file.
- **Truncated bodies**: If a response body was truncated (exceeds
  `max_body_size`), the `truncated` field in the metadata sidecar is set
  to `true`.
- **Request bodies**: When `save_request_bodies` is `true`, request bodies
  are saved as `<file>.request` alongside the response body.

## Use Cases

- **Offline browsing**: Mirror a website's assets for offline access.
- **Debugging**: Inspect response bodies on disk with any file viewer.
- **Archiving**: Capture API responses for later analysis or comparison.
- **Testing**: Build a local copy of API responses for mock data.

## Disk Usage Considerations

Mirroring writes every captured response body to disk. For high-traffic
proxies, this can consume significant disk space. Consider:

- Using `host_filter` to mirror only specific hosts.
- Periodically clearing the output directory.
- Monitoring the `bytes_written` statistic in the web UI.
