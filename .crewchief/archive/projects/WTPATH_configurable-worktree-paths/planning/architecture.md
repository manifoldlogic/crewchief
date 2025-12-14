# Architecture: Configurable Worktree Paths

## Overview

Change worktree path handling from **relative-only** to **flexible absolute/relative with template expansion**:

```
Current:  .crewchief/worktrees/${name}
Proposed: ~/.crewchief/worktrees/<repo-name>/${name}
```

**Key Principles**:
- MVP First: Support `~` expansion and `<repo-name>` placeholder only
- Backward Compatible: Relative paths still work
- Fail Fast: Invalid paths produce clear errors
- Consistent: Port Rust maproom's expansion logic to TypeScript

## Design Decisions

### Decision 1: Path Expansion Utility

**Context**: Need to expand `~` and `<repo-name>` in config paths before use

**Decision**: Create `packages/cli/src/utils/paths.ts` with incremental expansion functions

**Rationale**:
- Separates concerns (expansion vs worktree logic)
- Testable in isolation
- Reusable for future path configuration needs
- Mirrors existing Rust implementation pattern

### Decision 2: Placeholder Syntax

**Context**: Need syntax for repository name placeholder

**Decision**: Use `<repo-name>` syntax

**Rationale**:
- Clear and explicit
- Doesn't conflict with shell expansion (`$VAR`, `${VAR}`)
- Doesn't conflict with template literals (`${expr}`)
- Similar to existing patterns in configs (e.g., `<platform>` in VSCode)

### Decision 3: Repository Name Detection

**Context**: Need reliable way to get repository name

**Decision**: Try `git config remote.origin.url` first, fall back to directory basename

**Rationale**:
- Git remote URL is standard and reliable when present
- Directory basename is universally available fallback
- No need for user configuration in common case
- Consistent with how other tools identify projects

### Decision 4: Expansion Timing

**Context**: When to expand paths - config load or usage time?

**Decision**: Expand at usage time (worktree creation)

**Rationale**:
- Repository name may not be available at config load time
- Allows dynamic detection per worktree operation
- Simpler config schema (still just string)
- Matches user mental model (expansion happens when path is used)

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Tilde Expansion | `os.homedir()` | Cross-platform, built-in Node.js API |
| Path Resolution | `path.resolve()` | Handles absolute vs relative correctly |
| Git Operations | `simple-git` | Already in use, provides remote parsing |
| Validation | String checks + error throwing | Simple, fail-fast approach |

## Component Design

### Path Expansion Utility (`packages/cli/src/utils/paths.ts`)

**Responsibilities**:
- Expand `~` to home directory
- Extract repository name from git or directory
- Replace `<repo-name>` placeholder
- Validate expanded paths
- Resolve to absolute paths

**Interface**:
```typescript
export function expandTilde(pathStr: string): string
export async function getRepositoryName(cwd?: string): Promise<string>
export async function expandRepoPlaceholder(pathStr: string, cwd?: string): Promise<string>
export async function expandWorktreePath(pathStr: string, cwd?: string): Promise<string>
```

**Design**:
- Incremental: Each function does one thing
- Composable: Full expansion chains smaller functions
- Async where needed: Repository detection requires git commands
- Validated: Throws on invalid input with helpful messages

### WorktreeService Integration

**Change**: Use `expandWorktreePath()` before constructing final path

**Before**:
```typescript
const wtPath = path.join(this.cwd, basePath, name)
```

**After**:
```typescript
const expandedBasePath = await expandWorktreePath(basePath, this.cwd)
const wtPath = path.join(expandedBasePath, name)
```

**Key Insight**: Remove implicit `path.join(this.cwd, ...)`. Let expansion utility handle absolute vs relative resolution via `path.resolve()`.

### Config Schema Update

**Change**: Update default in `packages/cli/src/config/schema.ts`

**Before**:
```typescript
worktreeBasePath: z.string().default('.crewchief/worktrees'),
```

**After**:
```typescript
worktreeBasePath: z.string().default('~/.crewchief/worktrees/<repo-name>'),
```

## Data Flow

```
1. User: crewchief worktree create feature-x

2. Load config:
   → worktreeBasePath: "~/.crewchief/worktrees/<repo-name>"

3. Expand path (expandWorktreePath):
   → expandTilde("~/.crewchief/worktrees/<repo-name>")
   → "/home/user/.crewchief/worktrees/<repo-name>"

   → expandRepoPlaceholder("/home/user/.crewchief/worktrees/<repo-name>")
   → getRepositoryName() returns "crewchief"
   → "/home/user/.crewchief/worktrees/crewchief"

   → path.resolve("/home/user/.crewchief/worktrees/crewchief")
   → "/home/user/.crewchief/worktrees/crewchief" (already absolute)

4. Create worktree:
   → path.join("/home/user/.crewchief/worktrees/crewchief", "feature-x")
   → "/home/user/.crewchief/worktrees/crewchief/feature-x"

5. Git: git worktree add -B feature-x [expanded path] main
```

## Integration Points

**Worktree Commands** (`packages/cli/src/cli/worktree.ts`):
- All commands already use `config.repository.worktreeBasePath`
- No changes needed - expansion happens in WorktreeService

**Orchestrator** (`packages/cli/src/orchestrator/scheduler.ts`):
- Uses `config.repository.worktreeBasePath` for agent worktrees
- Automatically benefits from expansion

**Tests**:
- Mock `expandWorktreePath()` in unit tests
- Update test configs to use new default
- Add integration tests for expansion

## Performance Considerations

**Git Command Overhead**: Repository name detection calls `git config`, adds ~10-50ms. Acceptable for worktree creation (not in hot path).

**Caching**: Not needed for MVP. Worktree creation is infrequent operation.

## Maintainability

**Separation of Concerns**: Path expansion logic isolated in `utils/paths.ts`. Changes to expansion don't affect worktree logic.

**Testability**: Pure functions for tilde expansion, mocked git for repo detection. Easy to test edge cases.

**Future Extensions**: Adding new placeholders or environment variable expansion only requires changes to `paths.ts`.

**Documentation**: Clear examples in config files show both old and new patterns.
