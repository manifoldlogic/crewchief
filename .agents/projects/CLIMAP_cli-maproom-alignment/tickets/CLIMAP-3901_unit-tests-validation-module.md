# Ticket: CLIMAP-3901: Create unit tests for environment validation module

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
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive unit tests for the `maproom-validation.ts` module covering all validation paths: valid configuration, missing database URL, missing provider, invalid provider config, and provider-specific validation for OpenAI, Google, and Ollama. Verify security requirement that no credentials appear in error messages.

## Background
The validation module was created in CLIMAP-3001 to check configuration before running maproom commands. This is critical logic that blocks user execution when configuration is invalid. Thorough testing is essential to:

- Ensure validation catches all error conditions (no false negatives)
- Prevent false positives that block users with valid configs
- Verify security requirement: never leak credentials in error messages
- Build confidence in the validation layer before production use

This ticket implements Phase 4.1 (Unit Tests) from the CLIMAP execution plan. The testing strategy focuses on confidence over coverage percentage - we test all critical paths and edge cases to ensure the validation logic is robust and secure.

**Context from Plan:**
Phase 4 focuses on comprehensive testing of new functionality. Unit tests validate the validation logic itself, while integration tests (CLIMAP-3902) will test command integration and argument forwarding.

## Acceptance Criteria
- [ ] New test file created: `packages/cli/tests/unit/maproom-validation.test.ts`
- [ ] Minimum 8 unit tests implemented and passing
- [ ] Test: Valid config (MAPROOM_DATABASE_URL set) returns `valid=true`, no errors
- [ ] Test: Missing database URL (all variants) returns `valid=false` with error
- [ ] Test: Missing provider returns warning (not error), `valid=true`
- [ ] Test: Invalid provider value returns error
- [ ] Test: OpenAI provider validation - missing API key returns error
- [ ] Test: OpenAI provider validation - API key present returns valid
- [ ] Test: Google provider validation - missing project ID returns error
- [ ] Test: Google provider validation - project ID present returns valid
- [ ] Test: Ollama provider validation - no additional requirements, returns valid
- [ ] Test: Security check - no credentials appear in error messages
- [ ] Test: Fallback database URLs (PG_DATABASE_URL, DATABASE_URL, MAPROOM_DB_HOST) work
- [ ] All tests pass when running `pnpm test maproom-validation`
- [ ] Tests execute in <100ms total (validation is fast)

## Technical Requirements

**Test Framework:** Vitest (already configured in CLI package)

**Test File Location:** `/workspace/packages/cli/tests/unit/maproom-validation.test.ts`

**Directory Creation:** Create `tests/unit/` directory if it doesn't exist

**Test Structure:**
```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import { validateMaproomEnvironment } from '../../src/cli/maproom-validation.js'

describe('validateMaproomEnvironment', () => {
  beforeEach(() => {
    // Clean environment before each test to prevent cross-contamination
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.MAPROOM_DB_HOST
    delete process.env.PG_DATABASE_URL
    delete process.env.DATABASE_URL
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.MAPROOM_OPENAI_API_KEY
    delete process.env.GOOGLE_PROJECT_ID
    delete process.env.MAPROOM_GOOGLE_PROJECT_ID
  })

  // Tests go here
})
```

**Required Test Cases:**

1. **Valid Configuration Test**
   ```typescript
   it('returns valid when MAPROOM_DATABASE_URL is set', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(true)
     expect(result.errors).toHaveLength(0)
     expect(result.warnings).toHaveLength(0)
   })
   ```

2. **Missing Database URL Test**
   ```typescript
   it('returns error when no database URL is set', () => {
     // All DB env vars unset
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(false)
     expect(result.errors.length).toBeGreaterThan(0)
     expect(result.errors[0]).toContain('database')
   })
   ```

3. **Missing Provider Warning Test**
   ```typescript
   it('returns warning when MAPROOM_EMBEDDING_PROVIDER not set', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(true)
     expect(result.warnings.length).toBeGreaterThan(0)
     expect(result.warnings[0]).toContain('MAPROOM_EMBEDDING_PROVIDER')
   })
   ```

4. **Invalid Provider Test**
   ```typescript
   it('returns error for invalid provider value', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     process.env.MAPROOM_EMBEDDING_PROVIDER = 'invalid-provider'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(false)
     expect(result.errors).toContain(expect.stringContaining('invalid'))
   })
   ```

5. **OpenAI Provider - Missing API Key**
   ```typescript
   it('returns error when OpenAI provider missing API key', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(false)
     expect(result.errors).toContain(expect.stringContaining('OPENAI_API_KEY'))
   })
   ```

6. **OpenAI Provider - API Key Present**
   ```typescript
   it('returns valid when OpenAI provider has API key', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
     process.env.OPENAI_API_KEY = 'sk-test-key'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(true)
   })
   ```

7. **Google Provider - Missing Project ID**
   ```typescript
   it('returns error when Google provider missing project ID', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(false)
     expect(result.errors).toContain(expect.stringContaining('GOOGLE_PROJECT_ID'))
   })
   ```

8. **Google Provider - Project ID Present**
   ```typescript
   it('returns valid when Google provider has project ID', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
     process.env.GOOGLE_PROJECT_ID = 'my-project'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(true)
   })
   ```

9. **Ollama Provider - No Additional Config**
   ```typescript
   it('returns valid when Ollama provider set (no additional requirements)', () => {
     process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
     process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'
     const result = validateMaproomEnvironment()
     expect(result.valid).toBe(true)
     expect(result.errors).toHaveLength(0)
   })
   ```

