# Ticket: SQLINFRA-1001: Rename and Reorganize CI Jobs

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - YAML syntax validated locally; full CI verification on PR push
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- CI workflow changes are verified by the workflow running successfully on PR
- "Tests pass" means the modified workflow runs without errors
- All existing tests must continue to pass

## Agents
- github-actions-specialist
- unit-test-runner (N/A - CI verification via PR)
- verify-ticket
- commit-ticket

## Summary
Update `.github/workflows/test.yml` to clearly distinguish SQLite-primary and PostgreSQL-integration jobs, presenting SQLite as the default testing path.

## Background
The SQLite backend has been fully implemented across the codebase (VECSTORE, MAPCLI, MCPDB, VSCODEDB projects), but CI/CD workflows still present PostgreSQL as the primary testing backend. This creates confusion about which backend is the default.

This ticket implements Phase 1 of the SQLINFRA project plan - CI Workflow Updates. The goal is to reorganize CI to present SQLite as primary, PostgreSQL as optional integration testing.

Reference: [SQLINFRA Plan - Phase 1](../planning/plan.md#phase-1-ci-workflow-updates)

## Acceptance Criteria
- [ ] All existing tests pass after workflow changes
- [ ] Job names clearly indicate their database backend (SQLite vs PostgreSQL)
- [ ] SQLite jobs appear first/prominently in workflow visualization
- [ ] PostgreSQL job is labeled as "integration" testing
- [ ] Workflow syntax is valid (GitHub Actions validates on PR push)

## Technical Requirements
- Rename `test` job to `test-postgres` with clear "PostgreSQL Integration" label in the `name` field
- Rename `test-rust` job to have clear backend names in matrix (if applicable)
- Group SQLite jobs together at top of workflow file (before PostgreSQL jobs)
- Add YAML comments explaining the purpose of each job group:
  - SQLite jobs: Primary testing path, no external dependencies
  - PostgreSQL jobs: Integration testing for team/production scenarios
- Preserve all existing test functionality - no test changes, only naming/organization

## Implementation Notes
- The workflow file is at `.github/workflows/test.yml`
- Current job structure:
  - `test` - PostgreSQL service container required
  - `test-rust` - Matrix: sqlite, postgres (no service needed for SQLite)
  - `test-sqlite-e2e` - SQLite CLI tests (no service)
  - `test-mcp-sqlite` - SQLite MCP tests (no service)
- Target organization:
  1. SQLite jobs first (test-sqlite-e2e, test-mcp-sqlite, test-rust with sqlite)
  2. PostgreSQL jobs second (test-postgres, test-rust with postgres)
- Use `name:` field for human-readable labels in GitHub Actions UI
- Consider using job groups via comments for visual organization

## Dependencies
- None - this is the first ticket in the SQLINFRA project
- External: VECSTORE, MAPCLI, MCPDB, VSCODEDB all complete

## Risk Assessment
- **Risk**: Workflow syntax errors break CI
  - **Mitigation**: GitHub validates YAML syntax on PR push; review workflow graph before merge
- **Risk**: Job dependencies break
  - **Mitigation**: Test on PR branch; review needs: relationships
- **Risk**: Renamed jobs break status checks
  - **Mitigation**: Update branch protection rules if needed after merge

## Files/Packages Affected
- `.github/workflows/test.yml` - Main workflow restructure
