---
title: User Management
description: Create, edit, delete, and manage user accounts in Madhyamas Enterprise. Assign roles, reset passwords, and manage user status.
---

# User Management

Madhyamas Enterprise includes a full user management system with PostgreSQL-backed persistence, Argon2id password hashing, and role assignment. Users can be managed via the web UI, CLI, or REST API.

## User Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique identifier (auto-generated) |
| `username` | String | Login username (unique) |
| `email` | String | Email address |
| `display_name` | String | Display name (optional) |
| `role` | Enum | `admin`, `user`, `viewer`, `readonly` |
| `status` | Enum | `active`, `inactive`, `suspended`, `pending_verification` |
| `created_at` | Timestamp | Account creation time |
| `last_login` | Timestamp | Last successful login |
| `preferences` | JSON | User preferences (UI settings, etc.) |

## Web UI

The Users admin panel provides a full management interface:

![Enterprise user management panel](/screenshots/enterprise-users-panel.png)

### Accessing the Panel

1. Log in as an admin
2. Click the **Users** icon in the navigation rail (left sidebar)

### Creating a User

1. Click **Add User**
2. Fill in the form: username, email, password, role
3. Click **Create**

The new user can immediately log in with their credentials.

### Editing a User

1. Click the edit (pencil) icon next to a user
2. Modify email, display name, role, or status
3. Click **Save**

### Deleting a User

1. Click the delete (trash) icon next to a user
2. Confirm the deletion

::: warning Irreversible
Deleting a user is permanent. Consider suspending the user instead if you may need to reactivate them later.
:::

### Resetting a Password

1. Click the edit icon next to a user
2. Enter a new password in the password field
3. Click **Save**

Passwords are hashed with Argon2id before storage.

### Changing User Status

| Status | Effect |
|--------|--------|
| **Active** | User can log in and use the system normally |
| **Inactive** | User cannot log in (deactivated) |
| **Suspended** | User cannot log in (administrative action) |
| **PendingVerification** | User cannot log in until email is verified |

## CLI

```bash
# List all users
madhyamas users list
madhyamas users list --json  # JSON output for scripting

# Create a user
madhyamas users create \
  --username alice \
  --email alice@example.com \
  --password secure-password \
  --role user

# Delete a user
madhyamas users delete --id <user-id>

# Update a user's role
madhyamas users update-role --id <user-id> --role admin
```

## REST API

### List Users

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/users
```

### Create a User

```bash
curl -X POST http://localhost:3001/api/users \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "secure-password",
    "role": "user"
  }'
```

### Get a User

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/users/<user-id>
```

### Update a User

```bash
curl -X PUT http://localhost:3001/api/users/<user-id> \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "alice-new@example.com",
    "role": "admin",
    "status": "active"
  }'
```

### Delete a User

```bash
curl -X DELETE http://localhost:3001/api/users/<user-id> \
  -H "Authorization: Bearer <token>"
```

## Bootstrap Admin User

On first startup, Madhyamas creates a default admin user:

```bash
madhyamas \
  --admin-username admin \
  --admin-password your-secure-password \
  --enable-auth \
  --jwt-secret your-secret
```

If the admin user already exists (from a prior run or another instance), the bootstrap is a no-op — the `ON CONFLICT (username) DO NOTHING` clause prevents duplicate user errors.

::: tip Auto-generated password
If `--admin-password` is not set, a random password is generated and logged:
```
Bootstrap: created admin user 'admin'. Auto-generated password (CHANGE IMMEDIATELY): <password>
```
:::

## Password Security

- Passwords are hashed with **Argon2id** (memory-hard, resistant to GPU/ASIC attacks)
- Passwords are never logged or returned in API responses
- The `last_login` timestamp is updated on each successful login

## Best Practices

| Practice | Recommendation |
|----------|----------------|
| **Least privilege** | Assign the minimum role needed (`viewer` > `user` > `admin`) |
| **Deactivate, don't delete** | Use `inactive` status for departed users to preserve audit history |
| **Unique emails** | Each user should have a unique email for SSO and notifications |
| **Strong passwords** | Enforce strong passwords (12+ chars, mixed case, numbers, symbols) |
| **Regular review** | Periodically review the user list and remove inactive accounts |
| **Service accounts** | Use API keys instead of user accounts for automation |

## See Also

- [Authentication](./authentication) — Login flow, JWT, API keys
- [RBAC](./rbac) — Roles and permissions
- [Audit Logging](./audit-logging) — Tracking user actions
- [CLI & MCP Tools](./cli-mcp) — User management via CLI and MCP
