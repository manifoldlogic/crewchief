# Analysis: Worktree Use Auto-Scan Control

## Problem Definition

CrewChief currently performs automatic maproom scanning after every `worktree create` operation (line 143 in `packages/cli/src/git/worktrees.ts`). This unconditional behavior creates several problems:

1. **Unexpected Delays**: Users experience unexpected 5-30 second delays when creating worktrees, especially on large codebases
2. **No User Control**: Auto-scan happens regardless of user needs - even when they just want to quickly create a worktree without indexing
3. **Poor UX**: The delay feels like the tool is hanging or broken when users expect instant worktree operations
4. **Resource Waste**: Scanning happens even when users don't need or want semantic search on that worktree

**Note**: This issue only affects `worktree create`. The `worktree use` command (which switches between existing worktrees) does not trigger scanning and is not modified by this project. The project title "Worktree Use Auto-Scan Control" refers to controlling auto-scan when _using_ (creating) worktrees, not the `use` subcommand.

The core issue: **Auto-scan is a policy decision masquerading as a feature, with no escape hatch for users who don't want it.**

## Context

### Background

The auto-scan behavior was originally added to ensure new worktrees are immediately searchable via maproom semantic search. The intention was good: users wouldn't have to remember to manually run `crewchief maproom scan` after creating a worktree.

However, this convenience comes at a significant cost:
- Large codebases (>10k files) can take 30+ seconds to scan
- Many worktrees are short-lived and never need search
- CI/CD environments and automated scripts suffer unnecessary delays
- Users who don't use maproom search still pay the cost

### Why This Work Is Needed

This is a **breaking change by design** - we need to shift from "scan by default" to "scan by choice". The current behavior violates the principle of least surprise: worktree operations should be fast and predictable.

**Business Value**:
- Faster worktree operations improve developer velocity
- User control aligns with Unix philosophy (tools do one thing well)
- Opt-in scanning reduces surprise and frustration

**Technical Value**:
- Clean separation of concerns (worktree management ≠ indexing)
- Better testability (can test worktree ops without maproom dependency)
- Foundation for future features (conditional scanning based on worktree purpose)

## Existing Solutions

### Industry Patterns

1. **Git Worktree**: Fast worktree creation with no implicit indexing
2. **VS Code**: Workspace indexing is async and user-controllable
3. **JetBrains IDEs**: Allow users to exclude directories from indexing
4. **ripgrep**: Fast search without pre-indexing requirement

**Key Insight**: Best-in-class tools keep core operations fast and let users opt-in to expensive indexing.

### Codebase Patterns

Searching the codebase reveals several relevant patterns:

1. **Config-Controlled Behavior**: `WorktreeSchema` already has `copyIgnoredFiles` optional config (line 55 in `packages/cli/src/config/schema.ts`)
2. **Optional Operations**: `--no-copy-ignored` flag allows users to skip file copying (line 39 in test file)
3. **Manual Scan Available**: `crewchief maproom scan` command exists for on-demand scanning

**Pattern to Follow**: Add config field to `WorktreeSchema`, default to false, check before calling `runMaproomScan()`.

## Current State

### Implementation Details

**Location**: `packages/cli/src/git/worktrees.ts`

**Current Flow** (createWorktree method, lines 95-145):
1. Create git worktree
2. Save metadata
3. Copy ignored files (if configured)
4. **Always call `runMaproomScan(wtPath)`** (line 143)

**Auto-Scan Implementation** (runMaproomScan method, lines 25-93):
- Finds maproom binary (environment, packaged, or global)
- Runs `crewchief-maproom scan` in worktree directory
- Prints status messages to console
- Handles errors gracefully (warnings, not failures)

**Key Observation**: The scan is already non-blocking and fault-tolerant. The problem is not technical - it's the unconditional execution.

### Config Schema

**Current State** (`packages/cli/src/config/schema.ts`, lines 54-58):
```typescript
export const WorktreeSchema = z.object({
  copyIgnoredFiles: z.array(z.string()).optional(),
  copyFromPath: z.string().default('.'),
  overwriteStrategy: z.enum(['skip', 'overwrite', 'backup']).default('skip'),
})
```

**Missing**: No field to control auto-scan behavior.

### Testing Patterns

**Existing Test**: `packages/cli/src/cli/__tests__/worktree-create.test.ts`
- Mocks `WorktreeService`
- Tests flag combinations
- Verifies method calls with correct arguments

**Pattern to Follow**: Add tests for scan behavior with config enabled/disabled.

## Research Findings

### Performance Impact

From documentation and testing:
- **Small repos** (<1k files): 1-3 second delay
- **Medium repos** (1k-10k files): 5-15 second delay
- **Large repos** (>10k files): 30-60+ second delay

**User Impact**: On large codebases, worktree creation feels broken.

### Usage Patterns

Based on worktree command structure and README documentation:
1. **Quick Context Switches**: Users frequently switch worktrees for different tasks
2. **Agent Orchestration**: Automated agents create temporary worktrees
3. **Manual Scanning**: Users already have `crewchief maproom scan` for explicit indexing

**Insight**: Most worktree operations don't need immediate indexing.

### Dependency on WTPATH

The project summary notes this depends on **WTPATH** (config schema foundation). Reviewing WTPATH:
- Adds path expansion utilities
- Updates `WorktreeSchema` default path
- Already modifies config schema

**Implication**: We can add `autoScanOnWorktreeUse` to the same schema structure WTPATH establishes.

## Constraints

### Technical Constraints

1. **Backward Compatibility**: Existing users rely on auto-scan behavior
2. **Config Schema**: Must extend `WorktreeSchema` properly with Zod
3. **Test Coverage**: Must maintain existing test patterns
4. **Error Handling**: Scan failures must remain non-fatal

### Business Constraints

1. **Breaking Change**: Default behavior changes - requires clear communication
2. **Migration Path**: Users need easy way to restore old behavior
3. **Documentation**: Must explain opt-in vs opt-out clearly

### Design Constraints

1. **Default Value**: Must choose between opt-in (false) or opt-out (true)
2. **Flag Consistency**: Should align with existing flag patterns (--no-copy-ignored)
3. **Config Naming**: Field name should be clear and unambiguous

## Success Criteria

### Functional Success

- [ ] Config accepts `autoScanOnWorktreeUse: boolean` field
- [ ] Default value is `false` (opt-in scanning)
- [ ] `worktree create` skips scan by default
- [ ] `worktree create` runs scan when config is true
- [ ] Manual `crewchief maproom scan` still works
- [ ] `worktree use` command continues to work unchanged (does not trigger scans)

### Technical Success

- [ ] Config schema validates correctly
- [ ] No regression in existing worktree functionality
- [ ] Tests cover both enabled and disabled states
- [ ] Error handling remains robust

### User Experience Success

- [ ] Worktree creation feels instant on default settings
- [ ] Users can easily enable auto-scan via config
- [ ] Documentation clearly explains trade-offs
- [ ] Migration guide helps users transition

### Performance Success

- [ ] Worktree creation time reduced by 5-30 seconds (depending on repo size)
- [ ] No performance regression when auto-scan is enabled
- [ ] Test suite execution time unchanged

## Measurable Outcomes

1. **Worktree Creation Time**: <1 second for default operation (down from 5-30s)
2. **Config Adoption**: Clear documentation enables <5 minute migration for existing users
3. **Test Coverage**: 100% coverage of new config behavior
4. **Breaking Change Impact**: Zero production failures due to clear migration path
