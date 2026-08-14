---
title: Role-Based Access Control
description: Roles, permissions, and resource types in Madhyamas Enterprise RBAC. Understand the permission matrix and how enforcement works.
---

# Role-Based Access Control

Madhyamas Enterprise includes a role-based access control (RBAC) system that restricts what authenticated users can do. Each user is assigned a role, and each role grants a set of permissions over resources.

## Roles

Madhyamas defines four built-in roles:

| Role | Description | Typical User |
|------|-------------|-------------|
| **Admin** | Full access to all resources and admin panels | System administrator, DevOps |
| **User** | Read and write traffic data, sessions, mocks, rewrites, breakpoints, scripts | Developer, QA engineer |
| **Viewer** | Read-only access to traffic data and intercept rules | Stakeholder, manager |
| **ReadOnly** | Read-only access (same as Viewer) | Automated reporting, auditors |

## Resources

| Resource | Description |
|----------|-------------|
| `Traffic` | Captured HTTP/HTTPS traffic entries |
| `Session` | Traffic sessions (named groups) |
| `Mock` | Mock response rules |
| `Rewrite` | Rewrite rules |
| `Breakpoint` | Breakpoint rules |
| `Script` | JavaScript scripts |
| `Plugin` | WASM plugins |
| `Config` | System configuration |

## Permissions

| Permission | Description |
|-----------|-------------|
| `Read` | View/list entries |
| `Write` | Create/modify entries |
| `Delete` | Delete entries |
| `Execute` | Execute scripts or plugins |

## Permission Matrix

| Role | Traffic | Session | Mock | Rewrite | Breakpoint | Script | Plugin | Config |
|------|---------|---------|------|---------|------------|--------|--------|--------|
| **Admin** | R/W/D | R/W/D | R/W/D | R/W/D | R/W/D | R/W/D/E | R/W/D/E | R/W/D |
| **User** | R/W | R/W | R/W | R/W | R/W | R/W/E | — | — |
| **Viewer** | R | R | R | R | R | R | R | — |
| **ReadOnly** | R | R | R | R | R | R | R | — |

*R = Read, W = Write, D = Delete, E = Execute*

## How Enforcement Works

1. **Authentication** — The auth middleware validates the JWT or API key and extracts the user's role
2. **Authorization** — The `require_permission_middleware` checks the user's role against the required `(resource, permission)` pair for the endpoint
3. **Response** — If the user lacks permission, the server returns `403 Forbidden`

```rust
// Example: Creating a mock requires (Mock, Write) permission
// A Viewer role user will get 403 Forbidden
```

## RBAC Endpoints

### List Roles

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/rbac/roles
```

### List Permissions

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/rbac/permissions
```

### Check Permission

```bash
curl -X POST http://localhost:3001/api/rbac/check \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "<user-id>",
    "resource": "Mock",
    "permission": "Write"
  }'
```

Response:

```json
{
  "allowed": true
}
```

## Assigning Roles

### Via Web UI

1. Navigate to **Admin → Users**
2. Click the edit (pencil) icon next to a user
3. Select the role from the dropdown
4. Click **Save**

### Via CLI

```bash
madhyamas users update-role --id <user-id> --role admin
```

### Via API

```bash
curl -X PUT http://localhost:3001/api/users/<user-id> \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{"role": "admin"}'
```

## Common Scenarios

### Developer (User Role)

A developer who needs to debug APIs, create mocks, and write scripts:

- **Role**: `User`
- **Can**: View traffic, create/edit sessions, create mocks and rewrites, set breakpoints, write and execute scripts
- **Cannot**: Manage users, view audit logs, manage plugins, change system config

### QA Engineer (User Role)

A QA engineer who tests APIs with mocks and rewrites:

- **Role**: `User`
- **Can**: Same as developer
- **Cannot**: Same as developer

### Stakeholder (Viewer Role)

A manager who needs to review traffic but not modify anything:

- **Role**: `Viewer`
- **Can**: View traffic, sessions, mocks, rewrites, breakpoints, scripts, plugins
- **Cannot**: Create or modify any resource, manage users, view audit logs

### Auditor (ReadOnly Role)

An external auditor who reviews captured traffic for compliance:

- **Role**: `ReadOnly`
- **Can**: View all traffic and intercept rules (read-only)
- **Cannot**: Modify anything, export data, manage users

### CI/CD Pipeline (API Key)

An automated pipeline that exports traffic for analysis:

- **API Key** with scopes: `traffic:read`, `traffic:export`
- **Can**: List and export traffic
- **Cannot**: Modify anything, manage users

## See Also

- [Authentication](./authentication) — JWT and API key authentication
- [User Management](./user-management) — Creating and managing users
- [Audit Logging](./audit-logging) — Tracking permission-gated actions
- [CLI & MCP Tools](./cli-mcp) — RBAC via CLI and MCP
