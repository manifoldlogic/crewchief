# MCPDB Project Plan - MCP Server SQLite Support

## Project Summary

Update the TypeScript MCP server (`packages/maproom-mcp/`) to support SQLite database URLs and file paths, enabling zero-config MCP tool usage with the SQLite backend completed in MAPCLI.

## Phase Overview

| Phase | Focus | Deliverables |
|-------|-------|--------------|
| Phase 1 | URL Parsing | `resolveDatabaseConfig()`, SQLite URL support |
| Phase 2 | Daemon Integration | SQLite-aware daemon configuration |
| Phase 3 | Test Infrastructure | SQLite test helpers, integration tests |
| Phase 4 | CI Integration | GitHub Actions SQLite test job |

## Detailed Plan

### Phase 1: URL Parsing Enhancement

**Goal**: Parse and validate SQLite URLs, detect backend type

**Tickets**:
- **MCPDB-1001**: Add `DatabaseConfig` type and `resolveDatabaseConfig()` function
  - Define `DatabaseConfig` interface with `type`, `url`, `path` fields
  - Implement SQLite URL parsing with path expansion
  - Implement PostgreSQL URL detection
  - Add auto-detection for `~/.maproom/maproom.db`
  - Maintain backward-compatible `resolveDatabase()` export
  - Add `isSqliteUrl()` helper function

**Files Modified**:
- `src/utils/resolve-database.ts`

**Tests**:
- Update `tests/unit/resolve-database.test.ts`
- Add SQLite URL parsing test cases

**Agent**: General implementation agent (TypeScript)

---

### Phase 2: Daemon Integration & PostgreSQL Dependency Handling

**Goal**: Configure daemon client to work with SQLite URLs and handle legacy PostgreSQL dependencies

**Tickets**:
- **MCPDB-1002**: Update `daemon.ts` for SQLite configuration
  - Use `resolveDatabaseConfig()` instead of raw environment variable
  - Add SQLite file existence validation with helpful error messages
  - Pass resolved URL to daemon environment
  - Update error messages for SQLite-specific scenarios

- **MCPDB-1006**: Handle PostgreSQL-dependent code paths for SQLite mode
  - Add conditional logic to `search.ts` to skip `fetchChunkIds()` for SQLite
  - Return `chunk_id: 0` with warning log for SQLite search results
  - Add conditional logic to `handleStatus()` to return degraded response for SQLite
  - Export `resolveDatabaseConfig()` for use in tool handlers
  - Document SQLite limitations in code comments

**Files Modified**:
- `src/daemon.ts`
- `src/tools/search.ts`
- `src/index.ts`
- `src/utils/resolve-database.ts` (export addition)

**Tests**:
- Manual verification with SQLite fixture:
  1. Set `MAPROOM_DATABASE_URL=sqlite://<fixture-path>`
  2. Verify `search` returns results (with chunk_id=0 warning in logs)
  3. Verify `status` returns degraded response with hint
  4. Verify `open` works via daemon

**Agent**: General implementation agent (TypeScript)

---

### Phase 3: Test Infrastructure

**Goal**: Enable running tests with SQLite backend

**Tickets**:
- **MCPDB-1003**: Create SQLite test helpers
  - Add `tests/helpers/sqlite.ts` with fixture management
  - Add `createTestSqliteDatabase()` function
  - Add `cleanupTestSqliteDatabase()` function
  - Document SQLite test setup

- **MCPDB-1004**: Add SQLite integration tests
  - Create `tests/integration/sqlite-backend.test.ts`
  - Test `status` tool with SQLite
  - Test `search` tool with SQLite (FTS)
  - Test `open` tool with SQLite
  - Test error handling for missing SQLite file

**Files Created**:
- `tests/helpers/sqlite.ts`
- `tests/integration/sqlite-backend.test.ts`

**Agent**: integration-tester agent

---

### Phase 4: CI Integration

**Goal**: Run SQLite tests in CI without PostgreSQL

**Tickets**:
- **MCPDB-1005**: Add SQLite test job to GitHub Actions
  - Add `test-mcp-sqlite` job to `.github/workflows/test.yml`
  - Ensure SQLite fixture exists (or generate)
  - Run SQLite integration tests
  - Report SQLite test results

