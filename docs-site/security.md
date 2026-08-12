---
title: Security Overview
description: How Madhyamas keeps your debugging traffic safe — sandboxed scripting and plugins, CA key protection, IP allowlists, optional auth/RBAC, plugin signing, and how to report vulnerabilities.
---

# Security Overview

Madhyamas is a man-in-the-middle debugging proxy, so it handles sensitive data: every HTTP request and response that flows through it. This page summarizes the security model and the controls available to keep your traffic and your machine safe.

## Threat Model

Madhyamas is designed to be run by a developer or QA engineer on their own machine (or a trusted server) to inspect traffic they control. It is **not** designed to be exposed to the public internet without additional controls. The biggest risks are:

1. **Captured traffic exposure** — the web UI and API expose request/response bodies, which may contain secrets.
2. **CA private key compromise** — the Madhyamas CA key can sign certificates for any domain; if stolen, an attacker can impersonate sites to any client that trusts it.
3. **Malicious scripts/plugins** — scripts and plugins can modify traffic; a malicious one could exfiltrate data.

The controls below mitigate each of these.

## Network Exposure

By default, Madhyamas binds to `127.0.0.1` (loopback only), so only local processes can connect. To accept traffic from other devices (phones, other machines), you bind to `0.0.0.0` — at which point you should also enable [Access Control](./access-control) to restrict which client IPs can connect.

```bash
# Safe default: loopback only
madhyamas serve

# Exposed: combine with an IP allowlist
madhyamas serve --host 0.0.0.0 --allowed-ip 192.168.1.0/24
```

## Access Control (IP Allowlist)

The [Access Control](./access-control) feature restricts which client IPs can connect to the proxy. It uses CIDR notation, loopback is always allowed, and the allowlist updates live without a restart via `PATCH /api/config` or the `--allowed-ip` flag.

## Authentication and RBAC (Enterprise)

For team or shared deployments, enable the [enterprise](./enterprise) layer to require authentication:

- **JWT (HMAC-SHA256)** for interactive users.
- **API keys** (`mad_<uuid>`) for automation and CI.
- **RBAC** with Admin / User / Viewer / ReadOnly roles over Traffic, Session, Mock, Rewrite, Breakpoint, Script, Plugin, and Config resources.
- **Audit logging** of security-relevant events (login, key creation, traffic export, config changes).

## CA Certificate and Key

- The CA certificate and private key are generated on first run and stored in `~/.madhyamas/certs/`:
  - `madhyamas-ca.pem` — the public CA certificate (install this in client trust stores)
  - `madhyamas-ca-key.pem` — the private key (protect this)
- **Protect the private key.** Anyone with the key can sign certificates for any domain. Set file permissions to `600` and never commit it to version control.
- The CA cert is downloaded via `GET /api/cert/ca` or the **Setup** button in the web UI.
- When running in Docker or Kubernetes, store the certs on a persistent volume so clients only need to trust the CA once. See [Getting Started](./getting-started#docker).

## Sandboxed Scripting

[Scripts](./scripting) run in a `boa_engine` JavaScript runtime that is sandboxed by construction:

- **No filesystem, network, or process access.**
- A fresh execution context is created for each invocation — no shared state between scripts.
- Execution time is soft-limited (default 5 seconds).
- Scripts are trusted code, created by the proxy operator. Don't run scripts from untrusted sources without reviewing them.

## Sandboxed Plugins

[Plugins](./plugins) run in a `wasmtime` WebAssembly runtime with strict limits:

- **No filesystem, network, or host memory access** unless explicitly granted.
- **CPU** is bounded by a fuel budget (default 10 million instructions per invocation).
- **Memory** is capped (linear memory limited to 256 MiB).
- **Packages are verified** with SHA-256 checksums on install.
- **Optional Ed25519 signing** — publishers can sign packages and the installer verifies the signature when a `publisher_public_key` is declared in the manifest.

## TLS and HTTPS Interception

- Madhyamas generates a per-host leaf certificate signed by its CA on the fly.
- HTTPS interception can be disabled with `--no-https` or `madhyamas config update --intercept-https false` — in that mode HTTPS traffic passes through as an opaque tunnel.
- Failed TLS handshakes (e.g. from [certificate pinning](./https-certificates)) are recorded as `502` entries with explanatory messages, so you can diagnose them without exposing the unencrypted traffic.

## Data Storage

- Captured traffic is stored in a SQLite database at `~/.madhyamas/traffic.db`.
- The database contains full request and response bodies, including any secrets they hold. Protect the file with appropriate OS-level permissions.
- Use [Recording Limits](./recording-limits) to bound how much traffic is retained, and [Auto Save](./auto-save) for periodic backups to a directory you control.
- Use [Passthrough mode](./configuration#capture-modes) when you want the proxy running but don't need to record traffic.

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public GitHub issue.
2. Send an email to the maintainers or use [GitHub Security Advisories](https://github.com/ShristiLabs/madhyamas/security/advisories/new).
3. Include steps to reproduce, affected versions, and potential impact.

You can expect acknowledgment within 48 hours and an initial assessment within 5 business days. See [SECURITY.md](https://github.com/ShristiLabs/madhyamas/blob/main/docs/SECURITY.md) in the repo for the full policy.

## See also

- [Access Control](./access-control) — IP allowlist
- [Enterprise](./enterprise) — auth, RBAC, audit logging
- [HTTPS & Certificates](./https-certificates) — CA installation and pinning
- [Scripting](./scripting) — sandboxed JavaScript
- [Plugins](./plugins) — sandboxed WebAssembly
- [Configuration](./configuration) — `--host`, `--allowed-ip`, and other security-relevant flags
