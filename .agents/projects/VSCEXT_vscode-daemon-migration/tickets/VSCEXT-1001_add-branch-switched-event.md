# Ticket: VSCEXT-1001: Add BranchSwitchedEvent to events.ts

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
Add the `branch_switched` event type to the events.ts file that the unified watch command emits when the git branch changes. This is a prerequisite for the ProcessOrchestrator refactor.

## Background
The Rust `watch` command was unified (UNIWATCH project) to handle both file watching AND branch detection in a single process. This unified watch emits a `branch_switched` event when the git branch changes, but the extension's events.ts doesn't have this type defined yet.

Reference: planning/plan.md - Phase 1, Ticket 1001

## Acceptance Criteria
- [ ] `BranchSwitchedEvent` interface defined with all required fields
- [ ] `WatchEvent` union type includes `BranchSwitchedEvent`
- [ ] `isWatchEvent()` type guard validates `branch_switched` events
- [ ] Unit tests pass for valid and invalid branch_switched events

## Technical Requirements
- Add interface to `packages/vscode-maproom/src/process/events.ts`
- Event schema from Rust binary:
  ```typescript
  interface BranchSwitchedEvent {
    type: 'branch_switched'
    timestamp: string           // ISO 8601
    repo: string
    old_branch: string
    new_branch: string
    old_worktree_id: number
    new_worktree_id: number
    worktree_created: boolean   // true if new worktree was created
  }
  ```
- Type guard case:
  ```typescript
  case 'branch_switched':
    return (
      typeof event.timestamp === 'string' &&
      typeof event.repo === 'string' &&
      typeof event.old_branch === 'string' &&
      typeof event.new_branch === 'string' &&
      typeof event.old_worktree_id === 'number' &&
      typeof event.new_worktree_id === 'number' &&
      typeof event.worktree_created === 'boolean'
    )
  ```

## Implementation Notes
1. Review existing event types in `events.ts` for pattern consistency
2. The `worktree_created` boolean indicates if a new worktree entry was created in SQLite
3. Add unit tests following existing test patterns in the package
4. This event enables the status bar to show the current branch

## Dependencies
- None (this is a foundational ticket)

## Risk Assessment
- **Risk**: Event schema might not match Rust output exactly
  - **Mitigation**: Schema verified from Rust indexer source code (crates/maproom/src/indexer/mod.rs)

## Files/Packages Affected
- `packages/vscode-maproom/src/process/events.ts` - Add interface and update type guard
- `packages/vscode-maproom/src/process/events.test.ts` - Add unit tests (create if needed)
