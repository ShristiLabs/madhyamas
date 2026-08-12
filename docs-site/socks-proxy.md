---
title: SOCKS5 Proxy
description: Run a SOCKS5 listener alongside the HTTP/HTTPS proxy in Madhyamas to capture traffic from SOCKS-only clients and tunnel non-HTTP TCP connections.
---

# SOCKS5 Proxy

Madhyamas can run a **SOCKS5** proxy listener alongside its HTTP/HTTPS proxy listener. SOCKS5 is a generic TCP tunneling protocol used by browsers, CLI tools, and mobile devices that prefer SOCKS over HTTP `CONNECT`. This is convenient for capturing traffic from clients that only speak SOCKS, or for tunneling non-HTTP TCP connections through the proxy.

## HTTP Proxy vs SOCKS5 — Which Should I Use?

| Capability | HTTP proxy (`:8888`) | SOCKS5 (`:1080`) |
|---|---|---|
| HTTP interception (URL, headers, body) | Full | Blind tunnel (no inspection) |
| HTTPS interception (MITM TLS) | Via `CONNECT` + CA cert | Not possible |
| HTTPS passthrough (no MITM) | Via passthrough domains | Always (it's a tunnel) |
| Arbitrary TCP ports (e.g. `:3306`) | Via `CONNECT` only | Native |
| Username/password auth | Proxy-Authorization | RFC 1929 |
| Visible in web UI | Full detail | Connection entry |

**Rule of thumb:** use the **HTTP proxy port** when you need to inspect or modify HTTP/HTTPS traffic. Use the **SOCKS5 port** when the client only speaks SOCKS, or when you need to tunnel arbitrary TCP (databases, SSH, custom protocols) and still see the connection in the traffic list.

## What Gets Recorded

SOCKS5 is a **blind TCP tunnel** — the proxy relays raw bytes in both directions without interpreting the application protocol. Every SOCKS5 `CONNECT` creates a traffic entry flagged as passthrough so the connection is visible in the web UI. The entry contains:

| Field | Value |
|-------|-------|
| **Method** | `CONNECT` |
| **URL** | `tcp://host:port/` (or `https://host:port/` for port 443) |
| **Host** | Requested host (IP or domain) |
| **HTTP version** | `SOCKS5` |
| **Status** | `200` on success, `502`/`504` on failure |

The request/response contents (URL path, headers, body) are **not** captured because they flow inside the opaque tunnel.

::: warning
HTTPS **cannot** be MITM-intercepted via SOCKS. The client's TLS session goes straight to the target, so the proxy never sees decrypted traffic. To intercept HTTPS, use the HTTP proxy port with `CONNECT` and [install the CA certificate](./https-certificates).
:::

## Enabling SOCKS5

SOCKS5 is **disabled by default**. Enable it via CLI flags, the REST API, or the config file.

### CLI Flags

```bash
madhyamas serve --enable-socks --socks-port 1080
```

| Flag | Env var | Default | Description |
|------|---------|---------|-------------|
| `--enable-socks` | `MADHYAMAS_ENABLE_SOCKS` | `false` | Turn on the SOCKS5 listener |
| `--socks-port` | `MADHYAMAS_SOCKS_PORT` | `1080` | Port for the SOCKS5 listener |
| `--socks-username` | `MADHYAMAS_SOCKS_USERNAME` | (none) | Require username/password auth |
| `--socks-password` | `MADHYAMAS_SOCKS_PASSWORD` | (none) | Password (only with `--socks-username`) |

The SOCKS listener binds to the same `--host` as the HTTP proxy (default `127.0.0.1`).

### REST API

```bash
# Enable the SOCKS5 listener (requires restart to bind the port)
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"enable_socks": true, "socks_port": 1080}'

# Require authentication
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"socks_auth_username": "alice", "socks_auth_password": "secret"}'
```

::: warning
`enable_socks`, `socks_port`, and auth credentials take effect after a **restart** — the TCP listener is bound at startup.
:::

### Config File

Edit `~/.madhyamas/config.json`:

```json
{
  "enable_socks": true,
  "socks_port": 1080,
  "socks_auth_username": "alice",
  "socks_auth_password": "secret"
}
```

## Configuring Clients

Once the SOCKS5 listener is running on `127.0.0.1:1080`, point any SOCKS5-capable client at it.

### curl

```bash
# HTTP over SOCKS5
curl --socks5 127.0.0.1:1080 http://www.example.com

# HTTPS over SOCKS5 (blind tunnel — not intercepted)
curl --socks5 127.0.0.1:1080 https://www.example.com

# With authentication
curl --socks5 alice:secret@127.0.0.1:1080 http://www.example.com

# Arbitrary TCP port (e.g. a database)
curl --socks5 127.0.0.1:1080 http://db.internal:3306
```

### Firefox

1. Open `about:preferences` → **Network Settings** → **Settings…**
2. Select **Manual proxy configuration**
3. Set **SOCKS Host** = `127.0.0.1`, **Port** = `1080`, **SOCKS v5**
4. (Optional) Check **Proxy DNS when using SOCKS v5** to avoid DNS leaks
5. Leave HTTP/SSL proxies blank (or set them to the HTTP proxy port for those)

### Chrome / Edge

```bash
google-chrome --proxy-server="socks5://127.0.0.1:1080"
```

### Environment Variable

```bash
export ALL_PROXY=socks5://alice:secret@127.0.0.1:1080
```

## Authentication

Madhyamas supports the two SOCKS5 auth methods from RFC 1928/1929:

| Method | When used |
|--------|-----------|
| No authentication | Default (no `--socks-username`) |
| Username/password | When `--socks-username` (and `--socks-password`) are set |

When auth is required and the client doesn't offer the username/password method (or provides wrong credentials), the server rejects the connection.

## Common Use Cases

### Tunneling Non-HTTP TCP

Route arbitrary TCP connections (databases, SSH, custom protocols) through the proxy so they appear in the traffic list alongside your HTTP traffic.

### Clients That Only Speak SOCKS

Some tools and mobile apps only support SOCKS5 proxies. Enable the SOCKS5 listener to capture their traffic without reconfiguring the rest of your setup.

### DNS Leak Prevention

Use `--socks5-hostname` (curl) or "Proxy DNS when using SOCKS v5" (Firefox) to have the proxy resolve hostnames, avoiding DNS leaks on the client machine.

## Troubleshooting

### `curl: (97) SOCKS5: authentication failed`

The server requires username/password auth but the client didn't provide credentials (or provided wrong ones). Provide them: `curl --socks5 alice:secret@127.0.0.1:1080 ...`

### `curl: (97) SOCKS5: no acceptable auth method`

The server requires auth but the client only offered "no authentication". Either configure the client to send credentials, or remove `--socks-username` from the server to allow open access.

### HTTPS traffic isn't showing request details

This is expected — SOCKS5 is a blind tunnel. You'll see a `CONNECT` entry with `http_version: SOCKS5`, but no URL path, headers, or body. To intercept HTTPS, use the HTTP proxy port (`:8888`) with `CONNECT` and install the CA certificate.

### `Address already in use` on startup

Another process (or a previous Madhyamas instance) is using the SOCKS port. Either stop it or choose a different port with `--socks-port`.

### Connections to a target fail with 502

The proxy couldn't reach the target (DNS failure, refused, unreachable). The traffic entry's response body includes the specific SOCKS5 reply code and the underlying error.

## See also

- [Upstream Proxy](./upstream-proxy) — chain outbound traffic through another proxy
- [Configuration](./configuration) — `--enable-socks`, `--socks-port`, and related flags
- [Mobile Setup](./mobile-setup) — connecting mobile devices
- [REST API reference](./rest-api) — SOCKS configuration via `/api/config`
