# AI Agent Tooling Sync Workflow

Workflow for keeping the three AI-agent surfaces in sync when features change:
the **MCP server** (`crates/madhyamas-mcp/`), the **CLI** (`crates/madhyamas-cli/`),
and the **skill package** (`skills/madhyamas/`). Load this reference when adding,
removing, or changing an AI-agent-facing capability.

## The Three Surfaces

| Surface | Crate / Dir | Audience | Transport |
|---|---|---|---|
| MCP tools | `crates/madhyamas-mcp/src/tools/` | AI agents (Claude, Cursor, Windsurf, Devin, ...) | stdio JSON-RPC |
| CLI subcommands | `crates/madhyamas-cli/src/commands/` | Humans + AI agents in terminals | argv / stdout |
| Skill package | `skills/madhyamas/` | AI agents (procedural knowledge) | Markdown + references |

All three ultimately call the same REST API exposed by `crates/madhyamas-api/`.
They are *views* over the same backend; a feature exposed to one should generally
be exposed to all three.

## Sync Checklist (per feature change)

When a feature is added or its API changes:

1. **REST API** (`crates/madhyamas-api/src/`)
   - Add or update the route in `routes.rs`.
   - Implement the handler in the appropriate `*_handlers.rs`.
   - Validate inputs in `validation.rs` if user input is accepted.
   - Document the endpoint in `docs/API.md`.

2. **MCP tool** (`crates/madhyamas-mcp/src/tools/<area>.rs`)
   - Add a tool struct implementing the `Tool` trait (`tool_trait.rs`).
   - Register it in `mod.rs` / the relevant `register_*` function.
   - Name it `madhyamas_<area>_<action>` (e.g. `madhyamas_traffic_list`).
   - Define a strict JSON schema for parameters; mark required fields.
   - Return structured JSON the agent can reason about.

3. **CLI subcommand** (`crates/madhyamas-cli/src/commands/<area>.rs`)
   - Add the subcommand to the `Commands` enum (`mod.rs`).
   - Use `clap` derive; provide `--help` text, `--json` flag for machine output.
   - Call the REST API via `ApiClient`.
   - Mirror the MCP tool's parameters where it makes sense (names, defaults).

4. **Skill package** (`skills/madhyamas/`)
   - Update `references/mcp-tools.md` with the new tool (name, params, example).
   - Update `references/cli-commands.md` with the new subcommand.
   - Update `references/rest-api.md` with the new endpoint.
   - Bump the tool/command counts in `SKILL.md` description and `skills/README.md`.
   - Re-run `bash skills/madhyamas/scripts/validate.sh` to confirm counts.

5. **This agents/ package**
   - If the change affects what an agent should know, update the relevant
     `agents/agents/*.md` system prompt or a `agents/references/*.md` file.
   - Re-run `bash agents/scripts/validate.sh` and `bash agents/scripts/install.sh`.

## Naming Conventions

- **MCP tool**: `madhyamas_<area>_<action>` — snake_case, area matches the CLI subcommand group.
- **CLI subcommand**: `madhyamas <area> <action>` — e.g. `madhyamas traffic list`.
- **REST route**: `/api/<area>[/<id>[/<action>]]` — e.g. `GET /api/traffic`, `POST /api/mocks`.
- **Skill reference file**: `<area>.md` in `skills/madhyamas/references/`.

## Adding a New MCP Tool (concrete steps)

1. Open `crates/madhyamas-mcp/src/tools/<area>.rs` (create if new area).
2. Define a struct `pub struct <Area><Action>Tool { ... }`.
3. Implement `Tool` trait: `name()`, `description()`, `input_schema()`, `execute()`.
4. In `execute()`, deserialize params, call `ApiClient`, return JSON.
5. Register in the area's `register_<area>_tools()` function (called from `mod.rs`).
6. Add a doc comment with a usage example.
7. Update `skills/madhyamas/references/mcp-tools.md` and the tool count.

## Adding a New CLI Subcommand (concrete steps)

1. Open or create `crates/madhyamas-cli/src/commands/<area>.rs`.
2. Add a variant to the `Commands` enum in `mod.rs` with clap attributes.
3. Implement the handler: parse args, call `ApiClient`, print output (table or `--json`).
4. Add `--help` text and examples.
5. Update `skills/madhyamas/references/cli-commands.md` and the command count.

## Validation

After any change to the three surfaces:

```bash
# Rust side
cargo build -p madhyamas-mcp -p madhyamas-cli
cargo clippy -p madhyamas-mcp -p madhyamas-cli --all-targets

# Skill package
bash skills/madhyamas/scripts/validate.sh

# Agents package (this one)
bash agents/scripts/validate.sh
```

The skill `validate.sh` checks that MCP tool count and CLI command count match the
numbers advertised in `SKILL.md`. Keep those counts accurate.

## Backward Compatibility

- Never remove an MCP tool or CLI subcommand without a deprecation cycle.
- Adding optional parameters is safe; removing or renaming requires a major bump.
- REST routes follow the same rule — additive changes only within a minor version.
- When a tool MUST change shape, add a new tool (`madhyamas_<area>_<action>_v2`) and
  deprecate the old one in its description.
