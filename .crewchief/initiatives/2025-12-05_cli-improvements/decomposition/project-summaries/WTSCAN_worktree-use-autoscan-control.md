# Project: Worktree Use Auto-Scan Control

**Slug:** WTSCAN_worktree-use-autoscan-control
**Priority:** Medium
**Effort:** S (1-2 days)

## Summary

Add `worktree.autoScanOnWorktreeUse` config option (default: false) to control whether `worktree use` and `worktree create` automatically trigger maproom scans. Remove unconditional auto-scan behavior.

## Deliverables

1. **Config schema field** - Add `autoScanOnWorktreeUse` to `WorktreeSchema` (boolean, default: false)
2. **Conditional scanning** - Update `createWorktree()` to check config before calling `runMaproomScan()`
3. **Remove hardcoded scan** - Make scan opt-in rather than default
4. **Updated tests** - Verify auto-scan only happens when enabled
5. **Documentation** - Update README explaining auto-scan control and manual scan command

## Dependencies

**Requires:** WTPATH (config schema foundation)

## Value Proposition

Developers regain control over when indexing happens, avoiding unexpected delays when switching worktrees. Manual scanning via `crewchief maproom scan` remains available. Users who want auto-scan can enable it explicitly.

## Technical Approach

1. Add `autoScanOnWorktreeUse: z.boolean().default(false)` to `WorktreeSchema`
2. Update `WorktreeService.createWorktree()`:
   ```typescript
   if (config.worktree?.autoScanOnWorktreeUse) {
     await this.runMaproomScan(wtPath)
   }
   ```
3. Remove unconditional scan call (line 143 in `worktrees.ts`)
4. Update tests to verify scan only happens when config enabled
5. Document manual scan workflow

## Acceptance Criteria

- [ ] Config accepts `autoScanOnWorktreeUse` boolean
- [ ] `worktree create` doesn't scan by default
- [ ] `worktree create` scans when config enabled
- [ ] `worktree use` behavior matches create behavior
- [ ] Tests verify both enabled and disabled states
- [ ] Documentation explains manual vs auto-scan

## Breaking Changes

**Breaking:** Auto-scan is now opt-in, not default behavior.

**Migration:** Users who rely on auto-scan must add `worktree.autoScanOnWorktreeUse: true` to config. Document this prominently in changelog and migration guide.

**Rationale:** Auto-scan is unexpected behavior and can be slow on large codebases. Explicit control matches user expectations.
