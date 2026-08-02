# Low-Priority Feature Analysis — Detailed Implementation Plan

This document provides a detailed analysis of the 10 low-priority features
identified in [CHARLES_PROXY_FEATURE_COMPARISON.md](CHARLES_PROXY_FEATURE_COMPARISON.md)
(Section 4, "Lower Priority (niche / legacy)", items 16–25). For each feature
it documents:

- **What exists now** — current code, with file paths and line numbers
- **What needs to be done** — concrete work items
- **Where it needs to be done** — exact files to modify or create
- **How it should be done** — implementation approach and design decisions
- **How it would show up in the UI** — web UI, CLI, and API surface
- **How it can be tested** — verification strategy
- **What needs to be documented** — docs to create or update
- **Recommendation** — build, defer, or skip (with rationale)

> All file paths are relative to the repository root
> (`madhyamas/`).

These features are **low priority** because they are niche, legacy, or have
low demand relative to the high- and medium-priority items. Several are
**explicitly recommended to skip** (AMF/Flash, NTLM) because the underlying
technology is deprecated or the effort outweighs the value. Each section ends
with a clear recommendation so maintainers can decide what (if anything) to
invest in.

---

## Table of Contents

1. [Reverse Proxy](#1-reverse-proxy)
2. [Port Forwarding (TCP/UDP)](#2-port-forwarding-tcpudp)
3. [DNS Spoofing](#3-dns-spoofing)
4. [Protocol Buffers Full Decoder](#4-protocol-buffers-full-decoder)
5. [Validate (W3C HTML/CSS/Feed)](#5-validate-w3c-htmlcssfeed)
6. [AMF / Flash Remoting](#6-amf--flash-remoting)
7. [NTLM Authentication Pass-through](#7-ntlm-authentication-pass-through)
8. [Auto Browser/OS Proxy Configuration](#8-auto-browseros-proxy-configuration)
9. [Headless Mode](#9-headless-mode)
10. [Client Process Tracking](#10-client-process-tracking)
11. [Implementation Priority Order](#implementation-priority-order)

---

## 1. Reverse Proxy

### What exists now

Madhyamas is a **forward proxy only** — clients send requests to the proxy,
which forwards them to upstream servers. There is no reverse-proxy mode
where the proxy listens as if it were the origin server and forwards to a
real backend.

| Aspect | Location | Current State |
|---|---|---|
| Proxy listener | `crates/madhyamas-core/src/proxy/engine.rs:357-459` | `start()` binds a single `TcpListener` on `config.proxy_addr()` (default `127.0.0.1:8888`) and runs an accept loop that dispatches `CONNECT` vs. plain HTTP |
| Connection detection | `crates/madhyamas-core/src/proxy/engine.rs:462-507` | `handle_connection()` peeks the first 1024 bytes; routes `CONNECT` to `handle_https_tunnel()`, everything else to `handle_http_proxy()` — both assume the client is sending proxy-style absolute-URI requests |
| HTTP handler | `crates/madhyamas-core/src/proxy/engine.rs` (`handle_http_proxy`) | Expects absolute-form URLs (`GET http://host/path HTTP/1.1`); does not handle origin-form (`GET /path HTTP/1.1`) which is what a reverse proxy receives |
| Config | `crates/madhyamas-core/src/config.rs:17-196` | No `reverse_proxy`, `listen_as`, or backend-mapping fields in `ProxyConfig` |
| CLI | `crates/madhyamas/src/main.rs:37-214` | No `--reverse-proxy` or `--map-host` flags |
| Search results | — | No `reverse_proxy`, `ReverseProxy`, or `listen_as` references anywhere in the Rust backend (only mentioned in `docs/CHARLES_PROXY_FEATURE_COMPARISON.md:54`) |

### What needs to be done

1. **Add a reverse-proxy listener mode** that accepts origin-form requests
   (`GET /path`) and forwards them to a configured backend
2. **Add host→backend mapping config** (one or more virtual hosts, each
   mapping to an upstream origin)
3. **Reuse the existing `Pipeline`** for interception (rewrites, mocks,
   breakpoints, throttle) so reverse-proxied traffic is inspectable
4. **Handle TLS termination** at the proxy (the proxy presents a cert for
   the virtual host and forwards to the backend over HTTP or HTTPS)
5. **Add CLI flags** and **API config** for managing reverse-proxy mappings
6. **Add web UI** for managing mappings

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/config.rs` | Add `ReverseProxyConfig` struct with `enabled`, `listen_port`, `entries: Vec<ReverseProxyEntry>` (each entry: `host`, `backend_url`, `tls.enabled`, `tls.cert_path`); add field to `ProxyConfig` |
| `crates/madhyamas-core/src/proxy/reverse.rs` | **New file** — `ReverseProxyListener` that binds a separate port, accepts origin-form requests, resolves the backend from the `Host` header, and forwards via the shared `Pipeline` |
| `crates/madhyamas-core/src/proxy/mod.rs` | Add `pub mod reverse;` |
| `crates/madhyamas-core/src/proxy/engine.rs` | In `start()` (line 357), optionally spawn the reverse-proxy listener alongside the HTTP and SOCKS listeners |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | Add an origin-form request parser path (currently `parse_http_request` expects absolute-form); or normalize origin-form to absolute-form using the `Host` header before entering the pipeline |
| `crates/madhyamas-api/src/handlers.rs` | Include reverse-proxy config in `GET/PATCH /api/config` |
| `crates/madhyamas/src/main.rs` | Add `--reverse-proxy-enabled`, `--reverse-proxy-port`, `--reverse-proxy-map <host=backend>` (repeatable) CLI flags |
| `web/src/features/config/ConfigDialog.tsx` | Add a "Reverse Proxy" tab with host→backend mapping editor |

### How it should be done

**Design decision: Separate listener port vs. same-port detection**

- **Recommended: Separate port** (e.g., 8080 or 8443). Reverse-proxy traffic
  uses origin-form URLs and has no `CONNECT`; mixing it with forward-proxy
  traffic on the same port requires Host-header heuristics. A separate port
  is unambiguous and matches Charles behavior.

**Request flow:**
1. Client connects to the reverse-proxy port and sends `GET /path HTTP/1.1`
   with a `Host: api.example.com` header
2. The listener looks up `api.example.com` in the mapping table →
   `http://backend.internal:3000`
3. The request is rewritten to `http://backend.internal:3000/path` and run
   through the existing `Pipeline` (so rewrites/mocks/breakpoints apply)
4. The response is returned to the client with the original `Host` header
   preserved

**TLS termination:** When an entry has `tls.enabled`, the listener performs
a TLS handshake using a cert for the virtual host (from `CertificateManager`)
before reading the HTTP request. This lets the proxy MITM traffic for
services that can't be reconfigured to use an HTTP proxy.

**Why it's low priority:** Most clients that need debugging can be
configured to use an HTTP/SOCKS proxy. Reverse proxy is only needed for
clients that hardcode a server address and can't be pointed at a proxy
(e.g., some mobile apps, embedded devices).

### How it would show up in the UI

- **Config dialog**: New "Reverse Proxy" tab with a table of host→backend
  mappings, TLS toggle per entry, and a listen-port field
- **CLI**: `madhyamas serve --reverse-proxy-enabled --reverse-proxy-port 8443 --reverse-proxy-map api.example.com=http://backend:3000`
- **API**: `GET /api/config` returns `reverse_proxy` object; `PATCH /api/config` updates it
- **Traffic list**: Reverse-proxied entries show the original `Host` and the
  resolved backend in the detail view

### How it can be tested

1. **Mapping test**: Configure `api.example.com → http://localhost:3000`,
   send `curl -H "Host: api.example.com" http://localhost:8443/users`, verify
   the request reaches `localhost:3000/users` and is captured
2. **TLS test**: Enable TLS for an entry, send `curl -k https://localhost:8443/`
   with the right `Host`, verify MITM interception works
3. **Pipeline test**: Add a rewrite rule and verify it applies to
   reverse-proxied traffic
4. **Multi-host test**: Configure two mappings and verify requests route
   correctly based on `Host`
5. **Fallback test**: Request a host with no mapping → return 502

### What needs to be documented

- Update `CLAUDE.md` — add reverse-proxy config fields and CLI flags
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Reverse proxy row from ❌ to ✅
- Create `docs/REVERSE_PROXY.md` — setup guide, host mapping, TLS termination, use cases

### Recommendation

**Defer.** Niche feature; the forward proxy + SOCKS covers the vast majority
of debugging scenarios. Build only if a concrete use case (e.g., debugging a
mobile app that hardcodes a server URL) emerges. Effort: Medium-Hard.

---

## 2. Port Forwarding (TCP/UDP)

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| TCP listener | `crates/madhyamas-core/src/proxy/engine.rs:357-459` | Single `TcpListener` for the HTTP proxy; SOCKS5 listener spawned separately when `enable_socks` is true (line 382-398) |
| UDP support | — | **No UDP listener anywhere**; the entire codebase is TCP-based (hyper, tokio TCP streams) |
| Passthrough tunnel | `crates/madhyamas-core/src/proxy/engine.rs` (`handle_passthrough_tunnel`) | Blind TCP relay between client and upstream — the closest existing analog to port forwarding, but it's triggered by CONNECT, not a static port mapping |
| Config | `crates/madhyamas-core/src/config.rs:17-196` | No `port_forwarding` or `forward_rules` fields |
| CLI | `crates/madhyamas/src/main.rs:37-214` | No `--forward-port` flags |
| Docs | `docs/DEPLOYMENT.md:844-845` | Mentioned only as a future possibility; no implementation |

### What needs to be done

1. **Add a port-forwarding config** with a list of rules (listen port →
   target host:port, protocol TCP or UDP)
2. **Implement TCP port forwarding** — bind a listener, accept connections,
   connect to the target, and blind-relay (reuse the passthrough tunnel
   logic)
3. **Implement UDP port forwarding** (optional) — bind a UDP socket, relay
   datagrams to the target
4. **Record forwarded connections** as traffic entries (like SOCKS
   passthrough entries) so they're visible in the web UI
5. **Add CLI flags** and **API config**

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/config.rs` | Add `PortForwardingConfig { enabled, rules: Vec<PortForwardRule> }` where `PortForwardRule { listen_port, target_host, target_port, protocol: "tcp"\|"udp" }`; add to `ProxyConfig` |
| `crates/madhyamas-core/src/proxy/port_forward.rs` | **New file** — `PortForwarder` with `serve_tcp_rule()` and `serve_udp_rule()`; TCP path reuses the bidirectional copy from `handle_passthrough_tunnel` |
| `crates/madhyamas-core/src/proxy/mod.rs` | Add `pub mod port_forward;` |
| `crates/madhyamas-core/src/proxy/engine.rs` | In `start()` (line 357), spawn one listener task per forwarding rule |
| `crates/madhyamas-api/src/handlers.rs` | Include port-forwarding config in `GET/PATCH /api/config` |
| `crates/madhyamas/src/main.rs` | Add `--forward <listen_port=target_host:target_port>` (repeatable) and `--forward-protocol <tcp\|udp>` flags |
| `web/src/features/config/ConfigDialog.tsx` | Add a "Port Forwarding" tab with a rule table |

### How it should be done

**TCP forwarding** is straightforward — it's a simplified version of the
existing passthrough tunnel without the CONNECT handshake:

```rust
// In port_forward.rs
async fn serve_tcp_rule(rule: PortForwardRule, traffic_store: Arc<TrafficStore>) {
    let listener = TcpListener::bind(("0.0.0.0", rule.listen_port)).await?;
    loop {
        let (client, addr) = listener.accept().await?;
        let target = format!("{}:{}", rule.target_host, rule.target_port);
        // Record a passthrough-style traffic entry
        // Connect to target and bidirectionally copy (reuse tokio::io::copy)
        tokio::spawn(async move { relay_tcp(client, target).await });
    }
}
```

**UDP forwarding** is more involved: bind a `UdpSocket`, maintain a mapping
of client addresses → upstream sockets, and relay datagrams both ways. UDP
has no connection state, so the mapping must expire after an idle timeout.

**Traffic recording:** Forwarded connections are recorded as
passthrough-style entries with `http_version: "FORWARD"` (similar to how
SOCKS entries use `"SOCKS5"`), showing the listen port and target.

**Why it's low priority:** SOCKS5 (already implemented) covers most
port-forwarding use cases — clients can use `curl --socks5` or SSH dynamic
forwarding. Static port forwarding is only needed for clients that can't
speak SOCKS.

### How it would show up in the UI

- **Config dialog**: "Port Forwarding" tab with a rule table (listen port,
  target, protocol, enable toggle)
- **CLI**: `madhyamas serve --forward 8080=backend.internal:80 --forward 53=dns.internal:53 --forward-protocol udp`
- **API**: `GET /api/config` returns `port_forwarding` object
- **Traffic list**: Forwarded connections appear with a "FORWARD" proto
  label and the target address

### How it can be tested

1. **TCP test**: Forward `localhost:9090 → example.com:80`, send
   `curl http://localhost:9090/`, verify the request reaches example.com
2. **UDP test**: Forward a DNS port, send a DNS query via `dig`, verify the
   response comes back
3. **Idle timeout test** (UDP): Verify idle client mappings expire
4. **Traffic visibility test**: Verify forwarded connections appear in the
   web UI
5. **Multi-rule test**: Configure multiple rules and verify each works
   independently

### What needs to be documented

- Update `CLAUDE.md` — add port-forwarding config and CLI flags
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Port forwarding row from ❌ to ✅
- Create `docs/PORT_FORWARDING.md` — setup guide, TCP vs. UDP, use cases

### Recommendation

**Defer (TCP) / Skip (UDP).** TCP forwarding is low-effort but largely
redundant with SOCKS5. UDP forwarding adds a non-trivial amount of
state-management code for a rarely-needed capability. Build TCP forwarding
only if users request it; skip UDP unless there's a concrete demand.
Effort: Small (TCP), Medium (UDP).

---

## 3. DNS Spoofing

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| DNS handling | — | **No DNS resolution code in the proxy**; the proxy forwards to the host in the request URL or CONNECT target, relying on the OS resolver via `reqwest`/`TcpStream::connect` |
| Intercept modules | `crates/madhyamas-core/src/intercept/` | `block_list.rs`, `breakpoint.rs`, `handler.rs`, `mock.rs`, `regex_cache.rs`, `rewrite.rs`, `throttle.rs`, `types.rs` — **no `dns.rs`** |
| Config | `crates/madhyamas-core/src/config.rs:17-196` | No `dns_spoofing` or `host_overrides` fields |
| Rewrite | `crates/madhyamas-core/src/intercept/rewrite.rs:66-98` | `RewriteAction` can rewrite URLs and headers, but this happens *after* DNS resolution would normally occur — it's not true DNS spoofing |
| Search results | — | `dns_spoof`/`DnsSpoof` appear only in `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` (lines 54, 352) |

### What needs to be done

1. **Add a host-override mapping** (hostname → IP) that the proxy consults
   before connecting to any upstream
2. **Apply the override** in three places: `reqwest` upstream forwarding,
   `handle_https_tunnel` CONNECT target, and `handle_passthrough_tunnel`
3. **Optionally run a DNS server** (UDP/53) that returns spoofed records
   for clients configured to use the proxy as their DNS resolver
4. **Add CLI flags** and **API config**

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/config.rs` | Add `DnsSpoofingConfig { enabled, entries: Vec<DnsSpoofEntry> }` where `DnsSpoofEntry { hostname, ip_address, record_type: "A"\|"AAAA" }`; add to `ProxyConfig` |
| `crates/madhyamas-core/src/dns.rs` | **New file** — `DnsOverrideTable` with `resolve(host) -> Option<IpAddr>`; optional `DnsServer` that listens on UDP/53 and answers from the table |
| `crates/madhyamas-core/src/lib.rs` | Export `DnsOverrideTable` |
| `crates/madhyamas-core/src/proxy/engine.rs` | Before any `TcpStream::connect` or `reqwest` call, resolve the host through `DnsOverrideTable` first; connect to the override IP but preserve the original `Host` header |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | Apply the override in `forward_via_reqwest()` (resolve host before sending) |
| `crates/madhyamas-api/src/handlers.rs` | Include DNS spoofing config in `GET/PATCH /api/config` |
| `crates/madhyamas/src/main.rs` | Add `--spoof-dns <hostname=ip>` (repeatable) and `--dns-server-port <port>` flags |
| `web/src/features/config/ConfigDialog.tsx` | Add a "DNS Spoofing" tab with a hostname→IP table |

### How it should be done

**Two layers, both useful:**

1. **Host-override table (proxy-side):** Before the proxy connects to any
   upstream, it checks the override table. If `api.example.com → 10.0.0.5`
   is configured, the proxy connects to `10.0.0.5` but sends
   `Host: api.example.com`. This works for all traffic through the proxy
   without any client DNS configuration. This is the high-value, low-effort
   part.

2. **DNS server (client-side, optional):** A UDP DNS server on port 53 that
   answers from the override table. This is only needed for clients that
   resolve DNS themselves and connect directly (not through the proxy).
   Charles only supports this on Android via a VPN service.

**Implementation approach for the host-override:**
```rust
// In engine.rs, before connecting:
let target_ip = if let Some(override_ip) = self.dns_overrides.resolve(host) {
    info!("DNS spoof: {} -> {}", host, override_ip);
    override_ip
} else {
    // Normal DNS resolution via tokio::net::lookup_host
    resolve_host(host).await?
};
let stream = TcpStream::connect((target_ip, port)).await?;
// Preserve the original Host header in the forwarded request
```

**Why it's low priority:** The host-override layer is essentially a
specialized rewrite rule (redirect a host to a different IP). Users can
already achieve this with `/etc/hosts` or a rewrite rule. The DNS server
layer is platform-specific and Charles only ships it for Android.

### How it would show up in the UI

- **Config dialog**: "DNS Spoofing" tab with a hostname→IP table, record
  type selector, and an optional DNS server port
- **CLI**: `madhyamas serve --spoof-dns api.example.com=10.0.0.5 --spoof-dns staging.example.com=192.168.1.10`
- **API**: `GET /api/config` returns `dns_spoofing` object
- **Traffic detail**: Show a "DNS Spoofed" badge when a request was routed
  to an override IP

### How it can be tested

1. **Override test**: Spoof `example.com → 127.0.0.1`, run a local server
   on port 80, send `curl -x localhost:8888 http://example.com/`, verify
   the request hits the local server
2. **Host header test**: Verify the `Host` header sent upstream is the
   original hostname, not the override IP
3. **DNS server test**: Start the DNS server, configure a client to use it,
   verify `dig @localhost api.example.com` returns the spoofed IP
4. **Passthrough test**: Verify spoofing works for SSL passthrough
   connections
5. **Toggle test**: Disable spoofing and verify normal DNS resolution resumes

### What needs to be documented

- Update `CLAUDE.md` — add DNS spoofing config and CLI flags
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change DNS Spoofing row from ❌ to ✅
- Create `docs/DNS_SPOOFING.md` — setup guide, host override vs. DNS server, use cases (testing staging endpoints, local development)

### Recommendation

**Build the host-override layer only.** It's low-effort, high-value for
local development (point `api.example.com` at a local server without
editing `/etc/hosts`), and works for all proxy traffic. Skip the DNS server
— it's platform-specific and redundant with the override layer for proxy
users. Effort: Small (override) / Medium (DNS server, skip).

---

## 4. Protocol Buffers Full Decoder

### What exists now

The gRPC module has a **basic schema-less protobuf decoder** that parses
wire-format fields (varint, fixed64, fixed32, length-delimited) but cannot
map field numbers to names or fully decode messages without a `.proto` /
`.desc` descriptor file.

| Aspect | Location | Current State |
|---|---|---|
| Proto types | `crates/madhyamas-core/src/grpc/types.rs:94-130` | `ProtoMessage { message_type, fields, json }`, `ProtoField { number, wire_type, name, value }`, `ProtoValue` enum (Varint/Fixed64/Fixed32/Bytes/String/LengthDelimited/Group/Nested) — `name` is always `None` (no schema) |
| Basic decoder | `crates/madhyamas-core/src/grpc/frame.rs:130-161` | `decode_protobuf()` — parses wire format, heuristically tries UTF-8 then nested-message for length-delimited fields; **no field names, no type-aware decoding** |
| Varint reader | `crates/madhyamas-core/src/grpc/frame.rs:164-192` | `read_varint()` — correct varint parsing |
| Field value reader | `crates/madhyamas-core/src/grpc/frame.rs:195-250` | `read_field_value()` — handles all 6 wire types; groups are skipped (deprecated) |
| JSON conversion | `crates/madhyamas-core/src/grpc/frame.rs:252-276` | `proto_to_json()` — produces JSON with `field_N` keys when names are unknown |
| Frame storage | `crates/madhyamas-core/src/grpc/types.rs:42-61` | `GrpcFrame.decoded: Option<ProtoMessage>` — populated with schema-less decode; `json` is always `None` |
| Service descriptors | `crates/madhyamas-core/src/grpc/interceptor.rs:389-405` | `GrpcServiceDescriptor` and `GrpcMethodDescriptor` structs **exist but are never populated or used** — no code loads `.proto`/`.desc` files |
| gRPC manager | `crates/madhyamas-core/src/grpc/interceptor.rs:10-24` | `GrpcManager` — tracks connections/streams/frames; `record_frame()` stores raw base64 data but does not call `decode_protobuf()` |
| Web UI | `web/src/features/tools/GrpcPanel.tsx` | Shows raw frame data (base64); no decoded field view |
| Feature gate | `crates/madhyamas-core/Cargo.toml` | `grpc` feature; gRPC module is feature-gated |

### What needs to be done

1. **Wire `decode_protobuf()` into `GrpcManager::record_frame()`** so
   frames are decoded at capture time (currently the `decoded` field is
   always `None`)
2. **Add descriptor file loading** — parse `.proto` (via `prost-reflect` or
   `prost-build`) or binary `.desc` (FileDescriptorSet) files
3. **Map field numbers to names** using the loaded descriptor for the
   service/method
4. **Type-aware decoding** — decode `int32`, `string`, `bool`, `enum`,
   `message`, `repeated` correctly (currently everything is varint/bytes)
5. **Add an API endpoint** to upload/manage descriptor files
6. **Update the web UI** to show decoded fields with names and types
7. **Auto-fetch descriptors** from gRPC server reflection (optional but
   high-value — `grpc.reflection.v1alpha.ServerReflection`)

### Where it needs to be done

| File | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `prost-reflect = "0.14"` (dynamic protobuf descriptor support) |
| `crates/madhyamas-core/Cargo.toml` | Add `prost-reflect` dependency under the `grpc` feature |
| `crates/madhyamas-core/src/grpc/decoder.rs` | **New file** — `ProtobufDecoder` that holds a `prost_reflect::DescriptorPool` and decodes messages against loaded descriptors; falls back to schema-less `decode_protobuf()` when no descriptor is available |
| `crates/madhyamas-core/src/grpc/mod.rs` | Add `pub mod decoder;` and re-export |
| `crates/madhyamas-core/src/grpc/interceptor.rs` | In `record_frame()` (line 122), call the decoder to populate `frame.decoded`; add a `descriptor_pool: RwLock<DescriptorPool>` to `GrpcManager` with `load_descriptor()` and `register_service()` methods |
| `crates/madhyamas-core/src/grpc/types.rs` | Populate `ProtoField.name` and `ProtoMessage.json` when a descriptor is available |
| `crates/madhyamas-api/src/phase3_handlers.rs` | Add `POST /api/grpc/descriptors` (upload `.desc` file), `GET /api/grpc/descriptors`, `DELETE /api/grpc/descriptors/{name}` |
| `crates/madhyamas-api/src/routes.rs` | Add the descriptor routes |
| `web/src/features/tools/GrpcPanel.tsx` | Add a decoded-field tree view (field name, type, value) alongside the raw hex/base64 view; add a descriptor upload button |
| `web/src/lib/api/phase3.ts` | Add `useGrpcDescriptors()`, `useUploadDescriptor()` hooks |

### How it should be done

**Phase 1 — Wire up the existing schema-less decoder (quick win):**
1. Call `decode_protobuf()` in `GrpcManager::record_frame()` and store the
   result in `frame.decoded`
2. Update the web UI to show the decoded field tree (field numbers + wire
   types + heuristic string/nested values)
3. This is independently shippable and immediately useful — users can see
   field structure even without descriptors

**Phase 2 — Descriptor-based decoding:**
1. Use `prost-reflect` to parse binary `FileDescriptorSet` (`.desc`) files
   uploaded by the user
2. Build a `DescriptorPool` and look up the message type for each gRPC
   method (from the service descriptor)
3. Decode each frame against the message descriptor, populating field
   names, types, and producing a proper JSON representation
4. Store the descriptor pool in `GrpcManager` and associate descriptors
   with service names

**Phase 3 — Server reflection (optional):**
1. Implement a gRPC reflection client that calls
   `grpc.reflection.v1alpha.ServerReflection` on the target server
2. Auto-fetch and cache descriptors when a new gRPC connection is detected
3. This eliminates the need for users to manually upload `.desc` files

**Key design decisions:**
- Use `prost-reflect` (not `prost-build`) — `prost-build` is a build-time
  tool; `prost-reflect` provides runtime descriptor parsing
- Always fall back to schema-less decoding when no descriptor is available
- Cache decoded JSON in `ProtoMessage.json` to avoid re-decoding on every
  API request

### How it would show up in the UI

- **gRPC panel**: Each frame shows a "Decoded" tab with a field tree
  (field name, type, value) alongside the "Raw" tab (hex/base64)
- **Descriptor management**: Upload button for `.desc` files; list of
  loaded descriptors with service/method names
- **API**: `GET /api/grpc/frames/{id}` includes `decoded` with named
  fields; `POST /api/grpc/descriptors` uploads a descriptor
- **CLI**: `madhyamas grpc descriptors list`, `madhyamas grpc descriptors upload <file>`
- **MCP**: `madhyamas_grpc_decode_frame` tool

### How it can be tested

1. **Schema-less test**: Capture a gRPC call without descriptors, verify
   the decoded field tree shows field numbers and wire types
2. **Descriptor test**: Upload a `.desc` file for a known service, capture
   a call, verify field names and types are correct
3. **JSON test**: Verify `ProtoMessage.json` produces correct JSON for
   known message types
4. **Reflection test** (Phase 3): Connect to a gRPC server with reflection
   enabled, verify descriptors are auto-fetched
5. **Fallback test**: Remove a descriptor and verify decoding falls back
   to schema-less
6. **Nested message test**: Verify nested messages decode recursively

### What needs to be documented

- Update `CLAUDE.md` — note the protobuf decoder and descriptor support
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Protocol
  Buffers viewer row from 🟡 to ✅
- Create `docs/GRPC_DECODING.md` — guide on uploading descriptors, reading
  decoded frames, using reflection

### Recommendation

**Build Phase 1 now (quick win), Phase 2 when gRPC debugging demand grows.**
Wiring up the existing schema-less decoder is a small change with immediate
value — it makes the gRPC panel actually useful instead of showing raw
base64. Descriptor support is more effort and only needed by users doing
serious gRPC debugging. Effort: Small (Phase 1) / Medium (Phase 2) /
Medium-Hard (Phase 3).

---

## 5. Validate (W3C HTML/CSS/Feed)

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Validation module | `crates/madhyamas-api/src/validation.rs` | **Input validation for API requests only** (validates mock/rewrite/breakpoint payloads) — not W3C content validation |
| Response bodies | `crates/madhyamas-core/src/traffic/types.rs:121-156` | `ResponseData.body` stores raw response bytes; no validation is performed |
| Web UI body viewer | `web/src/features/traffic/TrafficDetail.tsx` | Renders response bodies (JSON/XML/HTML/form/binary) — no validation feedback |
| Search results | — | `w3c`/`W3C`/`validator` appear only in `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` (lines 171, 280, 354); no W3C validation code anywhere |

### What needs to be done

1. **Add a "Validate" action** on captured responses that sends the body to
   a W3C validator
2. **Support HTML, CSS, and Feed (RSS/Atom) validation**
3. **Store validation results** alongside the traffic entry
4. **Show results in the web UI** with error/warning listings and line
   numbers
5. **Add API endpoints** and **CLI/MCP commands**

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/validate.rs` | **New file** — `Validator` with `validate_html(body) -> ValidationResult`, `validate_css(body)`, `validate_feed(body)`; uses the W3C public validator API (`https://validator.w3.org/nu/`) or a bundled validator |
| `crates/madhyamas-core/src/lib.rs` | Export `Validator`, `ValidationResult` |
| `crates/madhyamas-core/src/traffic/types.rs` | Add `validation: Option<ValidationResult>` to `TrafficEntry` (or store separately to avoid bloating every entry) |
| `crates/madhyamas-api/src/routes.rs` | Add `POST /api/traffic/{id}/validate` and `GET /api/traffic/{id}/validation` |
| `crates/madhyamas-api/src/handlers.rs` | Add `validate_response` handler — fetches the response body, runs the validator, stores the result |
| `crates/madhyamas-cli/src/commands/validate.rs` | **New file** — `madhyamas validate <traffic_id>` CLI command |
| `crates/madhyamas-mcp/src/tools/validate.rs` | **New file** — `madhyamas_validate_response` MCP tool |
| `web/src/features/traffic/TrafficDetail.tsx` | Add a "Validation" tab showing errors/warnings with line numbers; add a "Validate" button |
| `web/src/lib/api/traffic.ts` | Add `useValidateResponse()`, `useValidationResult()` hooks |

### How it should be done

**Two approaches:**

1. **Remote validation (recommended):** Send the response body to the W3C
   public validator (`https://validator.w3.org/nu/` for HTML, the CSS
   validator, and the feed validator). This requires no bundled code and
   always uses the latest validation rules. Downside: requires internet
   access and sends response bodies to a third party (privacy concern for
   internal apps).

2. **Local validation (optional):** Bundle a validator library. For HTML,
   `html5ever` (already a transitive dep via the web ecosystem) can do
   well-formedness checks but not full W3C conformance. For CSS, there's
   no mature Rust CSS validator. Local validation would be partial.

**Recommended: Remote validation with a configurable endpoint** (so users
can point at a self-hosted validator instance like
[html5validator](https://github.com/svenkreiss/html5validator) for privacy).

**Validation result structure:**
```rust
pub struct ValidationResult {
    pub validator: String,       // "w3c-nu", "w3c-css", "w3c-feed"
    pub content_type: String,    // "html", "css", "feed"
    pub is_valid: bool,
    pub errors: Vec<ValidationMessage>,
    pub warnings: Vec<ValidationMessage>,
    pub validated_at: DateTime<Utc>,
}

pub struct ValidationMessage {
    pub line: u32,
    pub column: u32,
    pub message: String,
    pub message_type: String,    // "error", "warning", "info"
    pub extract: Option<String>, // surrounding code context
}
```

**Why it's low priority:** Developers can already copy a response body into
the W3C validator manually. Charles's Validate tool is a convenience
feature, not a core debugging capability. The privacy concern (sending
response bodies to a third party) also limits its appeal for internal apps.

### How it would show up in the UI

- **Traffic detail**: New "Validation" tab with a "Validate" button; after
  validation, shows a list of errors/warnings with line/column, clickable
  to jump to the relevant body line
- **CLI**: `madhyamas validate <traffic_id> --type html|css|feed`
- **MCP**: `madhyamas_validate_response(traffic_id, content_type)`
- **API**: `POST /api/traffic/{id}/validate` triggers validation;
  `GET /api/traffic/{id}/validation` returns cached results

### How it can be tested

1. **HTML test**: Capture a response with invalid HTML, validate, verify
   errors are reported with correct line numbers
2. **CSS test**: Capture a CSS response, validate, verify CSS errors
3. **Feed test**: Capture an RSS feed, validate, verify feed errors
4. **Valid content test**: Validate a well-formed response, verify
   `is_valid: true` with no errors
5. **Privacy test**: Configure a self-hosted validator endpoint, verify
   requests go to the configured endpoint, not the public W3C service
6. **Caching test**: Re-validate the same entry, verify cached results are
   returned without re-sending

### What needs to be documented

- Update `CLAUDE.md` — add Validate to the API endpoints table
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Validate row from ❌ to ✅
- Create `docs/VALIDATE.md` — guide on validating responses, configuring a self-hosted validator, privacy considerations

### Recommendation

**Defer.** Niche convenience feature; users can validate externally. Build
only if there's demand from users who frequently validate HTML/CSS/feeds
during debugging. Effort: Medium (remote) / Hard (local).

---

## 6. AMF / Flash Remoting

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| AMF parsing | — | **No AMF code anywhere** in the codebase |
| Flash support | — | Not referenced outside `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` (lines 59, 181, 355) |
| Binary body viewers | `web/src/features/traffic/TrafficDetail.tsx` | Hex/binary viewer for non-text bodies; no AMF-specific rendering |

### What needs to be done

AMF (Action Message Format) is a binary serialization format used by
Adobe Flash Remoting. To implement it:

1. **Add an AMF parser** that decodes AMF0 and AMF3 binary bodies into
   structured data
2. **Detect AMF bodies** by `Content-Type: application/x-amf`
3. **Render decoded AMF** in the web UI body viewer
4. **Add AMF message inspection** (headers, bodies, targets)

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/amf.rs` | **New file** — AMF0/AMF3 binary parser |
| `web/src/features/traffic/AmfViewer.tsx` | **New file** — AMF tree viewer component |
| `web/src/features/traffic/TrafficDetail.tsx` | Detect AMF content-type and render `AmfViewer` |

### How it should be done

**Skip.** Flash was officially end-of-lifed on December 31, 2020. Adobe
blocked Flash content from running in January 2021. AMF/Flash Remoting
traffic is effectively nonexistent in modern applications. Charles
maintains AMF support for legacy compatibility, but a new debugging proxy
in 2026 has no reason to invest in a dead format.

If a user ever encounters AMF traffic, the existing hex/binary viewer is
sufficient to inspect the raw bytes.

### Recommendation

**Skip.** Flash is deprecated and end-of-lifed. AMF traffic is effectively
nonexistent. The hex viewer is sufficient for the rare legacy case.
Effort: N/A (not recommended).

---

## 7. NTLM Authentication Pass-through

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Upstream proxy auth | `crates/madhyamas-core/src/config.rs:224-257` | `UpstreamProxyConfig` supports Basic auth (`auth_username`/`auth_password`) for HTTP/HTTPS/SOCKS5 upstream proxies — **no NTLM** |
| Proxy auth to clients | `crates/madhyamas-api/src/middleware.rs` | JWT-based auth for the API server (enterprise feature) — not NTLM |
| HTTP auth handling | `crates/madhyamas-core/src/proxy/pipeline.rs` (`forward_via_reqwest`) | `reqwest` handles 401/407 challenges with Basic auth only; no NTLM negotiation |
| Search results | — | `ntlm`/`NTLM` appears only in `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` (lines 60, 83, 234, 356) and `docs/UPSTREAM_PROXY.md:319-320` (noting it's unsupported) |

### What needs to be done

NTLM is a Microsoft proprietary authentication protocol used in some
corporate environments. To support it:

1. **Implement NTLM handshake** (Type 1 Negotiate, Type 2 Challenge,
   Type 3 Authenticate) for upstream proxy authentication
2. **Handle NTLM for upstream HTTP proxies** — respond to
   `Proxy-Authenticate: NTLM` challenges
3. **Handle NTLM for target server authentication** — respond to
   `WWW-Authenticate: NTLM` challenges
4. **Add config** for NTLM credentials (domain, username, password)

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/auth/ntlm.rs` | **New file** — NTLM handshake implementation (Type 1/2/3 message encoding/decoding) |
| `crates/madhyamas-core/src/config.rs` | Add NTLM credential fields to `UpstreamProxyConfig` (`ntlm_domain`, `ntlm_workstation`) |
| `crates/madhyamas-core/src/proxy/engine.rs` | In the upstream proxy CONNECT path, handle NTLM challenge/response |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | In `forward_via_reqwest()`, handle `WWW-Authenticate: NTLM` from target servers |

### How it should be done

**Skip.** NTLM is a legacy protocol that Microsoft has deprecated in favor
of Kerberos and Negotiate (SPNEGO). Modern corporate environments use
Kerberos or Basic auth over HTTPS. NTLM support requires implementing a
complex, poorly-documented challenge/response protocol with MD4/DES
cryptography (NTLMv2 adds HMAC-MD5), and `reqwest` doesn't support NTLM
natively.

If a user is behind an NTLM-only corporate proxy, they should use a
dedicated NTLM-to-Basic bridge (e.g., `cntlm` or `px`) in front of
Madhyamas, chaining Madhyamas through that bridge via the existing
upstream proxy support.

### Recommendation

**Skip.** Legacy protocol, low demand, complex to implement correctly, and
workarounds exist (NTLM bridge + upstream proxy chaining). Effort: N/A
(not recommended).

---

## 8. Auto Browser/OS Proxy Configuration

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Proxy configuration | `crates/madhyamas-core/src/proxy/engine.rs:357-368` | Binds a listener; clients must be manually configured to use it |
| System proxy detection | — | **No code to detect or modify OS/browser proxy settings** |
| Onboarding | `web/src/features/onboarding/` | Web UI shows the proxy address and instructions for manual configuration |
| Search results | — | `system_proxy`/`auto.?config`/`wpad` appear only in `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` (lines 56, 81, 217) and `docs/UPSTREAM_PROXY.md:317` |

### What needs to be done

Charles can automatically configure the OS and browser proxy settings on
startup and restore them on exit. To replicate:

1. **macOS**: Set the system proxy via `networksetup -setwebproxy` and
   `-setsecurewebproxy`
2. **Windows**: Set the proxy via the registry
   (`HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings`)
   or `netsh winhttp set proxy`
3. **Linux**: Set GNOME proxy settings via `gsettings` or KDE via
   `kwriteconfig5`
4. **Restore on exit**: Save the original settings and restore them on
   shutdown (signal handler / drop)
5. **PAC/WPAD** (optional): Serve a PAC file or respond to WPAD so clients
   auto-discover the proxy

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/system_proxy.rs` | **New file** — platform-specific proxy configuration (`set_proxy()`, `restore_proxy()`) using `std::process::Command` to call OS tools |
| `crates/madhyamas-core/src/lib.rs` | Export `SystemProxyConfig` |
| `crates/madhyamas/src/main.rs` | Add `--auto-config-proxy` flag; on startup, set the system proxy; on shutdown (ctrl-c / drop), restore |
| `crates/madhyamas-api/src/handlers.rs` | Add `POST /api/system-proxy/enable` and `POST /api/system-proxy/disable` endpoints |
| `web/src/features/onboarding/OnboardingWizard.tsx` | Add an "Auto-configure system proxy" button |

### How it should be done

**Skip the auto-configuration; document manual setup instead.**

Auto-configuring OS proxy settings is:
- **Platform-specific** — requires different code for macOS, Windows, Linux
  (GNOME, KDE), and each has quirks
- **Fragile** — requires shelling out to OS commands with admin privileges
  on some platforms; can leave the system in a broken state if Madhyamas
  crashes without restoring settings
- **Invasive** — changes system-wide settings, affecting all applications,
  not just the one being debugged
- **Already well-documented** — the onboarding wizard and
  `docs/NETWORK_CONFIGURATION.md` already guide users through manual setup

Charles can do this because it's a desktop app with a GUI that the user
interacts with directly. Madhyamas is a server-style binary (often run in
Docker or on a remote host) where auto-configuring the local OS proxy
doesn't make sense — the client is usually on a different machine.

**Better alternative:** Improve the onboarding wizard to:
1. Auto-detect the OS and show platform-specific instructions
2. Provide copy-paste commands (`networksetup ...` for macOS, etc.)
3. Offer a "Test proxy connection" button that verifies the client is
   configured correctly

### How it would show up in the UI

- **Onboarding wizard**: "Auto-configure" button (if implemented) or
  platform-specific copy-paste instructions (recommended)
- **CLI**: `madhyamas serve --auto-config-proxy` (if implemented)
- **API**: `POST /api/system-proxy/enable` (if implemented)

### How it can be tested

1. **macOS test**: Enable auto-config, verify `networksetup -getwebproxy`
   shows the proxy, disable, verify it's restored
2. **Windows test**: Enable, verify registry settings change, disable,
   verify restoration
3. **Crash recovery test**: Kill the process without graceful shutdown,
   verify settings are restored on next startup (requires a saved state)
4. **Docker test**: Verify auto-config is a no-op when running in a
   container (no host OS to configure)

### What needs to be documented

- Update `docs/NETWORK_CONFIGURATION.md` — add platform-specific
  copy-paste commands for manual proxy setup (macOS, Windows, Linux,
  iOS, Android)
- Update `docs/GETTING_STARTED.md` — link to the network configuration guide
- Do **not** create auto-config documentation (feature not recommended)

### Recommendation

**Skip.** Platform-specific, fragile, and invasive. Madhyamas is often run
on a remote host or in Docker where local OS proxy settings are
irrelevant. Invest in better onboarding documentation and copy-paste
instructions instead. Effort: N/A (not recommended) / Small (documentation
improvement, recommended).

---

## 9. Headless Mode

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Binary entry point | `crates/madhyamas/src/main.rs:242-305` | `main()` dispatches on subcommand: `Mcp` (stdio), `Cli` (one-shot), `Serve`/`None` (proxy + web UI) |
| Web UI serving | `crates/madhyamas-api/src/embedded_assets.rs` | Web UI is embedded via `rust-embed` and served by the API server on `api_port` |
| API server | `crates/madhyamas-api/src/lib.rs` | axum server binds `api_port`; always started in `serve` mode |
| Search results | — | `headless`/`Headless` appears in `scripts/capture-screenshots.mjs:35` (Playwright headless), `PRD-Madhyamas.md:138, 383` (mentioned as a future feature), and `docs/CHARLES_PROXY_FEATURE_COMPARISON.md:183, 311, 358` |
| Current behavior | — | **Madhyamas is already effectively headless** — it's a server binary with no GUI; the web UI is served to a browser, not rendered locally |

### What needs to be done

**Very little.** Madhyamas is already a headless server binary. Charles's
`-headless` flag exists because Charles is a GUI desktop app (Java Swing)
that needs an explicit flag to run without showing its window. Madhyamas
has no GUI to hide.

The only meaningful work is:

1. **Add a `--headless` flag** as a no-op alias for documentation/CLI
   compatibility (so users coming from Charles find a familiar flag)
2. **Add an option to disable the web UI** (serve API only, no embedded
   assets) for minimal-resource deployments
3. **Document** that Madhyamas is inherently headless

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas/src/main.rs` | Add `--headless` flag (no-op, logs an informational message that Madhyamas is always headless); add `--no-web-ui` flag to skip mounting the embedded assets router |
| `crates/madhyamas-api/src/lib.rs` | Add a `serve_web_ui: bool` parameter; when false, skip mounting the `embedded_assets` router (API-only mode) |
| `crates/madhyamas-core/src/config.rs` | Add `serve_web_ui: bool` to `ProxyConfig` (default: `true`) |

### How it should be done

**`--headless` flag:** Accept it, log "Madhyamas runs as a headless server
by default; the web UI is available at http://host:api_port", and continue
normally. This is purely for CLI ergonomics — users scripting against
Madhyamas who are used to Charles's `-headless` flag won't get an error.

**`--no-web-ui` flag:** When set, the API server doesn't mount the
embedded assets router. This saves ~1-2 MB of memory (the embedded assets
are already in the binary, but the router and static-file serving are
skipped) and reduces the attack surface for API-only deployments (e.g.,
when Madhyamas is used purely as a proxy with CLI/MCP control).

```rust
// In main.rs Args struct:
/// Run in headless mode (Madhyamas is always headless; this flag is
/// accepted for compatibility with Charles Proxy's -headless flag).
#[arg(long, global = true)]
headless: bool,

/// Disable the embedded web UI (API-only mode). Useful for minimal
/// deployments where the proxy is controlled via CLI or MCP.
#[arg(long, env = "MADHYAMAS_NO_WEB_UI", global = true)]
no_web_ui: bool,
```

### How it would show up in the UI

- **CLI**: `madhyamas serve --headless` (no-op, informational log);
  `madhyamas serve --no-web-ui` (API-only mode)
- **API**: `GET /api/config` includes `serve_web_ui: bool`
- **Logs**: On startup with `--headless`, log the web UI URL

### How it can be tested

1. **Headless flag test**: Run `madhyamas serve --headless`, verify it
   starts normally and logs the informational message
2. **No-web-ui test**: Run `madhyamas serve --no-web-ui`, verify the API
   works (`GET /api/health` returns 200) but `GET /` returns 404
3. **Normal test**: Run `madhyamas serve`, verify both API and web UI work
4. **MCP test**: Verify MCP mode works regardless of the web UI flag

### What needs to be documented

- Update `CLAUDE.md` — note that Madhyamas is inherently headless; document
  the `--headless` (no-op) and `--no-web-ui` flags
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Headless mode
  row from ❌ to ✅ (with note that it's inherently headless)
- Update `docs/DEPLOYMENT.md` — document API-only mode for minimal deployments

### Recommendation

**Build (trivial).** This is the lowest-effort item in the list. The
`--headless` flag is a no-op for CLI compatibility; the `--no-web-ui` flag
is a small, useful addition for API-only deployments. The main deliverable
is documentation clarifying that Madhyamas is already headless. Effort:
Trivial.

---

## 10. Client Process Tracking

### What exists now

| Aspect | Location | Current State |
|---|---|---|
| Client address | `crates/madhyamas-core/src/proxy/engine.rs:401-404` | `listener.accept()` returns `client_addr` (IP + port); used for access control and logging, **not stored in traffic entries** |
| Traffic entry | `crates/madhyamas-core/src/traffic/types.rs:208-236` | `TrafficEntry` has no `client_addr`, `client_port`, or `process` field |
| Request data | `crates/madhyamas-core/src/traffic/types.rs:67-92` | `RequestData` has no client/process fields |
| Process info | `crates/madhyamas-core/src/performance/monitor.rs:258-259` | `sysinfo::Pid::from(std::process::id())` — gets the **proxy's own** PID for memory monitoring, not client process PIDs |
| Search results | — | No `client_process`, `process_name`, or `pid` fields in traffic types; `lsof`/`netstat` not used anywhere in the proxy |

### What needs to be done

Charles can show which **local process** made each request (e.g., "Chrome",
"Slack", "curl"). This requires mapping the client TCP connection back to
the local process that opened it. To implement:

1. **Capture the client port** for each connection (already available via
   `client_addr.port()`)
2. **Resolve the client port to a process** — query the OS socket table to
   find which PID owns the local port
3. **Resolve the PID to a process name** — query the OS process list
4. **Store process info** in the traffic entry
5. **Display it** in the web UI

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/traffic/types.rs` | Add `client_process: Option<ClientProcessInfo>` to `TrafficEntry`; `ClientProcessInfo { pid: u32, name: String, command: Option<String> }` |
| `crates/madhyamas-core/src/traffic/store.rs` | Add `client_pid INTEGER`, `client_process TEXT` columns to the `requests` table; update insert/query SQL |
| `crates/madhyamas-core/src/process.rs` | **New file** — `ProcessResolver` with platform-specific implementations: `resolve(local_port: u16) -> Option<ClientProcessInfo>` |
| `crates/madhyamas-core/src/lib.rs` | Export `ProcessResolver`, `ClientProcessInfo` |
| `crates/madhyamas-core/src/proxy/engine.rs` | In the accept loop (line 401), after getting `client_addr`, call `ProcessResolver::resolve(client_addr.port())` and pass the result into the traffic entry |
| `crates/madhyamas-api/src/handlers.rs` | Include `client_process` in traffic API responses |
| `web/src/types/traffic.ts` | Add `client_process?: { pid: number; name: string; command?: string }` to `TrafficEntry` |
| `web/src/features/traffic/TrafficList.tsx` | Add an optional "Process" column (hidden by default, toggleable) |
| `web/src/features/traffic/TrafficDetail.tsx` | Show process info in the request summary |

### How it should be done

**Platform-specific socket-to-process resolution:**

| Platform | Method |
|---|---|
| **Linux** | Read `/proc/net/tcp` and `/proc/net/tcp6` to map local port → inode, then scan `/proc/*/fd/*` symlinks to find the PID owning that inode. No external dependencies. |
| **macOS** | Use `lsof -i :<port> -t` (requires the user to have permissions; may prompt for sudo on some macOS versions). Alternatively, use the `libproc` crate or `sysctl` with `NET_RT_DUMP`. |
| **Windows** | Use the `GetExtendedTcpTable` Win32 API (`iphlpapi.dll`) via the `windows` crate to map port → PID, then `OpenProcess` + `QueryFullProcessImageName` for the process name. |

**Implementation approach:**
```rust
pub struct ProcessResolver {
    cache: RwLock<HashMap<(IpAddr, u16), ClientProcessInfo>>,  // cache by (local_ip, local_port)
    cache_ttl: Duration,  // entries expire after 60s to handle process reuse
}

impl ProcessResolver {
    pub fn resolve(&self, local_addr: SocketAddr) -> Option<ClientProcessInfo> {
        // 1. Check cache
        // 2. Query OS socket table for the local port
        // 3. Resolve PID → process name
        // 4. Cache and return
    }
}
```

**Caching is critical:** Resolving the process for every request would be
expensive (reading `/proc` or calling `lsof` on every connection). Cache
by local port with a short TTL (connections from the same port within a
few seconds are almost certainly the same process).

**Limitations:**
- Only works for **local** connections (the proxy and the client are on
  the same machine). For remote clients (mobile devices, other servers),
  process info is unavailable — the field is `None`.
- macOS `lsof` may require elevated permissions for processes owned by
  other users.
- The mapping is ephemeral — by the time the resolver runs, the process
  may have closed the socket. The cache mitigates this but can't eliminate
  it.

**Why it's low priority:** Process tracking is only useful when debugging
traffic from multiple local applications simultaneously. For most users,
the `User-Agent` header and host are sufficient to identify the source.
It's also OS-specific with permission pitfalls.

### How it would show up in the UI

- **Traffic list**: Optional "Process" column showing the process name
  (e.g., "Chrome", "curl", "Slack"); hidden by default, toggleable via
  column settings
- **Traffic detail**: Process name, PID, and full command in the request
  summary header
- **Filtering**: Filter traffic by process name (e.g., "show only Chrome
  requests")
- **API**: `GET /api/traffic` responses include `client_process` when
  available
- **CLI**: `madhyamas traffic list --process chrome`

### How it can be tested

1. **Linux test**: Start the proxy, make a request with `curl`, verify
   the traffic entry shows `client_process: { pid, name: "curl" }`
2. **macOS test**: Same, verify `lsof`-based resolution works
3. **Windows test**: Same, verify `GetExtendedTcpTable`-based resolution
4. **Remote client test**: Connect from another machine, verify
   `client_process` is `None` (no error)
5. **Cache test**: Make multiple requests from the same process, verify
   the resolver isn't called repeatedly
6. **Permission test** (macOS): Verify graceful degradation when `lsof`
   can't see the process (returns `None`, no crash)
7. **Filter test**: Filter by process name in the web UI, verify only
   matching entries show

### What needs to be documented

- Update `CLAUDE.md` — note client process tracking and its limitations
  (local connections only, platform-specific)
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Client Process
  tracking row from ❌ to ✅
- Create `docs/CLIENT_PROCESS.md` — guide on the Process column, platform
  support, permission requirements, limitations

### Recommendation

**Defer.** OS-specific with permission pitfalls, only works for local
connections, and `User-Agent`/host is usually sufficient to identify
traffic sources. Build only if there's demand from users debugging
multiple local applications simultaneously. Effort: Medium (Linux) /
Medium-Hard (macOS/Windows).

---

## Implementation Priority Order

Based on complexity, impact, and demand — **only build the items that
have clear value; explicitly skip the deprecated/niche ones.**

| Priority | Feature | Effort | Impact | Recommendation |
|---|---|---|---|---|
| 1 | **Headless mode** | Trivial | Low | **Build** — no-op flag + `--no-web-ui` + documentation; quick win |
| 2 | **Protobuf decoder (Phase 1)** | Small | Medium | **Build Phase 1** — wire up existing schema-less decoder; makes gRPC panel usable |
| 3 | **DNS Spoofing (host override)** | Small | Medium | **Build host-override layer** — useful for local dev; skip DNS server |
| 4 | **Protobuf decoder (Phase 2)** | Medium | Medium | **Build when gRPC demand grows** — descriptor-based decoding with `prost-reflect` |
| 5 | **Reverse Proxy** | Medium-Hard | Low | **Defer** — niche; forward proxy + SOCKS covers most cases |
| 6 | **Port Forwarding (TCP)** | Small | Low | **Defer** — redundant with SOCKS5 |
| 7 | **Port Forwarding (UDP)** | Medium | Low | **Skip** — rarely needed, non-trivial state management |
| 8 | **Client Process Tracking** | Medium-Hard | Low | **Defer** — OS-specific, local-only, `User-Agent` usually sufficient |
| 9 | **Validate (W3C)** | Medium | Low | **Defer** — users can validate externally; privacy concerns |
| 10 | **AMF / Flash Remoting** | N/A | None | **Skip** — Flash is end-of-lifed (2020); dead format |
| 11 | **NTLM Authentication** | N/A | None | **Skip** — legacy; use NTLM bridge + upstream chaining instead |
| 12 | **Auto Browser/OS Proxy Config** | N/A | Low | **Skip** — platform-specific, fragile, invasive; improve docs instead |

**Recommended approach:** Ship items 1–3 first (trivial to small effort,
clear value), then evaluate items 4–9 based on user demand, and explicitly
skip items 10–12 (deprecated technology or poor effort-to-value ratio).
The single highest-leverage action is **item 2 (protobuf decoder Phase 1)**
— it turns the gRPC panel from a raw-base64 view into a usable debugging
tool with minimal code changes.

---

*Generated 2026-08-01. Based on codebase analysis as of this date.
Companion document to [HIGH_PRIORITY_FEATURE_ANALYSIS.md](HIGH_PRIORITY_FEATURE_ANALYSIS.md)
and [MEDIUM_PRIORITY_FEATURE_ANALYSIS.md](MEDIUM_PRIORITY_FEATURE_ANALYSIS.md).*
