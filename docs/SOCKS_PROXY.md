# SOCKS5 Proxy Support

> **Last verified:** 2026-08-12 against Madhyamas `0.1.6`.

Madhyamas can run a **SOCKS5** proxy listener (RFC 1928) alongside its
HTTP/HTTPS proxy listener. SOCKS5 is a generic TCP tunneling protocol used by
browsers, CLI tools, and mobile devices that prefer SOCKS over HTTP
`CONNECT`. This is convenient for capturing traffic from clients that only
speak SOCKS, or for tunneling non-HTTP TCP connections through the proxy.

> **Specs:** [RFC 1928](https://datatracker.ietf.org/doc/html/rfc1928)
> (SOCKS5), [RFC 1929](https://datatracker.ietf.org/doc/html/rfc1929)
> (username/password authentication).

---

## Overview

SOCKS5 is a **blind TCP tunnel**. The client asks the proxy to connect to an
arbitrary `host:port`; once connected, the proxy relays raw bytes in both
directions. Because the tunnel is opaque, the proxy does **not** interpret the
application protocol spoken through it.

```mermaid
graph LR
    Client["Client<br/>(browser / curl / app)"] -->|"SOCKS5 handshake<br/>+ CONNECT host:port"| Socks["Madhyamas<br/>SOCKS5 listener<br/>:1080"]
    Socks -->|"TcpStream::connect"| Target["Target server<br/>(any TCP host:port)"]
    Socks -.->|"records a traffic entry<br/>(passthrough)"| Store[("Traffic DB")]

    style Socks fill:#4a9eff,color:#fff
    style Client fill:#2d2d2d,color:#fff
    style Target fill:#2d2d2d,color:#fff
    style Store fill:#6b6b6b,color:#fff
```

### HTTP proxy vs. SOCKS5 — which should I use?

| Capability | HTTP proxy (`:8888`) | SOCKS5 (`:1080`) |
|---|---|---|
| HTTP interception (URL, headers, body) | ✅ Full | ❌ Blind tunnel |
| HTTPS interception (MITM TLS) | ✅ Via `CONNECT` + CA cert | ❌ Not possible |
| HTTPS passthrough (no MITM) | ✅ Via passthrough domains | ✅ Always (it's a tunnel) |
| Arbitrary TCP ports (e.g. `:3306`) | ⚠️ Via `CONNECT` only | ✅ Native |
| Username/password auth | ⚠️ Proxy-Authorization | ✅ RFC 1929 |
| Visible in web UI | ✅ Full detail | ✅ Connection entry |

**Rule of thumb:** use the **HTTP proxy port** when you need to inspect or
modify HTTP/HTTPS traffic. Use the **SOCKS5 port** when the client only speaks
SOCKS, or when you need to tunnel arbitrary TCP (databases, SSH, custom
protocols) and still see the connection in the traffic list.

---

## How it works

### Connection lifecycle

The SOCKS5 connection follows the three-phase handshake defined in RFC 1928,
then relays bytes until either side closes.

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant S as Madhyamas (SOCKS5)
    participant T as Target host:port
    participant DB as Traffic Store

    Note over C,S: Phase 1 — Method negotiation
    C->>S: VER=5, NMETHODS, METHODS[…]
    S->>C: VER=5, METHOD (0x00 no-auth / 0x02 user-pass / 0xFF none)

    alt METHOD = 0x02 (username/password, RFC 1929)
        Note over C,S: Phase 2 — Authentication
        C->>S: VER=1, ULEN, UNAME, PLEN, PASSWD
        S->>C: VER=1, STATUS (0x00 ok / non-zero fail)
        opt auth failed
            S--xC: connection closed
        end
    end

    Note over C,S: Phase 3 — Request
    C->>S: VER=5, CMD=1(CONNECT), RSV=0, ATYP, DST.ADDR, DST.PORT
    S->>T: TcpStream::connect(host:port)
    T-->>S: connected
    S->>C: VER=5, REP=0(succeeded), RSV=0, ATYP, BND.ADDR, BND.PORT
    S->>DB: store request (method=CONNECT, http_version=SOCKS5)
    S->>DB: store response (200, "Connection Established (SOCKS5)")

    Note over C,T: Phase 4 — Bidirectional relay
    loop until either side closes
        C->>S: application bytes
        S->>T: forward
        T->>S: application bytes
        S->>C: forward
    end
```

### What gets recorded

Every SOCKS5 `CONNECT` creates a **traffic entry** flagged as `is_passthrough`
so the connection is visible in the web UI. The entry contains:

| Field | Value |
|---|---|
| `method` | `CONNECT` |
| `url` | `tcp://host:port/` (or `https://host:port/` for port 443) |
| `host` | requested host (IP or domain) |
| `path` | `:port` |
| `http_version` | `SOCKS5` |
| `is_passthrough` | `true` |
| `response.status_code` | `200` on success, `502`/`504` on failure |
| `response.body` | Human-readable description of the tunnel |

The **request/response contents** (URL path, headers, body) are **not**
captured because they flow inside the opaque tunnel.

### Why HTTPS can't be intercepted via SOCKS

```mermaid
graph LR
    subgraph HTTP proxy [HTTP proxy port — MITM possible]
        C1[Client] -->|"CONNECT host:443"| P1[Madhyamas]
        P1 -->|"TLS handshake WITH client<br/>(forged cert from CA)"| C1
        P1 -->|"TLS handshake to target"| T1[Target]
    end

    subgraph SOCKS [SOCKS5 port — no MITM]
        C2[Client] -->|"SOCKS5 CONNECT host:443"| P2[Madhyamas]
        C2 -->|"end-to-end TLS<br/>(client ↔ target)"| T2[Target]
        P2 -.->|"blind byte relay"| C2
        P2 -.->|"blind byte relay"| T2
    end

    style P1 fill:#4a9eff,color:#fff
    style P2 fill:#9b59b6,color:#fff
```

With the HTTP proxy, the client issues `CONNECT` and then the proxy performs
its **own** TLS handshake with the client using a certificate signed by the
Madhyamas CA — this is MITM, and it's why you must install the CA cert.

With SOCKS5, the proxy is just a pipe. The client's TLS session goes straight
to the target, so the proxy never sees the decrypted traffic. This is the same
limitation Charles Proxy and other SOCKS implementations have. **To intercept
HTTPS, use the HTTP proxy port with `CONNECT` instead.**

---

## Enabling SOCKS5

SOCKS5 is **disabled by default**. You can enable it three ways.

### 1. CLI flags (quickest)

```bash
madhyamas serve --enable-socks --socks-port 1080
```

| Flag | Env var | Default | Description |
|---|---|---|---|
| `--enable-socks` | `MADHYAMAS_ENABLE_SOCKS` | `false` | Turn on the SOCKS5 listener |
| `--socks-port` | `MADHYAMAS_SOCKS_PORT` | `1080` | Port for the SOCKS5 listener |
| `--socks-username` | `MADHYAMAS_SOCKS_USERNAME` | *(none)* | Require username/password auth |
| `--socks-password` | `MADHYAMAS_SOCKS_PASSWORD` | *(none)* | Password (only with `--socks-username`) |

The SOCKS listener binds to the same `--host` as the HTTP proxy (default
`127.0.0.1`).

### 2. REST API

```bash
# Enable the SOCKS5 listener (requires restart to bind the port)
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"enable_socks": true, "socks_port": 1080}'

# Require authentication
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"socks_auth_username": "alice", "socks_auth_password": "secret"}'

# Disable auth (set username to null)
curl -X PATCH http://127.0.0.1:3001/api/config \
  -H "Content-Type: application/json" \
  -d '{"socks_auth_username": null}'

# Verify
curl -s http://127.0.0.1:3001/api/config | jq '.enable_socks, .socks_port, .socks_auth_enabled'
```

> **Note:** `enable_socks`, `socks_port`, and the auth credentials take effect
> after a **restart** — the TCP listener is bound at startup. The config is
> persisted to `~/.madhyamas/config.json` and survives restarts.

### 3. Configuration file

Edit `~/.madhyamas/config.json`:

```json
{
  "enable_socks": true,
  "socks_port": 1080,
  "socks_auth_username": "alice",
  "socks_auth_password": "secret"
}
```

---

## Configuring clients

Once the SOCKS5 listener is running on `127.0.0.1:1080`, point any SOCKS5-capable
client at it.

### curl

```bash
# HTTP over SOCKS5 (client resolves the hostname)
curl --socks5 127.0.0.1:1080 http://www.example.com

# HTTPS over SOCKS5 (blind tunnel — not intercepted)
curl --socks5 127.0.0.1:1080 https://www.example.com

# Proxy resolves the hostname (--socks5-hostname, useful for DNS leaks)
curl --socks5-hostname 127.0.0.1:1080 http://www.example.com

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

Launch with a flag:

```bash
google-chrome --proxy-server="socks5://127.0.0.1:1080"
```

### Environment variable (for tools that honor `ALL_PROXY`)

```bash
export ALL_PROXY=socks5://alice:secret@127.0.0.1:1080
```

### Mobile / other apps

Point the app's SOCKS5 setting at the proxy's public IP (see
`--public-ip` / `MADHYAMAS_PUBLIC_IP`) and port `1080`. Remember that SOCKS5
HTTPS traffic won't be intercepted — only the connection is logged.

---

## Authentication

Madhyamas supports the two SOCKS5 auth methods from RFC 1928/1929:

| Method | Code | When used |
|---|---|---|
| No authentication | `0x00` | Default (no `--socks-username`) |
| Username/password | `0x02` | When `--socks-username` (and `--socks-password`) are set |

### Method selection logic

```mermaid
flowchart TD
    G["Client greeting<br/>(offered methods)"] --> Q{Server requires auth?}
    Q -- "yes" --> Q1{Client offers 0x02?}
    Q1 -- "yes" --> M2["Select 0x02 (user/pass)"]
    Q1 -- "no" --> MFF["Select 0xFF (no acceptable) → reject"]
    Q -- "no" --> Q2{Client offers 0x00?}
    Q2 -- "yes" --> M0["Select 0x00 (no auth)"]
    Q2 -- "no" --> Q3{Client offers 0x02?}
    Q3 -- "yes" --> M2b["Select 0x02 (user/pass)"]
    Q3 -- "no" --> MFF2["Select 0xFF (no acceptable) → reject"]

    style M2 fill:#27ae60,color:#fff
    style M0 fill:#27ae60,color:#fff
    style M2b fill:#27ae60,color:#fff
    style MFF fill:#c0392b,color:#fff
    style MFF2 fill:#c0392b,color:#fff
```

When auth is required and the client doesn't offer the username/password
method, the server replies with `0xFF` (no acceptable methods) and closes
the connection. When auth is offered but the credentials are wrong, the server
replies with a non-zero status and closes.

---

## Troubleshooting

### `curl: (97) SOCKS5: authentication failed`

The server requires username/password auth (`--socks-username` is set) but the
client didn't provide credentials, or provided the wrong ones. Provide them:

```bash
curl --socks5 alice:secret@127.0.0.1:1080 http://example.com
```

### `curl: (97) SOCKS5: no acceptable auth method`

The server requires auth but the client only offered "no authentication".
Either configure the client to send credentials, or remove `--socks-username`
from the server to allow open access.

### HTTPS traffic isn't showing request details

This is expected. SOCKS5 is a blind tunnel — HTTPS goes straight to the
target. You'll see a `CONNECT` entry with `http_version: SOCKS5` and
`is_passthrough: true`, but no URL path, headers, or body. **To intercept
HTTPS, use the HTTP proxy port (`:8888`) with `CONNECT` and install the CA
certificate.**

### `Address already in use` on startup

Another process (or a previous Madhyamas instance) is using the SOCKS port.
Either stop it or choose a different port with `--socks-port`.

### Connections to a target fail with 502

The proxy couldn't reach the target (DNS failure, refused, unreachable). The
traffic entry's response body includes the specific SOCKS5 reply code and the
underlying I/O error. Common codes:

| Reply code | Meaning |
|---|---|
| `0x05` | Connection refused by target |
| `0x03` | Network unreachable |
| `0x04` | Host unreachable |
| `0x01` | General failure |
| `0x06` | TTL expired (connect timeout) |

---

## Architecture (implementation notes)

The SOCKS5 implementation lives in
[`crates/madhyamas-core/src/proxy/socks.rs`](../crates/madhyamas-core/src/proxy/socks.rs).
It is intentionally dependency-free — the SOCKS5 binary protocol is small
enough to parse by hand, which keeps the binary lean and the logic fully
testable.

```mermaid
graph TD
    subgraph Engine ["ProxyEngine::start()"]
        HTTP["HTTP/HTTPS listener<br/>:8888"] --> HC["handle_connection"]
        SOCKS["SOCKS5 listener<br/>:1080"] --> SC["serve_socks5 (spawned task)"]
    end

    SC --> Accept["accept() loop"]
    Accept --> Handle["handle_socks5_connection<br/>(per-connection task)"]
    Handle --> Parse["Pure parsers<br/>parse_greeting / parse_request<br/>parse_auth_credentials"]
    Handle --> Dial["TcpStream::connect(target)"]
    Dial --> Relay["relay() — bidirectional copy"]
    Handle --> Rec["TrafficStore.store_request/response"]

    style SOCKS fill:#9b59b6,color:#fff
    style Handle fill:#4a9eff,color:#fff
    style Parse fill:#27ae60,color:#fff
```

### Design decisions

- **Separate port** (default `1080`) rather than same-port protocol detection.
  This matches Charles Proxy's behaviour, avoids protocol ambiguity, and lets
  clients configure the SOCKS port explicitly.
- **Pure parsing functions** (`parse_greeting`, `parse_request`,
  `parse_auth_credentials`, `select_method`) operate on `&[u8]` with no I/O,
  so they are exhaustively unit-tested without sockets.
- **Blind relay** reuses the same bidirectional copy pattern as the HTTPS
  passthrough tunnel in the proxy engine, with a 5-minute idle timeout.
- **Shared traffic store**: SOCKS connections write into the same
  `TrafficStore` as HTTP traffic, so they appear in the web UI, CLI
  (`madhyamas traffic list`), and MCP tools uniformly.
- **No new dependencies**: the SOCKS5 handshake is implemented directly
  rather than pulling in `fast-socks5` or `tokio-socks`, keeping the supply
  chain surface unchanged.

### Supported and unsupported commands

| SOCKS5 command | Status |
|---|---|
| `CONNECT` (0x01) | ✅ Supported |
| `BIND` (0x02) | ❌ Not supported (replies `0x07 command not supported`) |
| `UDP ASSOCIATE` (0x03) | ❌ Not supported (replies `0x07 command not supported`) |

| Address type | Status |
|---|---|
| IPv4 (`0x01`) | ✅ |
| Domain name (`0x03`) | ✅ |
| IPv6 (`0x04`) | ✅ |

---

## Testing

The implementation includes **33 unit/integration tests** in
`socks.rs` covering:

- Greeting parsing (valid, wrong version, truncated, empty)
- Method selection logic (all branches of the flowchart above)
- Username/password auth parsing (valid, wrong version, truncated, empty)
- Request parsing (IPv4, IPv6, domain, unsupported command/address type,
  truncation)
- Reply builders (method, auth status, CONNECT success/failure)
- I/O error → SOCKS5 reply code mapping
- **End-to-end** handshake over real loopback sockets: no-auth tunnel,
  auth-required rejection, successful user/pass auth, and wrong-password
  rejection — each verifying bytes flow correctly through the tunnel.

Run them with:

```bash
cargo test -p madhyamas-core socks
```

### Manual verification

```bash
# Start with SOCKS5 (no auth)
madhyamas serve --enable-socks --socks-port 1080

# HTTP over SOCKS5
curl --socks5 127.0.0.1:1080 http://www.example.com   # → 200

# HTTPS over SOCKS5 (blind tunnel)
curl --socks5 127.0.0.1:1080 https://www.example.com  # → 200

# Inspect captured connections
curl http://127.0.0.1:3001/api/traffic | jq '.[] | {method, url, http_version: .request.http_version, is_passthrough}'
```

---

## See also

- [GETTING_STARTED.md](GETTING_STARTED.md) — initial setup and CA certificate
- [NETWORK_CONFIGURATION.md](NETWORK_CONFIGURATION.md) — client proxy setup
- [PROXY_FLOW.md](PROXY_FLOW.md) — how the HTTP/HTTPS proxy engine works
- [HTTP2_SUPPORT.md](HTTP2_SUPPORT.md) — HTTP/2 downstream support
- [CHARLES_PROXY_FEATURE_COMPARISON.md](CHARLES_PROXY_FEATURE_COMPARISON.md) —
  feature parity with Charles Proxy
