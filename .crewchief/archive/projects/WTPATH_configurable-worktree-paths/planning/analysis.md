# Analysis: Configurable Worktree Paths

## Problem Definition

CrewChief currently stores git worktrees at `.crewchief/worktrees/` relative to the repository root. This approach has several pain points:

**Workspace Clutter**: Every worktree created adds directories inside the repository workspace. For projects with many worktrees, this clutters the file tree in IDEs and editors.

**IDE Performance**: Many IDEs (VS Code, JetBrains) scan the entire workspace for indexing. Large numbers of worktrees inside `.crewchief/worktrees/` can slow down indexing, searches, and file watching.

**Disk Space Visibility**: Worktrees hidden inside `.crewchief/` are less visible for disk space monitoring and cleanup operations.

**Multi-Repo Conflicts**: When working on multiple repositories, each has its own `.crewchief/worktrees/` directory. There's no centralized location for all worktrees across projects.

## Context

This work is part of the CLI improvements initiative to make CrewChief more ergonomic for daily use. The current worktree storage approach was chosen for simplicity in the MVP, but user feedback indicates it creates friction in real-world workflows.

**User Pain Points**:
- "My IDE is slow because it's indexing all my worktrees"
- "I can't easily see how much disk space worktrees are using"
- "I want all my worktrees in one place, not scattered across repos"

## Existing Solutions

**Industry Patterns**:
- **Git**: Stores worktree metadata in `.git/worktrees/` but allows worktree directories anywhere
- **VS Code**: Uses `~/.config/Code/User/workspaceStorage/` for centralized per-workspace data
- **npm**: Uses `~/.npm/` for global packages across projects
- **Docker**: Uses `~/.docker/contexts/` with per-context subdirectories

**Pattern**: Tools managing project-related artifacts use `~/.tool-name/resource-type/<identifier>/` for centralized storage.

**Codebase Patterns**:
- Rust maproom already implements tilde expansion in `crates/maproom/src/db/connection.rs:25-39`
- Node.js provides `os.homedir()` for cross-platform home directory access

## Current State

**Config Schema** (`packages/cli/src/config/schema.ts:5`):
```typescript
worktreeBasePath: z.string().default('.crewchief/worktrees'),
```

**Worktree Creation** (`packages/cli/src/git/worktrees.ts:99`):
```typescript
async createWorktree(name: string, baseBranch: string, basePath: string, ...): Promise<string> {
  const wtPath = path.join(this.cwd, basePath, name)  // Always relative to cwd
  ensureDirSync(wtPath)
  // ...
}
```

**Key Limitation**: `basePath` is always joined with `this.cwd`, making all paths relative to the repository. No support for absolute paths or tilde expansion.

**Usage Sites**:
- `packages/cli/src/cli/worktree.ts:47` - User-initiated worktree creation
- `packages/cli/src/orchestrator/scheduler.ts:21` - Agent worktree creation
- All commands use `config.repository.worktreeBasePath`

## Research Findings

1. **Tilde Expansion is Standard**: Rust maproom already does it, users expect it to work
2. **Repository Isolation Needed**: Multiple repos would conflict in `~/.crewchief/worktrees/` without per-repo subdirectories
3. **Path Resolution Complexity**: Node.js `path.resolve()` handles absolute paths correctly - if given absolute path, returns it unchanged; if relative, resolves against cwd
4. **Git Remote Parsing**: Can extract repo name from `git config remote.origin.url` with regex
5. **Safety Checks Exist**: WorktreeService already uses `fs.realpathSync()` to prevent deleting current directory

## Constraints

**Technical**:
1. Must support Windows paths (backslashes, drive letters)
2. Must handle symlinks safely (existing code uses `realpathSync`)
3. Must work with existing git worktree commands
4. Config file is JavaScript (can compute paths at load time)

**Business**:
1. Breaking change to default - must communicate clearly
2. Existing users may rely on current location
3. Users may have scripts hardcoding `.crewchief/worktrees`

**User Experience**:
1. Zero-config should improve, not degrade
2. Migration must be opt-in or clearly communicated
3. Error messages must be helpful

## Success Criteria

1. Fresh installs use `~/.crewchief/worktrees/<repo-name>/` by default
2. Config values like `~/my-worktrees` expand correctly
3. Absolute paths like `/mnt/worktrees` work without joining with cwd
4. Existing users can set `worktreeBasePath: '.crewchief/worktrees'` to maintain old behavior
5. Works cross-platform (Windows, macOS, Linux)
6. Multiple repos don't conflict in shared worktree directory
7. Invalid paths produce helpful error messages
8. All existing tests pass with path expansion
