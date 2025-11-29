---
description: Review active projects and archive any that are complete
---

# Context

Active projects: `.crewchief/projects/`
Archive destination: `.crewchief/archive/projects/`
Permanent docs: `/docs/`

# Task

Review all active projects and archive those that are complete, ensuring knowledge is preserved and references are updated.

## Phase 1: Inventory Active Projects

1. List all projects in `.crewchief/projects/`
2. For each project, gather ticket files from `tickets/` directory

## Phase 2: Verify Completion (Source of Truth)

**CRITICAL: The source of truth for project completion is the "Verified" checkbox in each individual ticket file, NOT the ticket index.**

Ticket indexes and README status markers can become outdated when agents mark things complete prematurely. The `verify-ticket` agent is a quality gate that actually checks for completeness before checking the "Verified" box.

For each project:

1. **Read each ticket file** (not just the index)
2. **Check for the Verified checkbox pattern:**
   ```markdown
   - [x] **Verified** - by the verify-ticket agent
   ```
3. **Count verified vs unverified tickets:**
   - Verified: `- [x] **Verified**`
   - Unverified: `- [ ] **Verified**` (or missing)

**Complete (ready to archive):**
- ✓ ALL tickets have `- [x] **Verified**` checkbox checked
- ✓ No tickets with unchecked Verified checkbox

**Partially Complete (do not archive):**
- One or more tickets have unchecked Verified checkbox
- Even if ticket index shows "Complete" status

**Abandoned/Superseded (may archive with note):**
- Project explicitly marked abandoned in README
- Work superseded by different project
- No activity and no planned continuation
- Note: Add "ABANDONED" or "SUPERSEDED" prefix to archived folder name

## Phase 3: Update Project Documents

Before archiving, ensure project documents reflect actual completion status:

**Update Ticket Index (`tickets/{SLUG}_TICKET_INDEX.md`):**
1. Update each ticket's status to match verified state:
   - `✅ Complete` - only if `- [x] **Verified**` in ticket file
   - `🟡 Pending` - if not yet started or in progress
2. Update any "Status" field at the top to reflect true completion
3. Remove any outdated time estimates or week references

**Update Project README (`README.md`):**
1. Set status to "Complete" only if all tickets verified
2. Add completion date
3. Summarize what was delivered

**Why this matters:** These documents become the historical record in the archive. They should accurately reflect what was accomplished, not optimistic mid-project status markers.

## Phase 4: Knowledge Synthesis

Before archiving, check for extractable knowledge:

**Candidates for `/docs/`:**
- Architecture decisions with lasting value
- Configuration guides that apply beyond the project
- Performance findings or benchmarks
- Troubleshooting guides
- API documentation

**Synthesis process:**
1. Review `planning/` documents for reusable content
2. Check if `/docs/` already has equivalent content
3. If new and valuable, create/update appropriate `/docs/` file
4. Keep synthesis minimal - don't duplicate unnecessarily

**Skip synthesis if:**
- Content is project-specific implementation detail
- Equivalent documentation already exists in `/docs/`
- Information is outdated or superseded

## Phase 5: Update References

Before moving project folder, find and update references:

**Search for references:**
- Other project files mentioning this project
- `/docs/` files referencing project location
- Main `README.md` or `CLAUDE.md` files

**Update actions:**
- Change paths from `projects/` to `archive/projects/`
- Update status indicators (e.g., "In Progress" → "Completed")
- Remove from active project lists

## Phase 6: Archive

For each complete project:

```bash
mv .crewchief/projects/{PROJECT}/ .crewchief/archive/projects/
```

**Post-move verification:**
- Confirm project exists in archive location
- Verify no broken references remain
- Check that project is no longer in active projects list

## Decision Criteria

**Archive if ALL true:**
- ALL tickets have `- [x] **Verified**` checkbox checked (source of truth)
- No active development planned
- Knowledge synthesized (or determined unnecessary)

**Do NOT archive if ANY true:**
- Any ticket has unchecked Verified checkbox (even if index shows "Complete")
- Active development continuing
- Blocking other active projects

## Output

Provide summary for each project showing verification status:

```
## Archive Review

### Project: {PROJECT_SLUG}

**Ticket Verification Audit:**
| Ticket | Verified | Notes |
|--------|----------|-------|
| {SLUG}-1001 | ✅ | |
| {SLUG}-1002 | ✅ | |
| {SLUG}-2001 | ❌ | Missing Verified checkbox |

**Status**: {VERIFIED_COUNT}/{TOTAL_COUNT} verified
**Decision**: {Archive / Do Not Archive / Abandoned}
**Reason**: {explanation}
```

## Example Output

```
## Archive Review

### Project: EMBPERF_ollama-parallel-optimization

**Ticket Verification Audit:**
| Ticket | Verified | Notes |
|--------|----------|-------|
| EMBPERF-0001 | ✅ | |
| EMBPERF-1001 | ✅ | |
| EMBPERF-2001 | ✅ | |
| EMBPERF-3001 | ✅ | |
| EMBPERF-3002 | ✅ | |

**Status**: 5/5 verified
**Decision**: Archive
**Reason**: All tickets verified by verify-ticket agent
**Knowledge**: Performance docs → /docs/configuration/

---

### Project: IDXCLEAN_index-stale-worktree-cleanup

**Ticket Verification Audit:**
| Ticket | Verified | Notes |
|--------|----------|-------|
| IDXCLEAN-1001 | ✅ | |
| IDXCLEAN-1002 | ❌ | Task completed but not verified |
| IDXCLEAN-1003 | ❌ | Pending |
... (truncated)

**Status**: 1/17 verified
**Decision**: Do Not Archive
**Reason**: 16 tickets not yet verified

---

## Summary

- **Reviewed**: 5 projects
- **Archived**: 1 (EMBPERF)
- **Remaining Active**: 4
- **References Updated**: 3 files
```

## Future: Automation Hooks

This command will eventually be supported by:
- **Skill**: `/archive-projects` skill with interactive verification
- **Scripts**: Automated ticket scanning to extract Verified checkbox status
- **Reports**: Machine-readable JSON output for CI/CD integration

For now, manually inspect each ticket file to verify the `- [x] **Verified**` checkbox status.
