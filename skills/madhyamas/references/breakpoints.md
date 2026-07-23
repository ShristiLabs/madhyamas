# Breakpoints

## Overview

Breakpoints pause traffic matching a pattern, allowing inspection and modification before forwarding. When a breakpoint triggers, the traffic entry is held in a paused state until resumed with a decision (Allow, Modify, or Reject).

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_list_breakpoints` | List all breakpoint rules |
| `madhyamas_create_breakpoint` | Create a breakpoint rule |
| `madhyamas_delete_breakpoint` | Delete a breakpoint rule |

## CLI Commands

```bash
madhyamas breakpoints list
madhyamas breakpoints create --url-pattern <PATTERN> [--method <M>] [--direction <DIR>]
madhyamas breakpoints delete <ID>
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/breakpoints` | List all breakpoint rules |
| POST | `/api/breakpoints` | Create breakpoint rule |
| GET | `/api/breakpoints/{id}` | Get specific rule |
| DELETE | `/api/breakpoints/{id}` | Delete rule |
| GET | `/api/breakpoints/paused` | Get all paused traffic |
| GET | `/api/breakpoints/paused/{id}` | Get specific paused item |
| POST | `/api/breakpoints/paused/{id}/resume` | Resume with decision |

## Workflows

### Create a Request Breakpoint

Pause requests matching a URL pattern before they reach the server:

**MCP:** `madhyamas_create_breakpoint(url_pattern="*/api/auth*", direction="request")`

**CLI:** `madhyamas breakpoints create --url-pattern "*/api/auth*" --direction request`

**REST:**
```bash
curl -X POST http://localhost:3001/api/breakpoints \
  -H 'Content-Type: application/json' \
  -d '{"url_pattern":"*/api/auth*","direction":"request"}'
```

### Create a Response Breakpoint

Pause responses before they reach the client:

**MCP:** `madhyamas_create_breakpoint(url_pattern="*/api/data*", direction="response")`

**CLI:** `madhyamas breakpoints create --url-pattern "*/api/data*" --direction response`

### Create a Bidirectional Breakpoint

Pause both request and response:

**MCP:** `madhyamas_create_breakpoint(url_pattern="*/api/critical*", direction="both")`

### Create a Method-Specific Breakpoint

**MCP:** `madhyamas_create_breakpoint(url_pattern="*/api/users*", method="POST", direction="request")`

**CLI:** `madhyamas breakpoints create --url-pattern "*/api/users*" --method POST --direction request`

### List All Breakpoints

**MCP:** `madhyamas_list_breakpoints()`

**CLI:** `madhyamas breakpoints list`

**REST:** `curl http://localhost:3001/api/breakpoints`

### View Paused Traffic

When a breakpoint triggers, traffic is paused. View paused items:

**REST:** `curl http://localhost:3001/api/breakpoints/paused`

**Get specific paused item:** `curl http://localhost:3001/api/breakpoints/paused/{id}`

### Resume Paused Traffic

Resume a paused item with a decision:

**Allow** — forward without modification:
```bash
curl -X POST http://localhost:3001/api/breakpoints/paused/{id}/resume \
  -H 'Content-Type: application/json' \
  -d '{"decision":"allow"}'
```

**Modify** — forward with changes:
```bash
curl -X POST http://localhost:3001/api/breakpoints/paused/{id}/resume \
  -H 'Content-Type: application/json' \
  -d '{"decision":"modify","modifications":{"headers":{"Authorization":"Bearer newtoken"},"body":"{\"modified\":true}"}}'
```

**Reject** — abort the request:
```bash
curl -X POST http://localhost:3001/api/breakpoints/paused/{id}/resume \
  -H 'Content-Type: application/json' \
  -d '{"decision":"reject"}'
```

### Delete a Breakpoint

**MCP:** `madhyamas_delete_breakpoint(id="abc123")`

**CLI:** `madhyamas breakpoints delete abc123`

**REST:** `curl -X DELETE http://localhost:3001/api/breakpoints/abc123`

## Interception Pipeline Order

Breakpoints run at priority 30 in the interception pipeline:

1. Rewrites (priority 10)
2. Mocks (priority 20)
3. **Breakpoints (priority 30)**
4. Throttle (priority 40)

If a mock matches and responds, the breakpoint is not triggered. Breakpoints only fire for traffic that passes through mocks (i.e., no mock matched).

## Use Cases

- **Debug authentication**: Break on `*/auth*` requests to inspect tokens
- **Modify test data**: Break on POST requests to modify request bodies
- **Inject errors**: Break on responses and modify status codes to test error handling
- **Inspect headers**: Break to examine request/response headers before forwarding
