# Gateway Capability Scoping (Decision Document)

Status: draft for maintainer review
Issue: #85 (spike/scoping — no features are implemented by this document)

## Summary

Users increasingly ask Madhyamas for lightweight API-gateway behavior in front
of dev/test APIs: route, rate-limit, and transform traffic while debugging it.
Madhyamas already sits in the request path with a mature interception layer,
so it is well positioned to offer **debuggable gateway behaviors** — but a
deliberate line must be drawn before feature requests accrete into an
undesigned gateway competing with Kong, Envoy, and Traefik.

**Mission framing.** Madhyamas is a debugging and inspection proxy. Every
accepted capability below is justified by the same test: *does it help a
developer observe, understand, or deliberately perturb traffic?* Capabilities
whose value is production availability or scale fail that test and are
rejected. The recommended positioning is **"gateway behaviors you can
debug"** — not a gateway. Madhyamas's differentiator is that every gateway
action it performs is visible in the traffic timeline, inspectable,
breakpoint-able, and replayable, which full gateways do not optimize for.

## Existing capability inventory

| Capability | Where it lives today |
|---|---|
| URL/header/body/query rewriting with regex, templates ("No Caching", "Add CORS", "Add Auth Header", ...) | `crates/madhyamas-core/src/intercept/rewrite.rs` (`RewriteAction::{UrlRewrite, SetHeader, RemoveHeader, HeaderRewrite, BodyRewrite, QueryParam, RemoveQueryParam, MapToFile}`, `RewriteTemplates`) |
| Mocks: single/sequence, conditional (`RequestCondition::{HeaderEquals, HeaderMatches, QueryParamEquals, JsonPathEquals, JsonPathMatches, BodyMatches, And, Or}`), probabilistic, collections, hit stats | `crates/madhyamas-core/src/intercept/mock.rs` |
| Bandwidth/latency/jitter/packet-loss profiles | `crates/madhyamas-core/src/intercept/throttle.rs` (`ThrottleProfile`) |
| Domain/pattern blocking | `crates/madhyamas-core/src/intercept/block_list.rs` |
| Manual request/response breakpoints | `crates/madhyamas-core/src/intercept/breakpoint.rs` |
| Upstream forwarding, CONNECT pipeline, SOCKS, upstream proxy chaining | `crates/madhyamas-core/src/proxy/{engine.rs, pipeline.rs, socks.rs, upstream_proxy.rs}` |
| Scripting hooks (request/response/WebSocket/gRPC/session) | `crates/madhyamas-core/src/scripting/hooks.rs` (`ScriptHook`) |
| WASM plugin host with signed, hot-reloadable plugins; SDK with `on_request`/`on_response` and `Outcome::respond(...)` | `crates/madhyamas-core/src/plugin/` (esp. `wasm_runtime.rs`, `signing.rs`), `crates/madhyamas-plugin-sdk/src/lib.rs` |
| Example plugins: CORS helper, domain blocker, request logger | `plugins/cors-helper`, `plugins/domain-blocker`, `plugins/request-logger` |
| Enterprise JWT/API-key auth, scopes, middleware, RBAC, Redis shared state | `crates/madhyamas-enterprise/src/{auth.rs, middleware.rs, rbac.rs, redis_state.rs}` |
| Record/replay with request modification | `crates/madhyamas-core/src/replay.rs` (`ReplayManager`, `RequestModifications`) |
| Traffic store/events | `crates/madhyamas-core/src/traffic/` |
| Rule UI: `MocksPanel`, `MockEditDialog`, `RewritesPanel`, `ThrottlePanel`, `BlockListPanel`, `BreakpointsPanel`, `ReplayPanel`, `ScriptsPanel`, `PluginsPanel` | `web/src/features/tools/` |

Important distinction: today's enterprise auth (`auth.rs`, `middleware.rs`)
protects **the Madhyamas control API itself** (who may drive the proxy). It
does **not** enforce authentication on **proxied traffic** (what clients must
present to reach upstreams). The candidate features below concern the latter.

## Per-feature verdicts

