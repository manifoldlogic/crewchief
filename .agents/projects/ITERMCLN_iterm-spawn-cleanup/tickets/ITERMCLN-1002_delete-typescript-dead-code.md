# Ticket: ITERMCLN-1002: Delete Dead TypeScript Bridge Code

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
- general-development
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Delete all TypeScript files related to the abandoned JSON-RPC bridge approach and dead adapter code. These files total ~663 lines of dead code that are no longer needed.

## Background
The TypeScript codebase has three dead files from an abandoned JSON-RPC bridge approach:
1. `iterm.service.ts` - JSON-RPC client that tries to start the now-deleted `iterm_bridge.py`
2. `iterm.types.ts` - Type definitions for the dead service
3. `iterm.adapter.ts` - Imports from non-existent `terminal.interface.ts`

The `ITermProvider` in `providers/iterm.ts` currently depends on `ITermService`, so this ticket must be completed alongside ITERMCLN-2001 (which rewrites ITermProvider to use direct Python script calls via `spawnSync`).

Reference: ITERMCLN plan.md Phase 1 - Dead Code Removal

## Acceptance Criteria
- [ ] `packages/cli/src/iterm/iterm.service.ts` deleted (414 lines)
- [ ] `packages/cli/src/iterm/iterm.types.ts` deleted (94 lines)
- [ ] `packages/cli/src/terminal/iterm.adapter.ts` deleted (155 lines)
- [ ] No TypeScript compilation errors (requires ITERMCLN-2001 complete)

## Technical Requirements
- Delete these TypeScript files from `packages/cli/src/`:
  - `src/iterm/iterm.service.ts` (414 lines) - JSON-RPC client for dead Python bridge
  - `src/iterm/iterm.types.ts` (94 lines) - Type definitions for dead service
  - `src/terminal/iterm.adapter.ts` (155 lines) - Dead adapter with broken imports
- Verify no other files import from deleted modules
- Run `pnpm build` to verify compilation status

## Implementation Notes

**CRITICAL**: This ticket will break TypeScript compilation because `ITermProvider` imports `ITermService`. This is expected and intentional - the ticket must be committed together with ITERMCLN-2001 (ITermProvider rewrite) in a single atomic operation.

**Implementation Steps**:
1. Verify current imports:
   ```bash
   grep -r "from.*iterm.service\|from.*iterm.types\|from.*iterm.adapter" packages/cli/src/
   ```
2. Delete the three files:
   - `packages/cli/src/iterm/iterm.service.ts`
   - `packages/cli/src/iterm/iterm.types.ts`
   - `packages/cli/src/terminal/iterm.adapter.ts`
3. Run `pnpm build` to verify (expect failure until ITERMCLN-2001 is complete)
4. DO NOT commit this ticket alone - must be committed with ITERMCLN-2001

**Commit Strategy**:
This ticket and ITERMCLN-2001 must be completed and committed together as a single atomic change to maintain a working codebase at all commits.

## Dependencies
- **Prerequisite**: ITERMCLN-1001 (Python dead code cleanup) - should be complete
- **Must be committed with**: ITERMCLN-2001 (ITermProvider rewrite) - this is the replacement implementation

## Risk Assessment
- **Risk**: TypeScript build breaks between completing this ticket and ITERMCLN-2001
  - **Mitigation**: Complete and commit both tickets together in a single atomic operation. The commit-ticket agent should handle both ITERMCLN-1002 and ITERMCLN-2001 in one commit.
- **Risk**: Other code may have unexpected dependencies on deleted files
  - **Mitigation**: Grep for imports before deletion. The only known dependency is ITermProvider which will be rewritten in ITERMCLN-2001.

## Files/Packages Affected
- `packages/cli/src/iterm/iterm.service.ts` - DELETE
- `packages/cli/src/iterm/iterm.types.ts` - DELETE
- `packages/cli/src/terminal/iterm.adapter.ts` - DELETE
- `packages/cli/src/providers/iterm.ts` - Will have broken imports until ITERMCLN-2001 is complete
