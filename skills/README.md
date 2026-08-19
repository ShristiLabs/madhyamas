# Madhyamas Skills

[![skills.sh](https://img.shields.io/badge/skills.sh-madhyamas-blue?logo=vercel&logoColor=white)](https://skills.sh/ShristiLabs/madhyamas)
[![npm](https://img.shields.io/badge/npm-%40madhyamas%2Fskill-blue?logo=npm&logoColor=white)](https://www.npmjs.com/package/@madhyamas/skill)

AI agent skills package providing procedural knowledge for using the [Madhyamas](https://github.com/ShristiLabs/madhyamas) HTTP/HTTPS debugging proxy.

## What's Included

- **146 MCP tools** — Full coverage of traffic inspection, mocking, breakpoints, rewrites, throttling, replay, sessions, gRPC, scripting, and plugins (135 core + 11 enterprise)
- **159 CLI subcommands** — Complete command-line interface reference
- **184 REST API endpoints** — All HTTP endpoints with examples
- **18 workflow guides** — Step-by-step procedures for common debugging tasks
- **Multi-harness support** — Works with Claude, Windsurf, Cursor, Devin, OpenCode, CommandCode, and any Agent Skills-compatible harness

## Quick Start

### For AI Agent Users

1. Install Madhyamas: `cargo install madhyamas`
2. Start the proxy: `madhyamas serve`
3. Install the skill for your harness (choose one method below)

#### Via skills.sh (recommended)

```bash
# Install to all detected agents (interactive)
npx skills add ShristiLabs/madhyamas --skill madhyamas

# Install to a specific agent
npx skills add ShristiLabs/madhyamas --skill madhyamas -a claude-code
npx skills add ShristiLabs/madhyamas --skill madhyamas -a cursor
npx skills add ShristiLabs/madhyamas --skill madhyamas -a windsurf

# Install globally (user-level)
npx skills add ShristiLabs/madhyamas --skill madhyamas --global

# Non-interactive (CI/CD)
npx skills add ShristiLabs/madhyamas --skill madhyamas -y

# List available skills without installing
npx skills add ShristiLabs/madhyamas --list
```

#### Via npm

```bash
# Install globally
npm install -g @madhyamas/skill

# Or as a project dev dependency
npm install --save-dev @madhyamas/skill
```

#### Via build scripts (from source)

```bash
# Build all targets
bash skills/madhyamas/scripts/build.sh

# Install for your harness (project-level)
bash skills/madhyamas/scripts/install.sh claude
bash skills/madhyamas/scripts/install.sh devin
bash skills/madhyamas/scripts/install.sh windsurf
bash skills/madhyamas/scripts/install.sh cursor
bash skills/madhyamas/scripts/install.sh agents  # universal

# Or install globally
bash skills/madhyamas/scripts/install.sh claude --global

# Or install to all harnesses
bash skills/madhyamas/scripts/install.sh all
```

4. Configure MCP (see `assets/` for config templates)
5. Restart your AI agent

### For Skill Developers

```bash
# Validate the skill package
bash skills/madhyamas/scripts/validate.sh

# Build for all targets (outputs to dist/)
bash skills/madhyamas/scripts/build.sh

# Dry run (preview without writing)
bash skills/madhyamas/scripts/build.sh --dry-run
```

## Directory Structure

```
skills/madhyamas/
├── SKILL.md                    # Entry point (always loaded when skill triggers)
├── package.json                # npm package config for @madhyamas/skill
├── .npmignore                  # npm publish exclusions
├── references/                 # Detailed reference files (loaded on demand)
│   ├── setup.md                # Installation, configuration, CA certs
│   ├── mcp-tools.md            # All 146 MCP tools with parameters
│   ├── cli-commands.md         # All 159 CLI subcommands with flags
│   ├── rest-api.md             # All 184 REST API endpoints
│   ├── traffic-inspection.md   # Filtering, searching, analyzing traffic
│   ├── mocking.md              # Creating and managing mock responses
│   ├── breakpoints.md          # Pausing and modifying traffic
│   ├── rewrites.md             # URL/header/body rewriting
│   ├── throttling.md           # Network condition simulation
│   ├── replay.md               # Replaying and saving requests
│   ├── sessions.md             # Session management
│   ├── grpc.md                 # gRPC traffic inspection
│   ├── scripting.md            # JavaScript scripting (experimental)
│   ├── plugins.md              # Plugin management (experimental)
│   ├── websockets.md           # WebSocket traffic inspection
│   ├── export-import.md        # HAR/cURL export, persistence
│   ├── troubleshooting.md      # Common issues and solutions
│   └── harness-setup.md        # Per-harness MCP configuration
├── scripts/                    # Build and install tooling
│   ├── build.sh                # Build for all target harnesses
│   ├── install.sh              # Install to a specific harness
│   ├── validate.sh             # Validate skill package
│   └── pre-commit.sh           # Git pre-commit hook
└── assets/                     # MCP config templates
    ├── mcp-config.json         # Generic MCP config
    ├── claude-desktop-config.json
    ├── windsurf-mcp-config.json
    └── devin-mcp-config.json
```

## Supported Harnesses

| Harness | Format | Install Target | Notes |
|---------|--------|---------------|-------|
| Agent Skills (universal) | `.agents/skills/` | `agents` | Canonical format, works with many harnesses |
| Claude Code | `.claude/skills/` | `claude` | Adds `allowed-tools` frontmatter |
| Devin CLI | `.devin/skills/` | `devin` | Adds `triggers` frontmatter |
| Windsurf Skills | `.windsurf/skills/` | `windsurf` | Standard skill format |
| Windsurf Rules | `.windsurf/rules/` | (flattened) | Single-file with trigger frontmatter |
| Cursor | `.cursor/rules/` | `cursor` | Flattened `.mdc` file |
| OpenCode | `.opencode/skills/` | `opencode` | Standard skill format |
| CommandCode | `.commandcode/skills/` | `commandcode` | Standard skill format |

## Build Outputs

Running `build.sh` produces the following in `dist/`:

```
dist/
├── agents/madhyamas/           # Universal Agent Skills format
├── claude/madhyamas/           # Claude Code format
├── devin/madhyamas/            # Devin CLI format
├── windsurf-skills/madhyamas/  # Windsurf Skills format
├── windsurf-rules/madhyamas.md # Windsurf Rules (flattened)
├── cursor/madhyamas.mdc        # Cursor (flattened)
├── opencode/madhyamas/         # OpenCode format
├── commandcode/madhyamas/      # CommandCode format
├── madhyamas.skill             # Packaged .skill file (zip)
└── madhyamas-skill.zip         # Universal zip archive
```

## MCP Configuration

Add the Madhyamas MCP server to your AI agent's config:

```json
{
  "mcpServers": {
    "madhyamas": {
      "command": "madhyamas",
      "args": ["mcp"],
      "env": {
        "MADHYAMAS_API_URL": "http://127.0.0.1:3001"
      }
    }
  }
}
```

See `assets/` for harness-specific config files and `references/harness-setup.md` for detailed setup instructions.

## Validation

Run the validation script to check the skill package:

```bash
bash skills/madhyamas/scripts/validate.sh
```

Checks performed:
- Required files exist (SKILL.md, all reference files)
- SKILL.md has valid YAML frontmatter (name, description, license)
- SKILL.md is under 500 lines
- All referenced files in SKILL.md exist
- No emojis in markdown files
- MCP tool count matches code (146)
- CLI command count matches code (159)
- REST endpoint count matches code (184)
- Scripts are executable

## Publishing

### To skills.sh

skills.sh indexes skills from public GitHub repos via install telemetry. No submission form is needed — once users run `npx skills add ShristiLabs/madhyamas`, the skill appears on [skills.sh](https://skills.sh).

To improve discoverability:
- The skills.sh badge is in the repo README (static shields.io badge until skills.sh indexes the skill via telemetry)
- GitHub topics added: `agent-skill`, `claude-code`, `cursor`, `mcp`, `debugging-proxy`, `http-proxy`
- Verify discoverability: `npx skills add ShristiLabs/madhyamas --list`
- Once indexed on skills.sh, switch the badge to the dynamic format:
  ```markdown
  [![skills.sh](https://skills.sh/b/ShristiLabs/madhyamas)](https://skills.sh/ShristiLabs/madhyamas)
  ```

### To npm

The `package.json` at `skills/madhyamas/` is configured for npm publishing:

```bash
cd skills/madhyamas

# Validate before publishing
bash scripts/validate.sh

# Publish to npm (requires npm login)
npm login
npm publish --access public

# Or publish via skillpm (validates Agent Skills spec)
npx skillpm publish
```

Users then install with:
```bash
npm install -g @madhyamas/skill
# or
npx skillpm install @madhyamas/skill
```

### To anthropics/skills (official registry)

Once the skill is stable and has real users, submit a PR to [anthropics/skills](https://github.com/anthropics/skills):

```bash
gh repo fork anthropics/skills --clone
cd skills
cp -r /path/to/madhyamas/skills/madhyamas skills/madhyamas
git add skills/madhyamas
git commit -m "Add madhyamas skill for HTTP/HTTPS proxy debugging"
git push origin add-madhyamas-skill
gh pr create --title "Add madhyamas skill" --body "..."
```

## License

Dual MIT OR Apache-2.0, matching the main Madhyamas project.
