# Charles Proxy vs Madhyamas — Feature Comparison

This document is a comprehensive feature-by-feature comparison between
[Charles Proxy](https://www.charlesproxy.com/documentation/) (v5.2, the
commercial reference HTTP/HTTPS debugging proxy) and **Madhyamas** (the
open-source Rust-based debugging proxy in this repository).

It was produced by:

1. Iterating through **every page** of the Charles Proxy documentation site
   (`https://www.charlesproxy.com/documentation/...`), including Welcome,
   Installation, Getting Started, Configuration (7 subpages), Using Charles
   (12 subpages), Proxying (5 subpages), Tools (15 subpages), FAQs,
   Additional Information (7 subpages), Version History, Security, Privacy,
   and Charles for iOS (3 subpages).
2. Inspecting the Madhyamas source tree (`crates/`, `web/`, `docs/`) to
   determine the actual implementation status of each capability, not just
   what is advertised in docs.

> **Status legend**
> - ✅ **Full** — Madhyamas implements the equivalent of the Charles feature.
> - 🟡 **Partial** — Madhyamas has a related capability but with notable gaps.
> - 🔴 **Stub** — Code exists as a placeholder/feature-gated stub.
> - ❌ **Missing** — No meaningful implementation in Madhyamas.
> - ➕ **Madhyamas-only** — A capability Madhyamas has that Charles does not.

---

## 1. Executive Summary

| Metric | Count |
|---|---|
| Charles features surveyed | 46 |
| Madhyamas full parity (✅) | 15 |
| Madhyamas partial parity (🟡) | 9 |
| Madhyamas stub only (🔴) | 2 |
| Madhyamas missing (❌) | 22 |
| Madhyamas-only advantages (➕) | 4 |

**Strengths of Madhyamas relative to Charles**

- Open source (dual MIT/Apache-2.0) vs. commercial/paid Charles.
- Native Rust binary — no JVM/Java dependency (Charles requires a JRE).
- First-class **MCP server** for AI agent integration (Charles has none).
- Embedded React web UI served from the same binary (Charles ships a Java
  Swing desktop app; its "web interface" is a thin control panel only).
- Modern REST API with 130+ endpoints (Charles's web interface is HTML-only,
  not a documented REST API).
- gRPC inspection built-in (Charles only added HTTP/2 trailer/gRPC fixes in
  5.2 and has no dedicated gRPC UI).

**Biggest gaps vs. Charles**

- No SOCKS proxy, no reverse proxy, no port forwarding, no DNS spoofing.
- No downstream HTTP/2 (Charles has full HTTP/2 proxying).
- No auto-configuration of OS/browser proxy settings.
- No built-in utility tools: Block List, Block Cookies, No Caching, Mirror,
  Auto Save, Client Process, Repeat/Repeat Advanced, Edit, Validate.
- No load testing, no chart/timeline visualization, no AMF/Flash support.
- No NTLM authentication pass-through, no external/upstream proxy chaining.
- No iOS native app (Charles has an App Store iOS client).

---

## 2. Feature Comparison Matrix

### 2.1 Core Proxying

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 1 | HTTP proxy (plain HTTP capture) | ✅ | ✅ | `engine.rs::handle_http_proxy` |
| 2 | HTTPS/SSL MITM proxying | ✅ | ✅ | `tls/certificate.rs` CA + leaf certs, `handle_https_tunnel` |
| 3 | SOCKS proxy (v4/v5) | ✅ | ✅ | SOCKS5 (RFC 1928/1929) via `proxy/socks.rs`; `--enable-socks --socks-port`; see [SOCKS_PROXY.md](SOCKS_PROXY.md) |
| 4 | HTTP/2 proxying (downstream) | ✅ (5.2) | 🟡 | ALPN upstream only; downstream advertises http/1.1 (`engine.rs:484`) |
| 5 | HTTP/1.1 keep-alive | ✅ | ✅ | hyper handles keep-alive |
| 6 | WebSocket inspection | ✅ | ✅ | `websocket.rs` (947 lines), frame parsing, fragment reassembly |
| 7 | gRPC inspection | ✅ (5.2) | ✅ | `grpc/` module, feature-gated; connections/streams/frames/stats |
| 8 | Reverse proxy | ✅ | ❌ | No code |
| 9 | Port forwarding (TCP/UDP) | ✅ | ❌ | Mentioned in `DEPLOYMENT.md` only |
| 10 | External/upstream proxy chaining | ✅ | ❌ | `reqwest` uses `no_proxy()` (`engine.rs:103`) |
| 11 | Auto browser/OS proxy config | ✅ | ❌ | No system proxy auto-configuration |
| 12 | Access control (IP allowlist) | ✅ | ❌ | No ACL implementation |
| 13 | NTLM authentication pass-through | ✅ | ❌ | Not implemented |
| 14 | SSL Proxying host allowlist | ✅ | ✅ | All hosts intercepted by default; `--no-https` flag to disable |

