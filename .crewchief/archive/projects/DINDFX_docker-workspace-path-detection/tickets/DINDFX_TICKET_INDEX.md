# DINDFX Ticket Index

**Project:** Docker-in-Docker Workspace Path Detection
**Project Slug:** DINDFX
**Total Tickets:** 7
**Status:** Ready for Execution

---

## Project Overview

Enable automatic workspace path detection in devcontainer environments so `npx @crewchief/maproom-mcp setup` works without manual configuration.

**Approach:** Test-driven development - write failing tests first, then implement code to make them pass.

---

## Phase 1: Test Foundation (Write Failing Tests)

### DINDFX-1001: Write failing tests for workspace path detection
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 2-3 hours
**Dependencies:** None (starting point)

**Summary:** Create comprehensive unit and integration tests (18 total: 15 unit + 3 integration) that prove we understand the problem. All tests should fail initially with "function not defined" errors.

**Deliverables:**
- `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts` (unit tests)
- `packages/maproom-mcp/tests/integration/workspace-path-detection.int.test.ts` (integration tests)
- Mocking strategy for fs, execFileSync, spawn

**Plan Reference:** plan.md Phase 1

---

## Phase 2: Implementation (Make Tests Pass)

### DINDFX-2001: Implement isInsideDocker() function
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 0.5 hours
**Dependencies:** DINDFX-1001

**Summary:** Implement Docker container detection by checking for /.dockerenv, /run/.containerenv, and cgroup patterns. Makes 5 failing tests pass.

**Deliverables:**
- Add `isInsideDocker()` function to `bin/cli.cjs`
- Check for Docker marker files and cgroup
- Graceful error handling

**Plan Reference:** plan.md Phase 2 Step 2.1

---

### DINDFX-2002: Implement getWorkspaceHostPath() with execFileSync
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 1 hour
**Dependencies:** DINDFX-1001, DINDFX-2001

**Summary:** Implement the core discovery function using `execFileSync()` (security-safe) to inspect Docker mounts and find the actual host path for /workspace. Makes 5 failing tests pass.

**Deliverables:**
- Import `execFileSync` from child_process
- Add `getWorkspaceHostPath()` function to `bin/cli.cjs`
- Use array args for docker commands (no shell)
- Timeouts: 5s (hostname), 10s (docker inspect)
- Buffer limits: 1KB (hostname), 10KB (docker inspect)

**Plan Reference:** plan.md Phase 2 Step 2.2

---

### DINDFX-2003: Implement resolveWorkspacePath() with priority logic
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 0.5-1 hour
**Dependencies:** DINDFX-1001, DINDFX-2001, DINDFX-2002

**Summary:** Implement path resolution with three-tier priority: 1) User override, 2) Docker auto-detect, 3) Host cwd. Uses existing diagnosticLog() function. Makes 5 failing tests pass.

**Deliverables:**
- Add `resolveWorkspacePath()` function to `bin/cli.cjs`
- Three-tier priority system
- Use existing diagnosticLog (lines 95-102)
- Warning messages for detection failures

**Plan Reference:** plan.md Phase 2 Step 2.3

---

### DINDFX-2004: Integrate resolveWorkspacePath into runSetup()
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 0.5 hour
**Dependencies:** DINDFX-1001, DINDFX-2001, DINDFX-2002, DINDFX-2003

**Summary:** Integrate path resolution into setup flow by calling `resolveWorkspacePath()` before `startDockerCompose()`. Sets `process.env.WORKSPACE_HOST_PATH` for volume mounting. Makes 3 integration tests pass.

**Deliverables:**
- Modify `runSetup()` in `bin/cli.cjs`
- Call `resolveWorkspacePath()` after setupConfigDirectory
- Set `process.env.WORKSPACE_HOST_PATH`
- Add console output: `✓ Workspace path: <path>`
- All 18 tests (15 unit + 3 integration) pass

**Plan Reference:** plan.md Phase 2 Step 2.4

