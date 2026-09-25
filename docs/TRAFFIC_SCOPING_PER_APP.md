# Per-App Traffic Scoping (Analysis Document)

Status: draft for maintainer review
Roadmap item: "Per-app traffic scoping" — Under exploration
  ([public roadmap](https://shristilabs.github.io/madhyamas/roadmap))
Tracking issue: none yet

## Summary

Madhyamas captures traffic per *device*: once a device points at the proxy,
every application on it flows through the same capture. There is no notion of
which app made a request — captured entries record no client address, no
listener, and no TLS server name (SNI). This document analyses what "per-app"
can mean at a proxy, the options for getting there, and recommends a phased
approach: metadata first, whitelist recording second, and listener/device-side
scoping as the reliable end state.

## The fundamental constraint (read this first)

**A proxy never sees "the app". It sees connections.** At the network layer
the only identity signals available are:

| Signal | Available? | Distinguishes |
|---|---|---|
| Source address (`client_ip:port`) | Yes (dropped today, see inventory) | *Devices* — useless for apps on one device (all apps share the device IP) |
| Destination host / SNI / CONNECT authority | Yes (host already stored) | *Services* — not apps; two apps hitting the same API host are indistinguishable |
| Listener the connection arrived on | Yes (not recorded) | Whatever *routing rule* put the connection there — an app, if the user arranges it |
| TLS fingerprint / User-Agent heuristics | Partially | App *families*, brittle — see Rejected |

Reliable per-app identity therefore comes from exactly two places, both
outside the capture path itself:

1. **The device routes per-app** (Android `VpnService` supports per-app
   routing; the companion app already owns a VPN tunnel).
2. **The user routes per-app** (target app configured for a dedicated
   listener; everything else goes elsewhere).

Everything the proxy can do alone is *inference* (host whitelists) or
*infrastructure for the two reliable mechanisms* (listener/client metadata).
The options below are ranked against that reality.

## Problem statement

- **Noise**: capturing one app's debugging session drags in the whole
  device's traffic (OS telemetry, other apps' CDNs) — the signal-to-noise
  problem the whitelist mode addresses.
- **Attribution**: even with a clean capture, entries cannot be traced back
  to a client device or an arrival path after the fact — the metadata
  problem.
- **Isolation**: the strong form — "capture *only* this app" — cannot be
  guaranteed by the proxy alone today.

## Current-state inventory

| Concern | Where it lives today | Notes |
|---|---|---|
| `ignored_domains` config | `crates/madhyamas-core/src/config.rs:88` | Exclusion list, default empty. |
| Exclusion enforcement | `TrafficStore::is_host_ignored` (`traffic/store.rs:467-494`), called from `store_request` (`store.rs:832-839`) | **Storage-time, not interception-time**: the request is fully intercepted and proxied, only the recording is skipped. Exact + suffix + `*.domain` matching, case-insensitive. |
| Runtime updates | `set_ignored_domains` (`store.rs:436-443`) via `PATCH /api/config` (`madhyamas-api/src/handlers.rs:711-721`) | Live reload already works. |
| SSL passthrough list | `Config::should_passthrough` (`config.rs:1236-1242`) | Exact/suffix only (no wildcards); tunnels without interception — a different axis from recording. |
| Block list (different feature) | `intercept/block_list.rs:49`, pipeline priority 0 (`proxy/pipeline.rs:199-201`) | Denies requests with a response; does **not** suppress recording. |
| `RequestData` / `TrafficEntry` | `traffic/types.rs:68-93` / `:210-242` | No `client_addr`, `listener`, or `sni` fields. |
| Client address lifecycle | Accept loop has it (`engine.rs:450`, used for IP ACL at `:470`), then **drops it**: spawned task calls `handle_connection(client_socket)` (`engine.rs:500`, signature `:511`) | Single choke point to thread the address through. Same pattern in SOCKS (`socks.rs:392` available, dropped; listener spawned `engine.rs:431-447`). |
| SNI reality | CONNECT authority parsed at `engine.rs:627-633`; client cert minted from it (`:655`) | For MITM'd TLS the CONNECT host is the de-facto hostname claim; true ClientHello SNI is not separately extracted (the rustls `ServerName` at `:1401` is the *upstream* side). |
| SOCKS listener | `proxy/socks.rs:627-642` | **Blind tunnel** — one CONNECT entry (`is_passthrough = true`, `http_version = "SOCKS5"`), no inspection ([SOCKS_PROXY.md](SOCKS_PROXY.md)). |
| Entry construction points | TLS-failure `engine.rs:683-695`; passthrough `engine.rs:794-807`; MITM `pipeline.rs:239` and `pipeline.rs:594` | Only `RequestData` + session in scope at these points today. |
| Filtering | `TrafficFilter` (`types.rs:358-388`) → SQL in `get_traffic` (`store.rs:987`); API `TrafficQuery` (`handlers.rs:19-40`); web `useTraffic.ts:12-17` + `TrafficToolbar.tsx` | Server-side; new dimensions plug in at the same five layers. |

## Options considered

### Option A — Whitelist recording mode (inverse of ignored-domains)

A recording mode (`all` | `whitelist`) plus a `recorded_domains` list,
enforced at the same storage-time choke point as `ignored_domains`
(`store.rs:832-839`). "Record only `api.myapp.dev` and `*.cdn.myapp.dev`;
intercept and forward everything else silently."

**Pros**

- Mirror-image of an existing, battle-tested mechanism — matching semantics,
  runtime updates, and both backends come for free.
- Directly kills the noise problem; composes with Focus for viewing.
- No schema change.

**Cons**

- Still host inference: apps sharing infrastructure hosts (shared CDNs,
  analytics SDKs, OS services hitting the same domains) leak in or get
  excluded wrongly. Acceptable for debugging, never "guaranteed only this
  app".
- One more axis (mode + two lists) in recording config; UX must make the
  mode/list interplay obvious.

### Option B — Per-entry metadata: `client_addr`, `listener`, `sni` + filters

Persist the three missing signals on every entry and expose them as filter
dimensions end-to-end:

- `client_addr`: thread from the accept loops (`engine.rs:450→500`,
  `socks.rs:392`) through `handle_connection` into the entry constructors
  (`engine.rs:683/794`, `pipeline.rs:239/594`).
- `listener`: tag which listener accepted the connection (HTTP proxy vs
  SOCKS; future dedicated listeners — see Option D).
- `sni`: start with the CONNECT authority (already the cert-minting host at
  `engine.rs:655`); extracting true ClientHello SNI is a possible follow-up
  but changes nothing for MITM'd traffic where CONNECT host ≈ SNI by
  construction.

**Pros**

- Foundation for *every* other option: post-hoc filtering by device
  (`client_addr`) and by routing path (`listener`) is what makes Options C/D
  usable after capture.
- Immediate standalone value on multi-device setups (phones + desktop
  through one proxy) — "show only the Pixel's traffic".
- Cheap columns, one migration, both backends, additive filters.

**Cons**

- Schema migration on the hot path (`requests` table, SQLite + PostgreSQL)
  and signature churn at four entry-construction sites.
- Alone, it does **not** solve single-device per-app isolation (all apps
  share `client_addr` and the default listener) — it is infrastructure, not
  the end state.

### Option C — Device-side per-app routing (companion VPN app)

The Android companion app (`android/`) owns a `VpnService` tunnel. Android
supports per-app VPN routing (`addAllowedApplication` /
`addDisallowedApplication`): route *only the debugged app* through the
tunnel to Madhyamas, let everything else go direct.

**Pros**

- The **only mechanism with kernel-level per-app identity** — no host
  guessing; "capture only this app" becomes true by construction.
- No proxy-side schema or pipeline changes required to get correctness
  (Option B metadata still improves post-hoc tooling).
- The same tunnel can carry per-device identity (credential injection on
  the forwarding path) — see
  [TRAFFIC_SCOPING_PER_DEVICE.md](TRAFFIC_SCOPING_PER_DEVICE.md).

**Cons**

- Android-only. iOS has no equivalent outside MDM-managed per-app VPN
  profiles.
- Companion-app scope creep: the VPN app today has "one job only" (route
  everything — see
  [CERT_PINNING_PLAIN_ENGLISH.md](CERT_PINNING_PLAIN_ENGLISH.md) §14); per-app
  routing adds app-selection UI, package-name persistence, and edge cases
  (apps spawning other processes).
- Captured-traffic volume drops to just the app — which is the point, but
  changes workflows that today debug "the app plus its ecosystem".

### Option D — Dedicated per-target listeners + listener tag

Run N listeners ("App A → `:8081`", "App B → `:8082`"), point each app at its
own proxy port, tag entries by listener (depends on Option B's `listener`
field). Works cross-platform today with manual device config; can later be
productized ("listener profiles" with per-listener recording rules).

**Pros**

- Reliable identity (the user put the app there), cross-platform, no OS
  support needed.
- Composes with per-listener settings (per-listener whitelist, per-listener
  SSL passthrough).

**Cons**

- Manual setup per app until productized; N ports to firewall/document.
- Requires multi-listener support in the engine if listeners must differ in
  *behavior* (today HTTP and SOCKS listeners are fixed kinds —
  `engine.rs:431-447`).

### Option E — Status quo workaround (documented)

Target app → HTTP proxy port (inspected); everything else on the device →
SOCKS listener (blind tunnel, `socks.rs:627-642`) or direct. This is what
the roadmap's "What you can do today" section describes. It is manual,
device-global, and all-or-nothing per protocol — but it works with zero
changes and should stay documented.

## Tradeoff matrix

| Criterion | A whitelist | B metadata | C per-app VPN | D listeners | E workaround |
|---|---|---|---|---|---|
| Solves noise | yes | partially (post-hoc) | yes | yes | roughly |
| Reliable per-app identity | no (inference) | no (infrastructure) | **yes** | **yes** (manual) | no |
| Schema change | none | 3 columns + migration | none | none (needs B for tags) | none |
| Capture-path change | none | threading through 4 sites + 2 accept loops | none | engine listener support | none |
| Platform | all | all | Android only | all | all |
| Effort | small | medium | medium (Android app) | small-medium | none |
| Multi-device value | low | **high** | n/a | medium | low |

## Recommended approach

**B → A → (C ∨ D), with E documented throughout.**

1. **Metadata first (Option B).** Persist `client_addr`/`listener`/`sni`,
   expose as filters across `TrafficFilter` → SQL → `TrafficQuery` →
   `useTraffic.ts` → toolbar. It is additive, low-risk, immediately useful
   for multi-device debugging, and every later option consumes it.
2. **Whitelist mode second (Option A).** Small, mirrors `ignored_domains`
   exactly (same choke point, same runtime-update path, same matching
   semantics — keep the two lists mutually exclusive by mode to avoid
   config paradoxes). Ship together with B's filter work in one UI pass.
3. **Then choose the reliable-identity mechanism per platform demand:**
   - Android-heavy debugging → Option C (per-app VPN routing in the
     companion app) — the strongest end state where the OS supports it.
   - Cross-platform teams → Option D (listener profiles + `listener` tags).
   These are not mutually exclusive; C and D both get better with B's
     metadata in place.

**Why not A first.** A is the smallest change and the most requested, but it
bakes nothing in — B is the prerequisite for C/D being more than capture-time
guessing, and B's migration is the only risky step, so it should land early
and alone.

## Explicitly rejected

| Idea | Why rejected |
|---|---|
| TLS-fingerprint / JA3-based app identification | Brittle (fingerprints drift per library version), can only classify app families, and invites a maintenance treadmill for a signal the OS gives away for free via per-app routing. |
| User-Agent / header heuristics as identity | Trivially spoofed and routinely wrong (WebViews, native HTTP stacks with generic UAs). Fine as a *search aid*, never as scoping. |
| Deep packet inspection beyond host/SNI | Enormous surface, poor return; the proxy's job is observation, and the two reliable mechanisms make guessing unnecessary. |
| Auto-probabilistic "app clustering" | Unexplainable behavior in a debugging tool: silent misattribution is the failure mode this whole feature exists to remove. |

## Open questions (maintainers)

1. True SNI extraction (ClientHello interception) vs CONNECT-host
   equivalence — worth the added handshake complexity for MITM'd traffic?
   (Passthrough tunnels are the one case where CONNECT host is absent.)
2. Should whitelist mode live in `Config` (`config.rs:88`) like
   `ignored_domains`, or in the store's `RwLock` snapshot
   (`store.rs:55`)? The store path is runtime-updatable today; the config
   path is persisted. Recommend: both, mirroring `ignored_domains` exactly.
3. Wildcard support unification: passthrough matching (`config.rs:1236`)
   lacks the wildcard semantics `is_host_ignored` has — align when touching
   either list.
4. Listener profiles (Option D): productized config or documented recipe
   first?

## Follow-up issues (not yet created)

1. "Persist `client_addr`/`listener`/`sni` on traffic entries + filter
   params end-to-end" — types, both schemas, accept-loop threading, SQL,
   API, web UI.
2. "Whitelist recording mode (`recording_mode` + `recorded_domains`)" —
   store check, config, API, UI toggle.
3. "Per-app routing in the Android companion VPN" — device-side Option C.
4. "Docs: per-app scoping recipes" — end-user guide stitching E (today), A+B
   (near-term), C/D (end state); update the roadmap item when stages land.

## See also

- [TRAFFIC_SCOPING_PER_USER.md](TRAFFIC_SCOPING_PER_USER.md) — the
  orthogonal multi-user visibility problem
- [TRAFFIC_SCOPING_PER_DEVICE.md](TRAFFIC_SCOPING_PER_DEVICE.md) — device
  identity and the companion/manual steering modes that Option C builds on
- [RECORDING_LIMITS.md](RECORDING_LIMITS.md) — current ignored-domains
  behaviour (end-user: [Recording Limits](https://shristilabs.github.io/madhyamas/recording-limits))
- [SOCKS_PROXY.md](SOCKS_PROXY.md) — the blind-tunnel listener used by
  today's workaround
- [CERT_PINNING_OVERRIDES.md](CERT_PINNING_OVERRIDES.md) — per-app scoping
  raises pinning exposure: the narrower the capture, the more the target
  app's pinned hosts dominate it
- [PERSISTENCE.md](PERSISTENCE.md) — schema migration mechanics
