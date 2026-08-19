---
title: Use Cases
description: Real-world scenarios for using Madhyamas — from API debugging to production troubleshooting, mobile testing, security analysis, and AI-assisted development.
---

# Use Cases

Real-world scenarios where Madhyamas helps you debug, test, and build better
software. Each use case includes a concrete walkthrough with commands and
configuration you can copy.

---

## API Development & Debugging

### Inspect API calls from your frontend

You're building a React app and the API returns unexpected data. You need to
see exactly what your frontend sends and what the server returns.

**Steps:**

1. Start Madhyamas and configure your browser to use `localhost:8888` as the
   HTTP proxy.
2. Open `http://localhost:3001` for the dashboard.
3. Interact with your app — every request appears in real time.
4. Click any request to inspect headers, query params, request body, and
   response body side by side.
5. Use the **JSON viewer** to navigate large JSON responses with Tree view and
   JSONPath queries (`$.data.users[*].name`).

```bash
# Start Madhyamas
./madhyamas

# Or via CLI, then use your browser
# Filter for just API calls
madhyamas traffic list --host api.example.com --status 200
```

**Tip:** Use [Focus Mode](./focus) to highlight requests from a specific API
host without hiding other traffic — useful when your page loads assets from
CDNs alongside API calls.

### Debug CORS errors

Your frontend at `localhost:3000` calls an API at `api.example.com` and the
browser blocks the request with a CORS error. You need to see the actual
response headers.

**Steps:**

1. Capture the preflight `OPTIONS` request and the actual request.
2. Inspect the `Access-Control-Allow-Origin`, `Access-Control-Allow-Headers`,
   and `Access-Control-Allow-Methods` response headers.
3. If the API is under your control, fix the server. If not, use a
   [rewrite rule](./rewrites) to inject CORS headers:

```bash
# Add CORS headers to responses from api.example.com
madhyamas rewrites add \
  --name "Add CORS" \
  --match-host "api.example.com" \
  --action "response-header:Access-Control-Allow-Origin:*" \
  --action "response-header:Access-Control-Allow-Methods:GET,POST,PUT,DELETE,OPTIONS" \
  --enabled
```

Or apply the built-in **Add CORS** [rewrite template](./rewrite-templates)
with one click from the Web UI.

---

## Mobile App Debugging

### Debug an iOS app's network calls

Your iOS app works in the simulator but fails on a real device. You need to
inspect its network traffic.

**Steps:**

1. Start Madhyamas on your development machine.
2. On your iPhone, go to **Settings > Wi-Fi > Configure Proxy** and set it to
   your machine's IP on port `8888`.
3. Install the Madhyamas CA certificate: visit `http://localhost:3001/cert`
   on your phone, then trust it in **Settings > General > About > Certificate
   Trust Settings**.
4. Use your app — all HTTPS traffic is now visible in the dashboard.

```bash
# Start with public IP so your phone can connect
./madhyamas --host 0.0.0.0 --public-ip 192.168.1.100
```

See the [Mobile Setup Guide](./mobile-setup) for detailed iOS and Android
instructions.

### Bypass certificate pinning on Android

Some Android apps use certificate pinning and won't trust your proxy's CA.
Madhyamas records these failed TLS handshakes as `502` entries so you can see
which domains are pinning.

**Steps:**

1. Start Madhyamas with HTTPS interception enabled.
2. Use your app — pinned connections appear as `502` entries with the error
   "TLS handshake failed."
3. To bypass pinning on a rooted device, use tools like Frida or
   `objection` alongside Madhyamas.

