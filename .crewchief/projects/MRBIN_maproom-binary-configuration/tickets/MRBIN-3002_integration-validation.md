# Ticket: [MRBIN-3002]: Integration Validation

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
- typescript-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Perform comprehensive integration validation with real config files and manual testing to verify the complete implementation works correctly across all scenarios, platforms, and edge cases identified in the quality strategy.

## Background
This final ticket validates the entire implementation with real-world scenarios and manual testing. While unit tests validate individual components and integration tests validate automated scenarios, this ticket ensures the feature works end-to-end in realistic usage patterns.

The quality strategy defines 4 critical manual test scenarios that must be validated before the project is complete.

## Acceptance Criteria
- [ ] Test 1: Config file with local build works correctly
- [ ] Test 2: Environment variable override works correctly
- [ ] Test 3: Invalid config path shows warning and falls back
- [ ] Test 4: No config uses global/packaged correctly
- [ ] All maproom commands tested (scan, search, show, chat)
- [ ] Worktree auto-indexing tested with config
- [ ] Error messages are helpful and accurate
- [ ] Resolution order verified in all scenarios
- [ ] Windows testing completed (on CI or manually)
- [ ] All acceptance criteria from initiative verified

## Technical Requirements
- Test with actual config files (not mocks)
- Test on multiple platforms (Linux, macOS, Windows if possible)
- Verify all binary resolution paths (env, config, global, packaged)
- Verify error messages show resolution attempts
- Verify warnings appear for invalid paths
- Test all maproom subcommands
- Test worktree creation with auto-indexing

## Implementation Notes

### Manual Test Procedure

**Test 1: Config file with local build**
```bash
# Create config with local build path
cat > crewchief.config.local.js << 'EOF'
export default {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}
EOF

# Build if needed
cd crates/maproom && cargo build --release && cd ../..

# Test maproom commands
crewchief maproom scan
crewchief maproom search "test"

# Verify: Should use ./target/release/crewchief-maproom
# Check: Binary version matches local build
```

**Test 2: Environment variable override**
```bash
# With config file present, env var should override
CREWCHIEF_MAPROOM_BIN=/usr/local/bin/crewchief-maproom crewchief maproom scan

# Verify: Should use /usr/local/bin/crewchief-maproom
# Verify: Ignores config file path
```

**Test 3: Invalid config path**
```bash
# Create config with invalid path
cat > crewchief.config.local.js << 'EOF'
export default {
  repository: {
    maproomBinaryPath: '/nonexistent/binary/path'
  }
}
EOF

crewchief maproom scan

# Verify: Warning message about invalid path appears
# Verify: Falls back to global/packaged binary
# Verify: Command still works (doesn't fail)
```

**Test 4: No config file**
```bash
# Remove config files
rm -f crewchief.config.local.js crewchief.config.js

crewchief maproom scan

# Verify: Uses global install or packaged binary
# Verify: No errors about missing config
# Verify: Commands work normally
```

**Test 5: Worktree auto-indexing**
```bash
# With config file
crewchief worktree create test-validation-branch

# Verify: Scan runs automatically
# Verify: Uses configured binary path
# Verify: Success message shown
```

**Test 6: Error message quality**
```bash
# Move/rename all binaries to trigger not-found
crewchief maproom scan

# Verify: Error message is helpful
# Verify: Shows all resolution attempts
# Verify: Provides configuration guidance
# Verify: Exit code is non-zero
```

### Platform-Specific Testing

**Windows (if available via CI or manual):**
- Verify .exe suffix handling
- Verify platform-specific packaged paths
- Verify path resolution with backslashes
- Test all scenarios above on Windows

**macOS:**
- Test on darwin-arm64 (M1/M2) if available
- Test on darwin-x64 if available
- Verify packaged paths for both architectures

**Linux:**
- Test on linux-x64 (primary CI platform)
- Verify packaged paths

## Dependencies
- MRBIN-3001 (Documentation must be complete)

## Risk Assessment
- **Risk**: Manual testing incomplete due to platform availability
  - **Mitigation**: Use CI for Windows testing, document what was tested
- **Risk**: Edge cases missed during manual testing
  - **Mitigation**: Follow test matrix exactly, document all test results
- **Risk**: Real-world config parsing differs from unit tests
  - **Mitigation**: Use actual config files, not programmatic config objects

## Files/Packages Affected
- No code changes (validation only)
- Test results should be documented in ticket verification notes

## Verification Notes
Document test results for each scenario:

1. Test 1 (Config with local build):
   - [ ] Tested
   - Result: [Pass/Fail with details]

2. Test 2 (Env var override):
   - [ ] Tested
   - Result: [Pass/Fail with details]

3. Test 3 (Invalid config):
   - [ ] Tested
   - Result: [Pass/Fail with details]

4. Test 4 (No config):
   - [ ] Tested
   - Result: [Pass/Fail with details]

5. Test 5 (Worktree indexing):
   - [ ] Tested
   - Result: [Pass/Fail with details]

6. Test 6 (Error messages):
   - [ ] Tested
   - Result: [Pass/Fail with details]

7. Windows testing:
   - [ ] Tested (or N/A with explanation)
   - Result: [Pass/Fail with details]

Verify initiative acceptance criteria:
- [ ] Config-based binary path works
- [ ] Resolution order is correct
- [ ] Backwards compatible
- [ ] Documentation complete
- [ ] Error messages helpful
- [ ] Works on all platforms
