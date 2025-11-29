# Ticket: WORKFL-2001: Implement Project List Command

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
Implement `crewchief project list` command to show active projects.

## Background
Agents and users need to know what projects exist in the workspace. This command scans the `.crewchief/projects/` directory and outputs a formatted list.

Reference: planning/plan.md - Phase 2: Management, Step 4 (List Command)

## Acceptance Criteria
- [ ] Scans `.crewchief/projects/` directory for project folders
- [ ] Outputs list formatted as `SLUG - Name` for each project
- [ ] Handles empty projects directory gracefully
- [ ] Helper function `listActiveProjects()` created in `src/project/manager.ts`
- [ ] Returns exit code 0 on success

## Technical Requirements
- Create `packages/cli/src/project/manager.ts` for project management utilities
- Parse folder names using pattern `{SLUG}_{name}`
- Sort output alphabetically by SLUG
- Support `--json` flag for machine-readable output

## Implementation Notes
- Add `list` subcommand to project command
- Use `fs.readdirSync` to scan projects directory
- Filter for directories only (exclude files)
- Export `listActiveProjects` for reuse by other commands

## Dependencies
- WORKFL-1001 (command structure must exist)

## Risk Assessment
- **Risk**: None - read-only operation with no side effects

## Files/Packages Affected
- `packages/cli/src/cli/project.ts` (modified - add list subcommand)
- `packages/cli/src/project/manager.ts` (new)
