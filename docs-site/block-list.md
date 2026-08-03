# Block List

The Block List lets you **block requests to specific domains or patterns** from ever reaching upstream servers. When a request's host matches a block list entry, the proxy immediately returns a configurable response (by default `403 Forbidden`) instead of forwarding the request. No upstream connection is made.

## How It Works

The block list runs **first** in the intercept pipeline — before [rewrites](./rewrites), [mocks](./mocks), [breakpoints](./breakpoints), and [throttling](./throttling). This means blocked requests never trigger any other handler or reach the upstream server.

When a request is blocked:

1. The proxy returns the configured status code and body immediately
2. An `X-Blocked-By` header identifies which block list entry caused the block
3. The blocked response appears in the traffic list like any other response (with `duration: 0`, since there's no upstream round-trip)
4. The matching entry's hit count is incremented

## Pattern Matching

Matching is case-insensitive and trailing dots are stripped.

| Pattern | Matches | Does NOT match |
|---------|---------|----------------|
| `example.com` | `example.com`, `api.example.com`, `www.example.com` | `notexample.com`, `example.org` |
| `*.example.com` | `api.example.com`, `www.example.com` | `example.com` (bare domain) |
| `ads.*` | `ads.com`, `ads.net`, `ads.example.co.uk` | `ads` (no dot) |
| `*ads*` | `doubleclick.ads.com`, `ads.example.com`, `my-ads-server.com` | `example.com` |
| `*` | Any host | (nothing) |

### Key Behaviors

- **Exact domain includes subdomains**: `example.com` matches both `example.com` and `api.example.com`.
- **Leading wildcard excludes the bare domain**: `*.example.com` matches subdomains but not `example.com` itself.
- **General wildcards use glob semantics**: `*` matches any sequence of characters. Other regex metacharacters (`.`, `+`, `?`) are treated as literal characters.
- **First match wins**: if multiple entries match the same host, the first one in the list is used.

## Creating a Block List Entry

Each entry has a pattern and a configurable response:

| Field | Default | Description |
|-------|---------|-------------|
| **Pattern** | (required) | Domain or wildcard pattern to match |
| **Note** | (none) | Optional human-readable note |
| **Enabled** | `true` | Whether the entry is active |
| **Status Code** | `403` | HTTP status code returned to the client |
| **Response Body** | `Blocked by Madhyamas` | Body text returned |
| **Content Type** | `text/plain` | Response Content-Type header |

### From the Web UI

Open the **Block List** panel in the tools sidebar to add, edit, toggle, and delete entries. Changes take effect immediately for new requests.

### From the CLI

```bash
madhyamas blocklist list                       # List all entries
madhyamas blocklist add ads.example.com        # Block a domain
madhyamas blocklist add "*.tracker.com"       # Block wildcard subdomains
madhyamas blocklist remove <id>               # Remove an entry
madhyamas blocklist toggle <id>               # Enable/disable an entry
madhyamas blocklist stats                     # View summary statistics
```

## Managing Entries

- **Toggle**: each entry has an enable/disable switch. Disabled entries stay in your list but don't block traffic — useful for temporarily lifting a block.
- **Edit**: change the pattern, response, or note at any time.
- **Delete**: remove an entry permanently.
- **Hit count**: each entry tracks how many requests it has blocked, visible in the list and stats.

All entries persist to SQLite and survive restarts.

## Common Use Cases

### Blocking Ads and Trackers

Prevent ad and analytics scripts from making external requests while you develop or test your app:

```
doubleclick.net
google-analytics.com
scorecardresearch.com
```

### Simulating API Outages

Test how your app handles third-party API failures by blocking the host and returning a custom error:

- **Pattern**: `api.stripe.com`
- **Status Code**: `503`
- **Response Body**: `{"error":{"type":"api_error","message":"Service unavailable"}}`
- **Content Type**: `application/json`

### Air-Gapped Testing

Block all external traffic to test in an isolated environment. Use the `*` pattern to block every host (use with caution — this blocks all external requests).

### Legal Compliance Blocking

Return HTTP `451 Unavailable For Legal Reasons` for specific domains with a court-order note in the entry's note field.

## Troubleshooting

### "My request isn't being blocked"

1. Verify the entry exists and is enabled
2. Check that the pattern matches the request host — remember `*.example.com` does **not** match `example.com` (use `example.com` without the wildcard for that)
3. Check the traffic view — blocked requests appear with the configured status code and an `X-Blocked-By` header

### "A request is blocked but I didn't add that domain"

Block list entries persist across restarts. Check for leftover entries from a previous session.

### "Toggling one entry off didn't unblock the domain"

If multiple entries match the same host, disabling one won't help if another enabled entry still matches. Check for duplicate patterns.
