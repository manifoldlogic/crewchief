# Ticket: BINPKG-2901: Test local validation script catches missing binaries

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- test-runner
- verify-ticket
- commit-ticket

## Summary
Test that the validation script (BINPKG-2001) and prepublishOnly hook (BINPKG-2002) correctly catch and block incomplete publishes. This verifies our safety net works before relying on it in production.

## Background
The validation script is critical - it's the last line of defense against publishing incomplete packages. Version 1.3.0 was published without linux-x64 binaries, causing production failures. We need to verify the validation script works in all scenarios: missing platform, corrupted binary, all binaries present. This test ticket ensures the safety mechanisms from BINPKG-2001 and BINPKG-2002 actually work as designed.

## Acceptance Criteria

### Test Scenario 1: Missing Platform Binary
- [ ] Temporarily move one platform directory (e.g., `mv bin/linux-x64 bin/linux-x64.bak`)
- [ ] Run `node scripts/validate-binaries.js` from workspace root
- [ ] Verify script exits with code 1 (failure)
- [ ] Verify clear error message indicates which platform is missing
- [ ] Restore directory (`mv bin/linux-x64.bak bin/linux-x64`)
- [ ] Document actual error message and exit code

### Test Scenario 2: Corrupted Binary (Too Small)
- [ ] Backup actual binary to safe location
- [ ] Create small dummy file (e.g., `echo "fake" > bin/linux-x64/crewchief-maproom`)
- [ ] Run validation script
- [ ] Verify script fails with "Binary too small" or similar error
- [ ] Restore actual binary from backup
- [ ] Document actual error message and exit code