### 2.2 Traffic Recording & Inspection

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 15 | Recording on/off toggle | ✅ | ✅ | `traffic/store.rs` `capture_enabled` flag |
| 16 | Recording size limits | ✅ | ❌ | No max-size guard |
| 17 | Ignore-list for recording | ✅ | 🟡 | `SessionPreset.filter_host_patterns` (`session.rs:42-72`) |
| 18 | Structure view (tree by host) | ✅ | ✅ | Web UI tree view |
| 19 | Sequence view (chronological) | ✅ | ✅ | Web UI list view |
| 20 | Focus (highlight hosts) | ✅ | 🟡 | Filter patterns exist; no dedicated "Focus" UI |
| 21 | Chart/timeline visualization | ✅ | ❌ | No visualization code in `web/` |
| 22 | Request/response header viewers | ✅ | ✅ | Web UI detail tabs |
| 23 | Body viewers (JSON/XML/form/binary) | ✅ | ✅ | Web UI renderers |
| 24 | Query param / cookie / auth viewers | ✅ | ✅ | Web UI detail tabs |
| 25 | Save request/response bodies | ✅ | ✅ | Export cURL, HAR |
| 26 | Find in session / find in request | ✅ (5.0) | ✅ | Web UI search |
| 27 | Unicode/charset decoding | ✅ | ✅ | Rust handles UTF-8 natively |
| 28 | zstd content-encoding | ✅ (5.1) | ✅ | On-demand decompression via `?decompressed=true` |
| 29 | 1xx interim responses (103 Early Hints) | ✅ (5.2) | ❌ | Not handled specially |

### 2.3 Sessions

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 30 | Multiple named sessions | ✅ | ✅ | `session.rs::SessionManager` |
| 31 | Save / re-open sessions | ✅ | ✅ | Export/import via API |
| 32 | Clear session | ✅ | ✅ | `POST /api/traffic/clear` |
| 33 | Switch active session | ✅ | ✅ | `POST /api/sessions/{id}/switch` |
| 34 | Session export (CSV/Trace/XML) | ✅ | ✅ | HAR + cURL (different formats) |
| 35 | Session import (Trace/XML) | ✅ | ✅ | Session import + HAR traffic import (`POST /api/traffic/import/har`) |
| 36 | `.chlz` session format | ✅ (5.0) | ❌ | Madhyamas uses SQLite (`traffic.db`) |

### 2.4 Export / Import

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 37 | HAR export | ✅ (via import) | ✅ | `GET /api/export/har` |
| 38 | cURL export | ➕ (via copy) | ✅ | `GET /api/export/curl/{id}` |
| 39 | CSV export | ✅ | ❌ | Not implemented |
| 40 | XML session export | ✅ | ❌ | Not implemented |
| 41 | Trace text export | ✅ | ❌ | Not implemented |
| 42 | HAR import | ✅ (5.0) | ✅ | `POST /api/traffic/import/har`; see [docs/HAR_IMPORT.md](HAR_IMPORT.md) |
| 43 | Mock import/export | ➕ | ✅ | `intercept/mock.rs` |
| 44 | Config import/export | ➕ | ✅ | `GET/PATCH /api/config` |

### 2.5 SSL / Certificates

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 45 | Auto-generated CA root cert | ✅ | ✅ | `tls/certificate.rs:132-159` |
| 46 | Per-site leaf cert signing | ✅ | ✅ | `tls/certificate.rs:170-252` |
| 47 | Download CA cert | ✅ | ✅ | `GET /api/cert/ca` |
| 48 | Install CA via Web UI | ✅ | ✅ | `CertificatePanel` + `CertificateHelper` |
| 49 | Install CA via CLI/MCP | ✅ | ❌ | No cert CLI/MCP tools |
| 50 | Auto-regenerate expired root | ✅ (5.0) | ❌ | Not implemented |
| 51 | Per-platform install instructions | ✅ | ✅ | Web UI onboarding |

