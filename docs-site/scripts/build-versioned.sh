#!/usr/bin/env bash
# build-versioned.sh — build versioned Madhyamas docs.
#
# Modes:
#   build-versioned.sh snapshot                 Build the SNAPSHOT version (main branch)
#   build-versioned.sh release <tag>            Build a release version (e.g. v0.1.6),
#                                              refresh the latest/ alias and the root
#                                              redirect. <tag> must match vX.Y.Z.
#   build-versioned.sh backport <tag>           Rebuild an existing release version in
#                                              place (doc backports); latest/ is only
#                                              refreshed if <tag> is the newest release.
#
# Environment / flags:
#   --staging <dir>   Staging root that accumulates version trees
#                     (default: docs-site/.vitepress/staging).
#   --versions <list> Comma-separated list of all published versions, newest first
#                     (drives the nav version switcher). Default: derived from the
#                     staging directory contents plus SNAPSHOT, newest first.
#   --ci              Also chain from the previously deployed Pages artifact
#                     (CI only; requires GITHUB_TOKEN and the gh CLI).
#
# Layout produced under the staging root:
#   SNAPSHOT/          docs built from main (replaced on each snapshot deploy)
#   vX.Y.Z/            docs built from a release tag (preserved across deploys)
#   latest/            copy of the newest release version
#   index.html         meta-refresh redirect to latest/
#
# See docs-site/VERSIONING.md for the full versioning design.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DIST="$DOCS_DIR/.vitepress/dist"
STAGING="$DOCS_DIR/.vitepress/staging"
VERSIONS=""
CI=0

die() { echo "error: $*" >&2; exit 1; }

mode="${1:-}"
[ $# -gt 0 ] && shift
POSITIONAL=()
while [ $# -gt 0 ]; do
  case "$1" in
    --staging) STAGING="$2"; shift 2 ;;
    --versions) VERSIONS="$2"; shift 2 ;;
    --ci) CI=1; shift ;;
    *) POSITIONAL+=("$1"); shift ;;
  esac
done
set -- ${POSITIONAL[@]+"${POSITIONAL[@]}"}

# Sort versions newest-first: release tags by version, SNAPSHOT always first.
sort_versions() {
  # SNAPSHOT (main) first, then release tags newest-first.
  local rest=()
  for v in "$@"; do
    [ "$v" = "SNAPSHOT" ] || rest+=("$v")
  done
  if printf '%s\n' "$@" | grep -qx SNAPSHOT; then echo SNAPSHOT; fi
  if [ ${#rest[@]} -gt 0 ]; then printf '%s\n' "${rest[@]}" | LC_ALL=C sort -r -u -V; fi
}

# Derive the switcher version list from staging contents when not given.
derive_versions() {
  local dirs=()
  [ -d "$STAGING/SNAPSHOT" ] && dirs+=("SNAPSHOT")
  for d in "$STAGING"/v*/; do
    [ -d "$d" ] || continue
    dirs+=("$(basename "$d")")
  done
  [ ${#dirs[@]} -gt 0 ] || dirs+=("SNAPSHOT")
  sort_versions "${dirs[@]}" | paste -sd, -
}

newest_release() {
  for d in $(sort_versions $( [ -d "$STAGING" ] && ls "$STAGING" | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' || true)); do
    echo "$d"; return
  done
}

build_version() { # $1 = version label (SNAPSHOT or vX.Y.Z)
  local version="$1"
  echo "==> Building docs version ${version}"
  (
    cd "$DOCS_DIR"
    rm -rf "$DIST"
    DOCS_VERSION="$version" DOCS_VERSIONS="$VERSIONS" npm run build
  )
  rm -rf "$STAGING/$version"
  mkdir -p "$STAGING"
  mv "$DIST" "$STAGING/$version"
}

refresh_latest_and_redirect() { # $1 = newest release tag (may be empty)
  local newest="$1"
  rm -rf "$STAGING/latest"
  if [ -n "$newest" ] && [ -d "$STAGING/$newest" ]; then
    echo "==> Refreshing latest/ alias from ${newest}"
    cp -R "$STAGING/$newest" "$STAGING/latest"
  fi
  if [ -n "$newest" ]; then
    echo "==> Writing root redirect index.html -> latest/"
    cat > "$STAGING/index.html" <<EOF
<!DOCTYPE html>
<html lang="en-US">
  <head>
    <meta charset="utf-8">
    <title>Madhyamas Documentation</title>
    <link rel="canonical" href="https://shristilabs.github.io/madhyamas/latest/">
    <meta http-equiv="refresh" content="0; url=https://shristilabs.github.io/madhyamas/latest/">
  </head>
  <body>
    <p>Redirecting to the <a href="https://shristilabs.github.io/madhyamas/latest/">latest Madhyamas documentation</a>.</p>
  </body>
</html>
EOF
  fi
}

case "$mode" in
  snapshot)
    [ -n "$VERSIONS" ] || VERSIONS="$(derive_versions)"
    build_version "SNAPSHOT"
    refresh_latest_and_redirect "$(newest_release)"
    ;;
  release)
    tag="${1:?usage: build-versioned.sh release <tag>}"
    echo "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' || die "release tag must look like vX.Y.Z, got: $tag"
    [ -n "$VERSIONS" ] || VERSIONS="$(derive_versions)"
    case ",$VERSIONS," in
      *",$tag,"*) ;;
      *) VERSIONS="$(sort_versions "$tag" "$(echo "$VERSIONS" | tr ',' '\n')" | paste -sd, -)" ;;
    esac
    build_version "$tag"
    refresh_latest_and_redirect "$tag"
    ;;
  backport)
    tag="${1:?usage: build-versioned.sh backport <tag>}"
    echo "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$' || die "backport tag must look like vX.Y.Z, got: $tag"
    [ -n "$VERSIONS" ] || VERSIONS="$(derive_versions)"
    case ",$VERSIONS," in
      *",$tag,"*) ;;
      *) die "cannot backport ${tag}: not present in version list" ;;
    esac
    build_version "$tag"
    # Only refresh latest/ if this backport targets the newest release.
    refresh_latest_and_redirect "$(newest_release)"
    ;;
  *)
    die "usage: build-versioned.sh {snapshot|release <tag>|backport <tag>} [--staging <dir>] [--versions <list>] [--ci]"
    ;;
esac

echo "==> Staging tree:"
find "$STAGING" -maxdepth 1 -mindepth 1 | sort

# CI mode: replace staging content with prior deployed artifact + this build's
# overlays is handled by the workflow (it seeds staging before calling us and
# calls us with --staging pointing at the unpacked prior artifact).
if [ "$CI" -eq 1 ]; then
  echo "==> CI mode: staging is Pages-artifact compatible; repack in workflow"
fi
