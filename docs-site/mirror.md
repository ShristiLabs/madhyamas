# Mirror

The Mirror tool saves **response bodies to disk** following the URL path structure, creating a browsable site mirror. This is equivalent to Charles Proxy's "Mirror" / "Save Responses" feature.

## How It Works

When enabled, the proxy writes each captured response body to disk as a file whose path mirrors the request URL:

```
URL: https://api.example.com/v1/users/123
→ output_dir/api.example.com/v1/users/123/index.json

URL: https://cdn.example.com/assets/img/logo.png
→ output_dir/cdn.example.com/assets/img/logo.png
```

A `.meta.json` sidecar is written alongside each body file containing request/response metadata (URL, method, status code, headers, timestamp, duration).

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
- Paths ending with `/` or having no file extension are saved as `index.html` (or `index.json` based on content-type).
- **Query strings** are stripped from the filename and stored in the metadata sidecar to keep filenames clean.
- Path components are **sanitized** — `..`, path separators, and null bytes are removed to prevent directory traversal.

## Host Filtering

The `host_filter` option restricts mirroring to specific hosts. Patterns support:

- **Exact hostname**: `api.example.com`
- **Wildcard subdomain**: `*.example.com` (matches `api.example.com`)
- **Glob**: `*api*` (matches any host containing `api`)

When `host_filter` is empty or `null`, all hosts are mirrored.

## Configuration

| Field | Default | Description |
|-------|---------|-------------|
| **Enabled** | `false` | Master switch |
| **Output Directory** | `~/.madhyamas/mirror` | Directory for mirrored files |
| **Host Filter** | (all hosts) | Host patterns to mirror (null = all) |
| **Save Request Bodies** | `false` | Also save request bodies as `.request` files |

### From the Web UI

Navigate to **Mirror** in the tools sidebar. Toggle the enable switch, set the output directory and host filter, and click **Save Config**. Statistics (files written, bytes written) update automatically every 5 seconds.

### From the CLI

```bash
madhyamas mirror status                                      # Show status and statistics
madhyamas mirror start                                       # Start mirroring
madhyamas mirror stop                                        # Stop mirroring
madhyamas mirror config --output-dir /tmp/mirror \
  --host-filter "*.example.com" --save-request-bodies       # Update configuration
madhyamas mirror config --host-filter none                  # Clear host filter (mirror all)
```

### From the REST API

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
  -d '{"output_dir": "/tmp/mirror", "host_filter": ["*.example.com", "api.test.com"]}'
```

## Behavior Notes

- **Async writes**: Mirror writes happen on background tasks to avoid blocking the proxy pipeline.
- **Passthrough traffic**: Passthrough (SSL-tunneled) entries have no captured body and are not mirrored.
- **Overwrite by default**: each new response for the same URL overwrites the previous file.
- **Truncated bodies**: if a response body was truncated (exceeds `max_body_size`), the `truncated` field in the metadata sidecar is set to `true`.
- **Request bodies**: when `save_request_bodies` is enabled, request bodies are saved as `<file>.request` alongside the response body.

## Disk Usage

Mirroring writes every captured response body to disk. For high-traffic proxies, this can consume significant disk space. Consider:

- Using `host_filter` to mirror only specific hosts.
- Periodically clearing the output directory.
- Monitoring the `bytes_written` statistic in the web UI.

## Common Use Cases

### Offline Browsing

Mirror a website's assets so you can browse them offline, with the directory structure matching the original URLs.

### Inspecting Response Bodies on Disk

Save response bodies to disk to inspect them with any file viewer, diff tool, or script — useful for binary assets (images, fonts) that are hard to read in the web UI.

### Building Mock Data

Capture API responses to disk and use them as mock data for local development or testing.

### Archiving

Capture a snapshot of an API's responses at a point in time for later comparison or compliance.
