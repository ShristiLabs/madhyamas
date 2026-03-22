# Madhyamas — Consolidated Brainstorm Summary

> Deduplicated from 14 analysis rounds (620+ raw items → ~130 unique findings)
> Generated: 2026-03-22

---

## 🔴 Critical (Fix Immediately)

### 1. Manual HTTP/1.1 Parsing Is the Root Cause of Most Proxy Bugs
**`proxy/engine.rs`** — Naive string splitting instead of using `hyper`/`reqwest`. Causes:
- No chunked transfer encoding, no gzip/brotli, no HTTP/2
- Case-sensitive headers (`HashMap<String, String>`) — violates RFC 7230
- Duplicate headers silently dropped (e.g., multiple `Set-Cookie`)
- Entire response buffered in memory (OOM risk on large payloads)
- `Connection: close` hardcoded — no upstream keep-alive
- Same manual parsing duplicated in `replay.rs` (including a second `SkipServerVerification`)
**Fix:** Use `reqwest` for upstream connections; only use raw TCP for client-facing MITM side.

### 2. Regex Compiled on Every Single Request Match
**`intercept/types.rs`, `mock.rs`, `rewrite.rs`** — Every `MatchCondition` and template regex is compiled fresh on every match. With 10 rules × 3 conditions each = 30 compilations per request.
**Fix:** Pre-compile with `OnceLock` or store `Regex` objects in condition types.

### 3. `PATCH /api/config` Is a No-Op — Changes Silently Discarded
**`api/handlers.rs`** — Clones config, mutates clone, returns it as JSON, but never writes back (Arc is immutable). Frontend ConfigDialog appears to work but every change is lost on restart.
**Fix:** Wrap in `Arc<RwLock<ProxyConfig>>`.

### 4. 13 Handler Functions Defined but Never Registered as Routes
**`api/intercept_handlers.rs` vs `routes.rs`** — Mock analytics, hit history, test, preview, export/import, duplicate, rollback, version history, advanced create — all have working handlers but zero routes. Frontend calls all of them → 404.
**Fix:** Register all missing routes in `routes.rs`.

### 5. `TrafficStore::in_memory()` Used Despite Full SQLite Support
**`cli/main.rs` line 75** — One-line change (`TrafficStore::new(&config.db_path)`) would persist all traffic. Currently all data lost on restart.
**Fix:** Change to `TrafficStore::new()`.

### 6. CORS Allows ANY Origin — Dangerous for Traffic Interception Tool
**`api/lib.rs`** — `allow_origin(Any).allow_methods(Any).allow_headers(Any)`. Any website can read captured traffic (cookies, auth headers) via `localhost:3001`.
**Fix:** Restrict to localhost + private IP. Add security headers (CSP, X-Frame-Options, X-Content-Type-Options).

### 7. No Graceful Shutdown — Connections Abandoned on Exit
**`cli/main.rs`** — Drops `proxy_task` handle immediately, no SIGTERM/SIGINT handler, no connection draining, no SQLite WAL flush.
**Fix:** Add `tokio::signal::ctrl_c()` + `CancellationToken` + drain with timeout.

### 8. CLI Command Files Can't Compile (Corrupted)
**`cli/commands/replay.rs`, `sessions.rs`, `mocks.rs`** — Garbled text, undefined types, invalid syntax, mixed-in code from other files.
**Fix:** Rewrite from scratch based on actual API endpoints.

### 9. JWT "Implementation" Is Just Base64 — Zero Crypto
**`enterprise/auth.rs`** — `generate_jwt()` does `BASE64.encode(serde_json::to_string(claims))`. No signature, no HMAC. Anyone can forge tokens. Also `TokenExpired` variant stores and serializes the full JWT (token leak).
**Fix:** Use `jsonwebtoken` crate. Remove token from error variants.

### 10. API Key Prefix Is `pf_` (ProxyForge Legacy)
**`enterprise/auth.rs`** — `format!("pf_{}", uuid::Uuid::new_v4())`.
**Fix:** Change to `mad_` or `mdh_`.

---

## 🟠 High Priority (This Week)

