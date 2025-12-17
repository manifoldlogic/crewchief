---
name: worktree-management
description: This skill should be used for managing git worktrees when users need to work on multiple branches simultaneously, create isolated environments for experiments, or safely merge and clean up parallel development work. Uses the crewchief worktree CLI.
---

# Worktree Management Skill

## Overview

Git worktrees enable working on multiple branches simultaneously by creating separate working directories that share the same repository. The crewchief worktree CLI streamlines this workflow with automated branch creation, environment setup (copying ignored files like `.env`), safe merging, and cleanup operations.

Unlike traditional branch switching (`git checkout`), worktrees provide:
- Isolated directories for each branch (no switching overhead)
- Parallel development without conflicts
- Preserved build artifacts and dependencies per worktree
- Safe experimentation without affecting the main working directory

## Decision Tree

### Use worktree-management when:
- Working on multiple features simultaneously
- Running long-running processes (servers, tests) on different branches
- Experimenting with changes without affecting main work
- Code review that requires checking out a feature branch
- Comparing implementations across branches side-by-side
- Testing changes that require clean builds or different dependencies

### Use standard git workflow when:
- Working on a single branch at a time
- Simple branch switching is sufficient
- Disk space is limited (worktrees duplicate working directories)
- Your workflow doesn't benefit from parallel checkouts

## Worktree Lifecycle

The worktree workflow consists of five phases:

### 1. Create
Create a new worktree with a dedicated branch:

```bash
crewchief worktree create feature-x
```

This operation:
- Creates a new branch from the base branch (default: main)
- Creates a working directory for the branch
- Optionally copies ignored files (`.env`, `node_modules/.bin`, etc.) based on config
- Returns the path to the new worktree

### 2. Use
Switch to an existing worktree:

```bash
# Print path (for scripting)
cd $(crewchief worktree use feature-x)

# Open interactive subshell
crewchief worktree use feature-x --shell
```

The `--shell` option spawns an interactive shell in the worktree directory. Type `exit` to return to your original location.

### 3. Work
Make changes and commit normally within the worktree:

```bash
# Worktree is a regular git checkout
git add .
git commit -m "Implement feature"
git push origin feature-x
```

Worktrees function as standard git working directories with full git command support.

### 4. Merge
Merge changes back to the source branch and clean up:

```bash
crewchief worktree merge feature-x
```

This operation:
- Verifies no uncommitted changes exist
- Displays merge statistics (commits, files changed, insertions/deletions)
- Prompts for confirmation
- Copies ignored files back to source (if configured)
- Merges using the specified strategy (default: fast-forward)
- Removes the worktree directory
- Deletes the feature branch

**Important**: You must run this command from outside the worktree being merged.

### 5. Clean
Remove a worktree without merging:

```bash
# Remove worktree and branch
crewchief worktree clean feature-x

# Keep branch, remove worktree only
crewchief worktree clean feature-x --keep-branch

# Remove all worktrees except current
crewchief worktree clean --all
```

Use `--keep-branch` to preserve work for later use. The worktree directory is removed, but the branch remains available for future checkout.

## Safety Considerations

### Cannot Delete Current Worktree
The CLI prevents removing the worktree you're currently working in:

```
Error: Refusing to remove the current working tree.
       Switch to another directory and try again.
```

**Solution**: Navigate to a different directory (typically the main repository) before running `clean`.

### Cannot Merge From Inside Worktree
Merging must be performed from outside the target worktree:

```
Error: Cannot merge a worktree while inside it.
       Please switch to the main repository first.
```

**Solution**: Exit the worktree (if using `--shell`, type `exit`), then run the merge command.

### Unmerged Branches Require Force Delete
Git protects unmerged work by refusing to delete branches that haven't been merged:

```
Branch feature-x not fully merged - skipped deletion
To delete anyway (CAUTION: may lose work):
  git branch -D feature-x
Or merge the branch first:
  git checkout main && git merge feature-x
```

**Solution**: Either merge the branch first, or use `git branch -D` to force delete if you're certain the work should be discarded.

### Check for Uncommitted Changes Before Merge
The merge operation fails if the working tree has uncommitted changes:

```
Error: Working tree has uncommitted changes.
       Commit or stash them before merging.
```

**Solution**: Commit or stash changes in your current working directory before merging.

## CLI Command Reference

### Create Worktree
```bash
crewchief worktree create <name> [options]

Options:
  --branch <base>        Base branch to create from (default: main)
  --shell                Start interactive subshell after creating
  --no-copy-ignored      Skip copying ignored files (override config)
```

Examples:
```bash
# Create and print path
cd $(crewchief worktree create feature-x)

# Create and open subshell
crewchief worktree create feature-x --shell

# Create from specific branch
cd $(crewchief worktree create hotfix --branch release-1.0)
```

### List Worktrees
```bash
crewchief worktree list
```

Displays all active worktrees with their paths, branches, and agent status (if applicable).

### Use Worktree
```bash
crewchief worktree use <name> [options]

Options:
  --shell                Start interactive subshell in worktree
  -p, --print            Print absolute path (default behavior)
```

Examples:
```bash
# Switch to worktree (prints path)
cd $(crewchief worktree use feature-x)

# Open worktree in subshell
crewchief worktree use feature-x --shell

# Use in scripts
path=$(crewchief worktree use my-branch)
code "$path"
```

### Clean Worktree
```bash
crewchief worktree clean [selector] [options]

Options:
  --all                  Remove all non-current worktrees
  --keep-branch          Keep git branch after removing worktree
  --keep-maproom         Skip maproom database cleanup
```

