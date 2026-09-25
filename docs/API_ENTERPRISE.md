# API — Enterprise

Enterprise endpoints are feature-gated behind the `enterprise` Cargo feature
and must be enabled at startup. When an auth service is configured, these
endpoints are JWT-protected via `auth_middleware` (see [ENTERPRISE.md](ENTERPRISE.md)).
Public routes (`/auth/login`, `/health/detailed`) bypass auth. Base path: `/api`.

## Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/auth/login` | Authenticate and receive a JWT (public) |
| POST | `/auth/logout` | Invalidate the current session |
| GET | `/auth/me` | Get the current authenticated user |
| POST | `/auth/validate` | Validate a JWT token and return claims |
| GET | `/auth/api-keys` | List API keys for the current user |
| POST | `/auth/api-keys` | Create a new API key |
| DELETE | `/auth/api-keys/{id}` | Revoke an API key |

JWTs use HMAC-SHA256 with claims: `sub` (user ID), `iss` ("madhyamas"),
`aud` ("madhyamas-api"), `exp`, `iat`, `role`, `sid`. API keys use the format
`madhyamas_{hex}` and are sent via the `X-API-Key` header (configurable);
per-device credentials use the distinct `mdy_dev_{hex}` prefix and are
connect-only (see [Devices](#devices)).

## Users

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/users` | List all users |
| POST | `/users` | Create a user |
| GET | `/users/{id}` | Get a user |
| PUT | `/users/{id}` | Update a user (email, role, status) |
| DELETE | `/users/{id}` | Delete a user |

## RBAC

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rbac/roles` | List all available roles |
| GET | `/rbac/permissions` | List all available permissions |
| POST | `/rbac/check` | Check if a user has a permission on a resource |

### Roles and permissions

| Role | Permissions |
|------|-------------|
| Admin | Full CRUD on all resources + Script/Plugin Execute + Config Read/Write |
| User | Read/Write on Traffic, Session, Mock, Rewrite, Breakpoint; Script Read/Execute |
| Viewer | Read on Traffic, Session, Mock, Rewrite, Breakpoint, Script, Plugin |
| ReadOnly | Read on Traffic, Session, Mock, Rewrite, Breakpoint, Script, Plugin |

Resources: `Traffic`, `Session`, `Mock`, `Rewrite`, `Breakpoint`, `Script`,
`Plugin`, `Config`. Permissions: `Read`, `Write`, `Delete`, `Execute`.

## Audit

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/audit` | Query audit events (filter by type, user, time range) |
| GET | `/audit/stats` | Audit statistics (totals, by type, top users) |
| GET | `/audit/export` | Export audit events matching a query |
| DELETE | `/audit/clear` | Clear all audit events |

### Audit event types

`Login`, `Logout`, `ApiKeyCreated`, `ApiKeyRevoked`, `DeviceRegistered`,
`DeviceKeyRotated`, `DeviceRevoked`, `DeviceEnrollmentIssued`,
`DeviceEnrolled`, `TrafficExported`, `SessionCreated`,
`SessionDeleted`, `MockCreated`, `MockDeleted`, `BreakpointCreated`,
`BreakpointDeleted`, `ConfigChanged`, `Custom`.

Each event records: `user_id`, `api_key_id`, `client_ip`, `timestamp`,
`description`, and arbitrary `metadata`. The log is capped at 10,000 events
with FIFO eviction.

## Devices

Device principals for credential-based traffic attribution
([CREDENTIAL_ONBOARDING.md](CREDENTIAL_ONBOARDING.md)). Each device gets a
per-device credential with the `mdy_dev_` prefix — connect-only: it
authenticates `CONNECT`/proxy requests (via `X-API-Key`,
`Proxy-Authorization: Bearer`, or as either half of
`Proxy-Authorization: Basic` for manual proxy-auth fields on iOS/OEM-Android)
and is rejected on every REST/MCP/CLI endpoint. The plaintext key is
returned exactly once at creation/rotation (show-once); only its SHA-256
hash is stored.

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/devices` | List the current user's devices (with `last_seen`) |
| POST | `/devices` | Register a device `{name, install_uuid?, mac_address?}`; returns `{device, key}` (show-once) |
| POST | `/devices/{id}/rotate` | Revoke the device's current key and mint a new one; returns `{device, key}` (show-once) |
| POST | `/devices/{id}/revoke` | Revoke the device and deactivate its keys (record kept, status `revoked`) |
| DELETE | `/devices/{id}` | Revoke the device's keys and delete the record |
| POST | `/devices/{id}/enrollment-token` | Issue a short-lived single-use enrollment token; returns `{device, token, expires_at}` (show-once) |
| POST | `/devices/enroll` | **Public.** Redeem `{token}` for the long-lived credential; returns `{device, key}` (show-once) |

Lifecycle notes:

- `last_seen` is derived from proxy-auth events — a successful device
  CONNECT stamps it (and flips the panel status from *pending* to
  *live/seen*).
- Revocation takes effect on the next CONNECT: the key lookup fails and the
  engine answers `407 Proxy Authentication Required`.
- Rotate deactivates the old key and issues a new one atomically from the
  caller's perspective; the device identity (and future per-device traffic
  attribution) is unaffected.
- Audit: `DeviceRegistered`, `DeviceKeyRotated`, `DeviceRevoked`,
  `DeviceEnrollmentIssued`, `DeviceEnrolled`.

### QR enrollment (issue #106)

The credential dialog renders a `madhyamas://connect` QR alongside the
manual values. The QR carries an **enrollment token** by default — not the
long-lived key — so a photographed QR expires:

```text
madhyamas://connect?host=proxy.example.com&port=8888&tls=0
  &token=mdy_enroll_...              (or key=mdy_dev_... in manual mode)
  &name=Hari%27s%20Pixel
  &ca=http://proxy.example.com:3001/api/cert/ca
  &api=http://proxy.example.com:3001/api
```

- `tls` is `0` today (a TLS-wrapped proxy listener is a later issue) but
  the field always round-trips.
- `ca`/`api` are derived from the instance's own origin (the web UI is
  served by the API server).

