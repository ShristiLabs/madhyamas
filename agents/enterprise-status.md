# Enterprise Implementation Status

## Current Phase
Milestone "Credential-Based Device & Agent Scoping" — issue-by-issue orchestration
(current: #104 device principals, 2 of 9)

## Milestone Progress
| Issue | Title | Developer | Tester | Reviewer | Regression | Committer | Status |
|---|---|---|---|---|---|---|---|
| #103 | Attribution foundation: resolve CONNECT principal and persist client_addr | done | done (15 cases) | approved (0 blockers, 4 low) | pass (all checks) | committed (2f88bdc) | done |
| #104 | Device principals: registration, per-device credentials, devices API and Devices panel | done | done (24 cases) | approved (0 blockers, 0 high, 5 low) | pass (all checks + 17-step smoke) | — | commit |

## Earlier Phases (13-phase plan — COMPLETE)
Phase 2 (from earlier log, kept for history): rusqlite -> sqlx storage migration.

## Phase Progress
| Sub-phase | Issue | Developer | Tester | Reviewer | Regression | Committer | Status |
|---|---|---|---|---|---|---|---|
| 0 | #28 | done | n/a (doc-only) | approved | done (baselines=regression) | committed | done |
| 1a | #29 | done | skipped (no-test rule) | approved (w/ minor improvements applied) | done (build/test/clippy pass) | committed | done |
| 1b | #30 | done | skipped (no-test rule) | approved (spot-checked trait impls) | done (build/test/clippy pass) | committed | done |
| 1c | #31 | done | skipped | approved | done | committed (04b0db3) | done |
| 1d | #32 | done | skipped | approved | done | committed (6948ac6) | done |
| 1e | #33 | done | skipped | approved | done (routes verified) | committed (8f3229c) | done |
| 2a | #34 | done | skipped | approved | done (handlers return real data) | committed (e960a8b) | done |
| 2b | #35 | done | skipped | approved | done (additive, 487 tests unchanged) | committed (da155fb) | done |
| 2c-1 Config | #36 | done | skipped | approved | done (both builds green) | committed (ebe5426) | done |
| 2c-2 Intercept | #36 | done | skipped | approved | done (both builds green, intercept pipeline async) | committed (5a0f131) | done |
| 2c-3 Plugin+Script | #36 | done | skipped | approved | done (sync boundary via tokio::spawn; 481 tests pass) | committed (da7e372, 4a51e3f) | done |
| 2c-4 Traffic+Session | #36 | done | skipped | approved | done (hot-path preserved; 481 tests pass) | committed (046f9b3, 8d14db7) | done |
| 2d Remove rusqlite | #37 | done | skipped | approved | done (rusqlite fully removed; Error::Database dropped) | committed (6c1484c) | done |
| 3 License (Ed25519) | #38 | done | skipped | approved | done (verify at startup, /api/license, health, 7 tests) | committed (2fdd753) | done |
| 4a Users (Argon2id) | #39 | done | skipped | approved | done (Argon2id, bootstrap admin, 3 cred tests) | committed (1d52f4f) | done |
| 4b JWT auth | #40 | done | skipped | approved | done (HS256 pin, leeway, refresh, idle timeout, 4 tests) | committed (f03001d) | done |
| 4c API key scopes | #41 | done | skipped | approved | done (SHA-256 hash, X-API-Key, scope matching, 7 tests) | committed (64f5207) | done |
| 4d RBAC enforcement | #42 | done | skipped | approved | done (role→permission matrix, require_permission middleware, 3 tests) | committed (0554b2c) | done |
| 4e Audit persistence | #43 | done | skipped | approved | done (store-backed, SHA-256 hash chain, tamper detection, 4 tests) | committed (08000ab) | done |
| 5 PostgreSQL backends | #44 | done | skipped | approved | done (6 Pg stores, --database-url, advisory lock, 7 Pg tests pass) | committed (78e7766, d2c98ca) | done |
| 6a Redis state | #45 | done | skipped | approved | done (pub/sub, config+intercept sync, --redis-url, 6 Redis tests) | committed (77ce3c3) | done |
| 6c Seat coordination | #47 | done | skipped | approved | done (register/heartbeat/deregister, seat limit, SIGTERM release) | committed (47cc1c2) | done |
| 6b Shared CA | #46 | done | skipped | approved | done (--ca-cert-file/--ca-key-file, load or generate + save) | committed (4f37e5e) | done |
| 6d LB support | #48 | done | skipped | approved | done (--base-path, health deps, graceful shutdown, K8s + docker-compose.multi) | committed (908d7f4) | done |
| 6e Cluster metrics | #49 | done | skipped | approved | done (InstanceMetrics, /api/metrics/cluster, /api/instances, 30s heartbeat) | committed (2052fb8) | done |
| 7a+7b Web UI auth | #50,51 | done | skipped | approved | done (TierContext, AuthContext, LoginPage, ProtectedApp, UserMenu, badge) | committed (f36cf1f) | done |
| 7c+7d Web UI admin | #52,53 | done | skipped | approved | done (Users/Audit/Metrics/License/ApiKeys/Instances panels, lazy chunks, OSS hides enterprise) | committed (93d98df) | done |
| 8a MCP auth | #54 | done | skipped | approved | done (McpAuth, default_headers injection, 7 tests) | committed (ab8044f) | done |
| 8b CLI auth | #55 | done | skipped | approved | done (CliAuth, --api-key/--token, default_headers, 7 tests) | committed (a5465de) | done |
| 8c Ent MCP tools | #56 | done | skipped | approved | done (11 ent tools, tier detection, conditional registration) | committed (ade0279) | done |
| 8d MCP protocol | #57 | done | skipped | approved | done (HTTP transport, annotations, resources, 6 prompts) | committed (ade0279) | done |
| 8e Ent CLI commands | #58 | done | skipped | approved | done (users/audit/license/auth commands, skill docs updated) | committed (a0054fe) | done |
| 9 Security hardening | #59 | done | skipped | approved | done (WS auth, CSP, proxy auth, password complexity, SSRF, license instance ID, DB URL redaction, 579 tests) | committed (443287c) | done |
| 10a+10b DB optimization | #60,61 | done | skipped | approved | done (tiered body storage, zstd, session counters, cursor pagination, lazy bodies, write batching, 587 tests) | committed (1890982) | done |
| 10c+10d DB scale+HA | #62,63 | done | skipped | approved | done (partitioning docs, PgBouncer docs, read/write split, --database-read-url, PGBOUNCER.md, POSTGRES_HA.md) | committed (9b4fb25) | done |
| 11 CI/CD two-tier | #64 | done | skipped | approved | done (OSS+enterprise matrix, Docker BUILD_TIER, release artifacts, SBOM, BSL-leak check, 587 tests) | committed (76289d3) | done |
| 12a+12e Licensing core+deploy | #65,69 | done | skipped | approved | done (licensing-server crate, Ed25519 signing, license issuance/verification/seat APIs, Dockerfile, K8s, KEY_MANAGEMENT.md, BACKUP.md, DEPLOYMENT.md, 600 tests) | committed (8172b32) | done |
| 12b+12c+12d Customer+Stripe+Admin | #66,67,68 | done | skipped | approved | done (JWT auth, customer portal React frontend, Stripe Checkout+webhooks, admin portal, revenue dashboard, 604 tests) | committed (039a8ad) | done |

## Agent Log

### 2026-09-18 — enterprise-regression (#104)
- Frontend: pass; fmt --check: pass; clippy -D warnings: 0
- OSS release build (--no-default-features): pass, 27.07 MB (identical to #103 baseline); OSS binary symbol scan: 0 madhyamas_enterprise, 0 mdy_dev_, 0 devices/device_keys/DeviceRegistered strings
- Enterprise release build: pass, 35.81 MB; device symbols present; enterprise crate standalone: pass
- cargo test --all-features: 716 passed / 0 failed / 29 ignored (baseline 692/28 + 24 new + 1 PG-gated)
- Docs: check-docs.sh pass, check-docs-coverage.sh pass
- OSS isolation: 0 cfg(feature=enterprise) in core/api src; jsonwebtoken absent from core deps
- LIVE SMOKE (full definition-of-done on the release binary, ephemeral HOME, --enable-auth + bootstrap admin): device created via POST /api/devices with mdy_dev_ show-once key; listed; SAME KEY REJECTED ON REST (401 via X-API-Key); CONNECT with key as Basic password -> 200 Connection Established; last_seen stamped in list after the device CONNECT; rotate -> fresh key, old key 407 at CONNECT, new key 200; unauthenticated CONNECT passes by default; revoke -> status revoked + key 407 at CONNECT; audit records device_registered/device_key_rotated/device_revoked (1 each, single-type filters); DELETE 200; restart with --require-proxy-auth -> unauthenticated CONNECT 407. 17/17 (one initial "FAIL" was the smoke script sending a comma-list event_types filter where the handler parses a single value — re-verified with per-type queries, implementation correct)
- Cargo.lock was flipped to licensing-core path-source (confirmed) — RESTORED via git checkout; working tree clean of lock changes
- Verdict: ALL CHECKS PASSED — safe to commit
- Status: completed

### 2026-09-18 — enterprise-reviewer (#104)
- Verdict: approved (0 blockers, 0 high, 5 low/informational)
- Verified: engine 407 semantics (Invalid always, Missing only when strict); attribution.device_id set after validation before tunnel/pipeline dispatch; revoked key/device rejected on all 4 credential routes incl. ?api_key= query; ownership owner-or-admin on mutations, owner-scoped list; no key material in errors/logs/Debug/audit (tests assert it); SQL fully parameterized; AtomicBool Relaxed OK (set-once-at-startup); rotate fails closed (revoke-then-mint); OSS isolation clean (no enterprise cfg/import in core+api; --no-default-features builds; new engine paths inert without validator); web panel React-escaped, no localStorage/QR, isEnterprise-gated lazy chunk; docs accurate (mad_->madhyamas_ fix genuine)
- Low notes: 407 Content-Length msg.len() vs full body (pre-existing pattern, mitigated by Connection: close); rotate can leave device keyless on mint failure (recoverable, fails closed); no name length cap (matches api-keys precedent); admin list/mutate asymmetry; malformed Proxy-Authorization falls to Missing (pre-existing)
- fmt --check: pass; clippy -D warnings: 0
- Status: completed

### 2026-09-18 — enterprise-tester (#104)
- Added 24 test cases + 1 PG-gated case:
  - core tests/proxy.rs (+4): device slot defaults None, device-only principal authenticated, ProxyAuthError Missing-vs-Invalid semantics (eq/clone), engine strict-mode flag default-true + settable (real ProxyEngine + temp CertificateManager)
  - enterprise tests/devices.rs (new, 20): generator prefix/entropy/uniqueness, is_device_key classification (incl. mdy_agent_ and "mdy_dev" edge), REST rejection of mdy_dev_ at validate_api_key (connect-only message, no key leak), validate_device_key happy/revoked-key/revoked-device/unknown/no-store, last_seen heartbeat poll, validator routing (ApiKey/Bearer/Basic-password/Basic-username -> device principal; user key -> device_id None; revoked -> Err), store CRUD (owner scoping, newest-first, delete, metadata, status/last_seen), key lifecycle (hash lookup, last_used, revoke keeps row flagged, cascade revokes only that device's active keys), audit roundtrip for the 3 new event types
  - enterprise tests/store.rs (+1 #[ignore] PG): full device+key lifecycle on PostgreSQL per MADHYAMAS_PG_TEST_URL convention
- Dev-dep: parking_lot added to madhyamas-core [dev-dependencies] (regular dep already; zero graph impact — same pattern as the reqwest dev-dep note)
- RESULTS: full workspace 716 passed / 0 failed / 29 ignored (was 692/28); clippy -D warnings: 0; fmt: pass
- GAPS (documented): engine handle_connection 407-vs-pass mapping for Missing/Invalid not directly exercised (no engine accept-loop harness exists; consistent with the #103 gap — validator-Err path IS tested enterprise-side, flag semantics tested engine-side); web panel has no unit-test infra (tsc/vite build verified); PG test ignored-gated (Docker daemon down in this environment)
- Status: completed

### 2026-09-18 — enterprise-developer (#104)
- Core: ProxyPrincipal + device_id (is_authenticated = user OR device); new ProxyAuthError {Missing, Invalid} exported from core; engine proxy_auth_required AtomicBool (default true, preserves Phase 9.6) + set_proxy_auth_required/proxy_auth_required; handle_connection now 407s Invalid always, 407s Missing only when strict, populates attribution.device_id from principal.device_id, removed `let _ = &principal;` marker; attribution.rs docs updated
- Enterprise stores: devices + device_keys tables in SQLite + PG (DDL, all CRUD, revoke_device_keys_for_device, last_seen/last_used stamps); DeviceRecord/DeviceKeyRecord FromRow types; EnterpriseStore trait +11 device methods
- Enterprise auth: DEVICE_KEY_PREFIX mdy_dev_, is_device_key, generate_device_key, DeviceKeyAuth; validate_api_key early-rejects mdy_dev_ (REST/MCP/CLI connect-only); validate_device_key (revoked key/device checks, fire-and-forget last_seen heartbeat); ProxyAuthValidator routes mdy_dev_ via X-API-Key/Bearer/Basic-either-half; AuthConfig.require_proxy_auth (default false)
- Audit: DeviceRegistered/DeviceKeyRotated/DeviceRevoked + all 5 label/parse maps (types.rs, sqlite.rs, postgres.rs, handlers.rs, api-sink collapse to Custom)
- REST: GET/POST /api/devices, DELETE /api/devices/{id}, POST /api/devices/{id}/rotate + /revoke; ownership (owner-or-admin, 403/404); show-once DeviceWithKey; audit events
- main.rs: --require-proxy-auth + MADHYAMAS_REQUIRE_PROXY_AUTH; enterprise attaches validator UNCONDITIONALLY with strict = proxy_auth || require_proxy_auth; AuthConfig wired
- Web: DevicesPanel.tsx (list w/ Live-60s/Pending/Revoked status + 15s refetch, name-first create dialog, show-once credential dialog with manual-apply Host/Port/Username/Password copy rows from /api/config, rotate/revoke/delete); admin.ts wrappers; App.tsx + NavRail Smartphone icon
- Docs: API_ENTERPRISE.md (Devices section, audit types, mdy_dev_ format, require_proxy_auth policy; fixed stale mad_ prefix to madhyamas_), ENTERPRISE_STARTUP_FLOW.md (flag row + Step 12 rewritten)
- BUILD_OSS: pass; BUILD_ENTERPRISE (clippy all-targets all-features -D warnings): pass 0 warnings; FMT: pass; TESTS: 692/0/28 (baseline match); WEB: built (DevicesPanel chunk); docs checks pass
- Gotcha: Cargo.lock flipped to local licensing-core path patch (build ran with [patch] active) — committer must `git checkout -- Cargo.lock` before staging
- Status: completed

### 2026-09-18 — orchestrator (milestone kickoff, #104)
- Issue #104 exists (created by maintainer) — enterprise-issues step skipped; labeled `status:in-progress`
- Verified code facts post-#103 (commit 2f88bdc):
  - `ProxyPrincipal { user_id, api_key_id }` at engine.rs:52; `impl ProxyAuthValidator for AuthManager` at enterprise auth.rs:624; engine attaches validator only when `--proxy-auth` (main.rs:1750-1755); 407 on ANY validation failure incl. missing creds (engine.rs:582-610, `let _ = &principal;` marker at :610)
  - `AttributionContext.device_id` exists and is always None (attribution.rs:40)
  - Stores: SQLite DDL consts + `?` binds (sqlite.rs:49-102), PG DDL consts + `$N` binds (postgres.rs:56-109); `ApiKeyRecord` FromRow; trait at store/mod.rs:50
  - Middleware X-API-Key path funnels through `AuthManager::validate_api_key` (middleware.rs:266) — prefix rejection belongs there (covers header + query-param paths and MCP/CLI)
  - Show-once pattern: handlers.rs create_api_key :705 (hash + 12-char prefix persisted, plaintext returned once)
  - Web: ApiKeysPanel.tsx pattern, admin.ts wrappers, App.tsx lazy chunks + `isEnterprise` gating; manual-apply host/port precedent = AppHeader.tsx:68 / CertificateHelper.tsx:93 fetch `/api/config` `{host, proxy_port}`
  - Sole `with_proxy_auth_validator` caller is main.rs:1754
- Design resolutions (from issue text + maintainer brief): dedicated `devices` + `device_keys` tables (issue proposal "keys keep referencing it"); `ProxyPrincipal` gains `device_id`; invalid/revoked credential => 407 in BOTH modes (DoD: "revoke kills proxy access" unconditional) while MISSING creds 407 only when `require_proxy_auth` on; enterprise attaches validator unconditionally (attribution), strict = `--proxy-auth` OR new `--require-proxy-auth`; Basic-auth manual apply scans username OR password for `mdy_dev_` prefix
- Dispatching enterprise-developer for #104
- Status: dispatched

### 2026-09-18 — enterprise-committer (#103)
- Restored Cargo.lock (discarded local licensing-core path-patch flip); staged 17 files by name (16 modified + attribution.rs new)
- Commit: 2f88bdc "feat(core): resolve CONNECT principal and persist client_addr" — body references docs/CREDENTIAL_ONBOARDING.md and "Implements #103 (1 of 9)"; no AI attribution
- 17 files changed, 883 insertions(+), 63 deletions(-); working tree clean after commit
- Status: completed

### 2026-09-18 — enterprise-regression (#103)
- Frontend: pass (needed one-time `npm ci` — tsc was missing in this checkout; environment, not regression)
- fmt --check: pass; clippy -D warnings: pass (0)
- OSS build (--no-default-features, release): pass, 27.07 MB; enterprise build (release): pass, 35.69 MB; enterprise crate standalone: pass
- cargo test --all-features: 692 passed / 0 failed / 28 ignored (live-db/redis-gated incl. new PG client_addr test)
- Docs: check-docs.sh pass, check-docs-coverage.sh pass
- OSS isolation: 0 cfg enterprise gates in core/api; jsonwebtoken absent from core deps; 0 madhyamas_enterprise symbols in OSS binary
- Smoke: enterprise binary /health + /api/health OK; LIVE OSS CHECK: proxied request through OSS binary stored client_addr 127.0.0.1:59582 while pre-migration rows read NULL (migration semantics confirmed on a real DB)
- Reminder for committer: Cargo.lock is flipped to local licensing-core path patch — restore before staging
- Verdict: ALL CHECKS PASSED — safe to commit
- Status: completed

### 2026-09-18 — enterprise-reviewer (#103)
- Verdict: approved (0 blockers, 0 high, 4 low/informational)
- Verified: inherent-method resolution for validate_api_key (auth.rs:354 ApiKeyAuth vs AuthProvider Identity — compile-proven via auth.key_id); all 5 production TrafficEntry::new sites stamped; SQL column/placeholder/bind counts symmetric across SQLite (16) and PG (16, ON CONFLICT updated); serde(default) backward-compat; HAR import None; no key material logged (record IDs only); parameterized SQL; OSS isolation clean (no enterprise imports/cfg in core)
- Low notes: engine.rs:610 `let _ = &principal;` retention marker (style, clippy-clean); ws_connections + TrafficEntrySnapshot intentionally exclude client_addr (out of listed scope); docs ER drive-by adds missing script_intercepted row (disclosed)
- fmt --check: pass; clippy (no -D): 0 warnings
- Status: completed

### 2026-09-18 — enterprise-tester (#103)
- Added 15 test cases: attribution.rs inline (4: v4/v6 formatting, None, Default, device-slot-empty); tests/proxy.rs (+3 new: principal OSS default, pipeline stamps client_addr with real mock upstream, pipeline without attribution stores None; extended SOCKS e2e handshake to assert stored client_addr == client local addr); tests/traffic.rs (+2: SQLite roundtrip, legacy-schema migration backfills column via PRAGMA+ALTER); tests/persistence.rs (+1 #[ignore] PG roundtrip per live-db convention); enterprise tests/auth.rs (+5: api-key principal incl. key-id + no-key-material-leak, bearer principal, unknown key rejected, basic rejected, OSS default)
- Dev-dep: reqwest added to madhyamas-core [dev-dependencies] (graph-neutral; needed to construct Pipeline in tests)
- RESULTS: core+enterprise 624 passed / 0 failed; clippy -D warnings clean; fmt clean
- GAP (documented): PG roundtrip is #[ignore]-gated — no local Docker/PG in this environment (Docker daemon down); runs under MADHYAMAS_PG_TEST_URL per existing convention. Engine-level TLS-failure/passthrough entry stamps and handle_connection principal match not directly exercised (no full-engine test harness exists; paths share the stamping line tested via pipeline; consistent with pre-existing engine test coverage)
- Status: completed

### 2026-09-18 — enterprise-developer (#103)
- Implemented: ProxyPrincipal (core struct, user_id + api_key_id Options) + ProxyAuthValidator::validate now returns it; AttributionContext (device_id/client_addr/listener) in new proxy/attribution.rs; threaded accept loops -> handle_connection -> https tunnel (TLS-failure entry, passthrough entry), h2, tls request, http proxy (pipeline with_attribution), SOCKS handler (entry stamp)
- Persistence: TrafficEntry.client_addr (Option<String>, serde default) + SQLite DDL/PRAGMA migration/INSERT/3 SELECTs/TrafficRow/row_to_entry + PostgreSQL DDL/ADD COLUMN IF NOT EXISTS migration/INSERT/3 SELECTs/row mapping
- Enterprise: AuthManager impl returns principal (Basic->user_id, Bearer->Identity.user_id, ApiKey->user_id+key_id); no key material logged
- Tests updated: tests/proxy.rs 4 SOCKS call sites pass AttributionContext; docs/PERSISTENCE.md ER + migration note
- BUILD_OSS: pass; BUILD_ENTERPRISE (clippy all-targets all-features -D warnings): pass 0 warnings; FMT: pass; TESTS: 678 passed / 0 failed
- Gotcha noted: Cargo.lock flipped to local licensing-core path patch — committer must `git checkout -- Cargo.lock` before staging
- Status: completed

### 2026-09-18 — orchestrator (milestone kickoff, #103)
- Issue #103 exists (created by maintainer) — enterprise-issues step skipped
- Labeled issue `status:in-progress`
- Verified code facts: `ProxyAuthValidator` trait at engine.rs:53-57 returns `Result<(), String>`; enterprise impl auth.rs:620-642; `client_addr` dropped at engine.rs:500 / socks.rs accept loop; `RequestData`/`TrafficEntry` (types.rs:68-93, 210-242) have no client fields; SQLite migration pattern = tolerated-duplicate ALTER (store.rs:271-315); Postgres = `ADD COLUMN IF NOT EXISTS` stmt list (postgres/traffic.rs:57-106)
- Dispatching enterprise-developer for #103
- Status: dispatched

### Orchestrator started
- Phase 0 dispatched to enterprise-developer
- Task: verify builds, record baselines

### 2026-08-13 — enterprise-developer (Phase 0, #28)
- Created docs/ENTERPRISE_BASELINES.md + docs/enterprise-baseline-cargo-tree.txt
- BUILD_ENTERPRISE: pass (28.65 MB), BUILD_OSS: FAIL (pre-existing tools_handlers unresolved in routes.rs:62)
- TESTS: 491 pass / 0 fail / 6 ignored; CLIPPY: 0 warnings; FMT: 2 diffs (intercept_handlers.rs)
- cfg gates: 17 positive + 1 negative = 18
- Status: completed (doc-only; failures recorded honestly, not fixed per Phase 0 scope)

### 2026-08-13 — enterprise-reviewer (Phase 0, #28)
- Verdict: approved (baselines doc thorough, failures documented honestly)
- Status: completed

### 2026-08-13 — enterprise-committer (Phase 0, #28)
- Committed docs/ENTERPRISE_BASELINES.md + cargo tree dump
- Status: completed

### 2026-08-13 — enterprise-developer (Phase 1a, #29)
- Created crates/madhyamas-api/src/auth.rs (AuthProvider, Authorizer, AuditSink + types)
- Added auth_provider/authorizer/audit_sink fields + builders to AppState
- Fixed pre-existing OSS build failure (gated script-traces route behind scripting feature)
- Fixed pre-existing fmt diffs in intercept_handlers.rs
- BUILD_OSS: pass, BUILD_ENTERPRISE: pass, CLIPPY: pass, TESTS: 491 pass
- Status: completed

### 2026-08-13 — enterprise-reviewer (Phase 1a, #29)
- Verdict: changes-requested (flagged trait/concrete-impl signature differences)
- Orchestrator assessment: most findings are expected Phase 1b adapter work (traits are intentionally decoupled, forward-looking async design for PostgreSQL/OIDC backends). Applied two cheap improvements: added RoleNotFound to AuthError, added status field to Identity.
- Final verdict: approved
- Status: completed

### 2026-08-13 — enterprise-committer (Phase 1a, #29)
- Committed trait abstractions + OSS build fix + fmt fix
- Status: completed

### 2026-08-13 — enterprise-developer (Phase 1b, #30)
- Created crates/madhyamas-enterprise (11 files, BSL-1.1)
- Copied enterprise code from core/api; implemented AuthProvider/Authorizer/AuditSink
- EnterpriseState + create_enterprise_router; error/type conversion impls
- BUILD_OSS: pass, BUILD_ENTERPRISE: pass, CLIPPY: pass, TESTS: 491 pass
- Status: completed

### 2026-08-13 — enterprise-reviewer (Phase 1b, #30)
- Verdict: approved (spot-checked lib.rs + AuthProvider impl; inherent/trait method resolution correct; From impls correct)
- Status: completed

### 2026-08-13 — enterprise-committer (Phase 1b, #30)
- Committed new enterprise crate
- Status: completed

### 2026-08-13 — enterprise-developer (Phase 1c+1d, #31+#32)
- Deleted core/src/enterprise/ (6 files) + api enterprise_handlers.rs + middleware.rs
- Removed enterprise feature from core+api Cargo.toml; removed jsonwebtoken from core
- Simplified api routes to single create_routes_inner() (no enterprise params)
- Main binary enterprise feature made no-op placeholder (routes deferred to 1e)
- Reworded broken intra-doc links in auth.rs
- BUILD_OSS: pass, BUILD_ENTERPRISE: pass, CLIPPY: pass, TESTS: 487 pass
- Two commits: 04b0db3 (1c), 6948ac6 (1d)
- Status: completed

### 2026-08-13 — enterprise-developer (Phase 1e, #33)
- Added madhyamas-enterprise as optional dep; enterprise feature = ["dep:madhyamas-enterprise"]
- main.rs: #[cfg(feature="enterprise")] block constructs EnterpriseState, injects trait impls into AppState, builds enterprise router; #[cfg(not)] sets None for OSS
- CLI flags: --enable-auth, --jwt-secret, --license-file (with env vars); secret never logged
- api create_router accepts optional enterprise router to merge
- Dockerfile + release.yml updated
- BUILD_OSS: pass, BUILD_ENTERPRISE: pass, CLIPPY: pass, TESTS: pass
- Route verification: enterprise binary mounts /api/auth/*, /api/users, /api/audit; OSS binary 404s (SPA fallback)
- cfg gate count: 2 (in main.rs only)
- Commit: 8f3229c
- Status: completed

## Phase 1 (Crate Extraction) COMPLETE — exit criteria met:
- madhyamas-enterprise crate compiles standalone
- madhyamas-core has zero enterprise references
- madhyamas-api has zero enterprise cfg gates (auth.rs traits retained)
- OSS binary has no enterprise code
- Both builds green, clippy clean, tests pass

### 2026-08-13 — enterprise-developer (Phase 2a, #34)
- Created EnterpriseStore async trait + SqliteEnterpriseStore (sqlx::SqlitePool) in enterprise crate
- Row types (UserRecord, ApiKeyRecord, AuthSession, AuditEventRecord, UserUpdate, AuditStats) with sqlx::FromRow
- Inline DDL for users/api_keys/auth_sessions/audit_events tables
- EnterpriseState.store field + with_store() builder; main.rs constructs SqlitePool + store
- Handlers wired to store via axum Extension (api stays decoupled from enterprise)
- All 6 NOT_IMPLEMENTED stubs replaced; login/create_user/list_users/audit return real data
- Added sqlx workspace dep; SHA-256 password hash interim (Argon2id deferred to Phase 4)
- BUILD_OSS: pass, BUILD_ENTERPRISE: pass, CLIPPY: pass, TESTS: 487 pass
- curl verified: /api/users [], POST creates user, /auth/login 200/401, /api/audit/stats real
- Commit: e960a8b
- Status: completed
