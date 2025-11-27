# Ticket: TESTFIX-1009: Full CI Validation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- github-actions-specialist
- verify-ticket
- commit-ticket

## Summary
Push all test fixes to a branch and verify the complete CI pipeline passes. Fix any CI-specific failures that don't reproduce locally.

## Background
This is the final ticket in the TESTFIX project. All local tests should pass at this point (tickets TESTFIX-1001 through TESTFIX-1008 complete). This ticket validates that the CI environment also passes all tests, ensuring no environment-specific issues exist. Any CI-specific failures (environment differences, missing dependencies, timing issues) are identified and fixed. Success means all 8 CI jobs pass with green checkmarks (5 original + 3 new jobs added in TESTFIX-1008).

This ticket implements the final validation step from the TESTFIX project plan, ensuring the entire test suite runs successfully in the CI environment before closing the project.

## Acceptance Criteria
- [ ] All changes pushed to a feature branch
- [ ] GitHub Actions workflow triggered successfully
- [ ] test-sqlite-e2e job passes
- [ ] test-mcp-sqlite job passes
- [ ] test-rust-sqlite job passes
- [ ] test-postgres job passes
- [ ] test-rust-postgres job passes
- [ ] test-cli-unit job passes (NEW - added in TESTFIX-1008)
- [ ] test-vscode-extension job passes (NEW - added in TESTFIX-1008)
- [ ] test-daemon-client job passes (NEW - added in TESTFIX-1008)
- [ ] All 8/8 CI jobs show green checkmarks
- [ ] Any CI-specific failures identified and fixed

## Technical Requirements

**Pre-Push Checklist:**
- All local tests pass (Rust and TypeScript)
- No uncommitted changes remaining
- Branch is based on latest main
- Build artifacts are gitignored (not committed)

**CI Monitoring:**
- Watch GitHub Actions dashboard for each job
- Capture logs from any failing jobs
- Identify root cause of CI-specific failures
- Document any environment differences discovered

**Common CI Failure Patterns to Address:**
- Missing build step (binary not built before test)
- Service not ready (PostgreSQL not yet accepting connections)
- Path differences (absolute vs relative paths)
- Timing issues (tests assume instant response)
- Environment variables not set properly
- Docker service health check failures
- Cache issues between job runs

## Implementation Notes

### Step 1: Pre-Flight Verification
1. Ensure all local tests pass:
   ```bash
   cd /workspace
   pnpm test
   cargo test --all
   ```
2. Verify no uncommitted changes:
   ```bash
   git status
   ```
3. Sync with latest main:
   ```bash
   git fetch origin
   git rebase origin/main
   ```

### Step 2: Branch and Push
1. Create feature branch if not exists:
   ```bash
   git checkout -b testfix/all-tests
   ```
2. Commit all changes with appropriate commit messages following Conventional Commits
3. Push branch:
   ```bash
   git push -u origin testfix/all-tests
   ```

### Step 3: Monitor CI
1. Navigate to GitHub Actions: `https://github.com/{org}/{repo}/actions`
2. Watch workflow run for the pushed branch
3. Monitor each of the 5 jobs:
   - test-sqlite-e2e
   - test-mcp-sqlite
   - test-rust-sqlite
   - test-postgres
   - test-rust-postgres

### Step 4: Fix CI Failures (if any)
If failures occur:
1. Download job logs from failed step
2. Identify failing step and error message
3. Reproduce locally if possible
4. Implement fix (common fixes below)
5. Commit fix with clear message
6. Push to trigger CI re-run
7. Repeat until all jobs pass

**Common Fixes:**
- Add missing build steps in workflow
- Add health checks for services
- Increase timeouts for slow operations
- Add wait-for scripts for service readiness
- Fix environment variable configuration
- Adjust paths to be CI-compatible

### Step 5: Success Validation
1. Verify all 5/5 jobs show green checkmarks
2. Review workflow summary for any warnings
3. Confirm no flaky test behavior (re-run if suspicious)
4. Document any CI-specific configuration added

## Dependencies
- TESTFIX-1001: Clean worktrees vitest config (must be complete)
- TESTFIX-1002: Verify test environment (must be complete)
- TESTFIX-1003: Fix Rust compilation (must be complete)
- TESTFIX-1004: Run Rust tests (must be complete)
- TESTFIX-1005: Fix CLI tests (must be complete)
- TESTFIX-1006: Fix VSCode tests (must be complete)
- TESTFIX-1007: Configure MCP daemon tests (must be complete)
- TESTFIX-1008: Verify CI config (must be complete)

All previous tickets must be complete with all local tests passing before this ticket can succeed.

## Risk Assessment

- **Risk**: CI failures that don't reproduce locally due to environment differences
  - **Mitigation**: Use GitHub Actions debug logging (`ACTIONS_STEP_DEBUG=true`); compare local vs CI environments; use `act` tool for local CI simulation if needed

- **Risk**: Flaky tests that pass sometimes but fail intermittently
  - **Mitigation**: Re-run workflow multiple times; add retries to workflow if needed; increase timeouts; identify and fix race conditions

- **Risk**: Service startup timing issues (PostgreSQL not ready when tests start)
  - **Mitigation**: Add health checks or wait-for scripts in workflow; use service readiness probes; add explicit wait steps

- **Risk**: Changes to main branch cause merge conflicts during rebase
  - **Mitigation**: Rebase on latest main before final push; resolve conflicts locally before pushing

- **Risk**: Workflow file changes break CI in unexpected ways
  - **Mitigation**: Test workflow changes in a separate branch first; use workflow validation tools; review workflow syntax carefully

- **Risk**: Cache corruption between CI runs
  - **Mitigation**: Clear caches if failures persist; add cache key versioning

## Files/Packages Affected

**Committed in previous tickets:**
- `packages/cli/` - Test files and fixes
- `packages/vscode-maproom/` - Test files and fixes
- `packages/maproom-mcp/` - Test configuration
- `crates/maproom/` - Rust test fixes
- Various configuration files

**Potentially modified in this ticket:**
- `.github/workflows/test.yml` - If CI-specific fixes needed
- `.github/workflows/*.yml` - Other workflow files if issues found
- Any source files requiring CI-specific adjustments
- Environment configuration files if CI environment differs
