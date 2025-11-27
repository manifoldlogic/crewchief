# Ticket Index: WORKFL (Agent Workflow Commands)

## Project Status: Not Started

None of the planned CLI commands have been implemented. The project scope is to migrate agentic workflow logic from `.claude/commands` (prompt files) into executable CLI commands within `packages/cli`.

## Tickets

| Ticket ID | Title | Status | Phase |
|-----------|-------|--------|-------|
| WORKFL-1001 | Create Project Command Structure | Pending | 1 |
| WORKFL-1002 | Create Planning Document Templates | Pending | 1 |
| WORKFL-1003 | Implement Project Init Command | Pending | 1 |
| WORKFL-2001 | Implement Project List Command | Pending | 2 |
| WORKFL-2002 | Implement Project Status Command | Pending | 2 |

## Phase Summary

### Phase 1: Scaffolding (Tickets 1001-1003)
- Command structure registration
- Document templates
- `crewchief project init` command

### Phase 2: Management (Tickets 2001-2002)
- `crewchief project list` command
- `crewchief project status` command

## Implementation Notes

**Current state**: No implementation exists. The following files/directories do not exist:
- `packages/cli/src/cli/project.ts`
- `packages/cli/src/templates/project/`
- `packages/cli/src/project/manager.ts`

**Success criteria**: When complete, we can replace manual instructions in `.claude/commands/create-project.md` with "Run `crewchief project init ...`".
