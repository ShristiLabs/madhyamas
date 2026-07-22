# Madhyamas — Proxy Flow & Internals

This document explains in detail how the Madhyamas proxy works end‑to‑end:
how connections are accepted, how TLS/HTTPS interception is performed, how
certificates are generated and used, how traffic is stored, and how it is
displayed in the web UI. Mermaid diagrams are used throughout.

> Source references use `<ref_file />` / `<ref_snippet />` tags so you can
> jump directly to the relevant code.

---

## 1. High‑Level Architecture

Madhyamas is a single unified binary that runs three cooperating subsystems
inside one process:

1. **Proxy engine** (`madhyamas-core::proxy`) — listens on the proxy port
   (default `8888`), accepts TCP connections, performs TLS interception, and
   forwards traffic to upstream servers.
2. **API + Web UI server** (`madhyamas-api`) — listens on the API port
   (default `3001`), serves the embedded React SPA, exposes the REST API,
   and pushes real‑time traffic updates over a WebSocket.
3. **Traffic store** (`madhyamas-core::traffic`) — a SQLite database that
   persists every captured request/response.

```mermaid
flowchart LR
    subgraph Client["Client (browser / app / CLI)"]
        App[HTTP/HTTPS App]
        Browser[Web Browser UI]
    end

    subgraph Madhyamas["Madhyamas Process (single binary)"]
        direction TB
        Proxy["Proxy Engine<br/>:8888 (TCP)"]
        API["API + Web UI Server<br/>:3001 (HTTP/WS)"]
        Store[("TrafficStore<br/>SQLite (WAL)")]
        Certs["CertificateManager<br/>CA + leaf certs"]
        Proxy --> Store
        Proxy --> Certs
        API --> Store
    end

    Upstream["Upstream Server<br/>(internet)"]

    App -- "HTTP or CONNECT" --> Proxy
    Proxy -- "reqwest (HTTP/1.1 + HTTP/2)" --> Upstream
    Upstream --> Proxy
    Proxy --> App

    Browser -- "HTTP (SPA) + WS" --> API
    API -- "embedded assets / REST / WS" --> Browser
```

The binary entry point wires everything together in `run_proxy_server`:
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas/src/main.rs" lines="225-364" />

---

## 2. Connection Lifecycle (HTTP vs HTTPS)

When a client connects to the proxy port, the engine peeks the first bytes
to decide whether this is a plain HTTP request or an HTTPS `CONNECT`
tunnel. The two paths then diverge.

```mermaid
flowchart TD
    Accept["TcpListener::accept<br/>(engine.rs:252)"] --> Peek["peek first 1024 bytes"]
    Peek --> Check{"Starts with<br/>'CONNECT '?"}
    Check -- Yes --> HTTPS["handle_https_tunnel"]
    Check -- No --> HTTP["handle_http_proxy"]

    HTTPS --> ParseConn["Parse CONNECT host:port"]
    ParseConn --> GenCert["cert_manager.generate_cert_for_host(host)"]
    GenCert --> Send200["Write '200 Connection Established'"]
    Send200 --> TLS["TlsAcceptor.accept(client_socket)"]
    TLS --> HandshakeOK{"Handshake OK?"}
    HandshakeOK -- No --> Record502["Record 502 traffic entry<br/>(untrusted CA / pinning)"]
    HandshakeOK -- Yes --> ALPN["Inspect ALPN protocol"]
    ALPN --> TLSLoop["handle_tls_request<br/>(keep-alive loop)"]

    HTTP --> ParseReq["parse_http_request"]
    ParseReq --> Body["read_full_request_body"]
    Body --> WSCheck{"WebSocket upgrade?"}
    WSCheck -- Yes --> WS["handle_websocket_upgrade_http"]
    WSCheck -- No --> Pipe["pipeline.process_request"]
    TLSLoop --> WSCheck2{"WebSocket upgrade?"}
    WSCheck2 -- Yes --> WSTLS["handle_websocket_upgrade_tls"]
    WSCheck2 -- No --> Pipe
```

Source: <ref_file file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/engine.rs" />

The peek/branch logic lives in `handle_connection`:
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/engine.rs" lines="275-320" />

---

## 3. Certificate Management & TLS Handshake

