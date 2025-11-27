# CLIUX: CLI UX Refinements

## Status: Complete

**Completed**: 2025-11-27

All tickets implemented, tested, and verified.

## Overview

Minor UX refinements to the CrewChief CLI to improve command semantics and organization.

## Problem Statement

The current CLI has three areas needing refinement:

1. **`worktree use` auto-creates**: The command creates worktrees if they don't exist, violating the principle of least surprise. Users expect "use" to work with existing worktrees only.

2. **Subshell as default**: Both `worktree create` and `worktree use` spawn interactive subshells by default, which is invasive for scripting and requires users to type `exit` to return.

3. **`spawn` at top level**: The agent spawning command is at the root level when it logically belongs under the `agent` subcommand group.

## Proposed Solution

### Worktree Commands
- `worktree use` - Only work with existing worktrees; print path to **stdout** by default
- `worktree create` - Print path to **stdout** by default; `--shell` for interactive subshell
- Clear error messages (exit code 1) when worktree not found

### Command Organization
- Move `spawn` to `agent spawn`
- Group all agent-related commands together

### Output Streams (Critical for Scripting)
```bash
# Path goes to stdout, messages to stderr
# This enables: cd $(crewchief worktree use feature-x)
```

### New Defaults
```bash
# Print path (new default)
cd $(crewchief worktree use feature-x)
cd $(crewchief worktree create new-feature)

# Interactive subshell (opt-in)
crewchief worktree use feature-x --shell
crewchief worktree create new-feature --shell
```

## Relevant Agents

- **general-purpose**: Implementation of command changes
- **integration-tester**: End-to-end workflow tests
- **verify-ticket**: Verification of completed work
- **commit-ticket**: Final commits

## Planning Documents

| Document | Description |
|----------|-------------|
| [analysis.md](planning/analysis.md) | Deep dive into current CLI behavior and change requirements |
| [architecture.md](planning/architecture.md) | Solution design with stdout/stderr requirements |
| [quality-strategy.md](planning/quality-strategy.md) | Testing strategy with mocking patterns |
| [security-review.md](planning/security-review.md) | Security assessment (minimal impact) |
| [plan.md](planning/plan.md) | Consolidated 4-ticket execution plan |
| [project-review.md](planning/project-review.md) | Pre-ticket review findings |
| [review-updates.md](planning/review-updates.md) | Changes made based on review |

## Execution Summary

| Phase | Description | Ticket |
|-------|-------------|--------|
| 1 | Modify `worktree use` | CLIUX-1001 |
| 1 | Modify `worktree create` | CLIUX-1002 |
| 2 | Migrate `spawn` to `agent spawn` | CLIUX-2001 |
| 3 | Integration testing | CLIUX-3001 |

**Tickets**: 4 total (consolidated from original 12 based on review)

## Files Affected

- `packages/cli/src/cli/worktree.ts` - Worktree command changes
- `packages/cli/src/cli/agent.ts` - Add spawn subcommand
- `packages/cli/src/cli/spawn.ts` - Remove/deprecate
- `packages/cli/src/cli/index.ts` - Update command registration

## Key Technical Decisions

1. **Stdout/stderr separation**: Path to stdout, messages to stderr (enables `cd $(...)`)
2. **Exit codes**: 0 for success, 1 for errors
3. **--print flag**: Kept as no-op alias for backwards compatibility
4. **Sequential execution**: Recommended over parallel for simplicity

## Completion Summary

All 4 tickets completed and verified:
- CLIUX-1001: `worktree use` now prints path by default, requires existing worktree
- CLIUX-1002: `worktree create` now prints path by default
- CLIUX-2001: `spawn` command migrated to `agent spawn`
- CLIUX-3001: Integration tests added to verify behavior

This project is complete and archived.
