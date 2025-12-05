# Project: Configurable Worktree Paths

**Slug:** WTPATH_configurable-worktree-paths
**Priority:** High
**Effort:** S (1-2 days)

## Summary

Change the default worktree storage location from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>/` with full support for path expansion (`~` and environment variables) and custom configuration via config file.

## Deliverables

1. **Path expansion utility** - Function to expand `~` and environment variables in paths
2. **Config schema update** - Change default `worktreeBasePath` to `~/.crewchief/worktrees/<repo-name>/`
3. **Repository name detection** - Extract repo name from git config for path interpolation
4. **Path resolution in worktree service** - Apply expansion when creating worktrees
5. **Updated tests** - Verify path expansion and default behavior
6. **Documentation** - Update README and config example with new default and migration guide

## Dependencies

None - This is a foundational project that others may depend on.

## Value Proposition

Developers can store worktrees outside the repository, reducing workspace clutter and improving IDE performance. The repository-specific subdirectory prevents naming conflicts when working on multiple repos. Existing users can maintain current location via config override.

## Technical Approach

1. Add `expandPath()` utility function using Node.js `os.homedir()` and `path.resolve()`
2. Detect repo name from `git config remote.origin.url` or fallback to directory name
3. Update `RepositorySchema` default to include `<repo-name>` placeholder
4. Apply expansion in `WorktreeService.createWorktree()` before calling git commands
5. Validate paths work cross-platform (Windows, macOS, Linux)

## Acceptance Criteria

- [ ] Config accepts paths like `~/.crewchief/worktrees/<repo-name>/`
- [ ] `~` expands to user home directory on all platforms
- [ ] `<repo-name>` is replaced with actual repository name
- [ ] Existing `.crewchief/worktrees` path still works via config
- [ ] Tests pass on Windows, macOS, Linux
- [ ] Documentation includes migration guide

## Breaking Changes

**Breaking:** Default worktree location changes from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>/`

**Migration:** Users can maintain old behavior by setting `worktreeBasePath: '.crewchief/worktrees'` in config.
