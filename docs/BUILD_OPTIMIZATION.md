# Build Optimization Proposal

**Status:** Proposal — no changes applied yet
**Date:** 2026-07-22
**Scope:** CI snapshot builds (`ci.yml`), release builds (`release.yml`), and Docker builds

---

## Current State Analysis

### What gets built

| Artifact | Count | Notes |
|----------|-------|-------|
| Rust crates | 5 workspace members | 31,097 lines of Rust |
| External deps | 426 crates in `Cargo.lock` | Heavy: `rusqlite/bundled`, `ring`, `brotli`, `reqwest`, `aws-lc-sys` |
| CI matrix targets | 8 platforms | 5 Linux (x86_64, aarch64, armv7, armv6, riscv64) + 2 macOS + 1 Windows |
| Frontend | React/TS via Vite | `tsc --noEmit` + `vite build`, ~848KB dist |

### Current release profile

```toml
[profile.release]
lto = true          # full LTO — very slow link step
strip = true        # fine, cheap
codegen-units = 1   # maximum optimization, minimum parallelism
```

### Key bottlenecks identified

1. **`lto = true` (fat LTO)** — the single biggest time sink. Full LTO re-analyzes all 426 crates' IR in a single linker pass. For a 426-dep project this adds 5–15 minutes per target.
2. **`codegen-units = 1`** — forces single-threaded codegen, compounding with LTO. On GitHub's 4-core runners this leaves 3 cores idle during the final codegen phase.
3. **`rusqlite` with `bundled`** — compiles SQLite from C source (~30K lines of C) on every target, every run. No shared cache across targets.
4. **`aws-lc-sys`** — pulled in transitively by `rustls` via `ring`/`aws-lc-rs`. Compiles a large C/C++ crypto library. Very slow on cross-compile targets.
5. **`brotli` crate (v6)** — full Brotli compressor + decompressor in C++. Only the decompressor is needed for response body decoding.
6. **`tokio` with `features = ["full"]`** — pulls in `tokio::process`, `tokio::signal`, `tokio::net::tcp` etc. The proxy only needs `rt-multi-thread`, `net`, `io-util`, `macros`, `time`, `sync`.
7. **`hyper` with `features = ["full"]`** — same issue; only `http1`, `http2`, `server`, `client` are needed.
8. **Frontend built 10+ times per CI run** — `npm ci && npm run build` runs in `rust-checks` (4 matrix jobs), `build-binaries` (8 jobs), `release.yml` test-gate, `build-rpm`, and Docker. The `web/dist` output is identical every time.
9. **`tsc --noEmit` before `vite build`** — runs full TypeScript type-checking on every frontend build. Vite already uses esbuild which is ~100x faster; the `tsc` step is redundant for snapshot builds.
10. **No incremental compilation** — `CARGO_INCREMENTAL` is off by default in CI (correct for cache hygiene), but the dependency layer is rebuilt from scratch whenever `Cargo.lock` changes even slightly.
11. **All 8 targets build sequentially per-matrix but the matrix runs in parallel** — good, but each runner rebuilds the full dep tree from cache misses.
12. **`cargo build --release` in Dockerfile builds ALL crates** — not just `-p madhyamas`, wasting time compiling dev-deps and test targets.

### Estimated time breakdown (per target, cold cache)

| Phase | Est. time | Notes |
|-------|-----------|-------|
| Frontend build (`npm ci` + `tsc` + `vite`) | 60–90s | Repeated 10+ times |
| Rust dependency compile | 8–15 min | `rusqlite`, `ring`, `aws-lc-sys`, `brotli` dominate |
| Rust application compile (LTO + codegen=1) | 5–10 min | Single-threaded codegen |
| Packaging | 10–20s | Negligible |
| **Total per target (cold)** | **~15–25 min** | |
| **Total per target (warm cache)** | **~8–12 min** | LTO + codegen still slow |