### Architecture
| # | Issue | Location | Fix |
|---|-------|----------|-----|
| 11 | 55KB monolith `engine.rs` — HTTP/HTTPS paths have ~200 lines duplicated | `proxy/engine.rs` | Extract shared pipeline into `pipeline.rs` |
| 12 | Synchronous SQLite blocks async runtime | `traffic/store.rs` | Use `tokio-rusqlite` or `spawn_blocking` + WAL mode |
| 13 | `Arc::get_mut().unwrap()` in builder — panics if Arc shared | `proxy/engine.rs` | Use `ProxyEngineBuilder` pattern |
| 14 | `ConnectionPool` (complete impl) never wired into proxy | `performance/pool.rs` | Integrate for upstream connections |
| 15 | `MetricsCollector` (complete impl) never called from proxy | `performance/metrics.rs` | Pass reference into engine, mock/breakpoint managers |
| 16 | `MemoryManager` never connected to TrafficStore | `performance/memory.rs` | Wire `entry_added()`/`entry_removed()` |
| 17 | Enterprise subsystem (auth/RBAC/audit) complete but never enforced | `enterprise/` | Add Axum middleware layer; or feature-gate |
| 18 | Plugin system entirely placeholder | `plugin/` | Mark as Phase 5 or implement CORS Helper plugin |
| 19 | Scripting runtime returns hardcoded placeholder | `scripting/runtime.rs` | Integrate `boa_engine` or mark stub |
| 20 | Three error types with no common trait | `core::Error`, `EnterpriseError`, `McpError` | Create unified `AppError` trait |

### API & Backend
| # | Issue | Location | Fix |
|---|-------|----------|-----|
| 21 | Phase 4 handlers all return stubs/zeros | `phase4_handlers.rs` (493 lines) | Remove or gate behind feature flag |
| 22 | `get_session` ignores path param, `delete_session` returns 501 | `handlers.rs` | Implement properly |
| 23 | No rate limiting on any endpoint | `api/lib.rs` | Add `tower_governor` |
| 24 | No request body size limits | `api/lib.rs` | Add `DefaultBodyLimit` layer |
| 25 | No input validation beyond deserialization | All handlers | Add `validator` crate |
| 26 | Inconsistent error response format (3 patterns) | All handler files | Standardize on single `ApiError` type |
| 27 | `save_all_rules` has no auth — anyone can overwrite all rules | `handlers.rs` | Add auth/CSRF protection |
| 28 | `TraceLayer` logs every request (potential data leak) | `api/lib.rs` | Exclude headers/body from logs |

### Frontend
| # | Issue | Location | Fix |
|---|-------|----------|-----|
| 29 | App.tsx logo shows "PF" (ProxyForge) | `App.tsx` header | Change to "M" |
| 30 | No favicon — broken `/vite.svg` reference | `index.html` | Create `web/public/favicon.svg` |
| 31 | `useTrafficCount` polls every 1s even in WebSocket mode | `hooks/useTraffic.ts` | Disable polling when WS active |
| 32 | WebSocket snapshot creates empty headers/body | `hooks/useTraffic.ts` | Lazy-load full entry on click |
| 33 | Bulk enable/disable fires N sequential API calls | `MocksPanel`, `BreakpointsPanel`, `RewritesPanel` | Add batch endpoint |
| 34 | Upstream proxy & capture config saved to localStorage only | `ConfigDialog.tsx` | Add backend endpoints |
| 35 | Appearance tab WebSocket setting uses wrong localStorage key | `ConfigDialog.tsx` | Fix key mismatch |
| 36 | No error boundaries — component crash kills entire UI | `App.tsx` | Add `ErrorBoundary` wrappers |
| 37 | gRPC/Scripts/Plugins panels show as working but are non-functional | 3 panels | Add "🧪 Experimental" badges |
| 38 | OnboardingWizard hits non-existent API endpoints | `OnboardingWizard.tsx` | Implement or hide |
| 39 | MockEditDialog discards all conditions after first | `MockEditDialog.tsx` | Fix or disable multi-condition UI |
| 40 | No virtual scrolling — renders all 10K traffic entries | `TrafficList.tsx` | Add `@tanstack/react-virtual` |
| 41 | Dark mode system preference overrides manual choice | `App.tsx` | Stop system listener when manual override set |
| 42 | WebRTC IP detection — fragile, permission-heavy | `CertificatePanel.tsx` | Use backend `/api/config` instead |
| 43 | ScriptsPanel uses plain textarea — no syntax highlighting | `ScriptsPanel.tsx` | Integrate Monaco or CodeMirror |
| 44 | CertificateHelper hardcodes port 3001 in 5 places | `CertificateHelper.tsx` | Use `apiPort` from config fetch |