See [Android Cert Pinning](https://github.com/ShristiLabs/madhyamas/blob/main/docs/ANDROID_CERT_PINNING.md)
for detailed bypass instructions.

---

## Mock APIs

### Build a frontend before the backend is ready

Your team is building a new feature but the API isn't implemented yet. You
need realistic responses to develop the UI.

**Steps:**

1. Create a mock that matches the planned API endpoint:

```bash
madhyamas mocks add \
  --name "Get Users" \
  --match-method GET \
  --match-url "*/api/users" \
  --status 200 \
  --content-type "application/json" \
  --body '{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}' \
  --enabled
```

2. Your frontend now receives the mock response instead of a 404.
3. Create multiple mocks for different endpoints and organize them into a
   [collection](./mocks#collections).
4. When the real API is ready, disable the mock with one click.

### Record real traffic as mocks

You want to reproduce a production issue locally. Record real API responses
from production, then replay them in your dev environment.

**Steps:**

1. Point Madhyamas at production traffic.
2. Use the **Record** feature to capture responses.
3. Export recorded mocks as JSON:

```bash
madhyamas mocks export --output production-mocks.json
```

4. In your dev environment, import and enable them:

```bash
madhyamas mocks import --input production-mocks.json
madhyamas mocks enable --all
```

### Test error handling

You need to verify your app handles 500 errors, timeouts, and rate limiting
gracefully.

```bash
# Mock a 500 error
madhyamas mocks add \
  --name "Server Error" \
  --match-url "*/api/payments" \
  --status 500 \
  --body '{"error":"internal_server_error"}' \
  --enabled

# Mock a 429 rate limit
madhyamas mocks add \
  --name "Rate Limited" \
  --match-url "*/api/search" \
  --status 429 \
  --header "Retry-After:60" \
  --enabled
```

---

## Performance Testing

### Simulate slow network conditions

Your app works great on fast WiFi but users on 3G report it's unusable. Test
under realistic network conditions.

**Steps:**

1. Enable throttling with a preset:

```bash
# Slow 3G preset
madhyamas throttle enable --preset "slow-3g"

# Or customize: 500ms latency, 1 Mbps bandwidth, 1% packet loss
madhyamas throttle enable --latency 500 --bandwidth 1mbps --packet-loss 1
```

2. Use your app and observe loading behavior.
3. Check the [waterfall timeline](./timeline-view) to see which requests are
   blocking and how long each takes.

### Batch replay for load testing

You want to send the same request 100 times with 10 concurrent connections to
stress-test an endpoint.

```bash
# Save a request first
madhyamas replay save --request-id abc123 --name "Login Test"

# Batch replay: 100 iterations, 10 concurrent, 100ms delay
madhyamas replay batch --id abc123 --iterations 100 --concurrency 10 --delay 100ms
```

View results in the dashboard or via CLI:

```bash
madhyamas replay history --limit 100
```

---

## Bug Reproduction & Regression Testing

### Capture and replay a failing request

A user reports a bug but you can't reproduce it. Capture the exact request
from their session and replay it.

**Steps:**

1. Export the user's session as HAR:

```bash
madhyamas export har --session "bug-report-123" --output bug.har
```

2. Import it on your machine:

```bash
madhyamas sessions import --input bug.har --name "Bug Report 123"
```

3. Find the failing request and replay it:

```bash
madhyamas replay execute --id <request-id>
```

4. Modify the request and replay to test fixes:

```bash
madhyamas replay execute --id <request-id> \
  --override-url "https://staging.example.com/api/login" \
  --override-header "Authorization: Bearer test-token"
```

### API regression testing after deployment

After deploying a new version of your API, replay a captured session to verify
nothing broke.

```bash
# Save a session of normal API usage
madhyamas sessions save --name "baseline-v1"

# After deployment, replay all requests against the new version
madhyamas replay batch --session "baseline-v1" \
  --override-host "staging.example.com" \
  --iterations 1 \
  --compare
```

---

## Security Analysis

### Inspect authentication tokens

You want to verify your app sends the correct auth headers and tokens aren't
leaking to third-party domains.

**Steps:**

1. Capture traffic while logging in and using the app.
2. Filter for auth-related requests:

```bash
madhyamas traffic list --header "Authorization" --host "api.example.com"
```

3. Inspect the `Authorization` header value — is it a JWT? An API key? Is it
   being sent to the right domains?
4. Check that third-party domains (analytics, CDNs) are **not** receiving your
   auth tokens.

### Block ads and trackers during development

Ad and analytics scripts pollute your traffic logs. Block them to see only
your app's requests.

```bash
# Block common ad/tracker domains
madhyamas blocklist add --pattern "*doubleclick.net*"
madhyamas blocklist add --pattern "*google-analytics.com*"
madhyamas blocklist add --pattern "*facebook.net*"
madhyamas blocklist add --pattern "*hotjar.com*"
```

Or use the Web UI's Block List panel to add patterns with one click. See
[Block List](./block-list) for pattern syntax.

### Remove security headers for testing

You need to test your app without CSP (Content-Security-Policy) to isolate
whether CSP is blocking a script.

```bash
madhyamas rewrites add \
  --name "Remove CSP" \
  --match-host "localhost:3000" \
  --action "response-header-remove:Content-Security-Policy" \
  --enabled
```

Or apply the **Remove Security Headers** [template](./rewrite-templates).

---

## Automation & Scripting

### Log all API calls automatically

You want a running log of every API call your app makes, written to a file
for later analysis.

Create a JavaScript script in the Web UI or via CLI:

```javascript
// script: api-logger
// hook: onResponse

function onResponse(request, response) {
  if (request.url.includes("/api/")) {
    console.log(`${request.method} ${request.url} -> ${response.status} (${response.bodySize} bytes)`);
  }
  return {};
}
```

```bash
madhyamas scripts add --name "api-logger" --file ./api-logger.js --enabled
```

Every API call is now logged. View logs in the Scripts panel or in the
terminal output.

### Auto-inject auth headers for local development

Your local dev server doesn't implement auth, but your frontend expects an
auth token. Inject one automatically.

```bash
madhyamas rewrites add \
  --name "Inject Dev Auth" \
  --match-host "localhost:3000" \
  --action "request-header:Authorization:Bearer dev-token-12345" \
  --enabled
```

### Block specific domains with a script

You want fine-grained blocking logic that patterns can't express — for
example, block all requests except those from your own domain.

```javascript
// script: domain-guard
// hook: onRequest

function onRequest(request) {
  const allowed = ["api.example.com", "cdn.example.com"];
  const host = request.headers["host"] || "";
  if (!allowed.some(a => host.includes(a))) {
    return {
      block: true,
      status: 403,
      body: "Blocked by domain-guard script"
    };
  }
  return {};
}
```

See the [Scripting Guide](./scripting) for the full JS API and 13 built-in
templates.

---

## AI-Assisted Debugging

### Debug with Claude Desktop

Connect Claude to your Madhyamas instance so it can inspect traffic, create
mocks, and replay requests on your behalf.

**Steps:**

1. Start Madhyamas in MCP server mode:

```bash
madhyamas mcp
```

2. Add to your Claude Desktop config
   (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/path/to/madhyamas",
      "args": ["mcp"]
    }
  }
}
```

3. Restart Claude Desktop and ask:

> "List all failed requests from the last 10 minutes and tell me what went
> wrong."

Claude will use Madhyamas MCP tools to query traffic, inspect responses, and
summarize the issues. See the [MCP Guide](./mcp) for setup with Windsurf,
Cursor, Devin, and other agents.

### Automated traffic analysis in CI/CD

Use the REST API to capture and analyze traffic during automated tests.

```bash
# Start a fresh session for your test run
curl -X POST http://localhost:3001/api/sessions \
  -H "Content-Type: application/json" \
  -d '{"name":"e2e-test-run-$(date +%s)"}'