With 8 parallel targets, wall-clock is ~25 min cold, ~12 min warm. The LTO/link step is not cacheable.

---

## Proposed Optimizations

### Tier 1: High impact, low risk (do first)

#### 1.1 — Relax the release profile for snapshot/CI builds

**Problem:** `lto = true` + `codegen-units = 1` is optimized for binary size and runtime performance, not build speed. Snapshot builds are for testing, not production distribution.

**Proposal:** Add a CI-specific profile that uses thin LTO and higher codegen-units:

```toml
[profile.release]
lto = true
strip = true
codegen-units = 1

# New: fast profile for CI snapshots
[profile.ci-release]
inherits = "release"
lto = "thin"          # thin LTO is ~3-5x faster than fat LTO
codegen-units = 16    # parallel codegen
opt-level = 2         # slightly less aggressive than 3, much faster
```

CI builds use `cargo build --profile ci-release` instead of `--release`.
Release/tagged builds keep the strict profile.

**Estimated savings:** 5–10 min per target (LTO + codegen phase).

#### 1.2 — Build frontend once, share across all jobs

**Problem:** `npm ci && npm run build` runs 10+ times per CI run, producing identical `web/dist` each time.

**Proposal:** Add a dedicated `build-frontend` job that builds `web/dist` once and uploads it as an artifact. All downstream jobs (`rust-checks`, `build-binaries`, `docker-build`) download it:

```yaml
build-frontend:
  name: Build frontend
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: "22"
        cache: "npm"
        cache-dependency-path: web/package-lock.json
    - working-directory: web
      run: npm ci && npm run build
    - uses: actions/upload-artifact@v4
      with:
        name: web-dist
        path: web/dist/
```

Downstream jobs:
```yaml
- uses: actions/download-artifact@v4
  with:
    name: web-dist
    path: web/dist/
```

**Estimated savings:** Eliminates ~9 redundant frontend builds. ~6–9 min total CI time saved.

#### 1.3 — Skip `tsc --noEmit` in snapshot builds

**Problem:** `npm run build` runs `tsc --noEmit && vite build`. The `tsc` step is a full type-check that takes 20–40s. Vite uses esbuild (already type-strips). Type-checking belongs in the `frontend-checks` job, not in every binary build.

**Proposal:** Add a `build:fast` script to `web/package.json`:

```json
"build:fast": "vite build",
"build": "tsc --noEmit && vite build"
```

Binary build jobs use `npm run build:fast`. The `frontend-checks` job keeps `npm run build` (with `tsc`).

**Estimated savings:** 20–40s × 10 builds = ~3–7 min total.

#### 1.4 — Use `Swatinem/rust-cache` instead of `actions/cache`

**Problem:** The current `actions/cache@v5` setup caches `~/.cargo/registry`, `~/.cargo/git`, and `target` as a single blob. Cache misses are frequent because the key includes the full `Cargo.lock` hash — any dep bump invalidates everything. Also, `target` is huge (GBs) and slow to save/restore.

**Proposal:** Replace with `Swatinem/rust-cache@v2`, which:
- Caches `~/.cargo/registry` and `target` separately with smarter keying
- Supports `shared-key` and `save-if` parameters
- Handles cross-compile targets correctly
- Is the de facto standard for Rust CI caching

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    workspaces: "."
    key: ${{ matrix.target }}
```

**Estimated savings:** Better cache hit rates, ~2–5 min on warm runs. Faster save/restore.

---

### Tier 2: Medium impact, moderate effort

#### 2.1 — Gate optional dependencies behind features properly

**Problem:** `jsonwebtoken`, `sysinfo`, and `brotli` are listed as unconditional dependencies in `madhyamas-core/Cargo.toml`, but `jsonwebtoken` is only used in the `enterprise` feature module (`src/enterprise/auth.rs`). The `enterprise` feature is `default = [...]` so it always compiles, but the dependency should be optional and mapped via the feature.

**Proposal:**

```toml
# crates/madhyamas-core/Cargo.toml
[dependencies]
# ... existing deps ...
# Remove these from unconditional [dependencies]:
# jsonwebtoken.workspace = true   ← move to optional

