# Ticket: Implement Project List Command

**ID:** WORKFL-2001
**Phase:** 2
**Status:** Pending
**Assigned To:** Typescript Engineer

## Summary
Implement `crewchief project list` to show active projects.

## Background
Agents need to know what projects exist.

## Acceptance Criteria
- [ ] Scans `.agents/projects/`.
- [ ] Outputs list of `SLUG - Name`.
- [ ] Helper: `listActiveProjects()` in `src/project/manager.ts`.

## Dependencies
- WORKFL-1001

