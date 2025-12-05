# Quality Strategy: Maproom Binary Configuration

## Testing Philosophy

**Goal:** Build confidence that binary resolution works correctly across all scenarios, not achieve 100% coverage.

**Principles:**
- Test the precedence order explicitly (highest risk area)
- Test platform differences (Windows vs Unix)
- Test failure paths (missing binary, invalid config)
- Reuse existing test patterns from the codebase
- Focus on integration points (config → resolution → execution)

**What we DON'T need to test:**
- Existing config loading logic (already tested)
- Basic fs.existsSync behavior (Node built-in)
- Bash command execution (platform-level)

## Test Types

### Unit Tests

**Scope:**
- Binary resolution precedence order (6 scenarios)
- Platform detection (Windows .exe vs Unix)
- Path validation (absolute, relative, missing)
- Config path resolution
- Warning emission on invalid paths

**Tools:**
- Vitest (existing test framework)
- vi.mock for fs and child_process
- vi.spyOn for console.warn

**Location:** `packages/cli/tests/utils/maproom-binary.test.ts`

**Coverage Target:** 90%+ for findMaproomBinary() function
- Focus on business logic, not mocked dependencies
- All precedence branches must be tested
- Platform-specific branches must be tested

**Test Cases:**

```typescript
describe('findMaproomBinary', () => {
  // Precedence tests
  it('prioritizes CREWCHIEF_MAPROOM_BIN env var over all others')
  it('uses config path when env var not set')
  it('uses global install when env var and config not set')
  it('falls back to packaged binary when nothing else available')
  it('returns not-found when no binary exists')

  // Platform tests
  it('uses .exe suffix on Windows')
  it('uses no suffix on Unix')

  // Path validation tests
  it('resolves relative config paths')
  it('handles absolute config paths')
  it('warns when config path does not exist')
  it('falls through to next priority when config path invalid')

  // Edge cases
  it('handles missing process.env.CREWCHIEF_MAPROOM_BIN gracefully')
  it('handles undefined options parameter')
  it('handles empty string config path')
  it('handles missing config file gracefully (no error)')
})
```

### Integration Tests

**Scope:**
- maproom command with config file
- worktree creation with auto-indexing
- Environment variable override behavior
- Error messages when binary not found

**Approach:**
- Extend existing integration tests in `packages/cli/tests/integration/maproom-commands.int.test.ts`
- Create temporary config files for testing
- Mock binary execution, focus on resolution
- Verify correct binary path is used

**Test Cases:**

```typescript
describe('maproom binary resolution integration', () => {
  it('uses binary from config file')
  it('environment variable overrides config file')
  it('shows helpful error when binary not found')
  it('worktree creation uses configured binary')
})
```

**NOT tested in integration:**
- Actual binary execution (too slow, environment-dependent)
- Database operations (not relevant to this project)
- Full end-to-end workflow (manual testing)
- Windows-specific integration (tested on CI or manually)

### Manual Testing

**Scope:** Real-world scenarios with actual config files

**Critical Paths:**
1. Developer workflow with local build
2. Production workflow with global install
3. Environment variable override
4. Invalid config path (warning appears)

**Test Procedure:**

```bash
# Test 1: Config file with local build
echo 'export default { repository: { maproomBinaryPath: "./target/release/crewchief-maproom" } }' > crewchief.config.local.js
crewchief maproom scan
# Expect: Uses local build

# Test 2: Environment variable override
CREWCHIEF_MAPROOM_BIN=/custom/path crewchief maproom scan
# Expect: Uses /custom/path

# Test 3: Invalid config path
echo 'export default { repository: { maproomBinaryPath: "/nonexistent/binary" } }' > crewchief.config.local.js
crewchief maproom scan
# Expect: Warning + fallback to global/packaged

# Test 4: No config, global install
rm crewchief.config.local.js
crewchief maproom scan
# Expect: Uses global install or packaged binary
```

## Critical Paths

The following paths MUST be tested (automated):

1. **Precedence Order**
   - Env var > Config > Global > Packaged
   - Each level overrides lower priority
   - Fallthrough when higher priority not available

2. **Platform Compatibility**
   - Windows: Uses crewchief-maproom.exe
   - Unix: Uses crewchief-maproom
   - Platform detection correct

3. **Path Resolution**
   - Relative paths resolved from config location
   - Absolute paths used as-is
   - Invalid paths trigger warning and fallthrough

4. **Error Handling**
   - Missing binary shows clear error message
   - Error message includes configuration guidance
   - Process exits with non-zero code

5. **Backwards Compatibility**
   - Existing tests pass without modification
   - Users without config see no change
   - Environment variable still works
   - Commands work without config file present

## Test Data Strategy

### Mock Data

