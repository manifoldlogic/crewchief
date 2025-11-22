# Ticket: UNIWATCH-4003: Update Documentation for Unified Watch Command

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Update CLAUDE.md, CLI help text, and add NDJSON event documentation for the unified watch command.

## Background
With unified watching, we need to update documentation to:
1. Explain that watch now handles branch switches automatically
2. Document the new branch_switched NDJSON event
3. Provide migration examples
4. Update help text

This ticket completes Phase 4 (CLI Integration & Polish) of the UNIWATCH project by ensuring all documentation reflects the unified watch command behavior.

## Acceptance Criteria
- [x] `crates/maproom/CLAUDE.md` updated with unified watch behavior
- [x] CLI help text updated (`--help` output for watch command)
- [x] NDJSON events documented (branch_switched event format)
- [x] Migration examples provided showing before/after usage
- [x] No broken documentation links
- [x] Markdown formatting correct and validated

## Technical Requirements
- Files to modify:
  - `crates/maproom/CLAUDE.md` (~30 line modifications)
  - `crates/maproom/src/cli/mod.rs` or wherever help text is defined (~10 modifications)
  - Consider adding `crates/maproom/docs/NDJSON_EVENTS.md` (~50 new lines)
- Use clear, concise language
- Include code examples that can be tested
- Ensure consistent terminology across all documentation

## Implementation Notes

### CLAUDE.md updates

Update the "Development" → "Run commands" section:
- Update watch command description to mention automatic branch detection
- Add note about branch_switched NDJSON event
- Update examples to show simplified usage
- Add migration note from old two-command pattern

Example update:
```markdown
### Watch Command (Unified)

Watch a repository for file changes and branch switches:

```bash
maproom watch
```

The watch command now automatically:
- Detects the current branch
- Watches for file changes
- Detects branch switches and re-indexes
- Emits NDJSON events including branch_switched

The `--worktree` flag is deprecated (still works with warning).
```

### CLI Help Text Updates

Update the clap definition for the watch command:

```rust
/// Watch repository for file changes and branch switches.
/// Automatically detects branch switches and re-indexes.
/// Emits NDJSON events to stdout for integration with tools.
#[clap(name = "watch")]
Watch {
    /// Repository name (auto-detected if not provided)
    #[clap(long)]
    repo: Option<String>,

    /// DEPRECATED: Worktree is now auto-detected
    #[clap(long)]
    worktree: Option<String>,

    /// Path to the repository (defaults to current directory)
    #[clap(long, default_value = ".")]
    path: PathBuf,

    /// Throttle duration in milliseconds
    #[clap(long, default_value = "100")]
    throttle: u64,
}
```

### NDJSON Event Documentation

Create or update documentation for NDJSON events, specifically the branch_switched event:

```markdown
## branch_switched Event

Emitted when the watch command detects a branch switch.

**Format:**
```json
{
  "type": "branch_switched",
  "timestamp": "2025-01-16T10:30:00Z",
  "repo": "crewchief",
  "old_branch": "main",
  "new_branch": "feature-auth",
  "old_worktree_id": 1,
  "new_worktree_id": 42,
  "worktree_created": false
}
```

**Fields:**
- `type`: Always "branch_switched"
- `timestamp`: ISO 8601 timestamp of when switch was detected
- `repo`: Repository name
- `old_branch`: Previous branch name
- `new_branch`: Current branch name
- `old_worktree_id`: Database ID of previous worktree record
- `new_worktree_id`: Database ID of current worktree record
- `worktree_created`: Boolean indicating if worktree record was newly created

**When emitted:**
- After successful branch switch detection
- After worktree record lookup/creation
- Before starting watch on new branch
```

### Migration Examples

Include clear before/after examples:

```markdown
## Migration from Separate Commands

**Before (required two commands):**
```bash
maproom watch --repo myproject --worktree main
maproom branch-watch --repo .
```

**After (single unified command):**
```bash
maproom watch
```

The watch command now handles both file watching and branch switch detection automatically.
```

## Dependencies
- UNIWATCH-4001 (CLI changes should be documented)
- UNIWATCH-2002 (BranchSwitchEvent format should be documented)

## Risk Assessment
- **Risk**: Documentation drift if implementation changes
  - **Mitigation**: Review documentation during verification phase, ensure examples are accurate
- **Risk**: Examples might become outdated
  - **Mitigation**: Include actual commands that can be tested manually
- **Risk**: Inconsistent terminology across docs
  - **Mitigation**: Use consistent terms: "unified watch", "branch switch", "auto-detect"

## Files/Packages Affected
- `crates/maproom/CLAUDE.md` (~30 line modifications)
- `crates/maproom/src/cli/mod.rs` (or wherever CLI is defined) (~10 line modifications)
- `crates/maproom/docs/NDJSON_EVENTS.md` (new file, ~50 lines) - optional but recommended
