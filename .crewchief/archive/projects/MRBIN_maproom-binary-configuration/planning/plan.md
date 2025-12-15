# Plan: Maproom Binary Configuration

## Overview

This is a **completion project** finishing an incomplete feature. Most of the implementation already exists:
- ✅ Config schema field
- ✅ Binary resolution function
- ✅ Comprehensive test coverage
- ✅ User documentation (README.md)

**Work remaining:**
1. Fix one call site (`cleanMaproomRecords`)
2. Verify existing developer documentation (already exists, may need minor enhancements)
3. Add tests for new config parameter scenario (26 tests already exist, need 2-3 new ones)

**Effort:** S (1 day, ~5 hours)

## Phases

### Phase 1: Code Integration

**Objective:** Make `cleanMaproomRecords()` use config-based binary resolution

**Deliverables:**
- Update `cleanMaproomRecords()` function signature to accept optional config
- Load config within function if not provided
- Pass config path to `findMaproomBinary()`
- Handle config load errors gracefully (fall back to env var/packaged)

**Specific changes:**
```typescript
// packages/cli/src/git/worktrees.ts:240

// Before:
export async function cleanMaproomRecords(): Promise<void> {
  const result = findMaproomBinary()
  // ...
}

// After:
export async function cleanMaproomRecords(config?: CrewChiefConfig): Promise<void> {
  let resolvedConfig = config
  if (!resolvedConfig) {
    try {
      resolvedConfig = await loadConfig()
    } catch {
      // Config not found or invalid - continue without it
    }
  }

  const result = findMaproomBinary({
    configPath: resolvedConfig?.repository.maproomBinaryPath
    // Note: configFileLocation not provided - relative paths must be
    // absolute or relative to CWD, not relative to config file
  })
  // ...
}
```

**Agent Assignments:**
- **typescript-specialist**: Update cleanMaproomRecords function
- **typescript-specialist**: Import loadConfig if not already imported
- **typescript-specialist**: Import CrewChiefConfig type

**Success Criteria:**
- [ ] Function signature accepts optional config parameter
- [ ] Config is loaded if not provided
- [ ] Config path is passed to findMaproomBinary
- [ ] configFileLocation intentionally omitted (relative paths relative to CWD, not config file)
- [ ] Error handling prevents crashes if config invalid
- [ ] Existing code continues to compile

**Time estimate:** 1 hour

**Note:** Three existing call sites (worktree.ts lines 216, 328, 390) don't need changes - they can rely on cleanMaproomRecords loading config internally.

### Phase 2: Test Coverage

**Objective:** Verify test coverage for cleanMaproomRecords config usage

**Deliverables:**
- Review existing 26 test cases in clean-maproom-records.test.ts
- Add 2-3 new tests specifically for config parameter passing:
  - Config parameter provided → uses it
  - Config parameter not provided → loads it
  - Config load fails → gracefully falls back

**Test location:** `packages/cli/tests/unit/clean-maproom-records.test.ts`

**Current coverage:** 26 existing tests cover binary resolution, error handling, and edge cases. New tests will specifically cover config parameter integration.

**Agent Assignments:**
- **unit-test-specialist**: Review existing tests
- **unit-test-specialist**: Add new test cases if needed
- **unit-test-specialist**: Verify mocking works correctly

**Success Criteria:**
- [ ] Tests cover config parameter usage
- [ ] Tests cover config load fallback
- [ ] Tests cover error handling
- [ ] All tests pass

**Time estimate:** 2 hours

### Phase 3: Documentation

**Objective:** Verify and enhance existing config-based binary resolution documentation

**Deliverables:**
- Review existing "Method 1: Configuration File" section in `docs/development/local-development.md` (lines 76-100)
- Verify accuracy and completeness
- Add clarification note if needed about relative path limitation (relative to CWD, not config file)
- Ensure consistency with README.md

**Current state:** Documentation already exists and is well-written with:
- Configuration example showing relative path to binary
- Benefits explanation
- Preference over environment variables clearly stated

**Potential enhancements (if needed):**
- Add note about relative path resolution (relative to CWD when used from cleanMaproomRecords)
- Verify examples are accurate
- Check consistency with README.md configuration section

**Agent Assignments:**
- **documentation-specialist**: Review existing documentation
- **documentation-specialist**: Add relative path clarification if helpful
- **documentation-specialist**: Verify consistency across docs

**Success Criteria:**
- [ ] Existing documentation verified as accurate
- [ ] Relative path limitation documented (if clarification needed)
- [ ] Consistent with README.md documentation
- [ ] No contradictions or outdated information

**Time estimate:** 0.5 hours

### Phase 4: Verification

**Objective:** Confirm all acceptance criteria met and no regressions

