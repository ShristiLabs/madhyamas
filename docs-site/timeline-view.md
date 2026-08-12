---
title: Timeline View
description: Visualize captured Madhyamas traffic as a waterfall chart — spot slow requests, parallel downloads, and overlapping calls at a glance with the Timeline view.
---

# Timeline View

The Timeline view is a **waterfall chart** visualization of captured traffic. Each request is shown as a horizontal bar positioned by its start time and sized proportional to its duration, making it easy to spot slow requests, parallel downloads, and overlapping calls at a glance.

## Switching Between List and Timeline

The traffic panel sub-toolbar contains a two-button toggle group (List icon / Activity icon) next to the Focus button:

- **List** (default) — the traditional sortable table view with columns for Method, Proto, Domain, Path, Status, Time, Size, and When.
- **Timeline** — the waterfall chart described below.

Filters, search, and the currently selected entry are preserved when switching between views. The detail panel on the right stays in sync — selecting a bar in the timeline opens the same detail panel as selecting a row in the list.

## How to Read the Waterfall

- **X-axis** — relative time from the first visible entry (in ms or s). Tick marks with labels run along the top.
- **Y-axis** — one row per request, sorted chronologically (oldest at top).
- **Bar position** — the left edge of each bar marks the request's start time relative to the earliest entry in the visible set.
- **Bar length** — proportional to the response duration. Requests with no response yet (pending) show a minimal marker.

The left label column shows the HTTP method (color-coded) and the host + path for quick identification.

## Color Coding by Status Code

Bars are colored by the HTTP response status code class:

| Color | Status class | Meaning |
|-------|-------------|---------|
| Green | 2xx | Success |
| Blue | 3xx | Redirection |
| Orange | 4xx | Client error |
| Red | 5xx | Server error |
| Gray | — | Pending / no response yet |

A legend is displayed at the top of the chart for reference.

## Hover Tooltip

Hovering over any row displays a tooltip with:

- HTTP method and full URL (host + path)
- Response status code (color-coded)
- Duration (formatted as ms or s)
- Absolute timestamp (wall-clock time)

## Click to Select

Clicking a bar selects that entry and opens the detail panel on the right, exactly like clicking a row in the list view. The selected bar is highlighted, and keyboard navigation (Enter / Space) is also supported.

## Mini-Chart in the Traffic Detail

The **Timing** tab in the traffic detail panel includes a mini waterfall bar at the top. This bar visualizes the selected request's duration as a colored horizontal bar (color-coded by status code) on a scale from 0 ms to the request's duration. The existing text fields (Timestamp, Duration, Request Size, Response Size) remain below the mini-chart.

## Performance

The timeline uses row virtualization, the same technique as the list view. This keeps the chart performant even with thousands of captured entries — only the visible rows (plus a small overscan buffer) are rendered.

## Common Use Cases

### Spotting Slow Requests

A waterfall makes outliers obvious: the longest bar in the chart is your slowest request, instantly visible without sorting or filtering.

### Understanding Parallelism

See which requests ran concurrently and which were sequential — useful for understanding how your app loads resources or batches API calls.

### Diagnosing Waterfall Stalls

Identify requests that block others — for example, a slow authentication call that delays every subsequent request. The relative positioning makes these dependencies visible.

## See also

- [Traffic Inspection](./traffic-inspection) — the list view and filtering
- [Focus](./focus) — highlight specific hosts in the timeline
- [Throttling](./throttling) — simulate slow networks to populate the waterfall
- [Sessions](./sessions) — scope the timeline to a session
