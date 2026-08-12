# Scripting System

Madhyamas includes a built-in JavaScript scripting engine that lets you
customise traffic manipulation, filtering, and automation at runtime.  Scripts
are written in JavaScript (ES6+) and executed by an embedded [`boa_engine`]
runtime — a pure-Rust ECMAScript engine with no native dependencies.

## Quick Start

1. Open the Madhyamas web UI and navigate to **Scripts** (in the tools
   sidebar or nav rail).
2. Go to the **Templates** tab and click **Use** on a template (e.g. "Log
   Requests").
3. Switch to the **Scripts** tab — the template is now a live script.
4. Toggle the switch to enable/disable it.

You can also manage scripts from the CLI:

```bash
# List scripts
madhyamas scripts list

# Create a script from a file
madhyamas scripts create --name "My Script" --file ./my-script.js --hook on_request

# Create a script with inline source
madhyamas scripts create --name "Block Ads" --inline 'function onRequest(r,c){...}' --hook on_request

# Test a script without affecting live traffic
madhyamas scripts test --file ./my-script.js --hook on_request

# Validate syntax
madhyamas scripts validate --file ./my-script.js

# View execution history
madhyamas scripts history <script-id>

# Toggle a script
madhyamas scripts toggle <script-id> --enabled false
```

Or via the MCP tools (for AI agent integration):

```
madhyamas_list_scripts
madhyamas_create_script
madhyamas_test_script
madhyamas_validate_script
madhyamas_get_script_history
```

## How Scripts Work

### Hooks

Scripts subscribe to one or more **hooks** — events that fire during the
proxy pipeline.  Each hook corresponds to a JavaScript function you define
in your script:

| Hook | JS Function | When it Fires |
|------|-------------|---------------|
| `on_request` | `onRequest(request, context)` | Before a request is forwarded upstream |
| `on_response` | `onResponse(request, response, context)` | After a response is received |
| `on_websocket_message` | `onWebSocketMessage(context)` | On WebSocket message |
| `on_grpc_message` | `onGrpcMessage(context)` | On gRPC message |
| `on_traffic_store` | `onTrafficStore(context)` | When traffic is stored |
| `on_session_start` | `onSessionStart(context)` | When a session starts |
| `on_session_end` | `onSessionEnd(context)` | When a session ends |

### Return Values

Your hook function must return an object with these fields:

```javascript
return {
    continue: true,      // false = short-circuit (return custom response)
    modified: false,     // true = request/response was modified
    response: {          // only when continue is false
        statusCode: 403,
        headers: { "Content-Type": "text/plain" },
        body: "Blocked"
    }
};
```

- **`continue: true`** (default): the proxy continues processing the
  request/response normally.
- **`continue: false`**: the proxy stops and returns the `response` object
  directly to the client.  This is how you block requests or mock responses.
- **`modified: true`**: the proxy reads back the modified `request` (for
  `on_request`) or `response` (for `on_response`) object and applies the
  changes to the live traffic.

### The `request` Object

```javascript
{
    method: "GET",              // HTTP method
    url: "https://example.com/api/users?id=42",
    host: "example.com",
    path: "/api/users",
    headers: {                  // modifiable
        "Accept": "application/json",
        "Authorization": "Bearer token123"
    },
    body: null,                 // request body (string or null)
    contentType: null,          // Content-Type header value
    query: { id: "42" }        // parsed query parameters
}
```

### The `response` Object

```javascript
{
    statusCode: 200,
    statusMessage: "OK",
    headers: {                  // modifiable
        "Content-Type": "application/json"
    },
    body: '{"status":"ok"}',   // response body (string or null)
    contentType: "application/json",
    durationMs: 42
}
```

### The `context` Object

```javascript
{
    requestId: "req-abc123",
    sessionId: "sess-xyz789",
    hook: "on_request",
    data: {}                   // custom data shared between hooks
}
```

## Built-in APIs

Scripts have access to these built-in APIs:

### `console.log(...)`

Logs messages.  Output is captured and shown in the test dialog and execution
history.

```javascript
console.log("Processing:", request.method, request.url);
```

### `JSON.parse(str)` / `JSON.stringify(obj)`

Standard JSON parsing and serialisation (built into boa).

### `base64.encode(str)` / `base64.decode(str)`

Base64 encoding/decoding.

```javascript
var encoded = base64.encode("hello");  // "aGVsbG8="
var decoded = base64.decode(encoded);  // "hello"
```

### `crypto.hash(input)`

SHA-256 hex digest.

```javascript
var hash = crypto.hash("test");
// "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
```

### `url.parse(urlString)` / `url.build(components)`

URL parsing and construction.

```javascript
var parts = url.parse("https://example.com:8080/api/users?id=42");
// { scheme: "https", host: "example.com", port: 8080, path: "/api/users", query: { id: "42" } }

var built = url.build({ scheme: "https", host: "example.com", path: "/api/v2" });
// "https://example.com/api/v2"
```

## Examples

### Log All Requests

```javascript
function onRequest(request, context) {
    console.log(request.method + ' ' + request.url);
    return { continue: true };
}
```

### Add CORS Headers

```javascript
function onResponse(request, response, context) {
    response.headers['Access-Control-Allow-Origin'] = '*';
    response.headers['Access-Control-Allow-Methods'] = 'GET, POST, PUT, DELETE, OPTIONS';
    response.headers['Access-Control-Allow-Headers'] = '*';
    return { continue: true, modified: true };
}
```

### Block Specific Domains

```javascript
var blockedDomains = ['ads.example.com', 'tracker.example.com'];

function onRequest(request, context) {
    var parts = url.parse(request.url);
    if (blockedDomains.indexOf(parts.host) !== -1) {
        console.log('Blocked: ' + parts.host);
        return {
            continue: false,
            response: {
                statusCode: 403,
                body: 'Blocked by Madhyamas'
            }
        };
    }
    return { continue: true };
}
```

### Mock API Responses

```javascript
function onRequest(request, context) {
    if (request.url.indexOf('/api/user/') !== -1) {
        return {
            continue: false,
            response: {
                statusCode: 200,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    id: 123,
                    name: 'Mock User',
                    email: 'mock@example.com'
                })
            }
        };
    }
    return { continue: true };
}
```

### Add Custom Headers

```javascript
function onRequest(request, context) {
    request.headers['X-Madhyamas'] = 'true';
    request.headers['X-Request-ID'] = context.requestId;
    return { continue: true, modified: true };
}
```

## Persistence

Scripts are persisted to the SQLite database (`~/.madhyamas/traffic.db`) and
survive restarts.  Execution history is also persisted, with the most recent
entries available via the API and CLI.

## Configuration

Script runtime configuration can be viewed and updated via the API:

```bash
# Get current config
curl http://127.0.0.1:3001/api/scripts/config

# Update config
curl -X PUT http://127.0.0.1:3001/api/scripts/config \
  -H 'Content-Type: application/json' \
  -d '{"timeout_ms": 10000, "enable_console": true, "max_memory_bytes": 10485760, "allow_network": false, "allow_fs": false}'
```

| Field | Default | Description |
|-------|---------|-------------|
| `timeout_ms` | 5000 | Soft execution time limit (ms) |
| `max_memory_bytes` | 10485760 | Reserved for future memory limits |
| `enable_console` | true | Enable `console.log` capture |
| `allow_network` | false | Always false (no network functions registered) |
| `allow_fs` | false | Always false (no filesystem functions registered) |

## Testing & Debugging

### Test Dialog (Web UI)

When editing a script, click **Test** to open the test dialog.  Select a hook,
click **Run Test**, and the script executes against a sample request/response
context.  The result shows:

- Success/error status
- Execution duration
- Console output
- Whether the request/response was modified
- Whether the script short-circuited (returned a custom response)

### CLI

```bash
madhyamas scripts test --file ./my-script.js --hook on_request
madhyamas scripts validate --file ./my-script.js
madhyamas scripts history <script-id>
```

### API

```bash
# Test
curl -X POST http://127.0.0.1:3001/api/scripts/test \
  -H 'Content-Type: application/json' \
  -d '{"source": "function onRequest(r,c){return{continue:true}}", "hook": "on_request"}'

# Validate
curl -X POST http://127.0.0.1:3001/api/scripts/validate \
  -H 'Content-Type: application/json' \
  -d '{"source": "function onRequest(r,c){return{continue:true}}"}'

# History
curl http://127.0.0.1:3001/api/scripts/<id>/history?limit=20
```

## Security

See [SCRIPTING_SECURITY.md](SCRIPTING_SECURITY.md) for the full security model.

**Key points:**
- Scripts run in a sandboxed `boa_engine` context with no filesystem or
  network access.
- A fresh context is created for each execution — no shared state between
  scripts.
- Execution time is soft-limited by `timeout_ms`.
- Scripts are trusted code (created by the proxy operator).

## Architecture

```
┌─────────────────────────────────────────┐
│           Web UI / CLI / MCP            │
│   (create, test, validate, history)     │
└──────────────┬──────────────────────────┘
               │ HTTP API
┌──────────────▼──────────────────────────┐
│         madhyamas-api (axum)            │
│   phase3_handlers.rs                    │
└──────────────┬──────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│      madhyamas-core/scripting/          │
│  ┌─────────────┐  ┌──────────────────┐  │
│  │  runtime.rs  │  │   engine.rs      │  │
│  │  (manager)   │──│  (boa_engine)    │  │
│  └──────┬──────┘  └──────────────────┘  │
│         │         ┌──────────────────┐  │
│         ├─────────│  persistence.rs   │  │
│         │         │  (SQLite)         │  │
│  ┌──────▼──────┐  └──────────────────┘  │
│  │   hooks.rs   │                       │
│  │  (context)   │                       │
│  └──────────────┘                       │
└─────────────────────────────────────────┘
               │
┌──────────────▼──────────────────────────┐
│      extension.rs (ScriptExtension)     │
│   applies results to proxy pipeline     │
└─────────────────────────────────────────┘
```

## See Also

- [SCRIPTING_API.md](SCRIPTING_API.md) — JavaScript API reference
- [SCRIPTING_SECURITY.md](SCRIPTING_SECURITY.md) — Scripting sandbox model
- [EXTENSION_SYSTEM.md](EXTENSION_SYSTEM.md) — Unified extension model
- [INTERCEPT_PIPELINE.md](INTERCEPT_PIPELINE.md) — Where scripts run in the pipeline