HTTPS interception requires a man‑in‑the‑middle TLS termination. Madhyamas
generates a per‑host leaf certificate signed by a locally‑generated CA. The
user installs this CA into their system/browser trust store once; afterwards
the proxy can present valid‑looking certificates for any HTTPS host.

### 3.1 CA Certificate Lifecycle

```mermaid
flowchart TD
    Start["CertificateManager::new(cert_path)"] --> Exists{"CA files<br/>already on disk?"}
    Exists -- No --> GenCA["generate_ca()<br/>ECDSA P-256, self-signed<br/>CN='Madhyamas Root CA'"]
    GenCA --> SaveCA["Write madhyamas-ca.pem<br/>+ madhyamas-ca-key.pem (chmod 0600)"]
    Exists -- Yes --> LoadCA["load_ca()<br/>Parse PEM key + params"]
    SaveCA --> Ready["CertificateManager ready"]
    LoadCA --> Ready
```

The CA is generated once using `rcgen` with ECDSA‑P256‑SHA256 and stored in
`~/.madhyamas/certs/`. The private key is locked to `0600` on Unix.
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/tls/certificate.rs" lines="47-159" />

### 3.2 Per‑Host Leaf Certificate Generation

For every distinct HTTPS host the proxy signs a leaf certificate on demand.
Certificates are cached in memory with a 24h TTL and a 10 000‑entry LRU
cap to avoid re‑signing on every request.

```mermaid
flowchart TD
    Req["generate_cert_for_host(hostname)"] --> Cache{"Cached &<br/>not expired?"}
    Cache -- Yes --> Return["Return cached cert"]
    Cache -- No --> Build["Build CertificateParams<br/>CN=hostname, SAN=hostname<br/>EKU: ServerAuth, ClientAuth"]
    Build --> KeyGen["KeyPair::generate ECDSA P-256"]
    KeyGen --> Sign["signed_by(CA key)"]
    Sign --> Evict["Evict expired + oldest<br/>if cache > 10k"]
    Evict --> Insert["Insert into cache"]
    Insert --> Return
```

<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/tls/certificate.rs" lines="170-252" />

### 3.3 TLS Handshake with the Client (Downstream)

Once the `CONNECT` request is parsed and a leaf cert is generated, the
proxy:

1. Sends `HTTP/1.1 200 Connection Established\r\n\r\n` to the client.
2. Builds a `rustls::ServerConfig` with the leaf cert chain + private key.
3. **Advertises only `http/1.1` via ALPN** (not `h2`) — the proxy cannot
   yet parse HTTP/2 frames on the downstream side. Forcing HTTP/1.1 here
   avoids binary garbage and 502 errors, while the **upstream** leg can
   still negotiate HTTP/2 via `reqwest`.
4. Runs `TlsAcceptor::accept`. If it fails (typical cause: the client
   does not trust the CA — common with Android cert pinning), a 502
   traffic entry is recorded so the failed attempt is visible in the UI.

```mermaid
sequenceDiagram
    participant Client
    participant Proxy as Proxy Engine
    participant Cert as CertificateManager
    participant Store as TrafficStore

    Client->>Proxy: CONNECT api.example.com:443 HTTP/1.1
    Proxy->>Cert: generate_cert_for_host("api.example.com")
    Cert-->>Proxy: leaf cert + private key (PEM)
    Proxy->>Client: HTTP/1.1 200 Connection Established\r\n\r\n

    Note over Proxy: Build rustls ServerConfig<br/>ALPN = [http/1.1 only]
    Proxy->>Client: TLS ServerHello + Certificate (leaf, signed by CA)
    Client->>Proxy: ClientHello finished / key exchange
    Note over Client: Client verifies chain against<br/>installed Madhyamas CA
    Proxy->>Proxy: TlsAcceptor::accept OK

    alt Handshake fails (untrusted CA / pinning)
        Proxy->>Store: store_request (CONNECT, 502)
        Proxy->>Store: store_response (502 Bad Gateway)
        Proxy-->>Client: connection dropped
    end
```

Source for the handshake and ALPN policy:
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/engine.rs" lines="322-448" />

### 3.4 TLS to the Upstream Server

When forwarding to an HTTPS upstream, the proxy uses `reqwest`, which
performs its own ALPN negotiation (HTTP/1.1 or HTTP/2) with the real
server. For raw WebSocket upgrades the engine builds a
`rustls::ClientConfig` with a `SkipServerVerification` verifier (it must
trust the upstream cert without relying on the system root store because
the proxy itself is the MITM).
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/engine.rs" lines="634-642" />

