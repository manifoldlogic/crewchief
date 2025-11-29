# Ticket: WORKFL-2003: Implement Project Tickets List Command

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement `crewchief project tickets list <slug>` command to display all tickets for a project with their individual status checkboxes (Task completed, Tests pass, Verified).

## Background
Both human users and AI agents need visibility into ticket progress with granular status information. The existing `project status` command (WORKFL-2002) shows overall project progress, but users need to see individual ticket status including which specific checkboxes are checked for each ticket.

This command supports:
- **Human users**: Terminal workflow with formatted table output
- **AI agents**: Tool calls with JSON output for programmatic decisions
- **Slash commands**: Can invoke this to track progress during `/work-on-project`

Reference: planning/plan.md - Phase 2: Management, Step 6 (Tickets List Command)

## Acceptance Criteria
- [ ] Accepts project SLUG as argument
- [ ] Reads all ticket files in `tickets/` subdirectory
- [ ] Parses Status section checkboxes for each ticket:
  - Task completed: `- [x]` or `- [ ]`
  - Tests pass: `- [x]` or `- [ ]`
  - Verified: `- [x]` or `- [ ]`
- [ ] Outputs formatted table showing:
  - Ticket ID
  - Title (from ticket heading)
  - Task completed status (checkmark or empty)
  - Tests pass status (checkmark or empty)
  - Verified status (checkmark or empty)
- [ ] Supports `--json` flag for machine-readable output
- [ ] Handles missing or malformed ticket files gracefully (warn and continue)
- [ ] Returns exit code 0 on success, non-zero if project not found

## Technical Requirements
- Add `tickets` subcommand group to project command
- Add `list` subcommand to `project tickets`
- Parse ticket files using regex: `/^- \[(x| )\] \*\*(Task completed|Tests pass|Verified)\*\*/gm`
- Extract ticket title from `# Ticket: {ID}: {Title}` heading
- Use existing `listActiveProjects` from manager.ts to validate project exists
- Create helper function `parseTicketStatus(filePath): TicketSummary`
- Create interface `TicketSummary { id, title, taskCompleted, testsPassed, verified }`

## Implementation Notes
- Pattern after existing `project list` and `project status` command structures
- Use chalk for colored status indicators in terminal output (green checkmark, red X)
- JSON output structure:
  ```json
  {
    "project": { "slug": "WORKFL", "name": "agent-workflow-commands" },
    "tickets": [
      { "id": "WORKFL-1001", "title": "...", "status": { "taskCompleted": false, "testsPassed": false, "verified": false } }
    ],
    "summary": { "total": 5, "complete": 0, "percentage": 0 }
  }
  ```

## Dependencies
- WORKFL-2001 (project list/validation functionality)
- WORKFL-2002 (status parsing utilities in manager.ts)

## Risk Assessment
- **Risk**: Ticket file format variations could break parsing
  - **Mitigation**: Use defensive parsing, log warnings for unparseable files, continue with others

## Files/Packages Affected
- `packages/cli/src/cli/project.ts` (modified - add tickets list subcommand)
- `packages/cli/src/project/manager.ts` (modified - add ticket parsing utilities)
- `packages/cli/src/project/types.ts` (new - TypeScript interfaces)
