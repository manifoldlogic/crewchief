# Architecture: Workflow Commands

## 1. CLI Command Structure
We will add a new namespace `project` to `packages/cli`.

```
crewchief project
  ├── init <slug> <name>       # Scaffolds .agents/projects/{SLUG}_{name}/...
  ├── list                     # Lists active projects
  ├── status <slug>            # Shows completion status
  └── tickets
      ├── list <slug>          # Lists tickets
      └── create <slug>        # Scaffolds ticket files (empty or from plan?)
```

## 2. Scaffolding Templates
The CLI will contain templates for the standard markdown files (`analysis.md`, `plan.md`, etc.).
- Located in `packages/cli/src/templates/project/`.
- When `init` runs, it copies/renders these templates.

## 3. MCP Integration
We need to expose these CLI commands as MCP tools so the Agent can use them.
- **Current**: Maproom MCP exposes search.
- **New**: CrewChief CLI needs to expose `project_management` tools.
- **Method**: The `crewchief` CLI is already an MCP server (via stdio)? No, `maproom-mcp` is.
- **Decision**: We should add `project_*` tools to the MCP server, which internally call the CLI logic (or import the logic if it's shared).
- **Refinement**: Since `packages/cli` is the orchestrator, maybe the MCP server should depend on `packages/cli` logic? Or duplicate the logic?
- **Best Path**: Implement the logic in `packages/cli/src/project/` and expose it via `crewchief` CLI. The Agent (Claude) calls `run_terminal_cmd` to execute `crewchief project init`.

## 4. Data Model
- Projects are folders.
- Tickets are markdown files matching regex `HEADLS-\d{4}_.*.md`.
- Status is tracked via `[x]` checkboxes or metadata fields.