[features]
default = ["grpc", "scripting", "plugins", "enterprise"]
enterprise = ["dep:jsonwebtoken"]   # only compiled when enterprise is on

[dependencies]
jsonwebtoken = { workspace = true, optional = true }
```

Same pattern for `sysinfo` (only used in `performance/monitor.rs`) and `brotli` (only needed for response decompression, could be behind a `decompression` feature).

**Impact:** When building without enterprise features (e.g., a minimal snapshot), `jsonwebtoken` and its transitive deps (`ring`/`aws-lc` parts) are skipped. Even with default features on, this is correct dependency hygiene.

**Estimated savings:** ~30–60s per target when enterprise is off; better cache isolation.

#### 2.2 — Narrow `tokio` and `hyper` feature flags

**Problem:** `tokio = { features = ["full"] }` and `hyper = { features = ["full"] }` pull in everything including `tokio::process`, `tokio::signal`, `tokio::fs`, etc.

**Proposal:** Audit actual usage and narrow to needed features:

```toml
# tokio: likely needs rt-multi-thread, net, io-util, macros, time, sync, parking_lot
tokio = { version = "1.36", features = ["rt-multi-thread", "net", "io-util", "macros", "time", "sync"] }

# hyper: needs http1, http2, server, client
hyper = { version = "1.2", features = ["http1", "http2", "server", "client"] }
```

**Estimated savings:** ~30–90s per target (fewer crates compiled). Needs testing to confirm no missing features.

#### 2.3 — Use `cargo-nextest` for faster test execution

**Problem:** `cargo test --verbose` compiles test binaries for all crates and runs them serially per test binary.

**Proposal:** Use `cargo-nextest` (pre-built binary via `cargo-binstall`):

```yaml
- name: Install nextest
  run: cargo binstall cargo-nextest --no-confirm
- name: Run tests
  run: cargo nextest run --profile ci
```

Nextest runs tests in parallel, has better failure isolation, and is 2–3x faster.

**Estimated savings:** ~1–2 min on test suite. Tests are a smaller fraction of total time.

#### 2.4 — Docker: build only the needed package

**Problem:** Dockerfile line 45 runs `cargo build --release` (all crates) for the dependency pre-build, then `cargo build --release -p madhyamas` for the real build.

**Proposal:** Use `-p madhyamas` in both steps and add `--locked`:

```dockerfile
RUN cargo build --release -p madhyamas --locked
# ... copy sources ...
RUN cargo build --release -p madhyamas --locked
```

Also add `.dockerignore` for `target/` to prevent leaking local build cache into the context.

**Estimated savings:** ~30–60s (skips compiling test-only deps).

---

### Tier 3: Advanced, higher effort

#### 3.1 — Use `cargo-chef` for Docker layer caching

**Problem:** The current Dockerfile uses a dummy-source trick to cache deps, but it's fragile and doesn't handle `Cargo.lock` changes well.

**Proposal:** Use `cargo-chef` which generates a recipe from `Cargo.toml`/`Cargo.lock` and builds deps in a dedicated layer:

```dockerfile
FROM rust:alpine AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p madhyamas
```

**Estimated savings:** Dramatically better Docker cache hit rate. ~5–10 min on cache hits.

#### 3.2 — Consider `cranelift` backend for faster codegen (experimental)

**Problem:** LLVM codegen is the slowest part of Rust compilation.

**Proposal:** For CI snapshot builds only, use the `cranelift` backend via `RUSTFLAGS`:

```yaml
env:
  RUSTFLAGS: "-Ccodegen-backend=cranelift"
