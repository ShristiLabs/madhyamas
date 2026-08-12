---
title: Configuration
description: Configure Madhyamas via the web UI, CLI flags, and environment variables — startup options, runtime settings, capture modes, data directory, and performance tuning.
---

# Configuration

Madhyamas is configurable through the web UI, CLI flags, and environment variables. This guide covers the most important settings and how to change them.

![Config Dialog](/screenshots/config-dialog.png)

## Accessing Configuration

### Via the Web UI

Click the **Config** button (sliders icon) in the top toolbar to open the configuration dialog. Here you can view and change settings without restarting.

### Via the CLI

```bash
# View current configuration
madhyamas config get

# Update a setting
madhyamas config update --intercept-https true

# View help for all options
madhyamas config update --help
```

## Startup Options

These options are set when starting the proxy and can't be changed at runtime:

| Flag | Environment Variable | Default | Description |
|------|---------------------|---------|-------------|
| `--host` | `MADHYAMAS_HOST` | `127.0.0.1` | Bind address. Use `0.0.0.0` for mobile/remote access |
| `--proxy-port` | `MADHYAMAS_PROXY_PORT` | `8888` | Proxy server port |
| `--api-port` | `MADHYAMAS_API_PORT` | `3001` | Web UI / API port |
| `--public-ip` | `MADHYAMAS_PUBLIC_IP` | auto-detect | IP shown in UI for remote access |
| `--verbose` | `RUST_LOG=debug` | off | Enable verbose logging |
| `--no-https` | — | off | Disable HTTPS interception |
| `--max-requests` | — | `10000` | Max requests kept in memory |
| `--rate-limit` | — | off | Enable API rate limiting |
| `--enable-socks` | `MADHYAMAS_ENABLE_SOCKS` | `false` | Enable the SOCKS5 listener (see [SOCKS5 Proxy](./socks-proxy)) |
| `--socks-port` | `MADHYAMAS_SOCKS_PORT` | `1080` | SOCKS5 listener port |
| `--allowed-ip` | `MADHYAMAS_ALLOWED_IPS` | (all) | IP/CIDR allowlist, repeatable (see [Access Control](./access-control)) |
| `--upstream-proxy-enabled` | `MADHYAMAS_UPSTREAM_PROXY_ENABLED` | `false` | Enable upstream proxy chaining (see [Upstream Proxy](./upstream-proxy)) |
| `--upstream-proxy` | `MADHYAMAS_UPSTREAM_PROXY` | — | Upstream proxy host:port |
| `--upstream-protocol` | `MADHYAMAS_UPSTREAM_PROTOCOL` | `http` | Upstream protocol: http/https/socks5 |
| `--upstream-auth` | `MADHYAMAS_UPSTREAM_AUTH` | — | Upstream proxy auth as `username:password` |
| `--upstream-no-proxy` | `MADHYAMAS_UPSTREAM_NO_PROXY` | — | Comma-separated bypass list |

## Runtime Configuration

These settings can be changed while the proxy is running:

### HTTPS Interception

```bash
madhyamas config update --intercept-https true   # Enable
madhyamas config update --intercept-https false   # Disable
```

When enabled, Madhyamas decrypts HTTPS traffic using its CA certificate. When disabled, HTTPS traffic passes through as an opaque tunnel.

### Maximum Body Size

```bash
madhyamas config update --max-body-size 52428800   # 50 MB
```

Controls the maximum response body size that Madhyamas will capture. Larger bodies are truncated. Increase this if you need to inspect large responses.

### Maximum Requests in Memory

```bash
madhyamas config update --max-requests 50000
```

Controls how many traffic entries are kept in memory. Older entries are automatically removed when the limit is reached. Lower this if memory usage is a concern.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (`trace`, `debug`, `info`, `warn`, `error`) | `info` |
| `MADHYAMAS_HOST` | Bind host | `127.0.0.1` |
| `MADHYAMAS_API_PORT` | API/Web UI port | `3001` |
| `MADHYAMAS_PROXY_PORT` | Proxy port | `8888` |
| `MADHYAMAS_PUBLIC_IP` | Public IP for remote access display | auto-detect |
| `MADHYAMAS_API_URL` | API URL for CLI/MCP modes | `http://127.0.0.1:3001` |
| `MADHYAMAS_WEB_DIR` | Override web asset directory (dev only) | embedded |

## Data Directory

Madhyamas stores its data in `~/.madhyamas/`:

```
~/.madhyamas/
├── certs/                  # TLS certificates
│   ├── madhyamas-ca.pem    # CA certificate (install in trust store)
│   └── madhyamas-ca-key.pem # CA private key
├── logs/                   # Application logs
└── traffic.db              # SQLite traffic database
```

### Changing the Data Directory

You can override individual paths with CLI flags:

```bash
madhyamas serve \
  --cert-path /custom/certs \
  --db-path /custom/traffic.db \
  --log-path /custom/logs
```

## Capture Modes

Madhyamas has two capture modes, toggled via the **Recording** button in the top toolbar:

| Mode | Behavior |
|------|----------|
| **Recording** (default) | Captures all traffic to the database and displays it in the UI |
| **Passthrough** | Traffic flows through the proxy but isn't recorded |

Use Passthrough mode when you want the proxy running but don't need to capture traffic — for example, when you're only using mocks or rewrites without needing the traffic log.

## Performance Tuning

### High Traffic Volume

If you're capturing a lot of traffic:

```bash
# Increase max requests
madhyamas config update --max-requests 50000

# Or reduce to save memory
madhyamas config update --max-requests 5000
```

### Large Response Bodies

If responses are being truncated:

```bash
# Increase max body size (default ~20 MB)
madhyamas config update --max-body-size 104857600   # 100 MB
```

### Regular Cleanup

```bash
# Clear all traffic
madhyamas traffic clear

# Delete old sessions
# Use the Sessions view in the web UI
```

## Related Configuration Guides

Some features have their own dedicated configuration pages:

- [Recording Limits](./recording-limits) — bound how much traffic is recorded (max entries, total size, body capture, ignored domains)
- [Access Control](./access-control) — restrict which client IPs can connect
- [SOCKS5 Proxy](./socks-proxy) — enable the SOCKS5 listener
- [Upstream Proxy](./upstream-proxy) — chain outbound traffic through another proxy
- [HTTP/2 & gRPC](./http2-grpc) — enable HTTP/2 downstream
- [Auto Save](./auto-save) — periodic session backup and rotation
- [Mirror](./mirror) — save response bodies to disk
