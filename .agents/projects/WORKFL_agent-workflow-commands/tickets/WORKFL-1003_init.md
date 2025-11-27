# Ticket: WORKFL-1003: Implement Project Init Command

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
Implement `crewchief project init <slug> <name>` command to scaffold a new project with all planning documents.

## Background
This command creates the folder structure and files for a new project, replacing the manual process currently documented in `.claude/commands/create-project.md`. The CLI will implement the SOP directly, generating standard markdown structures programmatically.

Reference: planning/plan.md - Phase 1: Scaffolding, Step 3 (Init Command)

## Acceptance Criteria
- [ ] Validates slug (uppercase, 2-8 characters, alphanumeric)
- [ ] Creates `.agents/projects/{SLUG}_{name}/` directory structure
- [ ] Creates `planning/` subdirectory
- [ ] Creates `tickets/` subdirectory
- [ ] Writes all template files (README.md, analysis.md, architecture.md, plan.md, quality-strategy.md, security-review.md)
- [ ] Does not overwrite if project exists (unless `--force` flag provided)
- [ ] Provides helpful error messages for validation failures

## Technical Requirements
- Use `fs` module for file operations
- Use regex validation for slug: `/^[A-Z][A-Z0-9]{1,7}$/`
- Name should be kebab-case, validated with: `/^[a-z][a-z0-9-]*$/`
- Support `--force` flag to overwrite existing projects
- Print summary of created files on success

## Implementation Notes
- Add `init` subcommand to project command in `project.ts`
- Import templates from `../templates/project`
- Use `fs.mkdirSync` with `{ recursive: true }` for directory creation
- Check for existing project before writing files

## Dependencies
- WORKFL-1001 (command structure must exist)
- WORKFL-1002 (templates must be defined)

## Risk Assessment
- **Risk**: File system operations could fail mid-write leaving partial project
  - **Mitigation**: Create all directories first, then write all files; consider atomic writes

## Files/Packages Affected
- `packages/cli/src/cli/project.ts` (modified - add init subcommand)
