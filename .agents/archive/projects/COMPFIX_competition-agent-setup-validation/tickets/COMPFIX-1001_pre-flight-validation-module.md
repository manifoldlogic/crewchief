# Ticket: COMPFIX-1001: Pre-Flight Validation Module

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
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create a comprehensive pre-flight validation module that verifies environment readiness before expensive agent operations. This module will check database connectivity, base branch indexing, worktree scanning status, MCP configuration validity, and file permissions to ensure 100% of setup failures are caught before wasting API credits.

## Background

Analysis of ultra-run-1762742953256 revealed systematic failures across all 6 completed generations:
- 0% search tool usage (agents don't have access to maproom tools)
- 0% task completion (agents can't complete tasks without tools)
- 0% success rate (complete framework failure)
- Wasted API credits (~$15-20 on failed run)

**Root causes identified:**
1. Agents don't have access to `mcp__maproom__search` tools
2. Worktrees never scanned (not indexed in database)
3. No pre-flight validation (tests start regardless of setup state)
4. Silent failures (no feedback when environment is broken)

This ticket implements the foundation of the solution: a validation framework that ensures all environments are properly configured before any agents are spawned. This is the critical path component - all other tickets depend on this validation infrastructure.

**Reference:** Section "Pre-Flight Validation Framework" in `planning/architecture.md`

## Acceptance Criteria

- [x] `PreFlightValidator` class created in `packages/cli/src/search-optimization/validation/pre-flight-validator.ts`
- [x] `checkDatabaseConnection()` method validates PostgreSQL connectivity using actual pg.Client
- [x] `verifyBaseBranchIndexed(repo, branch)` method checks base branch has chunks indexed via `maproom status --json`
- [x] `checkWorktreeScanned(repo, worktree)` method validates worktree has chunk_count > 0
- [x] `checkMcpConfigValid(worktreePath)` method parses and validates .mcp.json structure includes maproom server
- [x] `checkFilePermissions(worktreePath)` method tests actual read/write operations
- [x] `validateVariantEnvironment(env)` method runs all checks for a single variant
- [x] `validateCompetitionSetup(config)` method validates entire competition configuration
- [x] All validation functions return `CheckResult` with passed/failed status, message, and optional details
- [x] Error messages are actionable with clear troubleshooting guidance
- [x] Unit tests achieve 95%+ coverage on critical paths
- [x] All error paths tested (database failures, missing configs, permission errors)

## Technical Requirements

### Module Structure

```typescript
// packages/cli/src/search-optimization/validation/pre-flight-validator.ts

export interface CheckResult {
  passed: boolean
  message: string
  details?: any
}

export interface IndexStatus {
  indexed: boolean
  chunkCount: number
}

export interface VariantValidation {
  variantId: string
  worktreePath: string
  checks: {
    worktreeExists: CheckResult
    worktreeScanned: CheckResult
    mcpConfigValid: CheckResult
    toolsAccessible: CheckResult
    filePermissions: CheckResult
  }
  overall: 'pass' | 'fail'
  failureReason?: string
}

export interface ValidationResult {
  valid: boolean
  errors: ValidationError[]
  warnings: ValidationWarning[]
  variantResults: Map<string, VariantValidation>
}

export class PreFlightValidator {
  async checkDatabaseConnection(): Promise<boolean>
  async verifyBaseBranchIndexed(repo: string, branch: string): Promise<IndexStatus>
  async checkWorktreeScanned(repo: string, worktree: string): Promise<CheckResult>
  async checkMcpConfigValid(worktreePath: string): Promise<CheckResult>
  async checkFilePermissions(worktreePath: string): Promise<CheckResult>
  async validateVariantEnvironment(env: VariantEnvironment): Promise<VariantValidation>
  async validateCompetitionSetup(config: CompetitionConfig): Promise<ValidationResult>
}
```

### Database Connection Check

- Use `pg.Client` from 'pg' package
- Connect to `process.env.MAPROOM_DATABASE_URL`
- Execute `SELECT 1` to verify connectivity
- Handle connection errors gracefully
- Return boolean (true = connected, false = failed)

### Base Branch Indexed Check

- Execute `crewchief-maproom status --repo <repo> --worktree <branch> --json`
- Parse JSON output
- Verify worktree entry exists in `worktrees` array
- Check `chunk_count > 0`
- Return `IndexStatus` with indexed flag and chunk count

### Worktree Scanned Check

- Execute `crewchief-maproom status --repo <repo> --worktree <worktree> --json`
- Parse JSON output
- Return `CheckResult`:
  - `passed: false, message: 'Worktree not in database'` if not found
  - `passed: false, message: 'Worktree has 0 chunks indexed'` if chunk_count === 0
  - `passed: true, message: 'Indexed with N chunks'` if chunk_count > 0

### MCP Config Validation

- Read `.mcp.json` from worktree path
- Parse JSON structure
- Verify `mcpServers.maproom` exists
- Verify `command` and `args` fields present
- Return `CheckResult` with validation status
- Note: Cannot test actual tool availability without spawning agent

### File Permissions Check

- Test read access: read a known file like `package.json`
- Test write access: create temporary file `.crewchief-test-write`, then delete
- Return `CheckResult`:
  - `passed: true, message: 'Read/write permissions OK'` if both succeed
  - `passed: false, message: 'Permission error: <details>'` if either fails

### Error Message Quality

All error messages must be actionable:

**Good example:**
```
❌ Pre-flight validation failed: Database connection failed

Troubleshooting:
- Verify PostgreSQL is running: docker ps | grep postgres
- Check MAPROOM_DATABASE_URL environment variable
- Test connection: psql $MAPROOM_DATABASE_URL -c "SELECT 1"

Current value: postgresql://maproom:***@localhost:5432/maproom
```

**Bad example (avoid):**
```
Error: undefined
```

## Implementation Notes

### Technology Stack
- TypeScript with ESM modules
- pg (PostgreSQL client)
- Node.js child_process for executing maproom commands
- fs/promises for file operations
- Vitest for testing

### Testing Strategy

Create `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts`:

**Unit test coverage targets:**
- `checkDatabaseConnection`: 100% (critical path)
  - Valid connection returns true
  - Invalid URL returns false
  - Connection refused returns false
  - Mock pg.Client for failure scenarios

- `verifyBaseBranchIndexed`: 100% (critical path)
  - Base branch with chunks returns indexed=true
  - Base branch not in database returns indexed=false
  - Parse chunk count correctly

- `checkWorktreeScanned`: 100% (critical path)
  - Worktree with chunks passes
  - Worktree with 0 chunks fails with clear message
  - Worktree not in database fails with clear message

- `checkMcpConfigValid`: 100% (critical path)
  - Valid .mcp.json with maproom server passes
  - Missing .mcp.json fails
  - Malformed JSON fails
  - Missing maproom server fails

- `checkFilePermissions`: 90% (edge cases acceptable)
  - Read/write access passes
  - Read-only directory fails
  - Non-existent directory fails

**Mock strategy:**
- Mock `child_process.execSync` for maproom commands
- Mock `pg.Client` for database operations
- Use real `fs` with temporary directories for file permission tests
- Mock return values, don't mock implementations

### Performance Considerations

- Each validation check should complete in < 2 seconds
- Database connection test: single query
- Maproom status calls: may take 1-2s each
- File permission tests: minimal overhead
- Total per-variant validation: ~5-10 seconds acceptable

### Security Considerations

- Sanitize database URLs in error messages (hide passwords)
- Don't log sensitive environment variables
- Validate file paths to prevent traversal (handled by SDK, but verify)
- Use parameterized queries if extending database checks

## Dependencies

- **External:**
  - PostgreSQL running and accessible
  - `crewchief-maproom` binary in PATH
  - `.mcp.json` format specification (from Claude Code Agents SDK)

- **Internal:**
  - Type definitions for `VariantEnvironment`, `CompetitionConfig`
  - Maproom status command output format (JSON schema)

- **Blocks:**
  - COMPFIX-1003 (Enhanced Competition Runner) - needs this validation module
  - All Phase 2 tickets - depend on validation infrastructure

## Risk Assessment

- **Risk**: Maproom command output format changes
  - **Mitigation**: Parse JSON output (stable format), fallback to regex if needed

- **Risk**: Database connection tests are too slow
  - **Mitigation**: Single SELECT 1 query is fast (~10-50ms)

- **Risk**: File permission tests fail on different platforms
  - **Mitigation**: Test on Linux and macOS, document platform-specific issues

- **Risk**: False positives/negatives in validation
  - **Mitigation**: Comprehensive unit tests covering all edge cases

## Files/Packages Affected

**New files:**
- `packages/cli/src/search-optimization/validation/pre-flight-validator.ts`
- `packages/cli/src/search-optimization/validation/pre-flight-validator.test.ts`
- `packages/cli/src/search-optimization/validation/types.ts` (for shared interfaces)

**Dependencies to add:**
- `pg` - PostgreSQL client (may already be installed)

**No modifications to existing files** - this is a new module
