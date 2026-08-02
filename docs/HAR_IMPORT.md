# HAR Import

Madhyamas can import traffic from HAR (HTTP Archive) files. HAR is a
JSON-based format standardized by the W3C that records HTTP
request/response transactions. It is produced by browser DevTools
(Chrome, Firefox, Edge), Charles Proxy, Fiddler, and many other tools.

Importing a HAR file creates a **new session** containing all the
imported traffic entries, so your live capture is never disturbed.

## Why use HAR import?

- **Replay traffic captured elsewhere** — load a HAR exported from
  Chrome DevTools or another proxy and inspect it with Madhyamas'
  filtering, search, and detail views.
- **Share bug reports** — a teammate can send you a HAR file and you
  can load it directly into Madhyamas for analysis.
- **Offline analysis** — import a HAR recorded on a mobile device or
  in CI and examine the requests without needing a live proxy.

## Using the Web UI

1. Open the **Traffic** view in the Madhyamas web UI.
2. Click the **Import** button (upload icon) in the traffic sub-toolbar,
   next to the **Export** button.
3. Select a `.har` file from your filesystem.
4. The file is parsed and sent to the server. A new session named
   `"Imported HAR"` is created and automatically switched to.
5. A confirmation alert shows how many entries were imported and how
   many were skipped.

## Using the CLI

```bash
# Import a HAR file into a new session
madhyamas traffic import-har /path/to/traffic.har

# Name the new session and switch to it
madhyamas traffic import-har /path/to/traffic.har --name "Chrome capture" --switch

# Output the full result as JSON
madhyamas traffic import-har /path/to/traffic.har --json
```

The CLI reads the HAR file from disk, parses it as JSON, and sends it
to the running Madhyamas server's API.

## Using the MCP tool

AI agents integrated via the Model Context Protocol can call the
`madhyamas_import_har` tool:

```json
{
  "tool": "madhyamas_import_har",
  "arguments": {
    "har": { "log": { "version": "1.2", "entries": [ ... ] } },
    "session_name": "Imported from agent",
    "switch_session": true
  }
}
```

The `har` argument must be the full HAR JSON object. The tool returns
an `ImportResult` with `session_id`, `imported_count`,
`skipped_count`, and an `errors` array.

## API usage (curl)

```bash
curl -X POST http://127.0.0.1:3001/api/traffic/import/har \
  -H "Content-Type: application/json" \
  -d '{
    "har": '"$(cat traffic.har)"',
    "session_name": "My imported session",
    "switch_session": true
  }'
```

**Request body:**

| Field            | Type    | Required | Description                                              |
|------------------|---------|----------|----------------------------------------------------------|
| `har`            | object  | yes      | The full HAR JSON document (`{ "log": { ... } }`).       |
| `session_name`   | string  | no       | Name for the new session (default: `"Imported HAR"`).    |
| `switch_session` | boolean | no       | Switch active session to the new one (default: `false`). |

**Response (200 OK):**

```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "imported_count": 42,
  "skipped_count": 1,
  "errors": ["entry 7: HAR entry missing 'request' field"]
}
```

## Supported HAR versions

Both **HAR 1.1** and **HAR 1.2** are supported. The importer reads the
`log.entries[]` array and converts each entry's `request` and
`response` objects into Madhyamas traffic entries.

### Field mapping

| HAR field                         | Madhyamas field                 |
|-----------------------------------|---------------------------------|
| `entry.request.method`            | `request.method`                |
| `entry.request.url`               | `request.url`, `host`, `path`   |
| `entry.request.headers[]`         | `request.headers`               |
| `entry.request.postData.text`     | `request.body`                  |
| `entry.request.httpVersion`       | `request.http_version`          |
| `entry.response.status`           | `response.status_code`          |
| `entry.response.statusText`       | `response.status_message`       |
| `entry.response.headers[]`        | `response.headers`              |
| `entry.response.content.text`     | `response.body`                 |
| `entry.response.content.mimeType` | `response.content_type`         |
| `entry.startedDateTime`           | `timestamp`                     |
| `entry.time`                      | `response.duration_ms`          |

### Base64-encoded bodies

HAR allows binary bodies to be base64-encoded. When
`content.encoding` (response) or `postData.encoding` (request) is
`"base64"`, Madhyamas decodes the body before storing it.

## Error handling

Import is **best-effort**: a single invalid entry does not abort the
entire import. Invalid entries are skipped and their error messages are
collected in the `errors` array of the `ImportResult`. The
`imported_count` and `skipped_count` fields report how many entries
succeeded and failed, respectively.

The HAR document itself is validated before import — it must contain a
`log` object with an `entries` array, otherwise a `400 Bad Request` is
returned.

## Limitations

- **No WebSocket traffic** — HAR does not capture WebSocket frames, so
  imported sessions will not contain WebSocket data.
- **No granular timing** — only the total `entry.time` (in
  milliseconds) is recorded as `duration_ms`; HAR's detailed
  `timings` block (blocked, DNS, connect, send, wait, receive) is not
  preserved.
- **No SSL certificate details** — HAR's `serverCertificate` fields
  are not imported.
- **Headers are flattened** — HAR headers with the same name are
  merged into a single comma-separated value (matching HTTP semantics).
