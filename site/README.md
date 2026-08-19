# Madhyamas — Static Site

A clean, modern, responsive marketing site for the Madhyamas HTTP/HTTPS debugging proxy. No build step, no dependencies — just static HTML, CSS, and a sprinkle of vanilla JS.

## Structure

```
site/
├── index.html      # Page markup (all sections)
├── styles.css      # Design system + responsive layout
├── app.js          # Mobile nav, scroll effects, reveal animations
├── favicon.svg     # Madhyamas "M" mark
└── README.md       # This file
```

## Run locally

It's static — open it however you like:

```bash
# Python
python3 -m http.server --directory site 8080
# then visit http://localhost:8080

# Node (if http-server is installed)
npx http-server site -p 8080

# Or just open the file directly
open site/index.html
```

## Deploy

> **Retired from CI (2026-08):** this directory is **no longer deployed to
> GitHub Pages**. The `pages.yml` workflow that deployed it was removed —
> it conflicted with the canonical VitePress deployment (`docs-site/` via
> `deploy-docs.yml`): the two workflows overwrote each other's Pages
> deployment. The directory is kept for reference; it can be deleted or
> self-hosted at any time.

### Other static hosts

The site is dependency-free static files — it still works anywhere:

- **Netlify / Vercel / Cloudflare Pages**: set the publish directory to `site/`.
- **Any web server**: copy the contents of `site/` to your web root.

No environment variables, no framework runtime, no SSR.

## Design notes

- **Theme**: dark (`#0b0e14`) with a blue accent (`#2563eb`), matching the Madhyamas app UI.
- **Typography**: system font stack for fast loading and native feel.
- **Responsive**: fluid layouts with breakpoints at 980px and 620px.
- **Accessible**: semantic HTML, keyboard-reachable nav, `prefers-reduced-motion` support.
- **Zero dependencies**: no CDNs, no frameworks, no tracking.

## Updating content

All copy lives in `index.html`. Feature lists, the comparison table, install instructions, and links can be edited directly. Colors and spacing live in `styles.css` under the `:root` custom properties.