### Test Scenario 3: All Binaries Present (Happy Path)
- [ ] Ensure all 4 platform binaries exist (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- [ ] Run validation script from workspace root
- [ ] Verify script exits with code 0 (success)
- [ ] Verify success message is displayed for each platform
- [ ] Document actual success output

### Test Scenario 4: prepublishOnly Hook Integration
- [ ] Navigate to `packages/maproom-mcp` directory
- [ ] Run `pnpm publish --dry-run` (safe, won't actually publish)
- [ ] Verify validation runs automatically before publish
- [ ] Verify dry-run publish proceeds (meaning validation passed)
- [ ] Document hook execution order and output

### Test Scenario 5: Document Results
- [ ] Create test results document with:
  - Test date/time
  - Each scenario's expected vs actual results
  - Exit codes observed
  - Error messages captured
  - Any edge cases discovered
  - Recommendations for validation script improvements

## Technical Requirements

### Test Environment Setup
- **Test location**: Run from `/workspace` (repository root)
- **Validation script**: `node scripts/validate-binaries.js`
- **Package directory**: `packages/maproom-mcp`
- **Binary locations**: `packages/maproom-mcp/bin/<platform>/crewchief-maproom`

### Platform Requirements
All 4 platforms must be tested:
1. `linux-x64` - x86_64-unknown-linux-gnu
2. `linux-arm64` - aarch64-unknown-linux-gnu
3. `darwin-x64` - x86_64-apple-darwin
4. `darwin-arm64` - aarch64-apple-darwin

### Exit Code Verification
- **Success**: Exit code 0
- **Failure**: Exit code 1 (non-zero)
- **Check command**: `echo $?` immediately after validation script

### Safety Measures
- **CRITICAL**: Always use `--dry-run` flag with publish commands
- **Backup binaries**: Create backup directory before destructive tests
  ```bash
  mkdir -p /tmp/binpkg-test-backup
  cp -r packages/maproom-mcp/bin/* /tmp/binpkg-test-backup/
  ```
- **Restore after tests**: Copy binaries back from backup
  ```bash
  cp -r /tmp/binpkg-test-backup/* packages/maproom-mcp/bin/
  ```

### Test Script Pattern (Example)
```bash
#!/bin/bash
set -e  # Exit on error

# Backup
mkdir -p /tmp/binpkg-test-backup
cp -r packages/maproom-mcp/bin/* /tmp/binpkg-test-backup/

# Cleanup function
cleanup() {
  echo "Restoring binaries..."
  cp -r /tmp/binpkg-test-backup/* packages/maproom-mcp/bin/
}
trap cleanup EXIT  # Always restore on exit

# Test scenarios...
# (Each test goes here)
```

## Implementation Notes

### Testing Strategy
- **Non-destructive**: Always backup before modifying binaries
- **Fast iteration**: Use symbolic links for quicker testing if needed
- **Exit code validation**: Critical for npm/pnpm integration - test thoroughly
- **Output capture**: Save stdout/stderr for documentation

### Expected Validation Behavior
Based on BINPKG-2001 implementation, the script should:
1. Check for existence of each platform directory
2. Verify binary file exists in each platform directory
3. Check binary file size (>1MB for real binaries)
4. Optionally verify binary is executable
5. Report clear success/failure messages
6. Exit with appropriate code (0=success, 1=failure)

### Edge Cases to Test
- Missing directory vs empty directory
- Binary file present but zero bytes
- Binary file present but very small (<1KB)
- Binary file present but not executable
- Multiple platforms missing simultaneously

### Bug Handling
If validation doesn't work as expected:
1. Document the failure mode clearly
2. Create a bug ticket (BINPKG-2902 or next available)
3. Assign to general-purpose agent for fixing
4. Retest after fix is applied

### Documentation Output
Create test results document at:
- **File**: `.crewchief/projects/BINPKG_binary-packaging/testing/validation-test-results.md`
- **Format**: Markdown with clear sections for each scenario
- **Include**:
  - Test execution date/time
  - Environment details (Node version, OS, etc.)
  - Expected vs actual results table
  - Copy/paste of actual output
  - Exit codes observed
  - Recommendations for improvements

## Dependencies

### Required Tickets (Must be complete)
- **BINPKG-2001**: Validation script implementation
- **BINPKG-2002**: prepublishOnly hook integration

### Optional Dependencies
- BINPKG-1002-1005: Platform binaries (should already exist from Phase 1)

## Risk Assessment

### Risk 1: Accidentally Publishing Test Version
- **Likelihood**: Low
- **Impact**: High (unwanted npm publish)
- **Mitigation**:
  - ALWAYS use `--dry-run` flag
  - Never use actual `pnpm publish` without dry-run
  - Test in isolated environment first
  - Double-check commands before running

### Risk 2: Forgetting to Restore Binaries
- **Likelihood**: Medium
- **Impact**: High (broken local development)
- **Mitigation**:
  - Use bash `trap` pattern for automatic cleanup
  - Create backup before every destructive test
  - Document restore commands clearly
  - Test restore process before running destructive tests

### Risk 3: Validation Script Has Bugs
- **Likelihood**: Medium
- **Impact**: High (false confidence in safety net)
- **Mitigation**:
  - Test all edge cases thoroughly
  - Document any failures clearly
  - Create bug tickets for issues found
  - Retest after fixes applied
  - Don't proceed to CI testing (BINPKG-1901) until local tests pass

### Risk 4: Test Results Not Documented
- **Likelihood**: Low
- **Impact**: Medium (no record of what was tested)
- **Mitigation**:
  - Create results document as part of acceptance criteria
  - Include actual output, not just "passed/failed"
  - Document environment details
  - Save test script for repeatability

## Files/Packages Affected

### Files to Read (Reference)
- `/workspace/scripts/validate-binaries.js` - Script being tested
- `/workspace/packages/maproom-mcp/package.json` - Check prepublishOnly hook
- `/workspace/packages/maproom-mcp/bin/` - Binary directories to test

### Files to Create
- `/workspace/.crewchief/projects/BINPKG_binary-packaging/testing/validation-test-results.md` - Test results document
- `/workspace/.crewchief/projects/BINPKG_binary-packaging/testing/test-validation.sh` - Optional: Reusable test script

### Files to Temporarily Modify (with restore)
- `/workspace/packages/maproom-mcp/bin/linux-x64/crewchief-maproom` - Test corruption scenario
- `/workspace/packages/maproom-mcp/bin/linux-x64/` - Test missing platform scenario

### Directories to Backup
- `/workspace/packages/maproom-mcp/bin/` - All platform binaries

## Estimated Effort
**1-2 hours** - Comprehensive testing with documentation

Breakdown:
- 15 min: Setup backup/restore infrastructure
- 15 min: Test Scenario 1 (missing platform)
- 15 min: Test Scenario 2 (corrupted binary)
- 10 min: Test Scenario 3 (happy path)
- 15 min: Test Scenario 4 (prepublishOnly hook)
- 30 min: Document results and create test report
- 10 min: Cleanup and verification

## Priority
**High** - Critical validation of safety mechanisms before trusting them in production and CI

## Related Tickets

### Dependencies (Must complete before this ticket)
- **BINPKG-2001**: Create local validation script
- **BINPKG-2002**: Add prepublishOnly hook to package.json

### Blocked Tickets (Should wait for this ticket)
- **BINPKG-1901**: Canary release integration test (shouldn't run CI tests until local validation proven)

### Related Testing
- BINPKG-1901: CI-level testing (after local validation proven)

### Potential Bug Tickets
- BINPKG-2902+: Any bugs found during testing

## Reference Documentation

### Planning Documents
- **Project plan**: `.crewchief/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 2: Local Validation)
- **Architecture**: `.crewchief/projects/BINPKG_binary-packaging/planning/architecture.md` (Testing strategy)

### External References
- **npm publish lifecycle scripts**: https://docs.npmjs.com/cli/v10/using-npm/scripts#life-cycle-scripts
- **Exit codes in scripts**: Standard Unix convention (0=success, non-zero=failure)
- **Bash trap command**: https://tldp.org/LDP/Bash-Beginners-Guide/html/sect_12_02.html

### Related Code
- Validation script: `scripts/validate-binaries.js` (from BINPKG-2001)
- Package config: `packages/maproom-mcp/package.json` (prepublishOnly hook from BINPKG-2002)
