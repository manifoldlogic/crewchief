# Implementation Plan: Docker-in-Docker Workspace Path Detection

## Project Overview

**Goal:** Enable automatic workspace path detection in devcontainer environments so `npx @crewchief/maproom-mcp setup` works without manual configuration.

**Approach:** Test-driven development - write failing tests first, then implement code to make them pass.

**Timeline:** Single iteration, keep scope minimal.

## Execution Phases

### Phase 1: Test Foundation (Write Failing Tests)

**Objective:** Prove we understand the problem by writing tests that fail

**Agent:** General-purpose

**Deliverables:**
- [ ] Test file: `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`
- [ ] Test suite 1: `isInsideDocker()` - 5 test cases
- [ ] Test suite 2: `getWorkspaceHostPath()` - 5 test cases
- [ ] Test suite 3: `resolveWorkspacePath()` - 5 test cases
- [ ] Integration test file: `packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts`
- [ ] Integration suite: `runSetup() integration` - 3 test cases

**Success Criteria:**
- All tests written with clear GIVEN/WHEN/THEN structure
- All tests fail (functions don't exist yet)
- Tests cover happy path, error cases, and edge cases
- Mocking strategy implemented (fs, child_process)

**Estimated Effort:** 2-3 hours

**Verification:** Run `pnpm test workspace-path-detection` → all tests fail with "function not defined" errors

---

### Phase 2: Implementation (Make Tests Pass)

**Objective:** Implement minimal code to make all tests pass

**Agent:** General-purpose or docker-engineer

**Deliverables:**

#### Step 2.1: Implement Detection Function
- [ ] Add `isInsideDocker()` function to `bin/cli.cjs`
- [ ] Check for `/.dockerenv` file
- [ ] Check for `/run/.containerenv` file (Podman)
- [ ] Check `/proc/1/cgroup` as fallback
- [ ] Graceful error handling for file read failures
- [ ] Unit tests pass: `isInsideDocker()` suite ✅

#### Step 2.2: Implement Discovery Function
- [ ] Import `execFileSync` from child_process: `const { execFileSync } = require('child_process')`
- [ ] Add `getWorkspaceHostPath()` function to `bin/cli.cjs`
- [ ] Use `execFileSync('hostname', [], { encoding: 'utf8', timeout: 5000, maxBuffer: 1024 })`
- [ ] Use `execFileSync('docker', ['inspect', hostname, '--format', '...'], { encoding: 'utf8', timeout: 10000, maxBuffer: 10240 })`
- [ ] Add timeouts and buffer limits (5s/1KB for hostname, 10s/10KB for docker inspect)
- [ ] Return null on errors (graceful failure)
- [ ] Unit tests pass: `getWorkspaceHostPath()` suite ✅

#### Step 2.3: Implement Resolution Function
- [ ] Add `resolveWorkspacePath()` function to `bin/cli.cjs`
- [ ] Priority 1: Check for existing `WORKSPACE_HOST_PATH` env var
- [ ] Priority 2: Detect Docker-in-Docker and discover host path
- [ ] Priority 3: Use `process.cwd()` for host execution
- [ ] Use existing `diagnosticLog()` function (lines 95-102) for logging
- [ ] Log path resolution results through diagnosticLog (inherits redaction & conditional behavior)
- [ ] Unit tests pass: `resolveWorkspacePath()` suite ✅

#### Step 2.4: Integrate with Setup Flow
- [ ] Modify `runSetup()` function in `bin/cli.cjs`
- [ ] Call `resolveWorkspacePath()` before `startDockerCompose()`
- [ ] Set `process.env.WORKSPACE_HOST_PATH` with result
- [ ] Add console output: `✓ Workspace path: <path>`
- [ ] Integration tests pass ✅

**Success Criteria:**
- All unit tests pass (15 tests)
- All integration tests pass (3 tests)
- No new dependencies added
- Code follows existing style conventions
- JSDoc comments added to all functions
- execFileSync used from the start (security-safe implementation)

**Test Execution Order:**
1. Run `pnpm test isInsideDocker` → Must pass before moving to Step 2.2
2. Run `pnpm test getWorkspaceHostPath` → Must pass before moving to Step 2.3
3. Run `pnpm test resolveWorkspacePath` → Must pass before moving to Step 2.4
4. Run `pnpm test integration/workspace-path-detection` → Must pass to complete Phase 2

**Estimated Effort:** 3-4 hours

**Verification:** Run `pnpm test workspace-path-detection` → all tests pass ✅

---

### Phase 3: Path Validation & Security Testing

**Objective:** Add path validation and security test cases

**Agent:** General-purpose

**Deliverables:**
- [ ] Add path validation to `resolveWorkspacePath()`:
  - Check for `..` in path (path traversal prevention)
  - Warn (don't error) if path doesn't start with `/`
  - Don't verify path exists (host vs container filesystem)
- [ ] Add security test cases to test suite:
  - Test malicious path with `..` is rejected
  - Test relative paths trigger warning
  - Test command execution with special characters
- [ ] Security tests pass ✅

**Success Criteria:**
- Path traversal attempts blocked
- Security tests verify mitigations work
- execFileSync already used (no shell interpolation risk)
- Timeouts and buffer limits already configured in Phase 2

**Note:** execFileSync with timeouts/buffers implemented in Phase 2 (security-safe from the start)

**Estimated Effort:** 0.5-1 hour

**Verification:** Run security test suite → all tests pass

---

### Phase 4: Manual Testing & Validation

**Objective:** Verify solution works in real environments

**Agent:** General-purpose or docker-engineer

**Deliverables:**

#### Test 4.1: Devcontainer Environment (CRITICAL - Must Pass)
- [ ] Start fresh devcontainer
- [ ] Destroy existing maproom containers
- [ ] Run `npx @crewchief/maproom-mcp setup --provider=openai`
- [ ] Verify console shows: `✓ Workspace path: /host_mnt/...`
- [ ] Verify containers started successfully
- [ ] Verify volume mount: `docker inspect maproom-mcp | grep workspace`
- [ ] Verify file access: `docker exec maproom-mcp ls /workspace/packages`
- [ ] Reconnect to MCP: `/mcp`
- [ ] Test open tool: `mcp__maproom__open` with real file
- [ ] Test context tool: `mcp__maproom__context` with real chunk

#### Test 4.2: Host Environment (Important)
- [ ] Exit devcontainer (run on host machine)
- [ ] Run `npx @crewchief/maproom-mcp setup --provider=openai`
- [ ] Verify console shows: `✓ Workspace path: /Users/.../crewchief`
- [ ] Verify containers started
- [ ] Verify volume mount points to current directory
- [ ] Verify file access works

#### Test 4.3: User Override (Optional - Post-MVP)
- [ ] Set `export WORKSPACE_HOST_PATH=/custom/path`
- [ ] Run setup
- [ ] Verify console shows: `✓ Workspace path: /custom/path (user-provided)`
- [ ] Verify override is respected

#### Test 4.4: Error Cases (Optional - Post-MVP)
- [ ] Simulate docker inspect failure (disconnect docker socket temporarily)
- [ ] Verify warning message appears
- [ ] Verify setup continues with fallback
- [ ] Restore docker socket
- [ ] Verify normal operation resumes

**Success Criteria:**
- ✅ MUST: Setup works in devcontainer without manual config (Test 4.1)
- ✅ MUST: Setup works on host machine (Test 4.2)
- ⏸ NICE: User override mechanism works (Test 4.3)
- ⏸ NICE: Error cases handled gracefully with clear messages (Test 4.4)
- ✅ MUST: MCP tools (open, context) work after setup

**Estimated Effort:** 4-5 hours (realistic estimate accounting for debugging)

**Verification:** Complete manual testing checklist ✅

---

### Phase 5: Documentation & Cleanup

**Objective:** Document the solution and update related files

**Agent:** General-purpose

**Deliverables:**
- [ ] Update `DOCKER_WORKSPACE_SOLUTION.md` with implemented approach
- [ ] Update `packages/maproom-mcp/README.md` with devcontainer support section
- [ ] Add troubleshooting section for path detection failures
- [ ] Document `WORKSPACE_HOST_PATH` environment variable override
- [ ] Add comments to code explaining detection logic
- [ ] Update changelog/release notes
- [ ] Review and close related issues/tickets

**Success Criteria:**
- Documentation clear and accurate
- Users can troubleshoot common issues
- Future maintainers understand the implementation
- No outdated documentation remains

**Estimated Effort:** 1 hour

**Verification:** Documentation review

---

## Agent Assignments

### Primary Agent: General-Purpose

**Responsibilities:**
- Write all test files
- Implement detection functions
- Integrate with setup flow
- Apply security mitigations
- Update documentation

**Skills Required:**
- TypeScript/JavaScript
- Vitest test framework
- Docker CLI knowledge
- Child process management
- Security best practices

### Supporting Agents (If Needed)

**unit-test-runner:**
- Execute tests after each implementation step
- Report test results
- No code modifications

**verify-ticket:**
- Verify all acceptance criteria met
- Validate implementation matches plan
- Ensure no regressions

---

## Testing Milestones

### Milestone 1: Tests Written
- [ ] All 18 tests written (15 unit + 3 integration)
- [ ] All tests fail with expected errors
- [ ] Mocking strategy validated

### Milestone 2: Unit Tests Pass
- [ ] `isInsideDocker()` tests pass (5/5)
- [ ] `getWorkspaceHostPath()` tests pass (5/5)
- [ ] `resolveWorkspacePath()` tests pass (5/5)

### Milestone 3: Integration Tests Pass
- [ ] `runSetup()` integration tests pass (3/3)
- [ ] Environment variable propagation verified
- [ ] Docker compose spawn receives correct env

### Milestone 4: Security Tests Pass
- [ ] Command injection tests pass
- [ ] Path traversal tests pass
- [ ] DoS mitigation tests pass
- [ ] Buffer limit tests pass

### Milestone 5: Manual Tests Complete
- [ ] Devcontainer setup succeeds
- [ ] Host setup succeeds
- [ ] User override works
- [ ] Error cases handled
- [ ] MCP tools work

---

## Security Checkpoints

### Checkpoint 1: After Implementation (Phase 2)
- [ ] Review code for shell injection risks
- [ ] Verify no untrusted input used directly in shell commands
- [ ] Check for path traversal vulnerabilities

### Checkpoint 2: After Security Hardening (Phase 3)
- [ ] Verify `execFileSync()` used instead of `execSync()`
- [ ] Confirm all exec calls have timeouts
- [ ] Validate buffer limits applied
- [ ] Run security test suite

### Checkpoint 3: Before Manual Testing (Phase 4)
- [ ] Review diagnostic logging for information disclosure
- [ ] Verify read-only mount still configured
- [ ] Confirm graceful failure paths work

---

## Rollback Strategy

**If implementation causes regressions:**

1. **Immediate:** Revert commit to previous working state
2. **User workaround:** Document manual `WORKSPACE_HOST_PATH` export
3. **Investigation:** Review failed tests and error logs
4. **Fix:** Address root cause, add regression tests
5. **Re-deploy:** Test thoroughly before re-releasing

**Rollback trigger conditions:**
- Existing setup flows break
- Security vulnerability introduced
- Manual testing reveals critical bugs
- User reports cannot be resolved quickly

---

## Success Criteria Summary

### Must Have (MVP)
- ✅ All automated tests pass (18 tests)
- ✅ Setup works in devcontainer without manual config
- ✅ Setup works on host machine
- ✅ No regressions in existing functionality
- ✅ Security mitigations applied

### Nice to Have (Post-MVP)
- ⏸ E2E test in CI/CD
- ⏸ Windows/WSL2 support validated
- ⏸ Podman compatibility tested
- ⏸ Performance benchmarks documented

---

## Acceptance Criteria

**This project is complete when:**

1. ✅ User runs `npx @crewchief/maproom-mcp setup --provider=openai` in devcontainer
2. ✅ Setup completes without errors
3. ✅ Console shows: `✓ Workspace path: /host_mnt/.../crewchief`
4. ✅ Containers start successfully
5. ✅ MCP tools can access workspace files
6. ✅ All tests pass (unit, integration, security)
7. ✅ Manual testing checklist complete
8. ✅ Documentation updated
9. ✅ No security vulnerabilities introduced
10. ✅ No breaking changes to existing workflows

---

## Timeline Estimate

**Total Estimated Effort:** 12-16 hours (realistic, accounting for debugging and manual testing)

**Breakdown:**
- Phase 1 (Tests): 2-3 hours
- Phase 2 (Implementation): 3-4 hours
- Phase 3 (Path Validation & Security Testing): 0.5-1 hour
- Phase 4 (Manual Testing): 4-5 hours (includes debugging)
- Phase 5 (Documentation): 1 hour
- Buffer for unexpected issues: 1-2 hours

**Target Completion:** 1-2 development sessions (split at Phase 4 if needed)

---

## Next Steps

1. **Create ticket:** Generate implementation ticket from this plan
2. **Write tests:** Start with Phase 1 - failing tests
3. **Implement:** Make tests pass (Phase 2)
4. **Harden:** Apply security mitigations (Phase 3)
5. **Validate:** Manual testing (Phase 4)
6. **Document:** Update docs (Phase 5)
7. **Ship:** Merge and release

**Ready to begin:** Yes ✅
