# Enterprise Baselines (Phase 0)

> **Historical document.** This baseline was recorded before the enterprise
> crate extraction. All phases are now complete. Retained for reference only.

> **Phase 0 — Preparation.** Verification-only record of the codebase state
> before the enterprise crate extraction (Phase 1). No source code was modified
> to produce these baselines.

- **Date recorded:** 2026-08-13
- **Git SHA:** `f064d2845da74a93bb7aaee2f58c673cb2d1322b`
- **Branch:** `enterprise-features`
- **Toolchain:** rustc 1.94.0 (4a4ef493e 2026-03-02), cargo 1.94.0 (85eff7c80 2026-01-15)
- **Platform:** macOS (aarch64-apple-darwin)

---

## 1. Build Verification Summary

| Check | Command | Result |
|---|---|---|
| Frontend build | `cd web && npm run build` | PASS (2002 modules, 2.13s) |
| Enterprise build (default features) | `cargo build --release -p madhyamas` | PASS (2m 30s) |
| OSS build (no default features) | `cargo build --release --no-default-features -p madhyamas` | **FAIL** (1 error) |
| Test suite | `cargo test --all-features` | PASS (491 passed, 0 failed, 6 ignored) |
| Clippy | `cargo clippy --all-targets --all-features -- -D warnings` | PASS (0 warnings) |
| Format check | `cargo fmt --all -- --check` | **FAIL** (2 diffs in 1 file) |

### OSS build failure (pre-existing, NOT fixed in Phase 0)

The `--no-default-features` build fails with a single compile error:

```
error[E0433]: failed to resolve: use of unresolved module or unlinked crate `tools_handlers`
  --> crates/madhyamas-api/src/routes.rs:62:17
   |
62 |             get(tools_handlers::get_traffic_script_traces),
   |                 ^^^^^^^^^^^^^^ use of unresolved module or unlinked crate `tools_handlers`
```

**Root cause:** `tools_handlers` module is gated behind
`#[cfg(any(feature = "grpc", feature = "scripting", feature = "plugins"))]`
in `crates/madhyamas-api/src/lib.rs:12`, but `routes.rs:62` references it
unconditionally. With `--no-default-features`, none of those features are
enabled, so the module is configured out and the reference fails to resolve.

**Action:** This is a pre-existing bug to be addressed in a later phase. It is
recorded here as an honest baseline — Phase 0 does not fix source code.

### Format check failure (pre-existing, NOT fixed in Phase 0)

`cargo fmt --all -- --check` reports 2 formatting diffs, both in
`crates/madhyamas-api/src/intercept_handlers.rs` (lines 355 and 1284).

**Action:** Recorded as baseline. Fixing formatting belongs to a later phase.

---

## 2. Binary Sizes

| Build | Command | Size (bytes) | Size (human) | Status |
|---|---|---|---|---|
| Enterprise (default features) | `cargo build --release -p madhyamas` | 30,044,528 | 28.65 MB | PASS |
| OSS (no default features) | `cargo build --release --no-default-features -p madhyamas` | N/A | N/A | FAIL (no binary produced) |

> The enterprise binary was copied to
> `target/release/madhyamas-enterprise-baseline` for string analysis before the
> OSS build overwrote `target/release/madhyamas`. The OSS build failed, so no
> OSS binary exists for comparison.

---

## 3. Enterprise String Counts

| Binary | `strings ... \| grep -c enterprise` (case-sensitive) | `strings ... \| grep -ic enterprise` (case-insensitive) |
|---|---|---|
| Enterprise (default features) | 5 | 8 |
| OSS (no default features) | N/A (build failed) | N/A (build failed) |

The case-sensitive matches in the enterprise binary include:
`Enterprise`, `enterprises`, `Enterprises`, and `Enterprise error: `.
The additional case-insensitive matches come from embedded web-asset strings
(e.g. `enterprise` appearing inside bundled JS/HTML dictionary data).

---

## 4. cfg Gate Count

| Pattern | Count |
|---|---|
| `#[cfg(feature = "enterprise")]` | 17 |
| `#[cfg(not(feature = "enterprise"))]` | 1 |
| **Total enterprise cfg gates** (`grep -rn 'cfg.*feature.*enterprise' crates/`) | **18** |

This matches the expected ~17 from the migration analysis. The 17 positive
gates plus 1 negative gate (`cfg(not(...))`) total 18 enterprise-related cfg
directives across `crates/madhyamas-core` and `crates/madhyamas-api`.

### Distribution by crate

| Crate | File | Gate count |
|---|---|---|
| madhyamas-core | `src/lib.rs` | 2 |
| madhyamas-api | `src/lib.rs` | 6 |
| madhyamas-api | `src/routes.rs` | 10 |
| **Total** | | **18** |