### 3.5 Trust Model Summary

```mermaid
flowchart LR
    subgraph Trust["What the client must trust"]
        CA["Madhyamas Root CA<br/>(installed once)"]
    end
    subgraph ProxyGen["What the proxy generates per host"]
        Leaf["Leaf cert for host<br/>signed by CA"]
    end
    CA -. signs .-> Leaf
    Leaf -. presented to .-> ClientApp["Client app<br/>verifies chain → CA"]
```

> **Important**: Without installing the CA, every HTTPS site will fail with
> a TLS handshake error. The proxy records these as 502 entries so they are
> visible in the UI rather than silently dropped.

---

## 4. Request Processing Pipeline

Both the HTTP and HTTPS paths converge on a shared `Pipeline::process_request`
which runs the full interception chain. The pipeline borrows the various
managers from the engine for the lifetime of one (or many, on keep‑alive)
requests.

```mermaid
flowchart TD
    In["process_request(request_data, client_stream)"] --> Metrics["Record request metrics<br/>(bytes in)"]
    Metrics --> MemCheck["Check memory pressure"]
    MemCheck --> Rewrite["Apply rewrite rules to request"]
    Rewrite --> Hooks["Run script/plugin request hooks"]
    Hooks --> gRPC["Detect & record gRPC request"]
    gRPC --> Mock{"Mock matches?"}
    Mock -- Yes --> MockResp["Build mock response<br/>(+ optional throttle delay)"]
    MockResp --> Short1["short_circuit_response<br/>store + broadcast + write to client"]
    Short1 --> DoneResp["Outcome::Responded"]

    Mock -- No --> BPReq{"Breakpoint on request?"}
    BPReq -- Yes --> Pause["pause_and_wait (request)"]
    Pause --> BPDec{"Decision?"}
    BPDec -- Abort --> DoneAbort["Outcome::Aborted"]
    BPDec -- Continue --> Capture
    BPDec -- Modify --> ApplyModReq["Apply request modifications"]
    ApplyModReq --> Capture
    BPDec -- Respond --> BPResp["Build response from breakpoint"]
    BPResp --> Short1

    BPReq -- No --> Capture["Store request (if not excluded)<br/>broadcast Added event"]
    Capture --> Throttle["Apply throttle latency"]
    Throttle --> Forward["forward_via_reqwest"]
    Forward --> Resp{"Upstream OK?"}
    Resp -- No --> Err502["Store 502 error response"]
    Err502 --> DoneFwd["Outcome::Forwarded"]
    Resp -- Yes --> RewResp["Apply rewrite rules to response"]
    RewResp --> HookResp["Run script/plugin response hooks"]
    HookResp --> gRPCResp["Record gRPC response frames"]
    gRPCResp --> BPRespCheck{"Breakpoint on response?"}
    BPRespCheck -- Yes --> Pause2["pause_and_wait (response)"]
    Pause2 --> BPDec2{"Decision?"}
    BPDec2 -- Abort --> DoneAbort
    BPDec2 -- Modify --> ApplyModResp["Apply response modifications"]
    ApplyModResp --> StoreResp
    BPDec2 -- Continue --> StoreResp
    BPRespCheck -- No --> StoreResp["Store response<br/>broadcast Updated event"]
    StoreResp --> RecMock{"Mock recording on?"}
    RecMock -- Yes --> Rec["Record from traffic as mock"]
    RecMock -- No --> DoneFwd
    Rec --> DoneFwd
```

Source: <ref_file file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/pipeline.rs" />

Key entry point: <ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/pipeline.rs" lines="226-496" />

### 4.1 Upstream Forwarding (`reqwest`)

The proxy does **not** manually open a TCP socket to the upstream. Instead
it builds a `reqwest::Client` per request with:

- `redirect(Policy::none())` — the proxy must return 3xx to the client
  unchanged.
- `no_proxy()` — prevents a feedback loop if the host has system proxy
  env vars.
- `gzip/deflate/brotli = false` — the **raw compressed body** is stored
  and the `Content-Encoding` header is preserved, so the web UI can
  toggle between compressed and decompressed views. The client receives
  the original compressed bytes and decompresses normally.
