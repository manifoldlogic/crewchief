# Ticket: TESTFIX-1008: Verify CI Configuration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- github-actions-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Review and verify the CI configuration in `.github/workflows/test.yml` covers all packages with appropriate dependencies. Ensure no test coverage gaps exist between local and CI execution.

## Background
The CI workflow has 5 jobs: test-sqlite-e2e, test-mcp-sqlite, test-rust-sqlite, test-postgres, test-rust-postgres. This ticket verifies all packages are covered, dependencies are correct, and the workflow will pass once local tests are fixed. This is Phase 5 of the TESTFIX project - CI verification after fixing all local test issues in previous tickets.

## Acceptance Criteria
- [ ] All 4 TypeScript packages have CI test coverage (cli, mcp, daemon-client, vscode)
- [ ] **NEW: `test-cli-unit` job added** for CLI vitest unit tests
- [ ] **NEW: `test-vscode-extension` job added** for VSCode extension tests
- [ ] **NEW: `test-daemon-client` job added** for daemon-client tests
- [ ] Rust tests are covered for both SQLite and PostgreSQL features
- [ ] Dependencies are correctly specified (PostgreSQL service for database tests)
- [ ] No test commands in CI that will fail due to environment issues
- [ ] CI workflow has 8 jobs total (5 existing + 3 new)

## Technical Requirements

**CRITICAL: Current CI has coverage gaps. The following packages are NOT tested in CI:**
- `packages/cli` - vitest unit tests NOT run (only E2E shell script tests)
- `packages/vscode-maproom` - NO CI coverage at all
- `packages/daemon-client` - NO CI coverage at all

**Review existing jobs in test.yml:**

1. **test-sqlite-e2e**: CLI end-to-end **shell script** tests only
   - Runs `./tests/e2e/test_sqlite_flow.sh`
   - Does NOT run vitest unit tests in packages/cli

2. **test-mcp-sqlite**: MCP server tests with SQLite fixture
   - ✅ Runs `pnpm test:sqlite` in packages/maproom-mcp

3. **test-rust-sqlite**: Rust library tests
   - ✅ Runs `cargo test --features sqlite`

4. **test-postgres**: TypeScript PostgreSQL integration
   - ✅ Runs `pnpm test` in packages/maproom-mcp with PostgreSQL

5. **test-rust-postgres**: Rust PostgreSQL compilation
   - ✅ Runs `cargo test --features postgres`

**Add missing CI jobs:**

```yaml
# NEW: CLI Unit Tests
test-cli-unit:
  name: CLI Unit Tests
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: '20'
    - uses: pnpm/action-setup@v4
    - run: pnpm install --frozen-lockfile
    - run: pnpm --filter @crewchief/cli test

# NEW: VSCode Extension Tests
test-vscode-extension:
  name: VSCode Extension Tests
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: '20'
    - uses: pnpm/action-setup@v4
    - run: pnpm install --frozen-lockfile
    - run: pnpm --filter @crewchief/vscode-maproom test

# NEW: Daemon Client Tests
test-daemon-client:
  name: Daemon Client Tests
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: actions/setup-node@v4
      with:
        node-version: '20'
    - uses: pnpm/action-setup@v4
    - run: pnpm install --frozen-lockfile
    - run: pnpm --filter @crewchief/daemon-client test
```

## Implementation Notes
1. Read `.github/workflows/test.yml` thoroughly
2. Create checklist of all packages and their test coverage:
   - packages/cli (TypeScript)
   - packages/maproom-mcp (TypeScript)
   - packages/daemon-client (TypeScript)
   - packages/vscode-maproom (TypeScript)
   - crates/maproom (Rust)
3. Identify any gaps:
   - Missing packages
   - Missing test commands
   - Wrong paths
   - Missing dependencies (PostgreSQL service, build artifacts)
4. If changes needed, make minimal fixes to close coverage gaps
5. Document CI test matrix for future reference in ticket

**Do not:**
- Remove existing test jobs
- Make changes that could break currently working CI jobs
- Add unnecessary complexity to workflow

## Dependencies
- TESTFIX-1003 (Rust tests must compile)
- TESTFIX-1004 (Rust tests must pass)
- TESTFIX-1005 (CLI tests must pass)
- TESTFIX-1006 (VSCode tests must pass)
- TESTFIX-1007 (MCP/daemon-client configured)

## Risk Assessment
- **Risk**: CI changes may break working jobs
  - **Mitigation**: Only make necessary changes; test in branch first; review GitHub Actions syntax carefully

- **Risk**: Some tests may only fail in CI (environment differences)
  - **Mitigation**: Document any CI-specific requirements or known limitations in ticket completion notes

- **Risk**: CI may be missing important test coverage
  - **Mitigation**: Add missing test jobs if needed; ensure all packages have appropriate coverage

- **Risk**: PostgreSQL service configuration may be incorrect
  - **Mitigation**: Verify service configuration matches local development setup

## Files/Packages Affected
- `.github/workflows/test.yml` (primary - may need updates)
- Other workflow files if related (e.g., release workflows that run tests)
