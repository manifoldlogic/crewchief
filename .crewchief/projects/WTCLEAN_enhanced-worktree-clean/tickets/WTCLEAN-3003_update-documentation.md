# Ticket: [WTCLEAN-3003]: Update README Documentation

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
- typescript-engineer
- verify-ticket
- commit-ticket

## Summary
Update the CLI README documentation to reflect the new enhanced cleanup behavior, including the new flags, complete cleanup workflow, and manual recovery procedures.

## Background
The `worktree clean` command now performs complete cleanup (directory, metadata, branch, maproom database). Users need documentation explaining the new behavior, available flags, and how to handle failures.

This ticket implements Phase 3, Deliverable 3 from the plan: Update README documentation.

## Acceptance Criteria
- [ ] README updated with enhanced cleanup behavior
- [ ] `--keep-branch` flag documented with description and example
- [ ] `--keep-maproom` flag documented with description and example
- [ ] Complete cleanup workflow explained (what gets cleaned)
- [ ] Graceful degradation behavior documented (best-effort cleanup)
- [ ] Manual recovery procedures documented for common failures
- [ ] Examples show both success and failure scenarios
- [ ] Cross-platform considerations mentioned (binary resolution)
- [ ] CHANGELOG entry added for this feature

## Technical Requirements
- Update file: `packages/cli/README.md`
- Add/update section on `worktree clean` command
- Document new flags in flags table or list
- Add examples showing new behavior
- Add troubleshooting section for cleanup failures
- Document manual recovery procedures
- Explain graceful degradation (partial cleanup)
- Add note about batch cleanup performance for large databases
- Create CHANGELOG entry describing the enhancement

## Implementation Notes
Update the `worktree clean` section in README:

```markdown
### worktree clean

Remove a worktree and all associated artifacts.

**Enhanced Cleanup (Complete Removal):**
The clean command now performs complete cleanup automatically:
- Removes worktree directory
- Removes git worktree metadata
- Deletes git branch (safe delete, not force)
- Cleans maproom database records (if maproom installed)

**Usage:**
```bash
# Clean single worktree (complete cleanup)
crewchief worktree clean feature-123

# Clean all non-current worktrees
crewchief worktree clean --all

# Keep the git branch (only remove directory)
crewchief worktree clean feature-123 --keep-branch

# Skip maproom database cleanup
crewchief worktree clean feature-123 --keep-maproom

# Combine flags
crewchief worktree clean feature-123 --keep-branch --keep-maproom
```

**Flags:**
- `--keep-dir` - Keep the worktree directory (only remove metadata)
- `--keep-branch` - Keep git branch after removing worktree
- `--keep-maproom` - Skip maproom database cleanup
- `--all` - Clean all non-current worktrees
- `--stale` - Clean stale worktree metadata only

**Graceful Degradation:**
The clean command uses best-effort cleanup:
- If maproom binary not found: Warns but continues (directory and branch still cleaned)
- If branch deletion fails: Warns but continues (directory and maproom still cleaned)
- If multiple steps fail: Each failure logged separately with recovery instructions

**Troubleshooting:**

*Maproom binary not found:*
```
Warning: Maproom binary not found - database cleanup skipped
Run manually: crewchief-maproom db cleanup-stale --confirm
```
Install crewchief-maproom globally: `pnpm install -g`

*Branch not fully merged:*
```
Warning: Branch feature-123 not fully merged - skipped deletion
Delete manually: git branch -d feature-123 (or -D to force)
```
Review branch commits before force deleting: `git log feature-123`

*Branch checked out elsewhere:*
```
Warning: Branch feature-123 checked out in another worktree
Switch other worktree to different branch, then: git branch -d feature-123
```
Find worktree with branch: `git worktree list`

**Performance:**
- Cleanup typically completes in 1-5 seconds
- Maproom cleanup scans all worktrees in database (batch cleanup)
- For databases with 50+ worktrees, expect 2-5 seconds for maproom step
- Acceptable for cleanup operation (runs infrequently)

**Platform Support:**
- Binary discovery works on macOS, Linux, Windows
- Packaged binaries included for common platforms
- Falls back to system PATH if packaged binary not found
```

**CHANGELOG entry:**
```markdown
### Enhanced Worktree Clean

**New Features:**
- `worktree clean` now performs complete cleanup by default
- Automatically deletes git branch (safe delete only)
- Automatically cleans maproom database records
- New flags: `--keep-branch`, `--keep-maproom`
- Graceful error handling with manual recovery instructions
- Works on all platforms (macOS, Linux, Windows)

**Behavior Changes:**
- Branches deleted by default (use `--keep-branch` to preserve)
- Maproom records cleaned by default (use `--keep-maproom` to skip)
- Cleanup continues on errors (best-effort approach)
- Clear logging for each cleanup step

**Migration:**
- Existing behavior available via `--keep-branch` flag
- No breaking changes to existing flags or commands
- Complete cleanup is opt-out (not opt-in)
```

**Documentation structure:**
1. Feature overview (what it does)
2. Usage examples (common scenarios)
3. Flags reference (all available flags)
4. Graceful degradation explanation
5. Troubleshooting (common failures and fixes)
6. Performance characteristics
7. Platform support

**Writing style:**
- Clear, concise language
- Examples before explanations
- Action-oriented (tell users what to do)
- Troubleshooting uses actual error messages
- Include both success and failure scenarios

## Dependencies
- **All Phase 1 and Phase 2 tickets** - Implementation must be complete before documenting

## Risk Assessment
- **Risk**: Documentation out of sync with implementation
  - **Mitigation**: Write docs after implementation complete, verify examples work
- **Risk**: Examples don't match actual behavior
  - **Mitigation**: Test all examples manually before committing
- **Risk**: Troubleshooting doesn't cover all scenarios
  - **Mitigation**: Document common cases, add "For other errors" catch-all

## Files/Packages Affected
- `packages/cli/README.md` (update worktree clean section)
- `CHANGELOG.md` (add entry for this feature)

## Verification Notes
Verify-ticket agent should check:
- [ ] README updated with enhanced cleanup section
- [ ] Both new flags documented (`--keep-branch`, `--keep-maproom`)
- [ ] Complete cleanup workflow explained
- [ ] Examples show both flags and default behavior
- [ ] Graceful degradation documented
- [ ] Troubleshooting section includes common failures
- [ ] Manual recovery procedures provided
- [ ] Performance characteristics documented
- [ ] Platform support mentioned
- [ ] CHANGELOG entry added
- [ ] All examples tested and work correctly
- [ ] Documentation is clear and user-friendly