- HTTP/1.1 **and** HTTP/2 enabled — ALPN picks the best protocol with the
  upstream.

Hop‑by‑hop headers (`Connection`, `Keep-Alive`, `Transfer-Encoding`,
`Upgrade`, `Proxy-Connection`, `TE`, `Host`, `Content-Length`) are stripped
before forwarding; `Host` is omitted because `reqwest` sets `:authority`
from the URL (sending both causes HTTP/2 PROTOCOL_ERROR resets).

<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/pipeline.rs" lines="688-827" />

### 4.2 Self‑Exclusion (Feedback Loop Prevention)

The pipeline skips capturing requests whose `Host` ends with `:api_port`
or whose URL contains `:{api_port}/api/` — this prevents the web UI's own
API calls from appearing in the traffic list.
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/proxy/pipeline.rs" lines="136-153" />

---

## 5. Traffic Storage (SQLite)

All captured traffic is persisted in a single SQLite database at
`~/.madhyamas/traffic.db` (configurable via `--db-path`).

### 5.1 Database Schema

```mermaid
erDiagram
    sessions ||--o{ requests : has
    requests ||--o| responses : has
    sessions ||--o{ ws_connections : has
    ws_connections ||--o{ ws_messages : has

    sessions {
        TEXT id PK
        TEXT name
        INTEGER created_at
        INTEGER updated_at
    }
    requests {
        TEXT id PK
        TEXT session_id FK
        TEXT method
        TEXT url
        TEXT host
        TEXT path
        TEXT headers "JSON"
        BLOB body
        TEXT content_type
        INTEGER timestamp
        INTEGER modified
        TEXT notes
    }
    responses {
        TEXT request_id PK_FK
        INTEGER status_code
        TEXT status_message
        TEXT headers "JSON"
        BLOB body
        TEXT content_type
        INTEGER duration_ms
    }
    ws_connections {
        TEXT id PK
        TEXT session_id FK
        TEXT url
        TEXT host
        TEXT path
        TEXT state
        TEXT request_headers
        TEXT response_headers
        TEXT subprotocol
        INTEGER created_at
        INTEGER closed_at
        INTEGER messages_sent
        INTEGER messages_received
        INTEGER bytes_sent
        INTEGER bytes_received
    }
    ws_messages {
        TEXT id PK
        TEXT connection_id FK
        TEXT direction
        TEXT message_type
        BLOB payload_raw
        TEXT payload_text
        INTEGER opcode
        INTEGER is_final
        INTEGER mask
        INTEGER timestamp
    }
```

Schema creation: <ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/traffic/store.rs" lines="82-187" />

### 5.2 SQLite Pragmas (Performance)

The store tunes SQLite for a high‑write proxy workload:

| Pragma | Value | Why |
|--------|-------|-----|
| `journal_mode` | `WAL` | Concurrent readers + single writer, no reader/writer blocking |
| `synchronous` | `NORMAL` | Safe with WAL, much faster than FULL |
| `busy_timeout` | `5000ms` | Avoids "database is locked" under burst writes |
| `cache_size` | `-64000` (64 MB) | Larger page cache for faster reads |

<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/traffic/store.rs" lines="82-101" />

### 5.3 Write Path (Request → Response)

```mermaid
sequenceDiagram
    participant Pipeline
    participant Store as TrafficStore
    participant DB as SQLite (WAL)
    participant Bus as broadcast::Sender<TrafficEvent>

    Pipeline->>Store: store_request(entry)
    Store->>Store: clamp_body (max 20 MB)
    Store->>DB: INSERT OR REPLACE INTO requests
    Store->>Bus: emit TrafficEvent::Added(snapshot)
    Bus-->>Store: (subscribers notified)

    Note over Pipeline: ... upstream response arrives ...

    Pipeline->>Store: store_response(entry.id, response)
    Store->>Store: clamp_body
    Store->>DB: INSERT OR REPLACE INTO responses
    Store->>Store: get_by_id(request_id)
    Store->>DB: SELECT join request+response
    Store->>Bus: emit TrafficEvent::Updated(snapshot)
```

- **Capture toggle**: if `capture_enabled == false`, both `store_request`
  and `store_response` return early (passthrough mode).
- **Body truncation**: bodies larger than `max_body_size` (default 20 MB)
  are truncated before being written.
