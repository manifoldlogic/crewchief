# Ticket: [WTPATH-3002]: Documentation and Migration Guide

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
- docs-writer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation for the configurable worktree paths feature, including migration guide for users upgrading from the old default, troubleshooting guide, and examples of all supported path patterns.

## Background
With the config schema updated to the new default in WTPATH-3001, users need clear documentation to understand the breaking change, migration options, and new capabilities. This documentation will be the primary communication channel for the feature.

This ticket implements Phase 3 (Documentation) of the Configurable Worktree Paths project.

**Planning References:**
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/plan.md` (Phase 3)
- `/workspace/.crewchief/projects/WTPATH_configurable-worktree-paths/planning/project-review.md` (communication recommendations)

## Acceptance Criteria
- [ ] README includes migration section for users upgrading from old default
- [ ] Migration guide covers accepting new default OR reverting to old behavior
- [ ] Documentation explains all path features (tilde, placeholder, absolute, relative)
- [ ] Examples include: (1) default new behavior, (2) legacy opt-out, (3) custom SSD path, (4) shared team path, (5) home directory without repo-name
- [ ] Troubleshooting covers: (1) system directory rejection, (2) permission errors, (3) worktrees in two locations after upgrade, (4) special characters in repo name, (5) git remote detection failures
- [ ] Documentation explains repository name detection logic and fallback
- [ ] Repository rename behavior: documents that (1) old worktrees continue working, (2) new worktrees use new repo name, (3) paths are fixed at creation time, (4) this is expected behavior
- [ ] Documentation is clear, concise, and actionable
- [ ] Code examples use proper formatting and are copy-pasteable

## Technical Requirements

### Update CLI README
File: `/workspace/packages/cli/README.md`

Add new section "Worktree Configuration" with subsections:
1. **Path Patterns** - Explain supported patterns
2. **Migration from v1.x** - Breaking change notice and migration
3. **Examples** - Common configuration patterns
4. **Troubleshooting** - Common issues and solutions

### Migration Guide Content

**Section: Migration from v1.x**

```markdown
## Migration from v1.x

### Breaking Change: Default Worktree Location

**v1.x default**: `.crewchief/worktrees` (inside repository)
**v2.0+ default**: `~/.crewchief/worktrees/<repo-name>` (outside repository)

### Why This Change?

- **Repository isolation**: Multiple repositories can coexist without conflicts
- **Clean git status**: Worktrees outside repo don't clutter git status
- **Centralized management**: All worktrees in one location across projects
- **Developer experience**: Common convention used by other tools

### Migration Options

#### Option 1: Accept New Default (Recommended)

Do nothing. New worktrees will be created in `~/.crewchief/worktrees/<repo-name>/`.

**Existing worktrees continue to work** - git manages them by path, not location.

You'll have worktrees in two locations temporarily:
- Old: `.crewchief/worktrees/feature-old`
- New: `~/.crewchief/worktrees/myproject/feature-new`

You can manually migrate old worktrees or leave them in place.

#### Option 2: Revert to Old Behavior

Add to `crewchief.config.js`:

```javascript
export default {
  repository: {
    worktreeBasePath: '.crewchief/worktrees'
  }
}
```

All worktrees will continue to be created inside the repository.

#### Option 3: Custom Location

Use any path pattern:

```javascript
export default {
  repository: {
    // SSD for performance
    worktreeBasePath: '/mnt/ssd/worktrees/<repo-name>',

    // Shared team location
    worktreeBasePath: '/shared/worktrees/<repo-name>',

    // Home directory without repo name
    worktreeBasePath: '~/dev/worktrees',
  }
}
```

### Manual Worktree Migration

To move existing worktrees to the new location:

```bash
# 1. List existing worktrees
git worktree list

# 2. Remove old worktree
git worktree remove .crewchief/worktrees/feature-x

# 3. Create in new location
crewchief worktree create feature-x

# Note: This creates a fresh checkout. Local changes are lost.
# Commit changes before migration!
```

### Repository Rename Behavior

If you rename your repository or change the remote URL:
- Existing worktrees continue to work (git manages them)
- New worktrees use the new repository name
- This is expected behavior - worktree paths are fixed at creation time
```

### Path Patterns Documentation

```markdown
## Worktree Path Configuration

### Supported Path Patterns

#### Tilde Expansion

`~` expands to your home directory:

```javascript
worktreeBasePath: '~/worktrees'
// Linux/macOS: /home/username/worktrees
// Windows: C:\Users\username\worktrees
```

#### Repository Placeholder

`<repo-name>` is replaced with your repository name:

```javascript
worktreeBasePath: '~/.crewchief/worktrees/<repo-name>'
// Example: ~/.crewchief/worktrees/myproject
```

