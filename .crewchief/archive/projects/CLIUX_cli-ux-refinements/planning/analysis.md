# Analysis: CLI UX Refinements

## Problem Definition

The CrewChief CLI has grown organically and needs refinement in three areas:

1. **Worktree Command Behavior**: The `worktree use` command conflates two distinct operations:
   - Using an existing worktree
   - Creating a new worktree if it doesn't exist

2. **Subshell Default Behavior**: Both `worktree create` and `worktree use` spawn an interactive subshell by default, which:
   - Requires users to type `exit` to return
   - Creates shell nesting that can confuse users
   - May not be needed when users just want to switch directories in scripts

3. **Command Organization**: The `spawn` command exists at the top level, but logically belongs under `agent`:
   - `crewchief spawn claude` → should be `crewchief agent spawn claude`
   - Better grouping of agent-related commands

## Current Behavior Analysis

### `worktree use <name>` (packages/cli/src/cli/worktree.ts:172-257)

Current behavior:
1. Search for existing worktree by name/branch/path
2. If NOT found → create it automatically
3. If found → use it
4. Default action: spawn interactive subshell (`spawn(shell, { stdio: 'inherit', cwd: targetPath })`)
5. `--print` flag outputs path instead of spawning subshell

**Problem**: Users expecting `use` to only work with existing worktrees are surprised when it creates one. The "create-if-not-exists" semantic is convenient but violates principle of least surprise.

### `worktree create <name>` (packages/cli/src/cli/worktree.ts:20-62)

Current behavior:
1. Creates worktree via `WorktreeService.createWorktree()`
2. Default: spawns subshell
3. `--no-cd` flag skips subshell

### `spawn` Command (packages/cli/src/cli/spawn.ts)

Current behavior:
1. Top-level command: `crewchief spawn <agents> [task]`
2. Creates worktree for agent
3. Opens terminal pane
4. Runs agent command in pane

**Problem**: Semantically, spawning agents is an agent operation. Having it at root level makes the CLI surface area larger than needed.

### `agent` Commands (packages/cli/src/cli/agent.ts)

Current subcommands:
- `agent message <pattern> [message]` - Send messages to agents
- `agent list` - List running agents
- `agent close <agentId>` - Close an agent

Missing: The `spawn` functionality that creates agents.

## User Experience Goals

1. **Explicit Operations**:
   - `worktree use` should only switch to existing worktrees
   - `worktree create` should be the only way to create worktrees

2. **Non-invasive Defaults**:
   - Default behavior should be `cd` (print path for scripting)
   - Subshell should be opt-in via `--shell` flag

3. **Coherent Command Grouping**:
   - All agent operations under `agent` subcommand
   - `agent spawn` instead of top-level `spawn`

## Impact Assessment

### Breaking Changes

| Change | Impact | Mitigation |
|--------|--------|------------|
| `worktree use` no longer creates | Scripts expecting auto-create will fail | Clear error message with suggestion |
| Subshell no longer default | Scripts expecting subshell will get path output | `--shell` flag for explicit subshell |
| `spawn` → `agent spawn` | Existing scripts/docs will break | Deprecation alias (optional) |

### Affected Files

1. `packages/cli/src/cli/worktree.ts` - Major changes
2. `packages/cli/src/cli/spawn.ts` - Move/refactor
3. `packages/cli/src/cli/agent.ts` - Add spawn subcommand
4. `packages/cli/src/cli/index.ts` - Remove spawn registration

### Related Help Text

All command help text needs updating:
- `crewchief --help`
- `crewchief worktree --help`
- `crewchief worktree use --help`
- `crewchief worktree create --help`
- `crewchief agent --help`
- `crewchief agent spawn --help`

## Research: Shell CD Patterns

For the "just cd to worktree" behavior, there are two approaches:

### Option A: Print Path (Current `--print` behavior)
```bash
cd $(crewchief worktree use feature-x)
```
- Works in any shell
- Requires wrapping in `$()` or backticks
- Clean, Unix-y approach

### Option B: Shell Function Export
Some CLIs provide shell integration:
```bash
eval "$(crewchief shell-init bash)"
# Then: cw feature-x  (alias that cd's)
```
- More convenient but requires setup
- Not in scope for this project

**Decision**: Use Option A (print path as default) - simple, universal, no setup.

## Summary of Required Changes

1. **Modify `worktree use`**:
   - Remove auto-create behavior
   - Add clear error message when worktree not found
   - Change default from subshell to print path
   - Add `--shell` flag for explicit subshell
   - Keep `--print` as alias for backwards compatibility

2. **Modify `worktree create`**:
   - Change default from subshell to print path
   - Add `--shell` flag for explicit subshell
   - Rename `--no-cd` to be consistent (deprecate or remove)

3. **Move `spawn` to `agent spawn`**:
   - Move spawn logic into agent.ts
   - Update index.ts to remove top-level registration
   - Optionally add deprecation alias at top level

4. **Update all help text**:
   - Reflect new behaviors
   - Clear examples
   - Note breaking changes in appropriate places
