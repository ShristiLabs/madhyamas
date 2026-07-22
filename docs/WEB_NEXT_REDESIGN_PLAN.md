# Madhyamas Web UI Redesign Plan

## Goal
Replace the current `web/` UI with a modern, responsive, lightweight, performant,
and elegant UI developed in a **separate `web/` folder** without affecting
the existing `web/` folder. All current functionality must be retained.

## Decisions
- **Stack:** React 18 + Vite + TypeScript + Tailwind CSS (same as current)
- **Components:** shadcn/ui (refreshed theme + new components as needed)
- **Integration:** Backend serves `MADHYAMAS_WEB_DIR` env var (default `web/dist`).
  New UI builds to `web/dist`. Zero risk to current UI.
- **Visual direction:** DevTools dark-first — dense, monospace accents, status
  color coding, compact panels, power-user optimized. Light theme supported.

## Feature Surface (must retain)
1. **Traffic capture**
   - List: sortable + resizable columns, multi-select, virtualized for perf
   - Detail: Request / Response / Timing tabs; JSON viewer; body decode
     (base64: prefix + manual decode); copy-as cURL/HTTPie/Fetch/wget;
     export JSON/HAR
   - Toolbar: search, quick filters (Errors/Slow/API), advanced filter builder
     with chips, clear all
   - Clear (selected/all), Export HAR (selected/all)
   - WebSocket live mode + REST polling toggle, connection status indicator
   - Capture / passthrough toggle (header)
2. **Tools sidebar (8 panels)**
   - Breakpoints (rules CRUD, paused traffic, resume/abort)
   - Throttle (profile config, presets, enable toggle)
   - Mocks (rules CRUD, edit dialog w/ response configs, collections,
     recording, analytics, versioning, import/export, testing/preview,
     duplicate, rollback)
   - Rewrites (rules CRUD, templates, toggle)
   - Replay (saved requests, history, execute w/ modifications)
   - gRPC (connections, streams, frames, stats, clear)
   - Scripts (CRUD, templates, toggle, config)
   - Plugins (list, enable/disable, reload, stats)
3. **Supporting**
   - Certificate helper / panel (download CA, install instructions)
   - Config dialog (proxy/api ports, host, public IP, theme)
   - Onboarding wizard
   - Theme toggle (dark/light), keyboard shortcuts modal
   - Proxy address display

## Phases

### Phase 0 — Foundation & Integration
- [ ] Scaffold `web/` (Vite + React + TS + Tailwind + shadcn/ui)
- [ ] Vite dev proxy → `http://127.0.0.1:3001` for `/api` and `/api/ws`
- [ ] Design system: dark-first devtools tokens, base UI primitives
      (button, input, dialog, tabs, dropdown, tooltip, toast, badge, card,
      switch, checkbox, select, scroll-area, accordion, slider, label,
      textarea, resizable)
- [ ] Reuse (copy + adapt) `types/`, `hooks/`, `lib/api/` from `web/`
- [ ] Backend: read `MADHYAMAS_WEB_DIR` env var for `ServeDir` path
      (default `web/dist`) — minimal non-breaking change
- [ ] `web/README.md`

### Phase 1 — App Shell + Traffic Core
- [ ] App shell: header (logo, proxy address, capture toggle, setup, config,
      theme toggle), main area, toaster
- [ ] TrafficView: responsive 3-pane layout (list | detail | tools) with
      collapsible tools + responsive breakpoints
- [ ] TrafficToolbar: search, quick filters, filter builder, count
- [ ] TrafficList: virtualized, sortable, resizable columns, multi-select
- [ ] TrafficDetail: tabs, headers table, body view, JSON viewer, copy-as,
      export, timing
- [ ] WebSocket + REST polling hooks (useTraffic, useTrafficWebSocket,
      useWebSocket)
- [ ] Connection status + WS/poll toggle

### Phase 2 — Intercept Tools
- [ ] ToolsSidebar shell (vertical tab bar, categories, collapse)
- [ ] BreakpointsPanel
- [ ] ThrottlePanel
- [ ] MocksPanel + MockEditDialog + collections + recording + analytics +
      versioning + import/export + testing
- [ ] RewritesPanel

### Phase 3 — Debug & Extend Tools
- [ ] ReplayPanel
- [ ] GrpcPanel
- [ ] ScriptsPanel
- [ ] PluginsPanel

### Phase 4 — Supporting Features
- [ ] CertificateHelper / CertificatePanel
- [ ] ConfigDialog
- [ ] OnboardingWizard
- [ ] Keyboard shortcuts modal

### Phase 5 — Polish & Verification
- [ ] Responsive: mobile (stacked), tablet (collapsible panes), desktop
- [ ] Performance: list virtualization, memoization, lazy-load heavy panels
- [ ] Build, typecheck, lint pass
- [ ] Wire `MADHYAMAS_WEB_DIR=web/dist` into startup scripts (optional flag)
- [ ] Final visual + interaction review

## Design Principles (DevTools dark-first)
- **Density:** compact row heights (24-28px), tight padding, small font (12-13px)
- **Monospace:** method, status, URL, headers, bodies use mono accents
- **Color coding:** method colors (GET green, POST orange, etc.), status bands
  (2xx green, 3xx blue, 4xx amber, 5xx red)
- **Borders over shadows:** subtle 1px borders, minimal elevation
- **Status indicators:** live/polling dots, capture state, connection state
- **Keyboard-first:** shortcuts preserved (R, C, T, ?, Esc)
- **Responsive:** collapses to stacked/single-pane on narrow screens
