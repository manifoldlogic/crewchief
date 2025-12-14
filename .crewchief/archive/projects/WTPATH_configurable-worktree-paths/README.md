# Project: Configurable Worktree Paths

**Slug:** WTPATH
**Status:** Planning
**Created:** 2025-12-05

## Summary

Enable configurable worktree storage locations with support for tilde expansion (`~`) and repository name placeholders (`<repo-name>`). Change default from `.crewchief/worktrees` to `~/.crewchief/worktrees/<repo-name>/` to reduce workspace clutter and improve IDE performance.

## Problem Statement

CrewChief currently stores all worktrees at `.crewchief/worktrees/` inside the repository. This creates several issues:

1. **Workspace Clutter**: Worktrees clutter the IDE file tree
2. **IDE Performance**: Large worktree directories slow down IDE indexing
3. **Disk Space Visibility**: Hidden worktrees make disk usage tracking difficult
4. **Multi-Repo Conflicts**: No centralized location across repositories

## Proposed Solution

**New Default**: `~/.crewchief/worktrees/<repo-name>/`

**Key Features**:
- **Tilde Expansion**: `~/my-worktrees` expands to user home directory
- **Repository Placeholders**: `<repo-name>` replaced with detected repository name
- **Absolute Path Support**: `/mnt/worktrees` works without joining to cwd
- **Backward Compatible**: Existing `.crewchief/worktrees` still works via config

**Implementation Approach**:
1. Create path expansion utilities (`packages/cli/src/utils/paths.ts`)
2. Integrate expansion into WorktreeService
3. Update config schema default
4. Document migration path for existing users

## Relevant Agents

**Planning Phase**:
- project-planner (planning docs)

**Implementation Phase**:
- typescript-dev (path utilities, WorktreeService integration, config update)
- unit-test-runner (tests for utilities and integration)
- docs-writer (README updates, migration guide)
- verify-ticket (verification)
- commit-ticket (commits)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis and research
- [architecture.md](planning/architecture.md) - Solution design with path expansion utilities
- [plan.md](planning/plan.md) - 3-phase execution plan (utilities → integration → config)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach (unit + integration)
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk)

## Key Decisions

1. **Path Expansion Utility**: Separate `utils/paths.ts` for reusable, testable expansion logic
2. **Placeholder Syntax**: Use `<repo-name>` (clear, no shell conflicts)
3. **Repository Detection**: Try git remote first, fall back to directory basename
4. **Expansion Timing**: Expand at worktree creation time (not config load)
5. **Breaking Change**: New default requires clear migration documentation

## Tickets

Tickets will be created after planning review via `/workstream:project-tickets WTPATH`

## Next Steps

1. Run `/workstream:project-review WTPATH` to validate planning
2. Address any review feedback
3. Run `/workstream:project-tickets WTPATH` to generate implementation tickets
4. Execute tickets in order (Phase 1 → Phase 2 → Phase 3)
