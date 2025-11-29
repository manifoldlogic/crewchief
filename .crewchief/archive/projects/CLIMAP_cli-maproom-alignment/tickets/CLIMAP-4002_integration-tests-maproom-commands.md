# Ticket: CLIMAP-4002: Create integration tests for maproom command structure and validation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create integration tests that execute the CLI with various maproom commands and verify: (1) all subcommands are registered, (2) help text displays correctly, (3) arguments forward to Rust binary, (4) validation runs for appropriate commands, (5) validation can be bypassed with --help flag.

## Background
The CLIMAP project refactored maproom commands to use a subcommand pattern (CLIMAP-2001) and integrated environment validation (CLIMAP-3002). These changes require end-to-end integration tests that verify the complete command flow from user input through registration, help text display, argument forwarding, and validation behavior.

Integration tests complement the unit tests created in CLIMAP-4001 by testing the actual CLI execution flow rather than individual modules in isolation. These tests will execute the built CLI (`dist/cli/index.js`) and verify user-facing behavior.

This ticket implements Phase 4, Task 4.2 from the CLIMAP execution plan: "Integration Tests for maproom command structure."

**Context:**
- No backward compatibility concerns (tool has no existing users)
- Tests focus on user-facing behavior, not internal implementation
- Tests should verify command registration, help text, forwarding, and validation
- Integration tests require the CLI to be built before execution

## Acceptance Criteria
- [ ] New test file created: `packages/cli/tests/integration/maproom-commands.int.test.ts`
- [ ] Minimum 10 integration tests passing
- [ ] Test: `maproom --help` shows all 8 subcommands (scan, search, upsert, watch, db, branch-watch, cache, generate-embeddings)
- [ ] Test: `maproom scan --help` shows scan-specific help
- [ ] Test: `maproom db --help` shows db subcommands
- [ ] Test: `maproom scan` without env vars shows validation error
- [ ] Test: `maproom --help` bypasses validation (no env needed)
- [ ] Test: `maproom scan --help` bypasses validation
- [ ] Test: Arguments forward correctly to Rust binary (or show appropriate binary not found error)
- [ ] Test: Exit codes propagate correctly (validation error = exit 1, help = exit 0)
- [ ] All integration tests pass with `pnpm test:integration`

## Technical Requirements

### Test Framework
- **Framework:** Vitest (integration tests)
- **Test Location:** `packages/cli/tests/integration/maproom-commands.int.test.ts`
- **Prerequisites:** CLI must be built (`pnpm build`) before running integration tests

### Test Approach
- Use `execSync` or `spawnSync` to run the built CLI
- Execute: `node dist/cli/index.js maproom [subcommand] [args]`
- Capture stdout, stderr, and exit code for verification
- Test with controlled environment variables
- Don't rely on Rust binary being present (test CLI logic only)

### Test Structure Template
```typescript
// packages/cli/tests/integration/maproom-commands.int.test.ts
import { execSync } from 'node:child_process'
import { describe, it, expect } from 'vitest'

const CLI_PATH = 'node dist/cli/index.js'

function runCli(args: string, env: Record<string, string> = {}): {
  stdout: string
  stderr: string
  exitCode: number
} {
  try {
    const stdout = execSync(`${CLI_PATH} ${args}`, {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, ...env },
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

describe('Maproom command integration', () => {
  it('maproom --help shows all subcommands', () => {
    const { stdout, exitCode } = runCli('maproom --help')
    expect(exitCode).toBe(0)
    expect(stdout).toContain('scan')
    expect(stdout).toContain('search')
    expect(stdout).toContain('upsert')
    expect(stdout).toContain('watch')
    expect(stdout).toContain('db')
    expect(stdout).toContain('branch-watch')
    expect(stdout).toContain('cache')
    expect(stdout).toContain('generate-embeddings')
  })

  // ... more tests
})
```

### Test Categories

**1. Subcommand Registration Tests (3+ tests)**
- `maproom --help` shows all 8 subcommands
- `maproom scan --help` shows scan-specific help
- `maproom db --help` shows db subcommands (migrate)
- Each subcommand help works without throwing errors