### MCP
| # | Issue | Location | Fix |
|---|-------|----------|-----|
| 45 | Tracing writes to stdout — corrupts stdio JSON-RPC protocol | `mcp/main.rs` | Write to stderr |
| 46 | Doesn't handle `notifications/initialized` — protocol violation | `mcp/server.rs` | Check `id.is_null()`, return None |
| 47 | 9 `unwrap()` calls on serialization — can panic | `mcp/server.rs` | Replace with error handling |
| 48 | URL path construction vulnerable to path traversal | `mcp/tools/*.rs` | Sanitize IDs |
| 49 | `ToolExecutor` created fresh per tool call | `mcp/server.rs` | Cache in `McpServer` |
| 50 | 2 registered tools have no executor implementation | `registry.rs` vs `executor.rs` | Implement or remove |

---

## 🟡 Medium Priority (Next 2 Weeks)

### Branding & Cleanup
| # | Issue | Fix |
|---|-------|-----|
| 51 | `stop.sh` says "ProxyForge" | Replace all instances |
| 52 | Homebrew class names are `Proxyforge`/`ProxyforgeCli`/`ProxyforgeMcp` | Rename to `Madhyamas` variants |
| 53 | MSI WiX GUIDs use `ProxyforgeCLIExe` | Rename |
| 54 | `.gitignore` comment says "ProxyForge" | Change to "Madhyamas" |
| 55 | Delete `echo` file (6KB failed build log) | `rm echo` + gitignore |
| 56 | `.mcp.json` has hardcoded macOS developer path | Remove from repo, add to gitignore |
| 57 | Delete stray `.patch` files from `proxy/` directory | Apply or remove |
| 58 | Delete unused `HeadersView.tsx` | Dead component |
| 59 | Delete unused `resizable.tsx` UI component | Use it or remove it |

### Dependencies & Build
| # | Issue | Fix |
|---|-------|-----|
| 60 | `zustand` in package.json — zero imports | Remove or create stores |
| 61 | `react-split`, `@radix-ui/react-separator` — zero imports | Remove |
| 62 | `playwright` devDep — zero tests | Remove until tests exist |
| 63 | `config`, `toml` crates — zero imports | Remove or use for config files |
| 64 | `jsonpath_lib` — zero usage | Integrate for body filtering or remove |
| 65 | No `[profile.release]` optimizations (LTO, strip) | Add to root Cargo.toml |
| 66 | `rust-version = "1.88"` but README says "1.75+" | Align one or the other |
| 67 | CLI args lack `env` attributes (Docker vars ignored) | Add `env` to all clap args |
| 68 | `ViteConfig` sourcemap: true in production | Change to false/hidden |
| 69 | Tailwind content scans 3 non-existent directories | Simplify to `./src/**/*.{ts,tsx}` |

### Documentation
| # | Issue | Fix |
|---|-------|-----|
| 70 | README advertises non-functional features (gRPC, scripts, plugins) | Add status column or "🚧 Early Development" banner |
| 71 | ARCHITECTURE.md: wrong engine (hyper vs manual TCP), wrong port, broken table | Rewrite |
| 72 | API.md: WebSocket events use completely different protocol names | Match actual `TrafficEvent` types |
| 73 | DEPLOYMENT.md: references non-existent `config.toml`, wrong base image | Fix or add warnings |
| 74 | GETTING_STARTED.md: references `config.toml` that doesn't exist | Label as planned or remove |
| 75 | CONTRIBUTING.md: dead links (Discord, Twitter, CONTRIBUTORS.md) | Remove or create |
| 76 | CLAUDE.md: missing API endpoints list, some stale tech references | Update |
| 77 | CHANGELOG.md: placeholder date `2024-XX-XX` | Update or remove |
| 78 | DEVELOPMENT.md: wrong project structure (shows `handlers/` directory) | Fix tree |
| 79 | MOCK_RESPONSES_PLAN.md: duplicate AI header, hardcoded developer paths | Clean up |
| 80 | MCP-INTEGRATION.md: hardcoded developer macOS path | Replace with placeholder |
| 81 | Startup scripts display HTTPS port 8443 that doesn't work | Remove or label "not implemented" |

