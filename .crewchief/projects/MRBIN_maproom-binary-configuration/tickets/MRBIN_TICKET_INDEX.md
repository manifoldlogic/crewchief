# MRBIN Ticket Index

Project: Maproom Binary Configuration (Completion)
Total Tickets: 4 (1 Phase 1, 1 Phase 2, 1 Phase 3, 1 Phase 4)

## Project Overview

This is a **completion project** finishing an incomplete feature. Most implementation already exists:
- Config schema field (`maproomBinaryPath`) - complete
- Binary resolution function (`findMaproomBinary()`) - complete
- Comprehensive test coverage (26 tests) - complete
- User documentation - complete

**Work remaining:**
1. Fix one call site (`cleanMaproomRecords`)
2. Add 2-3 tests for config parameter scenarios
3. Verify/enhance developer documentation
4. Integration validation

**Total Effort:** S (1 day, ~5 hours)

## Phase 1: Code Integration (1 ticket)

Foundation work to make `cleanMaproomRecords()` use config-based binary resolution.

### MRBIN-1001: Clean Maproom Records Config Integration
- **File**: `MRBIN-1001_cleanmaproom-config-integration.md`
- **Agent**: typescript-specialist
- **Scope**: 1 hour
- **Summary**: Update `cleanMaproomRecords()` to accept optional config parameter and load config internally
- **Deliverable**: Config-based binary resolution works in `cleanMaproomRecords()`, backwards compatible
- **Dependencies**: None (all infrastructure exists)
- **Key Changes**:
  - Function signature: `cleanMaproomRecords(config?: CrewChiefConfig)`
  - Load config if not provided (with error handling)
  - Pass `config.repository.maproomBinaryPath` to `findMaproomBinary()`
  - Intentionally omit `configFileLocation` (paths relative to CWD)

## Phase 2: Test Coverage (1 ticket)

Add test coverage for the new config parameter functionality.

### MRBIN-2001: Test Coverage for Config Parameter Usage
- **File**: `MRBIN-2001_test-coverage-config-parameter.md`
- **Agent**: unit-test-specialist
- **Scope**: 2 hours
- **Summary**: Add 2-3 test cases for config parameter handling in `cleanMaproomRecords()`
- **Deliverable**: Tests verify config provided, config loaded, and error handling
- **Dependencies**: MRBIN-1001
- **Key Tests**:
  - Config parameter provided → uses it
  - Config parameter not provided → loads internally
  - Config load fails → graceful fallback
- **Existing Coverage**: 26 tests already exist, adding 2-3 new ones

## Phase 3: Documentation (1 ticket)

Verify and enhance existing documentation with implementation details.

### MRBIN-3001: Documentation Verification and Enhancement
- **File**: `MRBIN-3001_documentation-update.md`
- **Agent**: documentation-specialist
- **Scope**: 0.5 hours
- **Summary**: Review/update `local-development.md` with relative path behavior clarification
- **Deliverable**: Accurate documentation of config-based binary resolution
- **Dependencies**: MRBIN-1001, MRBIN-2001
- **Key Updates**:
  - Verify existing "Method 1: Configuration File" section
  - Add relative path resolution clarification (CWD vs config file)
  - Ensure consistency with `README.md`
  - Document priority order clearly

## Phase 4: Verification (1 ticket)

Comprehensive integration testing and validation of all acceptance criteria.

### MRBIN-4001: Integration Verification and Manual Testing
- **File**: `MRBIN-4001_integration-verification.md`
- **Agent**: typescript-specialist
- **Scope**: 1.5 hours
- **Summary**: Full test suite, manual testing across 6 scenarios, acceptance criteria verification
- **Deliverable**: All tests pass, all scenarios validated, project complete
- **Dependencies**: MRBIN-1001, MRBIN-2001, MRBIN-3001
- **Key Validations**:
  - Full test suite passes (29+ tests)
  - 6 manual test scenarios (config, env var, fallback, relative path, invalid path, cleanMaproomRecords)
  - No regressions in existing functionality
  - Cross-platform compatibility verified

## Execution Order

