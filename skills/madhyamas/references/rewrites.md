# Rewrites

## Overview

Rewrite rules modify URLs, headers, query parameters, or bodies of matching requests/responses. Rewrites run first in the interception pipeline (priority 10), before mocks and breakpoints.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_list_rewrites` | List all rewrite rules |
| `madhyamas_create_rewrite` | Create a rewrite rule |
| `madhyamas_delete_rewrite` | Delete a rewrite rule |
| `madhyamas_toggle_rewrite` | Enable/disable a rule |
| `madhyamas_get_rewrite_templates` | Get predefined templates |

## CLI Commands

```bash
madhyamas rewrites list
madhyamas rewrites create --name <NAME> --pattern <PATTERN> --action <ACTION>
madhyamas rewrites delete <ID>
madhyamas rewrites toggle <ID>
madhyamas rewrites templates
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/rewrites` | List all rewrite rules |
| POST | `/api/rewrites` | Create rewrite rule |
| GET | `/api/rewrites/templates` | Get predefined templates |
| GET | `/api/rewrites/{id}` | Get specific rule |
| DELETE | `/api/rewrites/{id}` | Delete rule |
| POST | `/api/rewrites/{id}/toggle` | Toggle rule |
| POST | `/api/rewrites/batch-toggle` | Toggle multiple rules |

## Workflows

### Create a URL Rewrite

Redirect requests from one URL to another:

**MCP:**
```
madhyamas_create_rewrite(
  name="Redirect API",
  condition={"type":"url_pattern","pattern":"*/api/v1/*"},
  direction="request",
  rewrites=[{"type":"url_rewrite","pattern":"api/v1","replacement":"api/v2"}]
)
```

**CLI:** `madhyamas rewrites create --name "Redirect API" --pattern "*/api/v1/*" --action "api/v2"`

**REST:**
```bash
curl -X POST http://localhost:3001/api/rewrites \
  -H 'Content-Type: application/json' \
  -d '{"name":"Redirect API","condition":{"type":"url_pattern","pattern":"*/api/v1/*"},"direction":"request","rewrites":[{"type":"url_rewrite","pattern":"api/v1","replacement":"api/v2"}]}'
```

### Add a Custom Header

**MCP:**
```
madhyamas_create_rewrite(
  name="Add Auth Header",
  condition={"type":"url_pattern","pattern":"*/api/*"},
  direction="request",
  rewrites=[{"type":"set_header","name":"X-Custom-Auth","value":"my-token"}]
)
```

### Remove a Header

**MCP:**
```
madhyamas_create_rewrite(
  name="Remove Security Headers",
  condition={"type":"url_pattern","pattern":"*"},
  direction="response",
  rewrites=[{"type":"remove_header","name":"X-Frame-Options"}]
)
```

### Modify Response Body

**MCP:**
```
madhyamas_create_rewrite(
  name="Modify Response",
  condition={"type":"url_pattern","pattern":"*/api/data*"},
  direction="response",
  rewrites=[{"type":"body_rewrite","pattern":"old_value","replacement":"new_value"}]
)
```

### Apply to Both Request and Response

**MCP:**
```
madhyamas_create_rewrite(
  name="Log All",
  condition={"type":"url_pattern","pattern":"*"},
  direction="both",
  rewrites=[{"type":"set_header","name":"X-Debug","value":"true"}]
)
```

### Use Predefined Templates

**MCP:** `madhyamas_get_rewrite_templates()`

**CLI:** `madhyamas rewrites templates`

**REST:** `curl http://localhost:3001/api/rewrites/templates`

Available templates: Add CORS, HTTP to HTTPS, Add Auth, Remove Security Headers.

### List All Rewrites

**MCP:** `madhyamas_list_rewrites()`

**CLI:** `madhyamas rewrites list`

**REST:** `curl http://localhost:3001/api/rewrites`

### Toggle a Rewrite

**MCP:** `madhyamas_toggle_rewrite(id="abc123", enabled=false)`

**CLI:** `madhyamas rewrites toggle abc123`

**REST:** `curl -X POST http://localhost:3001/api/rewrites/abc123/toggle -d '{"enabled":false}'`

### Delete a Rewrite

**MCP:** `madhyamas_delete_rewrite(id="abc123")`

**CLI:** `madhyamas rewrites delete abc123`

**REST:** `curl -X DELETE http://localhost:3001/api/rewrites/abc123`

## Rewrite Action Types

| Action Type | Parameters | Description |
|-------------|-----------|-------------|
| `url_rewrite` | `pattern`, `replacement` | Regex-based URL transformation |
| `set_header` | `name`, `value` | Set or add a header |
| `remove_header` | `name` | Remove a header |
| `replace_header` | `name`, `pattern`, `replacement` | Regex-replace header value |
| `set_query_param` | `name`, `value` | Set a query parameter |
| `remove_query_param` | `name` | Remove a query parameter |
| `body_rewrite` | `pattern`, `replacement` | Regex-based body transformation |
| `file_map` | `path` | Map response to local file |
| `url_map` | `url` | Redirect to different URL |

## Interception Pipeline Order

Rewrites run first (priority 10):
1. **Rewrites (priority 10)**
2. Mocks (priority 20)
3. Breakpoints (priority 30)
4. Throttle (priority 40)
