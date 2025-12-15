# Quality Strategy: Maproom Binary Configuration

## Testing Philosophy

**Confidence over coverage.** We test to ensure the feature works correctly, not to hit arbitrary coverage metrics.

**Focus areas:**
1. **Critical path**: Config-based binary resolution must work
2. **Integration**: cleanMaproomRecords must use config correctly
3. **Backward compatibility**: Existing usage must not break
4. **Error handling**: Graceful fallback when config unavailable

**Not testing:**
- Edge cases already covered by existing 20+ tests
- Internal implementation details (e.g., path.resolve mechanics)
- Documentation rendering (manual verification sufficient)

## Test Types

### Unit Tests

**Scope:**
- cleanMaproomRecords with config parameter
- cleanMaproomRecords without config (loads it)
- Config load error handling
- Binary resolution with config path

**Tools:**
- Vitest (existing test framework)
- Mocking: vi.mock for fs, child_process, loadConfig

**Coverage Target:**
- **Not aiming for 100%** - focus on confidence
- Critical paths must be covered:
  - Config parameter passed → uses it
  - Config not passed → loads it
  - Config load fails → falls back gracefully
- Existing tests (26 cases in clean-maproom-records.test.ts) already cover binary resolution logic

**Test files:**
- `packages/cli/tests/unit/clean-maproom-records.test.ts` (may need updates)
- `packages/cli/tests/utils/maproom-binary.test.ts` (already comprehensive)

**New test cases needed:**
```typescript
describe('cleanMaproomRecords with config', () => {
  it('uses config parameter when provided', async () => {
    const config = {
      repository: {
        maproomBinaryPath: '/custom/path/maproom'
      }
    }
    // Mock binary exists, verify it's used
  })

  it('loads config when not provided', async () => {
    // Mock loadConfig to return config with maproomBinaryPath
    // Verify config is loaded and binary path used
  })

  it('handles config load errors gracefully', async () => {
    // Mock loadConfig to throw error
    // Verify falls back to env var/packaged binary
  })
})
```

**Estimate:** 3 test cases, ~60-80 lines of code

### Integration Tests

**Scope:** Not needed for this project.

**Rationale:**
- Changes are localized to one function
- Unit tests with mocking provide sufficient confidence
- Existing integration tests (if any) will catch regressions
- Manual verification in Phase 4 covers end-to-end flow

### End-to-End Tests

**Scope:** Manual testing only (Phase 4 verification)

**Approach:**
1. Create crewchief.config.local.js with maproomBinaryPath
2. Run `crewchief maproom scan` → verify uses custom binary
3. Run without config → verify falls back correctly
4. Test with env var set → verify env var takes precedence

**Not automated because:**
- Requires actual binary files on filesystem
- Environment setup too complex for CI
- Manual verification sufficient for this feature
- Existing comprehensive unit tests catch logic issues

## Critical Paths

The following paths MUST be tested:

### 1. Config Parameter Provided
**Path:** cleanMaproomRecords called with config object
**Expected:** Uses config.repository.maproomBinaryPath
**Test:** Mock config, verify findMaproomBinary receives configPath

### 2. Config Parameter Not Provided
**Path:** cleanMaproomRecords called without config
**Expected:** Loads config via loadConfig(), uses maproomBinaryPath
**Test:** Mock loadConfig, verify it's called and result used

### 3. Config Load Fails
**Path:** cleanMaproomRecords called without config, loadConfig throws
**Expected:** Catches error, continues with fallback (env var/packaged)
**Test:** Mock loadConfig to throw, verify no crash and fallback works

### 4. Config Path Not Set
**Path:** Config exists but maproomBinaryPath is undefined
**Expected:** Falls through to global/packaged binary
**Test:** Mock config without maproomBinaryPath, verify fallback

### 5. Resolution Priority Order
**Path:** Multiple binary sources available
**Expected:** Env var > config > global > packaged
**Test:** Already covered by existing maproom-binary.test.ts (lines 58-129)

## Test Data Strategy

**Approach:** Mocking over fixtures

**Rationale:**
- No complex test data needed (config is simple objects)
- Mocking allows precise control over scenarios
- Faster than filesystem operations
- Already established pattern in codebase