# Run your tests...

# Check for any 5xx errors
curl "http://localhost:3001/api/traffic?status=5xx&session=current" | jq '.[] | {url, status, error}'

# Export the session for archival
curl -X GET "http://localhost:3001/api/export/har?session=current" -o test-run.har
```

---

## Team & Enterprise Scenarios

### Shared debugging proxy for a team

Multiple developers need to debug the same staging server. Instead of running
individual proxies, deploy one Madhyamas Enterprise instance with
authentication.

**Steps:**

1. Deploy Madhyamas Enterprise with PostgreSQL and Redis (see
   [Enterprise Getting Started](./enterprise/getting-started)):
2. Create user accounts for each developer:

```bash
madhyamas users create --username alice --role admin
madhyamas users create --username bob --role user
```

3. Each developer configures their browser to use the shared proxy and logs
   in with their credentials.
4. All traffic is captured with per-user attribution in the audit log.

### Compliance audit trail

Your organization requires an audit trail of who inspected what traffic and
when.

**Steps:**

1. Deploy Madhyamas Enterprise with audit logging enabled.
2. All user actions (login, traffic inspection, config changes, mock
   creation) are recorded in the audit log with SHA-256 hash chaining for
   tamper detection.
3. Export the audit log for compliance reviews:

```bash
madhyamas audit export --from 2025-01-01 --to 2025-01-31 --output audit-january.json
```

4. Verify hash chain integrity:

```bash
madhyamas audit verify --input audit-january.json
```

See the [Audit Logging Guide](./enterprise/audit-logging) for details.

### Multi-instance production debugging

You run Madhyamas behind a load balancer with multiple instances for high
availability. Traffic captured on any instance is visible on all.

**Steps:**

1. Deploy 2+ Madhyamas instances with shared PostgreSQL and Redis (see
   [Multi-Instance Deployment](./enterprise/deployment)).
2. Configure nginx as the load balancer:

```nginx
upstream madhyamas {
    server instance1:3001;
    server instance2:3001;
}
```

3. All instances share the same traffic store, sessions, and configuration.
4. Redis pub/sub propagates real-time traffic events across instances — the
   WebSocket dashboard updates regardless of which instance served the
   request.

### API key for CI/CD pipelines

Your CI pipeline needs to interact with Madhyamas programmatically.

```bash
# Create an API key (admin only)
madhyamas auth api-keys create --name "ci-pipeline" --scope "traffic:read,mocks:write"