### CI/CD
| # | Issue | Fix |
|---|-------|-----|
| 82 | Dependabot disabled (`open-pull-requests-limit: 0`) | Set to 5 for cargo/npm |
| 83 | `cargo audit` installed from source every run (~3 min) | Use `cargo-binstall` or cache |
| 84 | Release builds non-existent `madhyamas` package (5 of 15 jobs fail) | Change to `madhyamas-cli` |
| 85 | Release workflow references port 3000 (wrong) | Change to 3001 |
| 86 | Homebrew SHA256 checksums are all `PLACEHOLDER_*` | Auto-update in release workflow |
| 87 | `RUST_BACKTRACE=1` set globally in CI | Scope to test steps |
| 88 | No test coverage measurement | Add `cargo-tarpaulin` or `llvm-cov` |
| 89 | No release test gate | Re-run tests on release tag |
| 90 | Frontend built 6 times in release matrix | Build once, share artifact |

### Docker & Packaging
| # | Issue | Fix |
|---|-------|-----|
| 91 | No `.dockerignore` — `.git/`, `target/`, `node_modules/` in build context | Create `.dockerignore` |
| 92 | `Dockerfile.dev` uses unpinned `rust:latest-slim` | Pin to specific version |
| 93 | `docker-compose.yml` deprecated `version: "3.8"` | Remove the line |
| 94 | Dev service mounts `web/` as `:ro` — Vite can't write cache | Remove `:ro` |
| 95 | `EXPOSE 8443` but no TLS listener exists | Remove |
| 96 | MCP Docker service effectively useless (stdio needs local) | Remove from compose |
| 97 | `startup.sh` builds frontend locally AND Docker builds it again | Remove local build |
| 98 | RPM/AUR reference non-existent `config/default.toml` | Create the file |
| 99 | Snap `base: core22` approaching EOL | Update to `core24` |
| 100 | No `web/public/` directory (favicon, robots.txt) | Create with assets |

---

## 🔵 Architecture / Long-Term

| # | Idea | Impact |
|---|------|--------|
| 101 | Channel-based proxy pipeline (Raw → Parse → Mock → Breakpoint → Rewrite → Throttle → Upstream) | Enables new features without touching core loop |
| 102 | Separate control plane (API) from data plane (proxy) | Enables remote agents, multi-instance scaling |
| 103 | `InterceptHandler` trait for all managers | Eliminates duplicated check patterns |
| 104 | Unified persistence trait (TrafficStore, ReplayManager, WsManager all use SQLite) | Consistent data survival |
| 105 | `hyper` for upstream connections (gets HTTP/2, pooling, compression, chunked encoding for free) | Eliminates #1 root cause |
| 106 | Feature flags for Phase 3/4 (gRPC, scripting, plugins, enterprise) | Reduces binary size, removes stub confusion |
| 107 | OpenTelemetry tracing + Prometheus metrics endpoint | Enterprise observability |
| 108 | MCP `Tool` trait (each tool is self-registering) | Reduces 3-file change per tool to 1 |
| 109 | Frontend feature-based directory structure (`components/mock/`, `components/intercept/`) | Better organization for 22+ components |
| 110 | Shared API client in frontend (eliminate scattered `fetch()` calls) | Centralized error handling, auth injection |

---

## 🟢 Positive Highlights

Despite the issues, several subsystems are exceptionally well-designed:

