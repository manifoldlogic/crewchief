# CLIUX Ticket Index

## Project: CLI UX Refinements

**Status**: 🔄 In Progress
**Created**: 2025-11-26
**Tickets**: 4 total

## Overview

Minor UX refinements to the CrewChief CLI:
1. Modify `worktree use` to only work with existing worktrees, print path by default
2. Modify `worktree create` to print path by default
3. Move `spawn` command under `agent spawn`
4. Integration testing to verify all changes

## Execution Order

```
CLIUX-1001 → CLIUX-1002 → CLIUX-2001 → CLIUX-3001
```

Sequential execution recommended.

---

## Phase 1: Worktree Commands Modification

| Ticket | Title | Status | Agent |
|--------|-------|--------|-------|
| [CLIUX-1001](CLIUX-1001_modify-worktree-use.md) | Modify `worktree use` command | Pending | general-purpose |
| [CLIUX-1002](CLIUX-1002_modify-worktree-create.md) | Modify `worktree create` command | Pending | general-purpose |

---

## Phase 2: Agent Spawn Migration

| Ticket | Title | Status | Agent |
|--------|-------|--------|-------|
| [CLIUX-2001](CLIUX-2001_migrate-spawn-to-agent.md) | Migrate spawn to agent subcommand | Pending | general-purpose |

---

## Phase 3: Integration Testing

| Ticket | Title | Status | Agent |
|--------|-------|--------|-------|
| [CLIUX-3001](CLIUX-3001_integration-tests.md) | Integration tests and verification | Pending | integration-tester |

---

## Plan Reference

See [plan.md](../planning/plan.md) for full execution plan details.

## Files Affected

- `packages/cli/src/cli/worktree.ts` - Worktree command changes
- `packages/cli/src/cli/agent.ts` - Add spawn subcommand
- `packages/cli/src/cli/spawn.ts` - Remove/deprecate
- `packages/cli/src/cli/index.ts` - Update command registration
- `packages/cli/src/cli/__tests__/` - New test files