Examples:
```bash
# Remove specific worktree
crewchief worktree clean feature-x

# Keep branch for later use
crewchief worktree clean feature-x --keep-branch

# Remove all worktrees except current
crewchief worktree clean --all
```

### Merge Worktree
```bash
crewchief worktree merge <name> [options]

Options:
  --strategy <type>      Merge strategy: ff, squash, cherry-pick (default: ff)
  --no-delete            Keep worktree after merging
  --dry-run              Show what would be done without making changes
  --no-copy-ignored      Skip copying ignored files back to source
  --message <msg>        Custom commit message
  -y, --yes              Skip confirmation prompts
```

Examples:
```bash
# Standard merge with fast-forward
crewchief worktree merge feature-x

# Squash commits into single commit
crewchief worktree merge feature-x --strategy squash

# Preview merge without executing
crewchief worktree merge feature-x --dry-run

# Merge but keep worktree
crewchief worktree merge feature-x --no-delete
```

### Copy Ignored Files
```bash
crewchief worktree copy-ignored <selector> [options]

Options:
  --dry-run              Show what would be copied without copying
  --no-copy-ignored      Override config and skip copying
```

Manually copy ignored files to a worktree based on `worktree.copyIgnoredFiles` config. This is automatically called during `create` unless `--no-copy-ignored` is specified.

## Common Workflows

### Feature Development
Complete lifecycle from creation to merge:

```bash
# Create feature worktree
crewchief worktree create feature-auth --shell

# Work in the worktree
git add src/auth.ts
git commit -m "Add authentication module"
git push origin feature-auth

# Exit subshell
exit

# Merge back to main
crewchief worktree merge feature-auth
```

This workflow:
1. Creates isolated environment for feature work
2. Preserves main branch stability while developing
3. Merges and cleans up automatically

### Quick Experiment
Try something without committing to it:

```bash
# Create experimental worktree
crewchief worktree create experiment --shell

# Try changes
npm install experimental-package
# ... test it out ...

# Exit and discard
exit
crewchief worktree clean experiment
```

Use this for:
- Testing new dependencies
- Exploring refactoring ideas
- Reproducing bugs in isolation

### Parallel Development
Work on multiple features simultaneously:

```bash
# Create multiple worktrees
crewchief worktree create feature-a
crewchief worktree create feature-b

# Terminal 1: Work on feature A
cd $(crewchief worktree use feature-a)
npm run dev

# Terminal 2: Work on feature B
cd $(crewchief worktree use feature-b)
npm test

# Merge when ready
crewchief worktree merge feature-a
crewchief worktree merge feature-b
```

Benefits:
- Run dev server on one branch while coding another
- Compare implementations side-by-side
- Avoid context switching overhead

### Code Review Workflow
Check out a PR for review without disrupting current work:

```bash
# Create worktree from PR branch
crewchief worktree create review-pr-123 --branch feature-branch --shell

# Review, test, leave comments
npm test
npm run lint

# Clean up after review
exit
crewchief worktree clean review-pr-123
```

### Preserve Work After Clean
Keep the branch but remove the worktree:

```bash
# Clean worktree but preserve branch
crewchief worktree clean experiment --keep-branch

# Later, recreate worktree from existing branch
crewchief worktree create experiment --branch experiment
```

Use when:
- Freeing disk space but want to continue later
- Archiving work-in-progress branches
- Switching between active features

## Error Handling

### Worktree Not Found
```
Error: Worktree 'feature-x' not found.
       Create it with: crewchief worktree create feature-x
```

**Solution**: The worktree doesn't exist. Create it first or check the name with `crewchief worktree list`.

### Ambiguous Selector
```
Error: Ambiguous selector 'feat'. Candidates:
       /path/to/feature-a [feature-a]
       /path/to/feature-b [feature-b]
```

**Solution**: Use a more specific selector (full branch name or unique path component).

### No Commits to Merge
```
Warning: No commits to merge from this worktree
         Do you want to remove this worktree anyway?
```

**Solution**: This occurs when the worktree branch has no commits beyond the base branch. Confirm cleanup or cancel if commits are expected.

### Merge Conflicts
```
Error: Merge failed: Merge conflict in src/app.ts
```

**Solution**: Merge conflicts require manual resolution:
1. Resolve conflicts in the current branch
2. Complete the merge: `git merge --continue`
3. Clean up the worktree: `crewchief worktree clean feature-x --keep-branch`

### Maproom Cleanup Failures
```
Warning: Maproom binary not found - database cleanup skipped
         Install maproom or run: crewchief-maproom db cleanup-stale --confirm
```

**Solution**: Maproom cleanup is best-effort. If the binary isn't available, clean up manually:
```bash
crewchief-maproom db cleanup-stale --confirm
```

Common maproom cleanup issues:
- **Binary not found**: Install maproom or skip cleanup with `--keep-maproom`
- **Database locked**: Wait for other operations to complete, then run cleanup manually
- **Permission denied**: Check file permissions on maproom database

### Branch Already Exists
```
Error: Branch 'feature-x' already exists
```

**Solution**: Either:
- Use the existing branch: `crewchief worktree create feature-x --branch feature-x`
- Delete the existing branch: `git branch -d feature-x`
- Choose a different name

## Reference

For CLI implementation details and advanced usage:
- Command source: `packages/cli/src/cli/worktree.ts`
- Configuration: `crewchief.config.js` (`worktree.copyIgnoredFiles`)
- Help text: `crewchief worktree --help`