**Deliverables:**
- Run full test suite
- Manual testing with actual config file
- Verify all commands work (maproom, worktree:clean, etc.)
- Check documentation renders correctly

**Agent Assignments:**
- **unit-test-runner**: Execute `pnpm test`
- **verify-ticket**: Check acceptance criteria from project summary
- **manual-tester**: Test with real config file (if needed)

**Test scenarios:**
1. Set maproomBinaryPath in config → all commands use it
2. Set CREWCHIEF_MAPROOM_BIN env var → overrides config
3. No config, no env var → falls back to global/packaged
4. Config with relative path → resolves correctly
5. Config with invalid path → warns but continues

**Success Criteria:**
- [ ] All unit tests pass (existing + new)
- [ ] No TypeScript compilation errors
- [ ] No linting errors
- [ ] Manual config test succeeds
- [ ] Documentation builds without errors

**Time estimate:** 1.5 hours

## Dependencies

### Phase Dependencies

- Phase 1 → Phase 2: Code must exist before tests can be written
- Phase 2 → Phase 4: Tests must pass before verification
- Phase 3: Independent, can run in parallel with Phase 1-2

### External Dependencies

**None.** All required infrastructure exists:
- ✅ Config schema (already defined)
- ✅ Resolution function (already implemented)
- ✅ Test infrastructure (Vitest setup)
- ✅ Documentation structure (local-development.md exists)

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Config load breaks existing usage | Low | Medium | Make config parameter optional; existing calls work unchanged |
| Tests don't cover edge cases | Low | Medium | Review test coverage in Phase 2; add comprehensive scenarios |
| Documentation unclear | Low | Low | Include examples and comparison table |
| Regression in binary resolution | Low | High | Run full test suite (20+ existing tests catch issues) |
| Scope creep (MCP package) | Medium | Medium | Explicitly decided in architecture: MCP unchanged |

**Overall risk level:** LOW
- Minimal code changes
- Comprehensive existing tests
- Backwards compatible changes

## Success Metrics

**From project acceptance criteria:**

- [x] Config accepts `maproomBinaryPath` setting (already done)
- [ ] Config path takes precedence over packaged binary (verify in tests)
- [x] Env var still takes highest precedence (already implemented)
- [x] Global install checked before local packaged binary (already correct)
- [ ] Binary resolution is consistent across all commands (fix cleanMaproomRecords)
- [ ] Development workflow documented (add to local-development.md)

**Additional metrics:**
- [ ] All 3 CLI call sites pass config path
- [ ] Test coverage includes cleanMaproomRecords scenarios
- [ ] No breaking changes to existing code
- [ ] Documentation includes examples and priority order

## Timeline

**Total effort:** 5 hours (S - 1 day)

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 1: Code Integration | 1 hour | 1 hour |
| Phase 2: Test Coverage | 2 hours | 3 hours |
| Phase 3: Documentation | 0.5 hours | 3.5 hours |
| Phase 4: Verification | 1.5 hours | 5 hours |

**Buffer:** 1 hour for unexpected issues (20% of total)

## Out of Scope

The following are **explicitly excluded** to maintain focus:

1. **MCP package changes**: MCP serves different use case (IDE integration), has different resolution order intentionally
2. **Shared utility extraction**: CLI and MCP implementations are intentionally different
3. **Resolution order changes**: Current order is already correct
4. **New config fields**: Only using existing `maproomBinaryPath`
5. **Config file location tracking**: Using optional parameter, not changing loadConfig return type
6. **Relative path resolution from config file**: configFileLocation not passed to cleanMaproomRecords (paths relative to CWD, acceptable for MVP)
7. **Additional binary options**: No debug/profile binary variants
8. **Version constraints**: No binary version checking

## Post-Completion Actions

After all phases complete:

1. **Ticket creation** via `/workstream:project-tickets MRBIN`
2. **Knowledge synthesis**: No new docs needed (updates to existing)
3. **Archive decision**: Keep in active projects until tickets complete

## Rollback Plan

If issues discovered after completion:

**Scenario:** Config loading breaks something
**Action:**
1. Revert cleanMaproomRecords changes
2. Binary resolution still works via env var/packaged
3. No user-facing breakage (config is optional)

**Scenario:** Tests reveal edge cases
**Action:**
1. Add error handling for specific case
2. Document limitation if workaround not feasible
3. Priority: maintain backwards compatibility

## Future Enhancements

**Not in scope for this project, but possible later:**

1. Config validation errors shown in `crewchief config validate` command
2. `--binary-path` CLI flag for one-off overrides
3. Per-command binary overrides (scan vs search different binaries)
4. Binary version detection and warnings
5. Automatic binary download if not found

These can be separate projects if needed.