---

## 5. Test Results (`cargo test --all-features`)

| Crate / Test target | Passed | Failed | Ignored |
|---|---|---|---|
| madhyamas (main binary unit tests) | 10 | 0 | 0 |
| madhyamas-api (lib unit tests) | 0 | 0 | 0 |
| madhyamas-cli (lib unit tests) | 0 | 0 | 0 |
| madhyamas-cli (main binary unit tests) | 0 | 0 | 0 |
| madhyamas-core (lib unit tests) | 470 | 0 | 0 |
| madhyamas-core (wasm_plugin_integration) | 2 | 0 | 0 |
| madhyamas-mcp (lib unit tests) | 0 | 0 | 0 |
| madhyamas-mcp (main binary unit tests) | 0 | 0 | 0 |
| madhyamas-plugin-sdk (lib unit tests) | 3 | 0 | 0 |
| Doc-tests (madhyamas-api) | 0 | 0 | 4 |
| Doc-tests (madhyamas-cli) | 0 | 0 | 0 |
| Doc-tests (madhyamas-core) | 6 | 0 | 0 |
| Doc-tests (madhyamas-mcp) | 0 | 0 | 0 |
| Doc-tests (madhyamas-plugin-sdk) | 0 | 0 | 2 |
| **Total** | **491** | **0** | **6** |

All tests pass. Exit code 0.

---

## 6. Clippy Results

| Metric | Value |
|---|---|
| Command | `cargo clippy --all-targets --all-features -- -D warnings` |
| Warnings | 0 |
| Errors | 0 |
| Exit code | 0 (PASS) |

---

## 7. Format Check Results

| Metric | Value |
|---|---|
| Command | `cargo fmt --all -- --check` |
| Files with diffs | 1 (`crates/madhyamas-api/src/intercept_handlers.rs`) |
| Diff locations | 2 (lines 355, 1284) |
| Exit code | 0 (note: `cargo fmt --check` exits 0 even with diffs in this version; diffs are reported on stdout) |

---

## 8. Dependency Tree Audit

| Metric | Value |
|---|---|
| Command | `cargo tree -p madhyamas --features enterprise` |
| Total tree lines | 1,244 |
| Unique crate names | 375 |
| Top-level dependencies | 17 |

### Top-level dependencies of `madhyamas v0.1.6`

```
madhyamas v0.1.6
├── anyhow v1.0.104
├── axum v0.8.8
├── clap v4.6.0
├── madhyamas-api v0.1.6
├── madhyamas-cli v0.1.6
├── madhyamas-core v0.1.6
├── madhyamas-mcp v0.1.6
├── parking_lot v0.12.5
├── reqwest v0.13.2
├── rusqlite v0.31.0
├── rustls v0.23.37
├── serde v1.0.228
├── serde_json v1.0.149
├── tokio v1.50.0
├── tower-http v0.6.8
├── tracing v0.1.44
└── tracing-subscriber v0.3.23
```

The full dependency tree (1,244 lines) is saved alongside this document at
[`docs/enterprise-baseline-cargo-tree.txt`](enterprise-baseline-cargo-tree.txt).

---

## 9. Frontend Build

| Metric | Value |
|---|---|
| Command | `cd web && npm run build` |
| Modules transformed | 2,002 |
| Build time | 2.13s |
| Largest asset | `ScriptCodeEditor-eLGG65G6.js` (616.65 kB, 212.85 kB gzip) |
| Exit code | 0 (PASS) |

---

## 10. Issues Identified (for later phases)

| # | Issue | Severity | Phase to address |
|---|---|---|---|
| 1 | OSS build (`--no-default-features`) fails: `tools_handlers` unresolved in `routes.rs:62` | High | Phase 1 (crate extraction) or a pre-Phase-1 fix |
| 2 | `cargo fmt --check` reports 2 diffs in `intercept_handlers.rs` | Low | Any phase (trivial `cargo fmt`) |

These are pre-existing issues recorded honestly as baselines. Phase 0 does not
modify source code.

---

## How to Reproduce

```bash
# Frontend (must run first — assets are embedded via rust-embed)
cd web && npm run build

# Enterprise build (default features include enterprise)
cargo build --release -p madhyamas
ls -la target/release/madhyamas

# OSS build (no default features)
cargo build --release --no-default-features -p madhyamas

# Tests
cargo test --all-features

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Format check
cargo fmt --all -- --check

# Dependency tree
cargo tree -p madhyamas --features enterprise

# Enterprise string count
strings target/release/madhyamas | grep -c enterprise

# cfg gate count
grep -rn 'cfg.*feature.*enterprise' crates/ | wc -l
```
