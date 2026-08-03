# Importing HAR Files

Madhyamas can import traffic from **HAR (HTTP Archive)** files. HAR is a JSON-based standard produced by browser DevTools (Chrome, Firefox, Edge), Charles Proxy, Fiddler, and many other tools. Importing a HAR file creates a **new session** containing all the entries, so your live capture is never disturbed.

## Why Import HAR?

- **Replay traffic captured elsewhere** — load a HAR exported from Chrome DevTools or another proxy and inspect it with Madhyamas' filtering, search, and detail views.
- **Share bug reports** — a teammate can send you a HAR file and you can load it directly into Madhyamas for analysis.
- **Offline analysis** — import a HAR recorded on a mobile device or in CI and examine the requests without needing a live proxy.

## From the Web UI

1. Open the **Traffic** view
2. Click the **Import** button (upload icon) in the traffic sub-toolbar, next to the **Export** button
3. Select a `.har` file from your filesystem
4. The file is parsed and a new session named "Imported HAR" is created and automatically switched to
5. A confirmation alert shows how many entries were imported and how many were skipped

## From the CLI

```bash
# Import a HAR file into a new session
madhyamas traffic import-har /path/to/traffic.har

# Name the new session and switch to it
madhyamas traffic import-har /path/to/traffic.har --name "Chrome capture" --switch

# Output the full result as JSON
madhyamas traffic import-har /path/to/traffic.har --json
```

The CLI reads the HAR file from disk, parses it, and sends it to the running Madhyamas server.

## Supported HAR Versions

Both **HAR 1.1** and **HAR 1.2** are supported. The importer reads the `log.entries[]` array and converts each entry's `request` and `response` objects into Madhyamas traffic entries.

### Field Mapping

| HAR field | Madhyamas field |
|-----------|-----------------|
| `entry.request.method` | `request.method` |
| `entry.request.url` | `request.url`, `host`, `path` |
| `entry.request.headers[]` | `request.headers` |
| `entry.request.postData.text` | `request.body` |
| `entry.response.status` | `response.status_code` |
| `entry.response.headers[]` | `response.headers` |
| `entry.response.content.text` | `response.body` |
| `entry.response.content.mimeType` | `response.content_type` |
| `entry.startedDateTime` | `timestamp` |
| `entry.time` | `response.duration_ms` |

### Base64-Encoded Bodies

HAR allows binary bodies to be base64-encoded. When `content.encoding` (response) or `postData.encoding` (request) is `"base64"`, Madhyamas decodes the body before storing it.

## Error Handling

Import is **best-effort**: a single invalid entry does not abort the entire import. Invalid entries are skipped and their error messages are collected in the result. The `imported_count` and `skipped_count` fields tell you how many entries succeeded and failed.

## Limitations

- **No WebSocket traffic** — HAR does not capture WebSocket frames, so imported sessions won't contain WebSocket data.
- **No granular timing** — only the total `entry.time` (in milliseconds) is recorded as `duration_ms`; HAR's detailed `timings` block is not preserved.
- **No SSL certificate details** — HAR's `serverCertificate` fields are not imported.
- **Headers are flattened** — HAR headers with the same name are merged into a single comma-separated value (matching HTTP semantics).

## Common Use Cases

### Analyzing a Browser Capture

Export a HAR from Chrome DevTools while reproducing a bug, then import it into Madhyamas to use the richer filtering, search, and detail views — or to replay specific requests.

### Sharing Debugging Context

A teammate sends you a HAR file from their machine. Import it into a named session to investigate without needing to reproduce the issue yourself.

### Cross-Tool Migration

Bring traffic captured in Charles Proxy or Fiddler into Madhyamas by exporting it as HAR first, then importing it here.
