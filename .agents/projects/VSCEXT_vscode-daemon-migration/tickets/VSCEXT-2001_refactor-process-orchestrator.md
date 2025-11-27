# Ticket: VSCEXT-2001: Refactor ProcessOrchestrator for single watch

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Refactor the existing ProcessOrchestrator to spawn a single unified `watch` process instead of separate `watch` and `branch-watch` processes. Reuse existing StdoutParser and CrashRecovery infrastructure.

## Background
The Rust `watch` command was unified (UNIWATCH project) to handle both file watching AND branch detection. The extension still spawns two processes. This refactor updates ProcessOrchestrator to use the single unified watch while preserving the valuable existing infrastructure.

Reference: planning/plan.md - Phase 2, Ticket 2001
Reference: planning/architecture.md - Single Watch Process via Refactored ProcessOrchestrator

## Acceptance Criteria
- [ ] `startWatching()` spawns single watch process (not watch + branch-watch)
- [ ] Uses verified CLI flags: `--path`, optional `--repo`, `--throttle`
- [ ] Parses `branch_switched` events correctly (via existing StdoutParser)
- [ ] Reuses existing StdoutParser for NDJSON parsing
- [ ] Reuses existing CrashRecovery for auto-restart with backoff
- [ ] Clean shutdown on extension deactivation (SIGTERM then SIGKILL)
- [ ] No references to `branch-watch` command remain

## Technical Requirements
- Keep: `StdoutParser` (process/parser.ts) - already handles NDJSON
- Keep: `CrashRecovery` (process/recovery.ts) - already implements exponential backoff
- Keep: Platform-aware binary selection logic
- Remove: `branch-watch` process spawning
- Remove: Dual-process coordination logic
- Add: Handling for `branch_switched` events

**Watch Invocation**:
```typescript
spawn(binaryPath, ['watch', '--path', workspaceRoot], {
  env: {
    MAPROOM_DATABASE_URL: `sqlite://${databasePath}`,
    MAPROOM_EMBEDDING_PROVIDER: provider,
  }
})
```

**Verified CLI flags** (from `crewchief-maproom watch --help`):
- `--repo <REPO>` - Repository name (defaults to git remote origin)
- `--path <PATH>` - Path to watch (defaults to current directory)
- `--throttle <THROTTLE>` - Debounce interval [default: 2s]
- `--worktree <WORKTREE>` - DEPRECATED (auto-detected from branch)

## Implementation Notes
1. Review current orchestrator.ts to understand dual-process logic
2. Remove the `startProcess('branch-watch', ...)` call
3. Update `startProcess('watch', ...)` to use `--path` flag
4. Add event handler for `branch_switched` events to emit to listeners
5. Ensure `stop()` method handles graceful shutdown
6. Update tests to verify single process spawning

Current problematic code (orchestrator.ts:167-184):
```typescript
// REMOVE THIS PATTERN:
await this.startProcess('watch', ['watch', '--repo', ...])
await this.startProcess('branch-watch', ['branch-watch', ...])
```

## Dependencies
- VSCEXT-1001 (BranchSwitchedEvent type needed for event handling)

## Risk Assessment
- **Risk**: Breaking existing watch functionality
  - **Mitigation**: Thorough testing, preserve StdoutParser/CrashRecovery
- **Risk**: Dual-process removal affects other code
  - **Mitigation**: Search for all references to branch-watch before removal

## Files/Packages Affected
- `packages/vscode-maproom/src/process/orchestrator.ts` - Main refactor
- `packages/vscode-maproom/src/process/orchestrator.test.ts` - Update tests
