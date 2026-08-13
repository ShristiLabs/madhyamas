# Enterprise Implementation Workflow

Shared reference for the enterprise agent pipeline. Load this before
starting any enterprise work to understand the agent chain, the phase
structure, and the handoff protocol.

## Agent Pipeline

```
                    enterprise-orchestrator
                           |
                           v
              enterprise-issues (GitHub issues)
                           |
                           v
              enterprise-developer (implements)
                           |
                    +------+------+
                    |             |
                    v             v
          enterprise-tester   enterprise-reviewer
          (unit tests)        (code review)
                    |             |
                    +------+------+
                           |
                           v
              enterprise-regression (build + test)
                           |
                           v
              enterprise-committer (commit)
                           |
                           v
                    (back to orchestrator
                     for next issue/phase)
```

## Phase Structure

The implementation plan (`docs/ENTERPRISE_IMPLEMENTATION_PLAN.md`)
defines 13 phases (0-12). Each phase has sub-phases and steps. The
orchestrator processes phases in dependency order:

```
Phase 0 → 1 → 2 → 5 → 6 → 10  (critical path)
              ↘ 3 → 4 → 7 → 9  (parallel)
                    ↘ 8        (parallel)
              ↘ 11             (parallel)
         3 → 12               (parallel)
```

## Issue Labels

GitHub issues created by `enterprise-issues` use these labels:

| Label | Meaning |
|---|---|
| `enterprise` | Part of the enterprise tier work |
| `phase:N` | Which implementation phase (0-12) |
| `priority:critical` | Blocks other work (critical path) |
| `priority:high` | Should be done soon |
| `priority:medium` | Normal priority |
| `priority:low` | Can be deferred |
| `agent:developer` | Needs implementation by enterprise-developer |
| `agent:tester` | Needs unit tests by enterprise-tester |
| `agent:reviewer` | Needs review by enterprise-reviewer |
| `agent:regression` | Needs regression check by enterprise-regression |
| `agent:committer` | Ready to commit by enterprise-committer |
| `status:blocked` | Blocked on another issue or external dependency |
| `status:in-progress` | Currently being worked on |
| `status:review` | Ready for review |
| `status:done` | Completed and committed |

## Handoff Protocol

Each agent produces a structured output that the next agent consumes:

### enterprise-issues output
```
ISSUE: #<number> — <title>
PHASE: <phase number>
PRIORITY: <critical|high|medium|low>
LABELS: enterprise, phase:N, priority:X, agent:developer
ACCEPTANCE: <bullet list of acceptance criteria>
APPROACH: <recommended approach with file references>
```

### enterprise-developer output
```
IMPLEMENTED: #<issue number>
FILES: <list of created/modified files>
CHANGES: <summary of changes>
BUILD: <pass/fail>
CLIPPY: <pass/warnings>
SURFACES: <which surfaces were updated: core/api/cli/mcp/web/docs>
REMAINING: <what still needs work>
```

### enterprise-tester output
```
TESTS: #<issue number>
FILES: <list of test files created/modified>
CASES: <number of test cases added>
COVERAGE: <percentage for changed modules>
RESULTS: <pass/fail count>
GAPS: <uncovered code paths>
```

### enterprise-reviewer output
```
REVIEW: #<issue number>
VERDICT: <approved|changes-requested|blocked>
FINDINGS:
  BLOCKER: <file:line — issue — fix>
  HIGH: <file:line — issue — fix>
  MEDIUM: <file:line — issue — fix>
  LOW: <file:line — issue — fix>
SURFACE_COVERAGE: <which surfaces are missing>
```

### enterprise-regression output
```
REGRESSION: #<issue number>
BUILD_OSS: <pass/fail>
BUILD_ENTERPRISE: <pass/fail>
TESTS: <pass/fail count>
CLIPPY: <pass/warnings>
FMT: <pass/fail>
DOCS_CHECK: <pass/fail>
REGRESSIONS: <list of any regressions found>
```

### enterprise-committer output
```
COMMITTED: #<issue number>
SHA: <commit hash>
MESSAGE: <commit message subject>
FILES: <number of files changed>
```

## Key Documents

| Document | Purpose |
|---|---|
| `docs/ENTERPRISE_IMPLEMENTATION_PLAN.md` | Master plan with 13 phases, steps, exit criteria |
| `docs/ENTERPRISE_OVERVIEW.md` | Architecture, crate structure, roadmap |
| `docs/ENTERPRISE_CRATE_MIGRATION.md` | Phase 1: crate extraction details |
| `docs/ENTERPRISE_STORAGE_TRAITS.md` | Phase 2: rusqlite to sqlx migration |
| `docs/ENTERPRISE_AUTH_RBAC.md` | Phase 4: auth, RBAC, audit design |
| `docs/ENTERPRISE_AI_AGENTS.md` | Phase 8: MCP/CLI auth, enterprise tools |
| `docs/ENTERPRISE_PERF_SECURITY.md` | Phase 9-10: security gaps, DB optimization |
| `docs/ENTERPRISE_MULTI_INSTANCE.md` | Phase 6: multi-instance, Redis, K8s |
| `docs/ENTERPRISE_WEB_UI.md` | Phase 7: web UI enterprise features |
| `docs/ENTERPRISE_CICD.md` | Phase 11: two-tier CI/CD |
| `docs/ENTERPRISE_LICENSING_SERVER.md` | Phase 12: licensing server |
| `docs/ENTERPRISE_OSS_COMPARISON.md` | Feature parity matrix |

## Build Verification Commands

```bash
# OSS build (no enterprise code)
cargo build --release --no-default-features -p madhyamas

# Enterprise build (default features include enterprise)
cargo build --release -p madhyamas

# All checks
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# Docs checks
bash scripts/check-docs.sh
bash scripts/check-docs-coverage.sh

# Frontend (must build before Rust due to rust-embed)
cd web && npm run build
```
