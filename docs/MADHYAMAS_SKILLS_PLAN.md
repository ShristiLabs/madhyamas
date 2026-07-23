# Madhyamas Skills — Build & Publish Plan

> **Status:** Planning
> **Date:** 2026-07-22
> **Goal:** Publish a skills package that gives AI agents procedural knowledge about how to use the Madhyamas debugging proxy — its MCP tools, CLI commands, and REST API — across all major AI agent harnesses.

---

## 1. Overview

### 1.1 What Are Madhyamas Skills?

Madhyamas Skills is a **single skill package** (with progressive-disclosure reference files) that teaches AI agents how to:

- **Set up** Madhyamas (install, configure proxy, install CA cert, connect clients)
- **Inspect traffic** (filter, search, analyze captured HTTP/HTTPS/WS/gRPC traffic)
- **Manipulate traffic** (mocks, breakpoints, rewrites, throttling, replay)
- **Manage sessions** (create, switch, export, import)
- **Automate workflows** (scripting, plugins, persistence, HAR export)
- **Troubleshoot** (cert pinning, port conflicts, database locks, connection issues)

The skill provides procedural guidance across **three interfaces**:
1. **MCP tools** (68 tools) — for Claude Desktop, Windsurf, and other MCP-compatible agents
2. **CLI commands** (56 subcommands) — for shell-based agents
3. **REST API** (130+ endpoints) — for agents making direct HTTP calls

### 1.2 Why a Single Skill?

A single skill with progressive disclosure:
- **One trigger** — agents load it whenever Madhyamas is relevant
- **Lean entry point** — SKILL.md stays under 500 lines; details load on demand
- **Cross-referenced** — reference files link to each other for related workflows
- **Easy to install** — one directory to copy/symlink

### 1.3 Target Harnesses

| Harness | Format | Directory |
|----------|--------|-----------|
| Claude Code | SKILL.md + frontmatter | `.claude/skills/madhyamas/` |
| Devin CLI | SKILL.md + frontmatter | `.devin/skills/madhyamas/` or `.agents/skills/madhyamas/` |
| Windsurf | SKILL.md + frontmatter | `.windsurf/skills/madhyamas/` |
| Cursor | `.mdc` + frontmatter | `.cursor/rules/madhyamas.mdc` |
| OpenCode | SKILL.md + frontmatter | `.opencode/skills/madhyamas/` |
| CommandCode | SKILL.md + frontmatter | `.commandcode/skills/madhyamas/` |
| Generic/Agent Skills standard | SKILL.md + frontmatter | `.agents/skills/madhyamas/` |

All formats share a common denominator: **Markdown with YAML frontmatter** containing `name` and `description`. The build system transforms the canonical source into each target format.

---

## 2. Source Skill Structure

### 2.1 Canonical Source Location

```
madhyamas/
└── skills/
    └── madhyamas/
        ├── SKILL.md                    # Entry point (<500 lines)
        ├── references/                 # Progressive disclosure docs
        │   ├── setup.md                # Installation & configuration
        │   ├── mcp-tools.md            # Complete MCP tool reference (68 tools)
        │   ├── cli-commands.md         # Complete CLI command reference (56 subcommands)
        │   ├── rest-api.md             # Complete REST API reference (130+ endpoints)
        │   ├── traffic-inspection.md   # Workflows: filter, search, analyze traffic
        │   ├── mocking.md              # Workflows: create, manage, test mocks
        │   ├── breakpoints.md          # Workflows: pause & modify traffic
        │   ├── rewrites.md             # Workflows: URL/header/body rewriting
        │   ├── throttling.md           # Workflows: network simulation
        │   ├── replay.md               # Workflows: replay & save requests
        │   ├── sessions.md             # Workflows: session management
        │   ├── grpc.md                 # Workflows: gRPC inspection
        │   ├── scripting.md            # Workflows: JS/TS scripts
        │   ├── plugins.md              # Workflows: plugin management
        │   ├── websockets.md           # Workflows: WebSocket traffic inspection
        │   ├── export-import.md        # Workflows: HAR export, cURL, persistence
        │   ├── troubleshooting.md      # Common issues & solutions
        │   └── harness-setup.md        # Per-harness MCP configuration guides
        ├── scripts/                    # Executable helpers
        │   ├── build.sh                # Multi-target build script
        │   ├── install.sh              # Install to a specific harness
        │   └── validate.py             # Validate skill structure
        └── assets/                     # Non-context files
            ├── mcp-config-claude.json  # Claude Desktop MCP config template
            ├── mcp-config-windsurf.json # Windsurf MCP config template
            └── mcp-config-generic.json # Generic MCP config template
```

