# MCPDB Ticket Index - MCP Server SQLite Support

## Overview

This index tracks all tickets for the MCPDB project, which adds SQLite backend support to the TypeScript MCP server.

**Project:** MCPDB - MCP Server SQLite Support
**Total Tickets:** 6
**Estimated Duration:** 2-3 days

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

## Phase 1: URL Parsing Enhancement

| Ticket | Title | Status | Agent | Est. |
|--------|-------|--------|-------|------|
| [MCPDB-1001](MCPDB-1001_url-parsing-enhancement.md) | URL Parsing Enhancement | Not Started | general-purpose | 0.5d |

**Deliverables:**
- `DatabaseConfig` interface
- `resolveDatabaseConfig()` function
- `isSqliteUrl()` helper
- SQLite URL parsing with path expansion
- Auto-detection of `~/.maproom/maproom.db`

## Phase 2: Daemon Integration & PostgreSQL Handling

| Ticket | Title | Status | Agent | Est. |
|--------|-------|--------|-------|------|
| [MCPDB-1002](MCPDB-1002_daemon-sqlite-configuration.md) | Daemon SQLite Configuration | Not Started | general-purpose | 0.25d |
| [MCPDB-1006](MCPDB-1006_postgresql-dependency-handling.md) | PostgreSQL Dependency Handling | Not Started | general-purpose | 0.25d |

**Deliverables:**
- Daemon uses `resolveDatabaseConfig()`
- SQLite file validation with helpful errors
- `search.ts` conditional bypass for `fetchChunkIds()`
- `handleStatus()` degraded response for SQLite
- Warning logs for SQLite limitations

## Phase 3: Test Infrastructure

| Ticket | Title | Status | Agent | Est. |
|--------|-------|--------|-------|------|
| [MCPDB-1003](MCPDB-1003_sqlite-test-helpers.md) | SQLite Test Helpers | Not Started | integration-tester | 0.5d |
| [MCPDB-1004](MCPDB-1004_sqlite-integration-tests.md) | SQLite Integration Tests | Not Started | integration-tester | 0.5d |

**Deliverables:**
- `tests/helpers/sqlite.ts` with fixture management
- `tests/integration/sqlite-backend.test.ts`
- `pnpm test:sqlite` script
- Tests for status, search, open tools
- Error handling tests

## Phase 4: CI Integration

| Ticket | Title | Status | Agent | Est. |
|--------|-------|--------|-------|------|
| [MCPDB-1005](MCPDB-1005_ci-sqlite-integration.md) | CI SQLite Integration | Not Started | github-actions-specialist | 0.5d |

**Deliverables:**
- `test-mcp-sqlite` job in `.github/workflows/test.yml`
- Fixture existence check/regeneration
- CI runs without PostgreSQL service

## Dependency Matrix

| Ticket | Depends On | Blocks |
|--------|------------|--------|
| MCPDB-1001 | - | MCPDB-1002, MCPDB-1006 |
| MCPDB-1002 | MCPDB-1001 | MCPDB-1003, MCPDB-1004 |
| MCPDB-1006 | MCPDB-1001, MCPDB-1002 | MCPDB-1003, MCPDB-1004 |
| MCPDB-1003 | MCPDB-1006 | MCPDB-1004, MCPDB-1005 |
| MCPDB-1004 | MCPDB-1003, MCPDB-1006 | MCPDB-1005 |
| MCPDB-1005 | MCPDB-1004 | - |

## Files Modified/Created Summary

### Modified Files
- `packages/maproom-mcp/src/utils/resolve-database.ts`
- `packages/maproom-mcp/src/daemon.ts`
- `packages/maproom-mcp/src/tools/search.ts`
- `packages/maproom-mcp/src/index.ts`
- `packages/maproom-mcp/package.json`
- `.github/workflows/test.yml`

### New Files
- `packages/maproom-mcp/tests/unit/resolve-database.test.ts`
- `packages/maproom-mcp/tests/helpers/sqlite.ts`
- `packages/maproom-mcp/tests/integration/sqlite-backend.test.ts`

## Success Criteria

### Must Pass (MVP)
- [ ] URL parser correctly identifies SQLite vs PostgreSQL
- [ ] `status` tool works with SQLite backend (degraded)
- [ ] `search` tool returns FTS results from SQLite
- [ ] `open` tool retrieves code from SQLite-indexed files
- [ ] Existing PostgreSQL tests continue to pass
- [ ] CI runs SQLite tests without PostgreSQL service

### Should Pass (Quality)
- [ ] Error messages guide users to fix issues
- [ ] Auto-detection finds `~/.maproom/maproom.db`
- [ ] Test suite runs in <30 seconds with SQLite
- [ ] No new npm dependencies added

## Known Limitations (SQLite Mode)

| MCP Tool | SQLite Support | Limitation |
|----------|---------------|------------|
| `search` | Full | `chunk_id` field is 0 (warning logged) |
| `open` | Full | No limitations |
| `status` | Partial | Returns degraded response (no detailed stats) |

## Plan Traceability

| Plan Section | Tickets |
|--------------|---------|
| Phase 1: URL Parsing Enhancement | MCPDB-1001 |
| Phase 2: Daemon Integration | MCPDB-1002, MCPDB-1006 |
| Phase 3: Test Infrastructure | MCPDB-1003, MCPDB-1004 |
| Phase 4: CI Integration | MCPDB-1005 |

## Progress Tracking

**Overall Progress:** 6/6 tickets complete (100%) ✅

| Phase | Tickets | Complete | Status |
|-------|---------|----------|--------|
| Phase 1 | 1 | 1 | Complete ✅ |
| Phase 2 | 2 | 2 | Complete ✅ |
| Phase 3 | 2 | 2 | Complete ✅ |
| Phase 4 | 1 | 1 | Complete ✅ |

---

**Last Updated:** 2025-11-26
**Created By:** `/create-project-tickets MCPDB`
