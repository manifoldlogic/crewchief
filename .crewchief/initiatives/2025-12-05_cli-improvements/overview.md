# Initiative: cli improvements

Created: 2025-12-05

## Vision Statement

Improve CrewChief CLI developer experience by making worktree locations configurable, eliminating unintended auto-scan behavior, providing comprehensive cleanup, and enabling flexible maproom binary configuration for local development workflows.

## Conceptual Frame

The CLI currently makes several assumptions that create friction for developers:
- Worktrees are stored in `.crewchief/worktrees` within the repo, cluttering the workspace
- `worktree use` automatically triggers maproom scans without user control
- `worktree clean` only removes the directory and git metadata, leaving orphaned maproom records
- Maproom binary path defaults to local build, causing confusion when switching between development and production

These defaults were chosen for initial convenience but now impede professional workflows where developers need predictable, controllable behavior and clean separation between repository workspace and worktree storage.

## Domain Coherence

**Core Domain Concepts:**
- Worktree lifecycle: create → use → clean (with proper cleanup of all related resources)
- Configuration hierarchy: system defaults → project config → local overrides
- Binary resolution: environment variable → config file → fallback search
- Maproom integration: explicit opt-in for automatic indexing

## Directional Clarity

**Desired End State:**
"When this initiative succeeds, developers can store worktrees outside their repository, control when indexing happens, completely clean up worktrees with one command, and easily point to custom maproom binaries during development."

**Success Signals:**
- [x] Research complete
- [ ] `worktreeBasePath` supports `~` expansion and absolute paths
- [ ] `worktree use` no longer auto-scans by default
- [ ] `worktree clean` deletes directory, branch, and maproom record
- [ ] Config setting controls maproom binary path
- [ ] Config setting controls auto-scan behavior
- [ ] Documentation updated with new defaults and migration guide

## Scope Boundaries

**In Scope:**
- Config schema changes for `worktreeBasePath` with path expansion
- Config schema addition for `maproomBinaryPath`
- Config schema addition for `autoScanOnWorktreeUse` (default: false)
- Modify `worktree use` to respect auto-scan config
- Enhance `worktree clean` to call maproom cleanup
- Default `worktreeBasePath` change to `~/.crewchief/worktrees/<repo-name>/`
- Update CLI README and config example

**Out of Scope:**
- Migrating existing worktrees (users can do manually)
- Changing worktree creation logic beyond path resolution
- Modifying maproom indexing behavior itself
- Adding new worktree management commands
- VSCode extension or MCP changes

## Derived Projects

### 1. Configurable Worktree Paths
**Estimated Scope:** Small (1-2 days)
- Update config schema to support path expansion
- Implement `~` and environment variable expansion
- Change default from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>/`
- Update documentation

### 2. Worktree Use Auto-Scan Control
**Estimated Scope:** Small (1-2 days)
- Add `autoScanOnWorktreeUse` config option (default: false)
- Modify `worktree use` command to check config before scanning
- Remove auto-scan behavior when not enabled
- Update tests and documentation

### 3. Enhanced Worktree Clean
**Estimated Scope:** Medium (2-3 days)
- Detect if maproom binary is available
- Call `delete_worktree_data` via daemon client or direct CLI call
- Handle cases where worktree record doesn't exist
- Delete git branch after removing worktree
- Update tests for new cleanup flow

### 4. Maproom Binary Configuration
**Estimated Scope:** Small (1 day)
- Add `maproomBinaryPath` config option
- Update binary resolution logic to check config before fallback search
- Remove local bin default preference
- Document development workflow with custom binary

## Status

- [x] Research complete
- [x] Analysis complete
- [x] Decomposition complete
- [ ] Projects created

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking change for existing users | Medium | Provide migration guide, maintain backward compatibility where possible |
| Path expansion edge cases (Windows, symlinks) | Low | Use Node.js path resolution utilities, test on multiple platforms |
| Maproom cleanup failure leaves inconsistent state | Medium | Make cleanup best-effort with clear error messages, don't fail entire clean operation |
| Config complexity increases | Low | Provide good defaults, comprehensive documentation, and validation |

## Migration Notes

Users with existing worktrees in `.crewchief/worktrees` will continue to work. New worktrees will use the new default location. To migrate:
1. Move existing worktrees to `~/.crewchief/worktrees/<repo-name>/`
2. Update git worktree registration: `git worktree repair`
3. Or set `worktreeBasePath` in config to keep old location
