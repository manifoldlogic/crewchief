# Ticket: IDXCLEAN-2003: Implement User-Friendly Output Formatting

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
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement user-friendly output formatting for the stale worktree cleanup command with progress indicators, emoji indicators, summary statistics, and clear distinction between dry-run and actual deletion modes.

## Background
Users need clear, actionable output that tells them exactly what will be deleted and what happened when running the cleanup command. This ticket focuses on the user experience of running cleanup commands, ensuring that output is professional, readable, and informative in both interactive and non-interactive terminals.

This ticket implements Phase 2 - CLI Command Interface, specifically ticket IDXCLEAN-2003 from the project plan (plan.md lines 301-332). It builds upon the execution logic established in IDXCLEAN-2002 by adding polished output formatting.

## Acceptance Criteria
- [ ] Emoji indicators for different phases (🔍 for detection, 📊 for reporting, 🗑️ for deletion, ✅ for success)
- [ ] Progress messages during detection phase
- [ ] Table or list format clearly displays stale worktrees with relevant information
- [ ] Summary statistics show total chunks and time taken in human-readable format
- [ ] Verbose mode (`--verbose`) shows additional details including paths and chunk counts per worktree
- [ ] Clear warning distinguishes dry-run output from actual deletion output
- [ ] Output is readable and professional in both interactive and non-interactive terminals

## Technical Requirements
- Use consistent emoji indicators across all output phases
- Format worktree lists clearly showing worktree name, id, and path (if verbose mode)
- Show chunk counts in human-readable format with commas for thousands (e.g., "487,329" instead of "487329")
- Clearly distinguish dry-run output from actual deletion output with warning messages
- Output should work correctly in both interactive and non-interactive terminal environments
- Implement formatting helpers that can be reused for consistent output
- Ensure output aligns with the example format shown in technical notes

## Implementation Notes

The output format should follow this structure:

```rust
// Example output format
🔍 Detecting stale worktrees...

📊 Found 95 stale worktrees:
  • experiment-1 (worktree_id=42, chunks=5230)
  • experiment-2 (worktree_id=43, chunks=4821)
  ...

💾 Total chunks to delete: 487,329

⚠️  This was a dry-run. Use --confirm to actually delete.
```

For verbose mode, include additional details:
```
📊 Found 95 stale worktrees:
  • experiment-1 (worktree_id=42, path=/path/to/worktree, chunks=5,230)
  • experiment-2 (worktree_id=43, path=/path/to/other, chunks=4,821)
```

Consider implementing helper functions:
- `format_number(n: usize) -> String` - Format numbers with thousands separators
- `format_worktree_list(worktrees: &[StaleWorktree], verbose: bool) -> String` - Format list of worktrees
- `print_summary(total_chunks: usize, elapsed: Duration)` - Print final summary

The formatting should integrate with the execution logic in the `execute` method of `CleanupStaleCommand`.

## Dependencies
- **IDXCLEAN-2002** (CLI Execution Logic) - Must be completed first as this ticket builds upon the command execution flow

## Risk Assessment
- **Risk**: Emoji characters may not render correctly in all terminal environments
  - **Mitigation**: Use widely-supported emoji characters; consider fallback to plain text indicators if needed
- **Risk**: Output formatting may be inconsistent across different terminal widths
  - **Mitigation**: Use simple list format that works well regardless of terminal width; avoid complex table layouts

## Files/Packages Affected
- `crates/maproom/src/cli/commands/db.rs` - Add formatting helper functions and integrate into execute method
