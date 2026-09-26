# Enterprise Implementation Status

## Current Phase
Milestone "Credential-Based Device & Agent Scoping" — issue-by-issue orchestration
(current: #107 IN PROGRESS — #108+ NOT started per maintainer instruction)

## Milestone Progress
| Issue | Title | Developer | Tester | Reviewer | Regression | Committer | Status |
|---|---|---|---|---|---|---|---|
| #103 | Attribution foundation: resolve CONNECT principal and persist client_addr | done | done (15 cases) | approved (0 blockers, 4 low) | pass (all checks) | committed (2f88bdc) | done |
| #104 | Device principals: registration, per-device credentials, devices API and Devices panel | done | done (24 cases) | approved (0 blockers, 0 high, 5 low) | pass (all checks + 17-step smoke) | committed (a9a0e9e) | done |
| #105 | Per-device traffic visibility: device sessions, device_id filter end-to-end, connected status | done | done (12 cases; caught + fixed legacy-DB migration-order blocker) | approved (0 blockers, 0 high, 1 medium info, 5 low) | pass (all checks + 10/10 two-device DoD smoke on pre-migration DB) | committed (7cfce6b) | done |
| #106 | QR enrollment: madhyamas://connect payload, enrollment tokens, live status loop | done | done (21 cases) | approved (0 blockers, 0 high, 1 medium, 6 low) | pass (all checks + 20/20 DoD smoke) | committed (b6d8b32) | done |
| #107 | Feature-scope taxonomy: endpoint/tool mapping and MCP tool filtering | — | — | — | — | — | active |

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

### 2026-09-18 — orchestrator (milestone kickoff, #107)
- Issue #107 exists (maintainer-created, OPEN) — enterprise-issues step skipped; full chain dispatched for #107 ONLY (#108+ explicitly out of scope per maintainer brief)
- Verified code facts post-#106 (commit b6d8b32):
  - CRITICAL: auth_middleware is applied ONLY on the enterprise router (enterprise/src/router.rs:153) — the OSS /api routes (traffic, mocks, rewrites, breakpoints, throttle, replay, blocklist, config, sessions, focus, mirror, logs, autosave, persistence, ws-traffic, grpc, scripts, plugins, secrets, cert, ws) merged under /api in api/src/lib.rs:446-489 have NO auth even in enterprise builds with --enable-auth. required_scope's traffic/mocks/... branches (middleware.rs:174-205) have never actually executed. Issue #107's DoD (traffic:read key reaching /api/traffic, 403 on /api/mocks) REQUIRES extending middleware coverage to the whole /api surface in the enterprise tier
  - Middleware facts: PUBLIC_PATHS + is_public_path handle full + /api-stripped forms (middleware.rs:71-116); API-key arm checks required_scope then inserts AuthUser (middleware.rs:275-302); JWT arm unchanged; scope_authorized + Scope::matches support `*` wildcards in either half (auth.rs:211-215)
  - required_scope today derives read/write/delete from method and maps resources traffic(/traffic+/sessions)/mocks/rewrites/breakpoints/throttle/blocklist/focus/scripts/plugins/config(/config+/secrets)/users/audit/rbac; /auth//onboarding/license/health/metrics/performance → None (no scope); /devices → None (unmapped)
  - WS: /api/ws authenticates INSIDE ws_handler via ?token= or Sec-WebSocket-Protocol (handlers.rs:1218-1280, Phase 9 design — middleware cannot reject before the upgrade extractor); web client sends JWT ?token= (useTrafficWebSocket.ts buildTrafficWsUrl). Middleware must exempt /ws; key-principal WS deferred to #108 (device-filtered stream)
  - Enterprise-router key-reachable surface today (the ONLY routes scope enforcement actually ran on): /auth/logout|me|validate|api-keys (None-mapped → any valid key), /devices* (None → any key, owner checks in handlers), /users (users:read/write), /rbac (rbac:*), /audit (audit:*), /onboarding (None), /config/export (config:read), /config/import (config:write), /metrics+/performance+/instances (None). OSS routes: unauthenticated for everyone
  - RBAC BYPASS FACT: require_permission_middleware passes API-key principals through with NO RBAC check (middleware.rs:438-441, "scope already enforced" — but required_scope returned None for /devices etc.), and create_api_key accepts ANY scopes from ANY authenticated user incl. `*` (handlers.rs:705-740). A regular user's `*` key today bypasses admin RBAC on /users and /audit/clear
  - Existing scope vocabulary (web ApiKeysPanel.tsx:39): traffic:read, traffic:write, mocks:read, mocks:write, config:read, config:write, `*` — plus arbitrary strings via raw API
  - MCP: madhyamas-mcp does NOT depend on madhyamas-enterprise (clean graph; scope matching must be local). tools/list serves registry unfiltered (server.rs:1013-1022 HTTP transport; :255/:302 stdio). ToolAnnotations already carries required_permission (Madhyamas extension, types.rs:197-209); enterprise tools already annotate (e.g. users:read at tools/enterprise.rs:41); OSS tools have NO annotations yet. Tier detection via GET /api/health/detailed with auth headers (server.rs:56-127); McpAuth ApiKey/Jwt/None injects default_headers
  - No principal-scope introspection endpoint exists: /api/auth/me returns UserInfo (id/username/email/role) without scopes (handlers.rs get_current_user)
- Design resolutions (maintainer brief + issue text):
  - Taxonomy adopted verbatim from CREDENTIAL_ONBOARDING.md table: traffic:read/export, mocks:read/write, rewrites:read/write, breakpoints:read/write, blocklist:read/write, throttle:read/write, replay:execute, config:read/write, sessions:read
  - (a) per-feature read/write split: YES (brief + issue both propose)
  - (b) config:write for agents: YES, in (noise control)
  - (c) replay:execute: YES, opt-in only
  - (d) exclusions: brief says "owner/JWT-only" but issue/doc say "owner/user-key only" — CONFLICT flagged to maintainer before dispatch (per standing instruction to ask on per-issue design decisions); recommendation: JWT-only for key/device/user-admin/scripts/plugins/traffic-deletion/session-switch (closes the `*`-key RBAC bypass), keep /auth/me|logout|validate key-reachable (self-identity), document as the reconciliation
  - Middleware coverage plan: move auth enforcement from the enterprise router to the whole /api nest (enterprise build only; OSS build untouched; require_auth=false still passes everything); base-path-aware path normalization; /api/ws exempt (handler-level auth stays); static assets outside /api stay public
  - MCP plan: extend /api/auth/me to include key scopes; MCP server fetches principal scopes at startup/first tools/list, filters tool list via required_permission + local wildcard matcher; annotate all tools with their scope
- Status: dispatched (pending maintainer answer on (d))

### 2026-09-18 — enterprise-regression (#107)
- Frontend (tsc+vite): pass; fmt --check: pass; clippy --all-targets --all-features -D warnings: 0
- OSS release build (--no-default-features): pass, 27,117,136 bytes (baseline 27.10 MB); symbol scan: 0 madhyamas_enterprise, 0 route_access, 0 jwt:only/JWT-only, 0 effective_scopes
- Enterprise release build: pass, 35,923,056 bytes (baseline 35.87 MB); enterprise crate standalone: pass
- cargo test --all-features: 792 passed / 0 failed / 31 ignored — EXACT baseline match (tester's count), zero regressions
- Docs: check-docs.sh pass, check-docs-coverage.sh pass; cfg-enterprise gates in core/api src: 0
- Disk: freed debug incremental (19G debug dir, 3.7G free at start) — builds + tests completed on 5.8G
- LIVE DoD SMOKE (enterprise release binary, repo-root cwd, ephemeral HOME, --enable-auth + bootstrap admin): 17/17 PASS —
  traffic:read key: GET /api/traffic 200, POST /api/mocks 403, GET /api/auth/me reports scopes [traffic:read, sessions:read];
  mocks:write key: POST /api/mocks 201 round-trip (three initial 422s were smoke payload shape, not implementation; correct CreateMockRequest verified), GET /api/mocks 403 (read half denied);
  `*` legacy key: traffic 200, users/devices/api-keys all 403 (RBAC bypass closed);
  unauth /api/traffic + /api/config 401 (visible behavior change confirmed), bogus key 401;
  public paths 200: /api/health, /api/license, /api/cert/ca; POST /api/devices/enroll garbage → 400 (shape validation, no 500);
  WS bad token → 401 pre-upgrade;
  viewer JWT: GET /api/users 403 (RBAC unchanged), GET /api/traffic 200 (pass-through);
  MCP traffic:read key tools/list: 17 tools, ZERO mock tools (DoD "cannot even discover mock tools");
  MCP mocks:write key: 22 tools incl. all 19 mock tools, no traffic tools;
  MCP `*` key: 99 tools, zero jwt:only-annotated tools visible (script-traces correctly traffic:read);
  MCP no-auth: 146 tools unfiltered (OSS/None degrade);
  auth-off parity instance: unauth GET /api/traffic + /api/config 200;
  server log grep for key/password material: 0
- OBSERVATION (pre-existing, not a regression): unmatched /api/* paths fall through to the SPA fallback (200 index.html) outside the auth layer — static-only content identical to the public web root; deny-by-default for unmapped-but-REGISTERED routes is enforced and test-pinned (unit + scopes.rs stub-router). Follow-up material: consider a JSON 404 for unmatched /api/* paths under auth
- Cargo.lock licensing-core path-patch flip: RESTORED via git checkout (working tree = source changes + status log only, no lock changes)
- Verdict: ALL CHECKS PASSED — safe to commit
- Status: completed

### 2026-09-18 — enterprise-reviewer (#107)
- Verdict: approved (0 blockers, 0 high, 1 medium informational, 6 low)
- Verified: whole-route cross-check of routes.rs + enterprise router against route_access — zero mismatches vs decisions D1-D4; public list exact-match with near-miss tests (/api/wsfoo, /api/ws-traffic, /api/cert, /api/cert/ca/anything stay non-public); JwtOnly enforced BEFORE scope matching (alias expansion/`*` cannot reach excluded surface; effective_scopes adds only sessions:read/traffic:export); require_permission_middleware now rejects key principals (bypass closed, defense in depth); layering = merge → auth layer → /api nest → base-path nest (middleware sees nest-stripped path in root AND base-path deployments, pinned by api router tests with a stub middleware); require_auth=false and OSS (api_auth=None) unchanged; MCP fallback correctly scoped (fetch only under McpAuth::ApiKey, None on any failure, tools/call unfiltered so stale discovery never breaks legit calls; REST enforces live); scope_satisfies is a faithful mirror of Scope::matches (granted-side wildcards, bare `*` = `*:*`, colonless grants match nothing colonful); every tool in both registries annotated (deny-by-default hides unannotated; jwt:only sentinel hides excluded-surface tools; cert tool public()); secret hygiene clean (audit/tracing carry user_id + key_id record IDs only; no key/token material anywhere in the diff); api crate enterprise-free (ApiAuthMiddleware is a pure axum/std type alias); tester's path-strip fix correct (strip "/api" not "/api/" keeps leading slash; both forms classify identically; old required_scope fully removed, no dependents); audit-flood guard limits key Login events to the /auth surface
- Medium (informational, non-blocking): MCP fetches key scopes ONCE at McpServer::new — scope changes/rotation need an MCP restart to reflect in tools/list discovery (REST enforces live; record in close-out; future refresh-on-tools/list material)
- Low: std::mem::forget runtime leaks in new MCP tests (deliberate, test-only); starts_with prefix matching means future /trafficx-style routes classify as traffic not deny-by-default (no such route today; new routes need explicit map entries); non-preflight OPTIONS classifies non-read (unreachable — CorsLayer answers preflights outside the nest); create_api_key still lets any JWT mint `*` keys (pre-existing posture; no escalation remains — a `*` key grants strictly less than its user's JWT); auth/me mock test doesn't assert X-API-Key header sent (default_headers inherited from 8a); docs "Unmapped paths are a routing matter (404)" is JWT-principal-only accurate (unauth unmapped gets 401)
- fmt --check: pass; clippy --all-targets --all-features -D warnings: 0; OSS check compile: pass
- Status: completed

### 2026-09-18 — orchestrator (#107 design settled, chain resumed)
- Maintainer settled BOTH open decisions; no open questions remain; dispatching straight through the chain (developer -> tester + reviewer -> regression -> committer) per instruction
- Decision 1 (exclusions are JWT-only): key/device management, user/admin endpoints, scripts/plugins, traffic deletion, session switching require a JWT web-session principal; API keys (all kinds) get 403 there. API keys keep self-identity endpoints only (/api/auth/me, /api/auth/logout, /api/auth/validate). This CLOSES the pre-existing RBAC bypass (middleware.rs:438-441 key pass-through on permission-gated routes + handlers.rs:705-740 any-user any-scope key creation incl. `*`). Reconciliation to document in close-out: pre-existing user keys with `*` lose ONLY the excluded routes; traffic/mocks/config/etc. reach unaffected
- Decision 2 (auth middleware covers the whole /api surface under --enable-auth): today enterprise-only (router.rs:153); consequence: unauthenticated GET /api/traffic returns 401 (today 200) with --enable-auth. Web UI / CLI / MCP send credentials (unaffected); OSS builds and auth-off deployments unchanged; /api/ws keeps in-handler ?token= auth (middleware exempts it); #106's public POST /api/devices/enroll must remain public (verify + preserve)
- Taxonomy calls to record in close-out: read/write split YES, config:write IN, replay:execute opt-in
- Re-verified at HEAD (b6d8b32): middleware.rs:438-441 pass-through intact; agents/enterprise-status.md is the sole uncommitted change (carried per instruction)
- Dispatching enterprise-developer for #107
- Status: dispatched
- Verified regression pass; fmt no-op; Cargo.lock NOT in status (restored by regression, not staged)
- Staged 16 files by name (15 modified + tests/enrollment.rs new; agents/enterprise-status.md included — carries #105 committer/close-out leftover bookkeeping + all #106 pipeline entries)
- Commit: b6d8b32 "feat(enterprise): QR enrollment tokens and connect-device onboarding" — body references docs/CREDENTIAL_ONBOARDING.md phase 2 journey steps 3-4, contains "Implements #106 (4 of 9)"; no AI attribution; author = user
- 16 files changed, 1783 insertions(+), 34 deletions(-); working tree clean after commit; NOT pushed (maintainer pushes)
- Status: completed

### 2026-09-18 — orchestrator (#106 close-out)
- Full chain green: issues (skipped — maintainer-created) -> developer -> tester (+21 cases, 749/0 suite) -> reviewer (approved, 1 medium + 6 low documented) -> regression (all checks + 20/20 live DoD smoke) -> committer (b6d8b32)
- Issue #106 closed with completion comment (implementation summary, smoke results, decisions: token-mode QR default with key= builder support; POST /api/devices unchanged; separate issue-token + public enroll endpoints; redeem = retire-and-remint for one live credential per device; wizard unmounted note; rotate-vs-token follow-up flagged)
- Milestone position: 4 of 9 complete (#107+ NOT started per maintainer instruction)
- Status: done

### 2026-09-18 — orchestrator (milestone kickoff, #106)
- Issue #106 exists (maintainer-created, OPEN) — enterprise-issues step skipped; full chain dispatched for #106 ONLY (#107+ explicitly out of scope per maintainer brief)
- Verified code facts post-#105 (commit 7cfce6b):
  - Devices REST: router.rs:95-99 (GET/POST /api/devices, DELETE /{id}, POST /{id}/rotate|revoke); handlers.rs create_device:844 mints show-once key via mint_device_key:969 (SHA-256 hash_api_key at rest, 12-char prefix); load_owned_device:989 enforces owner-or-admin
  - Device key surface: auth.rs DEVICE_KEY_PREFIX mdy_dev_ (:231), is_device_key:236, generate_device_key:244 (32 hex), validate_device_key:471 (revoked key/device checks + last_seen heartbeat); REST rejected at validate_api_key:412
  - Audit: AuditEventType::{DeviceRegistered,DeviceKeyRotated,DeviceRevoked} (audit.rs:37-41); label/parse maps in store/types.rs:212/233, sqlite.rs:645, postgres.rs:711, handlers.rs:1386
  - Auth middleware: PUBLIC_PATHS (middleware.rs:71) + is_public_path:85 strip /api prefix — the enroll endpoint must be added there (companion has no JWT; the token IS the credential)
  - Web: DevicesPanel.tsx CredentialDialog:457 renders show-once CopyRows (Password + Host/Port/Username from /api/config {host, public_ip, proxy_port}); WS live status via useWebSocket + buildTrafficWsUrl watching Added events carrying device_id (:107-118); viewTraffic dispatches `madhyamas:view-device-traffic` CustomEvent (:123-127); admin.ts device wrappers :206-249
  - QR infra: qrcode.react QRCodeSVG used at CertificateHelper.tsx:842 (size 160, level M)
  - Onboarding: steps API is ENTERPRISE-ONLY (handlers.rs get_onboarding_status:1494, 5 hardcoded steps; router.rs:112-117); OnboardingWizard.tsx renders steps via apiGet('/onboarding') — currently UNMOUNTED in the app (pre-existing, BRAINSTORM.md H20); OSS has no /onboarding route at all
  - Tier detection: useTier() hook (web/src/contexts/TierContext.tsx:72), isEnterprise = tierInfo?.tier === "enterprise"
- Design resolutions (issue text + doc QR payload/Transport-security sections + maintainer brief):
  - QR mode decision: token= is the DEFAULT (QR never carries a standing secret; photographed QRs expire). key= variant exists in the payload builder only (manual-mode/future companion use); raw-credential fallback renders exactly as today
  - Endpoint shape: POST /api/devices UNCHANGED (returns show-once DeviceWithKey — preserves #104 semantics + manual fallback); NEW POST /api/devices/{id}/enrollment-token (JWT, owner-or-admin) issues mdy_enroll_ token; NEW PUBLIC POST /api/devices/enroll {token} redeems → returns {device, key} and mints a FRESH mdy_dev_ key (create-time plaintext is unrecoverable — only its hash is stored)
  - Token: mdy_enroll_ + 32 hex (mirrors device keys), SHA-256 at rest, 15-min TTL enforced at redeem, single-use via atomic UPDATE ... WHERE redeemed_at IS NULL (rows-affected guard), device revoke/delete cascades outstanding tokens, opportunistic expired-token cleanup on issue
  - Redeem semantics: revoke the device's existing active keys + mint exactly one fresh key (one live credential per device — matches rotate semantics, kills the photographed-dialog key once the real device enrolls)
  - QR payload: madhyamas://connect?host&port&tls=0&token&name&ca={origin}/api/cert/ca&api={origin}/api — tls field exists and round-trips but is 0 until #110; ca/api derived client-side from window.location.origin (web UI is served by the API server — same source the dialog uses)
  - Dialog: QR (token) primary + "waiting for device… connected — capturing" live status from existing WS Added events + auto-navigate to ?device= via the existing custom event; manual section keeps CopyRows + screenshot warning + instant Rotate
  - Wizard: enterprise-only `device` step after `proxy` in get_onboarding_status + frontend case; client-side filter (useTier) so the step never renders in OSS even if returned
  - No key/token material in logs, WS events, or audit metadata (device_id only)
- Dispatching enterprise-developer for #106
- Status: dispatched

### 2026-09-18 — enterprise-developer (#106)
- Auth: ENROLLMENT_TOKEN_PREFIX mdy_enroll_ + is_enrollment_token + generate_enrollment_token (32 hex) + ENROLLMENT_TOKEN_TTL_SECS=900; validate_api_key early-rejects enrollment tokens (REST/MCP/CLI); ProxyAuthValidator rejects them on all three arms (Basic/Bearer/ApiKey) — exchange credentials, not proxy credentials
- Store: device_enrollment_tokens table (SQLite + PG DDL) + EnrollmentTokenRecord; trait methods create/get_by_hash/redeem (atomic single-use: UPDATE ... SET redeemed_at WHERE redeemed_at IS NULL AND revoked_at IS NULL AND expires_at > now, rows-affected==1)/revoke_for_device/delete_expired (returns count); revoke_device + delete_device now cascade enrollment tokens; opportunistic expired-token cleanup on issuance
- Audit: DeviceEnrollmentIssued + DeviceEnrolled variants; all 5 label/parse maps (audit.rs api-sink collapse, types.rs, sqlite.rs, postgres.rs, handlers.rs) — metadata carries device_id + expires_at/via ONLY, never token/key material (verified: server logs contain 0 mdy_enroll_/mdy_dev_ occurrences across two live runs)
- REST: POST /api/devices/{id}/enrollment-token (JWT, owner-or-admin via load_owned_device, 409 on revoked device) -> {device, token, expires_at}; PUBLIC POST /api/devices/enroll {token} -> DeviceWithKey (400 bad shape / 401 unknown-expired-used-revoked, indistinguishable against enumeration); redeem retires existing keys + mints exactly one fresh mdy_dev_ (create-time plaintext is unrecoverable — only its hash is stored); enroll added to PUBLIC_PATHS + is_public_path strip list; POST /api/devices UNCHANGED (#104 semantics preserved)
- Web: DevicesPanel CredentialDialog rework — QR (QRCodeSVG) carrying madhyamas://connect?host&port&tls=0&token&name&ca&api (buildConnectUri helper, ca/api from window.location.origin; token= default, key= supported by builder for manual mode), 1s countdown to expiry + regenerate, live "Waiting for device… / Connected — capturing" from panel WS capturingAt (entries arriving after dialog open), auto-navigate to ?device= via madhyamas:view-device-traffic after 1.5s, manual section ALWAYS rendered (Password/Host/Port/Username CopyRows + Camera screenshot warning + instant Rotate); admin.ts DeviceEnrollmentToken + createEnrollmentTokenApi
- Onboarding: get_onboarding_status inserts optional `device` step after `proxy` (total 6); OnboardingWizard renders the case (5-step how-to incl. screenshot-expiry note) + client-side tier filter (useTier) so the step never shows in OSS; step counter uses filtered length
- Docs: API_ENTERPRISE.md — enrollment endpoints, audit types, QR payload format + token rules (TTL/single-use/one-live-key/cascade/pruning)
- VERIFIED: fmt pass; clippy --all-targets --all-features -D warnings 0; tests 729/0/0 (baseline match); OSS release build 27.10MB pass; enterprise release build 35.87MB pass; web tsc+vite pass; LIVE SMOKE (enterprise binary, --enable-auth): issue-noauth 401, redeem-public 200 (fresh mdy_dev_), second-redeem 401, REST-with-token 401, CONNECT-with-token 407, create-key-dead-after-redeem 407, redeemed-key CONNECT 200, expired-token (backdated row) 401, garbage 400, unknown 401, audit issue+redeem recorded without secret material, onboarding steps [welcome,certificate,proxy,device,features,tips], served UI chunk contains madhyamas://connect
- Gotchas: Cargo.lock flipped to local licensing-core path patch (build ran with [patch] active — committer must git checkout -- Cargo.lock); local default-feature builds disk-serve web/dist from CWD (embedded-assets is release-workflow-only) — run smoke from repo root or set MADHYAMAS_WEB_DIR
- Status: completed

### 2026-09-18 — enterprise-tester (#106)
- Created tests/enrollment.rs (20 cases): token shape (prefix/length/hex/uniqueness, classification incl. mdy_dev_/mdy_agent_/no-underscore/empty), REST rejection (validate_api_key error names credential type, no token leak), proxy rejection on all four arms (ApiKey/Bearer/Basic-password/Basic-username; error points at redemption; no-store fails closed), store lifecycle (CRUD+unknown-hash, single-use sequential, concurrent 4-way redeem exactly-one-winner on a 1-connection pool, TTL past/future, revoke cascade + other-device untouched, prune counts expired-only incl. redeemed-expired), audit roundtrip for both new event types, handlers (issue: TTL ~15min asserted, hash-at-rest, 404/403/409 ownership; enroll: returns FRESH mdy_dev_ that authenticates via validate_device_key with device_name, create-time key dies at redemption, single-use 401, bad-shape 400, unknown 401, expired 401, revoked-device 401; audit issue+redeem recorded via synchronous in-memory ring with NO token/key material in serialized events and device_id in metadata; redeem event attributed to owner), onboarding device step present after proxy + optional + total matches
- tests/store.rs (+1 #[ignore] PG): full enrollment lifecycle on PostgreSQL per MADHYAMAS_PG_TEST_URL convention (redeem-once, TTL, cascade, prune=1, hash-at-rest)
- RESULTS: workspace 749 passed / 0 failed (baseline 729; +20 runnable, +1 PG-gated ignored); clippy -D warnings: 0; fmt: clean (2 fmt passes applied)
- GAPS (documented): web dialog (QR render/countdown/WS status loop/auto-nav) has no unit-test infra — covered by tsc/vite build + developer served-chunk smoke + regression live DoD; engine accept-loop CONNECT with redeemed key covered at validator level + live smoke only (no engine harness — consistent with #104/#105 gaps); middleware public-path bypass for /devices/enroll covered by live smoke (no in-process router middleware tests exist); PG test ignore-gated (Docker daemon); tarpaulin not installed — coverage estimated manually (all new store/auth/handler surfaces exercised on SQLite)
- Status: completed

### 2026-09-18 — enterprise-reviewer (#106)
- Verdict: approved (0 blockers, 0 high, 1 medium, 6 low)
- Verified: atomic single-use/TTL/revoked redemption via rows-affected CAS in BOTH stores (parameterized SQL, symmetric binds); enumeration resistance (unknown/expired/used/revoked all 401, bad shape 400); no token/key material in errors, audit metadata (device_id/expires_at/via only), WS events, or server logs (grep-clean live runs); issuance owner-or-admin via load_owned_device + 409 on revoked device; cascade complete for device revoke/delete; route table builds (POST /devices/enroll static vs DELETE /devices/{id} dynamic — no conflict, live-verified); PUBLIC_PATHS covers full + nest-stripped forms; enrollment tokens rejected on all 4 proxy-auth arms + REST; audit label/parse symmetry across all 5 maps + serde snake_case matches labels; onboarding device step server-side enterprise-only + client-side useTier filter; OSS isolation clean (zero core/api diff; sole grep hit pre-existing pubsub.rs doc comment)
- Medium (follow-up material, not blocking): rotate_device_key does not revoke outstanding enrollment tokens — the dialog's Rotate is labeled "rotate if exposed" but a photographed dialog leaks key AND token; post-rotate the token stays redeemable <=15 min and attacker redemption retires the fresh key (recoverable by re-rotate; revoke-device fully remediates today). Paired web half: CredentialDialog re-arm effect keys on [deviceId] so rotate (same id) shows the new password with the OLD QR token. Recommended: revoke tokens in rotate + re-key the effect on the issued object
- Low: QR renders while host=="" during the /api/config fetch window (gate on resolved host); enroll consumes token before mint (mint failure leaves device keyless — fail-closed, mirrors accepted #104 rotate ordering); no device_id index on device_enrollment_tokens (matches device_keys precedent); lexicographic RFC3339 SQL comparison documented-correct for the uniform Utc writer; no-op .replace in token generator (replicated pattern); rapid New-QR clicks can race responses (harmless)
- fmt --check: pass; clippy --all-targets --all-features: 0 warnings; tests 749/0
- Status: completed

### 2026-09-18 — enterprise-regression (#106)
- Frontend (tsc+vite): pass; fmt --check: pass; clippy --all-targets --all-features -D warnings: 0
- OSS release build (--no-default-features): pass, 27,100,528 bytes (baseline 27.10 MB, unchanged); symbol scan: 0 madhyamas_enterprise, 0 mdy_enroll_, 0 device_enrollment, 0 DeviceEnrolled/DeviceEnrollmentIssued, 0 enrollment-token
- Enterprise release build: pass, 35,873,328 bytes (baseline 35.87 MB); enterprise crate standalone: pass
- cargo test --all-features: 749 passed / 0 failed / 31 ignored (baseline 729/0/30; +20 runnable +1 PG-gated — zero regressions in existing)
- Docs: check-docs.sh pass, check-docs-coverage.sh pass; cfg-enterprise gates in core/api src: 0
- LIVE DoD SMOKE (enterprise release binary, repo-root cwd so web/dist disk-serves, ephemeral HOME, --enable-auth + admin login): 20/20 PASS — login JWT; device created (mdy_dev_ key); enrollment token issued (mdy_enroll_, no-JWT issue 401); PUBLIC redeem 200 returning fresh mdy_dev_; second redeem 401; redeemed key CONNECT 200; create-time key dead after redeem 407; token on REST 401; token at CONNECT 407; garbage shape 400; unknown token 401; backdated-expiry token 401; audit has device_enrollment_issued x2 + device_enrolled x1 with NO token/key material in any serialized event; GET /api/onboarding lists the device step; served DevicesPanel chunk contains madhyamas://connect; server log grep for mdy_enroll_/mdy_dev_ = 0
- Cargo.lock licensing-core path-patch flip: RESTORED via git checkout (working tree 15 modified + 1 new test file, no lock changes)
- Verdict: ALL CHECKS PASSED — safe to commit
- Status: completed

### 2026-09-25 — enterprise-committer (#105)
- Verified regression pass; cargo fmt no-op; Cargo.lock NOT in status (restored by regression, not staged)
- Staged 29 files by name (28 modified + tests/device_sessions.rs new; agents/enterprise-status.md included — #105 bookkeeping incl. tester/reviewer/regression log entries)
- Commit: 7cfce6b "feat: per-device traffic sessions, device_id filter and live status" — body references docs/CREDENTIAL_ONBOARDING.md journey step 5, contains "Implements #105 (3 of 9)"; no AI attribution; author = user
- 29 files changed, 1800 insertions(+), 86 deletions(-); working tree clean after commit; NOT pushed (maintainer pushes)
- Status: completed

### 2026-09-25 — orchestrator (#105 close-out)
- Full chain green: issues (skipped — maintainer-created) -> developer -> tester (resumed interrupted run; +12 cases; fixed real legacy-DB migration-order blocker + test deadlock) -> reviewer (approved) -> regression (all checks + 10/10 live DoD smoke) -> committer (7cfce6b)
- Issue #105 closed with completion comment (implementation summary, DoD smoke results, test counts, deviations: migration-order fix + deadlocked test helper rewrite, PG test ignore-gated, ownership-scoping flagged as future issue)
- Milestone position: 3 of 9 complete (#106 QR enrollment NOT started per maintainer instruction)
- Status: done

### 2026-09-25 — enterprise-regression (#105)
- Frontend (tsc+vite): pass; fmt --check: pass; clippy --all-targets --all-features -D warnings: 0
- OSS release build (--no-default-features): pass, 27.10 MB (baseline 27.07); symbol scan: 0 madhyamas_enterprise, 0 mdy_dev_, 0 device_keys/DeviceRegistered/device_session, 0 session_for_device
- Enterprise release build: pass, 35.82 MB (baseline 35.81); device symbols present; enterprise crate standalone: pass
- cargo test --all-features: 729 passed / 0 failed / 30 ignored (baseline 716/29; +13/+1 — all new #105 tests; zero regressions in existing)
- Docs: check-docs.sh pass, check-docs-coverage.sh pass; cfg-enterprise gates in core/api src: 0
- LIVE DoD SMOKE (enterprise release binary, ephemeral HOME, PRE-MIGRATION DB hand-built with client_addr but WITHOUT device_id + legacy row): 10/10 PASS — server up w/ --enable-auth + admin login (JWT); two devices created (mdy_dev_ keys, distinct ids); two authenticated proxied clients + one unauthenticated through ONE instance; alpha view exactly /from-alpha with device_id+session device-<id1>; beta view exactly /from-beta likewise; unknown device_id → empty; unfiltered view = /global + legacy pre-migration row only, ALL device_id null, no device-row leakage; /api/sessions lists "Device: Alpha Phone" + "Device: Beta Tablet" under deterministic ids; both devices' last_seen stamped (connected)
- Post-smoke DB inspection: device_id column added by startup migration to the legacy schema, idx_requests_device created, legacy-row device_id NULL in default-session, device rows in device-<uuid> sessions
- Cargo.lock licensing-core path-patch flip: RESTORED via git checkout (working tree now 28 files, no lock changes)
- Verdict: ALL CHECKS PASSED — safe to commit
- Status: completed

### 2026-09-25 — enterprise-reviewer (#105)
- Verdict: approved (0 blockers, 0 high, 1 medium informational, 5 low)
- Verified: SQLite migration-order fix correct (SCHEMA_CORE no longer creates idx_requests_device; unconditional post-migration CREATE INDEX covers fresh + legacy DBs; PG ordering already correct: CORE → ADD COLUMN IF NOT EXISTS → optimized stmts); ensure_device_session uses parking_lot::Mutex scoped BEFORE any .await in both stores (no lock-across-await); deterministic device-{id} upsert + name-rename propagation + process cache idempotent under concurrent first-entries (ON CONFLICT); session_for_device(None) == current_session_id() and the switch_session/instance_state global flow is untouched; device-filter SQL parameterized via push_bind in both backends and replaces the session predicate ONLY when the filter is set; WS Added AND Updated snapshots carry device_id (both go through TrafficEntrySnapshot::from); TrafficQuery/get_sessions shape matches web sessions.ts (extra TS fields optional); useTraffic useWebSocket:false + useWebSocket(url,onMessage,autoConnect,reconnect) signatures verified; enterprise device_name plumbs without key material; docs API_TRAFFIC/PERSISTENCE accurate incl. the Updated-event claim
- OSS isolation: no cfg-enterprise or enterprise imports in core/api src (sole mention = pre-existing doc comment in api/pubsub.rs); jsonwebtoken absent from core; ?device= view/chip/panel enterprise-gated
- Medium (informational, not a regression): /api/traffic?device_id= and the now-real /api/sessions are not ownership-scoped — any authenticated user can query any device's traffic and see other users' device-session names. Matches the pre-existing global-traffic-visibility model (traffic was never user-scoped); per-user traffic ACLs are out of #105 scope — flag for a future issue (cf. #109 device-scoped rules)
- Low: ensure_device_session error-comment's FK caveat is PG-accurate but SQLite-inaccurate (FKs declared, not enforced — no PRAGMA foreign_keys=ON; fallback insert succeeds); concurrent same-device first entries may double-upsert (idempotent, harmless); DevicesPanel capturingAt map bounded by device count; OSS user pasting ?device= URL silently gets the global view (by design); MCP traffic tools do not expose the device dimension (out of the issue's listed end-to-end scope — future issue material)
- fmt --check: pass; clippy --all-targets --all-features -D warnings: 0
- Status: completed

### 2026-09-25 — enterprise-tester (#105, resumed after interrupted run)
- Resumed the interrupted test pass (previous session modified test files without executing/reporting); verified, fixed, extended, and ran everything
- Verified/kept from the interrupted run: traffic.rs (+4: device roundtrip + filter scoping incl. unknown-device and global-scope-unchanged, session_for_device(None)==global, cross-instance idempotency via two stores on one DB file + rename propagation, legacy-schema device_id migration), persistence.rs (+1 #[ignore] PG roundtrip/filter/session-name), devices.rs (+2 device_name assertions on DeviceKeyAuth and device principal), device_sessions.rs (real-engine acceptance tests)
- Fixed a REAL implementation bug caught by the legacy-schema tests: `idx_requests_device` sat in SCHEMA_CORE, which runs BEFORE the PRAGMA-checked ALTER — opening any pre-#105 SQLite DB failed with "no such column: device_id" (upgrade breaker). Moved index creation to post-migration only (fresh DBs covered by the unconditional post-migration CREATE INDEX; PG ordering was already correct: migrations before optimized-index stmts)
- Fixed a test hang in device_sessions.rs TLS-failure test: raw CONNECT helper used read_to_end with no timeout; the engine's TLS acceptor waits for a COMPLETE ClientHello, so a truncated garbage record left both sides blocked forever. Now reads the 200 with a 3s bound, then shuts the socket down — the EOF fails the engine's handshake, recording the device-attributed 502 entry (the behavior under test)
- Fixed legacy-schema test seeding (sessions table now created before INSERT), 2 clippy field_reassign_with_default (struct-update syntax), fmt diffs
- Added (new this pass): api tests/router.rs (+3: get_traffic ?device_id= scopes response + entries carry device_id/session_id, no-param keeps global scope with null device_id, get_sessions returns real rows incl. "Device: Alpha Phone" with the web-client shape)
- CASES: 12 added by tester (4 device_sessions incl. the two-device DoD acceptance through a real engine with WS-snapshot device assertions, 4 traffic.rs, 1 PG-gated, 3 api) + 2 developer-inline in types.rs = +14 total
- RESULTS: workspace 729 passed / 0 failed / 30 ignored (baseline 716/29; +13 pass, +1 PG ignore); clippy -D warnings: 0; fmt: pass
- GAPS (documented): engine passthrough entry stamp (successful non-intercepted CONNECT) not directly exercised — needs TLS passthrough config + trusted upstream; shares the stamping lines with the covered TLS-failure point (same pattern as the #103-documented gap); pipeline short-circuit (mock/breakpoint) entry path not directly hit; PG test #[ignore]-gated (no Docker daemon in this env; runs under MADHYAMAS_PG_TEST_URL); web changes verified by tsc/vite build only (no web unit-test infra, consistent with #103/#104)
- Environment notes: freed 12G (target/debug/incremental) after a disk-full rmeta failure; killed one hung test binary from the interrupted run
- Status: completed

### 2026-09-18 — enterprise-developer (#105)
- Core types: `TrafficEntry.device_id` (nullable, serde default, follows the exact client_addr pattern) + `TrafficFilter.device_id`; `device_session_id()`/`device_session_name()` helpers in traffic::types (deterministic `device-{id}` scheme); `TrafficEntrySnapshot.device_id` (serde default) so WS events carry the device
- Attribution: `AttributionContext.device_name` (display metadata, inert in OSS) + `ProxyPrincipal.device_name`; engine copies both after validation
- Sessions: `TrafficStoreBackend::session_for_device(device_id, device_name) -> String` on the trait; SQLite + PG `ensure_device_session` upsert (`ON CONFLICT (id) DO UPDATE SET name/updated_at`), process-local name-keyed cache (rename propagates, zero steady-state roundtrips), fallback-to-global on error so capture never breaks; cross-instance idempotent by construction (deterministic id, no instance_state coupling)
- Stamping: all five entry-construction points (pipeline short-circuit, pipeline main, engine TLS-failure, engine passthrough, SOCKS tunnel) resolve the session via session_for_device and stamp entry.device_id; HAR import stays None
- Persistence: SQLite DDL + PRAGMA migration + idx_requests_device index + INSERT(17 cols)/3 SELECTs/TrafficRow/row_to_entry; PG mirror (CREATE TABLE, ADD COLUMN IF NOT EXISTS under advisory lock, index, INSERT $17, SELECTs, row map); get_traffic with device filter queries `WHERE r.device_id = ?` instead of the global-session predicate (both backends)
- API: `TrafficQuery.device_id` → filter; `get_sessions` now returns real `list_sessions()` rows (SessionResponse shape unchanged)
- Enterprise: `DeviceKeyAuth.device_name` from the device record → `device_principal` fills `ProxyPrincipal.device_name`; user/bearer/api-key principals explicitly device_name: None
- Web: types (TrafficEntry/TrafficFilter.device, snapshot.device_id); useTraffic sends `device_id` param + WS client-side scoping (device entries excluded globally, included only when matching filter — WS/REST parity); TrafficView accepts deviceFilter prop + `?device=` URL (enterprise-gated), forces REST polling for device views, syncs shareable URL; TrafficToolbar device chip; App.tsx custom-event navigation from Devices panel; DevicesPanel "view traffic" action + live "Connected — capturing" via WS Added events carrying device_id (buildTrafficWsUrl exported)
- Docs: API_TRAFFIC.md (device_id param + entries carry device_id + real sessions), PERSISTENCE.md (device_id column + index in ER + migration list + per-device session rows section)
- BUILD_OSS check: pass; clippy all-targets all-features: 0 warnings; fmt: pass; web build (tsc + vite): pass
- Status: completed

### 2026-09-18 — orchestrator (milestone kickoff, #105)
- Issue #105 exists (created by maintainer) — enterprise-issues step skipped; dispatching full chain for #105 only (#106+ explicitly out of scope per maintainer brief)
- Verified code facts post-#104 (commit a9a0e9e):
  - `AttributionContext { device_id, client_addr, listener }` threaded; `device_id` populated from `ProxyPrincipal` at engine.rs:695; five entry-construction sites stamp `client_addr` from attribution (pipeline.rs:257/437→614, engine.rs:847/963, socks.rs:632) — all pull `session_id = current_session_id()`
  - `TrafficEntry.client_addr` (serde default) persisted in SQLite (store.rs) + PG (postgres/traffic.rs) with PRAGMA/ADD COLUMN IF NOT EXISTS migrations — device_id follows the exact pattern
  - Engine/socks/pipeline hold `Arc<dyn TrafficStoreBackend>` (storage/mod.rs:58) — the per-device session helper belongs on the trait, implemented by both backends
  - `TrafficFilter` (types.rs:366) has no device param; `TrafficQuery` (handlers.rs:19) mirrors it; `get_sessions` (handlers.rs:234) is a hardcoded "Default Session" stub; `list_sessions` exists on both backends
  - Session sync: `switch_session` persists `current_session_id` to `instance_state`; `sync_current_session` (store.rs:1401) pulls it — device sessions instead use a DETERMINISTIC id (`device-{device_id}`) + upsert-by-id so every instance resolves the same row without coordination
  - Enterprise `validate_device_key` (auth.rs:467) already fetches the DeviceRecord (name available) → principal gains `device_name` for session naming; core-inert
  - WS events: `TrafficEntrySnapshot` (events.rs:27) carries session_id but not device_id — snapshot gains device_id (serde default) so the web client can scope live views and the Devices panel can derive "capturing"
  - Web: useTraffic.ts fetch params + client-side WS filter; TrafficToolbar/filters.ts generic ActiveFilter model; App.tsx view switching is state-based (no router) → `?device=` URL param + custom navigation event from DevicesPanel
- Design resolutions (issue text + maintainer brief): device entries go to the per-device session (unfiltered/global view = current global session, unchanged for OSS); when `TrafficFilter.device_id` is set, `get_traffic` queries by `device_id` instead of the global session predicate; OSS frontend ignores `?device=` (no device UI); device-filter picker chip in toolbar is enterprise-gated
- Dispatching enterprise-developer for #105
- Status: dispatched

### 2026-09-18 — enterprise-committer (#104)
- Verified regression pass; fmt no-op; Cargo.lock NOT modified (restored earlier, not staged)
- Staged 25 files by name (23 modified + tests/devices.rs and DevicesPanel.tsx new; agents/enterprise-status.md included per task brief — covers #103 leftover bookkeeping + #104 log)
- Commit: a9a0e9e "feat(enterprise): device principals, mdy_dev_ credentials, devices API and panel" — body references docs/CREDENTIAL_ONBOARDING.md, contains "Implements #104 (2 of 9)"; no AI attribution; author = user
- 25 files changed, 2366 insertions(+), 93 deletions(-); working tree clean after commit; not pushed
- Status: completed

### 2026-09-18 — orchestrator (#104 close-out)
- All pipeline stages green; issue closed with completion comment (full DoD smoke results incl. REST rejection, rotate/revoke 407s, require_proxy_auth, audit events)
- Milestone position: 2 of 9 complete; next #105 follows separately per maintainer instruction
- Status: done

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
