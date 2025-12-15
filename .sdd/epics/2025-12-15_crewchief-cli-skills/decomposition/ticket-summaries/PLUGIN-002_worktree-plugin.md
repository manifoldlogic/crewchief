# Ticket: Create Worktree Plugin

**Ticket ID:** PLUGIN-002
**Priority:** 2 (High)
**Effort:** M (2-3 days)

## Summary

Create the `worktree` plugin for the crewchief marketplace at `.crewchief/claude-code-plugins/plugins/worktree/`. The plugin provides git worktree management capabilities via the crewchief CLI, documenting the worktree lifecycle (create -> use -> merge -> clean) with safety guidance.

## Deliverables

1. **Plugin Structure:**
   ```
   .crewchief/claude-code-plugins/plugins/worktree/
   ├── .claude-plugin/
   │   └── plugin.json
   ├── README.md
   └── skills/
       └── worktree-management/
           └── SKILL.md
   ```

2. **plugin.json** with:
   - name: "worktree"
   - version: "0.1.0"
   - description for plugin discovery
   - author and repository info
   - keywords for discoverability

3. **README.md** with:
   - Plugin description
   - Installation instructions
   - Feature list
   - Usage examples
   - Prerequisites (git repository)

4. **skills/worktree-management/SKILL.md** with:
   - YAML frontmatter (name: worktree-management, description for skill discovery)
   - Worktree lifecycle documentation
   - CLI command reference (create, list, use, clean, merge)
   - Safety checks and error handling patterns
   - Common workflow examples

## Dependencies

- None (can be executed in parallel with PLUGIN-001)

## Value Proposition

Makes git worktrees accessible through Claude, enabling parallel development workflows. Users can safely create worktrees for experiments, merge changes back, and clean up without risking data loss or confusion from improper sequencing. Plugin architecture allows users to install only if they use worktrees.

## Acceptance Criteria

- [ ] Plugin directory structure matches specification
- [ ] plugin.json has valid name, version, description
- [ ] README.md documents installation and usage
- [ ] SKILL.md follows Claude Code skill format
- [ ] SKILL.md frontmatter has valid name (lowercase, hyphens) and description (<1024 chars)
- [ ] SKILL.md description clearly states when to use this skill
- [ ] Worktree lifecycle documented (create -> use -> merge -> clean)
- [ ] All CLI commands documented with examples
- [ ] Safety checks documented (don't delete current worktree, check for uncommitted changes)
- [ ] Common workflows have step-by-step examples

## Technical Notes

### plugin.json

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
    "parallel-development"
  ]
}
```

### SKILL.md Frontmatter

```yaml
---
name: worktree-management
description: This skill should be used for managing git worktrees when users need to work on multiple branches simultaneously, create isolated environments for experiments, or safely merge and clean up parallel development work. Uses the crewchief worktree CLI.
---
```

### CLI Commands to Document

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

### Worktree Lifecycle

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

### Safety Warnings

- Never delete the current worktree (CLI prevents this)
- Check for uncommitted changes before merge
- Unmerged branches require `git branch -D` to force delete

### Common Workflows

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

## Reference Documentation

- `/workspace/packages/cli/CLAUDE.md` - CLI package overview
- `/workspace/packages/cli/src/cli/worktree.ts` - Command implementations
- `/workspace/packages/cli/src/git/worktrees.ts` - Worktree service
- `/workspace/.crewchief/claude-code-plugins/plugins/github-actions/skills/gh-cli/SKILL.md` - Pattern example
