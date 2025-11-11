# Ticket: CLIMAP-3001: Create environment validation module for maproom commands

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create a new TypeScript validation module that validates MAPROOM_* environment variables before forwarding to the Rust binary. Provides structured validation results with friendly error messages, links to documentation, and actionable next steps. Includes database URL validation and embedding provider-specific checks.

## Background
Currently, the CLI forwards maproom commands directly to the Rust binary without validating environment configuration. When users have missing or incorrect configuration (missing DATABASE_URL, wrong provider config, missing API keys), they receive cryptic Rust errors that are difficult to understand and debug.

This ticket implements the Environment Validation Layer described in the CLIMAP architecture document (Section 2). The validation layer provides:
- Early detection of configuration errors before spawning Rust binary
- Friendly, actionable error messages with setup instructions
- Security-conscious output that never logs credentials
- Fast validation (<10ms) using only environment variable checks

Users commonly struggle with:
- Missing DATABASE_URL (and not knowing about the 4 fallback variants)
- Wrong embedding provider configuration
- Missing API keys for their chosen provider

This validation layer catches these issues early and provides helpful guidance.

## Acceptance Criteria
- [ ] New file created: `packages/cli/src/cli/maproom-validation.ts`
- [ ] `validateMaproomEnvironment()` function implemented with ValidationResult return type
- [ ] Returns structured `ValidationResult` with valid/errors/warnings fields
- [ ] Database URL validation checks all 4 fallback variants (MAPROOM_DATABASE_URL, MAPROOM_DB_HOST, PG_DATABASE_URL, DATABASE_URL)
- [ ] Embedding provider validation for all 3 providers (OpenAI, Google, Ollama)
- [ ] Provider-specific validation (API keys for OpenAI, project IDs for Google)
- [ ] `displayValidationResult()` function formats output nicely with emojis
- [ ] No credentials appear in error messages (security requirement verified)
- [ ] Error messages link to relevant README sections
- [ ] Validation executes in <10ms (no network calls, no file I/O)
- [ ] Unit tests created demonstrating validation logic (tested in CLIMAP-3902)

## Technical Requirements

**ValidationResult Interface:**
```typescript
export interface ValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}
```

**Functions to Implement:**

1. `validateMaproomEnvironment(): ValidationResult`
   - Check database URL across all 4 fallback variants
   - Check embedding provider (MAPROOM_EMBEDDING_PROVIDER)
   - Perform provider-specific validation based on selected provider
   - Return structured result with errors and warnings
   - Must be fast (<10ms) - only check process.env, no I/O

2. `displayValidationResult(result: ValidationResult): void`
   - Display errors with ❌ emoji and clear formatting
   - Display warnings with ⚠️ emoji
   - Show actionable next steps for each error
   - Include links to documentation sections
   - Use logger.error() for errors, logger.warn() for warnings

**Validation Logic:**

**Database URL Validation:**
- Error if none of these environment variables are set:
  - MAPROOM_DATABASE_URL
  - MAPROOM_DB_HOST
  - PG_DATABASE_URL
  - DATABASE_URL
- Error message: "No database connection configured. Set MAPROOM_DATABASE_URL environment variable."
- Include link: "See: https://github.com/your-org/crewchief#database-setup"
- Return early if this check fails (most critical)

**Embedding Provider Validation:**
- Warning (not error) if MAPROOM_EMBEDDING_PROVIDER not set
- Message: "Embeddings will not be generated during indexing."
- If provider is set, validate it's one of: ['ollama', 'openai', 'google']
- Error if unknown provider value

**Provider-Specific Validation (OpenAI):**
- If provider=openai, check for OPENAI_API_KEY or MAPROOM_OPENAI_API_KEY
- Error if neither is set
- Message: "OpenAI provider requires OPENAI_API_KEY or MAPROOM_OPENAI_API_KEY"

**Provider-Specific Validation (Google):**
- If provider=google, check for GOOGLE_PROJECT_ID or MAPROOM_GOOGLE_PROJECT_ID
- Error if neither is set
- Message: "Google provider requires GOOGLE_PROJECT_ID or MAPROOM_GOOGLE_PROJECT_ID"

**Provider-Specific Validation (Ollama):**
- No additional validation needed (uses local endpoint)

**Security Requirements:**
- NEVER log credential values in error messages
- NEVER log connection strings in error messages
- NEVER log API keys in error messages
- Only reference environment variable NAMES, never their values
- Use generic error messages that don't leak sensitive information

**Performance Requirements:**
- Validation must complete in <10ms
- No network calls
- No file system reads
- Only check process.env

## Implementation Notes

1. **File Creation**: Create new file at `src/cli/maproom-validation.ts`

2. **Environment Variable Access**: Use `process.env` to read all environment variables

3. **Database URL Checking**: Check all 4 variants in order of precedence:
   - MAPROOM_DATABASE_URL (highest priority)
   - MAPROOM_DB_HOST
   - PG_DATABASE_URL
   - DATABASE_URL (fallback)
   - If ANY of these is set, database validation passes

4. **Provider Validation**: Validate the provider value against known providers:
   ```typescript
   const VALID_PROVIDERS = ['ollama', 'openai', 'google']
   ```

5. **Security**: Never include actual environment variable values in error messages. Only reference the variable names.

6. **Logging**: Use the existing logger from `src/cli/logger.ts`:
   ```typescript
   import { logger } from './logger.js'
   logger.error('Error message')
   logger.warn('Warning message')
   ```

7. **Fast Execution**: Keep validation fast by only checking environment variables - no network calls, no file reads, no database connections

8. **Return Early**: If database URL is missing (most critical check), return immediately with error

9. **Error Format**: Structure errors as actionable messages with clear next steps

## Dependencies
- **CLIMAP-2001** (command refactoring) - Validation will be integrated into commands after they are refactored
- No blocking dependencies - can be implemented in parallel

## Risk Assessment

- **Risk**: False positives blocking users with valid alternative configurations
  - **Mitigation**: Comprehensive unit testing covering all configuration scenarios, including the 4 database URL fallback variants

- **Risk**: Security vulnerability - accidentally logging credentials in error messages
  - **Mitigation**: Code review focused on security, unit tests verifying no credentials appear in output, explicit security requirement in acceptance criteria

- **Risk**: Performance degradation - validation taking too long
  - **Mitigation**: Only perform environment variable checks (no I/O), measure execution time in tests, require <10ms performance

- **Risk**: Missing edge cases in provider-specific validation
  - **Mitigation**: Test suite covering all 3 providers and their specific requirements

## Files/Packages Affected
- `/workspace/packages/cli/src/cli/maproom-validation.ts` (NEW - to be created)

**Testing Requirements (CLIMAP-3902):**
- Unit tests will be created in a follow-up ticket
- Test cases needed:
  - Valid configuration returns `valid=true`
  - Missing database URL returns error
  - Missing provider shows warning
  - Invalid provider value shows error
  - OpenAI provider without API key shows error
  - Google provider without project ID shows error
  - Ollama provider passes without additional config
  - No credentials appear in error messages (security test)
  - Execution time is <10ms (performance test)

## Planning References
- CLIMAP Architecture Document - Section 2: Environment Validation Layer
- CLIMAP Security Review - Credential handling and output security
- CLIMAP Quality Strategy - Unit test requirements and coverage goals
