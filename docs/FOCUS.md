# Focus Feature

The Focus feature lets you **highlight traffic from specific hosts** in the
traffic view. Unlike a filter (which hides non-matching traffic entirely),
focus visually emphasizes matching rows while keeping all traffic visible.

## Focus vs Filter

| Aspect | Filter | Focus |
|--------|--------|-------|
| Non-matching traffic | Hidden | Dimmed (but visible) |
| Matching traffic | Shown normally | Bold + yellow border + star icon |
| Use case | Narrow down to specific requests | Spot specific hosts in a busy stream |
| Persistence | Session-based | SQLite (`focus_hosts` table) — survives restarts |

## Two Modes

1. **Highlight mode** (default): All traffic is visible. Rows whose host
   matches a focus pattern are highlighted with a yellow left border, a star
   icon, and bold domain text. Non-matching rows are dimmed (50% opacity).

2. **Focus-only mode** (toggle in FocusPanel): Only traffic from focused
   hosts is shown. Non-matching rows are hidden entirely (like a filter).

## Pattern Syntax

Focus patterns are case-insensitive and support:

| Pattern | Matches | Example |
|---------|---------|---------|
| `api.example.com` | Exact host or any subdomain | `api.example.com`, `sub.api.example.com` |
| `*.example.com` | Wildcard subdomain (not the bare domain) | `api.example.com`, `sub.example.com` |
| `*api*` | Glob — any host containing `api` | `api.example.com`, `myapi.service.com` |
| `api.*` | Glob — starts with `api.` | `api.example.com`, `api.service.io` |

## Web UI

### FocusPanel

Click the **Focus** button in the traffic sub-toolbar to open the FocusPanel
sidebar. From there you can:

- **Add** a focus host pattern (type and press Enter or click +)
- **Remove** individual focus hosts (hover over a row and click ×)
- **Clear all** focus hosts (trash icon in the panel header)
- **Toggle "Show only focused"** to switch between highlight mode and
  focus-only mode

### Right-click "Focus this host"

Right-click any traffic row to instantly add its host as a focus pattern.
This is the quickest way to focus on a specific service you see in the list.

## CLI

```bash
# List all focus host patterns
madhyamas focus list

# Add a focus host pattern
madhyamas focus add api.example.com
madhyamas focus add "*.example.com"
madhyamas focus add "*api*"

# Remove a focus host by ID
madhyamas focus remove <id>

# Clear all focus hosts
madhyamas focus clear
```

## MCP

The following MCP tools are available for AI agent integration:

| Tool | Description |
|------|-------------|
| `madhyamas_list_focus_hosts` | List all focus host patterns |
| `madhyamas_add_focus_host` | Add a focus host pattern (`pattern` parameter) |
| `madhyamas_remove_focus_host` | Remove a focus host by ID (`id` parameter) |
| `madhyamas_clear_focus_hosts` | Clear all focus hosts |

Example:

```json
{
  "tool": "madhyamas_add_focus_host",
  "arguments": { "pattern": "*.example.com" }
}
```

## API

All endpoints are under the `/api` prefix.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/focus` | List all focus hosts |
| `POST` | `/focus` | Add a focus host (`{ "pattern": "..." }`) |
| `DELETE` | `/focus/{id}` | Remove a focus host by ID |
| `DELETE` | `/focus` | Clear all focus hosts |

### Examples

```bash
# List focus hosts
curl http://127.0.0.1:3001/api/focus

# Add a focus host
curl -X POST http://127.0.0.1:3001/api/focus \
  -H "Content-Type: application/json" \
  -d '{"pattern": "*.example.com"}'

# Remove a focus host
curl -X DELETE http://127.0.0.1:3001/api/focus/<id>

# Clear all
curl -X DELETE http://127.0.0.1:3001/api/focus
```

### Host filtering in TrafficQuery

The `GET /traffic` endpoint now supports a `host` query parameter for
substring-based host filtering (independent of focus):

```bash
curl "http://127.0.0.1:3001/api/traffic?host=example.com&limit=50"
```

## Persistence

Focus hosts are stored in the `focus_hosts` SQLite table alongside the
traffic database (`~/.madhyamas/traffic.db`). They persist across restarts.
