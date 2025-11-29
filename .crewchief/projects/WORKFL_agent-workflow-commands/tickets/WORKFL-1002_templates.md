# Ticket: WORKFL-1002: Create Planning Document Templates

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
Create string templates or files for the standard planning documents used when scaffolding a new project.

## Background
When scaffolding a project with `crewchief project init`, we want consistent headers and sections for all planning documents. This ensures every project follows the same structure.

Reference: planning/plan.md - Phase 1: Scaffolding, Step 2 (Templates)

## Acceptance Criteria
- [ ] Templates for `analysis.md`, `architecture.md`, `plan.md`, `quality-strategy.md`, `security-review.md`, `README.md` defined in `src/templates/project/`
- [ ] Templates include standard placeholders (Project Name, Date, SLUG)
- [ ] Templates export as TypeScript string functions for single-binary distribution

## Technical Requirements
- Export as TS strings (easier for single-binary distribution than loading from assets)
- Each template should be a function that accepts parameters: `{ slug: string, name: string, date: string }`
- Use template literals for clean multi-line markdown

## Implementation Notes
- Create `packages/cli/src/templates/project/` directory
- Create individual template files: `analysis.ts`, `architecture.ts`, `plan.ts`, `quality-strategy.ts`, `security-review.ts`, `readme.ts`
- Create `index.ts` barrel file exporting all templates
- Reference existing templates in `.crewchief/reference/` for content structure

## Dependencies
- None

## Risk Assessment
- **Risk**: Template content may not match current conventions
  - **Mitigation**: Review `.crewchief/reference/` templates before implementation

## Files/Packages Affected
- `packages/cli/src/templates/project/` (new directory)
- `packages/cli/src/templates/project/index.ts` (new)
- `packages/cli/src/templates/project/analysis.ts` (new)
- `packages/cli/src/templates/project/architecture.ts` (new)
- `packages/cli/src/templates/project/plan.ts` (new)
- `packages/cli/src/templates/project/quality-strategy.ts` (new)
- `packages/cli/src/templates/project/security-review.ts` (new)
- `packages/cli/src/templates/project/readme.ts` (new)
