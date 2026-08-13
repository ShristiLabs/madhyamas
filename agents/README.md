# Madhyamas Specialized Agents

Specialized AI agent definitions for working on the Madhyamas debugging proxy.
Each agent is a focused worker with its own system prompt, tool scope, and
quality standards — designed to be more efficient than a general-purpose agent
for its specific activity.

## Design Principles

1. **LLM-agnostic** — No model names anywhere in the canonical source. The
   `install.sh` script injects `model: inherit` only for harnesses that
   require the field. Any LLM/harness/user can override per-agent.
2. **Harness-agnostic** — Agents are authored once in a superset frontmatter
   and fanned out to every supported harness by `install.sh`.
3. **Efficient** — Lean system prompts (under 500 lines) with shared
   `references/` loaded on demand (progressive disclosure). Least-privilege
   tool scopes (the reviewer agent is read-only).
4. **Best practices** — Each prompt follows a fixed structure (role →
   responsibilities → process → quality standards → output format → edge
   cases), cites source files for verification, and defines a deterministic
   output format so downstream agents can consume the result.

## Agents

| Agent | Color | Tools | Purpose |
|---|---|---|---|
| `docs-site-author` | magenta | read, write, edit, grep, glob, exec | End-user docs (`docs-site/`, VitePress) |
| `docs-author` | cyan | read, write, edit, grep, glob, exec | Developer reference docs (`docs/`) |
| `developer` | green | full | Feature dev across `crates/` + `web/` |
| `reviewer` | blue | read, grep, glob, exec | Read-only code review of git diffs |
| `plugin-engineer` | yellow | read, write, edit, grep, glob, exec | Build, test, sign, document WASM plugins |
| `ai-agent-tooling` | red | read, write, edit, grep, glob, exec | Sync MCP tools, CLI subcommands, skill package |

### Enterprise Implementation Pipeline

These 7 agents form a pipeline for implementing the enterprise tier
described in `docs/ENTERPRISE_IMPLEMENTATION_PLAN.md`. The orchestrator
coordinates the others in a chain: issues -> developer -> tester +
reviewer (parallel) -> regression -> committer.

| Agent | Color | Tools | Purpose |
|---|---|---|---|
| `enterprise-orchestrator` | cyan | full | Coordinate the pipeline, monitor status, log progress |
| `enterprise-issues` | magenta | full | Create GitHub issues with context, acceptance criteria, approach |
| `enterprise-developer` | green | full | Read issues, implement enterprise features following the plan |
| `enterprise-reviewer` | blue | read, grep, glob, exec | Review changes for correctness, security, performance, scalability |
| `enterprise-tester` | yellow | full | Generate unit tests, ensure 80%+ coverage on changed modules |
| `enterprise-regression` | red | full | Build both tiers, run all tests, verify no regressions |
| `enterprise-committer` | white | full | Format, stage, generate commit message, commit |

## Directory Structure

```
agents/
├── README.md                      # This file
├── agents/                        # Canonical agent definitions (source of truth)
│   ├── docs-site-author.md
│   ├── docs-author.md
│   ├── developer.md
│   ├── reviewer.md
│   ├── plugin-engineer.md
│   ├── ai-agent-tooling.md
│   ├── enterprise-orchestrator.md    # Enterprise pipeline coordinator
│   ├── enterprise-issues.md          # GitHub issue creation
│   ├── enterprise-developer.md       # Enterprise feature implementation
│   ├── enterprise-reviewer.md        # Enterprise code review (read-only)
│   ├── enterprise-tester.md          # Unit test generation (80%+ coverage)
│   ├── enterprise-regression.md      # Build + regression verification
│   └── enterprise-committer.md       # Format, stage, commit
├── references/                    # Shared, loaded on demand (keeps prompts lean)
│   ├── project-conventions.md     # Rust workspace + React conventions, build/test cmds
│   ├── docs-site-structure.md     # docs-site/ layout, IA, frontmatter, SEO rules
│   ├── plugin-workflow.md         # SDK + examples + test + doc workflow
│   ├── ai-agent-tooling-workflow.md  # MCP/CLI/skill sync checklist
│   └── enterprise-workflow.md     # Enterprise pipeline, handoff protocol, phases
└── scripts/
    ├── install.sh                 # Fan out to all harness locations (subagents + skills)
    └── validate.sh                # Check frontmatter, refs, agnosticism, line counts
```

## Installation

`install.sh` reads the canonical definitions and emits both **subagent
profiles** and **slash-command skill wrappers** to every supported harness.

