# Quality Strategy: CLI-Maproom Alignment

**Project:** CLIMAP - CLI-Maproom Alignment
**Date:** 2025-01-10

## Testing Philosophy

**Goal:** Ship with confidence, not ceremony

This is a refactoring project with clear boundaries:
- CLI command registration (TypeScript)
- Argument forwarding (unchanged logic)
- Environment validation (new, simple checks)
- Documentation updates (non-code)

**Risk Profile:** LOW
- No new business logic
- No database schema changes
- No API integrations
- Pure forwarding model unchanged

**Testing Strategy:** Focus on integration points and user-facing behavior

## Test Coverage Approach

### What MUST Be Tested

1. **Command Registration**
   - All new subcommands are registered
   - Arguments forward correctly
   - Help text displays properly

2. **Environment Validation**
   - Missing `MAPROOM_DATABASE_URL` shows error
   - Missing provider shows warning
   - Invalid provider config shows error
   - Validation doesn't run for non-db commands

3. **Binary Forwarding**
   - Arguments pass through correctly
   - Exit codes propagate
   - stdio inherits properly

### What Should NOT Be Tested

1. **Rust Binary Logic**
   - Not our responsibility
   - Rust has its own tests (763 passing)

2. **Environment Variable Fallbacks**
   - Handled by Rust config layer
   - Already tested in maproom

3. **Database Operations**
   - No database code in CLI
   - Forwarded to Rust

4. **Embedding Generation**
   - Rust responsibility
   - Not CLI concern

## Test Structure

### Unit Tests

**Location:** `packages/cli/tests/unit/maproom-validation.test.ts`

**Coverage:**
```typescript
describe('validateMaproomEnvironment', () => {
  beforeEach(() => {
    // Clean environment
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
  })

  it('returns valid when MAPROOM_DATABASE_URL is set', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
    expect(result.errors).toHaveLength(0)
  })

  it('returns error when no database URL is set', () => {
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(false)
    expect(result.errors).toContain(expect.stringContaining('No database connection'))
  })

  it('returns warning when embedding provider not set', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
    expect(result.warnings).toContain(expect.stringContaining('MAPROOM_EMBEDDING_PROVIDER'))
  })

  it('validates OpenAI provider requires API key', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(false)
    expect(result.errors).toContain(expect.stringContaining('OPENAI_API_KEY'))
  })

  it('validates Google provider requires project ID', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(false)
    expect(result.errors).toContain(expect.stringContaining('GOOGLE_PROJECT_ID'))
  })

  it('accepts Ollama provider without additional config', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
  })
})
```

**Effort:** 1 hour
**Value:** HIGH - Catches validation logic bugs

### Integration Tests

**Location:** `packages/cli/tests/integration/maproom-commands.int.test.ts`

**Coverage:**
```typescript
import { execSync } from 'node:child_process'

describe('Maproom commands', () => {
  // Helper to run CLI and capture output
  const runCli = (args: string): { stdout: string; stderr: string; exitCode: number } => {
    try {
      const stdout = execSync(`node dist/cli/index.js ${args}`, {
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          ...process.env,
          // Set minimal env to avoid validation errors in tests
          MAPROOM_DATABASE_URL: 'postgresql://test:test@localhost:5432/test',
        },
      })
      return { stdout, stderr: '', exitCode: 0 }
    } catch (error: any) {
      return {
        stdout: error.stdout?.toString() || '',
        stderr: error.stderr?.toString() || '',
        exitCode: error.status || 1,
      }
    }
  }

  describe('Subcommand registration', () => {
    it('maproom --help shows all subcommands', () => {
      const { stdout } = runCli('maproom --help')
      expect(stdout).toContain('scan')
      expect(stdout).toContain('search')
      expect(stdout).toContain('upsert')
      expect(stdout).toContain('watch')
      expect(stdout).toContain('db')
      expect(stdout).toContain('branch-watch')
      expect(stdout).toContain('cache')
      expect(stdout).toContain('generate-embeddings')
    })

    it('maproom scan --help shows scan-specific help', () => {
      const { stdout } = runCli('maproom scan --help')
      expect(stdout).toContain('scan')
      expect(stdout).toContain('index repository files')
    })

    it('maproom db --help shows db subcommands', () => {
      const { stdout } = runCli('maproom db --help')
      expect(stdout).toContain('migrate')
    })
  })

  describe('Environment validation', () => {
    it('scan without MAPROOM_DATABASE_URL shows error', () => {
      const { stderr, exitCode } = execSync('node dist/cli/index.js maproom scan', {
        encoding: 'utf8',
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          // Remove all database env vars
          PATH: process.env.PATH,
        },
      })
      expect(stderr).toContain('No database connection')
      expect(stderr).toContain('MAPROOM_DATABASE_URL')
      expect(exitCode).not.toBe(0)
    })

    it('help command does not trigger validation', () => {
      // Help should work even without env vars
      const { exitCode } = execSync('node dist/cli/index.js maproom --help', {
        encoding: 'utf8',
        env: { PATH: process.env.PATH },
      })
      expect(exitCode).toBe(0)
    })
  })
})
```

**Effort:** 2 hours
**Value:** HIGH - Ensures user-facing behavior correct

### Manual Testing Checklist

**Before Release:**

- [ ] **Command Help Text**
  - `crewchief --help` shows maproom in commands list
  - `crewchief maproom --help` shows all subcommands
  - `crewchief maproom scan --help` shows scan-specific help
  - `crewchief maproom db --help` shows db subcommands
  - `crewchief maproom db migrate --help` shows migrate help