Repository name is extracted from:
1. Git remote URL (e.g., `git@github.com:org/myproject.git` → `myproject`)
2. Directory basename (fallback if not a git repo)

#### Absolute Paths

Full paths work without joining to repository root:

```javascript
worktreeBasePath: '/tmp/worktrees'
```

#### Relative Paths

Paths without `/` or `~` are relative to repository root:

```javascript
worktreeBasePath: '.crewchief/worktrees'  // Inside repo (legacy)
worktreeBasePath: '../worktrees'          // Adjacent to repo
```

### Examples

```javascript
// Default: per-repo isolation in home directory
worktreeBasePath: '~/.crewchief/worktrees/<repo-name>'

// Custom: fast SSD with repo isolation
worktreeBasePath: '/mnt/ssd/worktrees/<repo-name>'

// Custom: shared team location
worktreeBasePath: '/shared/dev/worktrees/<repo-name>'

// Legacy: inside repository (v1.x behavior)
worktreeBasePath: '.crewchief/worktrees'

// Simple: home directory, all repos together
worktreeBasePath: '~/worktrees'
```
```

### Troubleshooting Section

```markdown
## Troubleshooting

### "Invalid worktree path: system directory not allowed"

**Cause**: Configured path resolves to a system directory (/, /etc, /usr, /System, C:\Windows)

**Solution**: Use a non-system directory:
```javascript
// Bad
worktreeBasePath: '/etc'

// Good
worktreeBasePath: '~/worktrees'
worktreeBasePath: '/home/user/worktrees'
```

### "Could not expand <repo-name>: not in a git repository"

**Cause**: Used `<repo-name>` placeholder but not in a git repository

**Solution**:
- Run commands from inside a git repository
- Or use a path without `<repo-name>` placeholder
- The placeholder is replaced with directory name as fallback

### Permission Denied Creating Worktree

**Cause**: Configured path is not writable

**Solution**:
- Verify directory permissions: `ls -la ~/`
- Create directory manually: `mkdir -p ~/.crewchief/worktrees`
- Choose different location with write access

### Worktrees in Two Locations After Upgrade

**Cause**: Upgraded from v1.x with new default location

**Solution**: This is expected. See [Migration from v1.x](#migration-from-v1x) for options:
- Keep both locations (old worktrees still work)
- Revert to old behavior with config override
- Manually migrate worktrees to new location

### Repository Name Has Special Characters

**Cause**: Repository name contains /, \, :, *, ?, ", <, >, |

**Solution**: These characters are automatically replaced with `-`.
Example: `my/repo` becomes `my-repo` in path.
```

## Implementation Notes

### Documentation Style
- Clear headings and subsections
- Short paragraphs (2-4 sentences)
- Code examples with comments
- Concrete examples over abstract descriptions
- Common patterns first, edge cases later

### Markdown Formatting
- Use code blocks with language tags (```javascript, ```bash)
- Use **bold** for emphasis on important concepts
- Use bullet points for lists
- Use numbered lists for sequential steps

### Audience
Primary audience: Developers upgrading from v1.x who will see worktrees in new location

Secondary audience: New users learning about configuration options

### Tone
- Clear and direct
- Acknowledge breaking change upfront
- Emphasize continuity (old worktrees still work)
- Provide actionable solutions

## Dependencies
- **WTPATH-3001** (Config Schema Update) - Should be completed first or in parallel

## Risk Assessment
- **Risk**: Users miss documentation and are confused by new behavior
  - **Mitigation**: Place migration guide prominently in README; mention in release notes

- **Risk**: Documentation is too technical or unclear
  - **Mitigation**: Use concrete examples; test with fresh eyes during verification

- **Risk**: Troubleshooting doesn't cover actual user issues
  - **Mitigation**: Base on project-review.md identified edge cases; can expand based on feedback

## Files/Packages Affected
- `/workspace/packages/cli/README.md` (add sections)

## Verification Notes

**Tests pass - N/A** (documentation-only ticket)

Verify-ticket agent should check:
- [ ] All acceptance criteria checkboxes are met
- [ ] Migration guide is prominent and easy to find
- [ ] All three migration options clearly explained (accept new default, revert, custom)
- [ ] Code examples are syntactically correct and copy-pasteable
- [ ] Path patterns section explains all four types (tilde, placeholder, absolute, relative)
- [ ] Examples show diverse use cases
- [ ] Troubleshooting covers common issues from project-review.md
- [ ] Documentation explains repository name detection and fallback
- [ ] Repository rename behavior documented
- [ ] Markdown formatting is correct (no broken links, proper code blocks)
- [ ] Tone is helpful and acknowledges user impact of breaking change
- [ ] No typos or grammatical errors
