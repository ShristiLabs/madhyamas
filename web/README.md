# Madhyamas Web UI

A modern, responsive, lightweight, and performant web UI for the Madhyamas
HTTP debugging proxy. Built with React 18, TypeScript, Vite, Tailwind
CSS, and shadcn/ui with a **DevTools dark-first** design language.

## Features

All functionality is supported:

- **Traffic capture** — virtualized sortable/resizable list, multi-select,
  Request/Response/Timing detail tabs, JSON viewer, body decode, copy-as
  (cURL/HTTPie/Fetch/wget), export JSON/HAR, WebSocket live + REST polling.
- **Intercept tools** — Breakpoints, Throttle, Mocks (collections, recording,
  analytics, versioning, import/export, testing), Rewrites.
- **Debug & extend** — Replay, gRPC, Scripts, Plugins.
- **Sessions** — create, list, switch, export, import, delete.
- **Supporting** — Certificate helper, Config dialog, Onboarding wizard,
  keyboard shortcuts, dark/light theme.

## Design

- **Dark-first DevTools aesthetic** — dense rows (26px), compact controls,
  monospace accents for methods/status/URLs/headers/bodies.
- **Status & method color coding** — 2xx green, 3xx blue, 4xx amber, 5xx red;
  GET green, POST orange, PUT blue, DELETE red, etc.
- **Responsive** — collapses to stacked single-pane on mobile/tablet; tools
  sidebar hidden below `lg` breakpoint.
- **Performant** — list virtualization (`@tanstack/react-virtual`), lazy-loaded
  heavy panels (Mocks, Replay, gRPC, Scripts, Plugins, Cert, Config),
  manual vendor chunk splitting. Initial JS ~53KB gzip.

## Development

```bash
# Install dependencies
npm install

# Dev server (proxies /api and /api/ws to http://127.0.0.1:3001)
npm run dev          # → http://localhost:5174

# Typecheck / lint / build
npm run typecheck
npm run lint
npm run build        # → dist/
```

> The dev server expects the Madhyamas backend running on port 3001
> (`cargo run` or `./startup-local.sh`).

## Serving from the backend

The backend reads `MADHYAMAS_WEB_DIR` (see
`crates/madhyamas-api/src/lib.rs`) and falls back to `web/dist` when unset.

```bash
# From the repo root — build and run
./startup-local.sh
```

## Structure

```
web/
├── src/
│   ├── App.tsx                  # Root: providers, shell, theme
│   ├── main.tsx                 # Entry
│   ├── index.css                # Design tokens (dark-first) + base styles
│   ├── components/ui/           # shadcn/ui primitives (refreshed, dense)
│   ├── features/
│   │   ├── shell/               # AppHeader, NavRail, CertificateHelper, ConfigDialog
│   │   ├── traffic/             # TrafficView, List (virtualized), Detail, Toolbar, FilterBuilder, JsonView
│   │   ├── tools/               # 8 tool panels + MockEditDialog
│   │   ├── sessions/            # SessionsPanel
│   │   ├── cert/                # CertificatePanel
│   │   ├── config/              # ConfigDialog
│   │   └── onboarding/          # OnboardingWizard
│   ├── hooks/                   # useTraffic, useTrafficWebSocket, useWebSocket
│   ├── lib/api/                 # phase3, cert, intercept, sessions API hooks
│   └── types/                   # traffic, filters, websocket
├── index.html
├── tailwind.config.js
├── vite.config.ts
└── package.json
```

## Tech

- React 18 + TypeScript + Vite 5
- Tailwind CSS 3 + shadcn/ui (Radix primitives)
- TanStack Query (data) + TanStack Virtual (list virtualization)
- lucide-react (icons), react-json-view-lite, qrcode.react
