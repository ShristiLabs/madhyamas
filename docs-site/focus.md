---
title: Focus
description: Highlight traffic from specific hosts in the Madhyamas traffic view without hiding the rest — visually emphasize the hosts you care about in a busy stream.
---

# Focus

Focus lets you **highlight traffic from specific hosts** in the traffic view. Unlike a filter (which hides non-matching traffic entirely), focus visually emphasizes matching rows while keeping all traffic visible — so you can spot the hosts you care about in a busy stream without losing context.

## Focus vs Filter

| Aspect | Filter | Focus |
|--------|--------|-------|
| Non-matching traffic | Hidden | Dimmed (but still visible) |
| Matching traffic | Shown normally | Bold + yellow left border + star icon |
| Use case | Narrow down to specific requests | Spot specific hosts in a busy stream |
| Persistence | Session-based | Stored in SQLite — survives restarts |

## Two Modes

1. **Highlight mode** (default): All traffic is visible. Rows whose host matches a focus pattern are highlighted with a yellow left border, a star icon, and bold domain text. Non-matching rows are dimmed (50% opacity).

2. **Focus-only mode**: Toggle "Show only focused" in the Focus panel to hide non-matching rows entirely — like a filter, but driven by your focus patterns.

## Pattern Syntax

Focus patterns are case-insensitive and support:

| Pattern | Matches | Example |
|---------|---------|---------|
| `api.example.com` | Exact host or any subdomain | `api.example.com`, `sub.api.example.com` |
| `*.example.com` | Wildcard subdomain (not the bare domain) | `api.example.com`, `sub.example.com` |
| `*api*` | Glob — any host containing `api` | `api.example.com`, `myapi.service.com` |
| `api.*` | Glob — starts with `api.` | `api.example.com`, `api.service.io` |

## Using the Web UI

### The Focus Panel

Click the **Focus** button in the traffic sub-toolbar to open the Focus panel sidebar. From there you can:

- **Add** a focus host pattern (type and press Enter, or click +)
- **Remove** individual focus hosts (hover a row and click ×)
- **Clear all** focus hosts (trash icon in the panel header)
- **Toggle "Show only focused"** to switch between highlight mode and focus-only mode

### Right-Click "Focus this host"

Right-click any traffic row and select **Focus this host** to instantly add its host as a focus pattern. This is the quickest way to focus on a specific service you see in the list.

## Using the CLI

```bash
madhyamas focus list                  # List all focus host patterns
madhyamas focus add api.example.com   # Add a focus host
madhyamas focus add "*.example.com"   # Add a wildcard pattern
madhyamas focus remove <id>          # Remove a focus host by ID
madhyamas focus clear                 # Clear all focus hosts
```

## Persistence

Focus hosts are stored in the SQLite database alongside your traffic. They persist across restarts, so your focus setup is ready the next time you launch the proxy.

## Common Use Cases

### Tracking a Specific Service

While debugging a multi-service app, focus on the one service you're actively investigating. Its requests stand out immediately, even when hundreds of other requests are flowing through the proxy.

### Comparing Environments

Add focus patterns for both your staging and production API hosts to compare their traffic side by side in the same view.

### Monitoring a Third-Party Integration

Focus on a third-party API host (e.g. a payment provider) to watch every call your app makes to it, without losing sight of the surrounding traffic.

## See also

- [Traffic Inspection](./traffic-inspection) — filtering and searching captured traffic
- [Timeline View](./timeline-view) — waterfall visualization
- [Sessions](./sessions) — separating traffic into named groups
- [REST API reference](./rest-api) — `/api/focus` endpoints
