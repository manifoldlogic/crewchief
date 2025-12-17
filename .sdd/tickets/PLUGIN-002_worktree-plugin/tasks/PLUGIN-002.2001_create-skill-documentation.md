# Task: [PLUGIN-002.2001]: Create Worktree Management Skill Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation task, no executable tests)
- [x] **Verified** - by the verify-task agent

## Agents
- general-implementation
- verify-task
- commit-task

## Summary
Create the worktree-management skill documentation (SKILL.md) with YAML frontmatter, worktree lifecycle phases, safety considerations, CLI command reference, and common workflow examples.

## Background
The worktree-management skill teaches Claude how to use the crewchief worktree CLI to manage parallel development workflows. This skill covers the complete worktree lifecycle from creation through merge/cleanup, with emphasis on safety and best practices.

This task implements the "Skill Documentation" phase from plan.md, building on the directory structure created in Phase 1.

## Acceptance Criteria
- [ ] File created at `.crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md`
- [ ] SKILL.md contains valid YAML frontmatter with name and description fields
- [ ] Frontmatter name is "worktree-management" (lowercase, hyphens)
- [ ] Frontmatter description is under 1024 characters
- [ ] Frontmatter description clearly states when to use this skill (parallel development, worktree lifecycle)
- [ ] Overview section explains worktree concept
- [ ] Decision tree section documents when to use worktree-management vs other git workflows
- [ ] Worktree lifecycle documented with all 5 phases: create -> use -> work -> merge -> clean
- [ ] Safety considerations section covers:
  - Cannot delete current worktree
  - Cannot merge from inside worktree
  - Unmerged branches require `git branch -D`
  - Check for uncommitted changes before merge
- [ ] All 6 CLI commands documented with correct syntax:
  - `crewchief worktree create <name> [--branch <base>] [--shell] [--no-copy-ignored]`
  - `crewchief worktree list`
  - `crewchief worktree use <name> [--shell]`
  - `crewchief worktree clean <selector> [--all] [--keep-branch] [--keep-maproom]`
  - `crewchief worktree merge <name> [--strategy <ff|squash|cherry-pick>] [--no-delete] [--dry-run]`
  - `crewchief worktree copy-ignored <selector> [--dry-run]`
- [ ] Common workflows documented with at least 3 examples:
  - Feature development (create -> work -> merge)
  - Quick experiment (create -> work -> clean)
  - Error recovery scenarios (handling merge conflicts)
- [ ] All instructions use imperative form (verb-first)
- [ ] Error handling section covers common failures
- [ ] No placeholder content remains

## Technical Requirements
- YAML frontmatter must be valid (triple-dash delimiters, proper key-value format)
- Description must trigger on worktree-related queries
- CLI command syntax must match actual crewchief implementation
- Examples must be copy-paste ready
- Markdown formatting must be consistent
- Use UTF-8 encoding

## Implementation Notes

### SKILL.md Frontmatter Template
```yaml
---
name: worktree-management
description: This skill should be used for managing git worktrees when users need to work on multiple branches simultaneously, create isolated environments for experiments, or safely merge and clean up parallel development work. Uses the crewchief worktree CLI.
---
```

### Worktree Lifecycle to Document
1. **Create** - `worktree create feature-x`
   - Creates new branch and checkout
   - Optionally copies ignored files (.env, etc.)

2. **Use** - `worktree use feature-x --shell`
   - Opens subshell in worktree directory
   - Type `exit` to return

3. **Work** - Make changes, commit normally
   - Worktree is a regular git checkout

4. **Merge** - `worktree merge feature-x`
   - Merges changes back to source branch
   - Cleans up worktree and branch

5. **Clean** - `worktree clean feature-x` (if merge not desired)
   - Removes worktree without merging
   - Use `--keep-branch` to preserve branch

### Safety Warnings to Document
- Never delete the current worktree (CLI prevents this)
- Check for uncommitted changes before merge
- Unmerged branches require `git branch -D` to force delete
- Cannot merge from inside the worktree being merged

### Common Workflows to Document

**Feature Development:**
```bash
crewchief worktree create feature-x
cd $(crewchief worktree use feature-x)
# ... make changes, commit ...
crewchief worktree merge feature-x
```

**Quick Experiment:**
```bash
crewchief worktree create experiment --shell
# ... try things out ...
exit
crewchief worktree clean experiment
```

**Preserve Work After Clean:**
```bash
crewchief worktree clean experiment --keep-branch
# Branch preserved, can create new worktree later
crewchief worktree create experiment --branch experiment
```

**Handling Merge Conflicts:**
```bash
crewchief worktree merge feature-x
# If merge conflicts occur:
# 1. Fix conflicts in source branch
# 2. Complete merge manually: git merge --continue
# 3. Clean up worktree: crewchief worktree clean feature-x
```

### CLI Commands Reference
Link to authoritative source: `/packages/cli/src/cli/worktree.ts` in crewchief repository

### Structure Outline
1. YAML Frontmatter
2. Overview
3. Decision Tree (when to use worktree-management)
4. Worktree Lifecycle
5. Safety Considerations
6. CLI Command Reference
7. Common Workflows
8. Error Handling

Estimated length: 200-250 lines

## Dependencies
- PLUGIN-002.1001 (directory structure must exist)
- External: crewchief CLI documentation for command syntax verification

## Risk Assessment
- **Risk**: CLI commands change between documentation and implementation
  - **Mitigation**: Link to CLI source as authoritative reference, verify commands against actual CLI
- **Risk**: Skill description doesn't trigger on worktree queries
  - **Mitigation**: Test description with various worktree-related queries during verification
- **Risk**: Description exceeds 1024 character limit
  - **Mitigation**: Keep description focused on trigger conditions, details go in body
- **Risk**: Lifecycle phases incomplete or incorrect sequence
  - **Mitigation**: Follow plan.md specification exactly, verify all 5 phases covered

## Files/Packages Affected
- `.crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md` (new)

## Deliverables Produced

Documents created in `skills/worktree-management/` directory:

- SKILL.md - Complete worktree-management skill with frontmatter, lifecycle phases, safety section, CLI command reference, workflow examples, and error handling

## Verification Notes

The verify-task agent should:
1. Validate YAML frontmatter syntax (no parsing errors)
2. Check frontmatter name is "worktree-management"
3. Verify frontmatter description is under 1024 characters
4. Confirm all 5 lifecycle phases are documented
5. Verify safety section covers all 4 safety considerations
6. Check all 6 CLI commands are documented with correct syntax
7. Confirm at least 3 common workflows are documented
8. Verify instructions use imperative form (verb-first)
9. Test that no placeholder content remains
10. Validate CLI command syntax against crewchief CLI source if possible

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-17 | verify-task | PASS | All 14 acceptance criteria met, documentation complete with valid YAML frontmatter, 5 lifecycle phases, 4 safety items, 6 CLI commands, 5 workflows, error handling section |