- [ ] **Environment Validation**
  - Without `MAPROOM_DATABASE_URL`: scan shows error
  - With `MAPROOM_DATABASE_URL`: scan attempts to run
  - Missing provider: shows warning but doesn't block
  - OpenAI without API key: shows error
  - Google without project ID: shows error
  - Ollama: works without additional config

- [ ] **Argument Forwarding**
  - `crewchief maproom scan --parallel` passes `--parallel` to Rust
  - `crewchief maproom search "query" --limit 10` passes all args
  - Exit codes propagate (Rust exit 1 → CLI exit 1)

- [ ] **Documentation**
  - README examples all work
  - Environment variable table is accurate
  - Troubleshooting section helps with common errors

**Effort:** 30 minutes
**Value:** CRITICAL - Last check before users see changes

## Critical Integration Points

### 1. Command Registration

**Risk:** Subcommand not accessible

**Test:** `crewchief maproom <subcommand> --help` works

**Mitigation:** Integration test coverage

### 2. Argument Forwarding

**Risk:** Args not passed to Rust binary

**Test:** `crewchief maproom scan --flag value` → Rust receives `['scan', '--flag', 'value']`

**Mitigation:**
- Unit test for argument array construction
- Integration test with actual binary
- Manual test with `--help` (no-op, safe to test)

### 3. Environment Validation

**Risk:** False positives/negatives

**Test:** Each validation condition covered

**Mitigation:**
- Comprehensive unit tests (6+ cases)
- Integration tests for error display
- Manual testing with real environment

## Performance Testing

**Concern:** Command startup time

**Baseline:** Measure current `maproom:scan --help` time
```bash
time crewchief maproom:scan --help
```

**After Changes:** Measure new `maproom scan --help` time
```bash
time crewchief maproom scan --help
```

**Acceptance:** No more than 10ms regression (validation overhead)

**Effort:** 5 minutes
**Value:** MEDIUM - Ensures no performance degradation

## Regression Testing

**Scope:** Ensure existing functionality unaffected

**Test Cases:**
1. **Other CLI commands still work**
   - `crewchief worktree create test` (unrelated to maproom)
   - `crewchief agent list` (unrelated to maproom)
   - `crewchief spawn claude "task"` (unrelated to maproom)

2. **Existing CLI tests still pass**
   - Run full test suite: `pnpm test`
   - Verify 922 tests still pass (from previous work)

**Effort:** 15 minutes
**Value:** HIGH - Catch unintended side effects

## Edge Cases

### 1. Binary Not Found

**Scenario:** `crewchief-maproom` binary missing

**Expected:** Clear error message with setup instructions

**Test:**
```typescript
it('shows helpful error when binary not found', () => {
  // Mock resolvePackagedMaproomBin to return null
  const { stderr } = runCli('maproom scan')
  expect(stderr).toContain('crewchief-maproom not found')
  expect(stderr).toContain('pnpm build:rust')
})
```

### 2. Empty Arguments

**Scenario:** `crewchief maproom` (no subcommand)

**Expected:** Show help text

**Test:**
```typescript
it('maproom without subcommand shows help', () => {
  const { stdout } = runCli('maproom')
  expect(stdout).toContain('scan')
  expect(stdout).toContain('search')
})
```

### 3. Unknown Subcommand

**Scenario:** `crewchief maproom invalid-command`

**Expected:** Rust binary handles it (unknown command error)

**Test:** Manual only (Rust responsibility)

## Test Data Requirements

**None.** This project doesn't require:
- Database fixtures
- Sample repositories
- Mock embeddings
- Test credentials

**Rationale:** Pure forwarding + validation logic doesn't need data

## Continuous Integration

**Add to CI Pipeline:**
```yaml
# .github/workflows/test.yml
- name: Test CLI maproom commands
  run: |
    pnpm --filter cli test
    pnpm --filter cli test:integration
```

**Coverage Requirements:**
- Unit tests: 100% of validation logic
- Integration tests: All command paths
- Overall: Not chasing coverage %, chasing confidence

## Definition of Done

### Code Complete
- [x] All subcommands registered
- [x] Deprecated aliases implemented
- [x] Validation layer added
- [x] Documentation updated

### Tests Pass
- [ ] Unit tests: 100% pass (6+ new tests)
- [ ] Integration tests: 100% pass (15+ new tests)
- [ ] Existing CLI tests: All 922 still pass
- [ ] Manual checklist: All items verified

### Quality Gates
- [ ] Performance: <10ms regression
- [ ] Backward compat: All aliases work
- [ ] Error messages: Clear and actionable
- [ ] Help text: Complete and accurate

## Risk Mitigation

### High-Risk Changes

**Validation layer:**
- **Risk:** False errors blocking users
- **Mitigation:** Comprehensive test coverage
- **Rollback:** Remove validation (easy, separate module)

### Medium-Risk Changes

**Documentation restructure:**
- **Risk:** Confusion during transition
- **Mitigation:** Clear examples in README
- **Rollback:** Revert docs (git revert)

### Low-Risk Changes

**Command naming refactor:**
- **Risk:** None (no existing users)
- **Mitigation:** N/A
- **Rollback:** Easy if needed (change command names)

**New command registration:**
- **Risk:** Minimal (additive only)
- **Mitigation:** Integration tests
- **Rollback:** Remove registrations (easy)

## Conclusion

**Testing Approach:** Pragmatic, focused on integration points

**Coverage:**
- Critical: Command registration, validation, forwarding
- Nice-to-have: Edge cases, error messages
- Not needed: Rust logic, database operations

**Effort:** ~2.5 hours total testing
- 1 hour: Unit tests
- 1 hour: Integration tests (no backward compat tests needed)
- 30 min: Manual verification

**Confidence:** HIGH - Simple refactoring with clear test boundaries, no users to break