---

## Phase 3: Path Validation & Security Testing

### DINDFX-3001: Add path validation and security tests
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 0.5-1 hour
**Dependencies:** DINDFX-2004

**Summary:** Add minimal path validation (warn about `..` and relative paths) and create security test cases to verify execFileSync prevents shell injection.

**Deliverables:**
- Add validation to `resolveWorkspacePath()` (warn not block)
- Security test cases for path traversal, relative paths, shell injection
- All security tests pass

**Plan Reference:** plan.md Phase 3

---

## Phase 4: Manual Testing & Validation

### DINDFX-4001: Manual testing in devcontainer and host environments
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 4-5 hours
**Dependencies:** DINDFX-3001

**Summary:** Manually verify solution works in actual devcontainer and host environments. Focus on CRITICAL devcontainer scenario (the primary problem being solved).

**Deliverables:**
- Test 4.1 (CRITICAL): Devcontainer setup without manual config
- Test 4.2 (Important): Host machine setup
- Document any issues found
- Verify MCP tools work after setup

**Plan Reference:** plan.md Phase 4

---

## Phase 5: Documentation & Cleanup

### DINDFX-5001: Update documentation for workspace path detection
**Status:** Pending
**Agent:** general-purpose
**Estimated Effort:** 1 hour
**Dependencies:** DINDFX-4001

**Summary:** Update README with devcontainer support documentation, troubleshooting guide, and manual override instructions.

**Deliverables:**
- Add "Devcontainer Support" section to README.md
- Troubleshooting guide for path detection issues
- Document WORKSPACE_HOST_PATH override option
- Review JSDoc comments for completeness

**Plan Reference:** plan.md Phase 5

---

## Execution Order

**Sequential execution recommended:**
1. DINDFX-1001 (Write tests)
2. DINDFX-2001 (Implement isInsideDocker)
3. DINDFX-2002 (Implement getWorkspaceHostPath)
4. DINDFX-2003 (Implement resolveWorkspacePath)
5. DINDFX-2004 (Integrate with runSetup)
6. DINDFX-3001 (Add path validation & security tests)
7. DINDFX-4001 (Manual testing)
8. DINDFX-5001 (Documentation)

**Critical Path:**
- DINDFX-1001 → DINDFX-2001 → DINDFX-2002 → DINDFX-2003 → DINDFX-2004
- This sequence ensures TDD methodology (tests first, then implementation)

**Test Verification Points:**
- After DINDFX-1001: All 18 tests fail
- After DINDFX-2001: 5 tests pass (isInsideDocker suite)
- After DINDFX-2002: 10 tests pass (+ getWorkspaceHostPath suite)
- After DINDFX-2003: 15 tests pass (+ resolveWorkspacePath suite)
- After DINDFX-2004: 18 tests pass (+ integration suite)
- After DINDFX-3001: All tests + security tests pass

---

## Success Metrics

**Primary Goal:** User runs `npx @crewchief/maproom-mcp setup --provider=openai` in devcontainer and it works without manual configuration.

**Technical Validation:**
- ✅ All 18+ automated tests pass
- ✅ Manual testing in devcontainer succeeds (DINDFX-4001)
- ✅ MCP tools can access workspace files
- ✅ No breaking changes to host execution

**Timeline:** 12-16 hours total (realistic estimate with debugging buffer)

---

## Planning Document References

- **Analysis:** `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/analysis.md`
- **Architecture:** `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/architecture.md`
- **Plan:** `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/plan.md`
- **Quality Strategy:** `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/quality-strategy.md`
- **Security Review:** `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/security-review.md`
- **Review Updates:** `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/review-updates.md`

---

## Next Actions

1. Review all tickets for clarity and completeness
2. Begin execution with DINDFX-1001 (write failing tests)
3. Follow TDD methodology: tests first, then implementation
4. Verify test results at each checkpoint
5. Complete manual testing before documentation phase

**Status:** Ready to begin implementation with DINDFX-1001