```

Requires a nightly toolchain with `cranelift` component. Codegen is 2–3x faster but produces slower binaries (fine for snapshots).

**Estimated savings:** Potentially 3–5 min per target. Risk: nightly-only, some crates may not compile.

#### 3.3 — Reduce cross-compile targets for snapshot builds

**Problem:** Every CI run builds 8 targets. ARMv6, ARMv7, and RISC-V are niche platforms.

**Proposal:** Build only x86_64 + aarch64 (Linux + macOS) + Windows x64 on every push (5 targets). Build the remaining 3 niche targets only on tagged releases.

```yaml
# ci.yml — snapshot (every push)
matrix:
  include:
    - target: x86_64-unknown-linux-gnu
    - target: aarch64-unknown-linux-gnu
    - target: x86_64-apple-darwin
    - target: aarch64-apple-darwin
    - target: x86_64-pc-windows-msvc

# release.yml — tagged releases (all 8 targets)
```

**Estimated savings:** 3 fewer matrix jobs per CI run. ~6–9 min wall-clock (parallel), ~20 min total compute.

#### 3.4 — Use `sccache` for cross-job compilation caching

**Problem:** Even with `rust-cache`, each runner compiles deps independently. `sccache` caches compiled artifacts by content hash and can share across runners via GitHub Actions cache.

**Proposal:**

```yaml
- name: Install sccache
  run: cargo binstall sccache --no-confirm
- name: Configure sccache
  run: |
    echo "RUSTC_WRAPPER=sccache" >> $GITHUB_ENV
    echo "SCCACHE_GHA_ENABLED=true" >> $GITHUB_ENV
- run: cargo build --profile ci-release
- name: Save sccache stats
  run: sccache --show-stats
```

**Estimated savings:** ~2–5 min on warm runs. Most impactful for cross-compile targets that share most deps.

---

## Summary: Estimated Impact

| Optimization | Tier | Per-target savings | CI-wide savings | Risk |
|-------------|------|-------------------|-----------------|------|
| Thin LTO + codegen=16 profile | 1 | 5–10 min | 5–10 min | Low |
| Build frontend once | 1 | — | 6–9 min | Low |
| Skip tsc in binary builds | 1 | 20–40s | 3–7 min | Low |
| `Swatinem/rust-cache` | 1 | 2–5 min | 2–5 min | Low |
| Gate optional deps behind features | 2 | 30–60s | 30–60s | Low |
| Narrow tokio/hyper features | 2 | 30–90s | 30–90s | Medium |
| `cargo-nextest` | 2 | — | 1–2 min | Low |
| Docker: `-p madhyamas` + `.dockerignore` | 2 | 30–60s | 30–60s | Low |
| `cargo-chef` for Docker | 3 | — | 5–10 min (Docker) | Medium |
| Cranelift backend | 3 | 3–5 min | 3–5 min | High (nightly) |
| Reduce snapshot targets | 3 | — | 6–9 min wall-clock | Low |
| `sccache` | 3 | 2–5 min | 2–5 min | Medium |

### Combined estimate

| Scenario | Current | After Tier 1 | After Tier 1+2 | After all tiers |
|----------|---------|-------------|---------------|----------------|
| Cold cache, per target | 15–25 min | 8–15 min | 7–12 min | 5–10 min |
| Warm cache, per target | 8–12 min | 3–6 min | 2–5 min | 2–4 min |
| Total CI wall-clock (8 targets) | ~25 min | ~12 min | ~10 min | ~6–8 min |

### Recommended action plan

1. **Start with Tier 1** (all four items) — low risk, high impact, ~50% time reduction
2. **Then Tier 2** — moderate effort, additional ~15% reduction
3. **Tier 3 as needed** — target the biggest remaining bottleneck

The single highest-ROI change is **1.1 (thin LTO + parallel codegen for CI)** combined with **1.2 (build frontend once)**. Together they cut CI time roughly in half with minimal risk.
