# Ticket: IDXCLEAN-5001: Update Documentation for Cleanup Feature

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer (primary - feature implementer, understands implementation details)
- verify-ticket (documentation review)
- commit-ticket

## Summary
Update README, CHANGELOG, and create comprehensive user guides for the cleanup command and watch integration feature. This includes usage instructions, recovery procedures, and security considerations.

## Background
With all implementation complete (Phases 1-4), users need comprehensive documentation on how to use the cleanup features, what to expect, and how to recover if something goes wrong. This is a production requirement before the feature can be considered complete.

References: `.agents/projects/IDXCLEAN_index-stale-worktree-cleanup/planning/plan.md` - Phase 5 (Production Deployment), ticket IDXCLEAN-5001 (lines 700-725)

## Acceptance Criteria
- [x] README.md updated with cleanup command usage section
- [x] CHANGELOG.md updated with IDXCLEAN feature entries
- [x] User guide created (`docs/user-guide-cleanup.md`) with step-by-step cleanup instructions
- [x] Administrator guide created (`docs/admin-guide-cleanup.md`) - Note: Watch integration deferred (see Phase 4 blockers)
- [x] Recovery procedures documented (how to restore if accidental deletion) - in user guide
- [x] Security considerations documented (backup recommendations) - in admin guide

## Technical Requirements
- Documentation must be clear and accessible for non-technical users
- Include real, runnable examples (not just syntax)
- Explain dry-run workflow clearly (preview before confirm)
- Document all environment variables (MAPROOM_AUTO_CLEANUP)
- Include troubleshooting section for common issues
- Recovery procedures must be tested and verified accurate
- Follow existing documentation style and formatting
- All code examples must be valid and tested

## Implementation Notes

### README.md Addition
Add new section after existing command documentation:

```markdown
## Cleanup Stale Worktrees

Remove worktrees that no longer exist on disk:

```bash
# Preview what will be deleted (dry-run)
maproom db cleanup-stale

# Actually delete stale worktrees
maproom db cleanup-stale --confirm

# Show detailed information
maproom db cleanup-stale --verbose
```

## Automatic Cleanup

Enable automatic cleanup during watch:

```bash
export MAPROOM_AUTO_CLEANUP=true
maproom watch
```

Cleanup runs:
- Once at watch startup (background, non-blocking)
- Every 30 minutes when indexer is idle
- Automatically defers if indexing is active
```

### CHANGELOG.md Structure
Follow existing CHANGELOG format. Add entries for:
- New `cleanup-stale` command
- Watch integration with `MAPROOM_AUTO_CLEANUP`
- Background cleanup scheduling
- Dry-run safety features

### User Guide (docs/user-guide-cleanup.md)
New file covering:
- What the cleanup feature does
- When to use it
- Step-by-step workflow (dry-run → review → confirm)
- Understanding cleanup output
- Safety features
- What happens during cleanup
- Troubleshooting

### Admin Guide (docs/admin-guide-watch-cleanup.md)
New file covering:
- Watch integration overview
- Environment variable configuration
- Cleanup scheduling behavior
- Performance considerations
- Monitoring and logging
- Integration with CI/CD
- Best practices

### Documentation Style
- Use existing docs as style reference
- Include realistic examples
- Provide context for each feature
- Explain the "why" not just the "how"
- Use clear section headers
- Include code blocks with proper syntax highlighting

## Dependencies
- IDXCLEAN-1001 through IDXCLEAN-4003 (all implementation tickets complete)
- Feature fully implemented and tested
- Command-line interface finalized

## Risk Assessment
- **Risk**: Incomplete or unclear documentation could lead to user errors
  - **Mitigation**: Include comprehensive examples, dry-run emphasis, clear recovery procedures
- **Risk**: Documentation becomes stale as feature evolves
  - **Mitigation**: Document in main README and CHANGELOG for visibility
- **Risk**: Users might skip dry-run and accidentally delete data
  - **Mitigation**: Make dry-run the default, require explicit `--confirm` flag, emphasize in docs

## Files/Packages Affected

### Modified Files
- `/workspace/README.md` - Added "Database Maintenance" section with cleanup commands
- `/workspace/CHANGELOG.md` - Added IDXCLEAN feature entries under "Stale Worktree Cleanup"
- `/workspace/crates/maproom/CLAUDE.md` - Already had cleanup documentation (added earlier)

### New Files
- `/workspace/docs/user-guide-cleanup.md` - User-facing cleanup instructions with recovery procedures
- `/workspace/docs/admin-guide-cleanup.md` - Administration guide (renamed from admin-guide-watch-cleanup.md since watch is deferred)

### Notes on Watch Integration
The original ticket specified `admin-guide-watch-cleanup.md` for watch integration documentation. Since Phase 4 (watch integration) is BLOCKED due to the watch command being removed in IDXABS-2001, the admin guide was created with CLI-focused content and a placeholder section for future watch integration.
