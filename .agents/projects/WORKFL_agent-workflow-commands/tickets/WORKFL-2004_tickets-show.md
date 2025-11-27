# Ticket: WORKFL-2004: Implement Project Tickets Show Command

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
Implement `crewchief project tickets show <slug> <ticket-id>` command to display full ticket details including acceptance criteria progress, dependencies, and verification state.

## Background
Both human users and AI agents need to inspect individual tickets to understand their full state before working on them or reviewing their completion. This provides a comprehensive view that helps:

- **Human users**: Quickly assess ticket readiness without opening the markdown file
- **AI agents**: Make programmatic decisions about ticket state and next steps
- **Verify-ticket agent**: Cross-reference acceptance criteria with implementation
- **Work-on-project command**: Determine which ticket to work on next

Reference: planning/plan.md - Phase 2: Management, Step 7 (Tickets Show Command)

## Acceptance Criteria
- [ ] Accepts project SLUG and TICKET_ID as arguments
- [ ] Locates and reads the specific ticket file
- [ ] Displays full summary view including:
  - Ticket ID and Title
  - Status section with all 3 checkboxes and their states
  - Summary section content
  - Acceptance Criteria with individual checkbox states and progress (e.g., "3/5 complete")
  - Dependencies list
  - Assigned agents
- [ ] Supports `--json` flag for machine-readable output
- [ ] Shows clear error if ticket or project not found
- [ ] Returns exit code 0 on success, non-zero on error

## Technical Requirements
- Add `show` subcommand to `project tickets` command group
- Parse markdown sections: Status, Summary, Acceptance Criteria, Dependencies, Agents
- Create interface `TicketDetails` extending `TicketSummary` with:
  - `summary: string`
  - `acceptanceCriteria: { text: string, checked: boolean }[]`
  - `dependencies: string[]`
  - `agents: string[]`
- Use section-based parsing (identify `## ` headers, extract content until next header)
- Parse acceptance criteria checkboxes: `/^- \[(x| )\] (.+)$/gm`

## Implementation Notes
- Command structure: `crewchief project tickets show <slug> <ticket-id>`
- Human-readable output format:
  ```
  Ticket: WORKFL-1003 - Implement Project Init Command

  Status:
    [ ] Task completed
    [ ] Tests pass
    [ ] Verified

  Summary:
    Implement `crewchief project init <slug> <name>` to scaffold new projects

  Acceptance Criteria: (0/6 complete)
    [ ] Validates slug (uppercase, 2-8 chars, alphanumeric)
    [ ] Creates `.agents/projects/{SLUG}_{name}/` with subdirectories
    ...

  Dependencies:
    - WORKFL-1001 (structure)
    - WORKFL-1002 (templates)

  Agents: typescript-engineer, unit-test-runner, verify-ticket, commit-ticket
  ```
- JSON output structure:
  ```json
  {
    "id": "WORKFL-1003",
    "title": "Implement Project Init Command",
    "status": { "taskCompleted": false, "testsPassed": false, "verified": false },
    "summary": "Implement `crewchief project init <slug> <name>`...",
    "acceptanceCriteria": [
      { "text": "Validates slug (uppercase, 2-8 chars, alphanumeric)", "checked": false }
    ],
    "acceptanceCriteriaProgress": { "complete": 0, "total": 6 },
    "dependencies": ["WORKFL-1001", "WORKFL-1002"],
    "agents": ["typescript-engineer", "unit-test-runner", "verify-ticket", "commit-ticket"]
  }
  ```

## Dependencies
- WORKFL-2003 (ticket parsing utilities, `TicketSummary` interface)

## Risk Assessment
- **Risk**: Markdown section parsing could be fragile with varied formatting
  - **Mitigation**: Use defensive parsing with fallbacks for missing sections
- **Risk**: Ticket ID format variations
  - **Mitigation**: Support both exact match and case-insensitive match

## Files/Packages Affected
- `packages/cli/src/cli/project.ts` (modified - add tickets show subcommand)
- `packages/cli/src/project/manager.ts` (modified - add detailed ticket parsing)
- `packages/cli/src/project/types.ts` (modified - add TicketDetails interface)