### 2.2 SKILL.md Design

The SKILL.md is the **only file always loaded** when the skill triggers. It must be concise (<500 lines) and contain:

1. **Frontmatter** — `name`, `description` (with comprehensive trigger phrases)
2. **Quick Start** — 3-step setup (install, configure, verify)
3. **Interface Selection Guide** — which interface to use (MCP vs CLI vs REST API)
4. **Core Workflows** — brief overviews with links to reference files
5. **Reference Index** — table of all reference files and when to read them

#### Frontmatter (Canonical Source)

```yaml
---
name: madhyamas
description: >
  Procedural knowledge for using Madhyamas, an open-source HTTP/HTTPS debugging
  proxy built in Rust. Use this skill when: (1) debugging HTTP/HTTPS API traffic,
  (2) mocking API responses, (3) setting breakpoints on requests/responses,
  (4) rewriting URLs/headers/bodies, (5) throttling network conditions,
  (6) replaying captured requests, (7) inspecting WebSocket or gRPC traffic,
  (8) exporting traffic as HAR or cURL, (9) managing debugging sessions,
  (10) configuring MCP server for AI agent integration, (11) troubleshooting
  proxy/TLS/certificate issues, (12) using madhyamas CLI commands, or
  (13) calling the Madhyamas REST API. Covers MCP tools (68 tools), CLI
  commands (56 subcommands), and REST API (130+ endpoints).
license: MIT OR Apache-2.0
metadata:
  author: madhyamas
  version: "0.1.0"
  project-url: https://github.com/madhyamas/madhyamas
---
```

#### Body Structure (Outline)

```markdown
# Madhyamas Proxy — AI Agent Guide

## Quick Start
1. Install: `cargo install madhyamas` or download binary
2. Start: `madhyamas serve` (proxy on :8888, API/UI on :3001)
3. Verify: `curl http://localhost:3001/api/health`

## Choosing an Interface
| Interface | When to use | Setup |
|-----------|-------------|-------|
| MCP tools | You're in an MCP-compatible agent (Claude, Windsurf) | `madhyamas mcp` |
| CLI | You have shell access, prefer commands | `madhyamas traffic list` |
| REST API | You need fine-grained control, custom scripts | `curl http://localhost:3001/api/...` |

## Core Workflows
- **Inspect traffic**: See references/traffic-inspection.md
- **Mock responses**: See references/mocking.md
- **Set breakpoints**: See references/breakpoints.md
- ... (full index table)

## Reference Index
| File | When to read |
|------|-------------|
| references/setup.md | Installing, configuring, connecting clients |
| references/mcp-tools.md | Using MCP tools (full 68-tool reference) |
| ... | ... |
```

### 2.3 Reference File Design

Each reference file follows this pattern:

```markdown
# [Domain Title]

## Overview
Brief description of what this domain covers.

## MCP Tools
Table of relevant MCP tools with parameters.

## CLI Commands
Table of relevant CLI commands with flags.

## REST API
Table of relevant endpoints with methods/paths.

## Workflows
Step-by-step procedures for common tasks.

