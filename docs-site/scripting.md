# Scripting

Scripting lets you write **JavaScript** hooks that run inside the proxy to inspect, modify, block, or mock traffic automatically. Unlike [rewrites](./rewrites) (which are declarative rules) or [breakpoints](./breakpoints) (which pause for manual input), scripts give you full programmatic control over requests and responses.

![Scripts View](/screenshots/scripts-view.png)

## How Scripting Works

Scripts subscribe to **hooks** — events that fire during the proxy pipeline. Each hook corresponds to a JavaScript function you define. When a request or response passes through the proxy, your function runs and can:

- Log information about the traffic
- Modify request headers, URL, method, or body
- Modify response status, headers, or body
- Block a request entirely (return a custom response)
- Mock an API response without hitting the server

Scripts run in a sandboxed `boa_engine` runtime — a pure-Rust JavaScript engine with no filesystem, network, or process access. A fresh context is created for each execution, so scripts can't interfere with each other.

## Quick Start

1. Open the **Scripts** view from the tools sidebar or navigation rail
2. Go to the **Templates** tab and click **Use** on a template (e.g. "Log Requests")
3. Switch to the **Scripts** tab — the template is now a live script
4. Toggle the switch to enable it

Traffic will now flow through your script. You'll see a "script-intercepted" badge on affected rows in the traffic list.

## Hooks

Scripts subscribe to one or more hooks. Each hook maps to a JavaScript function:

| Hook | Function | When it fires |
|------|----------|--------------|
| `on_request` | `onRequest(request, context)` | Before a request is forwarded upstream |
| `on_response` | `onResponse(request, response, context)` | After a response is received |
| `on_websocket_message` | `onWebSocketMessage(context)` | On a WebSocket message |
| `on_grpc_message` | `onGrpcMessage(context)` | On a gRPC message |
| `on_traffic_store` | `onTrafficStore(context)` | When traffic is stored |
| `on_session_start` | `onSessionStart(context)` | When a session starts |
| `on_session_end` | `onSessionEnd(context)` | When a session ends |

## Return Values

Your hook function returns an object that tells the proxy what to do:

```javascript
return {
  continue: true,      // false = stop and return a custom response
  modified: false,    // true = the request/response was changed
  response: {          // only used when continue is false
    statusCode: 403,
    headers: { "Content-Type": "text/plain" },
    body: "Blocked"
  }
};
```

- **`continue: true`** (default): the proxy keeps processing normally.
- **`continue: false`**: the proxy stops and returns your `response` to the client. This is how you block requests or mock responses.
- **`modified: true`**: the proxy reads back the changes you made to the `request` (for `on_request`) or `response` (for `on_response`) and applies them to live traffic.

## Built-in Templates

The Templates tab includes ready-made scripts for common scenarios:

- **Log Requests** — log every request method and URL
- **Add CORS** — add `Access-Control-Allow-*` headers to responses
- **Block Domains** — block requests to specific domains
- **Modify Headers** — add or change headers on requests
- **Mock API** — return a fake JSON response for a matching URL
- **Inject Latency** — add an artificial delay to responses
- **Rewrite URL** — redirect requests to a different host or path
- **Inject Auth Token** — add an `Authorization` header to requests
- **Modify JSON Response** — transform a JSON response body
- **Override Status Code** — force a specific status code
- **Cache Buster** — strip caching headers
- **Strip Response Headers** — remove specific response headers
- **Conditional Mock** — mock a response only when a condition matches

## Built-in APIs

Scripts have access to a small set of safe APIs:

| API | Description |
|-----|-------------|
| `console.log(...)` | Log messages (shown in the test dialog and history) |
| `JSON.parse(str)` / `JSON.stringify(obj)` | Standard JSON parsing and serialization |
| `base64.encode(str)` / `base64.decode(str)` | Base64 encoding and decoding |
| `crypto.hash(input)` | SHA-256 hex digest |
| `url.parse(urlString)` / `url.build(components)` | URL parsing and construction |

## Testing Scripts

Before enabling a script on live traffic, you can test it safely:

1. Open a script in the editor
2. Click **Test** to open the test dialog
3. Select a hook and click **Run Test**
4. The script runs against a sample request/response and shows:
   - Success or error status
   - Execution duration
   - Console output
   - Whether the request/response was modified
   - Whether the script short-circuited (returned a custom response)

You can also validate syntax from the CLI:

```bash
madhyamas scripts validate --file ./my-script.js
madhyamas scripts test --file ./my-script.js --hook on_request
```

## Managing Scripts from the CLI

```bash
madhyamas scripts list                              # List all scripts
madhyamas scripts create --name "My Script" \
  --file ./my-script.js --hook on_request           # Create from a file
madhyamas scripts toggle <script-id> --enabled false # Disable a script
madhyamas scripts history <script-id>               # View execution history
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
  return { continue: true, modified: true };
}
```

### Block Specific Domains

```javascript
var blockedDomains = ['ads.example.com', 'tracker.example.com'];

function onRequest(request, context) {
  var parts = url.parse(request.url);
  if (blockedDomains.indexOf(parts.host) !== -1) {
    return {
      continue: false,
      response: { statusCode: 403, body: 'Blocked by Madhyamas' }
    };
  }
  return { continue: true };
}
```

### Mock an API Response

```javascript
function onRequest(request, context) {
  if (request.url.indexOf('/api/user/') !== -1) {
    return {
      continue: false,
      response: {
        statusCode: 200,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id: 123, name: 'Mock User' })
      }
    };
  }
  return { continue: true };
}
```

## Persistence

Scripts are stored in the SQLite database and survive restarts. Execution history is also persisted, so you can review what your scripts did after the fact.

## Security

Scripts are sandboxed by construction:

- No filesystem, network, or process access
- A fresh context is created for each execution — no shared state between scripts
- Execution time is soft-limited (default 5 seconds)
- Scripts are trusted code, created by the proxy operator

## Common Use Cases

### Custom Logging

Log every request to a specific API with its headers and body for later analysis — more flexible than the traffic list's built-in filters.

### Dynamic Mocking

Mock responses that depend on the request body or query parameters — something static [mocks](./mocks) can't do.

### Request/Response Transformation

Apply complex transformations (e.g. rewrite JSON fields, redact sensitive headers) that go beyond what [rewrites](./rewrites) can express.

### Conditional Blocking

Block requests based on runtime logic — for example, block all requests except those carrying a specific debug token.
