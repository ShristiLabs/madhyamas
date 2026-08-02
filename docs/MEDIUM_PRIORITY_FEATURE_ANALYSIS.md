# Medium-Priority Feature Analysis — Detailed Implementation Plan

This document provides a detailed analysis of the 9 medium-priority features
identified in [CHARLES_PROXY_FEATURE_COMPARISON.md](CHARLES_PROXY_FEATURE_COMPARISON.md)
(Section 4, "Medium Priority (utility & UX)", items 8–16). For each feature it
documents:

- **What exists now** — current code, with file paths and line numbers
- **What needs to be done** — concrete work items
- **Where it needs to be done** — exact files to modify or create
- **How it should be done** — implementation approach and design decisions
- **How it would show up in the UI** — web UI, CLI, and API surface
- **How it can be tested** — verification strategy
- **What needs to be documented** — docs to create or update

> All file paths are relative to the repository root
> (`/Users/harikiranbavineni/madhyamas/`).

---

## Table of Contents

1. [Repeat Advanced (Concurrency / Iterations)](#1-repeat-advanced-concurrency--iterations)
2. [Edit-then-Repeat](#2-edit-then-repeat)
3. [Chart / Timeline Visualization](#3-chart--timeline-visualization)
4. [Focus Feature (Host Highlighting)](#4-focus-feature-host-highlighting)
5. [Mirror Tool (Save Responses to Disk)](#5-mirror-tool-save-responses-to-disk)
6. [Auto Save (Periodic Session Save)](#6-auto-save-periodic-session-save)
7. [Recording Size Limits](#7-recording-size-limits)
8. [HAR Import](#8-har-import)
9. [zstd Decompression](#9-zstd-decompression)
10. [Implementation Priority Order](#implementation-priority-order)

---

## 1. Repeat Advanced (Concurrency / Iterations)

### What exists now

The replay subsystem supports **single-request replay only** — no concurrency,
no iteration count, no delay between requests.

| Aspect | Location | Current State |
|---|---|---|
| Core replay manager | `crates/madhyamas-core/src/replay.rs:104-112` | `ReplayManager` struct — in-memory `saved_requests` + `history` with `parking_lot::RwLock`; `max_history = 500` (FIFO eviction) |
| Main replay method | `crates/madhyamas-core/src/replay.rs:204-257` | `pub async fn replay(&self, id: &str, modifications: Option<RequestModifications>) -> ReplayResult` — executes **one** request via a per-call `reqwest::Client` (line 276), 120s timeout (line 279), `.no_proxy()` to avoid feedback loop (line 278) |
| Modifications struct | `crates/madhyamas-core/src/replay.rs:386-400` | `RequestModifications { url, method, headers, remove_headers, body, follow_redirects }` — already supports per-replay edits, but UI never passes it |
| Replay result | `crates/madhyamas-core/src/replay.rs:55-71` | `ReplayResult { id, saved_request_id, request, response, error, executed_at, duration_ms }` — single result per replay |
| Persistence | `crates/madhyamas-core/src/replay.rs:365-371` | `Persistable` impl returns `Ok(())` — "In-memory only for now; no backing store wired up yet" |
| API handler | `crates/madhyamas-api/src/intercept_handlers.rs:1147-1154` | `replay_request(State, Path<id>, Json<ReplayRequest { modifications }>)` — returns single `ReplayResult` |
| API routes | `crates/madhyamas-api/src/routes.rs:262-284` | `POST /replay/execute/{id}` (single), `GET /replay/history`, `DELETE /replay/history` |
| CLI | `crates/madhyamas-cli/src/commands/replay.rs:61-74` | `ReplayCommands` enum: `Run`, `Save`, `List`, `Delete`, `Export`, `History` — no batch/concurrency subcommand |
| MCP tools | `crates/madhyamas-mcp/src/tools/replay.rs:9-38` | `replay_request(client, api_url, traffic_id, modifications)` → `POST /api/replay/execute/{id}`; registry schema at `registry.rs:579-607` |
| Web UI panel | `web/src/features/tools/ReplayPanel.tsx:98-143` | Saved-requests list with Replay/Delete buttons; line 127 calls `replayRequest.mutateAsync({ id: saved.id })` with **no modifications** |
| Web API hook | `web/src/lib/api/intercept.ts:755-765` | `useReplayRequest()` already accepts `{ id, modifications? }` — UI just doesn't expose it |
| Web types | `web/src/lib/api/intercept.ts:196-221` | `SavedRequest`, `ReplayResult` — no batch/aggregate result type |

### What needs to be done

1. **Add a batch/advanced replay method** to `ReplayManager` that runs N
   iterations with optional concurrency and inter-request delay
2. **Add an aggregate result type** summarizing success/failure counts and
   timing statistics
3. **Add a new API endpoint** for batch replay (keep the existing single-replay
   endpoint unchanged)
4. **Add CLI subcommand** `madhyamas replay run-advanced` with `--iterations`,
   `--concurrency`, `--delay-ms` flags
5. **Add MCP tool** `madhyamas_replay_advanced` for AI-driven load testing
6. **Add web UI controls** in `ReplayPanel` for iterations, concurrency, delay,
   and a results summary view

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/replay.rs` | Add `ReplayBatchConfig { iterations: usize, concurrency: usize, delay_ms: Option<u64> }` struct; add `pub async fn replay_batch(&self, id: &str, modifications: Option<RequestModifications>, config: ReplayBatchConfig) -> ReplayBatchResult`; use `futures::stream::buffer_unordered` or `tokio::task::JoinSet` for concurrency; add `ReplayBatchResult { results: Vec<ReplayResult>, total, succeeded, failed, min_ms, max_ms, avg_ms, p95_ms }` |
| `crates/madhyamas-api/src/intercept_handlers.rs` | Add `replay_request_batch` handler accepting `ReplayBatchRequest { modifications: Option<RequestModifications>, config: ReplayBatchConfig }`; return `ReplayBatchResult` |
| `crates/madhyamas-api/src/routes.rs` | Add `POST /replay/execute/{id}/batch` route |
| `crates/madhyamas-cli/src/commands/replay.rs` | Add `RunAdvanced(ReplayAdvancedArgs)` variant with `--iterations <N>`, `--concurrency <N>`, `--delay-ms <MS>` flags; call the batch endpoint |
| `crates/madhyamas-mcp/src/tools/replay.rs` | Add `replay_request_advanced()` function |
| `crates/madhyamas-mcp/src/tools/registry.rs` | Register `madhyamas_replay_advanced` tool schema |
| `crates/madhyamas-mcp/src/tools/executor.rs` | Dispatch the new tool |
| `web/src/lib/api/intercept.ts` | Add `ReplayBatchConfig`, `ReplayBatchResult` interfaces; add `useReplayRequestBatch()` mutation hook calling `POST /replay/execute/{id}/batch` |
| `web/src/features/tools/ReplayPanel.tsx` | Add "Advanced" toggle/section with iterations input, concurrency slider, delay input; show aggregate results (success/failure counts, min/avg/max/p95 latency) |
| `web/src/types/` | Add batch result types if not colocated in `intercept.ts` |

### How it should be done

**Batch replay implementation (in `replay.rs`):**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBatchConfig {
    pub iterations: usize,    // total requests to send
    pub concurrency: usize,   // simultaneous in-flight requests
    pub delay_ms: Option<u64>, // delay between dispatches
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBatchResult {
    pub saved_request_id: String,
    pub results: Vec<ReplayResult>,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub min_ms: u64,
    pub max_ms: u64,
    pub avg_ms: u64,
    pub p95_ms: u64,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

impl ReplayManager {
    pub async fn replay_batch(
        &self,
        id: &str,
        modifications: Option<RequestModifications>,
        config: ReplayBatchConfig,
    ) -> ReplayBatchResult {
        // Build a stream of `iterations` futures, each calling self.replay(...)
        // Apply .buffer_unordered(config.concurrency) for concurrency control
        // Optional tokio::time::sleep for delay_ms between dispatches
        // Collect results, compute statistics
    }
}
```

**Key design decisions:**
- **Keep the single-replay endpoint unchanged** — backward compatible. The
  batch endpoint is additive.
- **Per-iteration modifications are identical** — for true per-iteration
  variation (e.g., incrementing a counter in the body), a future enhancement
  could support template variables. For now, all iterations use the same
  modifications.
- **Concurrency via `buffer_unordered`** — idiomatic Tokio approach; preserves
  result ordering by index if needed, but results can be collected in
  completion order.
- **History cap** — batch results should be summarized in history, not stored
  as N individual `ReplayResult` entries (would blow past `max_history = 500`).
  Store one `ReplayBatchResult` summary instead, or cap the per-batch history
  insertions.
- **Safety limits** — cap `iterations` (e.g., max 10,000) and `concurrency`
  (e.g., max 100) to prevent accidental DoS of the user's own machine or the
  target server.

### How it would show up in the UI

- **ReplayPanel**: An "Advanced" expander or mode toggle revealing:
  - Iterations input (number, default 10)
  - Concurrency slider (1–100, default 1)
  - Delay between requests (ms, default 0)
  - "Run Batch" button
- **Results view**: After batch completes, show a summary card:
  - Total / Succeeded / Failed counts
  - Min / Avg / Max / P95 latency
  - A small histogram or sparkline of response times (optional, ties into
    feature #3)
  - Expandable list of individual results for debugging failures
- **CLI**: `madhyamas replay run-advanced <id> --iterations 100 --concurrency 10 --delay-ms 50`
- **MCP**: `madhyamas_replay_advanced` with `id`, `iterations`, `concurrency`,
  `delay_ms`, optional `modifications`
- **API**: `POST /api/replay/execute/{id}/batch` with JSON body
  `{ "modifications": {...}, "config": { "iterations": 100, "concurrency": 10, "delay_ms": 50 } }`

### How it can be tested

1. **Unit test**: `replay_batch` with `iterations=5, concurrency=2` against a
   mock HTTP server; verify 5 results, correct success/failure counts
2. **Concurrency test**: Verify `concurrency=10` actually runs 10 in-flight
   (use a slow endpoint and check timing overlap)
3. **Statistics test**: Verify min/avg/max/p95 computation with known durations
4. **Delay test**: Verify inter-request delay is respected
5. **Safety limit test**: Verify `iterations` > cap is rejected with 400
6. **API test**: `curl -X POST .../replay/execute/{id}/batch -d '{...}'`
7. **CLI test**: `madhyamas replay run-advanced <id> --iterations 20 --concurrency 4`
8. **UI test**: Configure batch in ReplayPanel, run, verify summary displays
9. **Load test**: Run 1000 iterations against a real API and verify the proxy
   and UI remain responsive

### What needs to be documented

- Update `CLAUDE.md` — add `POST /replay/execute/{id}/batch` to the Replay API
  endpoints table; add `madhyamas replay run-advanced` to CLI examples
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Repeat Advanced
  row from ❌ to ✅
- Update the madhyamas skill — add Repeat Advanced workflow and MCP tool
- Create or update `docs/REPLAY.md` — document single vs batch replay,
  concurrency/delay tuning, safety limits, and load-testing use cases

---

## 2. Edit-then-Repeat

### What exists now

The replay API **already accepts modifications** (`RequestModifications`), and
the web API hook **already passes them through** — but the UI never exposes an
editor. There is also a richer `Modification` enum used by breakpoints that
could be reused.

| Aspect | Location | Current State |
|---|---|---|
| Replay modifications struct | `crates/madhyamas-core/src/replay.rs:386-400` | `RequestModifications { url, method, headers, remove_headers, body, follow_redirects }` — backend fully supports it |
| Replay API handler | `crates/madhyamas-api/src/intercept_handlers.rs:1142-1154` | `ReplayRequest { modifications: Option<RequestModifications> }` — accepts modifications, applies them in `replay()` |
| Web API hook | `web/src/lib/api/intercept.ts:755-765` | `useReplayRequest()` accepts `{ id, modifications? }` — **already wired**, UI just doesn't call it with modifications |
| Replay UI | `web/src/features/tools/ReplayPanel.tsx:127` | Calls `replayRequest.mutateAsync({ id: saved.id })` — **no modifications passed**; no edit dialog |
| Breakpoint `Modification` enum | `crates/madhyamas-core/src/intercept/types.rs:348-378` | Richer enum: `SetHeader`, `RemoveHeader`, `SetBody`, `SetBodyBase64`, `SetUrl`, `SetPath`, `SetStatusCode`, `RegexReplace`, `UrlRegexReplace`, `Delay` |
| Breakpoint apply functions | `crates/madhyamas-core/src/intercept/breakpoint.rs:285-397` | `apply_request_modifications()` and `apply_response_modifications()` — reusable logic for editing requests |
| Breakpoint UI | `web/src/features/tools/BreakpointsPanel.tsx:337-398` | `PausedTrafficItem` shows paused requests read-only (headers/body) — **no edit UI** either |
| MCP replay tool | `crates/madhyamas-mcp/src/tools/replay.rs:9-38` | `replay_request(..., modifications: Option<Value>)` — already supports modifications in schema (`registry.rs:579-607`) |

### What needs to be done

1. **Build a reusable request editor component** in the web UI (shared between
   replay and breakpoints)
2. **Wire the editor into `ReplayPanel`** — "Edit & Replay" button opens the
   editor, then submits with modifications
3. **Optionally extend `RequestModifications`** to support richer edits
   (regex replace, base64 body) by aligning with the breakpoint `Modification`
   enum
4. **Add CLI support** for passing modifications (e.g., `--header`,
   `--body-file`, `--url`)
5. **Document the edit-then-repeat workflow**

### Where it needs to be done

| File | Change |
|---|---|
| `web/src/features/traffic/RequestEditor.tsx` | **New file** — reusable request editor component with fields: method (dropdown), URL, headers (key-value editor with add/remove), body (textarea with content-type selector), content-type selector |
| `web/src/features/tools/ReplayPanel.tsx` | Replace direct replay button (line 127) with two buttons: "Replay" (no edits) and "Edit & Replay" (opens `RequestEditor` in a dialog, then calls `replayRequest.mutateAsync({ id, modifications })`) |
| `web/src/features/tools/BreakpointsPanel.tsx` | (Optional, future) Reuse `RequestEditor` for breakpoint modification UI |
| `web/src/lib/api/intercept.ts` | No change needed — hook already supports modifications; may add a `RequestModifications` TS interface for type safety |
| `crates/madhyamas-core/src/replay.rs` | (Optional) Extend `RequestModifications` with `regex_replaces: Vec<(String, String)>` and `body_base64: Option<String>` to match breakpoint capabilities |
| `crates/madhyamas-cli/src/commands/replay.rs` | Add `--header "Key: Value"` (repeatable), `--body <text>`, `--body-file <path>`, `--url <url>`, `--method <METHOD>` flags to `Run` command; build `RequestModifications` from them |
| `crates/madhyamas-mcp/src/tools/replay.rs` | No change needed — already accepts modifications; update tool description to mention edit-then-repeat |

### How it should be done

**Reusable `RequestEditor` component:**

```tsx
interface RequestEditorProps {
  initialRequest: RequestData;
  onSubmit: (modifications: RequestModifications) => void;
  onCancel: () => void;
}

// Fields:
// - Method dropdown (GET/POST/PUT/PATCH/DELETE/HEAD/OPTIONS)
// - URL text input
// - Headers editor: list of {name, value} rows with add/remove buttons
// - Content-Type selector (auto-detected from headers)
// - Body textarea (monospace, with syntax highlighting via Prism.js — already a dep)
// - Body content-type toggle: text / base64 (for binary bodies)
```

**Modifications construction:** The editor diffs the edited request against the
original and produces a `RequestModifications`:
- If URL changed → `modifications.url = Some(new_url)`
- If method changed → `modifications.method = Some(new_method)`
- Added/changed headers → `modifications.headers.insert(name, value)`
- Removed headers → `modifications.remove_headers.push(name)`
- If body changed → `modifications.body = Some(new_body)`

**Key design decisions:**
- **Diff-based modifications** — only send what changed, not the whole
  request. This keeps the payload small and avoids clobbering headers the user
  didn't touch.
- **Reuse for breakpoints** — the same `RequestEditor` can later be wired into
  `BreakpointsPanel` for the breakpoint edit workflow (currently read-only).
  This addresses two gaps with one component.
- **Base64 body support** — for binary request bodies (e.g., file uploads),
  offer a "base64" mode. This aligns with `Modification::SetBodyBase64` in the
  breakpoint system.
- **No backend changes required for the basic case** — `RequestModifications`
  and the API already support everything needed. Backend changes are only
  needed if we want regex replace or base64 body (optional enhancement).

### How it would show up in the UI

- **ReplayPanel**: Each saved request row gets two action buttons:
  - **Replay** (lightning icon) — replays immediately, no edits (current behavior)
  - **Edit & Replay** (pencil icon) — opens `RequestEditor` dialog pre-filled
    with the saved request's method/URL/headers/body; user edits, clicks
    "Send", modifications are passed to the API
- **Traffic detail**: Right-click a traffic entry → "Edit & Replay" (saves the
  request first, then opens the editor) — quick path from inspection to
  modified replay
- **CLI**: `madhyamas replay run <id> --header "Authorization: Bearer newtoken" --body '{"key":"value"}' --url https://staging.example.com/api`
- **MCP**: `madhyamas_replay_request` with `modifications` object (already
  supported in the schema)
- **API**: `POST /api/replay/execute/{id}` with `{ "modifications": { "url": "...", "headers": {...}, "body": "..." } }` (already works)

### How it can be tested

1. **Editor test**: Open the editor, change the URL, verify the modifications
  payload contains only the URL change
2. **Header add test**: Add a header in the editor, replay, verify the upstream
  request contains the new header
3. **Header remove test**: Remove an existing header, replay, verify it's
  absent upstream
4. **Body change test**: Edit the body, replay, verify the upstream body
  matches
5. **Method change test**: Change POST to PUT, replay, verify method changes
6. **Base64 body test**: Switch to base64 mode, paste base64, replay, verify
  binary body is sent correctly
7. **No-change test**: Open editor, change nothing, submit — verify
  modifications is `null` or empty (no-op replay)
8. **CLI test**: `madhyamas replay run <id> --header "X-Test: 1" --body 'hello'`
9. **MCP test**: Call `madhyamas_replay_request` with modifications via an AI
  agent
10. **Breakpoint reuse test**: (Future) Verify the same editor works in the
    breakpoint paused-request UI

### What needs to be documented

- Update `CLAUDE.md` — note the edit-then-repeat workflow and `RequestEditor`
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Edit row from ❌
  to ✅
- Update the madhyamas skill — add edit-then-repeat workflow
- Create or update `docs/REPLAY.md` — document the editor, modification
  options, and CLI flags

---

## 3. Chart / Timeline Visualization

### What exists now

The web UI displays traffic as a **flat list/table** with no graphical
timeline. Timing data is available but only shown as text in the detail view.

| Aspect | Location | Current State |
|---|---|---|
| Traffic list | `web/src/features/traffic/TrafficList.tsx:199-213` | Table columns: Method, Proto, Domain, Path, Status, Time (duration_ms), Size, When (timestamp). Row height 26px. Virtual scrolling via `@tanstack/react-virtual` |
| Traffic detail timing | `web/src/features/traffic/TrafficDetail.tsx:271-319` | "Timing" tab shows text: Timestamp, Duration (ms), Request Size, Response Size — **no chart** |
| Traffic types (web) | `web/src/types/traffic.ts:24-38` | `TrafficEntry` has `timestamp: string` (ISO 8601), `response?.duration_ms: number`, `request_size`, `response_size` |
| Traffic types (Rust) | `crates/madhyamas-core/src/traffic/types.rs:208-236` | `TrafficEntry { timestamp: DateTime<Utc>, response: Option<ResponseData { duration_ms: u64 }> }` |
| DB schema | `crates/madhyamas-core/src/traffic/store.rs:111-139` | `requests.timestamp INTEGER` (Unix epoch), `responses.duration_ms INTEGER` — **no granular timing breakdown** (DNS/TCP/TTFB not captured) |
| Charting library | `web/package.json` | **None installed** — no recharts, chart.js, d3, visx, or nivo |
| Traffic API | `crates/madhyamas-api/src/handlers.rs:16-76` | `TrafficQuery` supports `limit`, `offset`, `search`, `method`, `status_code`, `file_type`, `header`, `cookie`, `is_passthrough` — **no `host` filter param** despite `TrafficFilter` having a `host` field |
| Export HAR | `web/src/features/traffic/TrafficView.tsx:124-145` | HAR export exists; `time` field = `duration_ms` |

### What needs to be done

1. **Install a charting library** (recharts recommended — React-native, simple
   API, good for bar/timeline charts)
2. **Create a waterfall/timeline component** showing each request as a
   horizontal bar positioned by start time, length proportional to duration
3. **Integrate as a toggleable view** in `TrafficView` (list view ↔ timeline
   view)
4. **Add a mini-chart to the traffic detail** timing tab
5. **Color-code bars** by status code class (2xx green, 3xx blue, 4xx orange,
   5xx red) or by duration thresholds

### Where it needs to be done

| File | Change |
|---|---|
| `web/package.json` | Add `recharts` dependency (or `visx`, `nivo` — recharts is simplest for this use case) |
| `web/src/features/traffic/TrafficTimeline.tsx` | **New file** — waterfall chart component: X-axis = time range (min timestamp to max timestamp + duration), Y-axis = one row per request, bar = [start, start+duration_ms], color by status code; tooltip on hover shows method/host/path/status/duration; click selects entry |
| `web/src/features/traffic/TrafficView.tsx` | Add view toggle (List / Timeline) in the toolbar; render `TrafficTimeline` when timeline mode is selected, passing the same filtered traffic data |
| `web/src/features/traffic/TrafficDetail.tsx:271-319` | Replace text-only timing tab with a mini waterfall bar + the existing text fields |
| `web/src/types/traffic.ts` | No change — `timestamp` and `duration_ms` are sufficient for a basic waterfall |

### How it should be done

**Waterfall chart design:**

```
Time →  0ms    100ms   200ms   300ms   400ms   500ms
        │       │       │       │       │       │
GET  /api/users     ████████████░░░░                    200 (120ms)
POST /api/login     ░░░░██████████████████████          200 (250ms)
GET  /api/data      ░░░░░░░░░░░░██████████████████████  500 (180ms)
GET  /static/app.js ░░░░░░░░░░░░░░░░░░░░██████░░░░░░░░  200 (80ms)
        │       │       │       │       │       │
        Request phase (░) | Response phase (█)
```

**Data mapping:**
- Each `TrafficEntry` → one row
- Bar start = `entry.timestamp` (relative to the earliest entry in the visible
  set)
- Bar length = `entry.response?.duration_ms ?? 0`
- Color: green (2xx), blue (3xx), orange (4xx), red (5xx), gray (pending/no
  response)
- Since we don't have granular DNS/TCP/TTFB breakdown, the entire bar is one
  segment. A future enhancement (with HTTP/2 support, feature #1 of the
  high-priority doc) could add timing phases.

**Key design decisions:**
- **recharts vs custom SVG** — recharts is faster to implement but less
  flexible for waterfall layouts. For a true waterfall (variable-length bars
  per row), a custom SVG or visx-based approach may be cleaner. Recommendation:
  start with a simple recharts `BarChart` with horizontal bars; if the layout
  isn't right, switch to a custom SVG component.
- **Virtualization** — for large traffic sets (1000+ entries), the timeline
  must virtualize rows (only render visible ones). Use
  `@tanstack/react-virtual` (already installed) for the Y-axis virtualization.
- **Time axis** — relative time from the first visible entry, not absolute
  wall-clock. Show absolute time in the tooltip.
- **Selection sync** — clicking a bar in the timeline should select the same
  entry in the list view and open the detail panel (shared state via
  `TrafficView`).

### How it would show up in the UI

- **TrafficView toolbar**: New toggle button group: "List" | "Timeline"
  (default: List)
- **Timeline view**: Full-height waterfall chart replacing the table; scroll
  vertically through requests; horizontal time axis at top; color legend
  (2xx/3xx/4xx/5xx) in a corner
- **Hover tooltip**: Method, host, path, status, duration, size
- **Click**: Selects entry → opens detail panel (same as clicking a row in
  list view)
- **Traffic detail timing tab**: Mini waterfall bar showing this request's
  duration in context, plus the existing text fields
- **No CLI/API change** — this is a pure frontend visualization feature

### How it can be tested

1. **Render test**: Capture 20+ requests, switch to timeline view, verify bars
   render with correct positions and lengths
2. **Color test**: Verify 2xx=green, 3xx=blue, 4xx=orange, 5xx=red
3. **Tooltip test**: Hover a bar, verify tooltip shows correct method/host/
   status/duration
4. **Selection test**: Click a bar, verify the detail panel opens with the
   correct entry
5. **Virtualization test**: Capture 5000+ requests, verify timeline scrolls
   smoothly (no rendering all 5000 bars at once)
6. **Empty state test**: No traffic → show "No traffic to display" message
7. **Pending request test**: A request with no response yet → gray bar with
   zero or minimal length
8. **Time axis test**: Verify the time axis scales correctly (ms for fast
   bursts, seconds for long captures)
9. **Toggle test**: Switch between List and Timeline views, verify state
   (selection, filters) is preserved

### What needs to be documented

- Update `CLAUDE.md` — mention the timeline view in the web UI description
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Chart/timeline
  row from ❌ to ✅
- Update the madhyamas skill — add timeline view to the traffic inspection
  workflow
- Create `docs/TIMELINE_VIEW.md` — guide on using the waterfall chart,
  color coding, and interpreting timing data

---

## 4. Focus Feature (Host Highlighting)

### What exists now

There is **no focus/highlight functionality**. Host filtering exists in the
filter builder but there's no dedicated "Focus" UI that highlights specific
hosts and dims the rest.

| Aspect | Location | Current State |
|---|---|---|
| Session preset filter | `crates/madhyamas-core/src/session.rs:42-48` | `SessionPreset { filter_host_patterns: Vec<String>, ... }` — defined but **not actively applied** in the session manager |
| Traffic filter (Rust) | `crates/madhyamas-core/src/traffic/types.rs` | `TrafficFilter` has a `host` field |
| Traffic API query | `crates/madhyamas-api/src/handlers.rs:16-29` | `TrafficQuery` — **no `host` parameter** despite `TrafficFilter` supporting it |
| Web filter builder | `web/src/features/traffic/` (FilterBuilder) | Supports domain filtering via the filter UI |
| Web traffic types | `web/src/types/traffic.ts:42-56` | `TrafficFilter { host?: string, ... }` — host field exists |
| Focus/highlight code | — | **None** — grep for "focus", "highlight", "starred", "favorite" finds only CSS `:focus` states and syntax highlighting |
| Persistence | — | No table or config for focused/starred hosts |

### What needs to be done

1. **Add a focus hosts store** — a persisted list of "focused" host patterns
2. **Add API endpoints** for managing focus hosts (CRUD)
3. **Add a Focus UI panel** — list of focused hosts with add/remove, and a
   "Show only focused" toggle
4. **Integrate with the traffic list** — highlight focused-host rows (bold,
   colored badge) and optionally filter to show only focused traffic
5. **Add a quick-focus action** — right-click a traffic entry → "Focus this
   host"
6. **Persist focus state** across restarts

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/traffic/store.rs` | Add `focus_hosts` table: `CREATE TABLE IF NOT EXISTS focus_hosts (id TEXT PRIMARY KEY, pattern TEXT UNIQUE, created_at INTEGER)`; add CRUD methods: `add_focus_host()`, `remove_focus_host()`, `list_focus_hosts()`, `clear_focus_hosts()` |
| `crates/madhyamas-core/src/traffic/types.rs` | Add `FocusHost { id: String, pattern: String, created_at: DateTime<Utc> }` struct |
| `crates/madhyamas-core/src/lib.rs` | Export `FocusHost` |
| `crates/madhyamas-api/src/handlers.rs` | Add `get_focus_hosts`, `add_focus_host`, `remove_focus_host`, `clear_focus_hosts` handlers; add `host` parameter to `TrafficQuery` (currently missing) |
| `crates/madhyamas-api/src/routes.rs` | Add routes: `GET /focus`, `POST /focus`, `DELETE /focus/{id}`, `DELETE /focus` (clear all) |
| `crates/madhyamas-api/src/handlers.rs` | Add `focus_hosts` to `AppState` or read directly from `traffic_store` |
| `crates/madhyamas-cli/src/commands/` | Add `focus.rs` with `madhyamas focus list|add|remove|clear` subcommands |
| `crates/madhyamas-cli/src/commands/mod.rs` | Add `focus` module and command enum variant |
| `crates/madhyamas-mcp/src/tools/` | Add `focus.rs` with MCP tools |
| `crates/madhyamas-mcp/src/tools/registry.rs` | Register focus tools |
| `web/src/types/traffic.ts` | Add `FocusHost { id: string; pattern: string; created_at: string }` interface |
| `web/src/lib/api/traffic.ts` | Add `useFocusHosts()`, `useAddFocusHost()`, `useRemoveFocusHost()` hooks |
| `web/src/features/traffic/FocusPanel.tsx` | **New file** — Focus panel: list of focused hosts, add input, remove buttons, "Show only focused" toggle |
| `web/src/features/traffic/TrafficList.tsx:315-326` | Highlight rows whose `entry.request.host` matches a focus pattern (bold text, colored left border, or a star icon); dim non-focused rows when "focus mode" is on |
| `web/src/features/traffic/TrafficView.tsx` | Integrate `FocusPanel` (sidebar or toolbar dropdown); apply focus filter when "show only focused" is toggled |
| `web/src/features/traffic/TrafficList.tsx` | Add right-click context menu: "Focus this host" |

### How it should be done

**Focus vs Filter — key distinction:**
- **Filter** (existing): *hides* non-matching traffic entirely
- **Focus** (new): *highlights* matching traffic and *dims* (but doesn't hide)
  the rest. This is Charles's behavior — you can see focused hosts stand out
  while still seeing the full traffic context.

**Two modes:**
1. **Highlight mode** (default): All traffic visible; focused hosts are bold
   with a colored indicator (e.g., yellow left border or star icon)
2. **Focus-only mode** (toggle): Only focused-host traffic is shown (acts like
   a filter, but using the focus list as the pattern source)

**Pattern matching:** Reuse the existing glob/wildcard matching from
`MatchCondition` in `intercept/types.rs` (same as block list). Support:
- Exact host: `api.example.com`
- Wildcard subdomain: `*.example.com`
- Glob: `*api*`

**Persistence:** Store in SQLite (`focus_hosts` table) so focus hosts survive
restarts. This is simpler than a config file and allows API management.

**Key design decisions:**
- **Client-side highlighting** — for the highlight mode, the frontend checks
  each row's host against the focus list (loaded via `useFocusHosts()`). No
  backend filtering needed for highlighting. For focus-only mode, either
  filter client-side or add a `host` query param to the traffic API.
- **Add `host` to `TrafficQuery`** — this is a small fix that also benefits
  general filtering (the field exists in `TrafficFilter` but isn't exposed in
  the API query struct).
- **No new intercept pipeline entry** — focus is a UI/display concern, not a
  proxy interception concern. It doesn't affect traffic flow.

### How it would show up in the UI

- **TrafficView toolbar**: A "Focus" button (star/target icon) that opens the
  `FocusPanel` as a dropdown or sidebar
- **FocusPanel**:
  - Input field to add a host pattern (with autocomplete from existing traffic
    hosts)
  - List of focused hosts with remove (×) buttons
  - Toggle: "Show only focused traffic"
  - "Clear all" button
- **TrafficList**: Focused-host rows get a yellow left border + bold domain
  text + a small star icon. When "show only focused" is on, non-focused rows
  are hidden.
- **Right-click context menu**: Right-click any traffic row → "Focus this
  host" (adds `entry.request.host` to the focus list)
- **CLI**: `madhyamas focus list`, `madhyamas focus add "*.example.com"`,
  `madhyamas focus remove <id>`, `madhyamas focus clear`
- **MCP**: `madhyamas_list_focus_hosts`, `madhyamas_add_focus_host`,
  `madhyamas_remove_focus_host`
- **API**: `GET /api/focus`, `POST /api/focus { "pattern": "*.example.com" }`,
  `DELETE /api/focus/{id}`, `DELETE /api/focus`

### How it can be tested

1. **Add focus host test**: Add `api.example.com` via the API, verify it
  appears in `GET /api/focus`
2. **Highlight test**: With focus hosts set, verify traffic list highlights
  matching rows (bold, colored border)
3. **Wildcard test**: Add `*.example.com`, verify `api.example.com` and
  `www.example.com` are highlighted but `example.org` is not
4. **Focus-only mode test**: Toggle "show only focused", verify only matching
  rows are displayed
5. **Persistence test**: Add focus hosts, restart the proxy, verify they
  persist (SQLite)
6. **Right-click test**: Right-click a traffic row → "Focus this host" →
  verify the host is added
7. **Remove test**: Remove a focus host, verify highlighting updates
  immediately
8. **CLI test**: `madhyamas focus add "*.example.com"` then `madhyamas focus
  list`
9. **API host filter test**: `GET /api/traffic?host=example.com` returns only
  matching traffic (tests the new `host` query param)

### What needs to be documented

- Update `CLAUDE.md` — add Focus endpoints to the API table; add `madhyamas
  focus` to CLI examples
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Focus row from 🟡
  to ✅
- Update the madhyamas skill — add Focus workflow
- Create `docs/FOCUS.md` — guide on using focus hosts, pattern syntax,
  highlight vs filter-only modes

---

## 5. Mirror Tool (Save Responses to Disk)

### What exists now

There is **no mirror functionality** — no code saves response bodies to disk
as a site mirror.

| Aspect | Location | Current State |
|---|---|---|
| Mirror code | — | **None** — grep for "mirror", "save_to_disk", "save_response" finds only documentation references |
| Response body storage | `crates/madhyamas-core/src/traffic/store.rs:129-139` | Responses stored as BLOB in SQLite (`responses.body`); truncated at `max_body_size` (20 MB default) via `clamp_body()` (lines 311-322) |
| Response body access | `crates/madhyamas-core/src/traffic/store.rs:369-408` | `store_response()` stores body; `get_by_id()` retrieves full entry with body |
| Traffic event system | `crates/madhyamas-core/src/traffic/store.rs` | `emit_event(TrafficEvent::Updated(snapshot))` — could hook into this for mirror writes |
| Config | `crates/madhyamas-core/src/config.rs:18-147` | No mirror-related config fields |
| Download functionality | `crates/madhyamas-api/src/handlers.rs` | Only CA cert download (`GET /api/cert/ca`); no response body download |
| BRAINSTORM.md | `docs/BRAINSTORM.md` | Mirror mentioned as a concept only |

### What needs to be done

1. **Add mirror configuration** to `ProxyConfig` (output directory, enabled
   flag, host filter)
2. **Implement a mirror writer** that saves response bodies to disk following
  the URL path structure
3. **Hook the mirror writer** into the traffic storage pipeline (after
  response is captured)
4. **Add API endpoints** to start/stop mirroring and query mirror status
5. **Add web UI** toggle and configuration in the config dialog or a dedicated
  Mirror panel
6. **Handle file naming** — map URL paths to filesystem paths safely

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/config.rs` | Add `MirrorConfig { enabled: bool, output_dir: String, host_filter: Option<Vec<String>>, save_request_bodies: bool }` to `ProxyConfig` |
| `crates/madhyamas-core/src/mirror.rs` | **New file** — `MirrorWriter` struct with `write_response(host, path, response: &ResponseData) -> Result<PathBuf>`; maps URL to filesystem path; creates directories; writes body; optionally writes a metadata sidecar (headers, status, timestamp) |
| `crates/madhyamas-core/src/lib.rs` | Export `MirrorWriter`, `MirrorConfig` |
| `crates/madhyamas-core/src/traffic/store.rs` | In `store_response()` (line 369), after storing to DB, if mirror is enabled, call `mirror_writer.write_response(...)`; or emit a hook event that the engine handles |
| `crates/madhyamas-core/src/proxy/engine.rs` | Hold an `Option<Arc<MirrorWriter>>` field; pass response data to it after capture |
| `crates/madhyamas-api/src/handlers.rs` | Add `get_mirror_status`, `toggle_mirror`, `update_mirror_config` handlers; add `mirror_writer` to `AppState` |
| `crates/madhyamas-api/src/routes.rs` | Add routes: `GET /mirror`, `POST /mirror/toggle`, `PATCH /mirror/config` |
| `crates/madhyamas-api/src/handlers.rs` | Add `mirror_writer` field to `AppState` |
| `crates/madhyamas-cli/src/commands/` | Add `mirror.rs` with `madhyamas mirror status|start|stop|config` subcommands |
| `crates/madhyamas-mcp/src/tools/` | Add `mirror.rs` with MCP tools |
| `web/src/features/tools/MirrorPanel.tsx` | **New file** — Mirror panel: enable toggle, output directory input, host filter list, stats (files mirrored, total size) |
| `web/src/features/tools/ToolsSidebar.tsx` | Add Mirror to the tools navigation |
| `web/src/App.tsx` | Add `MirrorPanel` to tool view routing |

### How it should be done

**Filesystem path mapping:**

```
URL: https://api.example.com/v1/users/123?format=json
→ output_dir/api.example.com/v1/users/123/index.json
   (or: output_dir/api.example.com/v1/users/123?format=json)

URL: https://cdn.example.com/assets/img/logo.png
→ output_dir/cdn.example.com/assets/img/logo.png
```

**Rules:**
- Host becomes the top-level directory
- URL path maps directly to filesystem path
- If path ends with `/` or has no file extension, save as `index.html` (or
  `index.json` based on content-type)
- Query strings: append to filename with a safe separator (e.g.,
  `users_123_format=json.json`) or store in a metadata sidecar
- Sanitize path components (no `..`, no absolute paths, no null bytes)
- Create parent directories as needed

**Metadata sidecar:** For each mirrored response, optionally write a
`.meta.json` file alongside the body:
```json
{
  "url": "https://api.example.com/v1/users/123",
  "method": "GET",
  "status_code": 200,
  "headers": { "content-type": "application/json", ... },
  "timestamp": "2026-08-01T12:00:00Z",
  "duration_ms": 145
}
```

**MirrorWriter implementation:**

```rust
pub struct MirrorWriter {
    config: RwLock<MirrorConfig>,
    files_written: AtomicU64,
    bytes_written: AtomicU64,
}

impl MirrorWriter {
    pub fn write_response(
        &self,
        host: &str,
        path: &str,
        method: &str,
        response: &ResponseData,
        timestamp: DateTime<Utc>,
    ) -> Result<PathBuf> {
        let config = self.config.read();
        if !config.enabled { return Ok(PathBuf::new()); }

        // Check host filter
        if let Some(filter) = &config.host_filter {
            if !filter.iter().any(|p| matches_pattern(p, host)) {
                return Ok(PathBuf::new());
            }
        }

        let file_path = self.url_to_file_path(host, path, response)?;
        // Create parent dirs, write body, write metadata sidecar
        // Update counters
        Ok(file_path)
    }
}
```

**Key design decisions:**
- **Write asynchronously** — mirror writes should not block the proxy
  pipeline. Use `tokio::spawn` to write in the background.
- **Don't mirror passthrough traffic** — passthrough (SSL passthrough) entries
  have no captured body, so skip them.
- **Don't mirror if body is truncated** — if the body was clamped at
  `max_body_size`, note it in the metadata sidecar (incomplete mirror).
- **Overwrite by default** — each new response for the same URL overwrites the
  previous file (mirrors Charles behavior). An optional "versioned" mode could
  append timestamps.
- **Host filter** — allow mirroring only specific hosts (e.g., only
  `cdn.example.com`) to avoid filling disk with API responses.

### How it would show up in the UI

- **Tools sidebar**: New "Mirror" icon (folder/download icon)
- **MirrorPanel**:
  - Enable/disable toggle
  - Output directory input (default: `~/.madhyamas/mirror/`)
  - Host filter list (optional — empty = mirror all)
  - "Save request bodies too" checkbox
  - Stats: files mirrored, total disk usage, last mirrored URL
  - "Open mirror folder" button (opens in OS file browser)
- **Traffic detail**: A "Mirrored" badge on responses that have been saved to
  disk, with the file path
- **CLI**: `madhyamas mirror start --output-dir ./mirror --host-filter
  "*.example.com"`, `madhyamas mirror stop`, `madhyamas mirror status`
- **MCP**: `madhyamas_start_mirror`, `madhyamas_stop_mirror`,
  `madhyamas_mirror_status`
- **API**: `GET /api/mirror`, `POST /api/mirror/toggle`, `PATCH /api/mirror/config`

### How it can be tested

1. **Basic mirror test**: Enable mirror, make a request to `example.com/page`,
  verify `output_dir/example.com/page/index.html` exists with correct content
2. **Path mapping test**: Verify various URL paths map correctly (trailing
  slashes, query strings, no extension)
3. **Host filter test**: Set host filter to `*.example.com`, make requests to
  `example.com` and `other.com`, verify only `example.com` is mirrored
4. **Overwrite test**: Make the same request twice, verify the file is
  overwritten (not duplicated)
5. **Passthrough test**: Verify SSL passthrough traffic is not mirrored (no
  body to save)
6. **Large body test**: Make a request with a >20MB response, verify the file
  is truncated and the metadata sidecar notes "truncated"
7. **Disable test**: Disable mirror, make requests, verify no new files are
  written
8. **Path safety test**: Make a request with `../` in the path, verify it
  doesn't escape the output directory
9. **CLI test**: `madhyamas mirror start` then `madhyamas mirror status`
10. **Disk usage test**: Mirror 1000 requests, verify stats show correct file
    count and total size

### What needs to be documented

- Update `CLAUDE.md` — add Mirror config fields, API endpoints, CLI commands
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Mirror row from ❌
  to ✅
- Update the madhyamas skill — add Mirror workflow
- Create `docs/MIRROR.md` — guide on mirroring, path mapping rules, host
  filtering, metadata sidecar format, disk usage considerations

---

## 6. Auto Save (Periodic Session Save)

### What exists now

There is **no auto-save mechanism**. Sessions are persisted to SQLite in real
time (every request/response is stored immediately), but there's no periodic
"snapshot" or export for backup purposes.

| Aspect | Location | Current State |
|---|---|---|
| Session manager | `crates/madhyamas-core/src/session.rs:75-77` | `SessionManager { traffic_store: Arc<TrafficStore> }` — no auto-save logic |
| Session export | `crates/madhyamas-core/src/session.rs:116-137` | `export_session(session_id)` → `SessionExport { version, exported_at, session, entries }` — manual only |
| Session import | `crates/madhyamas-core/src/session.rs:139-175` | `import_session(export)` — creates new session, inserts entries |
| Traffic store | `crates/madhyamas-core/src/traffic/store.rs:14-24` | Real-time SQLite storage — every request/response written immediately via `store_request()` / `store_response()` |
| Config | `crates/madhyamas-core/src/config.rs:18-147` | No auto-save config fields |
| Background tasks | `crates/madhyamas-core/src/proxy/engine.rs` | No periodic task infrastructure beyond the proxy accept loop |
| Web UI | `web/src/features/config/ConfigDialog.tsx:563-664` | Capture tab is client-side only (localStorage); no auto-save UI |

**Important clarification:** Since traffic is stored in SQLite in real time,
"auto save" in the Charles sense (periodic session snapshot to avoid data
loss) is less critical — data isn't lost on crash. However, auto-save is still
valuable for:
- **Periodic HAR/session export** to a backup directory (disaster recovery)
- **Automatic session rotation** — start a new session every N minutes or
  after M requests, archiving the old one
- **Automatic cleanup** — prune old sessions to prevent unbounded DB growth

### What needs to be done

1. **Add auto-save config** to `ProxyConfig` (enabled, interval, export format,
  output directory, max backup count)
2. **Implement a periodic background task** using `tokio::time::interval` that
  exports the current session
3. **Add automatic session rotation** (optional) — start a new session after
  N requests or M minutes
4. **Add automatic cleanup** of old backup files (keep last N)
5. **Add API endpoints** to query/configure auto-save
6. **Add web UI** controls in the config dialog

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/config.rs` | Add `AutoSaveConfig { enabled: bool, interval_seconds: u64, export_format: String, output_dir: String, max_backups: usize, rotate_after_requests: Option<usize>, rotate_after_minutes: Option<u64> }` to `ProxyConfig` |
| `crates/madhyamas-core/src/auto_save.rs` | **New file** — `AutoSaveManager` struct with `start()` (spawns a `tokio::spawn` task with `tokio::time::interval`), `stop()`, and the periodic save logic: export current session to `output_dir/session-{name}-{timestamp}.har`, prune old backups beyond `max_backups` |
| `crates/madhyamas-core/src/lib.rs` | Export `AutoSaveManager`, `AutoSaveConfig` |
| `crates/madhyamas-core/src/proxy/engine.rs` | Hold `Option<Arc<AutoSaveManager>>` field; start it in `start()` if enabled; stop it on shutdown |
| `crates/madhyamas-api/src/handlers.rs` | Add `get_autosave_config`, `update_autosave_config` handlers; add `autosave_manager` to `AppState` |
| `crates/madhyamas-api/src/routes.rs` | Add routes: `GET /autosave`, `PATCH /autosave` |
| `crates/madhyamas-cli/src/commands/` | Add autosave config to the config command |
| `web/src/features/config/ConfigDialog.tsx` | Add "Auto Save" section in the Capture or General tab: enable toggle, interval input, export format dropdown (HAR/Session), output directory, max backups |
| `web/src/lib/api/` | Add `useAutoSaveConfig()`, `useUpdateAutoSaveConfig()` hooks |

### How it should be done

**AutoSaveManager implementation:**

```rust
pub struct AutoSaveManager {
    config: RwLock<AutoSaveConfig>,
    traffic_store: Arc<TrafficStore>,
    session_manager: Arc<SessionManager>,
    stop_token: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl AutoSaveManager {
    pub fn start(self: Arc<Self>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        *self.stop_token.lock() = Some(tx);

        let manager = self.clone();
        tokio::spawn(async move {
            let config = manager.config.read().clone();
            if !config.enabled { return; }

            let mut interval = tokio::time::interval(
                Duration::from_secs(config.interval_seconds)
            );
            interval.tick().await; // skip immediate first tick

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = manager.save_snapshot().await {
                            error!("Auto-save failed: {}", e);
                        }
                    }
                    _ = rx => {
                        info!("Auto-save manager stopped");
                        break;
                    }
                }
            }
        });
    }

    async fn save_snapshot(&self) -> Result<()> {
        let config = self.config.read().clone();
        let session_id = self.traffic_store.current_session_id();

        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let filename = format!("session-{}.{}", timestamp, config.export_format);
        let path = Path::new(&config.output_dir).join(filename);

        match config.export_format.as_str() {
            "har" => {
                let har = self.traffic_store.export_har(&session_id)?;
                fs::write(&path, serde_json::to_vec_pretty(&har)?)?;
            }
            "session" => {
                let export = self.session_manager.export_session(&session_id)?;
                fs::write(&path, serde_json::to_vec_pretty(&export)?)?;
            }
            _ => return Err(Error::Config("Unknown export format".into())),
        }

        // Prune old backups
        self.prune_backups(&config)?;
        Ok(())
    }
}
```

**Key design decisions:**
- **Real-time SQLite is the primary store** — auto-save is a *backup*
  mechanism, not the primary persistence. This is different from Charles,
  which keeps traffic in memory and uses auto-save to prevent data loss.
- **Export format** — HAR (for interoperability) or Session (for
  Madhyamas-native restore). Default: HAR.
- **Backup pruning** — keep only the last `max_backups` files (default 10) to
  prevent unbounded disk usage. Delete oldest files first.
- **Session rotation** (optional) — if `rotate_after_requests` or
  `rotate_after_minutes` is set, the auto-save task also creates a new session
  and archives the old one. This prevents any single session from growing too
  large.
- **Graceful shutdown** — use a `oneshot` channel to stop the background task
  cleanly when the proxy shuts down.

### How it would show up in the UI

- **Config dialog**: New "Auto Save" section (in Capture tab or a new tab):
  - Enable toggle
  - Save every: [number] [seconds/minutes] (default: 300 seconds / 5 minutes)
  - Export format: [HAR | Session] dropdown
  - Output directory: text input (default: `~/.madhyamas/backups/`)
  - Max backups: number input (default: 10)
  - Session rotation: optional — "Rotate after [N] requests" or "Rotate after
    [N] minutes"
- **Status bar**: Show "Auto-save: ON (every 5m)" or "Auto-save: OFF"
- **CLI**: `madhyamas config get` shows auto-save config; `madhyamas config set
  autosave.enabled true`
- **API**: `GET /api/autosave`, `PATCH /api/autosave`

### How it can be tested

1. **Basic auto-save test**: Enable auto-save with 5-second interval, capture
  traffic, wait 15 seconds, verify 3 backup files exist in the output
  directory
2. **HAR format test**: Verify backup files are valid HAR (parse with a HAR
  validator)
3. **Session format test**: Verify session backup files can be imported via
  `POST /api/sessions/import`
4. **Pruning test**: Set `max_backups = 3`, wait for 5 saves, verify only the
  3 most recent files remain
5. **Disable test**: Disable auto-save, verify no new backup files are created
6. **Rotation test**: Set `rotate_after_requests = 10`, capture 11 requests,
  verify a new session is created and the old one is archived
7. **Shutdown test**: Start auto-save, shut down the proxy, verify the
  background task stops cleanly (no orphaned process)
8. **Disk full test**: Fill the disk, verify auto-save fails gracefully with a
  log error (doesn't crash the proxy)
9. **API test**: `PATCH /api/autosave { "enabled": true, "interval_seconds": 60 }`
10. **UI test**: Configure auto-save via the config dialog, verify it takes
    effect

### What needs to be documented

- Update `CLAUDE.md` — add Auto Save config fields, API endpoints
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Auto Save row from
  ❌ to ✅
- Update the madhyamas skill — add Auto Save workflow
- Create `docs/AUTO_SAVE.md` — guide on auto-save, backup formats, pruning,
  session rotation, disaster recovery

---

## 7. Recording Size Limits

### What exists now

Body size is limited (20 MB per body), and `max_requests` is defined in config
(10,000) but **not enforced**. A full `MemoryManager` with GC logic exists but
is **not integrated** into the traffic store.

| Aspect | Location | Current State |
|---|---|---|
| Body size limit | `crates/madhyamas-core/src/traffic/store.rs:22` | `max_body_size: AtomicUsize` — default 20 MB (config.rs:402); enforced via `clamp_body()` (lines 311-322) |
| Max requests config | `crates/madhyamas-core/src/config.rs:46` | `max_requests: usize` — default 10,000 (line 400) — **defined but NOT enforced** in TrafficStore |
| Entry count | `crates/madhyamas-core/src/traffic/store.rs` | No count tracking, no pruning when limit exceeded |
| MemoryManager (unused) | `crates/madhyamas-core/src/performance/memory.rs:11-26` | Full GC implementation: `max_memory_bytes` (500 MB), `max_entries` (100,000), `GarbageCollectionConfig` (min_interval 60s, target_usage 70%, preserve_recent 300s), `check_memory()`, `is_under_pressure()` — **NOT integrated** with TrafficStore |
| Capture toggle | `crates/madhyamas-core/src/traffic/store.rs:290-297` | `capture_enabled: AtomicBool` — on/off only, no granularity |
| Capture API | `crates/madhyamas-api/src/handlers.rs:342-358` | `GET /capture`, `POST /capture/toggle` — simple toggle |
| Config API | `crates/madhyamas-api/src/handlers.rs:292-356` | `GET/PATCH /config` — includes `max_body_size` but not `max_requests` enforcement |
| Web UI capture | `web/src/features/config/ConfigDialog.tsx:563-664` | Capture tab is **client-side only** (localStorage) — `max_body_size_kb` slider, `capture_request_bodies`, `capture_response_bodies`, `ignored_domains` — **not sent to backend** |
| Web UI header | `web/src/features/shell/AppHeader.tsx:108-135` | Recording/Passthrough toggle button |

### What needs to be done

1. **Enforce `max_requests`** in `TrafficStore` — prune oldest entries when
  the limit is exceeded
2. **Integrate the existing `MemoryManager`** with `TrafficStore` for
  pressure-based cleanup (or implement a simpler entry-count-based approach)
3. **Add total recording size limit** (max DB size or max total body bytes)
4. **Add config fields** for all limits and expose them via the API
5. **Connect the web UI capture tab** to the backend (currently localStorage
  only)
6. **Add a "recording quota" indicator** in the UI showing current usage vs
  limits
7. **Add automatic cleanup** — prune old entries when limits are approached

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/traffic/store.rs` | Add `max_entries: AtomicUsize` field (from config); in `store_request()`, after insert, check entry count and prune oldest if over limit; add `prune_oldest(count)` method that deletes the oldest N entries from `requests` and `responses` tables; add `get_entry_count()` method |
| `crates/madhyamas-core/src/config.rs` | Add `max_total_size_mb: Option<usize>` (total recording size limit); ensure `max_requests` is passed to TrafficStore on init |
| `crates/madhyamas-core/src/traffic/store.rs` | Add `max_total_size_bytes: AtomicUsize` field; add periodic size check (or check on each insert) that sums `length(body)` and prunes if over limit |
| `crates/madhyamas-core/src/performance/memory.rs` | (Optional) Wire `MemoryManager` into TrafficStore: call `entry_added()` in `store_request()`, `entry_removed()` in `delete_traffic()`; trigger GC when `is_under_pressure()` returns true |
| `crates/madhyamas-api/src/handlers.rs` | Add `max_requests`, `max_total_size_mb` to `GET /api/config` response and `PATCH /api/config` request; add `GET /api/capture/stats` endpoint returning current entry count, total size, limits, usage percentage |
| `crates/madhyamas-api/src/routes.rs` | Add `GET /capture/stats` route |
| `crates/madhyamas-cli/src/commands/` | Add capture stats to the config or capture command |
| `web/src/features/config/ConfigDialog.tsx:563-664` | Replace localStorage-only capture tab with API-backed settings: wire `max_body_size_kb`, `max_requests`, `max_total_size_mb`, `capture_request_bodies`, `capture_response_bodies`, `ignored_domains` to `PATCH /api/config` |
| `web/src/features/shell/AppHeader.tsx` | Add a recording quota indicator: "12,345 / 10,000 entries (123%)" with a progress bar; turn red when over limit |
| `web/src/lib/api/` | Add `useCaptureStats()` hook for `GET /api/capture/stats` |

### How it should be done

**Entry count enforcement (simplest approach):**

```rust
// In TrafficStore::store_request(), after the INSERT:
let count = self.get_entry_count()?;
let max = self.max_entries.load(Ordering::Relaxed);
if count > max {
    let to_prune = count - max;
    self.prune_oldest(to_prune)?;
}

fn prune_oldest(&self, count: usize) -> crate::Result<()> {
    let conn = self.conn.lock();
    // Delete the oldest `count` entries
    conn.execute(
        "DELETE FROM responses WHERE request_id IN (
            SELECT id FROM requests
            ORDER BY timestamp ASC
            LIMIT ?1
        )",
        params![count as i64],
    ).map_err(Error::Database)?;

    conn.execute(
        "DELETE FROM requests WHERE id IN (
            SELECT id FROM requests
            ORDER BY timestamp ASC
            LIMIT ?1
        )",
        params![count as i64],
    ).map_err(Error::Database)?;

    // Emit pruned events for WebSocket clients
    Ok(())
}
```

**Total size enforcement:**

```rust
fn check_total_size(&self) -> crate::Result<()> {
    let max = self.max_total_size_bytes.load(Ordering::Relaxed);
    if max == 0 { return Ok(()); } // 0 = unlimited

    let conn = self.conn.lock();
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(length(body)), 0) FROM responses",
        [],
        |row| row.get(0),
    ).map_err(Error::Database)?;

    if (total as usize) > max {
        // Prune oldest entries until under limit
        let overage = (total as usize) - max;
        // Estimate: delete oldest entries until we've freed `overage` bytes
        conn.execute(
            "DELETE FROM responses WHERE request_id IN (
                SELECT r.id FROM requests r
                JOIN responses p ON p.request_id = r.id
                ORDER BY r.timestamp ASC
                LIMIT (SELECT COUNT(*) FROM responses) / 10
            )",
            [],
        )?;
        // Also delete the corresponding requests
        // ...
    }
    Ok(())
}
```

**Key design decisions:**
- **Pruning strategy** — delete oldest entries first (FIFO). This preserves
  recent traffic, which is most likely to be relevant.
- **Pruning frequency** — check on every `store_request()` call, but only
  prune when the limit is exceeded. For efficiency, check entry count (cheap
  `COUNT(*)` query) more frequently than total size (expensive `SUM(length())`
  query). Consider checking total size every N inserts (e.g., every 100).
- **Granular body capture** — the web UI capture tab already has
  `capture_request_bodies` and `capture_response_bodies` toggles (in
  localStorage). Wire these to the backend so the proxy can skip storing bodies
  entirely (store headers only) — this dramatically reduces DB size.
- **Ignored domains** — the web UI has an `ignored_domains` list (localStorage).
  Wire this to the backend so the proxy doesn't store traffic for specified
  domains (similar to Charles's "Ignore-list for recording").
- **MemoryManager integration** (optional, more sophisticated) — the existing
  `MemoryManager` in `performance/memory.rs` has a full GC implementation. It
  could be wired in for pressure-based cleanup. However, since traffic is in
  SQLite (not in-memory), the "memory pressure" concept is less relevant —
  disk size is the real constraint. The simpler entry-count + total-size
  approach is recommended first.

### How it would show up in the UI

- **Config dialog (Capture tab)** — now backed by the API:
  - Max entries: number input (default 10,000)
  - Max total size: number input in MB (default 500 MB, 0 = unlimited)
  - Max body size: slider (already exists, now saved to backend)
  - Capture request bodies: checkbox (now saved to backend)
  - Capture response bodies: checkbox (now saved to backend)
  - Ignored domains: textarea (now saved to backend)
- **AppHeader**: Recording quota indicator next to the Recording/Passthrough
  toggle:
  - "9,876 / 10,000 entries" with a green progress bar
  - Turns yellow at 80%, red at 100%
  - Tooltip shows total size: "245 MB / 500 MB"
- **CLI**: `madhyamas config get` shows all limits; `madhyamas capture stats`
  shows current usage
- **API**: `GET /api/capture/stats` → `{ "entry_count": 9876, "max_entries": 10000, "total_size_bytes": 257000000, "max_total_size_bytes": 524288000, "usage_percent": 98.76 }`

### How it can be tested

1. **Entry limit test**: Set `max_entries = 10`, capture 15 requests, verify
  only the 10 most recent remain
2. **Size limit test**: Set `max_total_size_mb = 1`, capture responses with
  large bodies, verify total stays under 1 MB
3. **Pruning correctness test**: Verify pruned entries' responses are also
  deleted (no orphaned response rows)
4. **WebSocket notification test**: Verify pruning emits events so the web UI
  updates (entries disappear from the list)
5. **No-body capture test**: Disable `capture_response_bodies`, make requests,
  verify responses are stored with headers but no body
6. **Ignored domains test**: Add `*.example.com` to ignored domains, make
  requests to `example.com`, verify they're not stored
7. **Quota indicator test**: Capture traffic until near the limit, verify the
  header indicator turns yellow then red
8. **API test**: `GET /api/capture/stats` returns correct counts and sizes
9. **Config persistence test**: Set limits via `PATCH /api/config`, restart,
  verify limits persist
10. **UI wiring test**: Change settings in the config dialog, verify they're
    sent to the backend (not just localStorage)

### What needs to be documented

- Update `CLAUDE.md` — add recording limit config fields, `GET /capture/stats`
  endpoint
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change Recording size
  limits row from ❌ to ✅; change Ignore-list row from 🟡 to ✅
- Update the madhyamas skill — add recording limits and ignored domains
  workflow
- Create `docs/RECORDING_LIMITS.md` — guide on entry/size limits, body capture
  toggles, ignored domains, pruning behavior

---

## 8. HAR Import

### What exists now

HAR **export** exists but HAR **import** (for traffic entries) does not. HAR
import exists only for **mocks** (converting HAR entries to mock rules).

| Aspect | Location | Current State |
|---|---|---|
| HAR export (traffic) | `crates/madhyamas-core/src/traffic/store.rs:739-783` | `export_har(session_id)` → HAR 1.2 JSON with `log.entries[]` (method, url, headers, status, bodySize, content.size, time) |
| HAR export API | `crates/madhyamas-api/src/handlers.rs:218-232` | `export_har(State)` → `GET /api/export/har` |
| HAR export UI | `web/src/features/traffic/TrafficView.tsx:124-145` | "Export HAR" dropdown (Selected / All); downloads `.har` file |
| Single-entry HAR | `web/src/features/traffic/TrafficDetail.tsx:917-972` | `generateHAR(entry)` — client-side single-entry HAR generation |
| HAR import (mocks) | `crates/madhyamas-core/src/intercept/mock.rs:1148-1216` | `import_from_har(har_json)` — parses HAR entries, creates `MockRule` objects (not traffic entries) |
| Mock import API | `crates/madhyamas-api/src/intercept_handlers.rs:577-617` | `import_mocks(State, Json<ImportMocksRequest { format, data }>)` — supports "har", "openapi", "postman" formats |
| Mock import UI | `web/src/features/tools/MocksPanel.tsx:468-475` | File input accepting `.json,.har` |
| Session import | `crates/madhyamas-core/src/session.rs:139-175` | `import_session(SessionExport)` — creates new session, inserts entries via `store_request()` + `store_response()` — **model for HAR import** |
| Session import API | `crates/madhyamas-api/src/handlers.rs:787-801` | `import_session(State, Json<SessionExport>)` → `POST /api/sessions/import` |
| Session import UI | `web/src/features/sessions/SessionsPanel.tsx:100-117` | File input accepting `.json`; reads file, parses JSON, calls `importSession.mutateAsync(data)` |
| Traffic store insert | `crates/madhyamas-core/src/traffic/store.rs:324-367` | `store_request(&TrafficEntry)` — inserts into `requests` table |
| Traffic store insert | `crates/madhyamas-core/src/traffic/store.rs:369-408` | `store_response(request_id, &ResponseData)` — inserts into `responses` table |
| HAR import (traffic) | — | **Does not exist** |

### What needs to be done

1. **Add a HAR-to-TrafficEntry converter** in the traffic store (or a new
  module)
2. **Add an API endpoint** `POST /api/traffic/import/har` that accepts HAR JSON
  and creates a new session with the imported entries
3. **Add a web UI** import button in the traffic view (alongside the export
  button)
4. **Handle HAR format variations** — HAR 1.1 vs 1.2, missing fields, base64
  encoded bodies
5. **Validate the HAR** before import (check `log.version`, `log.entries[]`
  structure)

### Where it needs to be done

| File | Change |
|---|---|
| `crates/madhyamas-core/src/traffic/store.rs` | Add `pub fn import_har(&self, har: &serde_json::Value, session_name: Option<&str>) -> crate::Result<ImportResult>` — parses HAR JSON, creates a new session, converts each `log.entries[]` to `TrafficEntry`, calls `store_request()` + `store_response()`; return `ImportResult { session_id, imported_count, skipped_count, errors }` |
| `crates/madhyamas-core/src/traffic/types.rs` | Add `ImportResult { session_id: String, imported_count: usize, skipped_count: usize, errors: Vec<String> }` struct |
| `crates/madhyamas-core/src/lib.rs` | Export `ImportResult` |
| `crates/madhyamas-api/src/handlers.rs` | Add `import_traffic_har(State, Json<HarImportRequest>)` handler — accepts `{ "har": <HAR JSON>, "session_name": Option<String> }`; calls `traffic_store.import_har()`; returns `ImportResult` |
| `crates/madhyamas-api/src/routes.rs` | Add `POST /traffic/import/har` route |
| `crates/madhyamas-api/src/validation.rs` | Add validation for HAR import (check `log` exists, `entries` is an array) |
| `crates/madhyamas-cli/src/commands/` | Add `madhyamas traffic import-har <file>` subcommand |
| `crates/madhyamas-mcp/src/tools/` | Add `madhyamas_import_har` MCP tool |
| `web/src/features/traffic/TrafficView.tsx` | Add "Import HAR" button next to "Export HAR"; file input accepting `.har,.json`; reads file, parses JSON, calls `POST /api/traffic/import/har` |
| `web/src/lib/api/traffic.ts` | Add `useImportHar()` mutation hook |

### How it should be done

**HAR-to-TrafficEntry conversion:**

```rust
pub fn import_har(
    &self,
    har: &serde_json::Value,
    session_name: Option<&str>,
) -> crate::Result<ImportResult> {
    let log = har.get("log")
        .ok_or_else(|| Error::Config("Invalid HAR: missing 'log' field".into()))?;
    let entries = log.get("entries")
        .and_then(|e| e.as_array())
        .ok_or_else(|| Error::Config("Invalid HAR: missing 'log.entries' array".into()))?;

    // Create a new session for the imported traffic
    let session_name = session_name.unwrap_or("Imported HAR");
    let session_id = /* create new session */;

    let mut imported = 0;
    let mut skipped = 0;
    let mut errors = Vec::new();

    for (i, entry) in entries.iter().enumerate() {
        match self.convert_har_entry(entry, &session_id) {
            Ok(entry) => {
                self.store_request(&entry)?;
                if let Some(resp) = &entry.response {
                    self.store_response(&entry.id, resp)?;
                }
                imported += 1;
            }
            Err(e) => {
                skipped += 1;
                errors.push(format!("Entry {}: {}", i, e));
            }
        }
    }

    Ok(ImportResult { session_id, imported_count: imported, skipped_count: skipped, errors })
}

fn convert_har_entry(
    &self,
    har_entry: &serde_json::Value,
    session_id: &str,
) -> crate::Result<TrafficEntry> {
    let req = har_entry.get("request")
        .ok_or_else(|| Error::Config("Missing 'request' field".into()))?;

    let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("GET");

    // Parse URL into host and path
    let parsed = url::Url::parse(url).ok();
    let host = parsed.as_ref().map(|u| u.host_str().unwrap_or("")).unwrap_or("");
    let path = parsed.as_ref().map(|u| u.path()).unwrap_or("/");

    // Convert headers
    let headers = req.get("headers")
        .and_then(|h| h.as_array())
        .map(|arr| {
            arr.iter().filter_map(|h| {
                let name = h.get("name")?.as_str()?.to_string();
                let value = h.get("value")?.as_str()?.to_string();
                Some((name, value))
            }).collect::<HashMap<_, _>>()
        }).unwrap_or_default();

    // Convert body (if present)
    let body = req.get("postData")
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.as_bytes().to_vec());

    // Convert response (if present)
    let response = har_entry.get("response").map(|resp| {
        let status_code = resp.get("status").and_then(|s| s.as_u64()).unwrap_or(0) as u16;
        let resp_headers = /* similar to request headers */;
        let resp_body = resp.get("content")
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.as_bytes().to_vec());
        let duration_ms = har_entry.get("time").and_then(|t| t.as_u64()).unwrap_or(0);
        ResponseData { status_code, headers: resp_headers, body: resp_body, duration_ms, ..Default::default() }
    });

    // Parse timestamp
    let timestamp = har_entry.get("startedDateTime")
        .and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    Ok(TrafficEntry {
        id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        request: RequestData { method: method.parse().unwrap_or(HttpMethod::Get), url: url.to_string(), host: host.to_string(), path: path.to_string(), headers, body, ..Default::default() },
        response,
        timestamp,
        ..Default::default()
    })
}
```

**Key design decisions:**
- **Create a new session** — imported HAR traffic goes into a new session
  (named "Imported HAR" or user-provided name), not the current session. This
  keeps imported traffic separate from live capture and matches the session
  import behavior.
- **Switch to the new session** — after import, optionally switch the active
  session to the imported one so the user sees the imported traffic
  immediately.
- **Handle base64 bodies** — HAR spec allows `content.encoding: "base64"`.
  Decode base64 bodies before storing.
- **Skip invalid entries** — don't fail the entire import for one bad entry;
  collect errors and report them in `ImportResult.errors`.
- **Reuse mock HAR parsing** — the mock import (`mock.rs:1148-1216`) already
  parses HAR entries. The conversion logic is similar but the output is
  different (MockRule vs TrafficEntry). Some parsing helpers could be shared.

### How it would show up in the UI

- **TrafficView toolbar**: "Import HAR" button (upload icon) next to "Export
  HAR" (download icon)
- **Import dialog**: File picker accepting `.har` and `.json` files; optional
  session name input; "Import" button
- **Import result**: Toast notification: "Imported 145 entries (2 skipped)"
  with a link to switch to the new session
- **CLI**: `madhyamas traffic import-har ./traffic.har --session-name "Bug Reproduction"`
- **MCP**: `madhyamas_import_har` with `{ "har": <HAR JSON>, "session_name": "..." }`
- **API**: `POST /api/traffic/import/har` with `{ "har": {...}, "session_name": "..." }` → `{ "session_id": "...", "imported_count": 145, "skipped_count": 2, "errors": [...] }`

### How it can be tested

1. **Round-trip test**: Export a session as HAR, import it back, verify the
  imported traffic matches (same URLs, methods, status codes, headers)
2. **External HAR test**: Import a HAR file from Chrome DevTools or Charles,
  verify entries are created correctly
3. **Base64 body test**: Import a HAR with base64-encoded response bodies,
  verify bodies are decoded correctly
4. **Missing fields test**: Import a HAR with entries missing `response` or
  `postData`, verify they're handled gracefully (response = None, body = None)
5. **Invalid HAR test**: Import a non-HAR JSON file, verify a clear error is
  returned
6. **Partial failure test**: Import a HAR with one malformed entry, verify
  other entries are imported and the error is reported
7. **Session creation test**: Verify a new session is created with the correct
  name
8. **Large HAR test**: Import a HAR with 10,000 entries, verify performance is
  acceptable (no timeout)
9. **CLI test**: `madhyamas traffic import-har ./test.har`
10. **UI test**: Click "Import HAR", select a file, verify toast and session
    switch

### What needs to be documented

- Update `CLAUDE.md` — add `POST /traffic/import/har` to the API endpoints
  table; add `madhyamas traffic import-har` to CLI examples
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change HAR import row
  from ❌ to ✅
- Update the madhyamas skill — add HAR import workflow
- Create or update `docs/IMPORT_EXPORT.md` — document HAR import/export,
  session import/export, supported HAR versions, limitations

---

## 9. zstd Decompression

### What exists now

The proxy stores **raw compressed bodies** (Content-Encoding header preserved)
and has a `decompress_body()` function that supports gzip, deflate, and
brotli — but **not zstd**. The function is also marked `#[allow(dead_code)]`
(not currently called in the main pipeline).

| Aspect | Location | Current State |
|---|---|---|
| Decompression function | `crates/madhyamas-core/src/proxy/pipeline.rs:1053-1127` | `decompress_body(content_encoding, body, out_headers)` — supports `"gzip"`, `"x-gzip"`, `"deflate"`, `"br"` (brotli); **no `"zstd"` case**; marked `#[allow(dead_code)]` |
| Decompression deps | `Cargo.toml:97-98` | `flate2` (gzip/deflate), `brotli` (brotli) — **no zstd crate** |
| reqwest features | `Cargo.toml:94` | `reqwest = { version = "0.13", features = ["json", "gzip", "deflate", "brotli", "socks"] }` — **no `"zstd"` feature** |
| Auto-decompression | `crates/madhyamas-core/src/proxy/pipeline.rs:758` | Explicitly disabled (`No auto-decompression`) — proxy stores raw compressed bodies with Content-Encoding header intact |
| zstd in Cargo.lock | — | **Not present** — no zstd crate in the dependency tree |
| Web UI body display | `web/src/features/traffic/TrafficDetail.tsx` | Displays body as text; if compressed, shows raw bytes or attempts client-side decompression (limited) |

### What needs to be done

1. **Add the `zstd` crate** as a dependency
2. **Add a `"zstd"` case** to `decompress_body()` in `pipeline.rs`
3. **Add `"zstd"` to reqwest features** (for upstream client compatibility)
4. **Wire `decompress_body()` into the storage pipeline** (currently dead
  code) so zstd-compressed bodies can be displayed in the UI
5. **Add web UI support** for displaying zstd-compressed bodies (decompress
  client-side or rely on backend decompression)

### Where it needs to be done

| File | Change |
|---|---|
| `Cargo.toml` (workspace) | Add `zstd = "0.13"` to `[workspace.dependencies]`; add `"zstd"` to reqwest features: `features = ["json", "gzip", "deflate", "brotli", "zstd", "socks"]` |
| `crates/madhyamas-core/Cargo.toml` | Add `zstd.workspace = true` and `flate2.workspace = true` (if not already direct deps) |
| `crates/madhyamas-core/src/proxy/pipeline.rs:1053-1127` | Add `"zstd"` match arm to `decompress_body()`; remove `#[allow(dead_code)]` if wiring it in |
| `crates/madhyamas-core/src/proxy/pipeline.rs` | (If wiring in) Call `decompress_body()` when storing response bodies, or when the web UI requests a decompressed view |
| `crates/madhyamas-api/src/handlers.rs` | (Optional) Add a `?decompressed=true` query param to `GET /api/traffic/{id}` that returns the decompressed body |
| `web/src/features/traffic/TrafficDetail.tsx` | Add zstd to the list of encodings the body viewer can handle; if backend decompression is available, request decompressed body |

### How it should be done

**zstd decompression in `decompress_body()`:**

```rust
"zstd" => {
    let mut decoder = match zstd::stream::read::Decoder::new(&body[..]) {
        Ok(d) => d,
        Err(e) => {
            debug!("Failed to create zstd decoder: {}", e);
            return None;
        }
    };
    let mut out = Vec::with_capacity(body.len() * 4);
    use std::io::Read;
    match decoder.read_to_end(&mut out) {
        Ok(_) => Some(out),
        Err(e) => {
            debug!("Failed to decompress zstd body: {}", e);
            None
        }
    }
}
```

**Wiring `decompress_body()` into the pipeline:**

The function is currently dead code. There are two approaches:

1. **Decompress on store** (simpler, changes stored data):
   - In `store_response()`, call `decompress_body()` before storing
   - Remove the `Content-Encoding` header and update `Content-Length`
   - Pro: Web UI always shows decompressed bodies; simpler frontend
   - Con: Loses the original compressed representation; can't show
     "compressed size vs decompressed size"

2. **Decompress on demand** (preserves original, more flexible):
   - Keep storing raw compressed bodies (current behavior)
   - Add a `?decompressed=true` query param to `GET /api/traffic/{id}`
   - The handler calls `decompress_body()` on the fly before returning
   - Pro: Preserves original data; web UI can toggle between compressed/
     decompressed views
   - Con: More complex; decompression happens on every request

**Recommended: Approach 2 (decompress on demand)** — it preserves the original
compressed body (useful for debugging compression issues) and gives the user
control. The web UI's body viewer already has encoding awareness; add a
"Decompressed" toggle.

**Key design decisions:**
- **zstd crate version** — use `zstd = "0.13"` (stable, widely used Rust
  bindings to the C zstd library). For pure-Rust, `zstd-safe` could be used,
  but the C binding is faster and more battle-tested.
- **reqwest `"zstd"` feature** — adding this lets reqwest auto-decompress
  zstd responses from upstream servers. However, since we explicitly disable
  auto-decompression (pipeline.rs:758), this feature is mainly for
  completeness. The proxy will still store raw zstd bodies and decompress on
  demand.
- **Error handling** — if zstd decompression fails (corrupt data, unsupported
  version), fall back to returning the raw body (same as the existing
  gzip/deflate/brotli behavior).

### How it would show up in the UI

- **Traffic detail body viewer**: A "Decompressed" toggle (or auto-detect
  based on Content-Encoding). When the body is zstd-compressed, clicking
  "Decompressed" fetches `GET /api/traffic/{id}?decompressed=true` and shows
  the plaintext body.
- **Content-Encoding display**: The response headers tab shows
  `Content-Encoding: zstd` — the UI should recognize this and show a
  "zstd" badge next to the body viewer.
- **No CLI change** — decompression is transparent
- **API**: `GET /api/traffic/{id}?decompressed=true` returns the body with
  zstd/gzip/deflate/brotli decompressed and Content-Encoding/Content-Length
  headers adjusted

### How it can be tested

1. **Unit test**: Create a zstd-compressed body, call `decompress_body()`,
  verify output matches the original uncompressed data
2. **Round-trip test**: Compress a body with zstd, decompress, verify
  equality
3. **Integration test**: Make a request to a server that returns
  `Content-Encoding: zstd` (e.g., using `curl --compressed` or a test server),
  verify the body is captured and can be decompressed via the API
4. **Fallback test**: Send a corrupt zstd body, verify `decompress_body()`
  returns the raw body (doesn't panic)
5. **Mixed encoding test**: Verify gzip and brotli still work after adding
  zstd (no regression)
6. **API test**: `GET /api/traffic/{id}?decompressed=true` on a zstd response
  returns decompressed body
7. **UI test**: Capture a zstd-compressed response, open traffic detail,
  verify the body viewer shows decompressed content when toggled
8. **reqwest test**: Verify upstream requests to zstd-capable servers work
  (reqwest with `"zstd"` feature)

### What needs to be documented

- Update `CLAUDE.md` — note zstd support in the content-encoding list
- Update `docs/CHARLES_PROXY_FEATURE_COMPARISON.md` — change zstd row from ❌
  to ✅
- Update the madhyamas skill — mention zstd decompression support
- Update `docs/ARCHITECTURE.md` — add zstd to the content-encoding handling
  description

---

## Implementation Priority Order

Based on complexity, impact, dependencies, and quick-win potential:

| Priority | Feature | Effort | Impact | Dependencies | Notes |
|---|---|---|---|---|---|
| 1 | **zstd Decompression** | Small | Medium | `zstd` crate | Add one match arm + one dependency. Quick win. |
| 2 | **Edit-then-Repeat** | Small-Medium | High | None — backend already supports modifications | UI-only work (build `RequestEditor` component). High value for debugging workflows. |
| 3 | **HAR Import** | Medium | High | None — follows session import pattern | High interoperability value (import from Chrome/Charles/Fiddler). |
| 4 | **Repeat Advanced** | Medium | Medium-High | None — extends existing replay | Adds load-testing capability. Backend + UI work. |
| 5 | **Recording Size Limits** | Medium | High | None — `max_requests` config already exists | Prevents runaway DB growth. Wires existing unused config + MemoryManager. |
| 6 | **Focus Feature** | Medium | Medium | New DB table + API + UI | UX improvement for high-traffic debugging. |
| 7 | **Chart / Timeline** | Medium | Medium | New charting library (recharts) | Pure frontend. High visual impact but requires careful virtualization. |
| 8 | **Auto Save** | Medium | Medium-Low | Background task infrastructure | Less critical since SQLite already persists in real time. Mainly for backups. |
| 9 | **Mirror Tool** | Medium-Hard | Low-Medium | New module + disk I/O | Niche feature; lower demand. |

**Recommended approach:**

1. **Ship #1 (zstd) first** — it's a one-file, one-dependency change with no
   risk. Immediate compatibility win.
2. **Ship #2 (Edit-then-Repeat) next** — the backend already supports it; it's
   purely a UI component (`RequestEditor`). High daily-use value for
   debugging. The same component can later be reused for breakpoints.
3. **Ship #3 (HAR Import) and #4 (Repeat Advanced) together** — both extend
   existing subsystems (import/export and replay) following established
   patterns. Medium effort, high interoperability and testing value.
4. **Ship #5 (Recording Size Limits)** — wires existing unused config
   (`max_requests`) and the unused `MemoryManager`. Prevents the most common
   operational issue (runaway DB growth). Also connects the localStorage-only
   capture UI to the backend.
5. **Ship #6 (Focus) and #7 (Chart/Timeline)** — both are UX-focused frontend
   work. Focus is straightforward CRUD + highlighting. Chart requires a new
   dependency and careful virtualization but has high visual impact.
6. **Ship #8 (Auto Save) and #9 (Mirror) last** — both are lower-demand
   features. Auto Save is less critical given SQLite's real-time persistence.
   Mirror is niche but straightforward to implement.

**Cross-feature synergies:**
- **Edit-then-Repeat (#2) + Repeat Advanced (#4)**: The `RequestEditor`
  component from #2 can be embedded in the batch replay UI from #4, allowing
  edit-then-batch-replay.
- **Chart/Timeline (#3) + Repeat Advanced (#4)**: Batch replay results
  (timing statistics) can be visualized in a chart, reusing the charting
  library installed for #3.
- **Recording Size Limits (#5) + Auto Save (#6)**: Both deal with data
  lifecycle management. The background task infrastructure from Auto Save can
  be reused for periodic cleanup in Recording Size Limits.
- **HAR Import (#3) + Auto Save (#6)**: Auto Save's HAR export format
  produces files that HAR Import can consume — round-trip compatibility.

---

*Generated 2026-08-01. Based on codebase analysis as of this date. Companion
document to [HIGH_PRIORITY_FEATURE_ANALYSIS.md](HIGH_PRIORITY_FEATURE_ANALYSIS.md).*
