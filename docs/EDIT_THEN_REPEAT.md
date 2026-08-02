# Edit-then-Repeat

The edit-then-repeat feature allows you to modify a saved request before
replaying it. This is equivalent to Charles Proxy's "Edit" tool and is useful
for debugging, testing different payloads, or re-running requests against
different endpoints.

## Overview

When you replay a saved request, you can optionally apply
`RequestModifications` that override the original request's URL, method,
headers, and body. The modifications are **diff-based** — only the fields
that changed are sent to the backend, so the original request is preserved
and only the edited parts are overridden.

### RequestModifications

```json
{
  "url": "https://new-host.example.com/path",
  "method": "POST",
  "headers": { "Authorization": "Bearer token" },
  "remove_headers": ["Cookie"],
  "body": "{\"key\":\"value\"}",
  "follow_redirects": true
}
```

| Field | Type | Description |
|-------|------|-------------|
| `url` | `string?` | Override the request URL |
| `method` | `string?` | Override the HTTP method |
| `headers` | `object?` | Headers to add or replace (key-value) |
| `remove_headers` | `string[]?` | Header names to remove |
| `body` | `string?` | New request body (raw text) |
| `follow_redirects` | `boolean?` | Follow 3xx redirect responses (default: false) |

## Web UI

1. Save a request from the traffic view (Replay panel → "Save Current").
2. In the **Saved Requests** list, click **"Edit & Replay"** next to a
   saved request.
3. The **Request Editor** dialog opens, pre-filled with the saved request's
   method, URL, headers, and body.
4. Modify any fields:
   - **Method** — dropdown selector (GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS)
   - **URL** — text input
   - **Headers** — key-value editor with add/remove rows
   - **Content-Type** — selector that auto-detects from headers
   - **Body** — monospace textarea
5. Click **"Replay with Changes"**. The editor diffs the edited values
   against the original and sends only the changed fields as modifications.
6. The replay result dialog shows the response status, headers, and body.

The existing **"Replay"** button remains for quick no-edit replay.

## CLI

The `madhyamas replay run` command accepts modification flags:

```bash
# Simple replay (no modifications)
madhyamas replay run <saved-request-id>

# Override URL and method
madhyamas replay run <id> --url https://staging.example.com/api --method POST

# Add/replace headers (repeatable)
madhyamas replay run <id> \
  --header "Authorization: Bearer token" \
  --header "X-Custom: value"

# Override body from text
madhyamas replay run <id> --body '{"key":"value"}'

# Override body from a file
madhyamas replay run <id> --body-file ./payload.json

# Follow redirects
madhyamas replay run <id> --follow-redirects

# Output as JSON
madhyamas replay run <id> --json
```

### CLI Flags

| Flag | Description |
|------|-------------|
| `--url <URL>` | Override the request URL |
| `--method <METHOD>` | Override the HTTP method |
| `--header "Key: Value"` | Add/replace a header (repeatable) |
| `--body <TEXT>` | New request body (raw text) |
| `--body-file <PATH>` | Read request body from a file |
| `--follow-redirects` | Follow 3xx redirect responses |
| `--json` | Output result as JSON |

## MCP

The `madhyamas_replay_request` MCP tool accepts an optional `modifications`
object with the same fields as `RequestModifications`:

```json
{
  "id": "saved-request-id",
  "modifications": {
    "url": "https://staging.example.com/api",
    "method": "POST",
    "headers": {
      "Authorization": "Bearer token"
    },
    "body": "{\"key\":\"value\"}",
    "follow_redirects": true
  }
}
```

## API

Send a POST to `/api/replay/execute/{id}` with a JSON body containing
optional `modifications`:

```bash
curl -X POST http://127.0.0.1:3001/api/replay/execute/<saved-request-id> \
  -H "Content-Type: application/json" \
  -d '{
    "modifications": {
      "url": "https://staging.example.com/api",
      "headers": { "Authorization": "Bearer token" },
      "body": "{\"key\":\"value\"}"
    }
  }'
```

The response is a `ReplayResult` object containing the request that was sent,
the response received (status, headers, body), duration, and any error.

## Diff-based Modifications

The web UI's RequestEditor uses a **diff-based** approach: it compares the
edited request against the original and only sends the fields that changed.
This means:

- If only the URL changed, only `modifications.url` is sent.
- Added or changed headers go into `modifications.headers`.
- Removed headers go into `modifications.remove_headers`.
- If the body changed, `modifications.body` is sent with the new content.

This keeps the payload minimal and makes it easy to see exactly what was
changed in each replay.
