# Plan: Worktree Plugin

## Overview

This document outlines the execution plan for creating the worktree plugin. The work is organized into two phases: Foundation (plugin structure and metadata) and Content (skill documentation). Each phase produces testable deliverables.

## Phases

### Phase 1: Plugin Foundation

**Objective:** Create the plugin directory structure with required metadata files.

**Deliverables:**
- Plugin directory at `.crewchief/claude-code-plugins/plugins/worktree/`
- `.claude-plugin/plugin.json` with name, version, description, author, repository, keywords
- `README.md` with installation instructions, features, prerequisites, usage examples, troubleshooting

**Agent Assignments:**
- General implementation agent: Create directory structure and write metadata files

**Acceptance Criteria:**
- [ ] Directory structure matches specification:
  ```
  plugins/worktree/
  ├── .claude-plugin/
  │   └── plugin.json
  ├── README.md
  └── skills/
      └── worktree-management/
          └── SKILL.md  (placeholder for Phase 2)
  ```
- [ ] plugin.json validates as JSON with required fields (name, version, description, author, repository, keywords)
- [ ] plugin.json name is "worktree", version is "0.1.0"
- [ ] README.md includes all sections: Introduction, Features, Prerequisites, Installation, Usage Examples, Troubleshooting
- [ ] No placeholder content remains in Phase 1 deliverables

**Estimated Effort:** 1-2 hours

### Phase 2: Skill Documentation

**Objective:** Create the worktree-management skill with comprehensive lifecycle documentation.

**Deliverables:**
- `skills/worktree-management/SKILL.md` with:
  - Valid YAML frontmatter (name: worktree-management, description <1024 chars)
  - Overview section
  - Decision tree section (when to use worktree-management vs other git workflows)
  - Worktree lifecycle phases (create -> use -> work -> merge -> clean)
  - Safety considerations section
  - CLI command reference for all 6 commands
  - Common workflow examples
  - Error handling guidance

**Agent Assignments:**
- General implementation agent: Write skill documentation

**Acceptance Criteria:**
- [ ] SKILL.md frontmatter has valid name "worktree-management" (lowercase, hyphens)
- [ ] SKILL.md description is under 1024 characters
- [ ] SKILL.md description clearly states when to use this skill (parallel development, worktree lifecycle)
- [ ] Worktree lifecycle documented with all 5 phases
- [ ] Safety section covers:
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
- [ ] Common workflows documented:
  - Feature development (create -> work -> merge)
  - Quick experiment (create -> work -> clean)
  - Merge strategies comparison
  - Error recovery scenarios (e.g., handling merge conflicts)
- [ ] Error handling covers common failures
- [ ] All instructions use imperative form (verb-first)

**Estimated Effort:** 2-3 hours

## Dependencies

### External Dependencies

| Dependency | Description | Mitigation |
|------------|-------------|------------|
| crewchief CLI | Must be installed for commands to work | Document as prerequisite in README |
| Git repository | Commands only work in git repos | Document as prerequisite |
| Git submodule | Marketplace is a submodule | Ensure submodule is initialized |

### Internal Dependencies

| Phase | Depends On |
|-------|------------|
| Phase 2 | Phase 1 (directory structure must exist) |

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| CLI commands change | Low | Medium | Link to CLI source as authoritative reference |
| Plugin schema changes | Low | High | Follow existing maproom plugin pattern exactly |
| Description too long | Low | Low | Keep under 1024 chars, focus on trigger conditions |
| Skill doesn't activate | Medium | Medium | Test description with various worktree-related queries |

## Success Metrics

### Completion Criteria

- [ ] All 3 files created in correct locations
- [ ] plugin.json is valid JSON
- [ ] README.md is complete
- [ ] SKILL.md frontmatter is valid
- [ ] Worktree lifecycle is documented
- [ ] All 6 CLI commands are documented
- [ ] Safety considerations are covered
- [ ] Common workflows have step-by-step examples

### Quality Criteria

- [ ] Instructions use imperative form throughout
- [ ] CLI examples are copy-paste ready
- [ ] No placeholder content
- [ ] Consistent formatting with maproom plugin
- [ ] Safety warnings are prominent

### Testing Checklist

- [ ] Plugin can be installed: `/plugin install worktree@crewchief`
- [ ] Skill appears in installed skills
- [ ] Query "How do I create a worktree?" triggers skill activation
- [ ] CLI commands work when executed
- [ ] Worktree lifecycle is complete and correct

## File Manifest

Files to be created:

```
.crewchief/claude-code-plugins/plugins/worktree/
├── .claude-plugin/
│   └── plugin.json                              # ~20 lines
├── README.md                                    # ~100 lines
└── skills/
    └── worktree-management/
        └── SKILL.md                             # ~200-250 lines
```

Total new files: 3
Total lines: ~320-370

## Implementation Notes

### plugin.json Template

```json
{
  "name": "worktree",
  "version": "0.1.0",
  "description": "Git worktree management using the crewchief CLI. Create, manage, and merge parallel development branches safely.",
  "author": {
    "name": "Daniel Bushman",
    "email": "dbushman@manifoldlogic.com",
    "url": "https://github.com/manifoldlogic/claude-code-plugins"
  },
  "repository": "https://github.com/manifoldlogic/claude-code-plugins",
  "keywords": [
    "git",
    "worktree",
    "branches",
    "parallel-development",
    "parallel",
    "isolation"
  ]
}
```

### SKILL.md Frontmatter Template

```yaml
---
name: worktree-management
description: This skill should be used for managing git worktrees when users need to work on multiple branches simultaneously, create isolated environments for experiments, or safely merge and clean up parallel development work. Uses the crewchief worktree CLI.
---
```

### Key CLI Commands to Document

```bash
# Create a new worktree
crewchief worktree create <name> [--branch <base>] [--shell]

# List all worktrees
crewchief worktree list

# Switch to existing worktree
crewchief worktree use <name> [--shell]

# Clean up worktree
crewchief worktree clean <name> [--keep-branch] [--keep-maproom]

# Clean all non-current worktrees
crewchief worktree clean --all

# Merge worktree back to source
crewchief worktree merge <name> [--strategy <ff|squash|cherry-pick>] [--no-delete]

# Copy ignored files to worktree
crewchief worktree copy-ignored <name> [--dry-run]
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

## Post-Completion Steps

After implementation:

1. Run verification: Check all files exist and validate
2. Validate JSON: `jq . .crewchief/claude-code-plugins/plugins/worktree/.claude-plugin/plugin.json`
3. Check frontmatter: `head -10 .crewchief/claude-code-plugins/plugins/worktree/skills/worktree-management/SKILL.md`
4. Test installation: `/plugin install worktree@crewchief`
5. Test activation: Try worktree-related queries to verify skill triggers
6. Update marketplace.json (handled by PLUGIN-003 ticket)