**Files Modified**:
- `.github/workflows/test.yml`

**Agent**: github-actions-specialist agent

---

## Execution Order

```
MCPDB-1001 (URL Parsing)
    │
    ▼
MCPDB-1002 (Daemon Integration)
    │
    ▼
MCPDB-1006 (PostgreSQL Dependency Handling)
    │
    ├───────────────────┐
    ▼                   ▼
MCPDB-1003          MCPDB-1004
(Test Helpers)      (Integration Tests)
    │                   │
    └─────────┬─────────┘
              ▼
        MCPDB-1005
        (CI Integration)
```

**Notes**:
- MCPDB-1006 must complete before tests (handles conditional execution)
- MCPDB-1003 and MCPDB-1004 can run in parallel after MCPDB-1006
- MCPDB-1005 depends on all previous tickets

## Agent Assignments

| Ticket | Agent | Rationale |
|--------|-------|-----------|
| MCPDB-1001 | General TypeScript | URL parsing, no specialized knowledge |
| MCPDB-1002 | General TypeScript | Daemon configuration update |
| MCPDB-1006 | General TypeScript | Conditional logic for PostgreSQL dependencies |
| MCPDB-1003 | integration-tester | Test infrastructure creation |
| MCPDB-1004 | integration-tester | Integration test development |
| MCPDB-1005 | github-actions-specialist | CI workflow expertise |

## Success Criteria

### Phase 1 Complete
- [ ] `resolveDatabaseConfig()` correctly parses SQLite URLs
- [ ] Auto-detection finds `~/.maproom/maproom.db`
- [ ] Unit tests pass for all URL parsing scenarios

### Phase 2 Complete
- [ ] Daemon starts with SQLite URL
- [ ] Helpful error for missing SQLite file
- [ ] `search` tool works with SQLite (chunk_id=0 with warning)
- [ ] `status` tool returns degraded response for SQLite
- [ ] `open` tool works via daemon (no changes needed)
- [ ] Manual verification: all three tools tested with SQLite fixture

### Phase 3 Complete
- [ ] `pnpm test:sqlite` runs without PostgreSQL
- [ ] Integration tests verify status, search, open tools
- [ ] Test documentation updated

### Phase 4 Complete
- [ ] CI runs SQLite tests automatically
- [ ] CI passes with both SQLite and PostgreSQL
- [ ] Zero regressions in existing tests

## Risk Mitigation

| Risk | Mitigation | Owner |
|------|------------|-------|
| Breaking PostgreSQL | Run full test suite before merge | verify-ticket agent |
| Path handling issues | Test on Linux CI (matches production) | integration-tester |
| CI timeout | Set reasonable timeout for SQLite tests | github-actions-specialist |
| fetchChunkIds failure | Skip for SQLite, use chunk_id=0 with warning | MCPDB-1006 |
| Status tool failure | Return degraded response with SQLite hint | MCPDB-1006 |
| Rollback needed | Revert commits if PostgreSQL tests fail | verify-ticket agent |

## Dependencies

### Completed (Prerequisites)
- VECSTORE: VectorStore trait with SQLite implementation
- MAPCLI: CLI SQLite support via `MAPROOM_DATABASE_URL`

### External
- Pre-indexed SQLite fixture at `crates/maproom/tests/fixtures/pre-indexed-maproom.db`

## Timeline Estimate

**Total**: 2-3 days

| Phase | Estimate |
|-------|----------|
| Phase 1 | 0.5 day |
| Phase 2 | 0.5 day |
| Phase 3 | 1 day |
| Phase 4 | 0.5 day |

## Out of Scope

- PostgreSQL functionality changes
- Rust daemon modifications (covered by MAPCLI)
- VSCode extension (separate VSCODEDB project)
- Embedding provider changes
- New npm dependencies

## Definition of Done

1. All tickets completed and verified
2. SQLite integration tests pass
3. PostgreSQL tests continue to pass
4. CI pipeline includes SQLite job
5. Documentation updated
6. Changes committed with conventional commits
