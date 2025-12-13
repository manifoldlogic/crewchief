# Project: CI Workflow Cleanup

**Slug:** CICLEAN
**Status:** Completed
**Created:** 2025-12-09
**Completed:** 2025-12-13

## Summary

Clean up the CI test workflow (`.github/workflows/test.yml`) to remove outdated Cargo feature flags (`--features sqlite`, `--features postgres`) and PostgreSQL test jobs that reference code and backends that no longer exist in the codebase.

## Problem Statement

The CI workflow is failing on PR #19 because it references Cargo features (`sqlite`, `postgres`) that were removed when the codebase was migrated to SQLite-only. Additionally, two entire test jobs (`test-postgres`, `test-rust-postgres`) are trying to test PostgreSQL integration, but PostgreSQL support was completely removed from the project.

### Current Failures
- **test-rust-sqlite**: ❌ `cargo check --features sqlite` fails (feature doesn't exist)
- **test-rust-postgres**: ❌ `cargo check --features postgres` fails (feature doesn't exist)
- **test-postgres**: ❌ PostgreSQL integration tests fail (backend removed)
- **test-sqlite-e2e**: ❌ E2E script uses `--features sqlite` (binary won't build)
- **test-mcp-sqlite**: ✅ Currently passing (but uses `--features sqlite` in conditional fixture generation)
- **test-typescript**: ✅ Passing (PostgreSQL rejection tests are valid)

## Proposed Solution

Remove all references to non-existent Cargo features and PostgreSQL test jobs from the CI workflow, E2E test scripts, and documentation. This is a pure configuration cleanup with no code changes.

### Changes Required
1. **CI Workflow** (`.github/workflows/test.yml`):
   - Remove `test-postgres` job (lines 234-360)
   - Remove `test-rust-postgres` job (lines 362-401)
   - Rename `test-rust-sqlite` → `test-rust` (remove "SQLite" from name)
   - Remove `--features sqlite|postgres` from all cargo commands
   - Update documentation comments to reflect SQLite-only architecture

2. **E2E Test Script** (`tests/e2e/test_sqlite_flow.sh`):
   - Remove `--features sqlite` from build command (line 73)
   - Update error messages to not reference features (line 61)

3. **Test Helper** (`packages/maproom-mcp/tests/helpers/sqlite.ts`):
   - Update error messages to not reference `--features sqlite` (lines 49, 92)

4. **Documentation** (`docs/testing/SQLITE_INTEGRATION_TESTS.md`):
   - Update fixture generation instructions (lines 62, 148)

### Expected Outcomes
- All CI checks pass (4/4 instead of 2/6)
- CI runs faster (~8-10 minutes vs ~15 minutes)
- No PostgreSQL service containers or credentials in workflow
- Clear documentation that SQLite is the only supported backend

## Relevant Agents

### Planning Phase
- **project-planner**: Created comprehensive planning documents

### Implementation Phase
- **code-editor**: Modify CI workflow, test scripts, documentation
- **bash-agent**: Run local validation (cargo check, cargo test, E2E tests)
- **verify-ticket**: Validate YAML syntax, confirm tests pass
- **commit-ticket**: Create commit for all changes

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis (6 failing jobs, root causes)
- [architecture.md](planning/architecture.md) - Solution design (remove features, remove PostgreSQL jobs)
- [plan.md](planning/plan.md) - Execution plan (3 phases: CI workflow, test scripts, validation)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach (local validation, no regressions)
- [security-review.md](planning/security-review.md) - Security assessment (no concerns, positive impact)

## Key Decisions

1. **Remove PostgreSQL entirely**: No attempt to preserve PostgreSQL jobs as placeholders
2. **Remove feature flags completely**: SQLite is compiled unconditionally, no flags needed
3. **Rename jobs for clarity**: "Rust Tests" instead of "Rust SQLite Tests" (since there's only one backend)
4. **Keep PostgreSQL rejection tests**: TypeScript tests that verify PostgreSQL URLs are rejected are intentional and valid

## Context

This project completes the SQLite-only migration that was done in previous projects:
- **SQLVEC** (sqlite-vec-backend): Migrated to SQLite with sqlite-vec extension
- **SQLITE** (full-sqlite-implementation): Implemented full SQLite backend
- **SQLFIX** (sqlite-backend-fixes): Fixed SQLite compilation issues
- **SQLINFRA** (infrastructure-simplification): Removed PostgreSQL infrastructure

The codebase has been fully migrated, but the CI workflow was not updated. This project fixes that gap.

## Next Steps

1. ✅ Planning complete (all documents filled)
2. ⏭️ **Recommended**: Run `/review-project CICLEAN` to validate planning
3. Then: Generate tickets with `/create-project-tickets CICLEAN`
4. Then: Execute work with `/work-on-project CICLEAN` or `/single-ticket CICLEAN-XXXX`

## Impact

**Immediate:**
- Unblocks PR #19
- Fixes failing CI checks
- Eliminates developer confusion about CI failures

**Long-term:**
- Faster CI runs (30-40% reduction)
- Clearer documentation of SQLite-only architecture
- Reduced CI resource usage (no PostgreSQL containers)
- Improved developer confidence in CI
