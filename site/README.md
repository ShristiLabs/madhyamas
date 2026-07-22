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

### GitHub Pages (automatic)

A workflow at `.github/workflows/pages.yml` deploys this directory to GitHub Pages
on every push to `main` that changes `site/**`. It can also be triggered manually
from the Actions tab ("Deploy site to GitHub Pages" → Run workflow).

One-time setup in the repo:

1. Go to **Settings → Pages → Build and deployment → Source**
2. Select **GitHub Actions** (not "Deploy from a branch")
3. Push to `main` (or run the workflow manually) — the site goes live at
   `https://shristilabs.github.io/madhyamas/`

The `site/.nojekyll` file tells Pages to serve the files verbatim without
running Jekyll. All asset paths in `index.html` are relative, so the site
works correctly under the project subpath without a base URL.

### Other static hosts

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