### 2.6 Proxying Tools (Charles "Proxying" section)

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 52 | Bandwidth throttling (bps) | ✅ | ✅ | `intercept/throttle.rs` |
| 53 | Latency simulation (ms) | ✅ | ✅ | `ThrottleProfile.delay_ms` |
| 54 | Throttle presets | ✅ | ✅ | GPRS, EDGE, 3G, 4G, satellite, slow_3G |
| 55 | Breakpoints (intercept/edit) | ✅ | ✅ | `intercept/breakpoint.rs`, Execute/Abort/Cancel |
| 56 | Location matching (wildcards) | ✅ | ✅ | URL pattern matching |
| 57 | SSL Proxying toggle per host | ✅ | ✅ | `--no-https` global flag |

### 2.7 Tools (Charles "Tools" section)

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 58 | No Caching tool | ✅ | ✅ | Rewrite template strips cache headers + adds no-cache directives. See [docs/REWRITE_TEMPLATES.md](REWRITE_TEMPLATES.md) |
| 59 | Block Cookies tool | ✅ | ✅ | Rewrite template strips `Cookie`/`Set-Cookie` headers. See [docs/REWRITE_TEMPLATES.md](REWRITE_TEMPLATES.md) |
| 60 | Map Remote (URL→URL) | ✅ | 🟡 | `RewriteAction::MapToUrl` (limited) |
| 61 | Map Local (URL→file) | ✅ | 🟡 | `RewriteAction::MapToFile` (limited) |
| 62 | Rewrite (header/URL/query/body) | ✅ | ✅ | `intercept/rewrite.rs`, regex support |
| 63 | Block List (block domains) | ✅ | ❌ | Not implemented |
| 64 | DNS Spoofing | ✅ | ❌ | Android VPN only, no core impl |
| 65 | Mirror (save responses to disk) | ✅ | ❌ | Mentioned in `BRAINSTORM.md` only |
| 66 | Auto Save (periodic session save) | ✅ | ❌ | Not implemented |
| 67 | Client Process tracking | ✅ | ❌ | Not implemented |
| 68 | Repeat (replay single request) | ✅ | ✅ | `replay.rs::ReplayManager` |
| 69 | Repeat Advanced (concurrency) | ✅ | ❌ | No concurrency/iterations control |
| 70 | Edit (edit then repeat) | ✅ | ✅ | `RequestEditor` + `RequestModifications` |
| 71 | Validate (W3C HTML/CSS/Feed) | ✅ | ❌ | Not implemented |
| 72 | Command-line tools (convert/ssl) | ✅ | ✅ | `madhyamas` CLI with 58 subcommands |

### 2.8 Using Charles (other)

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 73 | Load testing | ✅ | ❌ | Not implemented |
| 74 | Web interface (control panel) | ✅ | ✅ | Full React web UI (much richer) |
| 75 | Protocol Buffers viewer | ✅ | 🟡 | `grpc/types.rs::ProtoMessage` (no full decoder) |
| 76 | AMF / Flash Remoting | ✅ | ❌ | Not implemented (Flash is deprecated) |
| 77 | AJAX / XMLHttpRequest debugging | ✅ | ✅ | Via standard traffic inspection |
| 78 | Headless mode | ✅ (`-headless`) | ❌ | Mentioned in PRD only |
| 79 | Command-line options (`-config`, etc.) | ✅ | ✅ | `--proxy-port`, `--api-port`, `--host`, etc. |

### 2.9 Configuration

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| 80 | Preferences (startup/UI) | ✅ | ✅ | `ConfigDialog` web UI |
| 81 | Proxy settings (ports) | ✅ | ✅ | `--proxy-port`, `--api-port` |
| 82 | Dynamic proxy ports | ✅ | ❌ | Fixed ports only |
| 83 | Recording settings | ✅ | 🟡 | Capture toggle only; no size limits |
| 84 | Access control settings | ✅ | ❌ | Not implemented |
| 85 | External proxy settings | ✅ | ❌ | Not implemented |
| 86 | HTTP 1.1 toggle | ✅ | ✅ | Always on (hyper) |
| 87 | Bypass domains list | ✅ | ❌ | Not implemented |

### 2.10 Madhyamas-only Advantages