10. **Security Test - No Credentials in Messages**
    ```typescript
    it('does not leak credentials in error messages', () => {
      process.env.MAPROOM_DATABASE_URL = 'postgresql://user:secret-password@localhost/db'
      process.env.OPENAI_API_KEY = 'sk-super-secret-key-12345'
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'

      // Force an error by removing DB URL
      delete process.env.MAPROOM_DATABASE_URL

      const result = validateMaproomEnvironment()

      // Check that no credential values appear in messages
      const allMessages = [...result.errors, ...result.warnings].join(' ')
      expect(allMessages).not.toContain('secret-password')
      expect(allMessages).not.toContain('sk-super-secret-key')

      // Only env var names should appear, not values
      expect(allMessages).toContain('MAPROOM_DATABASE_URL') // Name is OK
    })
    ```

11. **Fallback Database URLs Test**
    ```typescript
    it('accepts PG_DATABASE_URL as fallback', () => {
      process.env.PG_DATABASE_URL = 'postgresql://localhost/test'
      const result = validateMaproomEnvironment()
      expect(result.valid).toBe(true)
    })

    it('accepts DATABASE_URL as fallback', () => {
      process.env.DATABASE_URL = 'postgresql://localhost/test'
      const result = validateMaproomEnvironment()
      expect(result.valid).toBe(true)
    })

    it('accepts MAPROOM_DB_HOST as fallback', () => {
      process.env.MAPROOM_DB_HOST = 'localhost'
      const result = validateMaproomEnvironment()
      expect(result.valid).toBe(true)
    })
    ```

## Implementation Notes

1. **Create Test Directory:**
   ```bash
   mkdir -p /workspace/packages/cli/tests/unit/
   ```

2. **Environment Cleanup:**
   - Use `beforeEach()` hook to delete all relevant env vars
   - This prevents tests from affecting each other
   - Critical for reliable test execution

3. **Test Isolation:**
   - Each test should set only the env vars it needs
   - Tests should not depend on execution order
   - Clean slate before each test

4. **Descriptive Test Names:**
   - Use clear `it()` descriptions
   - Test name should explain what's being tested
   - Makes failures easy to diagnose

5. **Assertion Patterns:**
   - Use `expect.stringContaining()` for partial matches
   - Use `.toHaveLength()` for array length checks
   - Use `.toContain()` for array element checks
   - Test both positive and negative cases

6. **Security Test:**
   - Most critical test - verifies no credential leakage
   - Set actual credential-like values in env vars
   - Verify they DON'T appear in error/warning messages
   - Only env var NAMES should appear, never values

7. **Performance:**
   - Tests should run fast (<100ms total)
   - Validation itself is synchronous and fast
   - No network calls, no file I/O

8. **Running Tests:**
   ```bash
   cd /workspace/packages/cli
   pnpm test maproom-validation
   ```

9. **Test Coverage:**
   - Aim for 100% coverage of validation logic
   - Test all branches (success and failure paths)
   - Test all three providers
   - Test all fallback database URLs

10. **Import Path:**
    - Use `.js` extension in imports (ESM requirement)
    - Path: `../../src/cli/maproom-validation.js`

## Dependencies
- **CLIMAP-3001** - Validation module must exist (REQUIRED)
- **CLIMAP-3002** - Integration with commands (not blocking for unit tests)

## Risk Assessment

- **Risk**: Tests might be flaky due to global environment state
  - **Mitigation**: Comprehensive `beforeEach()` cleanup, test isolation, no shared state between tests

- **Risk**: Missing critical test cases
  - **Mitigation**: Comprehensive list of 11+ test cases covering all paths, security review of test coverage

- **Risk**: Security test might not catch all credential leakage scenarios
  - **Mitigation**: Test with realistic credential formats, check all message types (errors + warnings), verify only names appear

- **Risk**: False confidence from passing tests that don't actually validate behavior
  - **Mitigation**: Each test explicitly verifies expected behavior, not just "no errors", check specific error messages

## Files/Packages Affected
- `/workspace/packages/cli/tests/unit/maproom-validation.test.ts` (NEW - to be created)

## Testing the Tests

**Run Tests:**
```bash
cd /workspace/packages/cli
pnpm test maproom-validation
```

**Expected Output:**
```
✓ packages/cli/tests/unit/maproom-validation.test.ts (11)
  ✓ validateMaproomEnvironment (11)
    ✓ returns valid when MAPROOM_DATABASE_URL is set
    ✓ returns error when no database URL is set
    ✓ returns warning when MAPROOM_EMBEDDING_PROVIDER not set
    ✓ returns error for invalid provider value
    ✓ returns error when OpenAI provider missing API key
    ✓ returns valid when OpenAI provider has API key
    ✓ returns error when Google provider missing project ID
    ✓ returns valid when Google provider has project ID
    ✓ returns valid when Ollama provider set
    ✓ does not leak credentials in error messages
    ✓ accepts PG_DATABASE_URL as fallback
    ✓ accepts DATABASE_URL as fallback
    ✓ accepts MAPROOM_DB_HOST as fallback

Test Files  1 passed (1)
Tests  11+ passed (11+)
```

**Verify:**
- All tests pass
- Execution time <100ms
- No warnings or errors from test framework
- Coverage includes all validation paths

## Planning References
- `.agents/projects/CLIMAP_cli-maproom-alignment/planning/plan.md` - Phase 4.1: Unit Tests
- `.agents/projects/CLIMAP_cli-maproom-alignment/planning/quality-strategy.md` - Unit test requirements
- `.agents/projects/CLIMAP_cli-maproom-alignment/planning/security-review.md` - Credential handling security
- `.agents/projects/CLIMAP_cli-maproom-alignment/tickets/CLIMAP-3001_environment-validation-module.md` - Module being tested
