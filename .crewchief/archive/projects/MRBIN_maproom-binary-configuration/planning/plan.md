# Plan: Maproom Binary Configuration

## Overview

This project will be executed in 3 phases over 1.5 days. The phases are designed to deliver incremental value, with each phase being independently testable and valuable.

**Total Effort:** S (1.5 days)
**Phases:** 3
**Tickets:** 7

## Phases

### Phase 1: Configuration Foundation

**Objective:** Add configuration schema and shared utility infrastructure without changing existing behavior.

**Deliverables:**
- Config schema extension (`maproomBinaryPath` added to RepositorySchema)
- Shared binary resolution utility (packages/cli/src/utils/maproom-binary.ts)
- Unit tests for binary resolution precedence order
- No changes to existing consumers yet (worktrees.ts, maproom.ts)

**Agent Assignments:**
- typescript-engineer: Implement config schema changes and utility function
- unit-test-runner: Execute unit tests for maproom-binary.ts
- verify-ticket: Validate schema validation and utility function behavior

**Why This Phase:**
- Establishes foundation without risking existing functionality
- Allows testing of resolution logic in isolation
- Schema change is backwards compatible (optional field)

**Acceptance Criteria:**
- RepositorySchema includes maproomBinaryPath field (optional string)
- findMaproomBinary() utility implements correct precedence order
- Unit tests verify all resolution paths (env, config, global, packaged)
- Existing tests still pass (no consumers changed yet)
- Maproom action handlers converted to async (prerequisite for Phase 2)

### Phase 2: CLI Integration

**Objective:** Integrate shared utility into CLI commands and remove duplicated code.

**Deliverables:**
- Update packages/cli/src/cli/maproom.ts to use findMaproomBinary()
- Update packages/cli/src/git/worktrees.ts to use findMaproomBinary()
- Remove old resolvePackagedMaproomBin() function
- Remove inline binary resolution from WorktreeService
- Updated error messages with clear configuration guidance

**Agent Assignments:**
- typescript-engineer: Refactor maproom.ts and worktrees.ts
- unit-test-runner: Execute integration tests for maproom commands
- verify-ticket: Validate binary resolution works across all CLI commands

**Why This Phase:**
- Consolidates duplicated code (~86 lines removed)
- Makes resolution consistent across CLI
- Behavior change (global > packaged) happens here

**Acceptance Criteria:**
- maproom.ts uses findMaproomBinary() utility
- worktrees.ts uses findMaproomBinary() utility
- All existing maproom integration tests pass
- Error messages guide users to configuration options
- Error messages show all resolution attempts with paths
- Resolution order is: env > config > global > packaged
- Commands work without config file (backwards compatible)

**Dependencies:**
- Requires Phase 1 completion (utility must exist)

### Phase 3: Documentation and Validation

**Objective:** Complete documentation and validate the full implementation with real-world scenarios.

**Deliverables:**
- Updated README.md with configuration example
- Updated docs/development/local-development.md with config-based workflow
- Updated packages/cli/README.md (if exists)
- Example configuration in crewchief.config.js
- Integration test validating config-based resolution
- Validation that all acceptance criteria are met

**Agent Assignments:**
- documentation-engineer: Update all documentation files
- integration-tester: Test with real config files and scenarios
- verify-ticket: Final validation of all acceptance criteria

**Why This Phase:**
- Ensures users can discover and use the new feature
- Validates real-world usage patterns
- Catches any documentation gaps

**Acceptance Criteria:**
- README.md documents maproomBinaryPath config option
- Development docs show how to use config for local builds
- Example config file includes maproomBinaryPath example
- Manual testing confirms config file works as expected
- Windows testing confirmed (on CI or manually)
- Release notes include priority order change and migration guide
- All acceptance criteria from initiative are verified

**Dependencies:**
- Requires Phase 2 completion (implementation must be done)

## Detailed Ticket Breakdown

### Phase 1 Tickets (MRBIN-1xxx)

**MRBIN-1001: Add maproomBinaryPath to config schema**
- File: packages/cli/src/config/schema.ts
- Add optional field to RepositorySchema
- Export updated types

**MRBIN-1002: Implement shared binary resolution utility**
- File: packages/cli/src/utils/maproom-binary.ts (NEW)
- Implement findMaproomBinary() with precedence logic
- Handle platform differences (Windows .exe)
- Add warning for invalid config paths
- Handle missing config gracefully

