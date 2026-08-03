# Rewrite Templates

Rewrite templates are **pre-built rewrite rules** that solve common debugging scenarios with a single click. Instead of manually configuring individual header manipulations, you pick a template and Madhyamas creates a fully configured [rewrite](./rewrites) rule for you. You can then customize it (for example, restrict it to specific hosts) or use it as-is.

## Available Templates

| Template | Direction | Actions | Use Case |
|----------|-----------|---------|----------|
| Add CORS Headers | Response | 3 | Add `Access-Control-Allow-*` headers to responses |
| HTTP to HTTPS | Request | 1 | Redirect HTTP requests to HTTPS |
| Add Auth Header | Request | 1 | Add an `Authorization: Bearer ...` header to requests |
| Remove Security Headers | Response | 2 | Strip CSP / `X-Frame-Options` for testing |
| No Caching | Both | 8 | Disable all client/intermediary caching |
| Block Cookies | Both | 2 | Strip cookies from requests and responses |

Templates are available from the **Rewrites** panel → **Create** → **Quick Templates** picker in the web UI.

## Where Templates Fit

Rewrite rules run at priority 10 in the intercept pipeline — after the [block list](./block-list) (priority 5) but before [mocks](./mocks) (20), [breakpoints](./breakpoints) (30), and [throttling](./throttling) (40). Templates with `direction: both` apply actions on both the request and the response.

## No Caching

The **No Caching** template prevents clients and intermediary caches from serving stale content. It ensures every request through the proxy reaches the upstream server and returns the latest response — essential when you're debugging and need to see fresh data on every reload.

Without it, browsers and HTTP caches can return `304 Not Modified` responses or serve cached copies, hiding changes you made on the server.

### What it does

The template creates a single rewrite rule with **8 actions**:

**On requests** — strip conditional request headers so the server can't answer `304 Not Modified`:

| Action | Header | Effect |
|--------|--------|--------|
| Remove | `If-Modified-Since` | Server can't compare modification time |
| Remove | `If-None-Match` | Server can't compare ETag |

**On responses** — remove validators/expiration hints and add explicit no-cache directives:

| Action | Header | Value | Effect |
|--------|--------|-------|--------|
| Remove | `ETag` | — | No validator for future conditional requests |
| Remove | `Last-Modified` | — | No validator for future conditional requests |
| Remove | `Expires` | — | Remove old expiration hint |
| Set | `Cache-Control` | `no-cache, no-store, must-revalidate` | Strong no-cache directive |
| Set | `Pragma` | `no-cache` | HTTP/1.0 fallback no-cache directive |
| Set | `Expires` | `0` | Expire immediately |

### When to use it

- **Debugging server changes** — see your latest code on every reload without hard-refreshing or clearing the cache
- **API development** — ensure you always hit the real backend, not a cache
- **Performance testing** — measure true upstream latency without cache hits

## Block Cookies

The **Block Cookies** template strips cookies from both directions of traffic. The client never sends cookies to the server, and the server never sets cookies on the client. This effectively makes every request look like it comes from an anonymous, first-time visitor.

### What it does

The template creates a single rewrite rule with **2 actions**:

| Direction | Action | Header | Effect |
|-----------|--------|--------|--------|
| Request | Remove | `Cookie` | Client can't send session/auth cookies |
| Response | Remove | `Set-Cookie` | Server can't set new cookies |

### When to use it

- **Anonymous visitor testing** — see how your site looks to a first-time user with no stored session, preferences, or auth
- **Login flow debugging** — test the login flow from a clean state on every request without clearing cookies manually
- **Cookie-related bug hunting** — isolate whether a bug is caused by a stale or malformed cookie
- **A/B test isolation** — ensure cookie-based A/B test buckets don't influence your debugging

::: tip
HTTP responses can contain multiple `Set-Cookie` headers. The template removes the entire `Set-Cookie` key, blocking all cookies in one action. If you need to block only specific cookies, create a custom rewrite rule instead.
:::

## Using Templates Together

No Caching and Block Cookies are independent and can be enabled simultaneously. When both are active, the request has conditional headers **and** cookies stripped, and the response has cache headers replaced **and** `Set-Cookie` removed.

## Managing Template Rules

Once created from a template, a rule behaves like any other rewrite rule. You can toggle it on/off, edit it (for example, to restrict it to a specific host pattern), or delete it. Disabling a rule leaves it in place but stops it from applying — useful for A/B comparison: enable No Caching, reload to see fresh content, then disable it and reload to see cached behavior.

See the [Rewrites](./rewrites) guide for general rewrite management (priority, enabling/disabling, editing).

## Common Use Cases

### Always-Fresh Debugging

Turn on No Caching once and never worry about stale content again — every reload hits the real backend.

### Testing as a New User

Combine Block Cookies with No Caching to simulate a brand-new visitor on every request: no session, no cache, no preferences.

### Stripping Security Headers for Testing

Use the Remove Security Headers template to test whether your app works without Content-Security-Policy or `X-Frame-Options` — useful when embedding content in third-party pages.