### Sequential Path (No Parallelization)
All tickets must be executed in order:
1. **MRBIN-1001** (Code Integration) - Foundation changes
2. **MRBIN-2001** (Test Coverage) - Tests require code to exist
3. **MRBIN-3001** (Documentation) - Documents implemented behavior
4. **MRBIN-4001** (Verification) - Validates everything works together

### Dependencies Graph
```
MRBIN-1001 (Code)
    ↓
MRBIN-2001 (Tests)
    ↓
MRBIN-3001 (Docs) ← Can technically run after 1001, but cleaner after tests
    ↓
MRBIN-4001 (Verification) ← Requires ALL previous tickets complete
```

## Summary Statistics

**Total Estimated Time**: 5 hours (1 day)

**By Phase**:
- Phase 1 (Code): 1 hour
- Phase 2 (Tests): 2 hours
- Phase 3 (Docs): 0.5 hours
- Phase 4 (Verification): 1.5 hours

**Buffer**: 1 hour for unexpected issues (20% of total)
**Total with buffer**: 6 hours (0.75 days)

**By Activity**:
- Implementation: 1 hour (20%)
- Testing: 3.5 hours (70%)
- Documentation: 0.5 hours (10%)

**Files Modified**:
- Source files: 1 (`worktrees.ts` - cleanMaproomRecords function)
- Test files: 1 (`clean-maproom-records.test.ts` - add 2-3 tests)
- Documentation: 1-2 (`local-development.md`, possibly `README.md`)

**Code Changes**:
- Lines added: ~15 (function signature + config loading)
- Lines changed: ~5 (findMaproomBinary call)
- Test lines added: ~50-75 (2-3 new test cases)
- Net change: +70-95 lines total

**Test Coverage**:
- Before: 26 tests in clean-maproom-records.test.ts
- After: 28-29 tests in clean-maproom-records.test.ts
- Coverage: Maintained at 90%+

## Project Acceptance Criteria Tracking

From `plan.md` - status tracking for verification:

- [x] Config accepts `maproomBinaryPath` setting (already done)
- [ ] Config path takes precedence over packaged binary (verify in MRBIN-4001)
- [x] Env var takes highest precedence (already implemented)
- [x] Global install checked before packaged binary (already correct)
- [ ] Binary resolution consistent across all commands (fix in MRBIN-1001)
- [ ] Development workflow documented (update in MRBIN-3001)

**Success Metrics:**
- All unit tests pass (including new ones)
- Manual test scenarios all pass
- No TypeScript compilation errors
- No linting errors
- Documentation accurate and complete

## Risk Mitigation Summary

| Risk | Probability | Impact | Mitigation Strategy | Ticket |
|------|-------------|--------|---------------------|--------|
| Config load breaks existing usage | Low | Medium | Optional parameter, backwards compatible | MRBIN-1001 |
| Tests don't cover edge cases | Low | Medium | 3 specific test scenarios | MRBIN-2001 |
| Documentation unclear | Low | Low | Verify with examples, cross-reference | MRBIN-3001 |
| Regression in binary resolution | Low | High | Full test suite + 6 manual scenarios | MRBIN-4001 |

**Overall Risk Level:** LOW
- Minimal code changes (1 function signature + body)
- Comprehensive test coverage (existing + new)
- Backwards compatible design
- Well-understood problem space

## Scope Boundaries

**In Scope:**
- Fix `cleanMaproomRecords()` config integration
- Add test coverage for config parameter
- Update developer documentation
- Full integration validation

**Explicitly Out of Scope:**
- MCP package changes (different use case, intentional)
- Shared utility extraction (implementations intentionally different)
- Resolution order changes (already correct)
- New config fields (using existing `maproomBinaryPath`)
- Config file location tracking in loadConfig (using optional parameter)
- Relative path resolution from config file (CWD resolution acceptable for MVP)
- Additional binary options (debug/profile variants)
- Version constraints (no binary version checking)

## Post-Completion

After all tickets verified:
1. Project can be archived
2. No additional knowledge synthesis needed (updates to existing docs only)
3. Feature is complete and production-ready

## Future Enhancements

Not in scope for this project, but possible later:
1. Config validation errors in `crewchief config validate` command
2. `--binary-path` CLI flag for one-off overrides
3. Per-command binary overrides
4. Binary version detection and warnings
5. Automatic binary download if not found
