# HTTP/2 & gRPC

Madhyamas supports **HTTP/2** on the downstream (client-facing) side of the proxy. Enabling it lets you intercept HTTP/2 traffic, including **gRPC**, which mandates HTTP/2.

![gRPC View](/screenshots/grpc-view.png)

## How It Works

When HTTP/2 downstream is enabled, the proxy advertises both `h2` and `http/1.1` via [ALPN](https://tools.ietf.org/html/rfc7301) during the TLS handshake with the client. The negotiated protocol determines how the proxy parses the client's requests:

| ALPN result | Handler | Stored as |
|-------------|---------|-----------|
| `h2` | HTTP/2 frame parser | `HTTP/2` |
| `http/1.1` | Existing HTTP/1.1 parser | `HTTP/1.1` |
| (none) | HTTP/1.1 fallback | `HTTP/1.1` |

Both paths feed into the **same interception pipeline** (rewrites, mocks, breakpoints, upstream forwarding, traffic recording), so all features work identically regardless of protocol.

## Enabling HTTP/2

HTTP/2 downstream is **disabled by default** for safety. Enable it via the web UI, REST API, or config file.

### Web UI

1. Open **Settings** (gear icon)
2. Go to the **General** tab
3. Under **HTTP/2**, toggle **Enable HTTP/2 Downstream**
4. Click **Save**
5. **Restart** the proxy for the change to take effect

### REST API

```bash
# Enable
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"enable_h2_downstream": true}'

# Verify
curl http://127.0.0.1:3001/api/config | jq .enable_h2_downstream
```

::: warning
A **restart is required** after changing this setting. The ALPN advertisement is baked into the TLS configuration at startup, so new TLS handshakes only use the updated ALPN list after the proxy restarts.
:::

## Traffic Display

The **Proto** column in the traffic list shows the negotiated HTTP version:

| Value | Meaning |
|-------|---------|
| `HTTP/2` | Client negotiated HTTP/2 via ALPN |
| `HTTP/1.1` | Client used HTTP/1.1 (ALPN fallback or no ALPN) |
| `HTTP` | Plain HTTP (no TLS) |
| `HTTPS` | HTTPS with no ALPN info (legacy entries) |

The Traffic Detail view shows the HTTP version in both the Request and Response tabs.

## gRPC Support

gRPC mandates HTTP/2, so enabling HTTP/2 downstream is **required** for gRPC interception. Once enabled:

- gRPC unary calls appear in the traffic list like normal HTTP requests
- gRPC streaming calls (server-streaming, client-streaming, bidi-streaming) are captured as individual streams
- The gRPC panel can inspect decoded protobuf frames

## HAR Export

The HAR export includes the actual `httpVersion` field for both requests and responses, reflecting the negotiated protocol (`HTTP/2` or `HTTP/1.1`).

## Limitations

- **WebSocket over HTTP/2** is not supported (WebSocket upgrades use HTTP/1.1 semantics).
- **HTTP/2 server push** (`PUSH_PROMISE` frames) from the proxy to the client is not implemented.
- **Trailers** (HTTP/2 response trailers) are not captured in the traffic store.
- **Per-stream priorities** are not preserved when forwarding upstream.
- **Restart required** to change the HTTP/2 setting (ALPN is set at TLS config creation time).

## Backward Compatibility

When HTTP/2 downstream is disabled (the default), the proxy behaves exactly as before — only `http/1.1` is advertised via ALPN. Existing databases are automatically migrated to store the HTTP version, and old entries default to `HTTP/1.1`.

## Common Use Cases

### Debugging gRPC Services

Enable HTTP/2 to intercept gRPC calls from your client, inspect the protobuf payloads, and replay or mock them — just like you would with REST APIs.

### Modern Browser Traffic

Browsers increasingly use HTTP/2 for HTTPS. Enable downstream HTTP/2 to see the actual protocol your app negotiates, with full header and body inspection.

### HTTP/2-Specific Header Bugs

Some bugs only appear with HTTP/2's header compression (HPACK) or pseudo-headers. Enabling HTTP/2 lets you reproduce and debug these in the traffic detail view.