```bash
# Install to all harnesses (default)
bash agents/scripts/install.sh

# Install to a specific harness
bash agents/scripts/install.sh claude
bash agents/scripts/install.sh devin
bash agents/scripts/install.sh agents        # universal .agents/
bash agents/scripts/install.sh windsurf
bash agents/scripts/install.sh cursor
bash agents/scripts/install.sh opencode
bash agents/scripts/install.sh commandcode

# Preview without writing
bash agents/scripts/install.sh --dry-run
```

### Supported Harnesses

| Harness | Subagent target | Skill target | Notes |
|---|---|---|---|
| Universal `.agents/` | `.agents/agents/<name>.md` | `.agents/skills/<name>/SKILL.md` | Canonical Agent Skills format |
| Claude Code | `.claude/agents/<name>.md` | `.claude/skills/<name>/SKILL.md` | `model: inherit` + `color` injected |
| Devin CLI | `.devin/agents/<name>.md` | `.devin/skills/<name>/SKILL.md` | `allowed-tools` list format |
| Windsurf | `.windsurf/agents/<name>.md` | `.windsurf/skills/<name>/SKILL.md` | Standard skill format |
| Cursor | `.cursor/rules/<name>.mdc` | (flattened into the rule) | Single `.mdc` per agent |
| OpenCode | `.opencode/agents/<name>.md` | `.opencode/skills/<name>/SKILL.md` | Standard skill format |
| CommandCode | `.commandcode/agents/<name>.md` | `.commandcode/skills/<name>/SKILL.md` | Standard skill format |

Shared `references/` are copied to `<harness>/skills/_shared-references/` so
skill wrappers can reference them with a stable relative path.

## Validation

```bash
bash agents/scripts/validate.sh
```

Checks performed:
- Required directories exist
- Every agent file has valid YAML frontmatter with `name`, `description`,
  `color`, `allowed-tools`, `triggers`
- `name` matches the filename
- **No model names** anywhere in canonical source (LLM-agnostic)
- Every `references/*.md` and `agents/*.md` mention resolves to a real file
- Agent system-prompt bodies under 500 lines
- No emojis in agent or reference markdown
- Scripts are executable and syntactically valid

## How the Agents Are Used

### As subagents (autonomous delegation)

The parent agent (or a human) dispatches a subagent with the appropriate
profile. Example in Devin CLI:

> "Review my feature branch using the reviewer subagent."

The parent selects the `reviewer` profile, which is read-only and produces a
ranked findings report the parent can act on.

### As slash-command skills (user-initiated)

A user invokes a skill directly:

> `/docs-site-author` — then describes the page to write.

The skill's system prompt is injected into the conversation, scoped tools are
applied, and the agent proceeds with the specialized workflow.

Both modes share the same canonical system prompt — `install.sh` only wraps
the frontmatter differently.

## Adding a New Agent

1. Create `agents/agents/<name>.md` with the superset frontmatter:
   ```yaml
   ---
   name: <name>                     # must match filename, lowercase-hyphens
   description: >                   # when to use this agent
     ...
   color: <blue|cyan|green|yellow|magenta|red>
   allowed-tools:                   # least privilege
     - read
     - ...
   triggers:
     - user
     - model
   ---
   ```
2. Write the system prompt body following the standard structure
   (role → responsibilities → process → quality standards → output format →
   edge cases). Keep under 500 lines. Reference shared `references/*.md`
   files rather than inlining large content.
3. If the agent needs new shared knowledge, add it under `agents/references/`.
4. Run `bash agents/scripts/validate.sh` to confirm the new file passes.
5. Run `bash agents/scripts/install.sh` to fan out to all harnesses.

## LLM-Agnosticism

The canonical source contains **no model names** (`gpt-*`, `claude-*`,
`sonnet`, `opus`, `haiku`, `gemini`, `llama`, `mistral`, `glm-*`, `grok`,
`swe-*`). The `validate.sh` script enforces this. Rationale:

- Different harnesses offer different models; pinning a name makes the agent
  non-portable.
- Model selection is a deployment-time concern, not an authoring-time one.
- A user or admin can override the model per-agent via their harness config
  without editing the canonical source.

`install.sh` injects `model: inherit` for harnesses that require the field
(Claude Code). Other harnesses omit it and use their default subagent model.

## Relationship to `skills/madhyamas/`

This `agents/` package is **project-local** and separate from the published
`skills/madhyamas/` skill package (which teaches AI agents how to *use*
Madhyamas as a debugging proxy). The `agents/` package teaches AI agents how
to *develop* Madhyamas itself. They do not overlap.

## License

Dual MIT OR Apache-2.0, matching the main Madhyamas project.
