# Ticket: [MRBIN-4001]: Integration Verification and Manual Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - full test suite passing
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Perform comprehensive integration verification including running the full test suite, manual testing with real configuration files across all scenarios, and confirming all project acceptance criteria are met with no regressions.

## Background
This is the final verification phase for the MRBIN project. All implementation (MRBIN-1001), testing (MRBIN-2001), and documentation (MRBIN-3001) tickets are complete. This ticket ensures everything works together correctly in real-world scenarios before project completion.

The verification includes automated tests, manual testing with actual config files, and cross-platform validation.

## Acceptance Criteria
- [ ] Full test suite passes (`pnpm test` - all tests)
- [ ] No TypeScript compilation errors (`pnpm build`)
- [ ] No linting errors (`pnpm lint`)
- [ ] Manual test scenario 1: Config with `maproomBinaryPath` → all commands use it
- [ ] Manual test scenario 2: `CREWCHIEF_MAPROOM_BIN` env var → overrides config
- [ ] Manual test scenario 3: No config, no env var → falls back to global/packaged
- [ ] Manual test scenario 4: Config with relative path → resolves correctly
- [ ] Manual test scenario 5: Config with invalid path → warns but continues with fallback
- [ ] Manual test scenario 6: `cleanMaproomRecords()` uses config in all call sites
- [ ] All project acceptance criteria verified (from plan.md)
- [ ] Documentation builds/renders correctly
- [ ] No regression in existing functionality

## Technical Requirements
- Run full test suite and verify all tests pass (including 26+ existing tests + 2-3 new ones)
- Run TypeScript compiler and verify no errors
- Run ESLint and verify no linting errors
- Create test configuration files for manual testing scenarios
- Execute commands manually to verify real-world behavior
- Test on primary platform (Linux/macOS)
- Verify Windows compatibility if possible (`.exe` suffix handling)
- Check error messages are clear and helpful
- Verify logging output is informative

## Implementation Notes

### Automated Verification

**1. Full test suite:**
```bash
pnpm test
# Expected: All tests pass
# Should include: 26 existing + 2-3 new cleanMaproomRecords tests
# Total: ~29 tests in clean-maproom-records.test.ts
```

**2. TypeScript compilation:**
```bash
pnpm build
# Expected: Build succeeds with no errors
# Verifies: Type safety, imports, exports all correct
```

**3. Linting:**
```bash
pnpm lint
# Expected: No linting errors
# Verifies: Code style consistency
```

### Manual Testing Scenarios

**Scenario 1: Config-based binary path**
```javascript
// crewchief.config.local.js
module.exports = {
  repository: {
    maproomBinaryPath: '/custom/path/to/maproom'
  }
}
```
Test commands:
- `crewchief maproom scan`
- `crewchief worktree:scan`
- `crewchief worktree:clean`

Expected: All commands attempt to use `/custom/path/to/maproom`

**Scenario 2: Environment variable override**
```bash
export CREWCHIEF_MAPROOM_BIN=/env/var/maproom
# Config file also present with different path
crewchief maproom scan
```
Expected: Uses env var path, not config path

**Scenario 3: Fallback behavior**
```bash
# No config file, no env var
crewchief maproom scan
```
Expected: Falls back to global install or packaged binary

**Scenario 4: Relative path resolution**
```javascript
// crewchief.config.local.js
module.exports = {
  repository: {
    maproomBinaryPath: './bin/maproom'
  }
}
```
Expected: Resolves relative to project root (with CWD caveat for cleanMaproomRecords)

**Scenario 5: Invalid path handling**
```javascript
// crewchief.config.local.js
module.exports = {
  repository: {
    maproomBinaryPath: '/nonexistent/path/maproom'
  }
}
```
Expected: Logs warning, falls back to global/packaged, command continues

**Scenario 6: cleanMaproomRecords integration**
```bash
# Test all three call sites that use cleanMaproomRecords:
crewchief worktree:clean
crewchief worktree:prune
crewchief worktree:use --clean <branch>
```
Expected: All commands respect config-based binary path

### Project Acceptance Criteria Verification

From plan.md, verify:
- [x] Config accepts `maproomBinaryPath` setting (already done)
- [ ] Config path takes precedence over packaged binary (verify in manual tests)
- [x] Env var takes highest precedence (already implemented, verify in tests)
- [x] Global install checked before packaged binary (already correct, verify in tests)
- [ ] Binary resolution is consistent across all commands (test cleanMaproomRecords)
- [ ] Development workflow documented (verify in MRBIN-3001)

### Cross-Platform Considerations

**Windows:**
- Binary path should handle `.exe` suffix automatically
- Path separators (backslash vs forward slash)
- Existing tests already cover Windows scenarios

**Linux/macOS:**
- No suffix on binary names
- Forward slash paths
- Permission checks (executable bit)

## Dependencies
- **MRBIN-1001**: Must be complete - code integration done
- **MRBIN-2001**: Must be complete - tests written
- **MRBIN-3001**: Must be complete - documentation updated

This ticket depends on ALL previous tickets being complete and verified.

## Risk Assessment
- **Risk**: Manual tests reveal edge cases not covered by unit tests
  - **Mitigation**: Fix issues immediately; add unit tests for discovered edge cases
- **Risk**: Platform-specific issues (Windows vs Unix)
  - **Mitigation**: Existing tests cover platform variations; verify no regressions
- **Risk**: Integration issues between components
  - **Mitigation**: Test all command entry points; verify config loading works consistently
- **Risk**: Performance regression
  - **Mitigation**: Config loading is fast; check that commands don't feel slower

## Files/Packages Affected
- All files in `packages/cli/` (verification only, no changes expected)
- Test files in `packages/cli/tests/` (verification only)
- Documentation files (verification only)

## Verification Notes

### Checklist for Manual Testing
For each scenario, verify:
1. Command executes without errors
2. Correct binary path is attempted
3. Error messages are clear if something fails
4. Fallback behavior works as expected
5. Logging output is informative

### Test Results Documentation
Document results for each scenario:
- Scenario number
- Configuration used
- Commands tested
- Expected behavior
- Actual behavior
- Pass/Fail status

### Final Acceptance
This ticket is complete when:
1. All automated tests pass
2. All 6 manual scenarios tested and pass
3. All project acceptance criteria verified
4. No regressions found
5. Documentation verified to match implementation
6. Ready for project completion

**Project Success Criteria:**
- ✅ `cleanMaproomRecords()` uses config-based binary resolution
- ✅ Binary resolution consistent across all CLI commands
- ✅ Test coverage comprehensive (29+ tests)
- ✅ Documentation accurate and complete
- ✅ No breaking changes to existing functionality

## Planning References
- Plan: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/plan.md` (Phase 4)
- Quality Strategy: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/quality-strategy.md` (Validation Approach section)
- Project Review: `.crewchief/projects/MRBIN_maproom-binary-configuration/planning/project-review.md` (Success Criteria)
