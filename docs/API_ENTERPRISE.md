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
`mad_{uuid}` and are sent via the `X-API-Key` header (configurable).

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

`Login`, `Logout`, `ApiKeyCreated`, `ApiKeyRevoked`, `TrafficExported`,
`SessionCreated`, `SessionDeleted`, `MockCreated`, `MockDeleted`,
`BreakpointCreated`, `BreakpointDeleted`, `ConfigChanged`, `Custom`.

Each event records: `user_id`, `api_key_id`, `client_ip`, `timestamp`,
`description`, and arbitrary `metadata`. The log is capped at 10,000 events
with FIFO eviction.

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

## Configuration Import/Export

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/config/export` | Export all configuration to JSON |
| POST | `/config/import` | Import configuration from JSON |

## See Also

- [API.md](API.md) — API index
- [ENTERPRISE.md](ENTERPRISE.md) — Enterprise feature internals (auth, RBAC, audit)
- [PERFORMANCE.md](PERFORMANCE.md) — Performance monitoring internals
