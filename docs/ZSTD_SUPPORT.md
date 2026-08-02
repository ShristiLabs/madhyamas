# zstd Content-Encoding Support

## Overview

[zstd](https://datatracker.ietf.org/doc/html/rfc8878) (Zstandard) is a modern
fast compression algorithm that offers better compression ratios than gzip
with comparable decompression speed. It is increasingly used by web servers
and CDNs via the `Content-Encoding: zstd` header (RFC 8878 / W3C
[Compression Stream Transport](https://www.w3.org/TR/compression-stream-transport/)).

Madhyamas stores raw compressed response bodies (the `Content-Encoding` header
is preserved) and decompresses them **on demand** when the user requests the
decompressed view. This preserves the original compressed bytes — useful for
debugging compression issues — while still letting you inspect the decoded
content.

Supported content encodings: **gzip**, **deflate**, **brotli**, and **zstd**.

## How It Works

### Backend decompression

The `decompress_body()` function in
`crates/madhyamas-core/src/proxy/pipeline.rs` handles all four encodings.
It is exposed as a public associated function on `Pipeline`:

```rust
Pipeline::decompress_body(
    content_encoding: Option<&str>,
    body: Vec<u8>,
    out_headers: &mut HashMap<String, String>,
) -> Option<Vec<u8>>
```

On success it removes the `Content-Encoding` header and updates
`Content-Length` to match the decompressed size. On failure it returns the
original body unchanged (so no data is ever lost).

### On-demand API endpoint

`GET /api/traffic/{id}` accepts an optional `?decompressed=true` query
parameter. When set, the handler calls `decompress_body()` on the response
body using the response's `Content-Encoding` header before returning the
entry. The original compressed body is left untouched in storage.

### Web UI

The body viewer in the Traffic Detail panel has a **Decompressed** toggle
(enabled by default for compressed responses). For gzip and deflate the
browser's native `DecompressionStream` API is used client-side. For **zstd**
— which is not supported by `DecompressionStream` in current browsers — the
UI fetches the decompressed body from the backend via
`?decompressed=true`. A purple **zstd** badge is shown next to the toggle
when the encoding is zstd.

## API Usage

### Get raw (compressed) body

```bash
curl http://127.0.0.1:3001/api/traffic/<id>
```

The `response.body` field contains the raw compressed bytes (UTF-8 or
base64-encoded for binary), and `response.headers` retains the
`Content-Encoding` header.

### Get decompressed body

```bash
curl 'http://127.0.0.1:3001/api/traffic/<id>?decompressed=true'
```

The `response.body` field contains the decompressed bytes, the
`Content-Encoding` header is removed, and `Content-Length` is updated to
the decompressed size.

### Example (JavaScript)

```js
// Fetch the decompressed response body
const entry = await fetch(
  `/api/traffic/${id}?decompressed=true`
).then(r => r.json());

console.log(entry.response.body); // decompressed content
```

## Web UI Usage

1. Select a traffic entry in the traffic list.
2. Open the **Response** tab in the detail panel.
3. If the response is compressed, a **Decompressed** toggle button appears
   in the body viewer toolbar (showing the encoding, e.g.
   `Decompressed (zstd)`).
4. Click the toggle to switch between the decompressed view and the raw
   compressed data.
5. A **zstd** badge is displayed when the encoding is zstd, indicating
   backend decompression is being used.

## Troubleshooting

### "Decompression failed — showing raw data"

This message appears when decompression could not be performed. Possible
causes:

- **Corrupt or truncated body** — The stored bytes are not valid zstd data.
  The original compressed bytes are shown instead.
- **Unsupported encoding** — An encoding other than gzip, deflate, brotli,
  or zstd is present. The raw body is returned as-is.
- **Backend unreachable** (zstd only) — The web UI could not reach the
  `?decompressed=true` endpoint. Check that the Madhyamas API server is
  running.

### zstd body shows as raw bytes in the web UI

Ensure the **Decompressed** toggle is enabled (it is on by default). If the
backend fetch fails, the UI falls back to showing the raw compressed data
with an error message.

### Request bodies are not decompressed

The `?decompressed=true` parameter only decompresses the **response** body.
Request bodies with `Content-Encoding` are stored as-is. This is by design —
request compression is rare and the raw bytes are preserved for debugging.

## Implementation Details

| Component | File | Change |
|-----------|------|--------|
| Workspace deps | `Cargo.toml` | `zstd = "0.13"` added; `"zstd"` added to reqwest features |
| Core crate | `crates/madhyamas-core/Cargo.toml` | `zstd.workspace = true` |
| Decompression | `crates/madhyamas-core/src/proxy/pipeline.rs` | `"zstd"` arm in `decompress_body()`; function made `pub` |
| API handler | `crates/madhyamas-api/src/handlers.rs` | `?decompressed=true` query param on `GET /api/traffic/{id}` |
| Web UI | `web/src/features/traffic/TrafficDetail.tsx` | zstd backend fetch + badge in body viewer |