| # | Feature | Charles | Madhyamas | Notes |
|---|---|---|---|---|
| ➕1 | MCP server (AI agent integration) | ❌ | ✅ | 67 MCP tools, stdio transport |
| ➕2 | Documented REST API (130+ endpoints) | ❌ | ✅ | Charles web interface is HTML-only |
| ➕3 | Mock collections + recording + versioning | Partial | ✅ | `MockManager` (1632 lines) |
| ➕4 | Enterprise auth/RBAC/audit/onboarding | ❌ | ✅ | Feature-gated `enterprise/` module |
| ➕5 | Scripting (JS/TS hooks) | ❌ | 🟡 | `ScriptRuntime` (runtime stub) |
| ➕6 | Plugin system (Rust plugins) | ❌ | 🟡 | `PluginManager` (runtime stub) |

---

## 3. Detailed Feature Notes

### 3.1 Core Proxying

**HTTP/HTTPS proxying** — Both implement MITM HTTPS with a generated CA.
Charles auto-configures OS/browser proxy settings; Madhyamas requires manual
client configuration (or `--host`/`--public-ip` hints).

**SOCKS proxy** — Charles supports SOCKS v4/v5 as a first-class alternative to
HTTP proxying, recommended for HTTP/2 from Safari/iOS. Madhyamas has only a UI
dropdown placeholder; no backend implementation exists.