**2. Validation Integration Tests (5+ tests)**
- `maproom scan` without MAPROOM_DATABASE_URL → validation error, exit 1
- Error message contains "MAPROOM_DATABASE_URL"
- `maproom scan` with valid env → forwards to Rust binary
- `maproom --help` works without env vars (validation bypassed)
- `maproom scan --help` works without env vars (validation bypassed)

**3. Argument Forwarding Tests (2+ tests)**
- `maproom scan --help` forwards to Rust binary (or shows binary not found)
- Help text from Rust binary appears (if binary exists)
- If binary missing, shows appropriate "crewchief-maproom not found" error

**4. Exit Code Propagation Tests (3+ tests)**
- Validation error → exit code 1
- Help command → exit code 0
- Binary not found → exit code 1

### Environment Setup for Tests
```typescript
const TEST_ENV = {
  // Minimal env to avoid validation errors
  MAPROOM_DATABASE_URL: 'postgresql://test:test@localhost:5432/test',
  MAPROOM_EMBEDDING_PROVIDER: 'openai',
  OPENAI_API_KEY: 'test-key-not-real',
}

const EMPTY_ENV = {
  // Remove database env to test validation
  PATH: process.env.PATH, // Keep PATH for node
  // Explicitly unset MAPROOM vars
}
```

## Implementation Notes

### Key Considerations
1. **Build Dependency:** Tests require CLI to be built first. Document this clearly in test file comments.
2. **Environment Isolation:** Use explicit environment variables in tests. Don't leak developer's real config.
3. **Binary Independence:** Tests should verify CLI forwarding logic, not Rust binary execution. It's OK if binary is missing.
4. **Output Validation:** Check both stdout/stderr content and exit codes.
5. **Descriptive Names:** Use clear test names that explain what's being verified.
6. **Test Grouping:** Use nested `describe` blocks to group related tests (registration, validation, forwarding).

### Test Execution Flow
```bash
# Build CLI first
cd /workspace/packages/cli
pnpm build

# Run integration tests
pnpm test:integration

# Or run specific file
pnpm test tests/integration/maproom-commands.int.test.ts
```

### Error Handling
- Wrap `execSync` in try/catch to capture non-zero exit codes
- Capture both stdout and stderr for complete output verification
- Handle cases where Rust binary doesn't exist gracefully
- Test both success and failure paths

### Security Considerations
- Don't expose real credentials in test environment variables
- Use fake/test values for API keys and connection strings
- Ensure test output doesn't leak sensitive information
- Verify validation doesn't log credentials (tested elsewhere, but verify in integration)

## Dependencies
- **CLIMAP-2001** - Command refactoring must be complete (maproom subcommand structure)
- **CLIMAP-3002** - Validation integration must be complete (environment validation)
- **Build prerequisite** - CLI must be built (`pnpm build`) before running tests

## Risk Assessment

**Risk: Tests depend on built CLI**
- **Mitigation:** Document build requirement clearly in test file and README
- **Mitigation:** Consider adding pre-test build step to `pnpm test:integration` script

**Risk: Rust binary might not exist in test environment**
- **Mitigation:** Test CLI forwarding logic only, not Rust binary execution
- **Mitigation:** Accept "binary not found" as valid outcome for forwarding tests

**Risk: Environment variable leakage between tests**
- **Mitigation:** Explicitly set environment for each test invocation
- **Mitigation:** Use isolated env objects, not process.env modifications

**Risk: Tests too slow (spawning processes)**
- **Mitigation:** Keep test count reasonable (10-15 tests)
- **Mitigation:** Integration tests are expected to be slower than unit tests

## Files/Packages Affected
- **New file:** `/workspace/packages/cli/tests/integration/maproom-commands.int.test.ts`
- **Potentially updated:** `/workspace/packages/cli/package.json` (add `test:integration` script if missing)

## Related Documentation
- CLIMAP Planning: `/workspace/.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/plan.md` (Phase 4, Task 4.2)
- Quality Strategy: `/workspace/.crewchief/projects/CLIMAP_cli-maproom-alignment/planning/quality-strategy.md` (Integration Test section)
- Related Tickets:
  - CLIMAP-2001: Command refactoring (dependency)
  - CLIMAP-3002: Validation integration (dependency)
  - CLIMAP-4001: Unit tests for validation module (parallel work)