**MRBIN-1003: Unit tests for binary resolution**
- File: packages/cli/tests/utils/maproom-binary.test.ts (NEW)
- Test precedence order
- Test platform handling
- Test missing binary scenarios
- Test missing config file scenario
- Mock fs.existsSync and spawnSync

**MRBIN-1004: Convert maproom action handlers to async**
- File: packages/cli/src/cli/maproom.ts
- Change all action handlers to async functions
- Update calls to use await
- Verify Commander supports async actions
- Pattern: `.action(async (args) => await runMaproomForward([...]))`

### Phase 2 Tickets (MRBIN-2xxx)

**MRBIN-2001: Refactor maproom.ts to use shared utility**
- File: packages/cli/src/cli/maproom.ts
- Import findMaproomBinary and loadConfig
- Make runMaproomForward() async
- Add try-catch for config loading (handle missing config)
- Replace resolvePackagedMaproomBin() with utility call
- Update error messages to show resolution attempts
- Remove old function
- Verify action handlers are async (from MRBIN-1004)

**MRBIN-2002: Refactor worktrees.ts to use shared utility**
- File: packages/cli/src/git/worktrees.ts
- Import findMaproomBinary
- Replace inline resolution in runMaproomScan()
- Load config and pass maproomBinaryPath
- Remove ~40 lines of duplicated logic

### Phase 3 Tickets (MRBIN-3xxx)

**MRBIN-3001: Update documentation**
- Files: README.md, docs/development/local-development.md, packages/cli/README.md
- Document maproomBinaryPath config option
- Show example configuration
- Update development workflow section
- Add to configuration reference

## Dependencies

### Internal Dependencies

```
MRBIN-1001 (Schema) ──┐
                      ├──> MRBIN-1002 (Utility) ──> MRBIN-1003 (Tests)
                      │                                  │
MRBIN-1004 (Async) ───┤                                  │
                      │                                  ▼
                      └──────────────────────────> MRBIN-2001 (maproom.ts)
                                                         │
                                                         ▼
                                                    MRBIN-2002 (worktrees.ts)
                                                         │
                                                         ▼
                                                    MRBIN-3001 (Docs)
```

**Critical Path:**
1. Schema change (MRBIN-1001)
2. Async conversion (MRBIN-1004) - NEW, runs parallel with utility work
3. Utility implementation (MRBIN-1002)
4. Unit tests (MRBIN-1003)
5. CLI integration (MRBIN-2001, MRBIN-2002) - requires MRBIN-1004 completion
6. Documentation (MRBIN-3001)

### External Dependencies

**None** - This project is self-contained within the CLI package.

**Parallel Development:**
- Can be developed in parallel with WTPATH project
- No conflicts expected (different files)

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking existing workflows | Medium | High | Comprehensive testing, backwards compatible schema, maintain env var priority |
| Async config loading issues | Medium | Medium | Use existing async patterns from worktrees.ts, make functions async |
| Platform-specific path issues | Low | Medium | Reuse existing platform detection, test on Windows/macOS/Linux |
| Incorrect precedence order | Low | High | Explicit unit tests for each priority level, integration tests |
| Missing edge cases | Medium | Low | Comprehensive test matrix, warn on invalid paths |
| Documentation incomplete | Low | Medium | Document in multiple places, include examples |

### Specific Mitigations

**Breaking Workflows:**
- All existing resolution paths continue to work
- Environment variable still has highest priority
- Optional config field means no config changes required
- Existing tests must pass (regression check)

**Async Challenges:**
- Config loading is already async in worktrees.ts (precedent exists)
- Make runMaproomForward async (Commander supports async)
- Use same pattern as existing async CLI commands

**Platform Issues:**
- Copy existing platform detection logic exactly
- Windows .exe handling already tested in current code
- Packaged path logic reuses existing paths

## Success Metrics

### Code Quality Metrics

- [ ] ~100 lines of duplicated code removed
- [ ] Zero new linting errors
- [ ] Test coverage maintained or improved
- [ ] All existing tests pass
- [ ] 6 new unit tests added (precedence scenarios)

### Functional Metrics

- [ ] Config schema accepts maproomBinaryPath
- [ ] Environment variable still takes precedence
- [ ] Config path overrides global and packaged
- [ ] Global install checked before packaged
- [ ] Invalid config path shows warning
- [ ] Binary resolution consistent across all commands

