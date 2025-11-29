# Execution Plan: CLI UX Refinements

## Overview

This plan organizes the CLI modifications into logical phases with clear deliverables and testing milestones. Based on project review feedback, tickets have been consolidated from 12 to 4 for execution efficiency.

## Phase 1: Worktree Commands Modification

**Objective**: Modify both `worktree use` and `worktree create` to use path output as default.

### Ticket: CLIUX-1001 - Modify `worktree use` command

**Scope**:
- Remove auto-create behavior (search only, no creation)
- Change default from subshell to print path to stdout
- Add `--shell` flag for opt-in subshell
- Keep `--print` as explicit alias (backwards compatibility)
- Remove `--branch` and `--base-path` options (no longer applicable)
- Add clear error message with suggestion when worktree not found
- Update help text with examples

**Output Requirements**:
- Path goes to **stdout** (newline-terminated)
- Messages go to **stderr** (via logger)
- This ensures `cd $(crewchief worktree use feature-x)` works correctly

**Exit Codes**:
- `0`: Success (worktree found)
- `1`: Worktree not found or error

**Tests** (colocated at `packages/cli/src/cli/__tests__/worktree-use.test.ts`):
- Prints path when worktree exists
- Errors (exit 1) when worktree does not exist
- Spawns shell with `--shell` flag
- Handles ambiguous selectors correctly

**Agent**: general-purpose

---

### Ticket: CLIUX-1002 - Modify `worktree create` command

**Scope**:
- Change default from subshell to print path to stdout
- Add `--shell` flag for opt-in subshell
- Remove `--no-cd` flag (replaced by new default)
- Update help text with examples

**Output Requirements**:
- Path goes to **stdout** (newline-terminated)
- Success message goes to **stderr** (via logger.success)
- This ensures `cd $(crewchief worktree create new-feature)` works correctly

**Exit Codes**:
- `0`: Success (worktree created)
- `1`: Creation failed

**Tests** (colocated at `packages/cli/src/cli/__tests__/worktree-create.test.ts`):
- Prints path after creation by default
- Spawns shell with `--shell` flag
- Preserves existing creation options (--branch, --base-path, --no-copy-ignored)

**Agent**: general-purpose

---

## Phase 2: Agent Spawn Migration

**Objective**: Move `spawn` from top-level to `agent spawn`.

### Ticket: CLIUX-2001 - Migrate spawn to agent subcommand

**Scope**:
- Add `spawn` subcommand to agent command group in `agent.ts`
- Move spawn logic from `spawn.ts` (import or inline)
- Preserve all existing options: `-n`, `-v`, `-a`, `--no-label`, `--backend`, `--headless`
- Remove `registerSpawnCommand()` call from `index.ts`
- Delete or mark `spawn.ts` as deprecated
- Add comprehensive help text with examples

**Behavior**: Identical to current `crewchief spawn`, accessible at `crewchief agent spawn`

**Tests** (colocated at `packages/cli/src/cli/__tests__/agent-spawn.test.ts`):
- Command accessible at `agent spawn`
- All options preserved and functional
- Top-level `spawn` no longer accessible

**Agent**: general-purpose

---

## Phase 3: Integration Testing and Verification

**Objective**: End-to-end validation of all changes.

### Ticket: CLIUX-3001 - Integration tests and final verification

**Scope**:
- Create integration test file at `packages/cli/src/cli/__tests__/integration/cli-ux.test.ts`
- Test full worktree create → use workflow
- Test error cases (nonexistent worktree)
- Execute manual testing checklist
- Verify help text accuracy for all modified commands

**Integration Tests**:
```typescript
// Test workflow: create → use
// Test: use nonexistent → error with suggestion
// Test: cd $(worktree use ...) works (stdout isolation)
// Test: agent spawn accessible, spawn not accessible
```

**Manual Testing Checklist** (from quality-strategy.md):
- [ ] `crewchief worktree use <existing>` prints path to stdout
- [ ] `crewchief worktree use <nonexistent>` shows error (exit 1) with suggestion
- [ ] `crewchief worktree use <existing> --shell` opens subshell
- [ ] `cd $(crewchief worktree use <existing>)` works in bash/zsh
- [ ] `crewchief worktree create <name>` prints path to stdout
- [ ] `crewchief worktree create <name> --shell` opens subshell
- [ ] `cd $(crewchief worktree create <name>)` works
- [ ] `crewchief agent spawn claude` works
- [ ] `crewchief spawn` no longer works
- [ ] All `--help` output is accurate

**Agent**: integration-tester

---

## Execution Order

```
CLIUX-1001 (worktree use) ──▶ CLIUX-1002 (worktree create) ──▶ CLIUX-2001 (agent spawn) ──▶ CLIUX-3001 (integration)
```

Sequential execution recommended. Total scope is small enough that parallelization adds complexity without benefit.

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking existing scripts | Clear error messages with suggestions; help text documents changes |
| Stdout/stderr confusion | Explicit documentation: path→stdout, messages→stderr |
| Regression in spawn logic | Tests verify all options preserved |
| Rollback needed | Simple `git revert` per ticket; changes are isolated |

## Success Criteria

1. All unit tests pass
2. All integration tests pass
3. Manual testing checklist complete
4. Help text accurately reflects behavior
5. `cd $(crewchief worktree use/create ...)` works correctly
6. No regressions in existing functionality

## Dependencies

- No external dependencies
- No infrastructure changes
- No configuration changes

## Estimated Scope

- **Files modified**: 4 (worktree.ts, agent.ts, spawn.ts, index.ts)
- **Lines changed**: ~200-300
- **New tests**: 10-15
- **Tickets**: 4 total across 3 phases

## Rollback Plan

Each ticket is independently revertable:
- `git revert <commit>` for any ticket
- No shared state between tickets
- No database or infrastructure changes
