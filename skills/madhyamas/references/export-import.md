# Export & Import

## Overview

Export captured traffic as HAR (HTTP Archive) or cURL commands. Export and import interception rules (mocks, rewrites, breakpoints, throttle). Persist rules to storage for survival across restarts.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_export_curl` | Export a request as cURL command |
| `madhyamas_export_session` | Export session as HAR/cURL |
| `madhyamas_import_session` | Import session from HAR |
| `madhyamas_export_mocks` | Export all mock rules as JSON |
| `madhyamas_import_mocks` | Import mocks (HAR/OpenAPI/Postman) |

## CLI Commands

```bash
madhyamas export har [--output <FILE>]
madhyamas export curl <ID>
madhyamas sessions export <ID> [--format <har|curl>]
madhyamas replay export <ID> [--format <curl|har>]
```

## REST API

### Traffic Export

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/export/har` | Export all traffic as HAR |
| GET | `/api/export/curl/{id}` | Export request as cURL |

### Session Export/Import

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/sessions/{id}/export` | Export session |
| POST | `/api/sessions/import` | Import session |

### Rule Persistence

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/persistence/export` | Export all rules as JSON |
| POST | `/api/persistence/import` | Import all rules from JSON |
| POST | `/api/persistence/save` | Save rules to persistent store |
| POST | `/api/persistence/load` | Load rules from persistent store |

### Mock Import/Export

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/mocks/export` | Export all mocks as JSON |
| POST | `/api/mocks/import` | Import mocks (HAR/OpenAPI/Postman) |

## Workflows

### Export All Traffic as HAR

**CLI:** `madhyamas export har --output traffic.har`

**REST:** `curl http://localhost:3001/api/export/har -o traffic.har`

### Export a Single Request as cURL

**MCP:** `madhyamas_export_curl(id="abc123")`

**CLI:** `madhyamas export curl abc123`

**REST:** `curl http://localhost:3001/api/export/curl/abc123`

### Export a Session

**MCP:** `madhyamas_export_session(id="abc123", format="har")`

**CLI:** `madhyamas sessions export abc123 --format har`

**REST:** `curl http://localhost:3001/api/sessions/abc123/export?format=har -o session.har`

### Import a Session

**MCP:** `madhyamas_import_session(session_data={...})`

**REST:**
```bash
curl -X POST http://localhost:3001/api/sessions/import \
  -H 'Content-Type: application/json' \
  -d @exported-session.json
```

### Export All Interception Rules

Export mocks, rewrites, breakpoints, and throttle settings as a single JSON:

**REST:** `curl http://localhost:3001/api/persistence/export -o rules.json`

### Import All Interception Rules

**REST:**
```bash
curl -X POST http://localhost:3001/api/persistence/import \
  -H 'Content-Type: application/json' \
  -d @rules.json
```

### Save Rules to Persistent Store

Save rules so they survive proxy restarts:

**REST:**
```bash
curl -X POST http://localhost:3001/api/persistence/save \
  -H 'X-Madhyamas-Confirm: true'
```

> The `X-Madhyamas-Confirm` header is required for CSRF protection.

### Load Rules from Persistent Store

**REST:** `curl -X POST http://localhost:3001/api/persistence/load`

### Export Mocks

**MCP:** `madhyamas_export_mocks()`

**REST:** `curl http://localhost:3001/api/mocks/export -o mocks.json`

### Import Mocks from HAR

**MCP:** `madhyamas_import_mocks(format="har", data="<har-json>")`

**REST:**
```bash
curl -X POST http://localhost:3001/api/mocks/import \
  -H 'Content-Type: application/json' \
  -d '{"format":"har","data":"<har-json-string>"}'
```

### Import Mocks from OpenAPI

**MCP:** `madhyamas_import_mocks(format="openapi", data="<openapi-json>")`

### Import Mocks from Postman

**MCP:** `madhyamas_import_mocks(format="postman", data="<postman-json>")`

## Use Cases

- **Share debugging sessions**: Export as HAR, share with team
- **Reproduce issues**: Export cURL command, run in terminal
- **Backup rules**: Export all rules before updating proxy
- **Migrate configs**: Export from one instance, import to another
- **Generate mocks from specs**: Import OpenAPI spec to create mock rules
- **Persist across restarts**: Save rules to persistent store