- **Snapshots**: events carry a `TrafficEntrySnapshot` that **excludes**
  the body bytes — only metadata + sizes — to keep WebSocket messages
  small. The full body is fetched on demand via `GET /api/traffic/{id}`.

<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/traffic/store.rs" lines="269-350" />

### 5.4 Read Path (Query / Filter)

`get_traffic` builds a parameterized SQL query against `requests LEFT JOIN
responses`, applying optional filters: `url_pattern`, `method`, status
range, `search`, `file_type`, `header`, `cookie`, `limit`, `offset`.
Results are ordered by `timestamp DESC`.

<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/traffic/store.rs" lines="353-506" />

---

## 6. Real‑Time Display (WebSocket + React UI)

### 6.1 Two‑Channel Update Model

The web UI receives traffic updates through **two** cooperating channels:

1. **REST** (`GET /api/traffic`, `GET /api/traffic/{id}`) — used for the
   initial list, filtering, and fetching full bodies on demand.
2. **WebSocket** (`GET /api/ws`) — used for live, push‑based updates as
   traffic is captured.

```mermaid
flowchart LR
    subgraph Backend
        Store[("TrafficStore<br/>SQLite")]
        Bus["broadcast::Sender<br/>TrafficEvent (cap 1024)"]
        WS["axum WS handler<br/>/api/ws"]
        REST["axum REST handlers<br/>/api/traffic"]
        Store -- "emit_event" --> Bus
        Bus -- "subscribe" --> WS
        Store -- "get_traffic/get_by_id" --> REST
    end

    subgraph Frontend["React Web UI"]
        Hook["useTrafficWebSocket"]
        Query["TanStack Query<br/>(REST fetches)"]
        List["TrafficList"]
        Detail["TrafficDetail"]
    end

    Browser["Browser"] -- "WS upgrade" --> WS
    WS -- "InitialTraffic + Traffic events" --> Browser
    Browser --> Hook
    Hook --> List
    Browser -- "HTTP GET" --> REST
    REST --> Browser
    Browser --> Query
    Query --> Detail
```

### 6.2 WebSocket Message Protocol

```mermaid
sequenceDiagram
    participant UI as Web UI (useTrafficWebSocket)
    participant WS as axum WS handler
    participant Store as TrafficStore
    participant Bus as broadcast channel

    UI->>WS: HTTP Upgrade → ws://host/api/ws
    WS->>UI: Connected { client_id }
    WS->>Store: get_traffic(default filter)
    Store-->>WS: existing entries
    WS->>UI: InitialTraffic [snapshots...]

    loop live traffic
        Store->>Bus: TrafficEvent::Added / Updated / Deleted / Cleared
        Bus->>WS: recv()
        WS->>UI: Traffic { event }
    end

    UI->>WS: ping (keep-alive)
    WS-->>UI: pong
```

Server‑side message types (`WsServerMessage`):
<ref_snippet file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-core/src/traffic/events.rs" lines="80-114" />

The axum handler subscribes to the store's broadcast channel, sends the
initial traffic snapshot, then forwards every subsequent event to the
client. Lagged receivers (slow clients) log a warning and keep going.
<ref_file file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-api/src/ws.rs" />

### 6.3 Frontend State Machine

The React hook `useTrafficWebSocket` drives the in‑memory traffic list:

```mermaid
stateDiagram-v2
    [*] --> Connecting: useWebSocket autoConnect
    Connecting --> Loading: onConnect
    Loading --> Populated: InitialTraffic received
    Populated --> Populated: Added → prepend
    Populated --> Populated: Updated → replace by id
    Populated --> Populated: Deleted → filter out
    Populated --> Empty: Cleared
    Populated --> Populated: reconnect (keep data)
    Empty --> Populated: Added
    Populated --> [*]: unmount
```

<ref_file file="/Users/harikiranbavineni/madhyamas/web/src/hooks/useTrafficWebSocket.ts" />

When the user clicks a row, `TrafficDetail` fetches the **full** entry
(including bodies) via `GET /api/traffic/{id}` and renders headers, body
(JSON/formatted), and timing. The list view only ever holds the
lightweight `TrafficEntrySnapshot` (no body bytes), keeping memory and
bandwidth low.

---

## 7. End‑to‑End Sequence (HTTPS Request)

