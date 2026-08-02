# Scripting API Reference

Complete reference for the Madhyamas scripting REST API, CLI commands, and MCP
tools.

## REST API

All endpoints are under the `/api` prefix.  The scripting endpoints are
conditionally compiled with the `scripting` feature flag.

### Scripts

#### `GET /scripts`

List all registered scripts.

**Response:** `200 OK`
```json
[
    {
        "id": "uuid-string",
        "name": "Log Requests",
        "description": "Logs method, URL, and headers for every request.",
        "source": "function onRequest(request, context) { ... }",
        "hooks": ["on_request"],
        "enabled": true,
        "priority": 100,
        "created_at": "2026-01-01T00:00:00Z",
        "modified_at": "2026-01-01T00:00:00Z"
    }
]
```

#### `POST /scripts`

Create a new script.

**Request:**
```json
{
    "name": "My Script",
    "source": "function onRequest(request, context) { return { continue: true }; }",
    "description": "Optional description",
    "hooks": ["on_request"]
}
```

**Response:** `201 Created`
```json
{
    "id": "uuid-string",
    "script": { ... }
}
```

#### `GET /scripts/{id}`

Get a single script by ID.

**Response:** `200 OK` with the script object, or `404 Not Found`.

#### `PUT /scripts/{id}`

Update a script's source code.

**Request:**
```json
{
    "source": "function onRequest(request, context) { ... }"
}
```

**Response:** `204 No Content` or `404 Not Found`.

#### `DELETE /scripts/{id}`

Delete a script.

**Response:** `204 No Content` or `404 Not Found`.

#### `POST /scripts/{id}/toggle`

Enable or disable a script.

**Request:**
```json
{
    "enabled": true
}
```

**Response:** `204 No Content` or `404 Not Found`.

### Templates & Config

#### `GET /scripts/templates`

List predefined script templates.

**Response:** `200 OK` — array of template objects (same shape as `Script`
but without `id`, `enabled`, `created_at`, `modified_at`).

#### `GET /scripts/config`

Get the current script runtime configuration.

**Response:** `200 OK`
```json
{
    "timeout_ms": 5000,
    "max_memory_bytes": 10485760,
    "enable_console": true,
    "allow_network": false,
    "allow_fs": false
}
```

#### `PUT /scripts/config`

Update the script runtime configuration.

**Request:** Same shape as the config response.

**Response:** `200 OK` with the updated config.

### Testing & Validation

#### `POST /scripts/test`

Dry-run a script against a sample context without affecting live traffic or
recording history.

**Request:**
```json
{
    "source": "function onRequest(request, context) { ... }",
    "hook": "on_request",
    "request": null,
    "response": null
}
```

If `request` or `response` are `null`, default sample objects are used.  You
can provide custom sample data:

```json
{
    "source": "...",
    "hook": "on_request",
    "request": {
        "method": "POST",
        "url": "https://api.example.com/login",
        "host": "api.example.com",
        "path": "/login",
        "headers": {},
        "body": "{\"username\":\"admin\"}",
        "content_type": "application/json",
        "query": {}
    }
}
```

**Response:** `200 OK`
```json
{
    "modified": true,
    "continue_": true,
    "response": null,
    "error": null,
    "console": ["Processing: POST /login"],
    "duration_ms": 3,
    "modified_request": { ... },
    "modified_response": null
}
```

If the script has an error:
```json
{
    "modified": false,
    "continue_": true,
    "response": null,
    "error": "Runtime error: TypeError: ...",
    "console": [],
    "duration_ms": 1,
    "modified_request": null,
    "modified_response": null
}
```

#### `POST /scripts/validate`

Validate a script's syntax without executing it.

**Request:**
```json
{
    "source": "function onRequest() { return { continue: true }; }"
}
```

**Response:** `200 OK`
```json
{
    "valid": true
}
```

Or if invalid:
```json
{
    "valid": false,
    "error": "Parse error: SyntaxError: unexpected token ..."
}
```

### History

#### `GET /scripts/history`

Get execution history for all scripts (most recent first).

