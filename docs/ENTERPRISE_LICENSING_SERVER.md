# Enterprise Licensing Server

> **Moved:** the licensing server no longer lives in this repository. It now
> lives in the private [ShristiLabs/licensing](https://github.com/ShristiLabs/licensing)
> repo as a **multi-product licensing platform** for all ShristiLabs products
> (Madhyamas and future products). This file is a pointer for developers
> arriving from old links; the historical design doc lives in this repo's
> git history.

## What moved

Everything under the former `licensing-server/` workspace member: the
Axum/PostgreSQL backend, the React customer portal, Kubernetes/Docker
deploy artifacts, and its docs (`DEPLOYMENT.md`, `KEY_MANAGEMENT.md`,
`BACKUP.md`).

## What stayed in madhyamas

The **product side** of licensing — offline Ed25519 verification at startup
(`--license-file`, `MADHYAMAS_LICENSE_PUBLIC_KEY`) — implemented in
`crates/madhyamas-enterprise/src/license.rs`.

## How the two sides stay compatible

Both sides use the shared `licensing-core` crate (in the licensing repo,
consumed here as a pinned git dependency):

- `LicenseClaims` — the signed payload (claims format **v2** adds the
  required `product_id`, e.g. `"madhyamas"`; per-product Ed25519 keypairs)
- Canonical JSON — the byte-exact serialization that gets signed
- `LicenseSigner` / `LicenseVerifier` — sign on the server, verify in the
  product

License files issued before `product_id` (v1) fail with a clean parse error
and must be re-issued.

## Where to go

- Licensing server source, deployment, portal: https://github.com/ShristiLabs/licensing
- Key generation/rotation (per product): that repo's `KEY_MANAGEMENT.md`
- Product-side verification flow: [ENTERPRISE_STARTUP_FLOW.md](ENTERPRISE_STARTUP_FLOW.md)
- Crate-level guide (including the `license` module): [ENTERPRISE_CRATE_GUIDE.md](ENTERPRISE_CRATE_GUIDE.md)
- End-user license docs: [docs-site/enterprise/licensing.md](../docs-site/enterprise/licensing.md)
