# Replay

## Overview

Replay captured requests to re-execute them, optionally with modifications. Save frequently used requests for repeated testing. View replay history to track past executions.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_replay_request` | Replay a captured request |
| `madhyamas_save_request` | Save a request for later replay |
| `madhyamas_list_saved_requests` | List all saved requests |
| `madhyamas_export_curl` | Export a request as cURL |

## CLI Commands

```bash
madhyamas replay run <ID>
madhyamas replay save <TRAFFIC_ID> [--name <NAME>] [--description <DESC>]
madhyamas replay list
madhyamas replay delete <ID>
madhyamas replay export <ID> [--format <curl|har>]
madhyamas replay history
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/replay/saved` | List saved requests |
| POST | `/api/replay/saved` | Save a request |
| GET | `/api/replay/saved/{id}` | Get saved request |
| DELETE | `/api/replay/saved/{id}` | Delete saved request |
| POST | `/api/replay/execute/{id}` | Execute replay |
| GET | `/api/replay/history` | Get replay history |
| DELETE | `/api/replay/history` | Clear replay history |

## Workflows

### Replay a Captured Request

Re-execute a previously captured traffic entry:

**MCP:** `madhyamas_replay_request(id="abc123")`

**CLI:** `madhyamas replay run abc123`

**REST:** `curl -X POST http://localhost:3001/api/replay/execute/abc123`

### Replay with Modifications

Modify headers, body, or URL before replaying:

**MCP:**
```
madhyamas_replay_request(
  id="abc123",
  modifications={
    "headers": {"Authorization": "Bearer new-token"},
    "body": "{\"updated\":true}"
  }
)
```

**REST:**
```bash
curl -X POST http://localhost:3001/api/replay/execute/abc123 \
  -H 'Content-Type: application/json' \
  -d '{"modifications":{"headers":{"Authorization":"Bearer new-token"},"body":"{\"updated\":true}"}}'
```

### Save a Request for Later Replay

**MCP:** `madhyamas_save_request(traffic_id="abc123", name="Login Request")`

**CLI:** `madhyamas replay save abc123 --name "Login Request"`

**REST:**
```bash
curl -X POST http://localhost:3001/api/replay/saved \
  -H 'Content-Type: application/json' \
  -d '{"traffic_id":"abc123","name":"Login Request"}'
```

### List Saved Requests

**MCP:** `madhyamas_list_saved_requests()`

**CLI:** `madhyamas replay list`

**REST:** `curl http://localhost:3001/api/replay/saved`

### Delete a Saved Request

**CLI:** `madhyamas replay delete abc123`

**REST:** `curl -X DELETE http://localhost:3001/api/replay/saved/abc123`

### View Replay History

**CLI:** `madhyamas replay history`

**REST:** `curl http://localhost:3001/api/replay/history`

### Export as cURL

Export a captured request as a cURL command for use in terminal:

**MCP:** `madhyamas_export_curl(id="abc123")`

**CLI:** `madhyamas export curl abc123`

**REST:** `curl http://localhost:3001/api/export/curl/abc123`

### Export as HAR

**CLI:** `madhyamas replay export abc123 --format har`

## Use Cases

- **Test different auth tokens**: Replay a request with modified Authorization header
- **Reproduce bugs**: Save a failing request, fix the server, replay to verify
- **Load testing**: Replay the same request multiple times
- **API exploration**: Export as cURL to share with team members
- **Regression testing**: Save key requests and replay after server changes
