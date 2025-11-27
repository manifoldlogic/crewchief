# Ticket Index: WORKFL (Agent Workflow Commands)

## Project Status: Not Started

None of the planned CLI commands have been implemented. The project creates CLI commands that support the project workflow by providing deterministic scaffolding and status operations for **both human users and AI agents**.

## Tickets

| Ticket ID | Title | Status | Phase |
|-----------|-------|--------|-------|
| WORKFL-1001 | Create Project Command Structure | Pending | 1 |
| WORKFL-1002 | Create Planning Document Templates | Pending | 1 |
| WORKFL-1003 | Implement Project Init Command | Pending | 1 |
| WORKFL-2001 | Implement Project List Command | Pending | 2 |
| WORKFL-2002 | Implement Project Status Command | Pending | 2 |
| WORKFL-2003 | Implement Project Tickets List Command | Pending | 2 |
| WORKFL-2004 | Implement Project Tickets Show Command | Pending | 2 |

## Phase Summary

### Phase 1: Scaffolding (Tickets 1001-1003)
- Command structure registration
- Document templates
- `crewchief project init` command

### Phase 2: Management (Tickets 2001-2004)
- `crewchief project list` command
- `crewchief project status` command
- `crewchief project tickets list` command (detailed ticket status)
- `crewchief project tickets show` command (full ticket details)

## Dependency Chain

```
WORKFL-1001 (structure)
    └── WORKFL-1002 (templates)
    └── WORKFL-1003 (init) [depends on 1001, 1002]

WORKFL-2001 (list)
    └── WORKFL-2002 (status) [depends on 2001]
    └── WORKFL-2003 (tickets list) [depends on 2001, 2002]
        └── WORKFL-2004 (tickets show) [depends on 2003]
```

## Implementation Notes

**Current state**: No implementation exists. The following files/directories do not exist:
- `packages/cli/src/cli/project.ts`
- `packages/cli/src/templates/project/`
- `packages/cli/src/project/manager.ts`
- `packages/cli/src/project/types.ts`

**Key design decisions**:
- All commands support `--json` flag for agent consumption
- Human-readable output by default for terminal users
- Commands support (not replace) existing slash commands
- Enables future creation of project workflow skills

**Success criteria**:
1. CLI commands work for both human users and AI agents
2. JSON output enables programmatic decisions in slash commands
3. Consistent ticket status parsing across all workflow agents
