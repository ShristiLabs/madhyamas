# Sessions

## Overview

Sessions organize captured traffic into named groups. The proxy always has one active session. New traffic is recorded to the active session. Switch between sessions to focus on different debugging contexts.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_list_sessions` | List all sessions |
| `madhyamas_create_session` | Create a new session |
| `madhyamas_switch_session` | Switch active session |
| `madhyamas_export_session` | Export session (HAR/cURL) |
| `madhyamas_import_session` | Import session from export |

## CLI Commands

```bash
madhyamas sessions list
madhyamas sessions create [--name <NAME>] [--description <DESC>]
madhyamas sessions delete <ID>
madhyamas sessions switch <ID>
madhyamas sessions export <ID> [--format <har|curl>]
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/sessions` | List all sessions |
| POST | `/api/sessions` | Create session |
| GET | `/api/sessions/{id}` | Get session details |
| DELETE | `/api/sessions/{id}` | Delete session |
| GET | `/api/sessions/{id}/export` | Export session |
| POST | `/api/sessions/{id}/switch` | Switch active session |
| POST | `/api/sessions/import` | Import session |

## Workflows

### List All Sessions

**MCP:** `madhyamas_list_sessions()`

**CLI:** `madhyamas sessions list`

**REST:** `curl http://localhost:3001/api/sessions`

### Create a New Session

**MCP:** `madhyamas_create_session(name="debug-auth", description="Authentication debugging")`

**CLI:** `madhyamas sessions create --name "debug-auth" --description "Authentication debugging"`

**REST:**
```bash
curl -X POST http://localhost:3001/api/sessions \
  -H 'Content-Type: application/json' \
  -d '{"name":"debug-auth","description":"Authentication debugging"}'
```

### Switch Active Session

Switching changes where new traffic is recorded:

**MCP:** `madhyamas_switch_session(id="abc123")`

**CLI:** `madhyamas sessions switch abc123`

**REST:** `curl -X POST http://localhost:3001/api/sessions/abc123/switch`

### Export a Session

Export all traffic in a session as HAR for sharing:

**MCP:** `madhyamas_export_session(id="abc123", format="har")`

**CLI:** `madhyamas sessions export abc123 --format har`

**REST:** `curl http://localhost:3001/api/sessions/abc123/export?format=har -o session.har`

### Import a Session

Import a previously exported session:

**MCP:** `madhyamas_import_session(session_data={...})`

**REST:**
```bash
curl -X POST http://localhost:3001/api/sessions/import \
  -H 'Content-Type: application/json' \
  -d @exported-session.json
```

### Delete a Session

**CLI:** `madhyamas sessions delete abc123`

**REST:** `curl -X DELETE http://localhost:3001/api/sessions/abc123`

## Use Cases

- **Separate debugging contexts**: Create a session per feature or bug being investigated
- **Save and share**: Export a session as HAR to share with team members
- **Compare scenarios**: Switch between sessions to compare traffic from different test runs
- **Archive**: Export sessions before clearing traffic, import later for reference

## Session Metadata

Each session contains:
- `id` — unique identifier
- `name` — session name
- `description` — optional description
- `created_at` — creation timestamp
- `updated_at` — last update timestamp
- `request_count` — number of captured requests

A "Default Session" is auto-created on first run.