# Use the API key in your pipeline
export MADHYAMAS_API_KEY="mad_xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
curl -H "X-API-Key: $MADHYAMAS_API_KEY" http://madhyamas:3001/api/traffic
```

See [Enterprise CLI & MCP](./enterprise/cli-mcp) for authenticated API and
MCP access.

---

## Advanced Networking

### Chain through a corporate proxy

Your office network requires all outbound traffic through a corporate proxy.
Madhyamas can chain through it.

```bash
./madhyamas --upstream-proxy-enabled \
  --upstream-proxy corporate-proxy.example.com:8080 \
  --upstream-protocol http
```

Use the bypass list for internal hosts that shouldn't go through the corporate
proxy:

```bash
--upstream-no-proxy "localhost,127.0.0.1,*.internal.example.com"
```

See [Upstream Proxy](./upstream-proxy) for all options.

### Debug gRPC microservices

Your microservices communicate via gRPC and you need to inspect the protobuf
payloads.

**Steps:**

1. Enable HTTP/2 downstream:

```bash
./madhyamas --enable-http2
```

2. Configure your gRPC client to use Madhyamas as its HTTP/2 proxy.
3. gRPC calls appear in the dashboard with decoded protobuf messages (when
   schema is available).

See [HTTP/2 & gRPC](./http2-grpc) for configuration details.

### Inspect WebSocket traffic

Your chat app uses WebSockets and messages aren't being delivered. Inspect
the WebSocket frames.

**Steps:**

1. Capture traffic while using the chat app.
2. WebSocket connections appear in the traffic list with a `WS` badge.
3. Click the connection to see all frames — both client-to-server and
   server-to-client — with timestamps and payload inspection.

See [WebSocket Inspection](./websockets) for details.

### Tunnel non-HTTP traffic with SOCKS5

You need to tunnel a database connection or SSH through the proxy.

```bash
./madhyamas --enable-socks --socks-port 1080

# Connect through SOCKS5
ssh -o ProxyCommand="nc -X 5 -x localhost:1080 %h %p" user@db-server.internal
```

See [SOCKS5 Proxy](./socks-proxy) for supported protocols and limitations.

---

## Session Management & Collaboration

### Share a debugging session with a teammate

You captured traffic that reproduces a bug. Export it so a teammate can
import and investigate.

```bash
# Export as HAR
madhyamas export har --session "bug-repro" --output bug-repro.har

