---
title: Enterprise CLI & MCP Tools
description: Enterprise CLI commands (users, audit, license, auth, API keys) and enterprise MCP tools for AI agent integration.
---

# Enterprise CLI & MCP Tools

Madhyamas Enterprise adds CLI commands and MCP tools for managing users, audit logs, licenses, API keys, and metrics. All enterprise commands require authentication via JWT or API key.

## CLI Authentication

Before using enterprise CLI commands, authenticate with the server:

### API Key (Recommended for Automation)

```bash
export MADHYAMAS_API_KEY=mad_abc123...
madhyamas users list
```

### JWT Token (Interactive)

```bash
# Login and store token
export MADHYAMAS_TOKEN=$(madhyamas auth login --username admin --password your-password --output json | jq -r .token)
madhyamas users list
```

### CLI Flags

```bash
madhyamas --api-url http://localhost:3001 --api-key mad_abc123... users list
madhyamas --api-url http://localhost:3001 --token eyJ... users list
```

## CLI Commands

### User Management

```bash
# List all users
madhyamas users list
madhyamas users list --json  # JSON output

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

### Audit Log

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

### License

```bash
# View license information
madhyamas license info
```

### Authentication

```bash
# Login
madhyamas auth login --username admin --password your-password

# Logout
madhyamas auth logout
```

### API Keys

```bash
# List API keys
madhyamas auth api-keys list

# Create an API key
madhyamas auth api-keys create \
  --name "CI Pipeline Key" \
  --scopes "traffic:read,traffic:export"

# Revoke an API key
madhyamas auth api-keys revoke --id <key-id>
```

## MCP Tools

Enterprise MCP tools allow AI agents to manage users, query audit logs, check license status, and monitor performance. All tools respect RBAC — the API key or JWT used by the agent must have sufficient permissions.

### User Management Tools

#### `madhyamas_list_users`

List all registered users.

```
madhyamas_list_users()
```

Returns: Array of user objects (id, username, email, role, status, created_at).

#### `madhyamas_create_user`

Create a new user.

```
madhyamas_create_user(
  username: "alice",
  email: "alice@example.com",
  password: "secure-password",
  role: "user"
)
```

Returns: Created user object.

#### `madhyamas_delete_user`

Delete a user by ID.

```
madhyamas_delete_user(user_id: "abc-123")
```

Returns: Success/failure confirmation.

#### `madhyamas_update_user_role`

Update a user's role.

```
madhyamas_update_user_role(
  user_id: "abc-123",
  role: "admin"
)
```

Returns: Updated user object.

### Audit Tools

#### `madhyamas_get_audit_events`

Query audit events with filters.

```
madhyamas_get_audit_events(
  event_type: "Login",
  limit: 50,
  offset: 0
)
```

Returns: Array of audit event objects.

#### `madhyamas_export_audit`

Export all audit events as JSON.

```
madhyamas_export_audit()
```

Returns: JSON string of all audit events.

### License & Health Tools

#### `madhyamas_get_license_info`

Get current license status and details.

```
madhyamas_get_license_info()
```

Returns: License object (customer, plan, seats, expiry, features).

#### `madhyamas_get_metrics`

Get current performance and operational metrics.

```
madhyamas_get_metrics()
```

Returns: Metrics object (requests, latency, throughput, intercept hits).

#### `madhyamas_get_health`

Get detailed health status.

```
madhyamas_get_health()
```

Returns: Health object (status, tier, dependencies, license, uptime).

### Configuration Tools

#### `madhyamas_export_config`

Export full Madhyamas configuration as JSON.

```
madhyamas_export_config()
```

Returns: JSON string of all configuration.

#### `madhyamas_import_config`

Import configuration from JSON.

```
madhyamas_import_config(config_json: '{"capture": {...}, "intercept": {...}}')
```

Returns: Success/failure confirmation.

## MCP Server Configuration

### With API Key

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001",
        "MADHYAMAS_API_KEY": "mad_abc123..."
      }
    }
  }
}
```

### With JWT Token

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://localhost:3001",
        "MADHYAMAS_TOKEN": "eyJ..."
      }
    }
  }
}
```

### HTTP Transport (Remote MCP)

```json
{
  "mcpServers": {
    "madhyamas": {
      "url": "http://madhyamas.internal:3001/mcp",
      "headers": {
        "X-API-Key": "mad_abc123..."
      }
    }
  }
}
```

## Example: AI Agent Managing Users

```python
# Example: An AI agent creates a new developer account
result = madhyamas_create_user(
    username="new-dev",
    email="new-dev@company.com",
    password="temporary-password",
    role="user"
)
print(f"Created user: {result['username']} with ID: {result['id']}")
```

## Example: CI Pipeline with API Key

```bash
#!/bin/bash
# CI script: Export traffic for analysis after test run

export MADHYAMAS_API_KEY=mad_ci_pipeline_key...
export MADHYAMAS_API_URL=http://madhyamas.internal:3001

# Export traffic from the test session
madhyamas export har --output test-traffic.har

# Query audit log for any config changes during the test
madhyamas audit list --event-type ConfigChanged --limit 10

# Check system health
madhyamas --json license info | jq .seats_used
```

## RBAC for Agents

AI agents are subject to the same RBAC as interactive users. The API key or JWT determines what the agent can do:

| Agent Task | Required Permission |
|-----------|-------------------|
| List users | `users:read` (admin role) |
| Create users | `users:write` (admin role) |
| Query audit | `audit:read` (admin role) |
| Export audit | `audit:export` (admin role) |
| Get metrics | `metrics:read` (admin role) |
| Export config | `config:read` (admin role) |
| Import config | `config:write` (admin role) |
| Get license info | No special permission (any authenticated user) |
| Get health | No special permission (public endpoint) |

::: tip Least privilege for agents
Create API keys with only the scopes the agent needs. For example, a monitoring agent only needs `metrics:read`, while a user management agent needs `users:read` and `users:write`.
:::

## See Also

- [Authentication](./authentication) — JWT, API keys, and SSO
- [User Management](./user-management) — User CRUD via web UI and API
- [Audit Logging](./audit-logging) — Audit events and compliance
- [RBAC](./rbac) — Roles and permissions
- [MCP & AI Agents](../mcp) — General MCP documentation
- [CLI Reference](../cli) — Full CLI command reference
