# Execution Plan: Workflow Commands

## Phase 1: Scaffolding (Tickets 1001-1003)

1. **WORKFL-1001 - Command Structure**: Create `src/cli/project.ts` and register commands
2. **WORKFL-1002 - Templates**: Create markdown templates for planning docs
3. **WORKFL-1003 - Init Command**: Implement `crewchief project init <slug> <name>`

## Phase 2: Management (Tickets 2001-2004)

4. **WORKFL-2001 - List Command**: Implement `crewchief project list` scanning `.agents/projects`
5. **WORKFL-2002 - Status Command**: Parse markdown checkboxes to report % complete
6. **WORKFL-2003 - Tickets List Command**: Implement `crewchief project tickets list <slug>`
   - Lists all tickets with detailed status (Task completed, Tests pass, Verified)
   - Human-readable table format by default
   - JSON output with `--json` flag for agent consumption
7. **WORKFL-2004 - Tickets Show Command**: Implement `crewchief project tickets show <slug> <ticket-id>`
   - Full ticket summary with acceptance criteria progress
   - Dependencies and agent assignments
   - JSON output with `--json` flag

## Agent Assignments

- **TypeScript Engineer**: All tickets

## Slash Command Integration

These CLI commands **support** (not replace) existing slash commands:
- `/create-project` can invoke `crewchief project init` for scaffolding
- `/work-on-project` can use `crewchief project tickets list --json` to track progress
- Future project workflow skills can compose these CLI primitives

## Success Definition

1. CLI commands work for both human users (terminal) and agents (tool calls)
2. JSON output enables programmatic decisions in slash commands and skills
3. Consistent ticket status parsing across all workflow agents
4. Reduced tokens for agents vs reading/interpreting prompt files

