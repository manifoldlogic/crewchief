# Ticket: CLIMAP-3002: Integrate environment validation into maproom command execution

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
Modify `runMaproomForward()` function to call validation before spawning the Rust binary. Display validation results, block execution on errors, allow execution on warnings only. Skip validation for help commands to maintain good UX.

## Background
After creating the validation module in CLIMAP-3001, this ticket integrates it into the command execution flow. The validation layer runs before forwarding to the Rust binary, blocking on errors but allowing warnings to proceed. Help commands skip validation to avoid blocking users trying to get help.

This implements **Phase 3: Environment Validation, Task 3.2** from the project plan - integrating validation checks into command execution to provide better error messages than cryptic Rust errors.

## Acceptance Criteria
- [ ] `runMaproomForward()` modified to call validation before spawning Rust binary
- [ ] Validation runs for commands: `scan`, `upsert`, `search`, `generate-embeddings`
- [ ] Validation skipped for commands: `--help`, `-h` flags, and `cache` subcommand
- [ ] Validation errors displayed via `displayValidationResult()` function
- [ ] Execution blocked (exit code 1) if validation errors exist
- [ ] Execution continues normally if only warnings exist
- [ ] Help commands bypass validation entirely and display help text
- [ ] Error messages appear before Rust binary would execute

## Technical Requirements

**Commands Requiring Validation:**
- `scan` - needs database + embeddings
- `upsert` - needs database + embeddings
- `search` - needs database
- `generate-embeddings` - needs database + embeddings

**Commands Skipping Validation:**
- `--help` or `-h` flag (any command)
- `cache` subcommand (no database needed)
- `branch-watch` subcommand (let Rust handle validation)
- `db migrate` subcommand (chicken-egg problem - might need to migrate before DB is valid)

**Validation Flow:**
1. Parse subcommand and flags from args array
2. Check if validation should be skipped (help flags or non-DB commands)
3. If validation needed, call `validateMaproomEnvironment()`
4. Display results using `displayValidationResult()`
5. Block execution if `validation.valid === false`
6. Continue to Rust binary forwarding if valid or warnings only

**Exit Behavior:**
- Errors: Display error message, set `process.exitCode = 1`, return early (don't spawn Rust binary)
- Warnings only: Display warnings, continue to Rust binary
- Valid: No validation output, continue to Rust binary

## Implementation Notes

**Proposed Implementation:**
```typescript
function runMaproomForward(args: string[]) {
  const subcommand = args[0]

  // Skip validation for help and non-DB commands
  const skipValidation =
    args.includes('--help') ||
    args.includes('-h') ||
    subcommand === 'cache'

  if (!skipValidation) {
    // Commands that need validation
    const needsValidation = ['scan', 'upsert', 'search', 'generate-embeddings']

    if (needsValidation.includes(subcommand)) {
      const validation = validateMaproomEnvironment()
      displayValidationResult(validation)

      if (!validation.valid) {
        logger.error('Fix the errors above before continuing.')
        process.exitCode = 1
        return
      }
    }
  }

  // Forward to Rust binary (existing logic)
  const bin = resolvePackagedMaproomBin()
  if (!bin) {
    logger.error('crewchief-maproom binary not found.')
    process.exitCode = 1
    return
  }

  const res = spawnSync(bin, args, { stdio: 'inherit' })
  if (res.status !== 0) process.exitCode = res.status ?? 1
}
```

**Key Design Decisions:**
1. Import validation functions from `./maproom-validation` module
2. Check subcommand to determine if validation needed
3. Check for help flags to skip validation (improves UX)
4. Display validation result before binary spawn
5. Block and return early if errors exist (don't spawn Rust)
6. Preserve all existing binary resolution and error handling logic
7. Validation happens synchronously before spawn (fast enough)

**Error Handling:**
- Validation errors: Display, set exit code, return early
- Binary not found: Preserve existing error handling
- Rust binary errors: Preserve existing error handling (forward exit code)

**Performance Considerations:**
- Validation is fast (<10ms) - minimal overhead
- Only runs for database-dependent commands
- Skipped for help commands (no delay)

## Dependencies
- **CLIMAP-3001** - Environment validation module must be created first (provides `validateMaproomEnvironment()` and `displayValidationResult()`)
- **CLIMAP-2001** - Command refactoring (new subcommand structure established)

## Risk Assessment
- **Risk**: Validation might block users inappropriately
  - **Mitigation**: Skip validation for help commands and non-DB commands like `cache`

- **Risk**: Validation might be too strict, preventing legitimate use cases
  - **Mitigation**: Warnings don't block execution, only errors block

- **Risk**: Performance overhead on every command
  - **Mitigation**: Validation is <10ms, only runs for DB commands

- **Risk**: Users might not understand validation errors
  - **Mitigation**: `displayValidationResult()` provides clear, actionable messages (from CLIMAP-3001)

## Files/Packages Affected
- `/workspace/packages/cli/src/cli/maproom.ts` - Modify `runMaproomForward()` function to add validation layer

**Testing Requirements:**
- Manual test: `crewchief maproom scan` without env vars → validation error message, no Rust spawn
- Manual test: `crewchief maproom --help` → no validation, help displays immediately
- Manual test: `crewchief maproom scan --help` → no validation, help displays
- Manual test: `crewchief maproom cache` → no validation, cache command runs
- Manual test: With valid env vars → validation passes silently or shows warnings, Rust binary runs
- Integration tests in CLIMAP-4002 will cover automated validation testing

**Related Documentation:**
- Architecture doc section 2 (validation integration)
- Plan phase 3, task 3.2 (integrate validation into commands)
- CLIMAP-3001 (validation module implementation)

**Estimated Effort:** 1-2 hours
