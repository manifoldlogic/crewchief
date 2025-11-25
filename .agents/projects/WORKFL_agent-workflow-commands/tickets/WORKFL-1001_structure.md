# Ticket: Create Project Command Structure

**ID:** WORKFL-1001
**Phase:** 1
**Status:** Pending
**Assigned To:** Typescript Engineer

## Summary
Register the `project` command namespace in the CLI using Commander.

## Background
We need a home for the new project management commands.

## Acceptance Criteria
- [ ] `src/cli/project.ts` created.
- [ ] `project` command registered in `src/cli/index.ts`.
- [ ] `crewchief project --help` runs.

## Technical Requirements
- Use `commander` nested commands.

## Dependencies
- None

