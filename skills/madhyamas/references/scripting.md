# Scripting

## Overview

Create JavaScript/TypeScript scripts that run on request/response hooks to automate traffic manipulation. Scripts can log, modify headers, block domains, add CORS, and mock APIs. This is an experimental feature.

> **Note:** The JavaScript engine integration is a placeholder. Script execution, sandboxing, timeout enforcement, and network/filesystem access controls are not yet fully implemented.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `madhyamas_list_scripts` | List all scripts |
| `madhyamas_create_script` | Create a script |
| `madhyamas_get_script` | Get script details |
| `madhyamas_update_script` | Update a script |
| `madhyamas_delete_script` | Delete a script |
| `madhyamas_toggle_script` | Enable/disable a script |
| `madhyamas_get_script_templates` | Get predefined templates |

## CLI Commands

```bash
madhyamas scripts list
madhyamas scripts create --name <NAME> --hook <HOOK> [--file <PATH> | --inline <CODE>]
madhyamas scripts get <ID>
madhyamas scripts delete <ID>
madhyamas scripts toggle <ID>
madhyamas scripts templates
```

## REST API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/scripts` | List all scripts |
| POST | `/api/scripts` | Create script |
| GET | `/api/scripts/templates` | Get templates |
| GET | `/api/scripts/config` | Get runtime config |
| GET | `/api/scripts/{id}` | Get script |
| PUT | `/api/scripts/{id}` | Update script |
| DELETE | `/api/scripts/{id}` | Delete script |
| POST | `/api/scripts/{id}/toggle` | Toggle script |

## Workflows

### Create a Script

**MCP:**
```
madhyamas_create_script(
  name="Log All Requests",
  source="console.log(request.url, request.method)",
  hook="on_request",
  enabled=true
)
```

**CLI:** `madhyamas scripts create --name "Log All Requests" --hook request --inline "console.log(request.url)"`

**REST:**
```bash
curl -X POST http://localhost:3001/api/scripts \
  -H 'Content-Type: application/json' \
  -d '{"name":"Log All Requests","source":"console.log(request.url)","hooks":["on_request"]}'
```

### Create a Script from File

**CLI:** `madhyamas scripts create --name "Custom Script" --hook response --file ./my-script.js`

### List All Scripts

**MCP:** `madhyamas_list_scripts()`

**CLI:** `madhyamas scripts list`

**REST:** `curl http://localhost:3001/api/scripts`

### Toggle a Script

**MCP:** `madhyamas_toggle_script(id="abc123", enabled=false)`

**CLI:** `madhyamas scripts toggle abc123`

**REST:** `curl -X POST http://localhost:3001/api/scripts/abc123/toggle -d '{"enabled":false}'`

### Get Script Templates

**MCP:** `madhyamas_get_script_templates()`

**CLI:** `madhyamas scripts templates`

**REST:** `curl http://localhost:3001/api/scripts/templates`

Available templates: Log Requests, Add CORS, Block Domains, Modify Headers, Mock API.

### Delete a Script

**MCP:** `madhyamas_delete_script(id="abc123")`

**CLI:** `madhyamas scripts delete abc123`

**REST:** `curl -X DELETE http://localhost:3001/api/scripts/abc123`

## Script Hooks

| Hook | When it Runs | Available Data |
|------|-------------|----------------|
| `on_request` | Before forwarding request to server | `request` (url, method, headers, body) |
| `on_response` | After receiving response from server | `request`, `response` (status, headers, body) |

## Script Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `timeout_ms` | 5000 | Execution timeout |
| `max_memory_bytes` | 10MB | Memory limit |
| `enable_console` | true | Allow console.log |
| `allow_network` | false | Network access (not yet enforced) |
| `allow_fs` | false | Filesystem access (not yet enforced) |

## Limitations

- No JS engine integrated yet (placeholder for `boa_engine`)
- Network and filesystem access not enforced
- Execution timeouts not enforced
- Scripts run in priority order; lower priority value runs first
