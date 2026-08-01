# High-Priority Feature Analysis — Detailed Implementation Plan

This document provides a detailed analysis of the 7 high-priority features
identified in [CHARLES_PROXY_FEATURE_COMPARISON.md](CHARLES_PROXY_FEATURE_COMPARISON.md).
For each feature it documents:

- **What exists now** — current code, with file paths and line numbers
- **What needs to be done** — concrete work items
- **Where it needs to be done** — exact files to modify or create
- **How it should be done** — implementation approach and design decisions
- **How it would show up in the UI** — web UI, CLI, and API surface
- **How it can be tested** — verification strategy
- **What needs to be documented** — docs to create or update

> All file paths are relative to the repository root
> (`/Users/harikiranbavineni/madhyamas/`).

---

## Table of Contents

1. [HTTP/2 Downstream Support](#1-http2-downstream-support)
2. [SOCKS Proxy Support](#2-socks-proxy-support)
3. [External/Upstream Proxy Chaining](#3-externalupstream-proxy-chaining)
4. [Access Control (IP Allowlist)](#4-access-control-ip-allowlist)
5. [Block List Tool](#5-block-list-tool)
6. [No Caching Tool](#6-no-caching-tool)
7. [Block Cookies Tool](#7-block-cookies-tool)

---

## 1. HTTP/2 Downstream Support

### What exists now

The proxy **explicitly forces HTTP/1.1 on the downstream (client-facing) side**
while using HTTP/2-capable `reqwest` for upstream connections.

| Aspect | Location | Current State |
|---|---|---|
| ALPN advertisement | `crates/madhyamas-core/src/proxy/engine.rs:757-760` | `config.alpn_protocols = vec![b"http/1.1".to_vec()]` — only `http/1.1` advertised |
| ALPN inspection | `crates/madhyamas-core/src/proxy/engine.rs:482-515` | Logs warning if `h2` negotiated unexpectedly; falls back to HTTP/1.1 |
| TODO comment | `crates/madhyamas-core/src/proxy/engine.rs:716-739` | Detailed comment: "integrate the `h2` crate to parse HTTP/2 frames" |
| Request parser | `crates/madhyamas-core/src/proxy/pipeline.rs:520-625` | `parse_http_request()` — text-based HTTP/1.1 parsing only |
| Upstream client | `crates/madhyamas-core/src/proxy/engine.rs:86-111` | `reqwest::Client` with connection pooling; supports HTTP/2 via ALPN |
| `h2` crate | `Cargo.lock:1121-1137` | Available as transitive dep via `reqwest` (v0.4.13); not directly imported |
| Traffic storage | `crates/madhyamas-core/src/traffic/store.rs:102-142` | No `http_version` column in `requests` or `responses` tables |
| Traffic types | `crates/madhyamas-core/src/traffic/types.rs:65-201` | `RequestData`, `ResponseData`, `TrafficEntry` — no HTTP version field |
| Web UI types | `web/src/types/traffic.ts:1-59` | Mirrors Rust types; no HTTP version field |
| Web UI display | `web/src/features/traffic/TrafficList.tsx:276-313` | "Proto" column shows HTTP vs HTTPS (URL scheme), not HTTP/1.1 vs HTTP/2 |
| HAR export | `web/src/features/traffic/TrafficDetail.tsx:897-930` | Hardcodes `httpVersion: "HTTP/1.1"` |
| Config | `crates/madhyamas-core/src/config.rs:17-61` | No HTTP/2 toggle |
| gRPC module | `crates/madhyamas-core/src/grpc/` | Feature-gated; effectively dead code without HTTP/2 downstream (gRPC mandates HTTP/2) |

### What needs to be done

1. **Add `h2` as a direct dependency** in `crates/madhyamas-core/Cargo.toml`
2. **Implement HTTP/2 frame parsing** for downstream client connections
3. **Add HTTP version tracking** to traffic types, database schema, and API
4. **Update ALPN advertisement** to offer `h2` (after `http/1.1` for fallback)
5. **Update web UI** to display HTTP version and fix HAR export
6. **Add config option** to enable/disable HTTP/2 downstream (for debugging)

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/Cargo.toml` | Add `h2 = "0.4"` to dependencies |
| `crates/madhyamas-core/src/proxy/engine.rs` | Modify `create_tls_server_config()` (line 757) to advertise `["http/1.1", "h2"]`; add `handle_h2_connection()` method; update `handle_tls_request()` to branch on ALPN result |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | Add `parse_h2_request()` or use `h2::server::handshake()` to accept HTTP/2 streams; convert h2 frames to `RequestData`/`ResponseData` |
| `crates/madhyamas-core/src/traffic/types.rs` | Add `http_version: Option<String>` to `RequestData` and `ResponseData` (e.g., `"HTTP/1.1"`, `"HTTP/2"`) |
| `crates/madhyamas-core/src/traffic/store.rs` | Add `http_version TEXT` column to `requests` and `responses` tables; update insert/query SQL |
| `crates/madhyamas-core/src/config.rs` | Add `enable_h2_downstream: bool` field to `ProxyConfig` (default: `false` initially, `true` once stable) |
| `crates/madhyamas-api/src/handlers.rs` | Include `http_version` in traffic API responses |
| `web/src/types/traffic.ts` | Add `http_version?: string` to `RequestData` and `ResponseData` |
| `web/src/features/traffic/TrafficList.tsx` | Update "Proto" column to show `HTTP/1.1` / `HTTP/2` / `HTTPS/2` instead of just HTTP/HTTPS |
| `web/src/features/traffic/TrafficDetail.tsx` | Display HTTP version in request/response headers view; fix HAR export to use actual version |
| `web/src/features/config/ConfigDialog.tsx` | Add toggle for HTTP/2 downstream support |

### How it should be done

**Phase 1 — HTTP version tracking (no behavior change):**
1. Add `http_version` field to types and DB schema (with migration)
2. Set it to `"HTTP/1.1"` for all current traffic
3. Update API and web UI to display it
4. This is low-risk and independently shippable

**Phase 2 — HTTP/2 frame parsing:**
1. Use `h2::server::handshake()` on the TLS stream when ALPN negotiates `h2`
2. The `h2` crate provides a `SendStream`/`RecvStream` API that handles frame parsing, multiplexing, and flow control
3. For each HTTP/2 stream, convert `h2::RecvStream` data into `RequestData`, run through the existing `Pipeline`, then write the response back via `h2::SendStream`
4. Keep the HTTP/1.1 path unchanged — branch based on ALPN result in `handle_tls_request()`

**Phase 3 — Enable by default:**
1. Flip `enable_h2_downstream` default to `true`
2. Update ALPN to `["h2", "http/1.1"]` (h2 preferred, http/1.1 fallback)
3. Test with gRPC clients (the primary beneficiary)

**Key design decisions:**
- Use `h2` crate directly (not hyper) for downstream — we need fine-grained control over frame parsing for interception
- Continue using `reqwest` for upstream — it already handles HTTP/2 ALPN
- The `Pipeline` abstraction should remain protocol-agnostic; convert to/from `RequestData`/`ResponseData` at the connection layer

### How it would show up in the UI

- **Web UI traffic list**: "Proto" column changes from `HTTP`/`HTTPS` to `HTTP/1.1`/`HTTP/2`/`HTTPS/1.1`/`HTTPS/2`
- **Web UI traffic detail**: HTTP version shown in request/response summary header
- **HAR export**: `httpVersion` field reflects actual protocol
- **Config dialog**: New toggle "Enable HTTP/2 downstream" under General or Advanced tab
- **CLI**: `madhyamas config get` shows `enable_h2_downstream` field
- **API**: `GET /api/traffic` responses include `http_version` in request and response objects

### How it can be tested

1. **Unit tests**: Test HTTP/2 frame parsing with synthetic h2 frames
2. **Integration test**: Use `curl --http2 --proxy localhost:8888 https://http2.example.com` and verify traffic is captured with `http_version: "HTTP/2"`
3. **gRPC test**: Use `grpcurl` through the proxy and verify gRPC frames are intercepted (currently dead code without h2)
4. **ALPN fallback test**: Verify HTTP/1.1-only clients still work when h2 is advertised
5. **HAR export test**: Export HAR and verify `httpVersion` is correct
6. **Web UI test**: Verify the Proto column shows correct version
7. **Compatibility test**: Test with Chrome, Firefox, Safari, curl, and mobile clients

### What needs to be documented

- Update `docs/PROXY_FLOW.md` — remove the "HTTP/1.1 only downstream" limitation note; document the h2 path
- Update `docs/ARCHITECTURE.md` — add HTTP/2 frame parsing to the proxy engine description
- Update `CLAUDE.md` — note HTTP/2 downstream support and the config toggle
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change HTTP/2 row from 🟡 to ✅
- Create `docs/HTTP2_SUPPORT.md` — detailed guide on HTTP/2 proxying, gRPC requirements, and troubleshooting

---

## 2. SOCKS Proxy Support

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| TCP listener | `crates/madhyamas-core/src/proxy/engine.rs:280-314` | Single `TcpListener` on `config.proxy_addr()` (default `127.0.0.1:8888`) |
| Connection detection | `crates/madhyamas-core/src/proxy/engine.rs:318-363` | `handle_connection()` peeks first 1024 bytes; checks for `"CONNECT "` string; otherwise treats as HTTP |
| SOCKS code | — | **No SOCKS implementation anywhere in the Rust backend** |
| UI placeholder | `web/src/features/config/ConfigDialog.tsx:49-58, 322-463` | `UpstreamConfig` interface includes `protocol: "http" \| "https" \| "socks5"`; `UpstreamProxyTab` has SOCKS5 dropdown; **saves to localStorage only, no API call** |
| Config | `crates/madhyamas-core/src/config.rs:17-61` | No `socks_port`, `proxy_mode`, or SOCKS-related fields |
| CLI | `crates/madhyamas/src/main.rs:44-132` | No `--socks-port` or `--proxy-mode` flags |
| Dependencies | `Cargo.toml` | No SOCKS crates (`fast-socks5`, `tokio-socks`, etc.) |

### What needs to be done

1. **Add SOCKS5 server implementation** (SOCKS4 optional)
2. **Add SOCKS listener** — either separate port or protocol detection on same port
3. **Add config fields** for SOCKS port and enable/disable
4. **Add CLI flags** for SOCKS configuration
5. **Connect the existing UI placeholder** to the backend API
6. **Handle SOCKS CONNECT command** — tunnel TCP connections (including for HTTPS)

### Where it needs to be done

| File | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `fast-socks5 = "0.9"` or implement SOCKS5 handshake manually (it's a simple binary protocol) |
| `crates/madhyamas-core/Cargo.toml` | Add SOCKS dependency |
| `crates/madhyamas-core/src/proxy/socks.rs` | **New file** — SOCKS5 handshake parser and connection handler |
| `crates/madhyamas-core/src/proxy/mod.rs` | Add `pub mod socks;` |
| `crates/madhyamas-core/src/proxy/engine.rs` | Add SOCKS listener in `start()`; add `handle_socks_connection()` method; update `handle_connection()` for protocol detection |
| `crates/madhyamas-core/src/config.rs` | Add `socks_port: Option<u16>` and `enable_socks: bool` to `ProxyConfig` |
| `crates/madhyamas/src/main.rs` | Add `--socks-port` and `--enable-socks` CLI flags; start SOCKS listener if enabled |
| `crates/madhyamas-api/src/handlers.rs` | Include SOCKS config in `GET /api/config` and `PATCH /api/config` |
| `web/src/features/config/ConfigDialog.tsx` | Add SOCKS settings to General or a new "Proxy Modes" tab (separate from the upstream proxy tab) |

### How it should be done

**Design decision: Separate port vs. same-port protocol detection**

- **Recommended: Separate port** (e.g., port 1080 for SOCKS, 8888 for HTTP)
  - Simpler implementation — no protocol ambiguity
  - Matches Charles behavior (separate SOCKS port)
  - Clients configure SOCKS port explicitly

- Alternative: Same-port detection by first byte
  - SOCKS5 starts with `0x05`, SOCKS4 with `0x04`, HTTP with ASCII
  - More complex but convenient for clients

**SOCKS5 handshake protocol** (RFC 1928):
1. Client sends version + auth methods
2. Server selects auth method (no-auth or username/password)
3. Client sends CONNECT/BIND/UDP ASSOCIATE request with target address
4. Server connects to target and replies with success/failure
5. Tunnel bidirectionally (same as HTTPS passthrough)

**Implementation approach:**
1. Create `proxy/socks.rs` with a `handle_socks5_handshake()` function
2. After handshake succeeds and target is connected, reuse the existing tunnel logic from `handle_passthrough_tunnel()` for blind TCP relay
3. For HTTPS targets via SOCKS: the client does TLS directly to the target (no MITM). To intercept, the client must use HTTP CONNECT through the HTTP proxy port instead.
4. For HTTP targets via SOCKS: parse the HTTP request from the tunneled stream and process through the `Pipeline`

**Note on HTTPS interception via SOCKS:** SOCKS is a blind TCP tunnel — the proxy cannot MITM TLS because the client connects directly to the target's TLS endpoint. To intercept HTTPS, clients should use the HTTP proxy port with CONNECT. This is the same limitation Charles has.

### How it would show up in the UI

- **Config dialog**: New section "SOCKS Proxy" with enable toggle and port field (default 1080)
- **CLI**: `madhyamas serve --socks-port 1080 --enable-socks`
- **API**: `GET /api/config` returns `socks_port` and `enable_socks` fields
- **Web UI header**: Show SOCKS port alongside proxy port in the status bar
- **Onboarding wizard**: Mention SOCKS port as an alternative for clients that prefer SOCKS

### How it can be tested

1. **Manual test**: `curl --socks5 localhost:1080 http://example.com` and verify traffic appears in the web UI
2. **SOCKS5 handshake test**: Write a unit test that sends the SOCKS5 handshake bytes and verifies the server response
3. **Auth test**: Test username/password authentication
4. **Tunnel test**: Verify TCP tunneling works for arbitrary ports (not just 80/443)
5. **Browser test**: Configure Firefox to use SOCKS5 proxy on port 1080 and verify traffic capture
6. **Compatibility test**: Test with `curl --socks5-hostname`, Firefox, and Chrome SOCKS settings

### What needs to be documented

- Update `CLAUDE.md` — add SOCKS port to configuration section
- Update `docs/GETTING_STARTED.md` — add SOCKS proxy configuration instructions
- Update `docs/NETWORK_CONFIGURATION.md` — add SOCKS client configuration for browsers, curl, mobile devices
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change SOCKS row from 🔴 to ✅
- Create `docs/SOCKS_PROXY.md` — detailed SOCKS setup guide, limitations (no HTTPS MITM via SOCKS)

---

## 3. External/Upstream Proxy Chaining

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Upstream client | `crates/madhyamas-core/src/proxy/engine.rs:86-111` | `reqwest::Client::builder().no_proxy()` — **explicitly disables all proxy chaining** |
| Forwarding | `crates/madhyamas-core/src/proxy/pipeline.rs:698-840` | `forward_via_reqwest()` uses the shared `reqwest::Client` to send requests directly to target |
| CONNECT handler | `crates/madhyamas-core/src/proxy/engine.rs:365-518` | `handle_https_tunnel()` connects directly to target via `TcpStream::connect()` |
| Passthrough | `crates/madhyamas-core/src/proxy/engine.rs:520-714` | `handle_passthrough_tunnel()` connects directly to upstream via `TcpStream::connect()` (line 566) |
| Config | `crates/madhyamas-core/src/config.rs:17-61` | No upstream proxy fields in `ProxyConfig` |
| API config | `crates/madhyamas-api/src/handlers.rs:292-356` | `GET/PATCH /api/config` — no upstream proxy fields |
| UI placeholder | `web/src/features/config/ConfigDialog.tsx:49-88, 322-463` | **Full UI exists** — `UpstreamProxyTab` with protocol selector (http/https/socks5), host, port, auth, no_proxy list. **Saves to localStorage only — no API call to backend** |
| CLI | `crates/madhyamas/src/main.rs:44-132` | No `--upstream-proxy` flags |
| Dependencies | `Cargo.toml:85` | `reqwest` without `"socks"` feature (needed for SOCKS5 upstream) |

### What needs to be done

1. **Add upstream proxy config** to `ProxyConfig` and the config API
2. **Modify `reqwest::Client` creation** to use `reqwest::Proxy` when configured
3. **Modify CONNECT handler** to tunnel through upstream proxy for HTTPS
4. **Modify passthrough handler** to connect through upstream proxy
5. **Enable `socks` feature** on `reqwest` for SOCKS5 upstream support
6. **Connect the existing UI** to the backend API (replace localStorage with API calls)
7. **Add CLI flags** for upstream proxy configuration

### Where it needs to be done

| File | Change |
|---|---|
| `Cargo.toml` (workspace, line 85) | Add `"socks"` to reqwest features: `reqwest = { version = "0.13", features = ["json", "gzip", "deflate", "brotli", "socks"] }` |
| `crates/madhyamas-core/src/config.rs` | Add `UpstreamProxyConfig` struct with `enabled`, `protocol`, `host`, `port`, `auth_username`, `auth_password`, `no_proxy_hosts`; add field to `ProxyConfig` |
| `crates/madhyamas-core/src/proxy/engine.rs` | Modify `http_client` creation (line 86-111): replace `.no_proxy()` with conditional `reqwest::Proxy::all()` / `Proxy::http()` / `Proxy::https()` based on config; modify `handle_https_tunnel()` and `handle_passthrough_tunnel()` to connect through upstream proxy |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | No change needed — `forward_via_reqwest()` uses the shared client which will have proxy configured |
| `crates/madhyamas-api/src/handlers.rs` | Add upstream proxy fields to `get_config()` response and `PatchConfigRequest` |
| `crates/madhyamas/src/main.rs` | Add `--upstream-proxy <url>`, `--upstream-protocol <http|https|socks5>`, `--upstream-auth <user:pass>` CLI flags |
| `web/src/features/config/ConfigDialog.tsx` | Replace `localStorage.setItem()` in `UpstreamProxyTab` (line 329) with `apiPatch('/config', { upstream_proxy: cfg })`; load config from API on mount |

### How it should be done

**For HTTP/HTTPS upstream proxy (reqwest-based):**
```rust
// In engine.rs, replace no_proxy() with:
if let Some(upstream) = &config.upstream_proxy {
    if upstream.enabled {
        let proxy_url = match upstream.protocol.as_str() {
            "http" => format!("http://{}:{}", upstream.host, upstream.port),
            "https" => format!("https://{}:{}", upstream.host, upstream.port),
            "socks5" => format!("socks5://{}:{}", upstream.host, upstream.port),
            _ => return Err(...),
        };
        let mut proxy = reqwest::Proxy::all(&proxy_url)?;
        if upstream.auth_username.is_some() {
            proxy = proxy.basic_auth(&upstream.auth_username, &upstream.auth_password);
        }
        builder = builder.proxy(proxy);
    }
} else {
    builder = builder.no_proxy(); // default: no upstream
}
```

**For HTTPS CONNECT through upstream proxy:**
The `handle_https_tunnel()` method currently connects directly to the target. When an upstream proxy is configured, it must:
1. Connect to the upstream proxy (not the target)
2. Send `CONNECT target:port HTTP/1.1` to the upstream proxy
3. Wait for `200 Connection Established` from the upstream proxy
4. Then proceed with TLS handshake as normal

**For passthrough tunnel through upstream proxy:**
Same as CONNECT above — connect to upstream proxy, send CONNECT, then blind-relay.

**Config struct:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpstreamProxyConfig {
    pub enabled: bool,
    pub protocol: String,      // "http", "https", "socks5"
    pub host: String,
    pub port: u16,
    pub auth_username: Option<String>,
    pub auth_password: Option<String>,
    pub no_proxy_hosts: Vec<String>,  // bypass list
}
```

### How it would show up in the UI

- **Config dialog**: The existing `UpstreamProxyTab` already has the full UI — just needs to be connected to the API instead of localStorage
- **CLI**: `madhyamas serve --upstream-proxy http://corp-proxy.example.com:8080 --upstream-auth user:pass`
- **API**: `GET /api/config` returns `upstream_proxy` object; `PATCH /api/config` accepts updates
- **Web UI status**: Show "Via: corp-proxy:8080" indicator in the header when upstream proxy is active
- **Traffic detail**: Optionally show that a request was forwarded through an upstream proxy

### How it can be tested

1. **Basic test**: Set up a simple upstream proxy (e.g., `mitmproxy` or `squid`) and configure Madhyamas to chain through it. Verify requests reach the target and responses come back.
2. **Auth test**: Configure upstream proxy with Basic auth; verify credentials are sent correctly
3. **HTTPS test**: Verify CONNECT tunneling through upstream proxy works for HTTPS sites
4. **SOCKS5 test**: Configure SOCKS5 upstream proxy; verify HTTP and HTTPS traffic flows
5. **Bypass test**: Add hosts to `no_proxy_hosts` and verify they connect directly
6. **Passthrough test**: Verify SSL passthrough domains still work through the upstream proxy
7. **UI test**: Verify the config dialog saves and loads upstream proxy settings via the API

### What needs to be documented

- Update `CLAUDE.md` — add upstream proxy config fields and CLI flags
- Update `docs/GETTING_STARTED.md` — add "Corporate Proxy" section
- Update `docs/NETWORK_CONFIGURATION.md` — add upstream proxy configuration
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change external proxy row from ❌ to ✅
- Create `docs/UPSTREAM_PROXY.md` — detailed guide on chaining through corporate proxies, auth configuration, bypass lists

---

## 4. Access Control (IP Allowlist)

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Proxy accept loop | `crates/madhyamas-core/src/proxy/engine.rs:295-314` | Accepts all connections unconditionally; `client_addr` is captured but not filtered |
| API middleware | `crates/madhyamas-api/src/middleware.rs:1-247` | Only JWT auth middleware for enterprise features; **no IP-based access control** |
| Config | `crates/madhyamas-core/src/config.rs:17-61` | No `allowed_ips`, `blocked_ips`, or ACL fields |
| API config | `crates/madhyamas-api/src/handlers.rs:292-356` | No ACL fields in config API |
| UI | `web/src/features/config/ConfigDialog.tsx` | No access control tab |
| Search results | — | No `access_control`, `allowlist`, `whitelist`, `acl`, `ip_filter` found anywhere in code |

### What needs to be done

1. **Add IP allowlist config** to `ProxyConfig` with CIDR support
2. **Implement IP filtering** in the proxy accept loop
3. **Implement IP filtering** for the API server (optional but recommended)
4. **Add API endpoints** for managing the allowlist
5. **Add CLI flags** for initial allowlist configuration
6. **Add web UI** for managing the allowlist

### Where it needs to be done

| File | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `ipnet = "2.9"` for CIDR parsing |
| `crates/madhyamas-core/Cargo.toml` | Add `ipnet` dependency |
| `crates/madhyamas-core/src/config.rs` | Add `allowed_ips: Vec<String>` to `ProxyConfig` (supports both single IPs and CIDR notation like `192.168.0.0/16`) |
| `crates/madhyamas-core/src/proxy/engine.rs` | In `start()` accept loop (line 295-314), check `client_addr` against allowlist before spawning handler; reject with connection close if not allowed |
| `crates/madhyamas-core/src/access_control.rs` | **New file** — `AccessControlList` struct with `is_allowed(ip: IpAddr) -> bool` method; parse CIDR entries using `ipnet::IpNet` |
| `crates/madhyamas-core/src/lib.rs` | Export `AccessControlList` |
| `crates/madhyamas-api/src/handlers.rs` | Include `allowed_ips` in `GET /api/config` and `PATCH /api/config` |
| `crates/madhyamas-api/src/middleware.rs` | Add `ip_access_middleware` for the API server (optional) |
| `crates/madhyamas/src/main.rs` | Add `--allowed-ip <ip>` CLI flag (repeatable) |
| `web/src/features/config/ConfigDialog.tsx` | Add "Access Control" tab with IP/CIDR list editor |

### How it should be done

**AccessControlList implementation:**
```rust
pub struct AccessControlList {
    entries: Vec<IpNet>,  // parsed CIDR ranges
    allow_all: bool,      // true if list is empty (default: allow all)
}

impl AccessControlList {
    pub fn new(rules: &[String]) -> Result<Self> {
        if rules.is_empty() {
            return Ok(Self { entries: vec![], allow_all: true });
        }
        let entries = rules.iter()
            .map(|r| r.parse::<IpNet>())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries, allow_all: false })
    }

    pub fn is_allowed(&self, addr: IpAddr) -> bool {
        if self.allow_all { return true; }
        self.entries.iter().any(|net| net.contains(&addr))
    }
}
```

**Proxy accept loop change:**
```rust
// In engine.rs start() method, after accept:
let (client_socket, client_addr) = listener.accept().await?;

// Check access control
if !self.access_control.is_allowed(client_addr.ip()) {
    warn!("Connection from {} rejected by access control", client_addr);
    drop(client_socket);  // close connection
    continue;
}
```

**Behavior:**
- Empty list = allow all (default, backward compatible)
- Non-empty list = allow only listed IPs/CIDRs
- `127.0.0.1` and `::1` should always be allowed (localhost)
- First connection from an unknown IP triggers a log warning (like Charles)

### How it would show up in the UI

- **Config dialog**: New "Access Control" tab with:
  - Toggle: "Restrict access to listed IPs only"
  - List of IP/CIDR entries with add/remove
  - Helper text explaining CIDR notation
  - Auto-detected local IP shown as suggestion
- **CLI**: `madhyamas serve --allowed-ip 192.168.1.0/24 --allowed-ip 10.0.0.5`
- **API**: `GET /api/config` returns `allowed_ips` array; `PATCH /api/config` updates it
- **Web UI header**: Show a lock icon when access control is active
- **Onboarding wizard**: Suggest adding remote device IPs when configuring mobile testing

### How it can be tested

1. **Unit test**: Test `AccessControlList::is_allowed()` with various IPs and CIDR ranges
2. **Integration test**: Start proxy with `allowed_ips: ["127.0.0.1"]`, verify local connections work and remote connections are rejected
3. **CIDR test**: Test with `192.168.0.0/16` and verify IPs in that range are allowed
4. **Empty list test**: Verify empty list allows all connections (backward compatibility)
5. **API test**: Verify `PATCH /api/config` with `allowed_ips` updates the live ACL
6. **UI test**: Add/remove IPs via the config dialog and verify they take effect

### What needs to be documented

- Update `CLAUDE.md` — add `allowed_ips` config field and `--allowed-ip` CLI flag
- Update `docs/GETTING_STARTED.md` — add "Access Control" section for multi-device testing
- Update `docs/NETWORK_CONFIGURATION.md` — add IP allowlist configuration
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change access control row from ❌ to ✅
- Create `docs/ACCESS_CONTROL.md` — detailed guide on IP allowlists, CIDR notation, multi-device setup

---

## 5. Block List Tool

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Intercept modules | `crates/madhyamas-core/src/intercept/` | `breakpoint.rs`, `handler.rs`, `mock.rs`, `regex_cache.rs`, `rewrite.rs`, `throttle.rs`, `types.rs` — **no `block_list.rs`** |
| InterceptHandler trait | `crates/madhyamas-core/src/intercept/handler.rs:36-81` | `InterceptAction::Abort` and `InterceptAction::Respond(ResponseData)` exist — can block requests |
| Pipeline priority | `crates/madhyamas-core/src/proxy/pipeline.rs:159-181` | Rewrites (10) → Mocks (20) → Breakpoints (30) → Throttle (40) |
| Script template | `crates/madhyamas-core/src/scripting/runtime.rs:404-428` | `block_domains()` script template exists — blocks via JS scripting, not native |
| Rewrite actions | `crates/madhyamas-core/src/intercept/rewrite.rs:66-98` | `RewriteAction` enum has no "block" variant — rewrites can only modify, not block |
| API routes | `crates/madhyamas-api/src/routes.rs:93-285` | No `/blocklist` routes |
| Web UI | `web/src/features/tools/` | No `BlockListPanel.tsx` |

### What needs to be done

1. **Create `BlockListManager`** as a new intercept module
2. **Implement `InterceptHandler` trait** with priority 5 (before rewrites)
3. **Wire into proxy engine** and pipeline
4. **Add API endpoints** for CRUD operations
5. **Add web UI panel** for managing blocked domains
6. **Add CLI commands** for block list management
7. **Add MCP tools** for AI agent integration

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/intercept/block_list.rs` | **New file** — `BlockListManager` struct with domain list, `InterceptHandler` impl |
| `crates/madhyamas-core/src/intercept/mod.rs` | Add `mod block_list;` and `pub use block_list::BlockListManager;` |
| `crates/madhyamas-core/src/proxy/engine.rs` | Add `block_list_manager: OnceLock<Arc<BlockListManager>>` field; add `with_block_list_manager()` builder |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | Add `block_list_manager` to `Pipeline` struct; add to `handlers()` list |
| `crates/madhyamas/src/main.rs` | Create `BlockListManager` instance; wire to engine |
| `crates/madhyamas-api/src/routes.rs` | Add routes: `GET/POST /blocklist`, `DELETE /blocklist/{id}`, `POST /blocklist/{id}/toggle` |
| `crates/madhyamas-api/src/intercept_handlers.rs` | Add `get_blocklist`, `create_blocklist_entry`, `delete_blocklist_entry`, `toggle_blocklist_entry` handlers |
| `crates/madhyamas-api/src/handlers.rs` | Add `block_list_manager` to `AppState` |
| `crates/madhyamas-cli/src/commands/blocklist.rs` | **New file** — `madhyamas blocklist list|add|remove|toggle` |
| `crates/madhyamas-cli/src/commands/mod.rs` | Add `blocklist` module and command enum variant |
| `crates/madhyamas-mcp/src/tools/blocklist.rs` | **New file** — MCP tools for block list |
| `crates/madhyamas-mcp/src/tools/registry.rs` | Register block list tools |
| `crates/madhyamas-mcp/src/tools/executor.rs` | Add block list tool handlers |
| `web/src/features/tools/BlockListPanel.tsx` | **New file** — UI panel for managing blocked domains |
| `web/src/lib/api/intercept.ts` | Add `BlockListEntry` type and `useBlockList`, `useCreateBlockListEntry`, `useDeleteBlockListEntry`, `useToggleBlockListEntry` hooks |
| `web/src/features/tools/ToolsSidebar.tsx` | Add Block List to the tools navigation |
| `web/src/App.tsx` | Add `BlockListPanel` to tool view routing |

### How it should be done

**BlockListManager structure:**
```rust
pub struct BlockListEntry {
    pub id: String,
    pub pattern: String,       // domain or wildcard pattern, e.g. "*.ads.example.com"
    pub enabled: bool,
    pub hit_count: u64,
    pub status_code: u16,      // default 403
    pub response_body: String, // default "Blocked by Madhyamas"
}

pub struct BlockListManager {
    entries: RwLock<Vec<BlockListEntry>>,
    store: Option<Arc<InterceptStore>>,
}

#[async_trait]
impl InterceptHandler for BlockListManager {
    fn name(&self) -> &'static str { "block_list" }
    fn priority(&self) -> u32 { 5 }  // before rewrites (10)

    async fn on_request(&self, request: &mut RequestData) -> InterceptAction {
        for entry in self.entries.read().await.iter() {
            if entry.enabled && matches_pattern(&entry.pattern, &request.host) {
                // Increment hit count
                return InterceptAction::Respond(ResponseData {
                    status_code: entry.status_code,
                    body: Some(entry.response_body.as_bytes().to_vec()),
                    content_type: Some("text/plain".to_string()),
                    ..Default::default()
                });
            }
        }
        InterceptAction::Continue
    }
}
```

**Pattern matching:** Support wildcards like `*.example.com` (matches subdomains) and exact domain matches. Reuse the existing pattern matching from `MatchCondition` in `intercept/types.rs`.

**Persistence:** Use `InterceptStore` (same as other managers) to persist block list entries across restarts.

### How it would show up in the UI

- **Tools sidebar**: New "Block List" icon (Shield or Ban icon) in the navigation rail
- **BlockListPanel**: 
  - Header with stats (total entries, enabled, total hits)
  - Search/filter bar
  - List of blocked domains with enable/disable switches, hit count, delete button
  - "Add Domain" dialog with pattern input, status code, and custom response body
  - Quick-add from traffic view (right-click a request → "Block this domain")
- **CLI**: `madhyamas blocklist list`, `madhyamas blocklist add "*.ads.example.com"`, `madhyamas blocklist remove <id>`, `madhyamas blocklist toggle <id>`
- **MCP**: `madhyamas_list_blocklist`, `madhyamas_add_blocklist_entry`, `madhyamas_remove_blocklist_entry`, `madhyamas_toggle_blocklist_entry`
- **API**: `GET /api/blocklist`, `POST /api/blocklist`, `DELETE /api/blocklist/{id}`, `POST /api/blocklist/{id}/toggle`

### How it can be tested

1. **Unit test**: Test `matches_pattern()` with exact domains, wildcards, and non-matches
2. **Integration test**: Add a block list entry for `example.com`, make a request through the proxy, verify 403 response
3. **Wildcard test**: Add `*.example.com`, verify `api.example.com` is blocked but `example.org` is not
4. **Toggle test**: Disable an entry and verify requests pass through
5. **Persistence test**: Restart the proxy and verify block list entries persist
6. **API test**: CRUD operations via `curl` against the API
7. **UI test**: Add/remove/toggle entries via the web UI
8. **Priority test**: Verify block list runs before rewrites (a rewrite rule on a blocked domain should not fire)

### What needs to be documented

- Update `CLAUDE.md` — add Block List to the intercept pipeline description and API endpoints table
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Block List row from ❌ to ✅
- Update the madhyamas skill (`.claude/skills/madhyamas/`) — add block list workflow and MCP tools
- Create `docs/BLOCK_LIST.md` — usage guide with examples (blocking ads, trackers, third-party resources)

---

## 6. No Caching Tool

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Rewrite header manipulation | `crates/madhyamas-core/src/intercept/rewrite.rs:245-377` | `SetHeader`, `RemoveHeader`, `HeaderRewrite` actions — can add/remove/modify any header on requests and responses |
| Rewrite templates | `crates/madhyamas-core/src/intercept/rewrite.rs:428-502` | `RewriteTemplates` with `http_to_https()`, `add_cors()`, `remove_security_headers()`, `add_auth_header()` — **no `no_caching()` template** |
| API templates | `crates/madhyamas-api/src/intercept_handlers.rs:879-924` | `get_rewrite_templates()` returns 4 templates — **no No Caching template** |
| Web UI templates | `web/src/features/tools/RewritesPanel.tsx` | Template list in the UI — **no No Caching template** |
| No-cache header usage | `crates/madhyamas-api/src/handlers.rs:456` | Only used on the certificate download endpoint, not as a feature |

### What needs to be done

This feature can be implemented **entirely as a rewrite template** — no new module needed. The existing rewrite infrastructure already supports all required header manipulations.

1. **Add `no_caching()` template** to `RewriteTemplates`
2. **Add template to API** `get_rewrite_templates()` response
3. **Add template to web UI** template list
4. **Document the template** and how to customize it

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/intercept/rewrite.rs` | Add `pub fn no_caching() -> RewriteRule` to `RewriteTemplates` impl |
| `crates/madhyamas-api/src/intercept_handlers.rs` | Add No Caching entry to `get_rewrite_templates()` JSON response |
| `web/src/features/tools/RewritesPanel.tsx` | Add "No Caching" to the templates list shown in the create dialog |

### How it should be done

**The No Caching template should create a rewrite rule that:**

On **requests** (remove conditional request headers):
- `RemoveHeader { name: "If-Modified-Since" }`
- `RemoveHeader { name: "If-None-Match" }`

On **responses** (remove caching headers, add no-cache directives):
- `RemoveHeader { name: "ETag" }`
- `RemoveHeader { name: "Last-Modified" }`
- `RemoveHeader { name: "Expires" }`
- `SetHeader { name: "Cache-Control", value: "no-cache, no-store, must-revalidate" }`
- `SetHeader { name: "Pragma", value: "no-cache" }`
- `SetHeader { name: "Expires", value: "0" }`

```rust
pub fn no_caching() -> RewriteRule {
    RewriteRule {
        name: "No Caching".to_string(),
        condition: MatchCondition::All,
        direction: RewriteDirection::Both,
        enabled: true,
        actions: vec![
            // Request: remove conditional request headers
            RewriteAction::RemoveHeader { name: "If-Modified-Since".to_string() },
            RewriteAction::RemoveHeader { name: "If-None-Match".to_string() },
            // Response: remove caching headers
            RewriteAction::RemoveHeader { name: "ETag".to_string() },
            RewriteAction::RemoveHeader { name: "Last-Modified".to_string() },
            RewriteAction::RemoveHeader { name: "Expires".to_string() },
            RewriteAction::SetHeader {
                name: "Cache-Control".to_string(),
                value: "no-cache, no-store, must-revalidate".to_string(),
            },
            RewriteAction::SetHeader {
                name: "Pragma".to_string(),
                value: "no-cache".to_string(),
            },
            RewriteAction::SetHeader {
                name: "Expires".to_string(),
                value: "0".to_string(),
            },
        ],
    }
}
```

**API template entry:**
```json
{
    "name": "No Caching",
    "description": "Prevent client caching by stripping cache-related headers and adding no-cache directives. Ensures you always see the latest version.",
    "template": { ... }
}
```

### How it would show up in the UI

- **Rewrites panel**: "No Caching" appears in the template picker when creating a new rewrite rule
- **One-click apply**: User selects the template, clicks "Create", and the rule is active immediately
- **Customizable**: After creating from the template, the user can edit individual actions (e.g., add specific hosts to the match condition)
- **CLI**: `madhyamas rewrites create --template no-caching`
- **MCP**: `madhyamas_create_rewrite` with template parameter `"no-caching"`
- **API**: `GET /api/rewrites/templates` includes the No Caching template

### How it can be tested

1. **Template test**: Create a rewrite from the No Caching template and verify all 8 actions are present
2. **Request test**: Make a request with `If-Modified-Since` header through the proxy; verify the header is stripped
3. **Response test**: Make a request to a server that returns `ETag` and `Last-Modified`; verify those headers are stripped and `Cache-Control: no-cache, no-store, must-revalidate` is added
4. **Browser test**: Load a page through the proxy with No Caching enabled; reload and verify the browser doesn't use a cached version
5. **Toggle test**: Disable the rule and verify caching headers pass through unchanged
6. **UI test**: Verify the template appears in the Rewrites panel template picker

### What needs to be documented

- Update `docs/MOCK_RESPONSES.md` or create `docs/REWRITE_TEMPLATES.md` — document all available rewrite templates including No Caching
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change No Caching row from 🔴 to ✅
- Update the madhyamas skill — add No Caching template to the rewrites workflow

---

## 7. Block Cookies Tool

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Rewrite header removal | `crates/madhyamas-core/src/intercept/rewrite.rs:269-271, 331-333` | `RewriteAction::RemoveHeader { name }` — can remove any header from both requests and responses |
| Rewrite templates | `crates/madhyamas-core/src/intercept/rewrite.rs:428-502` | 4 existing templates — **no `block_cookies()` template** |
| API templates | `crates/madhyamas-api/src/intercept_handlers.rs:879-924` | 4 templates in API — **no Block Cookies template** |
| Web UI | `web/src/features/tools/RewritesPanel.tsx` | Template list — **no Block Cookies template** |
| Cookie display | `web/src/features/traffic/TrafficDetail.tsx` | Cookies are displayed in the traffic detail view (read-only) |

### What needs to be done

Like No Caching, this feature can be implemented **entirely as a rewrite template**. The rewrite system already supports header removal.

1. **Add `block_cookies()` template** to `RewriteTemplates`
2. **Add template to API** response
3. **Add template to web UI** template list

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/intercept/rewrite.rs` | Add `pub fn block_cookies() -> RewriteRule` to `RewriteTemplates` impl |
| `crates/madhyamas-api/src/intercept_handlers.rs` | Add Block Cookies entry to `get_rewrite_templates()` JSON response |
| `web/src/features/tools/RewritesPanel.tsx` | Add "Block Cookies" to the templates list |

### How it should be done

**The Block Cookies template creates a rewrite rule that:**

On **requests** (prevent client from sending cookies):
- `RemoveHeader { name: "Cookie" }`

On **responses** (prevent server from setting cookies):
- `RemoveHeader { name: "Set-Cookie" }`

```rust
pub fn block_cookies() -> RewriteRule {
    RewriteRule {
        name: "Block Cookies".to_string(),
        condition: MatchCondition::All,
        direction: RewriteDirection::Both,
        enabled: true,
        actions: vec![
            RewriteAction::RemoveHeader { name: "Cookie".to_string() },
            RewriteAction::RemoveHeader { name: "Set-Cookie".to_string() },
        ],
    }
}
```

**Note:** `Set-Cookie` headers can appear multiple times in a response. The current `RemoveHeader` implementation uses `HashMap::remove()` which removes the key entirely. This is correct for blocking all cookies. If the user wants to block only specific cookies, they can customize the rule with `HeaderRewrite` using a regex pattern.

### How it would show up in the UI

- **Rewrites panel**: "Block Cookies" appears in the template picker
- **One-click apply**: Select template → Create → cookies are blocked immediately
- **Customizable**: User can restrict to specific hosts by editing the match condition
- **CLI**: `madhyamas rewrites create --template block-cookies`
- **MCP**: `madhyamas_create_rewrite` with template parameter `"block-cookies"`
- **API**: `GET /api/rewrites/templates` includes the Block Cookies template

### How it can be tested

1. **Template test**: Create a rewrite from the Block Cookies template; verify 2 actions (remove Cookie, remove Set-Cookie)
2. **Request test**: Make a request with a `Cookie: session=abc123` header; verify the header is stripped before reaching the server
3. **Response test**: Make a request to a server that returns `Set-Cookie`; verify the header is stripped before reaching the client
4. **Session test**: Verify that with Block Cookies enabled, websites that require cookies show you as logged out / new session
5. **Toggle test**: Disable the rule and verify cookies pass through unchanged
6. **UI test**: Verify the template appears in the Rewrites panel template picker

### What needs to be documented

- Update `docs/REWRITE_TEMPLATES.md` (or create it) — document the Block Cookies template
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Block Cookies row from ❌ to ✅
- Update the madhyamas skill — add Block Cookies template to the rewrites workflow

---

## Implementation Priority Order

Based on complexity, impact, and dependencies:

| Priority | Feature | Effort | Impact | Dependencies |
|---|---|---|---|---|
| 1 | **No Caching tool** | Small (template only) | Medium | None — uses existing rewrite infra |
| 2 | **Block Cookies tool** | Small (template only) | Medium | None — uses existing rewrite infra |
| 3 | **Block List tool** | Medium (new module) | High | None — follows existing intercept pattern |
| 4 | **Access Control** | Medium (new module) | High | `ipnet` crate |
| 5 | **External Proxy Chaining** | Medium-Hard | High | `reqwest` socks feature; UI already exists |
| 6 | **SOCKS Proxy** | Hard (new listener) | Medium | New SOCKS handshake implementation |
| 7 | **HTTP/2 Downstream** | Very Hard | Very High | `h2` crate; major engine changes; gRPC depends on it |

**Recommended approach:** Ship items 1-2 first (quick wins, no risk), then 3-4
(new modules following established patterns), then 5 (connect existing UI), then
6 (new protocol support), and finally 7 (the most complex but highest impact).

---

*Generated 2026-08-01. Based on codebase analysis as of this date.*
