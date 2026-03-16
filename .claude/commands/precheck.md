# Precheck - Run all CI checks and apply fixes

Run all GitHub Actions CI prechecks locally and apply automatic fixes where possible.

## Steps

### 1. Rust Format Check & Fix
Apply Rust formatting to all crates:

```bash
cargo fmt --all
```

### 2. Rust Clippy Lints
Run clippy with all targets and features. Fix any issues found:

```bash
cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged
```

If automatic fix fails, run clippy to see remaining issues:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. Rust Build
Build all crates to check for compilation errors:

```bash
cargo build
```

### 4. Rust Tests
Run all Rust tests:

```bash
cargo test
```

### 5. Frontend Lint
Run ESLint on the frontend and apply fixes:

```bash
cd web && npm run lint -- --fix
```

### 6. Frontend Build (includes TypeScript check)
Build the frontend which runs TypeScript compiler:

```bash
cd web && npm run build
```

### 7. Security Audit (optional)
Check for known vulnerabilities in dependencies:

```bash
cargo audit
cd web && npm audit --audit-level=high
```

## Summary

After running all checks, provide a summary of:
- Which checks passed
- Which checks failed and need manual attention
- Any fixes that were automatically applied
