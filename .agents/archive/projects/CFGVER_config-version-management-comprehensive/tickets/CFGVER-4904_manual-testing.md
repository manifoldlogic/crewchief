# Ticket: CFGVER-4904: Execute manual testing checklist for CLI integration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Execute comprehensive manual testing checklist to validate CLI integration on real systems (macOS and Linux). Automated tests don't cover user experience, edge cases, or platform differences. Manual testing ensures the complete flow works from the user's perspective.

## Background
Automated tests verify individual functions work correctly, but they don't catch:
- User experience issues (confusing messages, slow startup)
- Platform-specific problems (macOS vs Linux differences)
- Edge cases (Docker not running, permission denied, disk space)
- Integration issues (CLI + config-manager + Docker)

Manual testing validates that real users will have a smooth experience on real systems.

Reference: `quality-strategy.md` lines 153-175 for manual testing checklist.

## Acceptance Criteria
- [ ] All macOS checklist items verified and documented
- [ ] All Linux (devcontainer) checklist items verified and documented
- [ ] Edge case tests completed with results documented
- [ ] Screenshots or logs captured for each test scenario
- [ ] Issues discovered are documented with severity and reproduction steps
- [ ] Test report created in `.agents/projects/CFGVER_config-version-management/testing/manual-test-report.md`

## Technical Requirements
- Test on macOS (real host system)
- Test on Linux (devcontainer environment)
- Document test date, platform, versions
- Capture actual output (stdout/stderr) for each test
- Note any unexpected behavior or issues
- Create structured test report document

## Implementation Notes

### Manual Testing Checklist

**macOS Tests:**
- [ ] First run on clean system (no ~/.maproom-mcp/)
  - Expected: "⚡ Initializing Maproom configuration..." → "✅ Configuration initialized"
  - Verify: Version file created, config files copied, permissions correct

- [ ] Update from previous version (e.g., 1.2.2 → 1.2.3)
  - Expected: "⚡ Updating Maproom configuration..." → backup → update → "✅ Configuration updated successfully"
  - Verify: Backup created, version file updated, old config replaced

- [ ] Update with running containers (stops cleanly)
  - Expected: "Stopped containers" message appears
  - Verify: Containers stopped before config update

- [ ] Update with user .env file (preserves)
  - Expected: User .env file not overwritten
  - Verify: Custom environment variables still present after update

- [ ] Rollback works after failed update
  - Expected: "⚠️ Update failed, rolling back..." → "✅ Rollback successful"
  - Verify: Original config restored, version file reverted

- [ ] Error messages are clear and actionable
  - Expected: "❌ Update failed: [reason]" + recovery commands
  - Verify: Commands are copy-pasteable and work

- [ ] Progress messages are informative
  - Expected: Each step logged with clear description
  - Verify: User can understand what's happening

**Linux Tests (devcontainer):**
- [ ] First run on clean system
  - Same expectations as macOS
  - Verify: Linux permissions work (0o700, 0o600)

- [ ] Update from previous version
  - Same expectations as macOS
  - Verify: File paths work on Linux filesystem

- [ ] Update with running containers
  - Same expectations as macOS
  - Verify: Docker commands work in container

- [ ] Docker not running (error message)
  - Expected: "❌ Update failed: Cannot connect to Docker daemon"
  - Verify: Error message is clear, suggests starting Docker

- [ ] Permission denied (error message)
  - Expected: "❌ Update failed: Cache directory not writable"
  - Verify: Error message explains the issue

**Edge Case Tests:**
- [ ] Corrupted version file (triggers update)
  - Setup: Edit ~/.maproom-mcp/.maproom-version to invalid JSON
  - Expected: Detected as needing update, fixed automatically

- [ ] Modified config file (hash mismatch, triggers update)
  - Setup: Edit docker-compose.yml manually
  - Expected: Hash mismatch detected, config replaced

- [ ] Disk space low (error message)
  - Setup: Fill disk to near capacity (if possible)
  - Expected: Error message about disk space

- [ ] Network failure during npm install (outside our control)
  - Setup: Disconnect network during `npx -y @crewchief/maproom-mcp@latest`
  - Expected: npm error, not our error (document behavior)

### Test Procedure

For each test:
1. **Setup**: Prepare test scenario (e.g., clean system, old version, etc.)
2. **Execute**: Run `npx -y @crewchief/maproom-mcp@latest`
3. **Observe**: Capture full output (stdout and stderr)
4. **Verify**: Check expected result matches actual result
5. **Document**: Record pass/fail with notes and screenshots

### Test Report Template

Create: `.agents/projects/CFGVER_config-version-management/testing/manual-test-report.md`

```markdown
# Manual Testing Report: CFGVER CLI Integration

**Test Date**: YYYY-MM-DD
**Tester**: [Agent/Human Name]
**Package Version**: [Version tested]

## Test Environment
- **macOS Version**: [If applicable]
- **Linux Distribution**: [If applicable]
- **Docker Version**: [Version]
- **Node Version**: [Version]

## macOS Test Results

### Test 1: First Run on Clean System
- **Status**: ✅ PASS / ❌ FAIL
- **Output**: [Paste actual output]
- **Notes**: [Any observations]
- **Screenshot**: [If applicable]

[... Continue for each test ...]

## Linux Test Results

[Same format as macOS]

## Edge Case Test Results

[Same format]

## Issues Discovered

### Issue 1: [Title]
- **Severity**: Critical / High / Medium / Low
- **Description**: [What went wrong]
- **Reproduction Steps**: [How to reproduce]
- **Expected**: [What should happen]
- **Actual**: [What actually happened]
- **Recommendation**: [How to fix]

## Summary
- **Total Tests**: X
- **Passed**: Y
- **Failed**: Z
- **Issues Found**: N

## Conclusion
[Overall assessment of readiness]
```

## Dependencies
- **CFGVER-4001**: CLI integration must be complete
- **CFGVER-4002**: Progress messages must be implemented
- **CFGVER-4003**: Environment variable support for isolated testing
- **All previous tickets**: Complete implementation required for testing

## Risk Assessment
- **Risk**: Subjective pass/fail criteria
  - **Mitigation**: Document clear expectations for each test in checklist
  - **Impact**: Code review will validate test criteria

- **Risk**: Platform differences causing inconsistent results
  - **Mitigation**: Test both macOS and Linux, document differences
  - **Impact**: Expected - document as platform-specific behavior

- **Risk**: Edge cases difficult to reproduce
  - **Mitigation**: Best effort testing, document what's tested
  - **Impact**: Some edge cases may only be found in production

- **Risk**: Time-consuming manual process
  - **Mitigation**: Structured checklist and template streamline process
  - **Impact**: Acceptable for quality assurance

## Files/Packages Affected
- **Create**: `.agents/projects/CFGVER_config-version-management/testing/manual-test-report.md`
- **Test**: `packages/maproom-mcp/bin/cli.cjs` (CLI entry point)
- **Test**: `packages/maproom-mcp/src/config-manager.ts` (config management)
- **No code changes** (testing only)