## Examples
Concrete examples for MCP, CLI, and REST API.
```

This triple-pattern (MCP/CLI/REST) in each reference file ensures agents can use whichever interface they have available.

---

## 3. Reference File Catalog

### 3.1 File Inventory

| File | Lines (est.) | Content |
|------|-------------|---------|
| `setup.md` | ~200 | Installation (source/binary/Docker), CLI flags, env vars, data directory, CA cert installation (macOS/Windows/Linux/Android/iOS), proxy configuration for browsers/apps |
| `mcp-tools.md` | ~400 | All 68 MCP tools with full parameter schemas, grouped by category (traffic, mocks, breakpoints, replay, sessions, config, capture, throttle, rewrites, gRPC, scripts, plugins, cert) |
| `cli-commands.md` | ~350 | All 56 CLI subcommands with flags, grouped by command tree (traffic, mocks, breakpoints, sessions, replay, config, capture, throttle, rewrites, grpc, scripts, plugins, export) |
| `rest-api.md` | ~400 | All 130+ REST endpoints with methods, paths, request/response shapes, grouped by phase (core, interception, advanced, enterprise) |
| `traffic-inspection.md` | ~200 | Workflows: list/filter/search traffic, get entry details, analyze patterns, clear traffic, count, WebSocket traffic inspection |
| `mocking.md` | ~250 | Workflows: create simple/advanced mocks, manage collections, recording mode, import/export, test/preview, analytics, versioning |
| `breakpoints.md` | ~150 | Workflows: create breakpoints, pause/resume traffic, modify paused requests, breakpoint decisions |
| `rewrites.md` | ~150 | Workflows: create rewrite rules, URL/header/body rewriting, templates, batch toggle |
| `throttling.md` | ~120 | Workflows: set throttle profile, use presets (3G/4G/DSL), toggle throttling, custom profiles |
| `replay.md` | ~120 | Workflows: save requests, replay with modifications, view history, export as cURL |
| `sessions.md` | ~120 | Workflows: create/switch/delete sessions, export/import, session presets |
| `grpc.md` | ~120 | Workflows: inspect gRPC connections/streams/frames, view stats, clear data (experimental) |
| `scripting.md` | ~120 | Workflows: create/toggle scripts, hooks (on_request/on_response), templates (experimental) |
| `plugins.md` | ~100 | Workflows: list/enable/disable plugins, reload, view stats (experimental) |
| `websockets.md` | ~120 | Workflows: inspect WS connections, filter messages, clear WS traffic |
| `export-import.md` | ~120 | Workflows: HAR export, cURL export, session export/import, rule persistence (save/load) |
| `troubleshooting.md` | ~150 | Common issues: cert errors, port conflicts, DB locked, TLS handshake failures, cert pinning, MCP connection issues |
| `harness-setup.md` | ~200 | Per-harness MCP config: Claude Desktop, Windsurf, Cursor, OpenCode, CommandCode, Devin — JSON configs, env vars, verification steps |

**Total estimated lines:** ~3,300 across all reference files

### 3.2 Content Sourcing

All reference content is derived from:
- **MCP tools**: `crates/madhyamas-mcp/src/tools/` (registry.rs, executor.rs, all tool modules)
- **CLI commands**: `crates/madhyamas-cli/src/commands/` (all command modules)
- **REST API**: `crates/madhyamas-api/src/routes.rs`, `handlers.rs`, `intercept_handlers.rs`, `phase3_handlers.rs`, `phase4_handlers.rs`
- **Core engine**: `crates/madhyamas-core/src/` (proxy, TLS, traffic, intercept, gRPC, scripting, plugins)
- **Existing docs**: `docs/MCP-INTEGRATION.md`, `docs/API.md`, `docs/ARCHITECTURE.md`, `docs/PROXY_FLOW.md`, `docs/MOCK_RESPONSES.md`

---

## 4. Multi-Target Build System

### 4.1 Architecture

```
skills/madhyamas/          ← Canonical source (Agent Skills standard)
    ↓ build.sh