- **MatchCondition system** — 23 condition types with AND/OR/NOT logic, GraphQL support, 30+ unit tests
- **Mock system** — 4 response types (single/sequence/conditional/probabilistic), templates, recording, import from HAR/OpenAPI/Postman — rivals commercial tools
- **Traffic type serialization** — Clean UTF-8/binary handling with base64 fallback, 25+ tests
- **WebSocket client hooks** — Exponential backoff with jitter, max reconnect, proper cleanup
- **TrafficView UX** — Resizable panels, keyboard shortcuts, multi-select, localStorage persistence
- **Filter system** — 14 field types, 13 operators including regex, client-side application
- **CertificateHelper** — 36KB of platform-specific instructions for 6+ platforms
- **MCP protocol** — Correct JSON-RPC 2.0, proper error codes, 35 tools with JSON Schema
- **Core MetricsCollector** — Lock-free atomics, exponential latency histogram buckets
- **Workspace Cargo.toml** — Clean shared dependency management with consistent version pins
- **Zero `unsafe` blocks** — Entire Rust codebase uses safe Rust
- **TypeScript strict mode** — Maximum strictness with noUnusedLocals/Parameters

---

## 📋 Quick Wins (Under 15 min each)

| Action | Effort |
|--------|--------|
| Change "PF" → "M" in App.tsx | 1 min |
| Fix `ProxyConfig::default().api_port` to 3001 | 2 min |
| Change `TrafficStore::in_memory()` to `TrafficStore::new()` | 2 min |
| Delete `echo` file | 1 min |
| Add `.mcp.json` to `.gitignore` | 1 min |
| Fix `.gitignore` "ProxyForge" comment | 1 min |
| Remove unused deps (`zustand`, `react-split`, `separator`) | 2 min |
| Remove `version: "3.8"` from docker-compose | 1 min |
| Remove `EXPOSE 8443` from Dockerfile | 1 min |
| Fix `stop.sh` "ProxyForge" → "Madhyamas" | 2 min |
| Add `[profile.release]` LTO/strip to Cargo.toml | 5 min |
| Fix Homebrew class names `Proxyforge` → `Madhyamas` | 2 min |
| Add `PRAGMA journal_mode=WAL` to TrafficStore | 5 min |
| Add `prefers-reduced-motion` CSS media query | 2 min |
| Disable Vite sourcemaps in production | 1 min |
| Simplify Tailwind content paths | 1 min |
| Add `"typecheck": "tsc --noEmit"` to package.json | 1 min |
| Add `no-console` ESLint rule | 2 min |
| Create `.dockerignore` | 5 min |
| Enable Dependabot with limit 5 | 2 min |
| Create `.github/ISSUE_TEMPLATE/` | 15 min |
| Create `docs/SECURITY.md` | 10 min |
| Create `.editorconfig` | 5 min |
| Add security headers (CSP, X-Frame-Options) | 10 min |
| Fix MCP tracing to stderr | 5 min |
| Create `web/public/` with favicon + robots.txt | 10 min |

---

## 🎯 Suggested Action Order

### Phase 1: Quick Wins (Today)
1. All branding fixes (#PF→M, ProxyForge→Madhyamas, pf_→mad_)
2. Delete stray files (echo, .mcp.json, .patch files)
3. Fix `ProxyConfig::default().api_port` to 3001
4. Switch to SQLite TrafficStore
5. Add `.dockerignore`, `.editorconfig`, `.gitattributes`

### Phase 2: Security & Correctness (This Week)
6. Fix CORS to localhost-only + add security headers
7. Fix MCP stdio logging, notification handling, unwrap calls
8. Fix config persistence (RwLock wrapper)
9. Register 13 missing API routes
10. Add graceful shutdown handler

### Phase 3: Performance (Next 2 Weeks)
11. Pre-compile regexes (OnceLock)
12. Enable SQLite WAL mode
13. Add virtual scrolling to TrafficList
14. Wire ConnectionPool into proxy engine
15. Wire MetricsCollector into proxy/managers

### Phase 4: Foundation (Next Month)
16. Use `reqwest` for upstream connections (eliminates manual HTTP parsing)
17. Add API integration tests
18. Implement proper JWT with `jsonwebtoken` crate
19. Create config file support (`config.toml`)
20. Add batch API endpoints for bulk operations
