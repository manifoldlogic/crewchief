# Quality Strategy: CI Workflow Cleanup

## Testing Philosophy

Test for confidence, not coverage. Since this project only modifies CI configuration files and test scripts (no production code changes), the testing approach focuses on:

1. **Validation over testing**: Ensure configurations are syntactically correct
2. **Local smoke testing**: Run the same checks CI will run
3. **Fast feedback**: Test locally before pushing to avoid CI churn

## Test Types

### Configuration Validation

**Scope:** YAML syntax, shell script syntax, TypeScript syntax
**Tools:**
- `yamllint` for YAML validation
- `shellcheck` for bash script validation
- `tsc` for TypeScript compilation
**Coverage Target:** 100% of modified files (automated validation)

**Commands:**
```bash
# YAML validation
yamllint .github/workflows/test.yml

# Shell script validation
shellcheck tests/e2e/test_sqlite_flow.sh

# TypeScript validation
cd packages/maproom-mcp
pnpm build  # Will fail if syntax errors
```

### Integration Tests

**Scope:** CI workflow jobs execute correctly (via local simulation)
**Approach:** Run the same commands CI will run, in the same order

**Commands (matching CI jobs):**
```bash
# Simulate test-rust job
cd crates/maproom
cargo check
cargo test -- --test-threads=1

# Simulate test-sqlite-e2e job
cd ../../
./tests/e2e/test_sqlite_flow.sh

# Simulate test-mcp-sqlite job
cd packages/maproom-mcp
pnpm test

# Simulate test-typescript job
cd ../cli && pnpm test
cd ../vscode-maproom && pnpm test
cd ../daemon-client && pnpm test
```

### End-to-End Tests

**Scope:** Full CI workflow simulation (critical paths only)
**Approach:** Run E2E test script that builds binary and tests CLI commands

**Critical path:**
```bash
# This is the highest-risk change (E2E script feature flag removal)
./tests/e2e/test_sqlite_flow.sh
```

## Critical Paths

The following paths MUST be tested before considering this complete:

1. **Rust compilation without features**: `cargo check` must pass
2. **Rust tests without features**: `cargo test` must pass
3. **E2E binary build**: Binary must build without `--features sqlite` flag
4. **E2E test execution**: Full test script must run successfully
5. **MCP fixture generation**: Fixture generation without features must work

## Test Data Strategy

**Test fixtures:**
- Existing fixture: `crates/maproom/tests/fixtures/pre-indexed-maproom.db`
- No new fixtures needed
- Fixture generation command updated to remove `--features sqlite`

**Test data locations:**
- E2E tests use fixture copy in `/tmp`
- MCP tests use fixture directly (read-only)
- No test data cleanup needed (E2E script handles it)

## Quality Gates

Before ticket verification:
- [ ] YAML syntax valid (`yamllint` passes)
- [ ] Shell scripts valid (`shellcheck` passes or no new warnings)
- [ ] TypeScript compiles (`pnpm build` passes)
- [ ] Cargo check passes (`cargo check`)
- [ ] Cargo tests pass (`cargo test`)
- [ ] E2E tests pass (`./tests/e2e/test_sqlite_flow.sh`)
- [ ] MCP tests pass (`pnpm test` in maproom-mcp)
- [ ] No regressions (existing passing tests still pass)

## Regression Prevention

### What Could Break
1. **PostgreSQL rejection tests**: These should still pass (they test SQLite-only validation)
2. **Fixture generation**: Must work without `--features sqlite` flag
3. **Binary builds**: Must compile without features
4. **E2E test script**: Must build and run correctly

### How to Prevent
1. **Run full test suite locally**: Don't rely on CI for validation
2. **Test in isolation**: Use clean worktree to avoid local environment issues
3. **Check existing tests**: Verify TypeScript tests still pass

## Testing Tools

| Tool | Purpose | Usage |
|------|---------|-------|
| `yamllint` | YAML syntax | `yamllint .github/workflows/test.yml` |
| `shellcheck` | Bash syntax | `shellcheck tests/e2e/test_sqlite_flow.sh` |
| `cargo check` | Rust compilation | `cargo check` in crates/maproom |
| `cargo test` | Rust tests | `cargo test -- --test-threads=1` |
| `pnpm test` | TypeScript tests | In package directories |

## Local Validation Script

Create a one-command validation:

```bash
#!/bin/bash
# validate-ci-cleanup.sh

set -e

echo "=== CI Cleanup Validation ==="
echo

echo "1. Validating YAML syntax..."
yamllint .github/workflows/test.yml

echo "2. Validating shell scripts..."
shellcheck tests/e2e/test_sqlite_flow.sh || echo "Note: shellcheck warnings acceptable"

echo "3. Validating Rust compilation..."
cd crates/maproom
cargo check

echo "4. Running Rust tests..."
cargo test -- --test-threads=1

echo "5. Running E2E tests..."
cd ../../
./tests/e2e/test_sqlite_flow.sh

echo "6. Running MCP tests..."
cd packages/maproom-mcp
pnpm test

echo
echo "=== All validations passed! ==="
```

## Measurement Criteria

### Before Changes
- **Failing jobs**: 4 out of 6 (66% failure rate)
- **CI time**: ~15 minutes
- **PostgreSQL jobs**: 2 jobs with service containers

### After Changes (Success Criteria)
- **Passing jobs**: 4 out of 4 (100% pass rate)
- **CI time**: ~8-10 minutes (30-40% faster)
- **PostgreSQL jobs**: 0 (removed)

## What NOT to Test

Since this is configuration-only:
- No unit tests needed (no new logic)
- No code coverage metrics needed (no production code)
- No performance tests needed (CI time reduction is side effect)
- No security tests needed (no new attack surface)

## Documentation Testing

Updated documentation must be accurate:
- [ ] Workflow header comments match reality (SQLite-only, not dual-backend)
- [ ] Error messages provide correct commands (no `--features` flags)
- [ ] Documentation file reflects current build process
- [ ] Job summaries accurately describe what's being tested

## Sign-off Criteria

This project is ready for merge when:
1. All local validations pass (see Local Validation Script)
2. No YAML/shell syntax errors
3. No regressions in existing tests
4. Documentation updated and accurate
5. Verify-ticket agent confirms acceptance criteria met