dist/
├── claude/                ← .claude/skills/madhyamas/
├── devin/                 ← .devin/skills/madhyamas/
├── windsurf-skills/       ← .windsurf/skills/madhyamas/
├── windsurf-rules/        ← .windsurf/rules/madhyamas.md
├── cursor/                ← .cursor/rules/madhyamas.mdc
├── opencode/              ← .opencode/skills/madhyamas/
├── commandcode/           ← .commandcode/skills/madhyamas/
├── agents/                ← .agents/skills/madhyamas/ (universal)
└── madhyamas.skill        ← Packaged .skill zip (Claude skill format)
```

### 4.2 Build Script (`scripts/build.sh`)

The build script:
1. Reads the canonical source from `skills/madhyamas/`
2. For each target harness, transforms the frontmatter and file structure
3. Outputs to `dist/<harness>/`
4. Validates each output against the target's constraints
5. Packages a `.skill` zip for Claude distribution

#### Transformations per Harness

| Harness | Frontmatter Changes | File Changes |
|---------|-------------------|-------------|
| **Claude Code** | Add `allowed-tools`, `user-invocable: true` | Keep SKILL.md + references/ + scripts/ + assets/ |
| **Devin** | Add `triggers: ["user", "model"]`, `permissions` | Keep SKILL.md + references/ + scripts/ + assets/ |
| **Windsurf (Skills)** | Strip to `name` + `description` only | Keep SKILL.md + references/ |
| **Windsurf (Rules)** | Convert to `trigger: model_decision`, `description` | Flatten to single `.md` file (no references/) |
| **Cursor** | Convert to `description` + `alwaysApply: false` | Flatten to single `.mdc` file (inline references) |
| **OpenCode** | Strip to `name` + `description` + `metadata` | Keep SKILL.md + references/ |
| **CommandCode** | Strip to `name` + `description` + `metadata` | Keep SKILL.md + references/ |
| **Agents (universal)** | Keep canonical source as-is | No changes |
| **.skill package** | Keep canonical source | Zip the entire directory |

#### Cursor/Windsurf Rules Flattening

Cursor (`.mdc`) and Windsurf Rules don't support reference files. For these targets, the build script **concatenates** SKILL.md + all reference files into a single file, with section headers. This produces a larger file but maintains all content.

To stay within Windsurf's 12,000 char workspace rule limit, the build generates a **condensed version** that includes only the SKILL.md body + setup + mcp-tools + cli-commands + rest-api reference files (the most essential), with a note pointing to the full skill for other topics.

### 4.3 Install Script (`scripts/install.sh`)

```bash
# Install to a specific harness
./scripts/install.sh claude     # → ~/.claude/skills/madhyamas/
./scripts/install.sh devin      # → ~/.config/devin/skills/madhyamas/
./scripts/install.sh windsurf   # → ~/.codeium/windsurf/skills/madhyamas/
./scripts/install.sh cursor     # → ~/.cursor/rules/madhyamas.mdc
./scripts/install.sh opencode   # → ~/.config/opencode/skills/madhyamas/
./scripts/install.sh commandcode # → ~/.commandcode/skills/madhyamas/
./scripts/install.sh agents     # → .agents/skills/madhyamas/ (project-level)
./scripts/install.sh all        # Install to all detected harnesses
```

The install script:
1. Runs `build.sh` if `dist/` doesn't exist or is stale
2. Detects the harness's config directory (global vs project)
3. Copies/symlinks the appropriate output
4. Verifies installation by checking file presence

### 4.4 Validation Script (`scripts/validate.py`)

Checks:
- SKILL.md has valid YAML frontmatter
- `name` matches directory name, follows `^[a-z0-9]+(-[a-z0-9]+)*$`
- `description` is 1-1024 chars, includes trigger phrases
- All reference files referenced in SKILL.md exist
- No file exceeds reasonable size limits
- Frontmatter only contains known fields per target harness

---

## 5. MCP Configuration Templates

Since Madhyamas has a built-in MCP server, the skill includes config templates for each harness:

### 5.1 Claude Desktop (`assets/mcp-config-claude.json`)

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/usr/local/bin/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://127.0.0.1:3001"
      }
    }
  }
}
```

### 5.2 Windsurf (`assets/mcp-config-windsurf.json`)

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "/usr/local/bin/madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://127.0.0.1:3001"
      }
    }
  }
}
```

### 5.3 Generic (`assets/mcp-config-generic.json`)

Same structure — the `harness-setup.md` reference file explains where to place this file for each harness.

---

## 6. Publishing & Distribution

### 6.1 In-Repo Distribution

The skills live in `skills/madhyamas/` in the main madhyamas repo. Users can:

1. **Clone & install**: `git clone https://github.com/madhyamas/madhyamas.git && cd madhyamas && ./skills/madhyamas/scripts/install.sh all`
2. **Direct copy**: Copy `skills/madhyamas/` to their harness's skills directory
3. **Symlink**: `ln -s /path/to/madhyamas/skills/madhyamas ~/.claude/skills/madhyamas`

### 6.2 GitHub Releases

CI builds and attaches distributable artifacts to each release:

| Artifact | Description |
|----------|-------------|
| `madhyamas-skill.zip` | Universal `.agents/skills/madhyamas/` package |
| `madhyamas.skill` | Claude `.skill` format (zip with .skill extension) |
| `madhyamas-cursor.mdc` | Single-file Cursor rule |
| `madhyamas-windsurf-rule.md` | Single-file Windsurf rule |
| `SHA256SUMS` | Checksums for all artifacts |

### 6.3 GitHub Actions Workflow

```yaml
# .github/workflows/skills.yml
name: Build & Publish Skills

on:
  push:
    tags: ['v*']
    paths: ['skills/**']
  workflow_dispatch:

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build all skill targets
        run: ./skills/madhyamas/scripts/build.sh
      - name: Validate
        run: python3 ./skills/madhyamas/scripts/validate.py
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: madhyamas-skills
          path: dist/
      - name: Attach to release
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v2
        with:
          files: |
            dist/madhyamas.skill
            dist/madhyamas-skill.zip
            dist/cursor/madhyamas.mdc
            dist/windsurf-rules/madhyamas.md
```

### 6.4 Versioning

- Skills are versioned independently using `metadata.version` in frontmatter
- Follow semver: bump on content changes (patch), new workflows (minor), structural changes (major)
- Version is synced with the madhyamas release cycle but can be released independently

---

## 7. CI/CD Pipeline

### 7.1 Pre-commit Checks

Add to the existing `hooks/pre-commit`:

```bash
# Validate skill structure if skills/ changed
if git diff --cached --name-only | grep -q "^skills/"; then
    python3 skills/madhyamas/scripts/validate.py || exit 1
fi
```

### 7.2 CI Workflow (per PR)

```yaml
# Added to existing ci.yml
- name: Validate skills
  run: |
    python3 skills/madhyamas/scripts/validate.py
    ./skills/madhyamas/scripts/build.sh --dry-run
```

### 7.3 Release Workflow

Triggered on version tags:
1. Build all target formats
2. Validate each output
3. Package artifacts
4. Attach to GitHub release
5. Update `docs/MADHYAMAS_SKILLS_PLAN.md` with latest version

---

## 8. Implementation Roadmap

### Phase 1: Foundation (Week 1)

| Task | Deliverable |
|------|-------------|
| Create `skills/madhyamas/` directory structure | Directory + empty files |
| Write `SKILL.md` (entry point) | <500 lines, frontmatter + quick start + interface guide + reference index |
| Write `references/setup.md` | Installation, configuration, CA cert, client setup |
| Write `references/mcp-tools.md` | All 68 MCP tools with parameters |
| Write `references/cli-commands.md` | All 56 CLI subcommands with flags |
| Write `references/rest-api.md` | All 130+ REST endpoints |

### Phase 2: Workflow References (Week 2)

| Task | Deliverable |
|------|-------------|
| Write `references/traffic-inspection.md` | Filter/search/analyze workflows |
| Write `references/mocking.md` | Mock creation/management/testing workflows |
| Write `references/breakpoints.md` | Pause/resume/modify workflows |
| Write `references/rewrites.md` | URL/header/body rewrite workflows |
| Write `references/throttling.md` | Network simulation workflows |
| Write `references/replay.md` | Save/replay/export workflows |
| Write `references/sessions.md` | Session management workflows |

### Phase 3: Advanced & Support (Week 3)

| Task | Deliverable |
|------|-------------|
| Write `references/grpc.md` | gRPC inspection workflows |
| Write `references/scripting.md` | Script creation/management workflows |
| Write `references/plugins.md` | Plugin management workflows |
| Write `references/websockets.md` | WebSocket inspection workflows |
| Write `references/export-import.md` | HAR/cURL/persistence workflows |
| Write `references/troubleshooting.md` | Common issues & solutions |
| Write `references/harness-setup.md` | Per-harness MCP config guides |

### Phase 4: Build System (Week 3)

| Task | Deliverable |
|------|-------------|
| Write `scripts/build.sh` | Multi-target build script |
| Write `scripts/install.sh` | Per-harness install script |
| Write `scripts/validate.py` | Validation script |
| Create `assets/mcp-config-*.json` | MCP config templates |
| Test build against all targets | All dist/ outputs validated |

### Phase 5: CI/CD & Publishing (Week 4)