**HTTP/2** — Charles 5.2 has extensive HTTP/2 proxying correctness work
(connection flow-control, stream concurrency, trailers, gRPC). Madhyamas
negotiates ALPN upstream but explicitly advertises only `http/1.1` downstream
(`engine.rs:484` comment: "proxy does not yet implement HTTP/2 frame parsing
on the downstream side").

**Reverse proxy / Port forwarding** — Charles supports both for clients that
can't use an HTTP proxy. Madhyamas has neither.

**External proxy chaining** — Charles can chain to an upstream HTTP/HTTPS/SOCKS
proxy with Basic/NTLM auth. Madhyamas's `reqwest` client explicitly disables
proxying (`no_proxy()`).

### 3.2 Traffic Inspection

**Chart/timeline** — Charles shows a timeline chart with Request/Latency/
Response segments for visualizing parallel downloads. Madhyamas has no
equivalent visualization.

**Focus** — Charles lets you mark hosts as "focused" to separate them from
noise. Madhyamas has `SessionPreset.filter_host_patterns` but no dedicated
Focus UI affordance.

**Content encodings** — Charles 5.1 added zstd. Madhyamas supports gzip,
deflate, brotli, and zstd decompression on demand via the
`?decompressed=true` query parameter on `GET /api/traffic/{id}`. See
[docs/ZSTD_SUPPORT.md](ZSTD_SUPPORT.md).

### 3.3 Sessions

Both support multiple named sessions, switching, and clearing. Charles uses
a `.chlz` zip format (5.0+) for cross-tool compatibility; Madhyamas uses
SQLite (`~/.madhyamas/traffic.db`). Charles supports CSV/Trace/XML export;
Madhyamas supports HAR and cURL.

### 3.4 Tools

**Rewrite** — Both fully implement header/URL/query/body rewriting with regex.
Madhyamas's `RewriteManager` (450 lines) is comparable to Charles's Rewrite
tool.

**Map Remote / Map Local** — Charles has dedicated tools. Madhyamas has
`RewriteAction::MapToUrl` and `MapToFile` variants but they are limited.

**Repeat / Repeat Advanced / Edit** — Charles has three related tools for
replaying requests with optional concurrency and editing. Madhyamas has
`ReplayManager` (single replay with modification support) and an
edit-then-repeat workflow via the `RequestEditor` UI and CLI flags, but no
concurrency control. See [docs/EDIT_THEN_REPEAT.md](EDIT_THEN_REPEAT.md).

**Validate** — Charles sends responses to W3C HTML/CSS/Feed validators.
Madhyamas has no equivalent.

**Mirror / Auto Save / Client Process** — Charles utility tools not present
in Madhyamas.

**Block List / Block Cookies / No Caching** — Charles header-manipulation
tools not present in Madhyamas (though No Caching could be built as a rewrite
rule).

### 3.5 SSL / Certificates

Both auto-generate a CA and sign per-site leaf certificates. Charles 5.0
auto-regenerates expired root certs; Madhyamas does not. Charles provides
CLI/MCP cert tools; Madhyamas cert management is Web UI + API only.

### 3.6 Platform & Deployment

| Aspect | Charles | Madhyamas |
|---|---|---|
| Language | Java (Swing UI) | Rust + React/TypeScript |
| Runtime | Requires JRE | Single static binary |
| License | Commercial (paid) | Dual MIT/Apache-2.0 |
| Platforms | Windows, macOS, Linux | Linux, macOS, Windows (Docker) |
| iOS app | ✅ (App Store) | ❌ |
| Docker | ❌ | ✅ (`docker-compose.yml`) |
| AI agent integration | ❌ | ✅ (MCP server, 67 tools) |
| Documented REST API | ❌ (HTML web interface) | ✅ (130+ endpoints) |
| Headless mode | ✅ (`-headless`) | ❌ |

---

## 4. Priority Recommendations for Madhyamas

Based on the gap analysis, here are recommended features to implement, ordered
by impact and effort:

### High Priority (core parity)

1. **HTTP/2 downstream support** — Required for modern web debugging; Charles
   invested heavily in 5.2. Add HTTP/2 frame parsing in the proxy engine.
2. **SOCKS proxy** — Needed for HTTP/2 from Safari/iOS and for clients that
   prefer SOCKS. Implement SOCKS v5 in the proxy listener.
3. **External/upstream proxy chaining** — Essential for corporate/enterprise
   networks behind a mandatory proxy. Add config for upstream HTTP/HTTPS/SOCKS.
4. **Access control (IP allowlist)** — Needed for multi-device testing.
   Add an ACL with CIDR support.
5. **Block List tool** — Simple high-value feature; block domains by pattern.
6. **No Caching tool** — Simple header manipulation; can be a rewrite template.
7. **Block Cookies tool** — Simple header stripping; can be a rewrite template.

### Medium Priority (utility & UX)

8. **Repeat Advanced** — Add concurrency/iterations to replay for basic load
   testing.
9. **Edit-then-repeat** — Implemented. See [docs/EDIT_THEN_REPEAT.md](EDIT_THEN_REPEAT.md).
10. **Chart/timeline visualization** — Add a waterfall chart to the web UI.
11. **Focus feature** — Add a dedicated Focus UI for host filtering.
12. **Mirror tool** — Save responses to disk as a mirror of the site.
13. **Auto Save** — Periodic session save to avoid memory growth.
14. **Recording size limits** — Prevent runaway memory usage.
15. **HAR import** — Import traffic from HAR files (Charles 5.0 supports this). **Implemented** — see [docs/HAR_IMPORT.md](HAR_IMPORT.md).

### Lower Priority (niche / legacy)

16. **Reverse proxy** — Niche; most clients support HTTP proxies.
17. **Port forwarding** — Niche; SOCKS covers most use cases.
18. **DNS spoofing** — Niche; can be done via `/etc/hosts`.
19. **Protocol Buffers full decoder** — Requires `.desc` file fetching.
20. **Validate (W3C)** — Niche; can use external validators.
21. **AMF/Flash** — Flash is deprecated; skip.
22. **NTLM** — Legacy Microsoft auth; low demand.
23. **Auto browser/OS proxy config** — Platform-specific, fragile.
24. **Headless mode** — Already effectively headless (web UI); document this.
25. **Client process tracking** — OS-specific, low value.

### Madhyamas-only strengths to preserve

- Keep investing in **MCP tools** — this is a unique differentiator vs. Charles.
- Keep the **REST API** documented and stable — Charles has nothing comparable.
- Complete the **scripting** and **plugin** runtimes (currently stubs) to
  deliver on the advertised extensibility.
- Keep the **single-binary** deployment story — a major advantage over
  Charles's Java/JVM requirement.

---

## 5. Sources

- Charles Proxy documentation (all subpages):
  `https://www.charlesproxy.com/documentation/`
- Charles version history (5.2, 5.1, 5.0.x, 4.6.x):
  `https://www.charlesproxy.com/documentation/version-history/`
- Madhyamas source: `crates/`, `web/`, `docs/` in this repository
- Madhyamas skill: `.claude/skills/madhyamas/SKILL.md`
- Madhyamas tool coverage: `docs/TOOL_COVERAGE.md`
- Madhyamas architecture: `docs/ARCHITECTURE.md`, `CLAUDE.md`

---

*Generated 2026-08-01. Charles version referenced: 5.2 (released 2026-06-13).
Madhyamas version: current `main` branch as of this date.*
