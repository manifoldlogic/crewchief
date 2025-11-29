# Ticket: DINDFX-2001: Implement isInsideDocker() function

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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
Implement the `isInsideDocker()` function to detect if the code is running inside a Docker container by checking for standard Docker marker files and cgroup patterns. This is Phase 2 Step 2.1 of the DINDFX project.

## Background
After writing failing tests in Phase 1 (DINDFX-1001), we now implement the first function: `isInsideDocker()`. This function detects if the code is running inside a Docker container by checking for standard Docker marker files (`/.dockerenv`, `/run/.containerenv`) and cgroup patterns. This is the foundational detection that determines whether we need to discover the host workspace path.

This ticket implements **Phase 2 Step 2.1** from the DINDFX project plan, following a TDD approach where we make the 5 failing tests pass.

## Acceptance Criteria
- [ ] Function `isInsideDocker()` added to `packages/maproom-mcp/bin/cli.cjs` with proper JSDoc comments
- [ ] All 5 unit tests for `isInsideDocker()` pass
- [ ] Function returns `true` when `/.dockerenv` exists
- [ ] Function returns `true` when `/run/.containerenv` exists (Podman compatibility)
- [ ] Function returns `true` when cgroup contains "docker" or "containerd"
- [ ] Function returns `false` when not in Docker (no markers found)
- [ ] Function returns `false` gracefully when cgroup read fails (no crash)
- [ ] Verification: `pnpm test isInsideDocker` shows 5/5 passing

## Technical Requirements
- Import `fs` module if not already imported: `const fs = require('fs');`
- Add function to `bin/cli.cjs` near other utility functions
- Implement detection logic with three checks in priority order:
  1. Check for `/.dockerenv` file (most reliable Docker marker)
  2. Check for `/run/.containerenv` file (Podman compatibility)
  3. Check `/proc/1/cgroup` as fallback (contains "docker" or "containerd")
- Graceful error handling for file read failures (return false, don't crash)
- JSDoc comments explaining detection logic

**Reference implementation:**
```javascript
/**
 * Check if currently running inside a Docker container
 * @returns {boolean} True if inside Docker, false otherwise
 */
function isInsideDocker() {
  // Check for /.dockerenv (most reliable)
  if (fs.existsSync('/.dockerenv')) {
    return true;
  }

  // Check for /run/.containerenv (Podman compatibility)
  if (fs.existsSync('/run/.containerenv')) {
    return true;
  }

  // Fallback: check cgroup
  try {
    const cgroup = fs.readFileSync('/proc/1/cgroup', 'utf8');
    if (cgroup.includes('docker') || cgroup.includes('containerd')) {
      return true;
    }
  } catch (error) {
    // If /proc/1/cgroup doesn't exist, we're probably not in Linux
    return false;
  }

  return false;
}
```

## Implementation Notes
- Detection logic is simple and straightforward - no complex parsing needed
- Graceful failures are critical - never crash on missing files
- The three-layer detection approach ensures compatibility:
  - `/.dockerenv` covers standard Docker containers
  - `/run/.containerenv` covers Podman containers
  - `/proc/1/cgroup` fallback covers edge cases and other container runtimes
- Platform-specific behavior is handled gracefully (non-Linux systems return false)
- See `planning/architecture.md` Component Design section 1 for detailed rationale
- See `planning/quality-strategy.md` Test suite 1 for test coverage details

## Dependencies
- **DINDFX-1001** must be complete (tests written and failing)

## Risk Assessment
- **Risk**: Platform-specific behavior on non-Linux systems
  - **Mitigation**: Graceful fallback for systems without `/proc/1/cgroup`
- **Risk**: Podman detection may not be needed in all environments
  - **Mitigation**: Acceptable - checking `/run/.containerenv` doesn't hurt and improves compatibility
- **Risk**: False positives in unusual container environments
  - **Mitigation**: Acceptable for MVP - focus on standard devcontainers, can refine later if needed

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` (add isInsideDocker function)

## Related Documents
- `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/plan.md` (Phase 2 Step 2.1)
- `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/architecture.md` (Component Design section 1)
- `.crewchief/projects/DINDFX_docker-workspace-path-detection/planning/quality-strategy.md` (Test suite 1)

## Estimated Effort
0.5 hours