**Binary paths:**
- Valid paths: `/usr/local/bin/crewchief-maproom`, `./bin/darwin-arm64/crewchief-maproom`
- Invalid paths: `/nonexistent/binary`, `./missing/file`
- Relative paths: `../target/release/crewchief-maproom`, `./bin/crewchief-maproom`

**Config objects:**
```typescript
const validConfig = {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
    maproomBinaryPath: '/valid/path/crewchief-maproom'
  }
}

const configWithRelativePath = {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}

const configWithInvalidPath = {
  repository: {
    maproomBinaryPath: '/nonexistent/binary'
  }
}
```

**Environment variables:**
```typescript
// Set in beforeEach
process.env.CREWCHIEF_MAPROOM_BIN = '/env/override/binary'

// Restore in afterEach
delete process.env.CREWCHIEF_MAPROOM_BIN
```

### Fixtures

**No fixtures needed** - All test data is inline (config objects, paths)

**File system mocking:**
```typescript
vi.mock('node:fs', () => ({
  existsSync: vi.fn((path: string) => {
    // Return true for known valid paths
    return path.includes('/valid/') || path.includes('packaged')
  })
}))
```

## Quality Gates

### Before Merge (Automated)

- [ ] All unit tests pass (vitest)
- [ ] All integration tests pass
- [ ] No TypeScript errors (tsc --noEmit)
- [ ] No linting errors (pnpm lint)
- [ ] Test coverage >= 90% for new code
- [ ] All existing tests still pass (regression check)

### Before Verification (Per Ticket)

- [ ] Unit tests written and passing
- [ ] Integration tests updated (if applicable)
- [ ] No console errors during test runs
- [ ] TypeScript types are correct
- [ ] Code formatted (pnpm format)

### Before Release (Manual)

- [ ] Manual testing completed (all 4 scenarios)
- [ ] Documentation accurate and complete
- [ ] Example config file works
- [ ] Error messages are helpful
- [ ] Works on target platforms (macOS, Linux, Windows)

## Regression Prevention

**Existing test suites that must pass:**

1. `packages/cli/tests/integration/maproom-commands.int.test.ts`
   - Validates existing maproom commands still work
   - Verifies environment variable override

2. `packages/cli/tests/git/worktrees.test.ts` (if exists)
   - Validates worktree creation
   - Verifies auto-indexing

3. `packages/cli/tests/config/loader.test.ts` (if exists)
   - Validates config loading
   - Verifies schema validation

**Strategy:**
- Run full test suite before and after changes
- Compare results (should be identical except new tests)
- Any failing tests are blockers

## Performance Testing

**Not required** - Binary resolution is fast (<100ms overhead)

**If performance concerns arise:**
- Benchmark resolution time (should be <50ms)
- Profile fs.existsSync calls (should be <10)
- No caching needed (resolution is infrequent)

## Test Maintenance

**When to update tests:**
- Adding new resolution priority level
- Changing precedence order
- Adding new platform support
- Changing error messages

**Test review checklist:**
- [ ] Test names describe behavior, not implementation
- [ ] Mocks are minimal and focused
- [ ] Tests are independent (no shared state)
- [ ] Tests are deterministic (no race conditions)
- [ ] Tests are fast (<1s per test)

## Known Test Limitations

**Platform-specific testing:**
- Windows tests run on CI (GitHub Actions supports Windows)
- Verify Windows .exe handling in CI matrix
- macOS tests run on local dev machines
- Linux tests run on CI (default)

**Windows Test Requirements:**
- Must test .exe suffix handling
- Must test platform-specific packaged paths
- Can use GitHub Actions Windows runner
- Manual validation on actual Windows if CI unavailable

**Not tested automatically:**
- Actual binary execution (too slow, environment-dependent)
- Real file system operations (use mocks for speed)
- Network operations (not applicable to this project)

**Manual testing required for:**
- Real-world config file parsing
- Actual binary resolution on production systems
- User experience of error messages

## Confidence Metrics

**High confidence when:**
- All precedence tests pass (6 scenarios)
- Platform detection tests pass (Windows + Unix)
- Integration tests pass (2 scenarios)
- Manual testing confirms real-world usage
- Existing tests show no regression

**Medium confidence when:**
- Unit tests pass but integration tests skipped
- Manual testing incomplete
- Only tested on one platform

**Low confidence when:**
- Unit tests have gaps in precedence coverage
- No integration testing
- Existing tests failing

## Success Criteria

**Testing is complete when:**
- [ ] 6+ unit tests for precedence order
- [ ] 2+ unit tests for platform handling
- [ ] 2+ unit tests for path validation
- [ ] 2+ integration tests for CLI commands
- [ ] All 4 manual test scenarios validated
- [ ] All existing tests still pass
- [ ] Coverage >= 90% for new code
- [ ] No linting or TypeScript errors
