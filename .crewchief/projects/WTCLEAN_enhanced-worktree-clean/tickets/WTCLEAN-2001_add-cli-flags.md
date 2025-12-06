# Ticket: [WTCLEAN-2001]: Add CLI Flags for Opt-Out Behavior

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
Add `--keep-branch` and `--keep-maproom` flags to the `worktree clean` command to allow users to opt out of the new branch deletion and maproom cleanup behavior.

## Background
The enhanced clean command will delete branches and clean maproom records by default. Users need flags to preserve old behavior where branches and database records are kept after worktree removal.

This ticket implements Phase 2, Deliverable 2 from the plan: New CLI flags for opt-out behavior.

## Acceptance Criteria
- [x] `--keep-branch` flag added to `worktree clean` command
- [x] `--keep-maproom` flag added to `worktree clean` command
- [x] Flags are optional (default behavior is to clean everything)
- [x] Flags have descriptive help text
- [x] Flags work with existing flags (`--all`, `--stale`, `--keep-dir`)
- [x] TypeScript types updated to include new flag options
- [x] No breaking changes to existing command behavior

## Technical Requirements
- Modify file: `packages/cli/src/cli/worktree.ts`
- Add flags using Commander.js `.option()` method
- Flag definitions:
  - `--keep-branch` - "Keep git branch after removing worktree"
  - `--keep-maproom` - "Skip maproom database cleanup"
- Add options to command handler signature
- Update TypeScript interfaces if needed for option types
- Ensure flags are passed through to cleanup logic
- No changes to default behavior (these are opt-OUT flags)

## Implementation Notes
Add the flags to the clean command definition in `packages/cli/src/cli/worktree.ts`:

```typescript
worktree
  .command('clean')
  .description('Remove a worktree and its directory')
  // ... existing options ...
  .option('--keep-branch', 'Keep git branch after removing worktree')
  .option('--keep-maproom', 'Skip maproom database cleanup')
  .action(async (selector, opts) => {
    // opts.keepBranch and opts.keepMaproom will be available
    // Use these to conditionally skip branch deletion and maproom cleanup
  })
```

**Option naming convention:**
- CLI flag: `--keep-branch` (kebab-case)
- TypeScript property: `opts.keepBranch` (camelCase)
- Commander.js automatically converts kebab-case to camelCase

**Integration points:**
- These flags will be checked in later tickets (2002, 2003) before cleanup
- `if (!opts.keepMaproom) { cleanMaproomRecords() }`
- `if (!opts.keepBranch) { deleteBranch() }`

**Design decision:**
- Opt-OUT flags (not opt-IN) because complete cleanup is desired default
- Matches user expectations ("clean" = "remove everything")
- Old behavior still available via flags if needed

## Dependencies
- None (can be implemented independently)

## Risk Assessment
- **Risk**: Flags conflict with existing options
  - **Mitigation**: Test with all flag combinations, ensure no conflicts
- **Risk**: Flag names unclear to users
  - **Mitigation**: Clear help text, documentation in README (Phase 3)
- **Risk**: Breaking existing behavior
  - **Mitigation**: These are additive flags, existing flags unchanged

## Files/Packages Affected
- `packages/cli/src/cli/worktree.ts` (modify command definition)

## Verification Notes
Verify-ticket agent should check:
- [ ] Both flags added to clean command
- [ ] Flags have descriptive help text
- [ ] Help text displays correctly (`crewchief worktree clean --help`)
- [ ] Flags accessible in action handler (opts.keepBranch, opts.keepMaproom)
- [ ] No TypeScript compilation errors
- [ ] Existing tests still pass (no regressions)
- [ ] Flags work with existing flags (test `--all --keep-branch`, etc.)
