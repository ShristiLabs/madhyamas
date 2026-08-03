# Access Control

Access Control lets you restrict which client IP addresses are allowed to connect to the proxy. This is essential when you expose the proxy on a network (for mobile-device testing, shared debugging, or remote access) and want to ensure only trusted devices can route traffic through it.

## How It Works

When the IP allowlist is **enabled** (non-empty), every incoming TCP connection to the proxy is checked against the list *before* any HTTP or TLS processing happens. Connections from addresses that don't match any entry are immediately closed.

Connections from loopback addresses (`127.0.0.1`, `::1`) are **always allowed** — this prevents you from accidentally locking yourself out of a locally-running proxy.

When the allowlist is **disabled** (empty, the default), all connections are accepted regardless of source IP.

## Supported Entry Formats

Each entry in the allowlist can be a single IP or a CIDR range:

| Format | Example | Matches |
|--------|---------|---------|
| Single IPv4 | `192.168.1.50` | Only that exact IP |
| IPv4 CIDR | `192.168.0.0/16` | Any IP in `192.168.0.0` – `192.168.255.255` |
| Single IPv6 | `fd00::1` | Only that exact IP |
| IPv6 CIDR | `fd00::/8` | Any IP in `fd00::` – `fdff:ffff:...:ffff` |
| All IPv4 | `0.0.0.0/0` | Any IPv4 address |
| All IPv6 | `::/0` | Any IPv6 address |

A bare IP address is treated as a `/32` (IPv4) or `/128` (IPv6) host route — you don't need to write `192.168.1.50/32`.

## Configuring Access Control

### Via the Web UI

Open the **Config** dialog and find the access control settings. Add or remove IP/CIDR entries and click **Save Changes**. Changes take effect immediately for new connections — no restart needed.

### Via the CLI (startup)

Use the `--allowed-ip` flag, which is **repeatable**. Each occurrence adds one entry:

```bash
# Allow a /24 subnet and one specific IP
madhyamas serve --allowed-ip 192.168.1.0/24 --allowed-ip 10.0.0.5

# Allow an IPv6 range
madhyamas serve --allowed-ip fd00::/8
```

You can also use the `MADHYAMAS_ALLOWED_IPS` environment variable with comma-separated values:

```bash
export MADHYAMAS_ALLOWED_IPS="192.168.1.0/24,10.0.0.5"
madhyamas serve
```

### Via the REST API (live, no restart)

Update the allowlist at runtime. Changes take effect immediately for new connections:

```bash
# Enable access control — allow two subnets
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"allowed_ips": ["192.168.0.0/16", "10.0.0.0/8"]}'

# Disable access control (allow all)
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"allowed_ips": []}'
```

Invalid entries (e.g. `not-an-ip` or `10.0.0.0/33`) are rejected with `400 Bad Request` and the existing allowlist is left unchanged.

API changes are saved to the config file and survive restarts.

## Behavior Summary

| Scenario | Behavior |
|----------|----------|
| Empty `allowed_ips` | All connections accepted (default) |
| `127.0.0.1` connects | Always accepted (loopback) |
| `::1` connects | Always accepted (IPv6 loopback) |
| IP matches a CIDR entry | Accepted |
| IP doesn't match any entry | Connection closed immediately |
| Invalid entry in API request | Rejected, config unchanged |
| API update to `allowed_ips` | New connections checked immediately |

::: warning
The SOCKS5 listener uses a config snapshot taken at startup, so changes to access control for SOCKS5 require a restart. The HTTP proxy listener picks up changes live.
:::

## Common Use Cases

### Mobile Device Testing

Allow your phone's IP to route traffic through the proxy for debugging:

```bash
# Allow your phone's IP (e.g. 192.168.1.42 on your Wi-Fi)
madhyamas serve --allowed-ip 192.168.1.42

# Or allow the whole Wi-Fi subnet
madhyamas serve --allowed-ip 192.168.1.0/24
```

### Team Debugging

Allow multiple developers' machines, or the entire office VLAN:

```bash
madhyamas serve --allowed-ip 10.0.0.0/16
```

### Remote Server (SSH Tunnel)

When running Madhyamas on a remote server, restrict access to the server itself. You'll connect via SSH tunnel, which appears as loopback (always allowed):

```bash
madhyamas serve --host 0.0.0.0 --allowed-ip 127.0.0.1
```

### Emergency Lock Down

If you accidentally exposed the proxy publicly, lock it down immediately via the API without restarting:

```bash
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"allowed_ips": ["127.0.0.1"]}'
```

## Troubleshooting

### "I can't connect to my own proxy"

Loopback (`127.0.0.1`, `::1`) is always allowed. If you can't connect locally, the issue isn't the allowlist — check the proxy port, host binding, and firewall.

### "A remote device can't connect"

1. Verify the device's IP is in the allowlist
2. Add the device's IP via the API or CLI
3. Check the proxy logs for `Connection from <IP> rejected by IP access control` warnings

### "Startup fails with 'Invalid allowed_ips configuration'"

One of your entries is malformed. Common mistakes:

- `192.168.1.0/33` — IPv4 prefix max is `/32`
- `fd00::/129` — IPv6 prefix max is `/128`
- `192.168.1` — incomplete IP (use `192.168.1.0`)
