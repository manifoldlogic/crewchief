# Execution Plan: CI Workflow Cleanup

## Overview

This project removes outdated Cargo feature flags and PostgreSQL references from the CI workflow to align it with the SQLite-only codebase. The work is organized into 3 focused phases, each delivering independently testable improvements.

## Phases

### Phase 1: Fix CI Workflow Configuration

**Objective:** Update test.yml to remove non-existent feature flags and PostgreSQL jobs

**Deliverables:**
- Remove `test-postgres` job (lines 234-360)
- Remove `test-rust-postgres` job (lines 362-401)
- Rename `test-rust-sqlite` to `test-rust` with updated commands
- Remove `--features sqlite` and `--features postgres` from all cargo commands
- Update workflow header documentation to reflect SQLite-only architecture
- Update job summary messages to remove PostgreSQL references

**Agent Assignments:**
- code-editor: Modify `.github/workflows/test.yml`
  - Delete PostgreSQL jobs (234-360, 362-401)
  - Rename test-rust-sqlite → test-rust
  - Remove --features flags from cargo commands (lines 161, 208, 213)
  - Update header comments (lines 1-23)
  - Update job summaries (lines 216-224)
- verify-ticket: Validate YAML syntax, check job names are correct

**Success Criteria:**
- YAML file is valid (no syntax errors)
- All PostgreSQL jobs removed
- All cargo commands use no feature flags
- Job names reflect SQLite-only reality

### Phase 2: Fix E2E Test Script and Helper Files

**Objective:** Remove feature flag usage from test scripts and documentation

**Deliverables:**
- Update `tests/e2e/test_sqlite_flow.sh` to build without features
- Update error messages in E2E script
- Update `packages/maproom-mcp/tests/helpers/sqlite.ts` error messages
- Update `docs/testing/SQLITE_INTEGRATION_TESTS.md` instructions

**Agent Assignments:**
- code-editor: Modify test files
  - `tests/e2e/test_sqlite_flow.sh` (lines 61, 73)
  - `packages/maproom-mcp/tests/helpers/sqlite.ts` (lines 49, 92)
  - `docs/testing/SQLITE_INTEGRATION_TESTS.md` (lines 62, 148)
- verify-ticket: Validate script syntax, check error messages are consistent

**Success Criteria:**
- E2E script builds binary without feature flags
- Error messages provide correct cargo commands
- Documentation matches actual build process

### Phase 3: Validation and Verification

**Objective:** Ensure all CI checks pass after changes

**Deliverables:**
- Local validation of all modified files
- Dry-run of E2E test script
- Confirmation that TypeScript tests still pass
- Summary of what changed and why

**Agent Assignments:**
- bash-agent: Run local validation
  - `cargo check` (verify compilation)
  - `cargo test -- --test-threads=1` (verify tests)
  - `./tests/e2e/test_sqlite_flow.sh` (verify E2E)
  - `cd packages/maproom-mcp && pnpm test` (verify MCP tests)
- verify-ticket: Confirm all checks pass, review changes

**Success Criteria:**
- `cargo check` passes without errors
- `cargo test` passes all tests
- E2E script runs successfully
- TypeScript MCP tests pass
- No regressions in existing functionality

## Dependencies

### Phase Dependencies
- **Phase 2 depends on Phase 1**: E2E script changes won't work until CI workflow is fixed
- **Phase 3 depends on Phases 1+2**: Validation requires all changes to be in place

### External Dependencies
- None - all changes are self-contained configuration updates

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| YAML syntax error breaks CI | Low | High | Use YAML linter before commit, test in separate branch |
| E2E script fails after changes | Medium | Medium | Test locally before pushing, have rollback ready |
| Unintended test breakage | Low | Medium | Run full test suite locally before commit |
| Documentation out of sync | Low | Low | Update docs in same commit as code changes |

## Success Metrics

- [x] CI workflow test.yml is valid YAML
- [ ] All PostgreSQL jobs removed from workflow
- [ ] All cargo commands use no feature flags
- [ ] E2E test script builds binary successfully
- [ ] Local `cargo check` passes
- [ ] Local `cargo test` passes
- [ ] Local E2E tests pass
- [ ] MCP TypeScript tests pass
- [ ] CI runs in ~8-10 minutes (vs 15 minutes before)

## Validation Steps

### Pre-Commit Validation
```bash
# Validate YAML syntax
yamllint .github/workflows/test.yml

# Validate Rust compilation
cd crates/maproom
cargo check
cargo test -- --test-threads=1

# Validate E2E tests
cd ../../
./tests/e2e/test_sqlite_flow.sh

# Validate MCP tests
cd packages/maproom-mcp
pnpm test
```

### Post-Merge Validation
- Monitor CI run on PR
- Verify all jobs pass
- Confirm CI time reduced
- Check job summaries for accuracy

## Rollback Plan

If issues arise after merge:
1. Revert the single commit containing all changes
2. Investigate specific failure
3. Create targeted fix
4. Re-apply incrementally

Risk of needing rollback: Very low (configuration-only changes)
