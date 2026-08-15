---
title: Audit Logging
description: Tamper-evident audit logging with SHA-256 hash chain, PostgreSQL persistence, event types, querying, export, and compliance use cases.
---

# Audit Logging

Madhyamas Enterprise records security-relevant events in a tamper-evident audit log. Every authenticated action — logins, config changes, mock creation, traffic exports — is recorded with a cryptographic hash chain that makes it possible to detect after-the-fact tampering.

## How It Works

### Hash Chain Integrity

Each audit event includes a `prev_hash` field containing the SHA-256 hash of the previous event. This creates a chain where modifying or deleting any event breaks the chain and is immediately detectable.

```
Event 1: hash = SHA256(data_1)
Event 2: prev_hash = hash_1, hash = SHA256(data_2 + hash_1)
Event 3: prev_hash = hash_2, hash = SHA256(data_3 + hash_2)
```

Insertion is serialized across instances using a PostgreSQL advisory lock (`pg_advisory_xact_lock`), ensuring the hash chain remains consistent even when multiple instances log events simultaneously.

### PostgreSQL Persistence

Audit events are stored in PostgreSQL in the `audit_events` table, not in memory. This means audit logs survive restarts and are shared across all instances in a multi-instance deployment.

## Event Types

| Event Type | Trigger |
|-----------|---------|
| `Login` | User successfully logs in |
| `Logout` | User logs out |
| `ApiKeyCreated` | A new API key is created |
| `ApiKeyRevoked` | An API key is revoked |
| `TrafficExported` | Traffic is exported (HAR, JSON) |
| `SessionCreated` | A new session is created |
| `SessionDeleted` | A session is deleted |
| `MockCreated` | A mock rule is created |
| `MockDeleted` | A mock rule is deleted |
| `BreakpointCreated` | A breakpoint is created |
| `BreakpointDeleted` | A breakpoint is deleted |
| `ConfigChanged` | System configuration is modified |
| `Custom` | Custom event (via API or script) |

## Event Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique event identifier |
| `event_type` | Enum | One of the types above |
| `timestamp` | DateTime | UTC timestamp |
| `user_id` | UUID | User who performed the action (optional) |
| `api_key_id` | UUID | API key used (optional) |
| `client_ip` | String | Client IP address (optional) |
| `description` | String | Human-readable description |
| `metadata` | JSON | Arbitrary key-value metadata |
| `prev_hash` | String | SHA-256 hash of the previous event |

## Web UI

The Audit admin panel provides a visual interface for browsing and exporting audit events:

![Enterprise audit log panel](/screenshots/enterprise-audit-panel.png)

### Accessing the Panel

1. Log in as an admin
2. Click the **Audit Log** icon in the navigation rail

### Features

- **Statistics bar** — Total events, today's events, error count, event type breakdown
- **Filtering** — Filter by event type, user ID
- **Pagination** — Browse through events with previous/next buttons
- **Export** — Click **Export** to download all audit events as JSON

## CLI

```bash
# List audit events
madhyamas audit list
madhyamas audit list --event-type Login --limit 50
madhyamas audit list --user-id <user-id>

# Export all audit events as JSON
madhyamas audit export > audit-log.json

# View audit statistics
madhyamas audit stats
```

## REST API

### Query Audit Events

```bash
curl -H "Authorization: Bearer <token>" \
  "http://localhost:3001/api/audit?event_type=Login&limit=50"
```

Query parameters:

| Parameter | Type | Description |
|-----------|------|-------------|
| `event_types` | Comma-separated | Filter by event types |
| `user_id` | UUID | Filter by user |
| `resource` | String | Filter by resource type |
| `success` | Boolean | Filter by success/failure |
| `start_time` | ISO 8601 | Filter events after this time |
| `end_time` | ISO 8601 | Filter events before this time |
| `limit` | Integer | Maximum results (default: 100) |
| `offset` | Integer | Pagination offset |

### Audit Statistics

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/audit/stats
```

Response:

```json
{
  "total_events": 1542,
  "events_today": 87,
  "errors": 3,
  "by_type": {
    "Login": 45,
    "ConfigChanged": 12,
    "MockCreated": 30
  }
}
```

### Export Audit Events

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/audit/export > audit.json
```

### Clear Audit Events

```bash
curl -X DELETE http://localhost:3001/api/audit/clear \
  -H "Authorization: Bearer <token>"
```

::: warning Irreversible
Clearing audit events is permanent and breaks the hash chain. This action is itself logged as a `Custom` event.
:::

## Compliance Use Cases

### GDPR

- **Data export**: Use `GET /api/audit/export` to provide a complete audit trail
- **Right to be forgotten**: Audit events reference user IDs but do not contain PII beyond what was in the original request

### SOC 2

- **Access control**: Audit logs record every login and API key usage
- **Change management**: All config changes and mock/rewrite modifications are logged
- **Data integrity**: The hash chain provides tamper evidence

### HIPAA

- **Access logging**: All traffic exports are logged with user ID and timestamp
- **Audit trail**: Persistent PostgreSQL storage ensures logs survive restarts

## MCP Tools

AI agents can query audit events via MCP:

```
madhyamas_get_audit_events(event_type="Login", limit=50)
madhyamas_export_audit()
```

See [CLI & MCP Tools](./cli-mcp) for details.

## See Also

- [Authentication](./authentication) — Login and API key events
- [User Management](./user-management) — User-related audit events
- [RBAC](./rbac) — Permission-gated actions
- [CLI & MCP Tools](./cli-mcp) — Audit commands via CLI and MCP
