---
title: Upstream Proxy Chaining
description: Route all outbound Madhyamas traffic through a configurable upstream proxy — HTTP, HTTPS, or SOCKS5, with auth and a bypass list for corporate egress and geo-routing.
---

# Upstream Proxy Chaining

Madhyamas can route all outbound traffic through a configurable **upstream (external) proxy**. This is essential for corporate networks with a mandatory egress proxy, for geo-routing through remote proxies, or for chaining multiple debugging proxies together.

## Supported Protocols

| Protocol | HTTP forwarding | Raw TCP tunneling (CONNECT / passthrough / WebSocket) |
|----------|-----------------|--------------------------------------------------------|
| `http` | Yes | Yes (HTTP CONNECT) |
| `https` | Yes (TLS-wrapped) | No — use `http` or `socks5` for tunnel paths |
| `socks5` | Yes | Yes (SOCKS5 CONNECT) |

::: tip
HTTPS upstream proxies work for the HTTP forwarding path but **not** for raw TCP tunneling (CONNECT/passthrough), because the TLS layer can't be returned as a plain stream. If you need to tunnel HTTPS traffic through an upstream proxy, use `http` or `socks5`.
:::

## Quick Start

### CLI

```bash
# HTTP upstream proxy
madhyamas \
  --upstream-proxy-enabled \
  --upstream-proxy corp-proxy.example.com:8080 \
  --upstream-protocol http

# SOCKS5 upstream proxy with auth
madhyamas \
  --upstream-proxy-enabled \
  --upstream-proxy socks.example.com:1080 \
  --upstream-protocol socks5 \
  --upstream-auth alice:secret

# With a bypass list (don't proxy internal traffic)
madhyamas \
  --upstream-proxy-enabled \
  --upstream-proxy corp-proxy.example.com:8080 \
  --upstream-no-proxy "localhost,127.0.0.0/8,*.internal.corp"
```

### Environment Variables

```bash
export MADHYAMAS_UPSTREAM_PROXY_ENABLED=true
export MADHYAMAS_UPSTREAM_PROXY=corp-proxy.example.com:8080
export MADHYAMAS_UPSTREAM_PROTOCOL=http
export MADHYAMAS_UPSTREAM_AUTH=alice:secret
export MADHYAMAS_UPSTREAM_NO_PROXY="localhost,127.0.0.0/8"
madhyamas
```

### Web UI

Open the Config dialog → **Upstream Proxy** tab. Toggle "Enable Upstream Proxy", fill in the host/port/protocol, and click **Save Changes**. The settings persist to the config file and survive restarts.

### REST API

```bash
# Enable upstream proxy
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H 'Content-Type: application/json' \
  -d '{
    "upstream_proxy": {
      "enabled": true,
      "protocol": "http",
      "host": "corp-proxy.example.com",
      "port": 8080,
      "auth_username": "alice",
      "auth_password": "secret",
      "no_proxy_hosts": ["localhost", "127.0.0.0/8"]
    }
  }'

# Disable upstream proxy
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H 'Content-Type: application/json' \
  -d '{"upstream_proxy": {"enabled": false}}'
```

::: warning
The `auth_password` field is write-only — it's never returned in `GET /config` responses to avoid leaking credentials.
:::

## CLI Flags

| Flag | Env var | Description |
|------|---------|-------------|
| `--upstream-proxy-enabled` | `MADHYAMAS_UPSTREAM_PROXY_ENABLED` | Enable upstream proxy chaining |
| `--upstream-proxy <host:port>` | `MADHYAMAS_UPSTREAM_PROXY` | Upstream proxy address |
| `--upstream-protocol <http\|https\|socks5>` | `MADHYAMAS_UPSTREAM_PROTOCOL` | Proxy protocol (default: `http`) |
| `--upstream-auth <user:pass>` | `MADHYAMAS_UPSTREAM_AUTH` | Basic-auth (HTTP) or username/password (SOCKS5) |
| `--upstream-no-proxy <list>` | `MADHYAMAS_UPSTREAM_NO_PROXY` | Comma-separated bypass list |

## Bypass List (`no_proxy_hosts`)

The bypass list specifies hosts/CIDRs that should **skip** the upstream proxy and connect directly. Matching is case-insensitive and supports:

| Pattern | Example | Matches |
|---------|---------|---------|
| Exact hostname | `localhost` | `localhost`, `api.localhost` (suffix match) |
| Suffix match | `example.com` | `example.com`, `api.example.com` |
| Wildcard suffix | `*.internal.corp` | `anything.internal.corp` |
| IPv4 CIDR | `127.0.0.0/8` | `127.0.0.1`, `127.255.255.255` |
| IPv6 CIDR | `::1/128` | `::1` |

## What Takes Effect Live vs Requires Restart

| Change | Takes effect |
|--------|-------------|
| Bypass list (`no_proxy_hosts`) | Immediately for new connections |
| Auth credentials | Immediately for new connections |
| Protocol / host / port | **Requires restart** (the HTTP forwarding client is built once at startup) |

## Common Use Cases

### Corporate Egress Proxy

On networks that require all outbound traffic to go through a corporate proxy, chain Madhyamas through it so your debugging still works without bypassing network policy.

### Chaining Debugging Proxies

Run Madhyamas on your machine and chain it through a remote Madhyamas instance (or another proxy) to inspect traffic from a different network vantage point.

### Geo-Routing

Route traffic through an upstream proxy in another region to test geo-specific behavior — for example, to see how a CDN serves content in a different country.

### Selective Proxying

Use the bypass list to keep internal traffic direct while routing external traffic through the upstream proxy: `--upstream-no-proxy "localhost,127.0.0.0/8,*.internal.corp"`.

## Troubleshooting

### "Tunneled HTTPS connections fail with an https upstream proxy"

HTTPS upstream proxies don't support raw TCP tunneling. Switch the upstream protocol to `http` or `socks5` for CONNECT/passthrough paths.

### "Changing the upstream host via the API didn't take effect"

The HTTP forwarding path reads the upstream proxy at startup. Restart the proxy after changing the protocol, host, or port. Bypass list and auth changes take effect live.

### "Internal hosts are being proxied"

Add them to the bypass list (`--upstream-no-proxy` or `no_proxy_hosts` in the API). Use CIDR notation for IP ranges and wildcard suffixes for domains.

## See also

- [SOCKS5 Proxy](./socks-proxy) — a SOCKS5 listener for inbound clients
- [Configuration](./configuration) — upstream proxy flags and env vars
- [Access Control](./access-control) — restrict which clients can connect
- [REST API reference](./rest-api) — upstream config via `/api/config`
