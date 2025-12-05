# Project: Maproom Binary Configuration

**Slug:** MRBIN
**Status:** Planning Complete
**Created:** 2025-12-05
**Effort:** S (1 day)
**Priority:** High

## Summary

Add `repository.maproomBinaryPath` configuration option to explicitly specify the maproom binary location, consolidate duplicated binary resolution logic into a shared utility, and update resolution order to prioritize global installations over packaged binaries.

## Problem Statement

Developers and users lack a consistent, explicit way to configure which maproom binary the CLI uses. Binary resolution logic is duplicated across three files with subtle differences, causing:

- Inconsistent behavior across CLI commands
- Inability to configure binary path persistently (must use env vars)
- Confusion when both local builds and global installs exist
- ~140 lines of duplicated code across maproom.ts, worktrees.ts, and maproom-mcp

The current resolution prioritizes packaged binaries over global installs, which conflicts with production expectations where global npm installs should take precedence.

## Proposed Solution

### High-Level Approach

1. **Add Config Schema Field**: Extend `RepositorySchema` with optional `maproomBinaryPath: string`
2. **Create Shared Utility**: Extract binary resolution into `packages/cli/src/utils/maproom-binary.ts`
3. **Update Resolution Order**: Implement precedence: env var > config > global > packaged
4. **Consolidate Code**: Remove duplicated logic from maproom.ts and worktrees.ts
5. **Test Thoroughly**: Unit tests for precedence, integration tests for CLI commands
6. **Document Well**: Update README and development docs with configuration examples

### Resolution Priority Order

```
1. CREWCHIEF_MAPROOM_BIN environment variable (highest)
2. config.repository.maproomBinaryPath
3. Global install (command -v crewchief-maproom)
4. Packaged binary (bin/<platform>/crewchief-maproom)
```

### Key Design Decisions

- **Config Location**: Add to existing `RepositorySchema` (not new top-level section)
- **MCP Independence**: Keep maproom-mcp's findMaproomBinary() separate (different concerns)
- **Path Handling**: Support relative and absolute paths, warn on invalid
- **Backwards Compatibility**: All existing resolution paths continue to work

## Relevant Agents

### Planning Phase
- project-planner (this planning)

### Implementation Phase
- typescript-engineer (schema, utility, refactoring)
- unit-test-runner (test execution)
- integration-tester (CLI validation)
- documentation-engineer (README, docs)

### Verification Phase
- verify-ticket (acceptance criteria validation)
- commit-ticket (commit creation)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis and research findings
- [architecture.md](planning/architecture.md) - Solution design and component details
- [plan.md](planning/plan.md) - Execution plan with 3 phases, 6 tickets
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach and coverage targets
- [security-review.md](planning/security-review.md) - Security assessment and risk acceptance

## Project Phases

### Phase 1: Configuration Foundation (MRBIN-1xxx)
**Goal:** Add config schema and shared utility without changing existing behavior

**Deliverables:**
- Config schema extension
- Shared binary resolution utility
- Unit tests for precedence order

**Tickets:**
- MRBIN-1001: Add maproomBinaryPath to config schema
- MRBIN-1002: Implement shared binary resolution utility
- MRBIN-1003: Unit tests for binary resolution

### Phase 2: CLI Integration (MRBIN-2xxx)
**Goal:** Consolidate duplicated code and integrate shared utility

**Deliverables:**
- Refactored maproom.ts (remove ~44 lines)
- Refactored worktrees.ts (remove ~42 lines)
- Updated error messages

**Tickets:**
- MRBIN-2001: Refactor maproom.ts to use shared utility
- MRBIN-2002: Refactor worktrees.ts to use shared utility

### Phase 3: Documentation (MRBIN-3xxx)
**Goal:** Complete documentation and validate real-world usage

**Deliverables:**
- Updated README.md
- Updated development docs
- Example configurations
- Integration test validation

**Tickets:**
- MRBIN-3001: Update documentation

## Acceptance Criteria

From initiative summary:

- [x] Planning documents complete
- [ ] Config accepts `maproomBinaryPath` setting
- [ ] Config path takes precedence over packaged binary
- [ ] Env var still takes highest precedence
- [ ] Global install checked before local packaged binary
- [ ] Binary resolution is consistent across all commands
- [ ] Development workflow documented

## Value Proposition

**For Developers:**
- Persistent configuration (no env var needed per session)
- Team-shareable via crewchief.config.js
- Local-only overrides via crewchief.config.local.js
- Clear precedence order (predictable behavior)

**For Production:**
- Global install preferred (no stale local builds)
- Environment variable emergency override
- Backwards compatible (existing workflows unchanged)

**For Maintainers:**
- ~100 lines of duplicated code removed
- Single source of truth for resolution logic
- Easier to test and debug
- Consistent behavior across codebase

## Dependencies

**None** - This project is self-contained within the CLI package.

**Can run in parallel with:** WTPATH project (no file conflicts)

## Breaking Changes

**Non-breaking:** This is an additive change. Existing resolution continues to work.

**Behavior change:** Global installation now preferred over packaged binary. This is an intentional improvement that better matches production expectations.

Users who have both a global install and packaged binary will now use the global install by default. This can be overridden with:
- Environment variable: `CREWCHIEF_MAPROOM_BIN=/path/to/binary`
- Config file: `maproomBinaryPath: './path/to/binary'`

## Next Steps

1. **Review Planning**: Run `/workstream:project-review MRBIN` to validate planning completeness
2. **Generate Tickets**: Run `/workstream:project-tickets MRBIN` to create detailed ticket files
3. **Begin Implementation**: Start with Phase 1 (foundation) tickets
4. **Iterate**: Each phase is independently valuable and testable

## Project Metrics

**Code Impact:**
- Files modified: 3 (schema.ts, maproom.ts, worktrees.ts)
- Files created: 2 (maproom-binary.ts, maproom-binary.test.ts)
- Lines removed: ~100 (duplicated code)
- Lines added: ~80 (utility + tests)
- Net reduction: ~20 lines

**Testing:**
- Unit tests: 10+ new tests
- Integration tests: 2+ updated tests
- Manual tests: 4 scenarios
- Coverage target: 90%+ for new code

**Documentation:**
- README.md updated
- docs/development/local-development.md updated
- packages/cli/README.md updated (if exists)
- Security considerations documented
