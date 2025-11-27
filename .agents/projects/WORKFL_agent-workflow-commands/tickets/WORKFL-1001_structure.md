# Ticket: WORKFL-1001: Create Project Command Structure

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
Register the `project` command namespace in the CLI using Commander.

## Background
We need a home for the new project management commands. This implements the first step of Phase 1 from the WORKFL plan, establishing the CLI command structure for project management.

Reference: planning/plan.md - Phase 1: Scaffolding, Step 1 (Command Structure)

## Acceptance Criteria
- [ ] `src/cli/project.ts` created
- [ ] `project` command registered in `src/cli/index.ts`
- [ ] `crewchief project --help` runs successfully

## Technical Requirements
- Use `commander` nested commands pattern (consistent with existing commands like worktree, agent)
- Follow existing CLI command registration patterns from `src/cli/worktree.ts`
- Export `registerProjectCommands` function

## Implementation Notes
- Create a new file `packages/cli/src/cli/project.ts`
- Follow the pattern from `registerWorktreeCommands` in `worktree.ts`
- Add import and registration call in `index.ts`

## Dependencies
- None

## Risk Assessment
- **Risk**: None - this is foundational scaffolding with no external dependencies

## Files/Packages Affected
- `packages/cli/src/cli/project.ts` (new)
- `packages/cli/src/cli/index.ts` (modified)
