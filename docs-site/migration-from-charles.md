---
title: Migrating from Charles Proxy
description: Switch from Charles Proxy to Madhyamas — feature mapping, workflow translation, session import, and a side-by-side comparison of what each tool supports.
---

# Migrating from Charles Proxy

If you currently use [Charles Proxy](https://www.charlesproxy.com/) and want to switch to Madhyamas (or use both), this page maps Charles features to their Madhyamas equivalents and translates common workflows.

## Why Switch?

| | Charles Proxy | Madhyamas |
|---|---|---|
| **License** | Commercial, paid | Open source (MIT OR Apache-2.0) |
| **Runtime** | JVM/Java (requires a JRE) | Single native Rust binary |
| **UI** | Java Swing desktop app | Embedded React web UI (any browser) |
| **API** | HTML-only web interface | Documented REST API (184 endpoints) |
| **AI agent support** | None | Built-in MCP server (146 tools) |
| **gRPC** | Limited (5.2 fixes only) | Dedicated gRPC inspection UI |
| **Scripting** | None | Sandboxed JavaScript (boa_engine) |
| **Plugins** | None | Sandboxed WebAssembly (wasmtime) |

## Feature Mapping

### Core Proxying

| Charles feature | Madhyamas equivalent | Notes |
|---|---|---|
| HTTP proxy | [Getting Started](./getting-started) | Same — set proxy to `localhost:8888` |
| HTTPS/SSL MITM | [HTTPS & Certificates](./https-certificates) | Auto-generated CA; install via the Setup button |
| SOCKS proxy | [SOCKS5 Proxy](./socks-proxy) | SOCKS5 via `--enable-socks --socks-port` |
| WebSocket inspection | [WebSocket Inspection](./websockets) | Frame parsing, fragment reassembly |
| gRPC inspection | [HTTP/2 & gRPC](./http2-grpc) | Enable HTTP/2 downstream first |
| Access control (IP allowlist) | [Access Control](./access-control) | CIDR-based, live updates |
| SSL Proxying host allowlist | All hosts intercepted by default | Use `--no-https` to disable |

### Traffic Recording and Inspection

| Charles feature | Madhyamas equivalent |
|---|---|
| Recording on/off toggle | Recording button in the toolbar (Recording vs Passthrough) |
| Recording size limits | [Recording Limits](./recording-limits) — `max_requests`, `max_total_size_mb`, `max_body_size` |
| Ignore list for recording | `ignored_domains` config with wildcard/suffix matching |
| Structure view (tree by host) | Web UI tree view |
| Sequence view (chronological) | Web UI list view |
| Focus (highlight hosts) | [Focus](./focus) |
| Chart/timeline visualization | [Timeline View](./timeline-view) — waterfall with status-color-coded bars |
| Request/response viewers | Detail panel tabs (Request, Response, Timing, Preview) |
| zstd content-encoding | On-demand decompression toggle in the body viewer |
| Find in session | Search bar and advanced filter builder |

### Sessions

| Charles feature | Madhyamas equivalent |
|---|---|
| Multiple named sessions | [Sessions](./sessions) |
| Save / re-open sessions | Export/import via API or CLI |
| Clear session | `POST /api/traffic/clear` or `madhyamas traffic clear` |
| Switch active session | `POST /api/sessions/{id}/switch` or the Sessions view |
| Session export | HAR + cURL (Charles also supports CSV/Trace/XML) |
| Session import | [Importing HAR Files](./har-import) |

### Modifying Traffic

| Charles feature | Madhyamas equivalent |
|---|---|
| Breakpoints | [Breakpoints](./breakpoints) |
| Map Local / Map Remote | [Rewrites](./rewrites) + [Rewrite Templates](./rewrite-templates) |
| Rewrite | [Rewrites](./rewrites) |
| Block List | [Block List](./block-list) |
| Throttling | [Throttling](./throttling) — latency, bandwidth, packet loss |
| Repeat / Repeat Advanced | [Replay](./replay) — single, edit-then-repeat, and batch |
| Compose (edit & resend) | Edit-then-Repeat in the Replay view |
| Mocks | [Mocks](./mocks) — collections, recording, conditional, probabilistic |

### Tools

| Charles feature | Madhyamas equivalent |
|---|---|
| No Caching / No Cookies | [Rewrite Templates](./rewrite-templates) |
| Mirror | [Mirror](./mirror) |
| Auto Save | [Auto Save](./auto-save) |
| Process info (which app made a request) | Not supported |
| Validate (HTML/CSS validation) | Not supported |

### Export and Import

| Charles feature | Madhyamas equivalent |
|---|---|
| HAR export | `GET /api/export/har` or `madhyamas export har` |
| cURL export | `GET /api/export/curl/{id}` or `madhyamas export curl <id>` |
| HAR import | [Importing HAR Files](./har-import) |
| Mock import/export | `GET /api/mocks/export`, `POST /api/mocks/import` |
| Config import/export | `GET /api/config/export`, `POST /api/config/import` (enterprise) |

## Workflow Translation

### Importing a Charles Session

Charles can export HAR files, which Madhyamas imports directly:

1. In Charles, select a session and choose **File → Export → HAR**.
2. In Madhyamas, import the HAR file:
   ```bash
   curl -X POST -H 'Content-Type: application/json' -d @session.har \
     http://localhost:3001/api/traffic/import/har
   ```
   Or use the web UI's **Import** button. The HAR is imported as a new session — your live capture is untouched. See [Importing HAR Files](./har-import).

### Replacing Map Local with Rewrites

Charles's **Map Local** (serve a local file for a URL) maps to a [rewrite](./rewrites) that replaces the response body. Charles's **Map Remote** (redirect a URL to another server) maps to a rewrite that changes the request URL.

### Replacing Charles Breakpoints

Charles breakpoints pause traffic for manual editing. Madhyamas [breakpoints](./breakpoints) work the same way — set a condition (URL pattern, method, status code), and matching traffic pauses in the web UI for you to inspect and modify before continuing.

### Replacing Repeat Advanced

Charles's **Repeat Advanced** (repeat N times with concurrency) maps to Madhyamas [Repeat Advanced / batch replay](./replay#repeat-advanced-batch-replay) — `madhyamas replay run-advanced <id> --iterations N --concurrency C --delay-ms D`.

## Features Charles Has That Madhyamas Doesn't

These Charles features have no Madhyamas equivalent today:

- Reverse proxy
- Port forwarding (TCP/UDP)
- Auto browser/OS proxy configuration
- NTLM authentication pass-through
- CSV / XML / Trace session export
- `.chlz` session format
- Built-in HTML/CSS validation tool
- Client process tracking (which app made a request)
- Native iOS app

## Features Madhyamas Has That Charles Doesn't

- [MCP server](./mcp) for AI agent integration (146 tools)
- [Scripting](./scripting) — sandboxed JavaScript hooks
- [Plugins](./plugins) — sandboxed WebAssembly extensions
- Documented [REST API](./rest-api) with 184 endpoints
- [CLI](./cli) with 159 subcommands
- [Enterprise](./enterprise/) auth, RBAC, and audit logging

## See also

- [Getting Started](./getting-started) — install and run Madhyamas
- [HTTPS & Certificates](./https-certificates) — install the Madhyamas CA
- [Importing HAR Files](./har-import) — bring Charles captures into Madhyamas
- [CLI reference](./cli) — drive Madhyamas from the terminal
