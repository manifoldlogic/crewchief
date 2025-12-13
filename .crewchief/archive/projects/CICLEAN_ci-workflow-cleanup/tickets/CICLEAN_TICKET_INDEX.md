# CICLEAN Ticket Index

## Project: CI Workflow Cleanup

**Status**: Active
**Created**: 2025-12-13

## Overview

This project removes outdated Cargo feature flags and PostgreSQL references from the CI workflow to align it with the SQLite-only codebase architecture.

## Tickets by Phase

### Phase 1: Fix CI Workflow Configuration (3 tickets)

Objective: Update test.yml to remove non-existent feature flags and PostgreSQL jobs

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| CICLEAN-1001 | Remove PostgreSQL jobs from CI workflow | Open | code-editor |
| CICLEAN-1002 | Remove feature flags from Rust jobs | Open | code-editor |
| CICLEAN-1003 | Update workflow header documentation | Open | code-editor |

**Phase 1 Deliverables**:
- PostgreSQL test jobs removed (test-postgres, test-rust-postgres)
- Feature flags removed from all cargo commands
- Job renamed: test-rust-sqlite → test-rust
- Workflow documentation updated to reflect SQLite-only architecture

**Estimated Time**: 2-4 hours

---

### Phase 2: Fix E2E Test Script and Helper Files (3 tickets)

Objective: Remove feature flag usage from test scripts and documentation

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| CICLEAN-2001 | Update E2E test script to remove feature flag | Open | code-editor |
| CICLEAN-2002 | Update test helper error messages | Open | code-editor |
| CICLEAN-2003 | Update testing documentation | Open | code-editor |

**Phase 2 Deliverables**:
- E2E script builds binary without `--features sqlite` flag
- Test helper error messages provide correct commands
- Documentation reflects actual build process

**Estimated Time**: 2-3 hours

---

### Phase 3: Validation and Verification (1 ticket)

Objective: Ensure all CI checks pass after changes

| Ticket ID | Title | Status | Agent |
|-----------|-------|--------|-------|
| CICLEAN-3001 | Local validation and verification | Open | bash-agent |

**Phase 3 Deliverables**:
- All local validations pass (cargo check, cargo test, E2E, MCP tests)
- Confirmation of no regressions
- Summary of changes and validation results

**Estimated Time**: 1-2 hours

---

## Total Project Summary

**Total Tickets**: 7
- Phase 1: 3 tickets
- Phase 2: 3 tickets
- Phase 3: 1 ticket

**Estimated Total Time**: 5-9 hours

**Key Dependencies**:
- Phase 2 depends on Phase 1 (E2E script changes need workflow fixes)
- Phase 3 depends on Phases 1+2 (validation requires all changes)

## Success Metrics

- [ ] CI workflow test.yml is valid YAML
- [ ] All PostgreSQL jobs removed from workflow
- [ ] All cargo commands use no feature flags
- [ ] E2E test script builds binary successfully
- [ ] Local `cargo check` passes
- [ ] Local `cargo test` passes
- [ ] Local E2E tests pass
- [ ] MCP TypeScript tests pass
- [ ] CI runs in ~8-10 minutes (vs 15 minutes before)

## Files Modified

**CI Configuration**:
- `.github/workflows/test.yml` - Remove PostgreSQL jobs, remove feature flags, update docs

**Test Scripts**:
- `tests/e2e/test_sqlite_flow.sh` - Remove feature flag from build command

**Test Helpers**:
- `packages/maproom-mcp/tests/helpers/sqlite.ts` - Update error messages

**Documentation**:
- `docs/testing/SQLITE_INTEGRATION_TESTS.md` - Update fixture generation commands

## Planning References

- **Plan**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/plan.md`
- **Architecture**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/architecture.md`
- **Quality Strategy**: `/home/vscode/.crewchief/worktrees/audit/.crewchief/projects/CICLEAN_ci-workflow-cleanup/planning/quality-strategy.md`

## Completion Criteria

Project is complete when:
1. All 7 tickets have `[x] Verified` checkbox checked
2. All local validation passes (CICLEAN-3001)
3. CI runs successfully on PR
4. No regressions in existing functionality
