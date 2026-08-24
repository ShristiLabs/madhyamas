# Documentation Versioning

This document describes how the Madhyamas docs site
([VitePress](https://vitepress.dev/), `docs-site/`) publishes versioned
documentation. VitePress has no native versioning, so the design below is
built explicitly.

## Decided strategy

- **SNAPSHOT docs publish from the `main` branch.** Every push to `main`
  that touches `docs-site/**` rebuilds `/madhyamas/SNAPSHOT/`, replacing the
  previous SNAPSHOT. SNAPSHOT documents the unreleased state of `main`.
- **Release-versioned docs publish from release tags.** Cutting a `vX.Y.Z`
  tag builds `/madhyamas/vX.Y.Z/` from that tag. Release directories are
  preserved on every subsequent deploy.

No orphan/persistent docs branch is used.

## Path layout

| Path | Contents | Rebuilt by |
|---|---|---|
| `/madhyamas/SNAPSHOT/` | Docs for unreleased `main` | Every `main` push touching `docs-site/**` |
| `/madhyamas/vX.Y.Z/` | Docs for release tag `vX.Y.Z` (e.g. `/madhyamas/v0.1.6/`) | Tag push / release |
| `/madhyamas/latest/` | Copy of the newest release tree | Every release deploy |
| `/madhyamas/` | Redirect (`index.html`, meta-refresh) to `latest/` | Every release deploy |

**Canonical version form**: the full `v`-prefixed semver tag, `vX.Y.Z`
(e.g. `v0.1.6`). Minor-only aliases (`0.1`) are not published; the tag form
keeps the mapping from URL to git tag exact.

## How version preservation works in CI

GitHub Pages replaces the entire site on every deploy, so a naive
multi-version build would be wiped. The
[deploy-docs.yml](../.github/workflows/deploy-docs.yml) workflow solves this
with **artifact chaining**, entirely within the workflow:

1. It queries the last successful run of `deploy-docs.yml` and downloads
   that run's `github-pages` artifact (via `gh run download`) — this tar
   contains the full multi-version site as deployed.
2. The tar is unpacked into a staging directory, becoming the baseline.
3. The build script overlays only the version tree(s) being (re)built into
   staging — `SNAPSHOT/` for main pushes, `vX.Y.Z/` (+ `latest/` copy and
   root redirect) for releases — leaving all other version directories
   untouched.
4. Staging is re-tarred and uploaded with `actions/upload-pages-artifact`,
   then deployed.

A single concurrency group (`deploy-docs`, non-cancelling) serializes runs so
two deploys cannot chain from the same baseline.

**Tradeoffs (known and accepted):**

- Artifact chaining depends on the previous run's Pages artifact being still
  downloadable. If it has expired or been removed, the workflow logs a
  warning and rebuilds from a clean site — meaning older version directories
  are lost until their tags are re-run. Recovery is cheap: re-run
  `deploy-docs.yml` for each published tag (see Backports below).
- `workflow_dispatch`-triggered runs of `deploy-docs.yml` do not produce a
  `github-pages` artifact chain entry themselves unless they succeed through
  deploy, which they do — chaining works for any successful run regardless
  of trigger.

## Version switcher

`.vitepress/config.ts` builds each version with:

- `DOCS_VERSION` — the version being built (`SNAPSHOT` or `vX.Y.Z`),
  driving `base: /madhyamas/<DOCS_VERSION>/`.
- `DOCS_VERSIONS` — comma-separated list of all published versions
  (newest first), driving the nav dropdown.

The nav has a `Version: <current>` dropdown listing every published version.
**Behavior**: switching versions lands the user on that version's home page.
Staying on the equivalent page across versions is not attempted — page sets
differ between versions, so a silent redirect could 404. The switcher appears
on every version, including SNAPSHOT.

For release builds, the "Edit this page" link points at the tag's tree
(`.../edit/vX.Y.Z/docs-site/...`) instead of `main`.

## Release integration

- Pushing a `vX.Y.Z` tag triggers `deploy-docs.yml` in release mode
  (subject to its `paths` filter — releases with no doc changes may skip).
- To guarantee every release publishes docs, the `docs` job in
  [release.yml](../.github/workflows/release.yml) explicitly triggers
  `deploy-docs.yml` for the tag after the GitHub release is created. The two
  triggers are idempotent; the concurrency group serializes them.

## Backporting doc fixes to a published version

1. Cut a release branch from the tag: `git checkout -b release/v0.1.6 v0.1.6`.
2. Cherry-pick the doc fixes (`docs-site/**` only).
3. Run the workflow manually:
   `gh workflow run deploy-docs.yml --ref release/v0.1.6 -f docs_version=v0.1.6`.
   This rebuilds `v0.1.6/` from that branch. If `v0.1.6` is still the newest
   release, `latest/` and the root redirect are refreshed too.

## Local development and testing

```bash
cd docs-site
npm run dev        # dev server (defaults to SNAPSHOT base)

# Build the versioned staging tree locally:
./scripts/build-versioned.sh snapshot            # builds staging/SNAPSHOT/
./scripts/build-versioned.sh release v0.1.6      # adds staging/v0.1.6/ + latest/ + index.html
./scripts/build-versioned.sh backport v0.1.6     # rebuild an existing version in place
```

Output accumulates under `docs-site/.vitepress/staging/` — exactly the
layout CI deploys. `scripts/check-versioned-build.sh` verifies the layout
end to end (used by CI).