| Task | Deliverable |
|------|-------------|
| Add `.github/workflows/skills.yml` | Skills build & release workflow |
| Update pre-commit hook | Skill validation on commit |
| Test release flow | Dry-run release with artifacts |
| Update README.md | Add skills installation instructions |
| Update CLAUDE.md | Document skills directory |

---

## 9. Quality Standards

### 9.1 Content Guidelines

- **Imperative voice**: "Create a mock" not "You can create a mock"
- **Concrete examples**: Every workflow includes MCP, CLI, and REST API examples
- **No redundancy**: Information lives in one place; cross-reference instead of duplicating
- **Progressive disclosure**: SKILL.md is lean; details in references
- **Token efficiency**: Reference files are loaded only when needed

### 9.2 Accuracy Requirements

- MCP tool names, parameters, and descriptions must match `crates/madhyamas-mcp/src/tools/registry.rs`
- CLI commands must match `crates/madhyamas-cli/src/commands/mod.rs` Commands enum
- REST endpoints must match `crates/madhyamas-api/src/routes.rs`
- Environment variables and defaults must match `crates/madhyamas-core/src/config.rs`

### 9.3 Maintenance

- Skills are updated alongside code changes
- The validate script checks for broken cross-references
- CI runs validation on every PR touching `skills/`
- Version bump required when MCP tools, CLI commands, or API endpoints change

---

## 10. File Tree Summary

```
madhyamas/
├── skills/
│   └── madhyamas/
│       ├── SKILL.md
│       ├── references/
│       │   ├── setup.md
│       │   ├── mcp-tools.md
│       │   ├── cli-commands.md
│       │   ├── rest-api.md
│       │   ├── traffic-inspection.md
│       │   ├── mocking.md
│       │   ├── breakpoints.md
│       │   ├── rewrites.md
│       │   ├── throttling.md
│       │   ├── replay.md
│       │   ├── sessions.md
│       │   ├── grpc.md
│       │   ├── scripting.md
│       │   ├── plugins.md
│       │   ├── websockets.md
│       │   ├── export-import.md
│       │   ├── troubleshooting.md
│       │   └── harness-setup.md
│       ├── scripts/
│       │   ├── build.sh
│       │   ├── install.sh
│       │   └── validate.py
│       └── assets/
│           ├── mcp-config-claude.json
│           ├── mcp-config-windsurf.json
│           └── mcp-config-generic.json
├── .github/
│   └── workflows/
│       └── skills.yml              # New: skills build & release
├── docs/
│   └── MADHYAMAS_SKILLS_PLAN.md    # This document
└── hooks/
    └── pre-commit                  # Updated: add skill validation
```

---

## 11. Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Single skill (not multiple) | One trigger, simpler install, progressive disclosure handles complexity |
| Agent Skills standard as source | Universal compatibility — 6+ harnesses scan `.agents/skills/` |
| Build system generates targets | Avoid manual sync; one source of truth |
| Triple-pattern in references (MCP/CLI/REST) | Agents may have different interfaces available |
| In-repo (not separate repo) | Co-versioned with proxy, simpler for contributors |
| Cursor/Windsurf rules flattened | Those formats don't support reference files |
| `.skill` package for Claude | Standard Claude skill distribution format |
| Versioned independently | Skills can update without proxy release and vice versa |

---

## 12. Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| Skill content drifts from code | CI validation + pre-commit hook; update skills in same PR as code changes |
| Reference files too large for context | Progressive disclosure — only loaded on demand; each file <400 lines |
| Cursor `.mdc` file exceeds practical size | Build generates condensed version with essential content only |
| New harness format emerges | Build script is extensible — add a new target function |
| MCP tool signatures change | Validate script can diff against registry.rs (future enhancement) |
| Users don't know which harness they use | `install.sh all` auto-detects; README explains detection |

---

## Appendix A: Harness Format Quick Reference

### Claude Code
```yaml
---
name: madhyamas
description: ...
allowed-tools: Bash(madhyamas:*) Read Write Edit Grep
user-invocable: true
---
```
Location: `.claude/skills/madhyamas/SKILL.md`

