# Mocking

## Overview

Mock rules intercept requests matching a pattern and return custom responses instead of forwarding to the real server. Supports simple mocks, advanced mocks (sequences, conditional, probabilistic), collections, recording, import/export, analytics, and versioning.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_list_mocks` | List all mock rules |
| `madhyamas_create_mock` | Create a simple mock |
| `madhyamas_create_advanced_mock` | Create advanced mock (sequence/conditional/probabilistic) |
| `madhyamas_update_mock` | Update a mock rule |
| `madhyamas_get_mock` | Get mock details |
| `madhyamas_delete_mock` | Delete a mock rule |
| `madhyamas_toggle_mock` | Enable/disable a mock |
| `madhyamas_duplicate_mock` | Duplicate a mock |
| `madhyamas_rollback_mock` | Rollback to previous version |
| `madhyamas_get_mock_versions` | Get version history |
| `madhyamas_test_mock` | Test mock against sample request |
| `madhyamas_preview_mock_match` | Preview which mock matches a request |
| `madhyamas_export_mocks` | Export all mocks as JSON |
| `madhyamas_import_mocks` | Import mocks (HAR/OpenAPI/Postman) |
| `madhyamas_set_mock_recording` | Enable/disable recording mode |
| `madhyamas_get_mock_recording_status` | Get recording status |
| `madhyamas_get_recorded_mocks` | Get recorded mocks |
| `madhyamas_promote_recorded_mocks` | Promote recorded to active rules |

**Collections:** `madhyamas_list_mock_collections`, `madhyamas_create_mock_collection`, `madhyamas_delete_mock_collection`, `madhyamas_toggle_mock_collection`

**Analytics:** `madhyamas_get_mock_analytics`, `madhyamas_get_mock_hit_history`

## CLI Commands

```bash
madhyamas mocks list
madhyamas mocks create --url-pattern <PATTERN> [--method <M>] [--status-code <C>] [--body <B>] [--delay-ms <D>]
madhyamas mocks delete <ID>
madhyamas mocks toggle <ID> <true|false>
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/mocks` | List all mocks |
| POST | `/api/mocks` | Create mock |
| GET | `/api/mocks/{id}` | Get mock |
| PUT | `/api/mocks/{id}` | Update mock |
| DELETE | `/api/mocks/{id}` | Delete mock |
| POST | `/api/mocks/{id}/toggle` | Toggle mock |
| POST | `/api/mocks/advanced` | Create advanced mock |
| POST | `/api/mocks/{id}/duplicate` | Duplicate mock |
| POST | `/api/mocks/{id}/rollback` | Rollback mock |
| GET | `/api/mocks/{id}/versions` | Version history |
| POST | `/api/mocks/{id}/test` | Test mock |
| POST | `/api/mocks/preview` | Preview match |
| GET | `/api/mocks/export` | Export all |
| POST | `/api/mocks/import` | Import |
| GET/POST | `/api/mocks/collections` | Collection management |
| GET/POST | `/api/mocks/analytics` | Analytics |
| POST | `/api/mocks/recording` | Recording control |

## Workflows

### Create a Simple Mock

**MCP:**
```
madhyamas_create_mock(
  url_pattern="*/api/auth*",
  status_code=200,
  body='{"token":"fake-jwt-token"}',
  headers={"Content-Type": "application/json"}
)
```

**CLI:** `madhyamas mocks create --url-pattern "*/api/auth*" --status-code 200 --body '{"token":"fake-jwt-token"}'`

**REST:**
```bash
curl -X POST http://localhost:3001/api/mocks \
  -H 'Content-Type: application/json' \
  -d '{"url_pattern":"*/api/auth*","status_code":200,"body":"{\"token\":\"fake\"}"}'
