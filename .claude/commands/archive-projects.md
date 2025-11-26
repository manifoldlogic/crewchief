---
description: Review active projects and archive any that are complete
---

# Context

Active projects: `.agents/projects/`
Archive destination: `.agents/archive/projects/`
Permanent docs: `/docs/`

# Task

Review all active projects and archive those that are complete, ensuring knowledge is preserved and references are updated.

## Phase 1: Inventory Active Projects

1. List all projects in `.agents/projects/`
2. For each project, assess completion status:
   - Read `README.md` for project status
   - Check ticket index for completion checkboxes
   - Review individual tickets for verified status

## Phase 2: Evaluate Each Project

For each project, determine completion state:

**Complete (ready to archive):**
- ✓ All tickets have "Task completed" and "Verified" checkboxes checked
- ✓ No pending or in-progress tickets remain
- ✓ Project README reflects completed status

**Partially Complete (do not archive):**
- Some tickets complete, others pending
- Active work still in progress
- Dependencies on other incomplete projects

**Abandoned/Superseded (may archive with note):**
- Project explicitly marked abandoned
- Work superseded by different project
- No activity and no planned continuation

## Phase 3: Knowledge Synthesis

Before archiving a complete project, check for extractable knowledge:

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

## Phase 4: Update References

Before moving project folder, find and update references:

**Search for references:**
- Other project files mentioning this project
- `/docs/` files referencing project location
- Main `README.md` or `CLAUDE.md` files

**Update actions:**
- Change paths from `projects/` to `archive/projects/`
- Update status indicators (e.g., "In Progress" → "Completed")
- Remove from active project lists

## Phase 5: Archive

For each complete project:

```bash
mv .agents/projects/{PROJECT}/ .agents/archive/projects/
```

**Post-move verification:**
- Confirm project exists in archive location
- Verify no broken references remain
- Check that project is no longer in active projects list

## Decision Criteria

**Archive if ALL true:**
- All tickets verified complete
- No active development planned
- Knowledge synthesized (or determined unnecessary)

**Do NOT archive if ANY true:**
- Tickets still pending or in-progress
- Active development continuing
- Blocking other active projects

## Output

Provide summary:
- Projects reviewed (count)
- Projects archived (list with slugs)
- Knowledge synthesized to `/docs/` (if any)
- Projects remaining active (list with reasons)
- References updated (count)

## Example Output

```
## Archive Summary

### Reviewed: 5 projects

### Archived (2):
- EMBPERF_ollama-parallel-optimization → Performance docs added to /docs/configuration/
- OPNFIX_open-tool-path-fix → No synthesis needed

### Remaining Active (3):
- SRCHDUP_search-result-deduplication - Tickets pending
- IDXCLEAN_index-stale-worktree-cleanup - 8/17 tickets complete
- SQLINFRA_infrastructure-simplification - In progress

### References Updated: 3 files
```
