# Madhyamas Documentation Site

End-user documentation for Madhyamas, built with [VitePress](https://vitepress.dev/) and deployed to GitHub Pages.

## Structure

```
docs-site/
├── .vitepress/
│   ├── config.ts          # VitePress configuration (nav, sidebar, theme)
│   └── theme/
│       ├── index.ts       # Theme entry
│       └── style.css      # Brand styling overrides
├── public/
│   ├── favicon.svg        # Site favicon
│   └── screenshots/       # UI screenshots used in docs
├── index.md               # Home page (hero layout)
├── getting-started.md     # Installation & first steps
├── traffic-inspection.md  # Viewing, filtering, exporting traffic
├── https-certificates.md  # CA certificate installation
├── breakpoints.md         # Pausing & modifying requests
├── mocks.md               # Creating mock API responses
├── rewrites.md            # Modifying traffic on the fly
├── throttling.md          # Simulating slow networks
├── replay.md              # Re-executing captured requests
├── sessions.md            # Organizing traffic into sessions
├── mobile-setup.md        # Connecting phones/tablets
├── configuration.md       # Settings & options
└── package.json
```

## Development

```bash
cd docs-site
npm install
npm run dev      # Start dev server at http://localhost:5173/madhyamas/
```

## Build

```bash
npm run build    # Output to .vitepress/dist/
npm run preview  # Preview the built site
```

## Deployment

Documentation is automatically built and deployed to GitHub Pages via the
[deploy-docs.yml](../.github/workflows/deploy-docs.yml) GitHub Actions workflow.
The site is **versioned**: pushes to `main` publish `/madhyamas/SNAPSHOT/`,
and release tags publish `/madhyamas/vX.Y.Z/` (plus the `latest/` alias and a
root redirect). See [VERSIONING.md](VERSIONING.md) for the path layout, the
CI version-preservation mechanism (artifact chaining), the version switcher,
and the doc-backport procedure.

**Live URL**: https://shristilabs.github.io/madhyamas/ (redirects to `latest/`)

## Regenerating Screenshots

Screenshots are captured from the running Madhyamas web UI using Playwright:

```bash
# Start Madhyamas
madhyamas serve --host 0.0.0.0

# Generate traffic for realistic screenshots
curl -x http://localhost:8888 http://httpbin.org/get
curl -x http://localhost:8888 -k https://example.com

# Capture screenshots
cd madhyamas
npm install --no-save playwright
node scripts/capture-screenshots.mjs
cp web/public/docs/screenshots/*.png docs-site/public/screenshots/
```