```

### Create a Mock with Delay (Slow Response)

**MCP:** `madhyamas_create_mock(url_pattern="*/api/slow*", status_code=200, delay_ms=5000)`

**CLI:** `madhyamas mocks create --url-pattern "*/api/slow*" --status-code 200 --delay-ms 5000`

### Create an Error Mock

**MCP:** `madhyamas_create_mock(url_pattern="*/api/error*", status_code=500, body='{"error":"internal"}')`

**CLI:** `madhyamas mocks create --url-pattern "*/api/error*" --status-code 500 --body '{"error":"internal"}'`

### Create an Advanced Mock (Response Sequence)

Return different responses on successive calls — useful for testing token refresh flows:

**MCP:**
```
madhyamas_create_advanced_mock(
  name="Auth Sequence",
  condition={"type":"url_pattern","pattern":"*/api/auth*"},
  response_config={
    "type":"sequence",
    "responses":[
      {"status_code":200,"body":"{\"token\":\"first\"}"},
      {"status_code":401,"body":"{\"error\":\"expired\"}"}
    ]
  }
)
```

**REST:**
```bash
curl -X POST http://localhost:3001/api/mocks/advanced \
  -H 'Content-Type: application/json' \
  -d '{"name":"Auth Sequence","condition":{"type":"url_pattern","pattern":"*/api/auth*"},"response_config":{"type":"sequence","responses":[{"status_code":200,"body":"{\"token\":\"first\"}"},{"status_code":401,"body":"{\"error\":\"expired\"}"}]}}'
```

### Create a Probabilistic Mock

Return 200 90% of the time, 500 10% of the time:

**MCP:**
```
madhyamas_create_advanced_mock(
  name="Flaky API",
  condition={"type":"url_pattern","pattern":"*/api/flaky*"},
  response_config={
    "type":"probabilistic",
    "responses":[
      {"weight":90,"response":{"status_code":200}},
      {"weight":10,"response":{"status_code":500}}
    ]
  }
)
```

### List All Mocks

**MCP:** `madhyamas_list_mocks()`

**CLI:** `madhyamas mocks list`

**REST:** `curl http://localhost:3001/api/mocks`

### Toggle a Mock

**MCP:** `madhyamas_toggle_mock(id="abc123", enabled=false)`

**CLI:** `madhyamas mocks toggle abc123 false`

**REST:** `curl -X POST http://localhost:3001/api/mocks/abc123/toggle -d '{"enabled":false}'`

### Test a Mock Against a Sample Request

**MCP:**
```
madhyamas_test_mock(id="abc123", request={"url":"https://api.example.com/auth","method":"GET","headers":{}})
```

### Preview Which Mock Would Match

**MCP:** `madhyamas_preview_mock_match(request={"url":"https://api.example.com/auth","method":"GET"})`

### Record Live Responses as Mocks

1. Enable recording: `madhyamas_set_mock_recording(enabled=true)`
2. Make real API calls through the proxy
3. Get recorded mocks: `madhyamas_get_recorded_mocks()`
4. Promote to active rules: `madhyamas_promote_recorded_mocks()`

### Organize Mocks into Collections

**MCP:**
```
madhyamas_create_mock_collection(name="Auth Mocks", description="All auth endpoint mocks")
```

Then create mocks with `collection_id` parameter, or toggle entire collection:
```
madhyamas_toggle_mock_collection(id="col123", enabled=false)
```

### Export and Import Mocks

**Export:**
```
madhyamas_export_mocks()
```

**Import from HAR:**
```
madhyamas_import_mocks(format="har", data="<har-json-string>")
```

**Import from OpenAPI:**
```
madhyamas_import_mocks(format="openapi", data="<openapi-json-string>")
```

### View Mock Analytics

**MCP:** `madhyamas_get_mock_analytics()` — hit counts for all mocks

**MCP:** `madhyamas_get_mock_hit_history(id="abc123")` — detailed history for one mock

### Version Management

**Get versions:** `madhyamas_get_mock_versions(id="abc123")`

**Rollback:** `madhyamas_rollback_mock(id="abc123", version=3)`

**Duplicate:** `madhyamas_duplicate_mock(id="abc123", new_name="Auth Mock v2")`
