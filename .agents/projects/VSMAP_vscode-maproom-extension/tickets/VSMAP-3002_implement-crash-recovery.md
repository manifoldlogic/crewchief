# Ticket: VSMAP-3002: Implement process crash recovery with exponential backoff

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (262 tests pass)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Detect when watch processes crash and restart them automatically with exponential backoff. Show user notification on persistent failures.

## Background
This completes Phase 3 (Process Monitoring) of the VSMAP plan. The Rust watch processes may crash due to file system errors, out-of-memory conditions, or bugs. Users should not need to manually restart the extension - we should automatically recover with intelligent backoff to avoid rapid crash loops. After exhausting retries, we present clear error information and allow manual restart.

Reference: VSMAP_PLAN.md Phase 3 "Process Monitoring - Crash Recovery"

## Acceptance Criteria
- [x] Process crash detected via exit event listener
- [x] Automatic restart with exponential backoff: 1s, 2s, 4s, 8s, 16s
- [x] Maximum 5 restart attempts before giving up
- [x] After 5 failures, show error notification with "Show Logs" button
- [x] Circuit breaker pattern: stop retrying, require manual restart
- [x] User can manually restart via command: `Maproom: Restart Watchers`
- [x] Restart count resets after 60 seconds of successful runtime

## Technical Requirements
- Listen to process exit: `process.on('exit', (code, signal) => { ... })`
- Implement exponential backoff: `delay = Math.min(2^attempt * 1000, 16000)`
- Track restart count per process (separate counters for scan/watch)
- Reset count after 60s of successful runtime using timer
- Command registration: `maproom.restartWatchers` in package.json
- Circuit breaker states: CLOSED (normal), OPEN (failed), HALF_OPEN (testing)

## Implementation Notes
Create a crash recovery module with circuit breaker pattern:

```typescript
class CrashRecovery {
  private attemptCount = 0;
  private lastRestartTime?: Date;
  private state: 'CLOSED' | 'OPEN' | 'HALF_OPEN' = 'CLOSED';

  async handleCrash(
    processType: 'scan' | 'watch',
    exitCode: number,
    signal: string | null
  ): Promise<void>;

  private calculateBackoff(attempt: number): number {
    return Math.min(Math.pow(2, attempt) * 1000, 16000);
  }

  async restart(): Promise<void>;

  reset(): void {
    this.attemptCount = 0;
    this.state = 'CLOSED';
  }
}
```

Process lifecycle:
1. Process exits (expected or crash)
2. Check exit code: 0 = normal, non-zero = crash
3. If crash, increment attempt count
4. Calculate backoff delay
5. Wait for backoff
6. Attempt restart
7. If restart succeeds, set 60s timer to reset counter
8. If restart fails, increment counter and retry
9. After 5 attempts, enter OPEN state, show notification

Error notification:
```typescript
vscode.window.showErrorMessage(
  'Maproom watcher crashed after 5 restart attempts',
  'Show Logs',
  'Restart Manually'
).then(selection => {
  if (selection === 'Show Logs') {
    output.show();
  } else if (selection === 'Restart Manually') {
    vscode.commands.executeCommand('maproom.restartWatchers');
  }
});
```

Reset timer (after successful restart):
```typescript
setTimeout(() => {
  if (process.isRunning() && Date.now() - lastRestartTime > 60000) {
    recovery.reset();
  }
}, 60000);
```

Integration with BinarySpawner:
- Add exit listener when spawning processes
- Wire to CrashRecovery.handleCrash()
- Pass process type for separate tracking

## Dependencies
- VSMAP-1003 (binary spawner) for process lifecycle integration
- VSMAP-3001 (enhanced parser) for detailed error information

## Risk Assessment
- **Risk**: Crash loop may exhaust system resources before backoff kicks in
  - **Mitigation**: Start backoff immediately (1s first retry), exponential growth
- **Risk**: User may not understand why extension stopped working
  - **Mitigation**: Clear error notification with actionable buttons
- **Risk**: Manual restart may fail with same underlying issue
  - **Mitigation**: Reset attempt counter on manual restart, allow fresh attempts

## Files/Packages Affected
- `src/process/recovery.ts` (new file, ~150 lines)
- `src/process/spawner.ts` (integrate recovery on exit events)
- `package.json` (add `maproom.restartWatchers` command)
- `src/test/recovery.test.ts` (new test file for backoff logic)