**Query parameters:**
- `limit` (optional, default 100) — maximum entries to return

**Response:** `200 OK`
```json
[
    {
        "script_id": "uuid-string",
        "duration_ms": 3,
        "success": true,
        "error": null,
        "console": ["Processing: GET /api/users"],
        "timestamp": "2026-01-01T12:00:00Z"
    }
]
```

#### `GET /scripts/{id}/history`

Get execution history for a specific script.

**Query parameters:**
- `limit` (optional, default 50) — maximum entries to return

**Response:** `200 OK` with array of execution records, or `404 Not Found`.

#### `DELETE /scripts/{id}/history`

Clear execution history for a specific script.

**Response:** `204 No Content` or `404 Not Found`.

---

## CLI Commands

All script commands are under `madhyamas scripts`:

```
madhyamas scripts <subcommand>
```

### `scripts list`

List all scripts.

### `scripts create`

Create a new script.

**Flags:**
- `--name <NAME>` (required) — script name
- `--file <PATH>` — path to a `.js` file containing the source
- `--inline <SOURCE>` (-i) — inline script source
- `--hook <HOOK>` (-H, repeatable) — hook to attach to (e.g. `on_request`)
- `--description <DESC>` (-d) — optional description

Either `--file` or `--inline` must be provided.  If no `--hook` is specified,
defaults to `on_request`.

```bash
madhyamas scripts create --name "Log Requests" --file ./log.js --hook on_request
madhyamas scripts create --name "CORS" -i 'function onResponse(r,resp,c){...}' -H on_response
```

### `scripts get <id>`

Get a specific script by ID.

### `scripts delete <id>`

Delete a script.

### `scripts toggle <id> --enabled <bool>`

Enable or disable a script.

```bash
madhyamas scripts toggle abc-123 --enabled true
madhyamas scripts toggle abc-123 --enabled false
```

### `scripts templates`

List available script templates.

### `scripts test`

Dry-run a script against a sample context.

**Flags:**
- `--file <PATH>` — path to a `.js` file
- `--inline <SOURCE>` (-i) — inline source
- `--hook <HOOK>` (-H) — hook to test against

```bash
madhyamas scripts test --file ./my-script.js --hook on_request
```

### `scripts validate`

Validate a script's syntax without executing it.

**Flags:**
- `--file <PATH>` — path to a `.js` file
- `--inline <SOURCE>` (-i) — inline source

```bash
madhyamas scripts validate --file ./my-script.js
```

Exits with code 1 if the script is invalid.

### `scripts history <id>`

Show execution history for a script.

**Flags:**
- `--limit <N>` (-l, default 20) — maximum entries to show

```bash
madhyamas scripts history abc-123 --limit 50
```

---

## MCP Tools

The following MCP tools are available for AI agent integration:

### `madhyamas_list_scripts`

List all registered scripts.

**Input:** none

### `madhyamas_create_script`

Create a new script.

**Input:**
- `name` (string, required) — script name
- `source` (string, required) — script source code
- `hook` (string, optional) — hook to attach to
- `enabled` (boolean, optional) — enable immediately

### `madhyamas_get_script`

Get a specific script.

**Input:**
- `id` (string, required) — script ID

### `madhyamas_update_script`

Update an existing script.

**Input:**
- `id` (string, required) — script ID
- `script` (object, required) — full script object

### `madhyamas_delete_script`

Delete a script.

**Input:**
- `id` (string, required) — script ID

### `madhyamas_toggle_script`

Enable or disable a script.

**Input:**
- `id` (string, required) — script ID
- `enabled` (boolean, required) — enable or disable

### `madhyamas_get_script_templates`

List predefined script templates.

**Input:** none

### `madhyamas_test_script`

Dry-run a script against a sample context.

**Input:**
- `source` (string, required) — script source code
- `hook` (string, required) — hook to test against

### `madhyamas_validate_script`

Validate a script's syntax.

**Input:**
- `source` (string, required) — script source code

### `madhyamas_get_script_history`

Get execution history for a script.

**Input:**
- `id` (string, required) — script ID
- `limit` (integer, optional) — max entries (default 50)
