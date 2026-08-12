---
title: Enterprise Features
description: Authentication (JWT and API keys), role-based access control, audit logging, user management, and performance monitoring in Madhyamas — conditionally enabled for team and production deployments.
---

# Enterprise Features

Madhyamas includes an optional enterprise layer that adds authentication, role-based access control (RBAC), audit logging, user management, and performance monitoring. These features are **feature-gated** behind the `enterprise` Cargo feature and conditionally enabled at startup. When enabled, enterprise endpoints are protected by an auth middleware.

This page is an overview for operators. For the full endpoint reference, see [REST API reference](./rest-api#enterprise-endpoints-feature-gated).

## When to Enable Enterprise Features

- **Team deployments** where multiple users share one proxy instance and you need per-user accountability.
- **Production/shared debugging** where the proxy is exposed beyond loopback and you need access control.
- **Compliance** environments that require audit trails of who exported traffic or changed config.
- **CI/CD integration** where long-lived API keys are preferable to interactive login.

For single-developer, loopback-only usage, enterprise features are unnecessary — the proxy works without auth by default.

## Authentication

Madhyamas supports two authentication mechanisms.

### JWT (HMAC-SHA256)

- **Login**: `POST /api/auth/login` validates credentials and returns a JWT.
- **Claims**: `sub` (user ID), `iss` (`madhyamas`), `aud` (`madhyamas-api`), `exp`, `iat`, `role`, `sid` (session ID).
- **Transport**: clients send `Authorization: Bearer <token>`.
- **Logout**: `POST /api/auth/logout` invalidates the session.

### API Keys

- **Format**: `mad_<uuid>`.
- **Transport**: clients send the key via the `X-API-Key` header (header name is configurable).
- **Lifecycle**: create via `POST /api/auth/api-keys`, revoke via `DELETE /api/auth/api-keys/{id}`. Each key tracks `last_used` and `expires_at`.

API keys are well suited to CI pipelines and automation — see [CLI reference](./cli) for using `--api-url` with a key-authenticated proxy.

### Auth Middleware

When an auth service is configured, `auth_middleware` runs on enterprise routes:

1. Extracts the `Authorization: Bearer <token>` header.
2. Validates the JWT.
3. Injects claims into request extensions for downstream handlers.
4. Returns `401` on missing or invalid tokens.

Public paths that bypass auth: `/health`, `/api/health`, `/api/health/detailed`, `/api/auth/login`. Non-`/api` paths (static web assets) also bypass auth.

### Auth Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `enabled` | — | Enable authentication |
| `jwt_secret` | — | HMAC-SHA256 secret |
| `jwt_expiration_secs` | `3600` | Token lifetime |
| `api_key_header` | `X-API-Key` | API key header name |
| `require_auth` | — | Require auth for all requests |
| `refresh_interval_secs` | `300` | Token refresh interval |

## Role-Based Access Control

Roles bundle a set of permissions over resources. Permissions can be granted or revoked at runtime.

| Role | Resources | Permissions |
|------|-----------|-------------|
| **Admin** | Traffic, Session, Mock, Rewrite, Breakpoint, Script, Plugin, Config | Read, Write, Delete (+ Execute for Script/Plugin) |
| **User** | Traffic, Session, Mock, Rewrite, Breakpoint, Script | Read, Write (+ Execute for Script) |
| **Viewer** | Traffic, Session, Mock, Rewrite, Breakpoint, Script, Plugin | Read |
| **ReadOnly** | Traffic, Session, Mock, Rewrite, Breakpoint, Script, Plugin | Read |

**Resources**: `Traffic`, `Session`, `Mock`, `Rewrite`, `Breakpoint`, `Script`, `Plugin`, `Config`.
**Permissions**: `Read`, `Write`, `Delete`, `Execute`.

The `require_permission_middleware` checks the authenticated user's role against the required `(resource, permission)` pair and returns `403` on insufficient permissions.

### RBAC Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/rbac/roles` | List all roles |
| GET | `/rbac/permissions` | List all permissions |
| POST | `/rbac/check` | Check if a user has a permission |

## Audit Logging

The audit logger records security-relevant events in an in-memory ring buffer (capped at 10,000 events, FIFO eviction).

### Event Types

`Login`, `Logout`, `ApiKeyCreated`, `ApiKeyRevoked`, `TrafficExported`, `SessionCreated`, `SessionDeleted`, `MockCreated`, `MockDeleted`, `BreakpointCreated`, `BreakpointDeleted`, `ConfigChanged`, `Custom`.

### Event Fields

| Field | Description |
|-------|-------------|
| `id` | UUID |
| `event_type` | One of the types above |
| `timestamp` | `DateTime<Utc>` |
| `user_id` | Who performed the action (optional) |
| `api_key_id` | API key used (optional) |
| `client_ip` | Client IP address (optional) |
| `description` | Human-readable description |
| `metadata` | Arbitrary key-value map |

### Audit Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/audit` | Query audit entries (filters: `event_types`, `user_id`, `resource`, `success`, time range) |
| GET | `/audit/stats` | Audit statistics |
| GET | `/audit/export` | Export audit events |
| DELETE | `/audit/clear` | Clear audit events |

## User Management

| Type | Variants / Fields |
|------|-------------------|
| `UserRole` | `Admin`, `User`, `Viewer`, `ReadOnly` (default) |
| `UserStatus` | `Active` (default), `Inactive`, `Suspended`, `PendingVerification` |
| `User` | `id`, `username`, `email`, `display_name`, `role`, `status`, `created_at`, `last_login`, `preferences` |

### User Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List all users (admin) |
| POST | `/users` | Create a user (admin) |
| GET | `/users/{id}` | Get user details |
| PUT | `/users/{id}` | Update a user |
| DELETE | `/users/{id}` | Delete a user |

## Performance and Monitoring

| Method | Path | Description |
|--------|------|-------------|
| GET | `/metrics` | Performance metrics (request counts, latency, RPS) |
| GET | `/health/detailed` | Version, uptime, memory, connection stats |
| GET | `/performance` | Performance stats (metrics, memory, connection pool) |

## Onboarding

| Method | Path | Description |
|--------|------|-------------|
| GET | `/onboarding` | Get onboarding status |
| POST | `/onboarding/complete` | Complete an onboarding step |
| POST | `/onboarding/skip` | Skip onboarding |

## Configuration Import/Export

| Method | Path | Description |
|--------|------|-------------|
| GET | `/config/export` | Export all configuration |
| POST | `/config/import` | Import configuration |

::: warning Feature-gated stubs
Some enterprise endpoints are stubs that return `NOT_IMPLEMENTED`. They are conditionally enabled and may require JWT authentication via middleware. Check the [REST API reference](./rest-api) for the current status.
:::

## See also

- [REST API reference](./rest-api) — full endpoint reference including the enterprise section
- [Access Control](./access-control) — IP allowlist (independent of enterprise auth)
- [Configuration](./configuration) — startup flags and environment variables
- [Troubleshooting](./troubleshooting) — common issues