**Mock strategy:**
```typescript
// Mock loadConfig
vi.mock('../config/loader.js', () => ({
  loadConfig: vi.fn()
}))

// Mock findMaproomBinary
vi.mock('../utils/maproom-binary.js', () => ({
  findMaproomBinary: vi.fn()
}))

// Test setup
vi.mocked(loadConfig).mockResolvedValue({
  repository: {
    maproomBinaryPath: '/test/path'
  }
})

vi.mocked(findMaproomBinary).mockReturnValue({
  path: '/test/path',
  source: 'config'
})
```

**No fixtures needed** - all data can be inline test objects.

## Quality Gates

Before verification (Phase 4):

- [ ] All unit tests pass (`pnpm test`)
- [ ] No new linting errors (`pnpm lint`)
- [ ] TypeScript compiles without errors (`pnpm build`)
- [ ] No test coverage regression (verify existing tests still pass)

Before commit:

- [ ] Verify-ticket acceptance criteria met
- [ ] Manual testing confirms config works
- [ ] Documentation reviewed for clarity
- [ ] No breaking changes to public APIs

## Test Execution

### Local Development

```bash
# Run all tests
pnpm test

# Run specific test file
pnpm test clean-maproom-records

# Watch mode during development
pnpm test:watch

# With coverage (optional)
pnpm test --coverage
```

### CI Pipeline

**Existing CI runs tests automatically:**
- Triggered on PR creation
- Must pass before merge
- No changes needed to CI configuration

## Known Test Limitations

### What We're NOT Testing

1. **Actual binary execution**: Tests mock binary resolution, don't execute real maproom binary
   - **Rationale:** Unit tests focus on resolution logic, not binary behavior
   - **Mitigation:** Manual verification in Phase 4

2. **Filesystem interactions**: Tests mock fs.existsSync, don't check real files
   - **Rationale:** Makes tests fast and deterministic
   - **Mitigation:** Existing tests are comprehensive, proven reliable

3. **Cross-platform**: Tests don't run on all platforms (Windows, macOS, Linux)
   - **Rationale:** Resolution logic is platform-agnostic except for .exe suffix (already tested)
   - **Mitigation:** Existing platform tests cover this (lines 156-187)

4. **Config file discovery**: Tests mock loadConfig, don't test findConfigFile traversal
   - **Rationale:** findConfigFile is tested separately (implicit in existing usage)
   - **Mitigation:** Manual testing confirms end-to-end flow

## Regression Prevention

**Strategy:** Comprehensive existing test suite

**Existing coverage:**
- 26 test cases in clean-maproom-records.test.ts
- Comprehensive tests in maproom-binary.test.ts
- Covers all resolution priorities
- Covers platform variations
- Covers error cases

**New coverage:**
- 2-3 test cases for cleanMaproomRecords config parameter
- Focuses on config integration

**How regressions are prevented:**
1. Existing tests continue to pass (no changes to findMaproomBinary)
2. New tests cover new code path (cleanMaproomRecords with config)
3. CI runs all tests on every PR
4. Manual verification confirms end-to-end flow

## Performance Testing

**Not in scope for this project.**

**Rationale:**
- Config loading is fast (<1ms)
- Binary resolution is fast (<5ms)
- No performance-sensitive operations introduced
- Manual testing will detect any obvious slowdowns

**If performance issues arise:**
- Profile with Node.js profiler
- Check for excessive filesystem calls
- Optimize only if measured impact > 50ms

## Test Maintenance

**Low maintenance burden:**
- Tests use standard mocking patterns
- No complex fixtures or setup
- Clear test names describe scenarios
- Follows existing test structure

**When to update tests:**
1. If cleanMaproomRecords signature changes (unlikely)
2. If binary resolution logic changes (unlikely)
3. If new config fields added (future work)

**Test debt prevention:**
- Keep tests focused (one scenario per test)
- Use descriptive test names
- Mock at module boundaries, not internals
- Avoid brittle implementation details

## Documentation Testing

**Approach:** Manual review only

**Checklist:**
- [ ] Examples compile correctly (valid JavaScript)
- [ ] Code blocks render with syntax highlighting
- [ ] Links work (if any added)
- [ ] Consistent with README.md terminology
- [ ] Clear for target audience (developers)

**Not automated because:**
- Markdown linters don't validate code examples
- Manual review is quick and effective
- Examples are simple (config objects)
