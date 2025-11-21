# Ticket: DINDFX-2003: Implement resolveWorkspacePath() with priority logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the `resolveWorkspacePath()` function that orchestrates Docker detection and host path discovery using a three-tier priority system: 1) User override via env var, 2) Auto-detect in Docker, 3) Use current directory on host. This function uses the existing `diagnosticLog()` function for logging and makes the failing test suite pass.

## Background
After implementing Docker detection (DINDFX-2001) and host path discovery (DINDFX-2002), we now implement the resolution function that brings these components together. This implements Phase 2 Step 2.3 from the DINDFX project plan.

The resolution function provides a robust priority system that respects user overrides while providing intelligent defaults for both Docker-in-Docker and host environments. It leverages the existing `diagnosticLog()` function (lines 95-102 of bin/cli.cjs) for logging, ensuring consistent diagnostic behavior across the codebase.

## Acceptance Criteria
- [ ] `resolveWorkspacePath()` function added to `packages/maproom-mcp/bin/cli.cjs` with proper JSDoc
- [ ] All 5 unit tests for `resolveWorkspacePath()` pass
- [ ] Returns user-provided WORKSPACE_HOST_PATH if set (Priority 1)
- [ ] Calls `isInsideDocker()` when no user override exists
- [ ] Calls `getWorkspaceHostPath()` when inside Docker
- [ ] Returns discovered host path when available
- [ ] Warns and falls back to '/workspace' if inside Docker but discovery fails
- [ ] Returns `process.cwd()` when running on host (not in Docker)
- [ ] Uses existing `diagnosticLog()` for all logging (not a new function)
- [ ] Diagnostic logs only appear when DIAGNOSTIC_MODE enabled or provider not set
- [ ] Verification: `pnpm test resolveWorkspacePath` shows 5/5 passing

## Technical Requirements

**Function Signature:**
```javascript
/**
 * Resolve the appropriate workspace path for the current environment
 * Handles devcontainer (Docker-in-Docker), host, and custom override scenarios
 * @returns {string} Workspace path to use for volume mounting
 */
function resolveWorkspacePath() {
  // Priority 1: User override (for custom setups)
  if (process.env.WORKSPACE_HOST_PATH) {
    diagnosticLog('Using user-provided WORKSPACE_HOST_PATH', {
      path: process.env.WORKSPACE_HOST_PATH
    });
    return process.env.WORKSPACE_HOST_PATH;
  }

  // Priority 2: Docker-in-Docker detection
  if (isInsideDocker()) {
    diagnosticLog('Detected running inside Docker container');

    const hostPath = getWorkspaceHostPath();

    if (hostPath) {
      diagnosticLog('Discovered host workspace path', {
        hostPath,
        source: 'docker inspect'
      });
      return hostPath;
    }

    // Inside Docker but couldn't find mount - warn and use /workspace
    console.warn('⚠️  Running inside Docker but could not discover workspace host path.');
    console.warn('    Volume mount may fail. Set WORKSPACE_HOST_PATH manually if needed.');
    return '/workspace';
  }

  // Priority 3: Running on host - use current directory
  const hostPath = process.cwd();
  diagnosticLog('Running on host, using current directory', { hostPath });
  return hostPath;
}
```

**Priority System:**
1. **Priority 1**: Check for `WORKSPACE_HOST_PATH` env var (user override)
2. **Priority 2**: Detect Docker-in-Docker and discover host path using `isInsideDocker()` and `getWorkspaceHostPath()`
3. **Priority 3**: Use `process.cwd()` for host execution

**Logging:**
- Use existing `diagnosticLog()` function (lines 95-102 of bin/cli.cjs)
- Do NOT create a new logging function
- Warning messages use `console.warn` (always visible to users)
- Diagnostic logs automatically respect DIAGNOSTIC_MODE and redact sensitive data

## Implementation Notes

**CRITICAL**: Use the existing `diagnosticLog()` function (lines 95-102 of bin/cli.cjs). Do NOT create a new one. The existing function already handles:
- Conditional logging based on DIAGNOSTIC_MODE or provider settings
- Automatic redaction of sensitive data (paths, secrets)
- Consistent JSON formatting

**Three-Tier Priority Rationale:**
- User control > Auto-detection > Safe fallback
- Allows manual overrides for edge cases
- Graceful degradation when detection fails
- See architecture.md Component Design section 3 for detailed rationale

**Error Handling:**
- Warning messages use `console.warn` (not diagnosticLog) so users always see them
- Fallback to '/workspace' inside Docker prevents hard failures
- Host execution uses cwd (most common case)

**Test Coverage:**
The existing test suite (`__tests__/cli.test.js`) includes 5 tests covering:
1. Priority 1: Returns user-provided WORKSPACE_HOST_PATH when set
2. Priority 2: Calls isInsideDocker() and getWorkspaceHostPath() when in Docker
3. Priority 2: Returns discovered host path when available
4. Priority 2: Warns and falls back to '/workspace' when discovery fails
5. Priority 3: Returns process.cwd() when running on host

## Dependencies
- **DINDFX-1001** must be complete (tests written and failing) ✅
- **DINDFX-2001** must be complete (isInsideDocker implemented) ✅
- **DINDFX-2002** must be complete (getWorkspaceHostPath implemented) ✅

## Risk Assessment
- **Risk**: Accidental creation of duplicate diagnosticLog function
  - **Mitigation**: Explicitly documented to use existing function at lines 95-102, verification step confirms no duplication
- **Risk**: Priority order confusion leading to incorrect path selection
  - **Mitigation**: Clear comments in code, comprehensive test coverage for all priority levels
- **Risk**: Silent failures when detection doesn't work
  - **Mitigation**: Warning messages for detection failures, explicit fallback behavior

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` (add resolveWorkspacePath function)
- `packages/maproom-mcp/__tests__/cli.test.js` (tests already exist, will verify they pass)
