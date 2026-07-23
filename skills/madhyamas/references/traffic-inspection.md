# Traffic Inspection

## Overview

Capture, filter, search, and analyze HTTP/HTTPS traffic flowing through the proxy. Traffic is stored in SQLite and broadcast in real-time via WebSocket.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_get_traffic` | List traffic with advanced filtering |
| `madhyamas_get_traffic_entry` | Get full details of a specific entry |
| `madhyamas_search_traffic` | Search traffic by content |
| `madhyamas_get_traffic_count` | Get total count |
| `madhyamas_clear_traffic` | Clear all traffic |

## CLI Commands

```bash
madhyamas traffic list [OPTIONS]
madhyamas traffic get <ID>
madhyamas traffic search <QUERY>
madhyamas traffic count
madhyamas traffic clear
```

## REST API

| Method | Path |
|--------|------|
| GET | `/api/traffic` |
| GET | `/api/traffic/{id}` |
| POST | `/api/traffic/clear` |
| GET | `/api/traffic/count` |

## Workflows

### List All Traffic

**MCP:** `madhyamas_get_traffic()`

**CLI:** `madhyamas traffic list`

**REST:** `curl http://localhost:3001/api/traffic`

### Filter by Method and Status

**MCP:** `madhyamas_get_traffic(method="POST", status=500)`

**CLI:** `madhyamas traffic list --method POST --status 500`

**REST:** `curl 'http://localhost:3001/api/traffic?method=POST&status_code=500'`

### Filter by URL Pattern

**MCP:** `madhyamas_get_traffic(filter="*/api.example.com/*")`

**CLI:** `madhyamas traffic list --filter "*/api.example.com/*"`

### Filter by Response Time (Slow Requests)

**MCP:** `madhyamas_get_traffic(min_time=2000)` — requests slower than 2 seconds

### Filter by Response Size

**MCP:** `madhyamas_get_traffic(min_size=1000000)` — responses larger than 1MB

### Filter by Header

**MCP:** `madhyamas_get_traffic(header="Authorization:Bearer")` — entries with matching header

### Search Traffic Content

**MCP:** `madhyamas_search_traffic(query="user_id=12345")`

**CLI:** `madhyamas traffic search "user_id=12345"`

**REST:** `curl 'http://localhost:3001/api/traffic?search=user_id%3D12345'`

### Get Full Request/Response Details

**MCP:** `madhyamas_get_traffic_entry(id="abc123")`

**CLI:** `madhyamas traffic get abc123 --json`

**REST:** `curl http://localhost:3001/api/traffic/abc123`

### Get Traffic Count

**MCP:** `madhyamas_get_traffic_count()`

**CLI:** `madhyamas traffic count`

**REST:** `curl http://localhost:3001/api/traffic/count`

### Clear All Traffic

**MCP:** `madhyamas_clear_traffic()`

**CLI:** `madhyamas traffic clear`

**REST:** `curl -X POST http://localhost:3001/api/traffic/clear`

### Paginate Results

**MCP:** `madhyamas_get_traffic(limit=50, offset=100)` — page 3 with 50 per page

**REST:** `curl 'http://localhost:3001/api/traffic?limit=50&offset=100'`

## Analysis Patterns

### Find All Failed Requests

```
madhyamas_get_traffic(status=500)
madhyamas_get_traffic(status=502)
madhyamas_get_traffic(status=503)
```

Or via CLI: `madhyamas traffic list --status 500`

### Find Slow API Calls

```
madhyamas_get_traffic(filter="*/api/*", min_time=1000)
```

### Find Large Responses

```
madhyamas_get_traffic(max_size=100, file_type="json")  — small JSON responses
madhyamas_get_traffic(min_size=1000000)                 — large responses (>1MB)
```

### Export a Request as cURL

**MCP:** `madhyamas_export_curl(id="abc123")`

**CLI:** `madhyamas export curl abc123`

**REST:** `curl http://localhost:3001/api/export/curl/abc123`

## Capture Mode Control

Toggle between recording (default) and passthrough mode:

**MCP:** `madhyamas_toggle_capture()`

**CLI:** `madhyamas capture toggle`

**REST:** `curl -X POST http://localhost:3001/api/capture/toggle`

In passthrough mode, the proxy forwards traffic but does not record it to the database. Useful for reducing memory usage when not actively debugging.