### User Experience Metrics

- [ ] Clear error message when binary not found
- [ ] Documentation includes config example
- [ ] Development workflow documented
- [ ] Config validates with helpful Zod errors
- [ ] Works on Windows, macOS, Linux

### Integration Metrics

- [ ] maproom scan command works with config
- [ ] worktree create auto-indexing works with config
- [ ] Environment variable override still works
- [ ] Global install works without config
- [ ] Packaged binary fallback still works

## Rollout Strategy

**Phase 1 Completion:**
- Merge schema and utility changes
- Merge async conversion
- No user-facing changes yet
- Safe to deploy

**Phase 2 Completion:**
- Behavior change (global > packaged priority)
- Update release notes with new priority order
- Mark as minor version bump (additive feature)

**Phase 3 Completion:**
- Full feature available
- Documentation published
- Announce in changelog

## Migration Guide

**Release Notes Template:**

```markdown
## Binary Resolution Improvements

### New Feature: Config-Based Binary Path

You can now specify the maproom binary path in your config file:

```javascript
// crewchief.config.js
export default {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}
```

### Priority Order Change (Behavior Change)

The binary resolution priority order has changed to prefer global installations:

**Before:**
1. CREWCHIEF_MAPROOM_BIN env var
2. Packaged binary
3. Global install

**After:**
1. CREWCHIEF_MAPROOM_BIN env var
2. config.repository.maproomBinaryPath
3. Global install
4. Packaged binary

**Who is affected:**
- Users with both global install AND packaged binary will now use global install
- This is an intentional improvement to avoid stale packaged binaries

**Migration checklist:**
- [ ] If you have both installations, verify which binary is active
- [ ] To check: run `crewchief maproom --version` or check error messages
- [ ] To override: set `CREWCHIEF_MAPROOM_BIN` environment variable
- [ ] To configure: add `maproomBinaryPath` to crewchief.config.js

**Examples:**

```bash
# Force specific binary via env var
CREWCHIEF_MAPROOM_BIN=/custom/path crewchief maproom scan

# Use config file (persistent)
echo 'export default { repository: { maproomBinaryPath: "./bin/crewchief-maproom" } }' > crewchief.config.local.js

# Verify which binary is being used (error message shows resolution path)
crewchief maproom --help
```
```
```

## Rollback Plan

**If issues arise:**

1. **Phase 1 issues:** Revert schema change (breaking change for new configs only)
2. **Phase 2 issues:** Revert CLI integration, keep utility and schema
3. **Phase 3 issues:** Fix documentation without code changes

**Rollback Impact:**
- Users who configured maproomBinaryPath will need to use env var temporarily
- Existing users (no config) are unaffected
- Environment variable override always works

## Testing Strategy

### Unit Testing
- Binary resolution precedence (6 test cases)
- Platform detection (Windows vs Unix)
- Path validation (relative, absolute, missing)
- Config parsing (Zod validation)

### Integration Testing
- maproom scan command with config
- worktree create with auto-indexing
- Environment variable override
- Missing binary error message

### Manual Testing
- Create crewchief.config.local.js with maproomBinaryPath
- Run crewchief maproom scan
- Verify correct binary is used
- Test with invalid path (warning appears)
- Test with env var set (env var wins)

## Timeline

**Day 1:**
- Morning: Phase 1 (schema + async conversion + utility + tests) - 4-6 hours
- Afternoon: Phase 2 start (CLI integration) - 2-3 hours
- End of day: Phase 2 complete

**Day 2:**
- Morning: Phase 3 (documentation + validation + Windows testing) - 3-4 hours
- Buffer: Integration issues, manual testing - 1-2 hours

**Checkpoints:**
- 12:00 PM Day 1: Phase 1 complete, utility tested, async converted
- 5:00 PM Day 1: Phase 2 complete, CLI integrated
- 12:00 PM Day 2: Phase 3 complete, documentation done, Windows tested

## Communication

**Updates:**
- Commit messages reference ticket IDs (MRBIN-xxxx)
- PR description links to this plan
- Release notes mention new config option

**Breaking Changes:**
- Document priority order change in changelog
- Not technically breaking (backwards compatible)
- Behavior change worth calling out
- Include migration guide in release notes
- Provide examples of before/after behavior