### Devin CLI
```yaml
---
name: madhyamas
description: ...
triggers: ["user", "model"]
permissions:
  allow:
    - Read(**)
    - Bash(madhyamas:*)
---
```
Location: `.devin/skills/madhyamas/SKILL.md` or `.agents/skills/madhyamas/SKILL.md`

### Windsurf (Skills)
```yaml
---
name: madhyamas
description: ...
---
```
Location: `.windsurf/skills/madhyamas/SKILL.md`

### Windsurf (Rules)
```yaml
---
trigger: model_decision
description: ...
---
```
Location: `.windsurf/rules/madhyamas.md`

### Cursor
```yaml
---
description: ...
alwaysApply: false
---
```
Location: `.cursor/rules/madhyamas.mdc`

### OpenCode
```yaml
---
name: madhyamas
description: ...
metadata:
  author: madhyamas
  version: "0.1.0"
---
```
Location: `.opencode/skills/madhyamas/SKILL.md`

### CommandCode
```yaml
---
name: madhyamas
description: ...
metadata:
  author: madhyamas
  version: "0.1.0"
---
```
Location: `.commandcode/skills/madhyamas/SKILL.md`

### Agent Skills Standard (Universal)
```yaml
---
name: madhyamas
description: ...
license: MIT OR Apache-2.0
metadata:
  author: madhyamas
  version: "0.1.0"
  project-url: https://github.com/madhyamas/madhyamas
---
```
Location: `.agents/skills/madhyamas/SKILL.md`

---

## Appendix B: Capability Inventory Summary

### MCP Tools (68 total, 13 categories)

| Category | Count | Key Tools |
|----------|-------|-----------|
| Traffic Inspection | 5 | `get_traffic`, `get_traffic_entry`, `search_traffic`, `get_traffic_count`, `clear_traffic` |
| Mock Rules | 18 | `create_mock`, `create_advanced_mock`, `update_mock`, `test_mock`, `import_mocks`, `export_mocks`, recording tools |
| Mock Collections | 4 | `list/create/delete/toggle_mock_collections` |
| Mock Analytics | 2 | `get_mock_analytics`, `get_mock_hit_history` |
| Breakpoints | 3 | `list/create/delete_breakpoints` |
| Replay | 4 | `replay_request`, `save_request`, `list_saved_requests`, `export_curl` |
| Sessions | 5 | `list/create/switch/export/import_session` |
| Configuration | 2 | `get_config`, `update_config` |
| Capture Mode | 2 | `get_capture_status`, `toggle_capture` |
| Throttle | 4 | `get/set/toggle_throttle`, `get_throttle_presets` |
| Rewrites | 5 | `list/create/delete/toggle_rewrites`, `get_rewrite_templates` |
| gRPC | 5 | `get_grpc_connections/streams/frames/stats`, `clear_grpc` |
| Scripts | 7 | `list/create/get/update/delete/toggle_scripts`, `get_script_templates` |
| Plugins | 6 | `list/get/enable/disable_plugins`, `get_plugin_stats`, `reload_plugins` |
| Certificates | 1 | `get_cert_info` |

### CLI Commands (13 groups, 56 subcommands)

| Command Group | Subcommands |
|---------------|-------------|
| `traffic` | list, get, search, count, clear |
| `mocks` | list, create, delete, toggle |
| `breakpoints` | list, create, delete |
| `sessions` | list, create, delete, switch, export |
| `replay` | run, save, list, delete, export, history |
| `config` | get, update |
| `capture` | status, toggle, enable, disable |
| `throttle` | get, set, enable, disable, presets |
| `rewrites` | list, create, delete, toggle, templates |
| `grpc` | connections, streams, frames, stats, clear |
| `scripts` | list, create, get, delete, toggle, templates |
| `plugins` | list, get, enable, disable, stats, reload |
| `export` | har, curl |

### REST API (130+ endpoints, 4 phases)

| Phase | Endpoints | Coverage |
|-------|-----------|----------|
| Phase 1 (Core) | 24 | Traffic, sessions, config, WebSocket, health |
| Phase 2 (Interception) | 67 | Breakpoints, mocks, rewrites, throttle, replay, persistence |
| Phase 3 (Advanced) | 19 | gRPC, scripts, plugins (feature-gated) |
| Phase 4 (Enterprise) | 20+ | Auth, users, RBAC, audit, onboarding (stubs) |
