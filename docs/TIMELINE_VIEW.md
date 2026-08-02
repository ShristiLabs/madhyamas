# Timeline View (Waterfall Chart)

The Timeline view is a waterfall chart visualization of captured traffic. Each
request is shown as a horizontal bar positioned by its start time and sized
proportional to its duration, making it easy to spot slow requests, parallel
downloads, and overlapping calls at a glance.

## Switching Between List and Timeline Views

The traffic panel sub-toolbar contains a two-button toggle group (List icon /
Activity icon) next to the Focus button:

- **List** (default) — the traditional sortable table view with columns for
  Method, Proto, Domain, Path, Status, Time, Size, and When.
- **Timeline** — the waterfall chart described below.

Filters, search, and the currently selected entry are preserved when switching
between views. The detail panel on the right stays in sync — selecting a bar in
the timeline opens the same detail panel as selecting a row in the list.

## How to Read the Waterfall

- **X-axis** — relative time from the first visible entry (in ms or s). Tick
  marks with labels run along the top.
- **Y-axis** — one row per request, sorted chronologically (oldest at top).
- **Bar position** — the left edge of each bar marks the request's start time
  relative to the earliest entry in the visible set.
- **Bar length** — proportional to the response duration (`duration_ms`).
  Requests with no response yet (pending) show a minimal marker.

The left label column shows the HTTP method (color-coded) and the host + path
for quick identification.

## Color Coding by Status Code

Bars are colored by the HTTP response status code class:

| Color  | Status class | Meaning                        |
|--------|-------------|-------------------------------|
| Green  | 2xx         | Success                       |
| Blue   | 3xx         | Redirection                   |
| Orange | 4xx         | Client error                  |
| Red    | 5xx         | Server error                  |
| Gray   | —           | Pending / no response yet     |

A legend is displayed at the top of the chart for reference.

## Hover Tooltip

Hovering over any row displays a tooltip with:

- HTTP method and full URL (host + path)
- Response status code (color-coded)
- Duration (formatted as ms or s)
- Absolute timestamp (wall-clock time)

## Click to Select

Clicking a bar selects that entry and opens the detail panel on the right,
exactly like clicking a row in the list view. The selected bar is highlighted
with a primary-tinted background. Keyboard navigation (Enter / Space) is also
supported.

## Virtualization

The timeline uses `@tanstack/react-virtual` for row virtualization, the same
library used by the list view. This keeps the chart performant even with
thousands of captured entries — only the visible rows (plus a small overscan
buffer) are rendered.

## Mini-Chart in the Traffic Detail

The **Timing** tab in the traffic detail panel now includes a mini waterfall
bar at the top. This bar visualizes the selected request's duration as a
colored horizontal bar (color-coded by status code) on a scale from 0 ms to
the request's duration (or a minimum of 100 ms for very fast requests). The
existing text fields (Timestamp, Duration, Request Size, Response Size) remain
below the mini-chart.
