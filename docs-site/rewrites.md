---
title: Rewrites
description: Modify requests and responses automatically as they pass through the Madhyamas proxy — redirect traffic, inject headers, replace bodies, and fix URLs without pausing.
---

# Rewrites

Rewrites let you **modify requests and responses** as they pass through the proxy — without pausing traffic. Unlike breakpoints (which pause for manual intervention), rewrites apply automatically and instantly. Use them to redirect traffic, inject headers, replace response bodies, or fix URLs on the fly.

![Rewrites View](/screenshots/rewrites-view.png)

## How Rewrites Work

Rewrites are evaluated first in the interception pipeline, before mocks and breakpoints. When a request matches a rewrite rule:

1. The rewrite **modifies** the request (or response) according to your rules
2. The modified traffic continues through the pipeline normally
3. The original and modified versions are both visible in the traffic detail

## Quick Templates

For common scenarios, you don't have to configure rewrites by hand. Madhyamas ships **rewrite templates** — pre-built rules you can apply with a single click, including **No Caching** (disable all caching), **Block Cookies** (strip cookies both ways), **Add CORS**, **HTTP to HTTPS**, **Add Auth Header**, and **Remove Security Headers**. See the [Rewrite Templates](./rewrite-templates) guide for details.

## Creating a Rewrite

1. Navigate to the **Rewrites** view using the left navigation rail
2. Click **Add Rewrite**
3. Configure the match and action:

### Match Criteria

| Field | Description |
|-------|-------------|
| **URL Pattern** | Regex pattern to match the request URL (e.g., `https://api\.example\.com`). Empty matches **all** traffic — use with care. |
| **Method** | HTTP method to match — leave empty for any |
| **Phase** | Apply on **Request** or **Response** (or both) |

::: tip Escape regex metacharacters
The URL pattern and the URL rewrite pattern are both **regular expressions**, not glob/wildcard patterns. If you want to match a literal dot in a hostname, use `\.` — an unescaped `.` matches any character. For example, use `https://api\.example\.com` to match the literal host, not `https://api.example.com`.
:::

### Rewrite Actions

| Action | Direction | Description |
|--------|-----------|-------------|
| **Set Header** | Request / Response | Add or overwrite a header |
| **Remove Header** | Request / Response | Remove a specific header |
| **URL Rewrite** | Request | Replace part of the URL using a regex pattern and replacement string |
| **Body Rewrite** | Request / Response | Replace text in the body using a regex pattern and replacement string |

The backend also supports two additional actions via the [REST API](./rest-api) and [CLI](./cli) that are not yet exposed in the web UI:

| Action | Direction | Description |
|--------|-----------|-------------|
| **Map to URL** (`map_to_url`) | Request | Replace the entire request URL with a fixed target |
| **Map to File** (`map_to_file`) | Response | Replace the response body with the contents of a local file |

4. Click **Save** to activate the rewrite

## Common Use Cases

### Redirect API Calls to a Different Server

Map API calls from production to your local development server:

- **Match**: `*/api/*` on `production-server.com`
- **Action**: Redirect URL to `localhost:3000/api/*`

### Inject Authentication Headers

Add an authorization token to all API requests:

- **Match**: `*/api/*`
- **Action**: Add Header `Authorization: Bearer <your-token>`

### Replace Response Bodies

Swap out specific content in API responses for testing:

- **Match**: `*/api/config` on Response
- **Action**: Replace Body with custom JSON configuration

### Remove Security Headers for Testing

Strip Content-Security-Policy headers to test if your app works without them:

- **Match**: `*` on Response
- **Action**: Remove Header `Content-Security-Policy`

### Force HTTPS to HTTP

Redirect HTTPS requests to HTTP for easier debugging:

- **Match**: `https://*.example.com/*`
- **Action**: Replace URL `https://` → `http://`

## Managing Rewrites

### Enabling / Disabling

Each rewrite has a toggle switch. Disabled rewrites remain in your list but don't modify traffic. This lets you keep configurations ready and toggle them as needed.

### Priority

If multiple rewrites match the same request, they're applied in order of priority (lower number = higher priority). Set the priority when creating or editing a rewrite.

### Editing and Deleting

- Click any rewrite to edit its configuration
- Use the trash icon to delete a rewrite
- Rewrites are persisted across restarts

## See also

- [Rewrite Templates](./rewrite-templates) — pre-built rules for common scenarios
- [Breakpoints](./breakpoints) — interactive, one-off modifications
- [Mocks](./mocks) — return fake responses without hitting the server
- [Scripting](./scripting) — programmatic transformations
- [REST API reference](./rest-api) — `/api/rewrites` endpoints