| Feature | Verdict | Placement | Rationale |
|---|---|---|---|
| Path/host-based routing to upstreams | Core candidate | OSS core (`intercept/`) | `RewriteAction::UrlRewrite` already rewrites URLs by regex; what is missing is a first-class "route to upstream X" action with match conditions (host + path) and visibility in the timeline. Routing a dev frontend at a local/staging/production backend is a debugging workflow (environment switching), squarely on-mission. Requires proxy pipeline support for a full upstream override (scheme/host/port), not just URL string rewriting. |
| Rate limiting | Plugin candidate first | OSS plugin; distributed (Redis) in enterprise | Local per-process rate limiting is a debugging tool ("how does my client behave under 429s?"). Ship it as a plugin using `on_request` + `Outcome::respond(429, ...)`. Distributed limiting across instances is a multi-instance availability feature — that belongs in the enterprise tier next to `redis_state.rs` if demand materializes. `throttle.rs` shapes bandwidth, not request rate; overlap is conceptual, not structural. |
| Auth enforcement on proxied traffic | Plugin candidate | OSS plugin; enterprise for JWT against upstream IdPs | Enterprise already validates JWT/API keys, but for the control plane. For proxied traffic, a plugin that injects/validates auth headers (e.g. bearer tokens for dev APIs) is the 90% case and matches the existing `add_auth_header` rewrite template. Validating tokens issued by an external IdP, with JWKS caching and key rotation, is enterprise-grade complexity — place in `madhyamas-enterprise` if requested. |
| Request/response transformation | Already partially exists | OSS core (extend + document) | `rewrite.rs` (headers/body/query), scripting hooks, and WASM plugins cover this. Recommendation: document the patterns in a "gateway recipes" docs page rather than build new machinery. Gaps worth small follow-ups: request body templating parity with mock templates. |
| Upstream load balancing | Out of scope | — | Load balancing is a production availability feature: it optimizes for throughput and uptime, not observability. It drags in health probing, pool management, and balancing-policy bikeshedding — all off-mission and duplicative of every real gateway. Environment routing (above) covers the legitimate debugging need ("point at backend A or B"). |
| Response caching | Plugin candidate | OSS plugin ("record & serve") | Adjacent to existing capabilities: mocks serve canned responses, `replay.rs` re-executes captured requests, and `mirror.rs` copies traffic. A plugin that replays a stored response keyed by request shape ("serve from recording") is a natural extension with clear debugging value (deterministic dev environments). Not core: caching semantics (freshness, invalidation, Vary) are a rabbit hole. |
| CORS handling | Already partially exists | Document, don't rebuild | `RewriteTemplates::add_cors()` plus `plugins/cors-helper` cover it. Remaining work is documentation (when to use the template vs the plugin), not implementation. |
| Health checks / upstream availability | Out of scope (core); plugin acceptable | — | Monitoring upstream liveness is an ops concern; a debugging proxy should let you *simulate* failure (block list, throttling, mocks already do), not *manage* it. If a user wants probing, the plugin SDK is sufficient; no core follow-up. |
| Circuit breaking | Out of scope | — | A runtime resilience mechanism whose whole point is autonomous, non-interactive behavior — the opposite of a tool whose value is showing you everything. Rejected on mission grounds. Simulating breaker behavior (flaky responses via probabilistic mocks) is already possible. |

## Recommendations on the open questions

1. **Gateway mode vs independent rules.** Recommend: **independent rule
   features, no "gateway mode."** A mode implies a parallel operating model
   (upstream pools, route tables, lifecycle) that doubles the surface area
   for little gain. Each accepted behavior lands as another rule type in the
   existing interception pipeline (`intercept/handler.rs`), which keeps every
   gateway action visible, ordered, and toggleable like every other rule —
   the debugging-native property that is Madhyamas's differentiator.
2. **Distributed rate limiting / auth: OSS vs enterprise.** Single-user,
   local-process behavior is OSS (usually as a plugin). Anything requiring
   shared state across instances belongs in `madhyamas-enterprise`
   (`redis_state.rs`) — multi-instance deployments are already an enterprise
   feature, and distributed enforcement is only meaningful there.
3. **Positioning vs full gateways.** Do not position against Kong/Envoy/
   Traefik. Frame everything as **"gateway behaviors you can debug"**: route,
   rate-limit, and inject auth *while seeing every decision in the timeline*.
   If a user's need is production traffic management, the honest answer is a
   real gateway in front and Madhyamas as the observation point.
4. **What `web/` rule UI needs for routing-style features.** Today's rule
   editors (`MockEditDialog.tsx`, `RewritesPanel.tsx`) express
   match-then-modify. Routing needs: (a) a match builder for host + path
   prefix/regex (composable like `RequestCondition`'s `And`/`Or`);
   (b) an upstream selector (scheme/host/port, with a test-connection
   affordance); (c) a route-rule list panel analogous to `RewritesPanel`;
   (d) per-entry badgeing in the traffic timeline ("routed to :stage") so the
   routing decision is inspectable. No architectural change to `web/` is
   required — it is additive UI in `web/src/features/tools/`.

## Explicitly rejected

| Feature | Why rejected |
|---|---|
| Upstream load balancing | Production availability, not observability; policy sprawl; environment routing covers the debugging case. |
| Circuit breaking | Autonomous resilience is antithetical to a see-everything debugging tool; failure *simulation* already exists (mocks, block list, throttle). |
| Health checks in core | Ops monitoring, off-mission; plugin SDK suffices for the rare case. |
| API gateway "mode" | Parallel operating model with doubled surface; rules-in-pipeline keeps behaviors debuggable. |
| Production-grade response caching in core | Freshness/invalidation semantics are a maintenance sink; record-&-serve plugin covers the debugging use. |

## Recommended follow-up issues (not yet created — maintainers decide)

1. "Route rule: path/host-based upstream routing in the intercept pipeline" — new rule type + proxy upstream override + timeline badgeing.
2. "Rate-limiter plugin (OSS)" — `on_request` plugin returning 429 with configurable rate/burst.
3. "Auth-injection plugin (OSS)" — bearer/API-key header injection from the secrets store (`secrets/`), extending the `add_auth_header` template.
4. "Record-&-serve response plugin (OSS)" — serve stored responses keyed by request shape.
5. "Docs: gateway recipes page" — end-user recipes for routing, rate limiting, CORS, and auth injection using existing rewrite templates and plugins.
6. (Enterprise, conditional) "Distributed rate limiting via Redis" — only if multi-instance users ask.

## See also

- `docs/ARCHITECTURE.md` for the interception pipeline
- `docs/PLUGINS.md` and `docs/PLUGIN_DEVELOPMENT.md` for the plugin surface
- `crates/madhyamas-core/src/intercept/mod.rs` for the rule engine entry point
