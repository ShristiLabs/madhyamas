# Enterprise CI/CD Strategy

> Part of: [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md)

This document specifies the CI/CD changes needed to support building,
testing, releasing, and distributing both the **Simple (OSS)** and
**Enterprise** tiers of Madhyamas from the same repository — plus the
separate CI/CD pipeline for the licensing server.

---

## Table of Contents

1. [Current State](#1-current-state)
2. [What Changes and Why](#2-what-changes-and-why)
3. [Principle: One Repo, Two Builds](#3-principle-one-repo-two-builds)
4. [CI Changes (ci.yml)](#4-ci-changes-ciyml)
5. [Release Changes (release.yml)](#5-release-changes-releaseyml)
6. [Docker Changes](#6-docker-changes)
7. [Licensing Server CI/CD](#7-licensing-server-cicd)
8. [Secrets Management](#8-secrets-management)
9. [Branch Strategy](#9-branch-strategy)
10. [Release Cadence](#10-release-cadence)
11. [Implementation Phases](#11-implementation-phases)

---

## 1. Current State

The current CI/CD pipeline builds a **single binary** with all features
enabled (including `enterprise`, which is in the default feature set).

### Current workflows

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | Push/PR to main/develop | Build frontend, Rust checks (fmt, clippy, test), Docker build test, cross-compile for 8 targets, docs check |
| `release.yml` | Tag `v*` or manual dispatch | Build binaries for all platforms, MSI, Snap, RPM, create GitHub Release, publish to Homebrew/Chocolatey/Snap/Docker Hub/crates.io |
| `release-dispatch.yml` | Manual dispatch | Bump version, update CHANGELOG, create tag (triggers release.yml) |
| `codeql.yml` | Push/PR | GitHub CodeQL security analysis |
| `deploy-docs.yml` | Push to main | Deploy VitePress docs to GitHub Pages |
| `skills.yml` | Push to main | Build and publish AI skill package |
| `plugins-release.yml` | Tag `plugins-v*` | Build and publish WASM plugins |

### Current build command

```bash
# CI builds with default features (includes enterprise):
cargo build --profile ci-release --target ${{ matrix.target }} -p madhyamas

# Release builds with embedded-assets:
cargo build --release --target ${{ matrix.target }} -p madhyamas --features embedded-assets
```

### Current crates.io publish

```bash
cargo publish -p madhyamas-core --token "$CRATES_TOKEN"
cargo publish -p madhyamas-api --no-default-features --features grpc,scripting,plugins,enterprise --token "$CRATES_TOKEN"
cargo publish -p madhyamas-cli --token "$CRATES_TOKEN"
cargo publish -p madhyamas-mcp --token "$CRATES_TOKEN"
cargo publish -p madhyamas --token "$CRATES_TOKEN"
```

### Problem

The `enterprise` feature is in the **default** feature set of
`madhyamas-core`, `madhyamas-api`, and `madhyamas`. This means:

1. Every CI build compiles enterprise code (even for OSS users).
2. Every published binary includes enterprise code (dormant but present).
3. The crates.io publish includes enterprise code in the default feature set.
4. There is no way to build a "pure OSS" binary without enterprise code.
5. Enterprise dependencies (`argon2`, `openidconnect`, etc.) are in
   every build's dependency tree.

After the crate extraction (Phase 0 of the enterprise roadmap), this
changes: `enterprise` is removed from default features, and enterprise
code moves to a separate `madhyamas-enterprise` crate. The CI/CD
pipeline must adapt to build **both** tiers.

---

## 2. What Changes and Why

### Summary of changes

| Area | Current | After enterprise refactor |
|---|---|---|
| Default build | Includes enterprise (dormant) | Pure OSS — no enterprise code compiled |
| Enterprise build | Same as default | `--features enterprise` — includes `madhyamas-enterprise` crate |
| CI matrix | One build per target | Two builds per target: simple + enterprise |
| Release artifacts | One binary per platform | Two binaries per platform: `madhyamas` (OSS) + `madhyamas-enterprise` |
| Docker images | One image | Two images: `madhyamas` (OSS) + `madhyamas-enterprise` |
| crates.io publish | All crates with enterprise in default | Core/API/CLI/MCP without enterprise; `madhyamas-enterprise` not published to crates.io |
| GitHub Releases | One release | One release with both OSS and enterprise binaries |
| Licensing server | N/A | Separate repo, separate CI/CD pipeline |

### Why two builds?

The enterprise tier includes a separate `madhyamas-enterprise` crate
that carries a different license (BSL or proprietary). The OSS build
must not include this code — not compiled, not linked, not in the
dependency tree. This is a structural guarantee, not just a feature
gate. See [ENTERPRISE_OVERVIEW.md §2](ENTERPRISE_OVERVIEW.md#2-crate-architecture-separate-madhyamas-enterprise-crate).

### Build artifacts: OSS vs Enterprise

The table below summarizes every build artifact produced by the
CI/CD pipeline for each tier. Both tiers are built from the same
commit, share the same frontend (`web/dist/`), and are attached to
the same GitHub Release.

#### Binary artifacts (per platform)

| Artifact | OSS (Simple) | Enterprise | Notes |
|---|---|---|---|
| Linux x64 tarball | `madhyamas-v{ver}-x86_64-unknown-linux-gnu.tar.gz` | `madhyamas-enterprise-v{ver}-x86_64-unknown-linux-gnu.tar.gz` | Primary server platform |
| Linux x64 checksum | `madhyamas-v{ver}-x86_64-unknown-linux-gnu.tar.gz.sha256` | `madhyamas-enterprise-v{ver}-x86_64-unknown-linux-gnu.tar.gz.sha256` | SHA-256 for verification |
| Linux ARM64 tarball | `madhyamas-v{ver}-aarch64-unknown-linux-gnu.tar.gz` | `madhyamas-enterprise-v{ver}-aarch64-unknown-linux-gnu.tar.gz` | ARM servers (Graviton, RPi) |
| Linux ARM64 checksum | `madhyamas-v{ver}-aarch64-unknown-linux-gnu.tar.gz.sha256` | `madhyamas-enterprise-v{ver}-aarch64-unknown-linux-gnu.tar.gz.sha256` | |
| Linux ARMv7 tarball | `madhyamas-v{ver}-armv7-unknown-linux-gnueabihf.tar.gz` | — | Enterprise: no demand on Pi 2/3 |
| Linux ARMv6 tarball | `madhyamas-v{ver}-arm-unknown-linux-gnueabihf.tar.gz` | — | Enterprise: no demand on Pi 1/Zero |
| Linux RISC-V tarball | `madhyamas-v{ver}-riscv64gc-unknown-linux-gnu.tar.gz` | — | Experimental; enterprise: no demand |
| macOS Intel tarball | `madhyamas-v{ver}-x86_64-apple-darwin.tar.gz` | `madhyamas-enterprise-v{ver}-x86_64-apple-darwin.tar.gz` | Developer workstations |
| macOS ARM tarball | `madhyamas-v{ver}-aarch64-apple-darwin.tar.gz` | `madhyamas-enterprise-v{ver}-aarch64-apple-darwin.tar.gz` | Apple Silicon workstations |
| Windows x64 zip | `madhyamas-v{ver}-x86_64-pc-windows-msvc.zip` | `madhyamas-enterprise-v{ver}-x86_64-pc-windows-msvc.zip` | Windows servers/workstations |
| Windows MSI installer | `madhyamas-v{ver}-x64.msi` | — | Simple tier only (package manager) |

**Platform count:** OSS = 8 targets, Enterprise = 5 targets.

#### Package manager artifacts (OSS only)

| Artifact | Source file in repo | OSS | Enterprise | Notes |
|---|---|---|---|---|
| Homebrew formula | `packaging/homebrew/madhyamas.rb` | Yes | — | `brew install madhyamas` |
| Chocolatey nuspec + install script | `packaging/windows/chocolatey/madhyamas/` | Yes | — | `choco install madhyamas` |
| Snap snapcraft.yaml | `packaging/linux/snap/madhyamas.snapcraft.yaml` | Yes | — | `snap install madhyamas` |
| RPM spec + service | `packaging/linux/rpm/madhyamas.spec` | Yes | — | `dnf install madhyamas` |

Enterprise users don't install via package managers — they download
binaries from GitHub Releases or use Docker.

#### Publishing destinations (where artifacts are pushed)

| Destination | Type | OSS | Enterprise | What gets pushed | Auth secret |
|---|---|---|---|---|---|
| GitHub Releases | GitHub asset upload | Yes | Yes | Binary tarballs/zips + checksums + MSI + Snap + RPM | `GITHUB_TOKEN` (automatic) |
| GHCR | Docker registry | Yes | Yes | Docker images (`madhyamas` + `madhyamas-enterprise`) | `GITHUB_TOKEN` (automatic) |
| Docker Hub | Docker registry | Yes | — | Docker image (`madhyamas`) | `DOCKERHUB_USERNAME` + `DOCKERHUB_TOKEN` |
| crates.io | Rust package registry | Yes | — | 5 crates (core, api, cli, mcp, main binary) | `CRATES_TOKEN` |
| Homebrew tap repo | Separate git repo (`{owner}/homebrew-tap`) | Yes | — | `Formula/madhyamas.rb` (version + SHA256 updated, committed, pushed) | `HOMEBREW_TAP_TOKEN` |
| Chocolatey community feed | Package feed (chocolatey.org) | Yes | — | `madhyamas.{ver}.nupkg` (packed, pushed) | `CHOCOLATEY_API_KEY` |
| Snap Store | Package store (snapcraft.io) | Yes | — | `madhyamas-v{ver}.snap` (uploaded, released to stable) | `SNAPCRAFT_TOKEN` |
| AWS ECR | Docker registry | — | — (licensing server only) | `madhyamas-license-server:{sha}` + `:latest` | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` |

**Homebrew tap repo** is a separate GitHub repository
(`{owner}/homebrew-tap`) that contains Homebrew formula files. The
release workflow clones it, copies the updated `madhyamas.rb` formula
(with version and SHA256 checksums filled in), commits, and pushes.
Users install via `brew tap {owner}/tap && brew install madhyamas`.

**Chocolatey community feed** is the public package repository at
`chocolatey.org`. The release workflow packs a `.nupkg` from the
nuspec + install script in `packaging/windows/chocolatey/`, then
pushes it to `push.chocolatey.org`. Users install via
`choco install madhyamas`.

Neither of these publishing destinations is used for the enterprise
tier. Enterprise binaries are not distributed through package managers
— they require a license file and are distributed via GitHub Release
downloads or Docker images only.

#### Docker images

| Image | OSS | Enterprise | Tags | Notes |
|---|---|---|---|---|
| GHCR (simple) | `ghcr.io/{org}/madhyamas` | — | `latest`, `v{ver}`, `{major}.{minor}` | Default; no enterprise code |
| GHCR (enterprise) | — | `ghcr.io/{org}/madhyamas-enterprise` | `latest`, `v{ver}`, `{major}.{minor}` | Built with `BUILD_ENTERPRISE=true` |
| Docker Hub (simple) | `{org}/madhyamas` | — | `latest`, `v{ver}` | If `DOCKERHUB_*` secrets configured |

#### crates.io publish

| Crate | OSS (published) | Enterprise (not published) | Notes |
|---|---|---|---|
| `madhyamas-core` | Yes (default features, no enterprise) | — | `cargo publish -p madhyamas-core` |
| `madhyamas-api` | Yes (`--no-default-features --features grpc,scripting,plugins`) | — | Enterprise feature excluded |
| `madhyamas-cli` | Yes | — | No enterprise dependency |
| `madhyamas-mcp` | Yes | — | No enterprise dependency |
| `madhyamas` (binary) | Yes (default features, no enterprise) | — | `cargo install madhyamas` gets OSS only |
| `madhyamas-enterprise` | — | Not published | Available via git dep or binary download |
| `madhyamas-plugin-sdk` | Yes | — | No enterprise dependency |

`cargo install madhyamas` always produces the OSS binary. Enterprise
binaries are only available via GitHub Releases, Docker, or git
dependency.

#### CI build artifacts (per push/PR, 3-day retention)

| Artifact name | Tier | Contents | Notes |
|---|---|---|---|
| `frontend-dist` | Shared | `web/dist/` | One frontend build, used by both tiers |
| `madhyamas-simple-{target}` | OSS | Binary tarball/zip + checksum | Per target |
| `madhyamas-enterprise-{target}` | Enterprise | Binary tarball/zip + checksum | Per target (5 targets only) |

#### GitHub Release (per tag `v*`)

| Item | OSS | Enterprise | Notes |
|---|---|---|---|
| Release name | `Madhyamas v{ver}` | (same release) | One release, both tiers |
| Binary assets | 8 platform tarballs/zips + checksums | 5 platform tarballs/zips + checksums | All attached to same release |
| MSI asset | `madhyamas-v{ver}-x64.msi` | — | Simple tier only |
| Snap asset | `madhyamas-v{ver}.snap` | — | Simple tier only |
| RPM asset | `madhyamas-{ver}.x86_64.rpm` | — | Simple tier only |
| Release notes | Includes OSS install instructions (Homebrew, Chocolatey, Snap, DNF) | Includes enterprise download table + license requirement notice | Single body, both sections |

#### Licensing server (separate repo, separate pipeline)

| Artifact | Repo | Notes |
|---|---|---|
| Docker image | `madhyamas-license-server` | `ECR: madhyamas-license-server:{sha}` and `:latest` |
| No binary release | — | Server-side app, not distributed as a binary |
| No crates.io publish | — | Proprietary, not published |

---

## 3. Principle: One Repo, Two Builds

The enterprise code lives in the same repository as the OSS code
(`crates/madhyamas-enterprise/`), but it is compiled only when
`--features enterprise` is passed. The CI/CD pipeline builds both
tiers from the same commit:

```mermaid
graph TD
    PUSH["Git push / PR"] --> CI["CI Workflow"]
    CI --> BUILD_FE["Build frontend<br/>(shared — same web/dist)"]
    BUILD_FE --> SIMPLE["Build simple binary<br/>cargo build -p madhyamas<br/>(no enterprise feature)"]
    BUILD_FE --> ENTERPRISE["Build enterprise binary<br/>cargo build -p madhyamas --features enterprise"]
    SIMPLE --> TEST_SIMPLE["Test simple binary<br/>cargo test (no enterprise)"]
    ENTERPRISE --> TEST_ENT["Test enterprise binary<br/>cargo test --features enterprise"]
    TEST_SIMPLE --> CHECK["All checks pass?"]
    TEST_ENT --> CHECK
    CHECK -->|Yes| MERGE["Merge / continue"]
    CHECK -->|No| FAIL["Fail CI"]
```

### Why not two repos?

Keeping enterprise code in the same repo (but behind a feature flag +
separate crate) has significant advantages over a separate repo:

- **Shared CI infrastructure.** One set of GitHub Actions workflows,
  one runner cache, one Docker build context.
- **Atomic changes.** A change to `madhyamas-api` that adds a new
  trait method can update the enterprise implementation in the same
  commit/PR. No cross-repo coordination.
- **Shared frontend.** Both tiers use the same `web/` folder (see
  [ENTERPRISE_WEB_UI.md](ENTERPRISE_WEB_UI.md)). One frontend build
  serves both.
- **Shared test infrastructure.** Integration tests can verify that
  the simple build works without enterprise code and that the
  enterprise build works with it.

The `madhyamas-enterprise` crate can carry a different license
(BSL/commercial) even though it's in the same repo. The license file
in `crates/madhyamas-enterprise/` applies only to that crate's source
files. The default build (`cargo build -p madhyamas`) does not compile
this crate, so the resulting binary is pure MIT/Apache.

---

## 4. CI Changes (ci.yml)

### 4.1 Frontend build — unchanged

The frontend build is shared between both tiers (see
[ENTERPRISE_WEB_UI.md §3](ENTERPRISE_WEB_UI.md#3-recommended-approach-same-folder-runtime-gated)).
One `npm run build` produces `web/dist/` that serves both tiers. No
change needed to the `build-frontend` job.

### 4.2 Rust checks — add enterprise matrix dimension

The current `rust-checks` job runs `cargo clippy --all-features` and
`cargo build` (default features). After the refactor, it must check
**both** feature sets:

```yaml
# .github/workflows/ci.yml — MODIFIED rust-checks job

  rust-checks:
    name: Rust ${{ matrix.rust }} on ${{ matrix.os }} (${{ matrix.tier }})
    needs: build-frontend
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable]
        tier: [simple, enterprise]
        include:
          - os: ubuntu-latest
            rust: beta
            tier: simple
          - os: ubuntu-latest
            rust: beta
            tier: enterprise
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.os }}-${{ matrix.rust }}-${{ matrix.tier }}

      - name: Download frontend artifact
        uses: actions/download-artifact@v4
        with:
          name: frontend-dist
          path: web/dist/

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Run clippy (simple tier)
        if: matrix.tier == 'simple'
        run: cargo clippy --all-targets -- -D warnings

      - name: Run clippy (enterprise tier)
        if: matrix.tier == 'enterprise'
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Build (simple tier)
        if: matrix.tier == 'simple'
        run: cargo build --verbose

      - name: Build (enterprise tier)
        if: matrix.tier == 'enterprise'
        run: cargo build --verbose --features enterprise

      - name: Install nextest
        uses: taiki-e/install-action@v2
        with:
          tool: cargo-nextest

      - name: Run tests (simple tier)
        if: matrix.tier == 'simple'
        env:
          RUST_BACKTRACE: 1
        run: cargo nextest run --verbose

      - name: Run tests (enterprise tier)
        if: matrix.tier == 'enterprise'
        env:
          RUST_BACKTRACE: 1
        run: cargo nextest run --verbose --features enterprise
```

### 4.3 Security audit — add enterprise dependency check

The security audit should check both feature sets:

```yaml
  security-audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-binstall
        uses: cargo-bins/cargo-binstall@main

      - name: Install cargo-audit
        run: cargo binstall cargo-audit --no-confirm

      - name: Run security audit (default features)
        run: cargo audit

      - name: Run security audit (enterprise features)
        run: cargo audit --features enterprise

      # ... npm audit unchanged ...
```

### 4.4 Docker build test — add enterprise image

```yaml
  docker-build:
    name: Docker Build Test (${{ matrix.tier }})
    runs-on: ubuntu-latest
    strategy:
      matrix:
        tier: [simple, enterprise]
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v4

      - name: Build Docker image (simple)
        if: matrix.tier == 'simple'
        uses: docker/build-push-action@v7
        with:
          context: .
          push: false
          tags: madhyamas:test-simple
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Build Docker image (enterprise)
        if: matrix.tier == 'enterprise'
        uses: docker/build-push-action@v7
        with:
          context: .
          push: false
          tags: madhyamas:test-enterprise
          build-args: |
            BUILD_ENTERPRISE=true
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

### 4.5 Cross-compile builds — add tier matrix

The `build-binaries` job currently builds one binary per target. After
the refactor, it builds two (simple + enterprise) per target:

```yaml
  build-binaries:
    name: Build madhyamas (${{ matrix.tier }}) for ${{ matrix.target }}
    needs: [rust-checks, build-frontend]
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          # ... existing target definitions ...
          # Add tier dimension to each:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            tier: simple
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            tier: enterprise
          # ... repeat for each target ...
    steps:
      # ... existing steps ...

      - name: Build binary (simple tier)
        if: matrix.tier == 'simple'
        run: cargo build --profile ci-release --target ${{ matrix.target }} -p madhyamas

      - name: Build binary (enterprise tier)
        if: matrix.tier == 'enterprise'
        run: cargo build --profile ci-release --target ${{ matrix.target }} -p madhyamas --features enterprise

      - name: Package (Unix — simple)
        if: matrix.os != 'windows-latest' && matrix.tier == 'simple'
        shell: bash
        run: |
          ARTIFACT_NAME="madhyamas-${{ steps.sha.outputs.SHORT_SHA }}-${{ matrix.target }}"
          mkdir -p dist/$ARTIFACT_NAME
          cp target/${{ matrix.target }}/ci-release/madhyamas dist/$ARTIFACT_NAME/
          cd dist
          tar -czvf $ARTIFACT_NAME.tar.gz $ARTIFACT_NAME
          sha256sum $ARTIFACT_NAME.tar.gz > $ARTIFACT_NAME.tar.gz.sha256

      - name: Package (Unix — enterprise)
        if: matrix.os != 'windows-latest' && matrix.tier == 'enterprise'
        shell: bash
        run: |
          ARTIFACT_NAME="madhyamas-enterprise-${{ steps.sha.outputs.SHORT_SHA }}-${{ matrix.target }}"
          mkdir -p dist/$ARTIFACT_NAME
          cp target/${{ matrix.target }}/ci-release/madhyamas dist/$ARTIFACT_NAME/
          cd dist
          tar -czvf $ARTIFACT_NAME.tar.gz $ARTIFACT_NAME
          sha256sum $ARTIFACT_NAME.tar.gz > $ARTIFACT_NAME.tar.gz.sha256

      - name: Upload build artifact
        uses: actions/upload-artifact@v4
        with:
          name: madhyamas-${{ matrix.tier }}-${{ matrix.target }}
          path: dist/madhyamas-*.*
          retention-days: 3
```

### 4.6 Docs check — unchanged

The docs check job validates internal links and source path references.
Enterprise docs are in `docs/ENTERPRISE_*.md` and are checked the same
way as existing docs. No change needed.

### 4.7 New: Enterprise-specific test job

Add a job that verifies the **structural isolation** — the simple build
must not contain enterprise code:

```yaml
  # =============================================================================
  # Verify simple build has no enterprise code
  # =============================================================================
  verify-isolation:
    name: Verify Tier Isolation
    needs: build-frontend
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          key: isolation-check

      - name: Download frontend artifact
        uses: actions/download-artifact@v4
        with:
          name: frontend-dist
          path: web/dist/

      - name: Build simple binary
        run: cargo build --release -p madhyamas

      - name: Verify enterprise crate is not compiled
        run: |
          # Check that madhyamas-enterprise does not appear in the dependency tree
          if cargo tree -p madhyamas | grep -q "madhyamas-enterprise"; then
            echo "::error::madhyamas-enterprise found in simple build dependency tree!"
            cargo tree -p madhyamas | grep "madhyamas-enterprise"
            exit 1
          fi
          echo "OK: madhyamas-enterprise is not in the simple build"

      - name: Verify enterprise symbols are not in the binary
        run: |
          # Check that enterprise-related symbols are not in the binary
          BINARY=target/release/madhyamas
          if strings "$BINARY" | grep -qi "madhyamas_enterprise"; then
            echo "::error::Enterprise symbols found in simple binary!"
            strings "$BINARY" | grep -i "madhyamas_enterprise" | head -5
            exit 1
          fi
          echo "OK: No enterprise symbols in simple binary"

      - name: Build enterprise binary
        run: cargo build --release -p madhyamas --features enterprise

      - name: Verify enterprise crate IS compiled (enterprise build)
        run: |
          if ! cargo tree -p madhyamas --features enterprise | grep -q "madhyamas-enterprise"; then
            echo "::error::madhyamas-enterprise NOT found in enterprise build dependency tree!"
            exit 1
          fi
          echo "OK: madhyamas-enterprise is in the enterprise build"
```

---

## 5. Release Changes (release.yml)

### 5.1 Build binaries — two tiers

The release workflow builds binaries for all platforms. After the
refactor, it builds **both** tiers and uploads them to the same GitHub
Release:

```yaml
  build-binaries:
    needs: test-gate
    name: Build madhyamas (${{ matrix.tier }}) for ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          # Simple tier — all platforms
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            tier: simple
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            cross: true
            cross_arch: aarch64
            tier: simple
          # ... (all other targets for simple tier) ...

          # Enterprise tier — key platforms only (see 5.2)
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            tier: enterprise
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            archive: tar.gz
            cross: true
            cross_arch: aarch64
            tier: enterprise
          - target: x86_64-apple-darwin
            os: macos-14
            archive: tar.gz
            tier: enterprise
          - target: aarch64-apple-darwin
            os: macos-latest
            archive: tar.gz
            tier: enterprise
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            archive: zip
            tier: enterprise

    steps:
      # ... existing setup steps ...

      - name: Build binary (simple)
        if: matrix.tier == 'simple'
        run: cargo build --release --target ${{ matrix.target }} -p madhyamas --features embedded-assets

      - name: Build binary (enterprise)
        if: matrix.tier == 'enterprise'
        run: cargo build --release --target ${{ matrix.target }} -p madhyamas --features enterprise,embedded-assets

      - name: Package (Unix — simple)
        if: matrix.os != 'windows-latest' && matrix.tier == 'simple'
        shell: bash
        run: |
          ARTIFACT_NAME="madhyamas-v${{ steps.version.outputs.VERSION }}-${{ matrix.target }}"
          # ... package as before ...

      - name: Package (Unix — enterprise)
        if: matrix.os != 'windows-latest' && matrix.tier == 'enterprise'
        shell: bash
        run: |
          ARTIFACT_NAME="madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-${{ matrix.target }}"
          mkdir -p dist/$ARTIFACT_NAME
          cp target/${{ matrix.target }}/release/madhyamas dist/$ARTIFACT_NAME/
          cd dist
          tar -czvf $ARTIFACT_NAME.tar.gz $ARTIFACT_NAME
          sha256sum $ARTIFACT_NAME.tar.gz > $ARTIFACT_NAME.tar.gz.sha256

      - name: Package (Windows — enterprise)
        if: matrix.os == 'windows-latest' && matrix.tier == 'enterprise'
        shell: pwsh
        run: |
          $ARTIFACT_NAME = "madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-${{ matrix.target }}"
          # ... package as before with enterprise name ...

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: madhyamas-${{ matrix.tier }}-${{ matrix.target }}
          path: dist/madhyamas-*
```

### 5.2 Enterprise tier — fewer platforms

The simple tier builds for **all 8 targets** (Linux x64/ARM64/ARMv7/ARMv6/RISC-V, macOS Intel/ARM, Windows x64). The enterprise tier builds for **5 key targets** only:

| Target | Simple | Enterprise | Why |
|---|---|---|---|
| x86_64-unknown-linux-gnu | Yes | Yes | Primary server platform |
| aarch64-unknown-linux-gnu | Yes | Yes | ARM servers (AWS Graviton, RPi) |
| armv7-unknown-linux-gnueabihf | Yes | No | Enterprise customers don't run on Pi 2/3 |
| arm-unknown-linux-gnueabihf | Yes | No | Enterprise customers don't run on Pi 1/Zero |
| riscv64gc-unknown-linux-gnu | Yes | No | Experimental — no enterprise demand |
| x86_64-apple-darwin | Yes | Yes | Developer workstations |
| aarch64-apple-darwin | Yes | Yes | Apple Silicon workstations |
| x86_64-pc-windows-msvc | Yes | Yes | Windows servers/workstations |

This reduces enterprise build time by ~40% and avoids maintaining
enterprise binaries for platforms nobody uses.

### 5.3 GitHub Release — both tiers in one release

Both simple and enterprise binaries are attached to the same GitHub
Release. The release notes clearly distinguish them:

```yaml
  create-release:
    name: Create GitHub Release
    needs: [build-binaries, build-msi, build-snap, build-rpm]
    runs-on: ubuntu-latest
    if: always() && needs.build-binaries.result == 'success'
    permissions:
      contents: write
    steps:
      # ... existing download steps ...

      - name: Collect release files
        run: |
          mkdir -p release
          # Simple tier binaries
          cp artifacts/madhyamas-v*.tar.gz artifacts/madhyamas-v*.sha256 release/ 2>/dev/null || true
          cp artifacts/madhyamas-v*.zip release/ 2>/dev/null || true
          # Enterprise tier binaries
          cp artifacts/madhyamas-enterprise-v*.tar.gz artifacts/madhyamas-enterprise-v*.sha256 release/ 2>/dev/null || true
          cp artifacts/madhyamas-enterprise-v*.zip release/ 2>/dev/null || true
          # Package managers (simple tier only)
          cp artifacts/*.msi release/ 2>/dev/null || true
          cp artifacts/*.snap release/ 2>/dev/null || true
          cp artifacts/*.rpm release/ 2>/dev/null || true

      - name: Create Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: v${{ steps.version.outputs.VERSION }}
          name: Madhyamas v${{ steps.version.outputs.VERSION }}
          body: |
            ## Madhyamas v${{ steps.version.outputs.VERSION }}

            **Two tiers available:**

            ### Simple (Open Source — MIT/Apache-2.0)
            No authentication, no RBAC, no license required. For solo developers and small teams.

            | Platform | Download |
            |----------|----------|
            | Linux x64 | `madhyamas-v${{ steps.version.outputs.VERSION }}-x86_64-unknown-linux-gnu.tar.gz` |
            | Linux ARM64 | `madhyamas-v${{ steps.version.outputs.VERSION }}-aarch64-unknown-linux-gnu.tar.gz` |
            | macOS Intel | `madhyamas-v${{ steps.version.outputs.VERSION }}-x86_64-apple-darwin.tar.gz` |
            | macOS Apple Silicon | `madhyamas-v${{ steps.version.outputs.VERSION }}-aarch64-apple-darwin.tar.gz` |
            | Windows x64 | `madhyamas-v${{ steps.version.outputs.VERSION }}-x86_64-pc-windows-msvc.zip` |

            Also available via: Homebrew, Chocolatey, Snap, DNF/YUM, Docker, `cargo install`.

            ### Enterprise (License required)
            Authentication, RBAC, audit logging, SSO, multi-user. Requires a license from madhyamas.ai/register.

            | Platform | Download |
            |----------|----------|
            | Linux x64 | `madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-x86_64-unknown-linux-gnu.tar.gz` |
            | Linux ARM64 | `madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-aarch64-unknown-linux-gnu.tar.gz` |
            | macOS Intel | `madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-x86_64-apple-darwin.tar.gz` |
            | macOS Apple Silicon | `madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-aarch64-apple-darwin.tar.gz` |
            | Windows x64 | `madhyamas-enterprise-v${{ steps.version.outputs.VERSION }}-x86_64-pc-windows-msvc.zip` |

            Also available via: Docker (`ghcr.io/.../madhyamas-enterprise:latest`).

            ### Changes

            ${{ steps.changelog.outputs.CHANGES }}
          files: release/*
          draft: false
          prerelease: ${{ contains(steps.version.outputs.VERSION, '-') }}
```

### 5.4 Package manager publishing — simple tier only

Homebrew, Chocolatey, Snap, and RPM packages are for the **simple tier
only**. Enterprise users download binaries directly or use Docker.

No change to the `publish-homebrew`, `publish-chocolatey`,
`publish-snap`, and `build-rpm` jobs — they already build the simple
binary. Just ensure they use the simple-tier artifact:

```yaml
  publish-homebrew:
    # ... existing job ...
    steps:
      # ... existing steps ...
      - name: Download release checksums (simple tier only)
        run: |
          mkdir -p checksums
          for target in x86_64-apple-darwin aarch64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
            curl -sL "https://github.com/${{ github.repository }}/releases/download/v${{ steps.version.outputs.VERSION }}/madhyamas-v${{ steps.version.outputs.VERSION }}-${target}.tar.gz.sha256" -o "checksums/madhyamas-${target}.sha256" || true
          done
      # ... rest unchanged ...
```

### 5.5 Docker publishing — two images

Publish both simple and enterprise Docker images:

```yaml
  publish-docker:
    name: Publish Docker Image (${{ matrix.tier }})
    needs: build-binaries
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    strategy:
      matrix:
        tier: [simple, enterprise]
    steps:
      - uses: actions/checkout@v4

      - name: Set up QEMU
        uses: docker/setup-qemu-action@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v4

      - name: Login to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract version
        id: version
        run: |
          VERSION=${RELEASE_VERSION:-${GITHUB_REF#refs/tags/v}}
          echo "VERSION=${VERSION}" >> $GITHUB_OUTPUT
          REPO_LOWER=$(echo "${{ github.repository }}" | tr '[:upper:]' '[:lower:]')
          echo "REPO_LOWER=${REPO_LOWER}" >> $GITHUB_OUTPUT

      - name: Build and push (simple)
        if: matrix.tier == 'simple'
        uses: docker/build-push-action@v7
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          tags: |
            ghcr.io/${{ steps.version.outputs.REPO_LOWER }}:latest
            ghcr.io/${{ steps.version.outputs.REPO_LOWER }}:${{ steps.version.outputs.VERSION }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Build and push (enterprise)
        if: matrix.tier == 'enterprise'
        uses: docker/build-push-action@v7
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          build-args: |
            BUILD_ENTERPRISE=true
          tags: |
            ghcr.io/${{ steps.version.outputs.REPO_LOWER }}-enterprise:latest
            ghcr.io/${{ steps.version.outputs.REPO_LOWER }}-enterprise:${{ steps.version.outputs.VERSION }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

### 5.6 crates.io publishing — exclude enterprise

After the refactor, `enterprise` is not in default features. The
crates.io publish uses default features (no enterprise). The
`madhyamas-enterprise` crate is **not published to crates.io** (it
carries a different license and is only available via git dependency
or binary download):

```yaml
  publish-crates:
    name: Publish to crates.io
    needs: build-binaries
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Check for CRATES_TOKEN
        id: check_token
        # ... existing check ...

      - name: Publish crates (simple tier — no enterprise)
        if: steps.check_token.outputs.has_token == 'true'
        env:
          CRATES_TOKEN: ${{ secrets.CRATES_TOKEN }}
        run: |
          # Publish in dependency order
          cargo publish -p madhyamas-core --token "$CRATES_TOKEN"
          sleep 30
          # No --features enterprise (it's not in default features anymore)
          cargo publish -p madhyamas-api --no-default-features --features grpc,scripting,plugins --token "$CRATES_TOKEN"
          sleep 30
          cargo publish -p madhyamas-cli --token "$CRATES_TOKEN"
          sleep 30
          cargo publish -p madhyamas-mcp --token "$CRATES_TOKEN"
          sleep 30
          cargo publish -p madhyamas --token "$CRATES_TOKEN"
          # NOTE: madhyamas-enterprise is NOT published to crates.io
          # It is available via:
          # - Binary download from GitHub Releases
          # - Docker image (ghcr.io/.../madhyamas-enterprise)
          # - Git dependency: madhyamas-enterprise = { git = "..." }
```

### 5.7 MSI/Snap/RPM — simple tier only

The `build-msi`, `build-snap`, and `build-rpm` jobs build packages for
the simple tier only. Enterprise users don't install via package
managers (they need a license file and typically run in Docker).

No change needed to these jobs — they already build the simple binary
(default features). Just ensure the artifact name doesn't collide with
enterprise artifacts.

---

## 6. Docker Changes

### 6.1 Dockerfile — build arg for tier

The Dockerfile uses a build arg to control whether enterprise code is
compiled:

```dockerfile
# Dockerfile — MODIFIED to support both tiers

# Frontend build stage (shared — same for both tiers)
FROM node:20-alpine AS frontend-builder
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Backend build stage
FROM rust:alpine AS builder

# Build arg: BUILD_ENTERPRISE=true compiles with --features enterprise
ARG BUILD_ENTERPRISE=false

RUN apk add --no-cache musl-dev openssl-dev openssl openssl-libs-static pkgconf build-base

WORKDIR /app

# Copy workspace files (including madhyamas-enterprise crate)
COPY Cargo.toml Cargo.lock ./
COPY crates/madhyamas/Cargo.toml ./crates/madhyamas/
COPY crates/madhyamas-core/Cargo.toml ./crates/madhyamas-core/
COPY crates/madhyamas-api/Cargo.toml ./crates/madhyamas-api/
COPY crates/madhyamas-cli/Cargo.toml ./crates/madhyamas-cli/
COPY crates/madhyamas-mcp/Cargo.toml ./crates/madhyamas-mcp/
COPY crates/madhyamas-plugin-sdk/Cargo.toml ./crates/madhyamas-plugin-sdk/
# Copy enterprise crate Cargo.toml (needed for enterprise build, harmless for simple)
COPY crates/madhyamas-enterprise/Cargo.toml ./crates/madhyamas-enterprise/

# Create dummy files to build dependencies
RUN mkdir -p crates/madhyamas/src crates/madhyamas-core/src crates/madhyamas-api/src \
    crates/madhyamas-cli/src crates/madhyamas-mcp/src crates/madhyamas-plugin-sdk/src \
    crates/madhyamas-enterprise/src
RUN echo "fn main() {}" > crates/madhyamas/src/main.rs
RUN echo "fn main() {}" > crates/madhyamas-core/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-api/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-cli/src/main.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-cli/src/lib.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-mcp/src/lib.rs
RUN echo "fn main() {}" > crates/madhyamas-mcp/src/main.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-plugin-sdk/src/lib.rs
RUN echo "pub fn dummy() {}" > crates/madhyamas-enterprise/src/lib.rs

# Copy web dist for rust-embed
COPY --from=frontend-builder /app/web/dist ./web/dist

# Build dependencies (conditional on tier)
RUN if [ "$BUILD_ENTERPRISE" = "true" ]; then \
      cargo build --release -p madhyamas --features enterprise --locked; \
    else \
      cargo build --release -p madhyamas --locked; \
    fi

# Copy actual source files
COPY crates/madhyamas/src ./crates/madhyamas/src
COPY crates/madhyamas-core/src ./crates/madhyamas-core/src
COPY crates/madhyamas-api/src ./crates/madhyamas-api/src
COPY crates/madhyamas-cli/src ./crates/madhyamas-cli/src
COPY crates/madhyamas-mcp/src ./crates/madhyamas-mcp/src
COPY crates/madhyamas-plugin-sdk/src ./crates/madhyamas-plugin-sdk/src
COPY crates/madhyamas-enterprise/src ./crates/madhyamas-enterprise/src
COPY crates/madhyamas-core/tests ./crates/madhyamas-core/tests

# Touch source files to invalidate cache
RUN find crates -name "*.rs" -exec touch {} \;

# Build the final binary (conditional on tier)
RUN if [ "$BUILD_ENTERPRISE" = "true" ]; then \
      cargo build --release -p madhyamas --features enterprise --locked; \
    else \
      cargo build --release -p madhyamas --locked; \
    fi

# Runtime stage
FROM alpine:3.19
RUN apk add --no-cache ca-certificates openssl
RUN addgroup -S madhyamas && adduser -S madhyamas -G madhyamas
WORKDIR /app
COPY --from=builder /app/target/release/madhyamas /usr/local/bin/madhyamas
RUN mkdir -p /data/certs /data/sessions && chown -R madhyamas:madhyamas /data
USER madhyamas
EXPOSE 3001 8888
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3001/health || exit 1
ENV MADHYAMAS_PROXY_PORT=8888
ENV MADHYAMAS_API_PORT=3001
ENV MADHYAMAS_DATA_DIR=/data
ENV MADHYAMAS_LOG_LEVEL=info
ENTRYPOINT ["madhyamas"]
CMD []
```

### 6.2 Building both images

```bash
# Simple tier (default — no build arg needed)
docker build -t madhyamas:simple .

# Enterprise tier
docker build -t madhyamas:enterprise --build-arg BUILD_ENTERPRISE=true .
```

### 6.3 Docker Compose — two services

```yaml
# docker-compose.yml — MODIFIED to support both tiers

services:
  madhyamas-simple:
    build:
      context: .
    ports:
      - "3001:3001"
      - "8888:8888"
    volumes:
      - madhyamas-data:/data
    environment:
      - MADHYAMAS_PROXY_PORT=8888
      - MADHYAMAS_API_PORT=3001

  madhyamas-enterprise:
    build:
      context: .
      args:
        BUILD_ENTERPRISE: "true"
    ports:
      - "3002:3001"  # Different host port to avoid conflict
      - "8889:8888"
    volumes:
      - madhyamas-enterprise-data:/data
      - ./license.json:/data/license.json:ro  # Mount license file
    environment:
      - MADHYAMAS_PROXY_PORT=8888
      - MADHYAMAS_API_PORT=3001
      - MADHYAMAS_ENABLE_AUTH=true
      - MADHYAMAS_LICENSE_FILE=/data/license.json
      - MADHYAMAS_DB_BACKEND=sqlite
      - MADHYAMAS_JWT_SECRET_FILE=/data/jwt-secret

volumes:
  madhyamas-data:
  madhyamas-enterprise-data:
```

### 6.4 Docker Hub / GHCR tags

| Image | Tag | Tier |
|---|---|---|
| `ghcr.io/<org>/madhyamas` | `latest`, `v0.1.6`, `0.1` | Simple |
| `ghcr.io/<org>/madhyamas-enterprise` | `latest`, `v0.1.6`, `0.1` | Enterprise |

---

## 7. Licensing Server CI/CD

The licensing server is in a **separate repository**
(`madhyamas-license-server`) with its own CI/CD pipeline. See
[ENTERPRISE_LICENSING_SERVER.md §15](ENTERPRISE_LICENSING_SERVER.md#15-portal-frontend)
for the full portal frontend and build design.

### Licensing server workflow

```yaml
# madhyamas-license-server/.github/workflows/ci.yml

name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16-alpine
        env:
          POSTGRES_DB: licensedb_test
          POSTGRES_USER: license
          POSTGRES_PASSWORD: testpass
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
      redis:
        image: redis:7-alpine
        ports:
          - 6379:6379
    env:
      DATABASE_URL: postgres://license:testpass@localhost:5432/licensedb_test
      REDIS_URL: redis://localhost:6379
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: web/package-lock.json

      # Rust checks
      - run: cargo fmt --all -- --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test --workspace

      # Frontend checks
      - run: cd web && npm ci
      - run: cd web && npm run typecheck
      - run: cd web && npm run lint
      - run: cd web && npm run build

  build-and-deploy:
    needs: test
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: aws-actions/configure-aws-credentials@v4
        with:
          aws-access-key-id: ${{ secrets.AWS_ACCESS_KEY_ID }}
          aws-secret-access-key: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
          aws-region: us-east-1
      - uses: aws-actions/amazon-ecr-login@v2
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: docker/Dockerfile
          push: true
          tags: |
            ${{ secrets.ECR_REGISTRY }}/madhyamas-license-server:latest
            ${{ secrets.ECR_REGISTRY }}/madhyamas-license-server:${{ github.sha }}
      - name: Deploy to ECS
        run: |
          aws ecs update-service \
            --cluster madhyamas \
            --service license-server \
            --force-new-deployment
```

### Key differences from the proxy CI/CD

| Aspect | Proxy CI/CD (this repo) | License server CI/CD (separate repo) |
|---|---|---|
| Triggers | Push/PR to main/develop, tags `v*` | Push/PR to main |
| Services | None (SQLite, no external deps) | PostgreSQL + Redis (test services) |
| Build matrix | 8 targets × 2 tiers | 1 target (linux/amd64 only) |
| Artifacts | Binaries + Docker images + packages | Docker image only |
| Publishing | GitHub Releases, Homebrew, Chocolatey, Snap, crates.io, Docker | ECR + ECS deploy |
| Secrets | CRATES_TOKEN, HOMEBREW_TAP_TOKEN, etc. | AWS credentials, Stripe key, SES key |
| Database migrations | None (SQLite, schema in code) | `sqlx::migrate!` runs at startup |

---

## 8. Secrets Management

### 8.1 Proxy repository secrets (this repo)

No new secrets needed for the two-tier build. The existing secrets
work for both tiers:

| Secret | Used by | Notes |
|---|---|---|
| `CRATES_TOKEN` | `publish-crates` | Publishes simple tier only (no enterprise) |
| `HOMEBREW_TAP_TOKEN` | `publish-homebrew` | Simple tier only |
| `CHOCOLATEY_API_KEY` | `publish-chocolatey` | Simple tier only |
| `SNAPCRAFT_TOKEN` | `publish-snap` | Simple tier only |
| `DOCKERHUB_USERNAME` / `DOCKERHUB_TOKEN` | `publish-docker` | Both tiers (two images) |
| `GITHUB_TOKEN` | `publish-docker`, `create-release` | Both tiers |

### 8.2 Licensing server repository secrets (separate repo)

| Secret | Purpose |
|---|---|
| `AWS_ACCESS_KEY_ID` | ECR push, ECS deploy, SES email |
| `AWS_SECRET_ACCESS_KEY` | ECR push, ECS deploy, SES email |
| `ECR_REGISTRY` | ECR registry URL |
| `STRIPE_SECRET_KEY` | Stripe API (live key in prod, test key in CI) |
| `STRIPE_WEBHOOK_SECRET` | Stripe webhook signature verification |
| `ED25519_LICENSE_PRIVATE_KEY` | License signing key (from Secrets Manager) |
| `SES_SMTP_PASSWORD` | Email sending |
| `JWT_SECRET` | Portal auth JWT signing |

**CI vs production secrets:** The CI pipeline uses test keys (Stripe
test mode, throw-away JWT secret, no real license signing). Production
secrets are injected at runtime via AWS Secrets Manager / ECS task
definition, not via GitHub Actions secrets.

### 8.3 Ed25519 license signing key

The license signing private key is the most critical secret. It is
**never** in GitHub Actions secrets. It is stored in AWS Secrets
Manager and injected into the ECS task at runtime:

```mermaid
graph TD
    SM["AWS Secrets Manager<br/>(Ed25519 private key)"]
    TASK["ECS Task Definition<br/>(references secret ARN)"]
    CONTAINER["License server container<br/>(reads key from env at startup)"]
    SIGNER["License Signer module<br/>(uses key to sign licenses)"]

    SM -->|"runtime injection"| TASK
    TASK -->|"env var"| CONTAINER
    CONTAINER --> SIGNER
```

CI tests use a **throw-away test key** (generated in CI, not the
production key). The test key's public key is embedded in test
builds of the proxy binary for integration testing.

---

## 9. Branch Strategy

### 9.1 Proxy repository (this repo)

The branch strategy doesn't change — both tiers are in the same repo:

| Branch | Purpose |
|---|---|
| `main` | Stable release branch. CI runs both tiers. Tags trigger releases. |
| `develop` | Integration branch (if used). CI runs both tiers. |
| Feature branches | Short-lived. CI runs both tiers (or just simple if enterprise is unaffected). |

### 9.2 Licensing server repository (separate repo)

| Branch | Purpose |
|---|---|
| `main` | Production. Deploys to ECS on push. |
| `staging` | Pre-production. Deploys to staging ECS. |
| Feature branches | Short-lived. CI runs tests but does not deploy. |

### 9.3 Coordinating releases

When a proxy release changes the license file format or the
revocation API, the licensing server must be updated in lockstep:

1. Update `license-core` in the licensing server repo to match the
   new format.
2. Update the proxy binary's embedded public key if rotating keys.
3. Test the integration: licensing server issues license → proxy
   binary verifies it.
4. Deploy licensing server first (it can issue both old and new
   format licenses during transition).
5. Release proxy binary (it verifies both old and new format via
   `issuer_key_id`).

This coordination is **manual** (not automated across repos). It's
only needed for license format changes, which are rare.

---

## 10. Release Cadence

### 10.1 Proxy binary

| Release type | Frequency | Both tiers? | Notes |
|---|---|---|---|
| Major (x.0.0) | As needed | Yes | Breaking changes |
| Minor (0.x.0) | Monthly | Yes | New features |
| Patch (0.0.x) | As needed | Yes | Bug fixes |
| Pre-release | As needed | Yes | Beta/RC builds |

Both tiers are released **from the same tag**. The version number is
the same for both (e.g., `v0.2.0` produces both
`madhyamas-v0.2.0-linux-x64.tar.gz` and
`madhyamas-enterprise-v0.2.0-linux-x64.tar.gz`).

### 10.2 Licensing server

| Release type | Frequency | Notes |
|---|---|---|
| Feature | Weekly | Deployed on push to main |
| Hotfix | As needed | Fast-tracked deploy |
| Database migration | As needed | Requires downtime window |

The licensing server is deployed independently of the proxy binary.
It can be updated daily without proxy releases.

### 10.3 Versioning

| Component | Version scheme | Example |
|---|---|---|
| Proxy binary (simple + enterprise) | Semver from `Cargo.toml` | `0.2.0` |
| Licensing server | Git SHA or semver | `1.0.0` or `abc1234` |
| License file format | Embedded in license payload | `"format_version": 1` |
| Revocation API | URL versioned | `/api/v1/license/verify` |

---

## 11. Implementation Phases

### Phase CICD-1: CI matrix changes (after Phase 0: crate extraction)

**Prerequisite:** `madhyamas-enterprise` crate exists, `enterprise`
removed from default features.

| Task | File |
|---|---|
| Add `tier` dimension to `rust-checks` matrix | `ci.yml` |
| Add tier-conditional clippy/build/test steps | `ci.yml` |
| Add `tier` dimension to `build-binaries` matrix | `ci.yml` |
| Add tier-conditional build/package steps | `ci.yml` |
| Add `verify-isolation` job | `ci.yml` |
| Add enterprise feature to `security-audit` | `ci.yml` |
| Add `tier` dimension to `docker-build` job | `ci.yml` |

**Effort:** Medium. The matrix changes are mechanical. The
`verify-isolation` job is new but straightforward.

### Phase CICD-2: Release workflow changes (after Phase 1: license verification)

| Task | File |
|---|---|
| Add `tier` dimension to `build-binaries` in release.yml | `release.yml` |
| Enterprise tier: 5 targets (not 8) | `release.yml` |
| Enterprise artifact naming: `madhyamas-enterprise-v*` | `release.yml` |
| Update `create-release` to include both tiers in release notes | `release.yml` |
| Update `publish-docker` to build two images | `release.yml` |
| Update `publish-crates` to exclude enterprise feature | `release.yml` |
| Verify Homebrew/Chocolatey/Snap use simple-tier artifacts | `release.yml` |

**Effort:** Medium. The release notes template needs updating. The
Docker build-arg approach is already documented.

### Phase CICD-3: Dockerfile changes (after Phase 0)

| Task | File |
|---|---|
| Add `BUILD_ENTERPRISE` build arg | `Dockerfile` |
| Conditional `cargo build` based on build arg | `Dockerfile` |
| Copy `crates/madhyamas-enterprise/` in Docker context | `Dockerfile` |
| Update `docker-compose.yml` with enterprise service | `docker-compose.yml` |

**Effort:** Small. The Dockerfile changes are minimal.

### Phase CICD-4: Licensing server CI/CD (parallel with Phase L1)

| Task | File (in license-server repo) |
|---|---|
| Create CI workflow (test + build) | `.github/workflows/ci.yml` |
| Add PostgreSQL + Redis test services | `.github/workflows/ci.yml` |
| Create deploy workflow (ECR + ECS) | `.github/workflows/ci.yml` |
| Multi-stage Dockerfile (web + Rust) | `docker/Dockerfile` |
| Configure AWS secrets | GitHub repo settings |

**Effort:** Medium. Standard CI/CD setup for a Rust + React app with
external services.

### Roadmap

```mermaid
gantt
    title CI/CD Implementation Phases
    dateFormat YYYY-MM-DD
    axisFormat %b %d

    section Proxy CI/CD
    Phase CICD-1: CI matrix changes       :c1, after p0, 5d
    Phase CICD-2: Release workflow        :c2, after p1, 5d
    Phase CICD-3: Dockerfile changes      :c3, after p0, 3d

    section License Server CI/CD
    Phase CICD-4: License server pipeline :c4, 2026-08-13, 7d
```

Phase CICD-1 and CICD-3 depend on Phase 0 (crate extraction) because
they require `enterprise` to be out of default features. Phase CICD-2
depends on Phase 1 (license verification) because the release notes
reference license requirements. Phase CICD-4 is independent and can
start immediately.

---

## See Also

- [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md) — Master document
- [Enterprise Licensing Server](ENTERPRISE_LICENSING_SERVER.md) — Licensing server design (including portal CI/CD in §15)
- [Enterprise Web UI](ENTERPRISE_WEB_UI.md) — Frontend design (shared web/ for both tiers)
- [Enterprise Storage Traits](ENTERPRISE_STORAGE_TRAITS.md) — Storage migration (affects test setup)
- [Enterprise Auth, RBAC, and IdP](ENTERPRISE_AUTH_RBAC.md) — Auth design (affects test matrix)
