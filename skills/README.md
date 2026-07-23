# Madhyamas Skills

AI agent skills package providing procedural knowledge for using the [Madhyamas](https://github.com/madhyamas/madhyamas) HTTP/HTTPS debugging proxy.

## What's Included

- **67 MCP tools** — Full coverage of traffic inspection, mocking, breakpoints, rewrites, throttling, replay, sessions, gRPC, scripting, and plugins
- **58 CLI subcommands** — Complete command-line interface reference
- **130+ REST API endpoints** — All HTTP endpoints with examples
- **18 workflow guides** — Step-by-step procedures for common debugging tasks
- **Multi-harness support** — Works with Claude, Windsurf, Cursor, Devin, OpenCode, CommandCode, and any Agent Skills-compatible harness

## Quick Start

### For AI Agent Users

1. Install Madhyamas: `cargo install madhyamas`
2. Start the proxy: `madhyamas serve`
3. Install the skill for your harness:

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
├── references/                 # Detailed reference files (loaded on demand)
│   ├── setup.md                # Installation, configuration, CA certs
│   ├── mcp-tools.md            # All 67 MCP tools with parameters
│   ├── cli-commands.md         # All 58 CLI subcommands with flags
│   ├── rest-api.md             # All 130+ REST API endpoints
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
│   └── validate.sh             # Validate skill package
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
- MCP tool count matches expected (~67)
- CLI command count matches expected (~58)
- Scripts are executable

## License

Dual MIT OR Apache-2.0, matching the main Madhyamas project.
