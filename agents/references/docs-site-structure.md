# docs-site Structure & Authoring Guide

The end-user documentation site for Madhyamas, built with VitePress and deployed
to GitHub Pages. Load this reference when authoring or restructuring end-user docs.

## Stack

- **Generator**: [VitePress](https://vitepress.dev/)
- **Location**: `docs-site/` at repo root
- **Deploy**: GitHub Pages (base path `/madhyamas/`)
- **Source**: Markdown (`.md`) files; config in `.vitepress/config.ts`

## Directory Layout

```
docs-site/
├── .vitepress/
│   ├── config.ts          # VitePress config: nav, sidebar, theme, head
│   └── theme/
│       ├── index.ts       # Theme entry
│       └── style.css      # Brand styling overrides
├── public/
│   ├── favicon.svg
│   └── screenshots/       # UI screenshots referenced in docs
├── index.md               # Home page (VitePress hero layout)
├── getting-started.md
├── traffic-inspection.md
├── https-certificates.md
├── breakpoints.md
├── mocks.md
├── rewrites.md
├── rewrite-templates.md
├── throttling.md
├── replay.md
├── sessions.md
├── mobile-setup.md
├── configuration.md
├── timeline-view.md
├── har-import.md
├── scripting.md
├── plugins.md
├── socks-proxy.md
├── upstream-proxy.md
├── access-control.md
├── block-list.md
├── focus.md
├── mirror.md
├── auto-save.md
├── recording-limits.md
├── http2-grpc.md
└── package.json
```

## Commands

```bash
cd docs-site
npm install
npm run dev      # Dev server at http://localhost:5173/madhyamas/
npm run build    # Static site to .vitepress/dist/
npm run preview  # Preview the production build
```

## Authoring Rules

1. **One feature per file.** Filename matches the feature, lowercase-hyphenated.
2. **Frontmatter**: VitePress supports optional YAML frontmatter (`title`, `description`,
   `layout`). Use `title` only when it differs from the H1.
3. **H1 = page title.** Start every page with a single `# Title`.
4. **Structure**: H1 → short intro → "## Prerequisites" (if any) → "## Steps" →
   "## Verification" → "## Troubleshooting" (if applicable) → "## See also".
5. **Code blocks**: always specify language. Use `bash` for shell, `json` for config,
   `http` for HTTP examples, `ts`/`rust` for code.
6. **Screenshots**: place under `public/screenshots/`, reference with the
   root-absolute path `/screenshots/foo.png`. VitePress automatically prepends
   the configured `base` (`/madhyamas/`) at build time, so the rendered URL
   becomes `/madhyamas/screenshots/foo.png` on GitHub Pages. Do **not** hardcode
   the `/madhyamas/` prefix in Markdown — that would double the base path.
   Always provide alt text.
7. **Cross-links**: use relative links (`./mocks.md`) — VitePress rewrites them.
8. **No emojis** in prose, headings, or code comments.
9. **Voice**: second person ("You can..."), present tense, concise. Aim for a
   reader who has never used a debugging proxy before.
10. **Callouts**: use VitePress custom containers (`::: tip`, `::: warning`,
    `::: danger`, `::: info`) sparingly for emphasis.
11. **Tables** for option/flag references; **numbered lists** for procedures;
    **bullet lists** for enumerations.

## Navigation & IA

Edit `.vitepress/config.ts` to:
- Add a `nav` entry for top-level sections.
- Add a `sidebar` entry so the page appears in the left nav.
- Group related pages under a collapsible sidebar section.

The sidebar is the primary IA. Keep it shallow (max 2 levels). If a section grows
beyond ~10 pages, split into a sub-section.

## SEO & Metadata

- Every page should have a meaningful first paragraph (used as meta description).
- `config.ts` sets global `title`, `description`, `head` tags (OpenGraph, favicon).
- Keep page `<title>` (H1) under 60 characters where possible.
- Use descriptive anchor text for links; avoid "click here".

## Sync with `docs/`

`docs/` (developer reference) and `docs-site/` (end-user guide) cover the same
features from different angles. When a feature changes:
1. Update `docs/<FEATURE>.md` (technical reference, API contracts, internals).
2. Update `docs-site/<feature>.md` (user-facing how-to, screenshots, troubleshooting).
3. Cross-link between them only when the user genuinely benefits.

Do NOT duplicate content verbatim — the dev doc explains *how it works*, the
end-user doc explains *how to use it*.

## Deployment

GitHub Actions workflow builds `docs-site/` and publishes to GitHub Pages on push
to `main`. The base URL is `/madhyamas/`. Verify the build locally with
`npm run build` before committing large changes.