# Share the HAR file (via Slack, email, GitHub, etc.)
# Your teammate imports it:
madhyamas sessions import --input bug-repro.har --name "Bug from Alice"
```

### Compare staging vs production traffic

You suspect the API behaves differently in staging vs production. Capture
both and compare.

**Steps:**

1. Create a session named "production":

```bash
madhyamas sessions create --name "production"
```

2. Browse your app against production.
3. Switch to a "staging" session and browse against staging:

```bash
madhyamas sessions switch --name "staging"
```

4. Use the [Focus](./focus) feature to highlight requests from the API host
   in both sessions and compare responses side by side.

---

## Data Archival & Compliance

### Auto-save sessions for long-running captures

You're running a long capture session and want automatic backups in case of
a crash.

```bash
# Enable auto-save every 5 minutes, keep last 10 backups
madhyamas autosave enable --interval 5m --max-backups 10 --format har
```

See [Auto Save](./auto-save) for rotation and retention options.

### Mirror API responses to disk

You want to archive API responses for offline analysis or build a mock data
library from real traffic.

```bash
# Enable mirror tool — saves response bodies to ~/.madhyamas/mirror/
madhyamas mirror enable --path ~/.madhyamas/mirror/
```

Responses are saved following the URL path structure. See
[Mirror Tool](./mirror) for configuration.

---

## Migrating from Other Tools

### Switch from Charles Proxy

Coming from Charles? Here's how to map your workflow:

| Charles Feature | Madhyamas Equivalent |
|-----------------|---------------------|
| Map Local | [Rewrites](./rewrites) — replace response body |
| Map Remote | [Rewrites](./rewrites) — redirect host |
| Breakpoints | [Breakpoints](./breakpoints) |
| Repeat Advanced | [Batch Replay](./replay) |
| Throttling | [Throttling](./throttling) |
| SSL Certificates | Automatic CA generation |
| Sessions | [Sessions](./sessions) |

Import your Charles sessions:

```bash
# Export from Charles as HAR, then import
madhyamas sessions import --input charles-export.har --name "Charles Import"
```

See the full [Migration Guide](./migration-from-charles).

### Switch from Fiddler

Import Fiddler captures (exported as HAR):

```bash
madhyamas sessions import --input fiddler-export.har --name "Fiddler Import"
```

Use [rewrite templates](./rewrite-templates) to replace Fiddler's
auto-responder rules.

---

## Quick Reference: Use Case to Feature Map

| I want to... | Use this feature |
|--------------|-----------------|
| See all HTTP traffic | [Traffic Inspection](./traffic-inspection) |
| Pause and modify a request | [Breakpoints](./breakpoints) |
| Return fake responses | [Mocks](./mocks) |
| Automatically modify traffic | [Rewrites](./rewrites) |
| Re-send a captured request | [Replay](./replay) |
| Simulate slow network | [Throttling](./throttling) |
| Block domains | [Block List](./block-list) |
| Highlight specific hosts | [Focus](./focus) |
| See request timing | [Timeline View](./timeline-view) |
| Organize traffic into groups | [Sessions](./sessions) |
| Export traffic for sharing | [HAR Export](./sessions) |
| Import traffic from browser | [HAR Import](./har-import) |
| Write custom automation | [Scripting](./scripting) |
| Extend with WASM | [Plugins](./plugins) |
| Debug with AI agents | [MCP](./mcp) |
| Tunnel non-HTTP TCP | [SOCKS5](./socks-proxy) |
| Chain through corporate proxy | [Upstream Proxy](./upstream-proxy) |
| Inspect gRPC | [HTTP/2 & gRPC](./http2-grpc) |
| Inspect WebSocket | [WebSocket](./websockets) |
| Save responses to disk | [Mirror](./mirror) |
| Auto-backup sessions | [Auto Save](./auto-save) |
| Add auth and RBAC | [Enterprise](./enterprise/) |
| Audit who did what | [Audit Logging](./enterprise/audit-logging) |
| Scale across instances | [Multi-Instance](./enterprise/deployment) |
| Authenticate API access | [API Keys](./enterprise/authentication) |
