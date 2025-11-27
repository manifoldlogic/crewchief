# Ticket: WORKFL-2002: Implement Project Status Command

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
Implement `crewchief project status <slug>` command to check project progress.

## Background
Agents need to check progress on a project by examining ticket completion status. This command parses ticket files and reports overall progress.

Reference: planning/plan.md - Phase 2: Management, Step 5 (Status Command)

## Acceptance Criteria
- [ ] Accepts project SLUG as argument
- [ ] Reads ticket files in `tickets/` subdirectory
- [ ] Counts total checkboxes vs checked checkboxes in Status section
- [ ] Outputs summary (e.g., "3/5 tickets complete (60%)")
- [ ] Lists each ticket with its completion status
- [ ] Returns non-zero exit code if project not found

## Technical Requirements
- Simple markdown parsing for checkbox patterns: `- [x]` vs `- [ ]`
- Focus on Status section checkboxes (Task completed, Tests pass, Verified)
- Support `--json` flag for machine-readable output
- Support `--verbose` flag for detailed ticket breakdown

## Implementation Notes
- Add `status` subcommand to project command
- Parse markdown files looking for checkbox pattern `/- \[(x| )\]/gi`
- Calculate percentage complete based on checked/total ratio
- Consider tickets with all 3 status checkboxes checked as "complete"

## Dependencies
- WORKFL-2001 (list functionality for project validation)

## Risk Assessment
- **Risk**: Markdown parsing could be fragile with edge cases
  - **Mitigation**: Focus only on Status section checkboxes, ignore others

## Files/Packages Affected
- `packages/cli/src/cli/project.ts` (modified - add status subcommand)
- `packages/cli/src/project/manager.ts` (modified - add status parsing utilities)