Enrollment tokens:

- Format `mdy_enroll_{hex}` — rejected on REST/MCP/CLI and at the proxy
  listener alike: they are exchange credentials, not usable secrets.
- Single-use and 15-minute TTL, enforced atomically at redemption
  (`UPDATE ... WHERE redeemed_at IS NULL AND expires_at > now`): a token
  older than 15 minutes or already redeemed returns `401`, as do unknown
  and revoked tokens (indistinguishable, to prevent enumeration).
- Redeeming revokes the device's existing active keys (including the
  create-time show-once key) and mints exactly one fresh `mdy_dev_`
  credential — one live credential per device.
- Hashed at rest (SHA-256, same as device keys); the plaintext appears
  only in the QR payload. Neither tokens nor keys are ever logged or
  written to audit metadata (device IDs only).
- Revoking or deleting a device also revokes its outstanding enrollment
  tokens; expired token rows are pruned opportunistically on issuance.

### Proxy auth policy (`require_proxy_auth`)

The enterprise tier always validates proxy credentials when a store is
configured (so device traffic can be attributed). What happens to
unauthenticated connections is a policy:

- `--require-proxy-auth` / `MADHYAMAS_REQUIRE_PROXY_AUTH` (default off):
  unauthenticated CONNECT/HTTP proxy requests pass and are captured to the
  unattributed scope. When enabled, they receive `407`.
- `--proxy-auth` (Phase 9.6) keeps its stricter meaning: unauthenticated
  proxy requests are rejected with `407`.
- Supplied-but-invalid credentials (unknown, expired, revoked device key)
  are always rejected with `407`, regardless of the toggle — revoking a
  device cuts off its proxy access immediately.

## Metrics & Performance

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/metrics` | Current metrics snapshot (requests, latency, throughput, intercept hits) |
| GET | `/performance` | Combined performance stats (metrics + memory + pool) |
| GET | `/health/detailed` | Detailed health check (version, uptime, memory, connections) (public) |

## Onboarding

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/onboarding` | Get onboarding progress |
| POST | `/onboarding/complete` | Mark an onboarding step as completed |
| POST | `/onboarding/skip` | Skip onboarding entirely |

Steps: `welcome`, `certificate`, `proxy`, `device` (optional — connect a
phone/tablet via the Devices panel QR, issue #106), `features`
(optional), `tips` (optional).

## Configuration Import/Export

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/config/export` | Export all configuration to JSON |
| POST | `/config/import` | Import configuration from JSON |

## See Also

- [API.md](API.md) — API index
- [ENTERPRISE.md](ENTERPRISE.md) — Enterprise feature internals (auth, RBAC, audit)
- [PERFORMANCE.md](PERFORMANCE.md) — Performance monitoring internals