The following sequence ties everything together for a single HTTPS request
from a configured client to an upstream server, with capture enabled and no
mocks/breakpoints matching.

```mermaid
sequenceDiagram
    autonumber
    participant App as Client App
    participant Proxy as Proxy Engine :8888
    participant Cert as CertificateManager
    participant Pipe as Pipeline
    participant HTTP as reqwest (upstream)
    participant Store as TrafficStore (SQLite)
    participant Bus as broadcast channel
    participant WS as Web UI WS client

    App->>Proxy: CONNECT api.example.com:443
    Proxy->>Cert: generate_cert_for_host("api.example.com")
    Cert-->>Proxy: leaf cert (signed by CA)
    Proxy->>App: 200 Connection Established
    Proxy->>App: TLS handshake (leaf cert, ALPN http/1.1)
    App->>Proxy: GET /v1/data HTTP/1.1 (over TLS)

    Proxy->>Pipe: process_request
    Pipe->>Pipe: rewrite rules + hooks
    Pipe->>Store: store_request(entry)
    Store->>Bus: TrafficEvent::Added
    Bus->>WS: push snapshot
    Pipe->>HTTP: reqwest GET https://api.example.com/v1/data
    HTTP->>HTTP: ALPN (h2 or http/1.1) with upstream
    HTTP-->>Pipe: 200 OK + body (raw, compressed)
    Pipe->>Pipe: response rewrites + hooks
    Pipe->>Store: store_response(entry.id, response)
    Store->>Bus: TrafficEvent::Updated
    Bus->>WS: push updated snapshot
    Pipe->>App: HTTP/1.1 200 OK + body (re-serialized)
    App->>App: decompress / render
```

---

## 8. Component Map (Crate Level)

```mermaid
flowchart TD
    Main["madhyamas (binary)<br/>main.rs: serve/mcp/cli"]
    Core["madhyamas-core<br/>proxy · tls · traffic · intercept · grpc · scripting · plugin · enterprise"]
    API["madhyamas-api<br/>axum router · handlers · ws · embedded_assets"]
    CLI["madhyamas-cli<br/>commands + ApiClient"]
    MCP["madhyamas-mcp<br/>MCP server + tools"]

    Main --> Core
    Main --> API
    Main --> CLI
    Main --> MCP

    API --> Core
    CLI --> Core
    MCP --> Core

    Web["web/ (React + Vite)<br/>compiled → embedded_assets.rs via rust-embed"]
    Web -. build output .-> API
```

The web UI is built with Vite (`cd web && npm run build`) and the resulting
`web/dist/` is embedded into the `madhyamas-api` crate at compile time via
`rust-embed`, so the released binary is fully self‑contained. A
`MADHYAMAS_WEB_DIR` env var can override to serve from disk for dev.
<ref_file file="/Users/harikiranbavineni/madhyamas/crates/madhyamas-api/src/embedded_assets.rs" />

---

## 9. Key Design Decisions Recap

| Decision | Rationale |
|----------|-----------|
| **Per‑host leaf certs signed by a local CA** | Allows transparent HTTPS interception without per‑site setup; user installs CA once. |
| **ALPN advertises only `http/1.1` downstream** | Proxy cannot parse HTTP/2 frames on the client side yet; forcing HTTP/1.1 avoids 502 garbage. Upstream still uses HTTP/2 via `reqwest`. |
| **`reqwest` for upstream forwarding** | Handles HTTP/2, chunked encoding, connection pooling, and proper header handling without reimplementing HTTP. |
| **No auto‑decompression in `reqwest`** | Preserves the raw compressed body + `Content-Encoding` so the UI can toggle views and the client gets the original bytes. |
| **SQLite + WAL** | High‑write proxy workload with concurrent readers; WAL avoids reader/writer blocking. |
| **Snapshot vs full entry over WS** | WebSocket pushes metadata only (no bodies); full bodies are fetched on demand via REST, keeping live updates cheap. |
| **Self‑exclusion of `:api_port` requests** | Prevents the web UI's own API calls from creating a feedback loop in the traffic list. |
| **TLS handshake failures recorded as 502** | Makes failed interception attempts (e.g. cert pinning) visible in the UI instead of silently dropping them. |
| **Embedded web assets (`rust-embed`)** | Single self‑contained binary; no external file dependencies at runtime. |
