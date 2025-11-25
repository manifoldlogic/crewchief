# Ticket: Implement Project Init Command

**ID:** WORKFL-1003
**Phase:** 1
**Status:** Pending
**Assigned To:** Typescript Engineer

## Summary
Implement `crewchief project init <slug> <name>`.

## Background
This command creates the folder structure and files.

## Acceptance Criteria
- [ ] Validates slug (uppercase, 2-8 chars).
- [ ] Creates `.agents/projects/{SLUG}_{name}/planning/`.
- [ ] Writes all template files.
- [ ] Does not overwrite if exists (unless `--force`).

## Technical Requirements
- Use `fs` module.
- Use `slug` regex validation.

## Dependencies
- WORKFL-1001
- WORKFL-1002

