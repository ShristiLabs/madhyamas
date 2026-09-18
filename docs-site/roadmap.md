---
title: Roadmap
description: The public Madhyamas roadmap — per-user traffic scoping in Enterprise, per-app traffic scoping, and certificate overrides for pinned apps, with the workarounds you can use today while they are in progress.
---

# Roadmap

This page is a short, honest view of what the Madhyamas maintainers plan to improve next. Each item describes a limitation you may run into today and the direction planned to address it. There are no dates or version commitments here — scope and priority may change as the work progresses.

| Item | Status |
|------|--------|
| [Per-user traffic scoping (Enterprise)](#per-user-traffic-scoping-enterprise) | Planned |
| [Per-app traffic scoping](#per-app-traffic-scoping) | Under exploration |
| [Certificate overrides for pinned apps](#certificate-overrides-for-pinned-apps) | Under exploration |

If one of these items affects you, feedback is welcome — open an issue on [GitHub](https://github.com/ShristiLabs/madhyamas/issues).

## Per-user traffic scoping (Enterprise)

**Status: Planned**

### The problem today

When several people share one [Madhyamas Enterprise](./enterprise/) instance, they all see the same captured traffic. User roles — Admin, User, Viewer — control which *actions* a user may take, such as managing users or editing mocks, but not which *traffic* is visible. There is no per-user or per-API-key traffic isolation: anything captured on the instance can be browsed by anyone who can sign in.

### What's planned

Each user sees only their own traffic. Sessions are the natural building block — captured entries already belong to sessions — so the plan is to make sessions per-user rather than shared across the entire instance.

### What you can do today

- Run one Madhyamas instance per user. Single-instance deployments that use the built-in SQLite storage are isolated from each other by design — see [Multi-Instance Deployment](./enterprise/deployment) for deployment topologies.
- Use the browser-side filters, with the understanding that they are per-viewer and cosmetic: they change what *you* see, not what is stored or what anyone else sees. See [Focus](./focus) and [Traffic Inspection](./traffic-inspection).

## Per-app traffic scoping

**Status: Under exploration**

### The problem today

Madhyamas sees traffic per *device*, not per *app*. Once a device points at the proxy, every application on it flows through the same capture. The proxy has no notion of which app made a request: captured entries do not record the client address, the listener a request arrived on, or the server name requested during TLS (SNI). "Show only this app's requests" can only be approximated with host and URL heuristics.

### What's planned

Scoping to a single app at capture time:

- A whitelist mode for recording — the inverse of today's [ignored-domains](./recording-limits) exclusion lists — where you list exactly the hosts you want captured and everything else is skipped.
- Richer per-entry metadata: client address, listener, and SNI, together with filters that match on them, so traffic from different apps can be told apart reliably after capture.

### What you can do today

- Focus on a specific app's hosts and enable "Show only focused" — see [Focus](./focus).
- Narrow the traffic list with host filters and search — see [Traffic Inspection](./traffic-inspection).
- Keep noisy hosts out of the capture with ignored-domains recording limits — see [Recording Limits](./recording-limits).
- Point the app you are debugging at the HTTP proxy and send the rest of the device's traffic through the [SOCKS5 listener](./socks-proxy), which tunnels without inspection — only the target app's traffic is then fully captured. See [Mobile Setup](./mobile-setup) for device configuration.

## Certificate overrides for pinned apps

**Status: Under exploration**

### The problem today

Apps with certificate pinning reject every certificate Madhyamas generates. The leaf certificate Madhyamas presents is signed by the Madhyamas CA — not the identity the app pinned — so the TLS handshake fails and you see a 502 entry. The existing workarounds all fight the app itself: patching the APK, hooking it at runtime with Frida, or disabling pinning system-wide on a rooted device. See [HTTPS & Certificates](./https-certificates#certificate-pinning).

One case is not covered by any of those approaches: when you hold the private key of the pinned identity yourself — a staging or test backend whose certificate you control, or an enterprise app that pins your organisation's internal CA. The cleanest possible interception exists in principle here (present the app exactly the certificate it pinned), but Madhyamas cannot do it, because it always mints its own leaf certificates.

### What's planned

A per-host certificate override. For selected hostnames (or wildcard patterns) you supply the original certificate chain *and its private key*; Madhyamas then presents that chain verbatim to the app instead of a generated certificate:

- Pin checks pass, because the app sees the exact identity it pinned — whether it pins the leaf's public key (SPKI), the full leaf certificate, or a CA in the chain.
- Normal chain validation passes, because it is the genuine chain the real server would present.
- No root access, no modified APK, no runtime hooks.

This mirrors what mitmproxy (`--certs`) and Charles Proxy already offer for this situation.

One honest boundary: the override works only if you possess the private key of the pinned identity. Certificates themselves are public — anyone can download the chain a server presents — so without the key the TLS handshake fails before the pin check is even evaluated. If you don't hold the key, interception is cryptographically impossible; that is pinning working as designed, and the app-side bypasses above remain the only options.

### What you can do today

- If the app pins an internal CA your organisation controls, you can already make Madhyamas mint interception certificates from that CA using `--ca-cert-file` / `--ca-key-file` — see [Enterprise Configuration](./enterprise/configuration). Pins on the CA's key then match the generated chain.
- If you hold the key of a pinned *leaf* certificate, there is no proxy-side option yet; use the app-side bypass approaches in [HTTPS & Certificates](./https-certificates#certificate-pinning) in the meantime.

## See also

- [Sessions](./sessions) — grouping captured traffic, the foundation per-user scoping will build on
- [Enterprise Overview](./enterprise/) — authentication, roles, and audit logging
- [HTTPS & Certificates](./https-certificates) — how TLS interception works and today's certificate-pinning bypass options
