# DBFALLBK Ticket Index

**Project**: Database Connection Fallback
**Slug**: DBFALLBK
**Status**: Ready for Implementation
**Total Tickets**: 7

## Overview

This project implements consistent database connection fallback logic across Node.js CLI and Rust binary, and removes the confusing devcontainer postgres to use only maproom-postgres.

**Planning Documents**:
- [Analysis](../planning/analysis.md)
- [Architecture](../planning/architecture.md)
- [Quality Strategy](../planning/quality-strategy.md)
- [Security Review](../planning/security-review.md)
- [Implementation Plan](../planning/plan.md)
- [Project README](../README.md)

## Ticket Execution Order

### Phase 1: Remove Devcontainer Postgres (2 hours)

**DBFALLBK-1001** - Remove Devcontainer Postgres Service
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-1001_remove-devcontainer-postgres.md`
- **Agent**: general-purpose
- **Summary**: Remove postgres service from devcontainer docker-compose.yml and update DATABASE_URL to point to maproom-postgres
- **Dependencies**: None (first ticket)
- **Estimated Effort**: 2 hours

---

### Phase 2: Implement Rust Fallback Logic (4 hours)

**DBFALLBK-2001** - Implement Rust Database Connection Fallback Logic
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-2001_implement-rust-connection-fallback.md`
- **Agent**: general-purpose
- **Summary**: Create connection.rs module with get_database_url() function implementing 4-tier fallback hierarchy
- **Dependencies**: DBFALLBK-1001 (recommended, not required)
- **Estimated Effort**: 4 hours

**DBFALLBK-2901** - Test Rust Connection Fallback Logic
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-2901_test-rust-connection-fallback.md`
- **Agent**: general-purpose
- **Summary**: Write 4 unit tests and 1 integration test for Rust fallback logic
- **Dependencies**: DBFALLBK-2001 (must be complete)
- **Estimated Effort**: 2 hours

---

### Phase 3: Update Node.js CLI Logic (2 hours)

**DBFALLBK-3001** - Update Node.js CLI to Respect Explicit DATABASE_URL
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-3001_update-nodejs-cli-respect-database-url.md`
- **Agent**: general-purpose
- **Summary**: Modify scan and watch commands to check for existing DATABASE_URL before calling getDatabaseConnectionString()
- **Dependencies**: DBFALLBK-2001 (recommended for consistency)
- **Estimated Effort**: 2 hours

**DBFALLBK-3901** - Test Node.js CLI DATABASE_URL Behavior
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-3901_test-nodejs-cli-database-url.md`
- **Agent**: general-purpose
- **Summary**: Write 2 Node.js tests verifying DATABASE_URL respect and auto-detection fallback
- **Dependencies**: DBFALLBK-3001 (must be complete)
- **Estimated Effort**: 1 hour

---

### Phase 4: End-to-End Testing (3 hours)

**DBFALLBK-4001** - End-to-End Scenario Testing for Connection Fallback
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-4001_end-to-end-scenario-testing.md`
- **Agent**: general-purpose
- **Summary**: Execute and document 4 manual end-to-end scenarios (devcontainer, MCP user, direct Rust, MAPROOM_DB_HOST override)
- **Dependencies**: DBFALLBK-1001, DBFALLBK-2001, DBFALLBK-3001 (all must be complete)
- **Estimated Effort**: 3 hours

---

### Phase 5: Documentation & Cleanup (2 hours)

**DBFALLBK-5001** - Update Documentation for Single Database Architecture
- **File**: `/workspace/.agents/work-tickets/DBFALLBK-5001_update-documentation.md`
- **Agent**: general-purpose
- **Summary**: Update CLAUDE.md, DATABASE_ARCHITECTURE.md, and maproom-mcp README to reflect single database architecture and connection fallback
- **Dependencies**: DBFALLBK-1001 through DBFALLBK-4001 (all should be complete)
- **Estimated Effort**: 2 hours

---

## Summary by Phase

| Phase | Tickets | Effort | Description |
|-------|---------|--------|-------------|
| Phase 1 | 1 | 2h | Remove devcontainer postgres |
| Phase 2 | 2 | 6h | Rust fallback logic + tests |
| Phase 3 | 2 | 3h | Node.js CLI updates + tests |
| Phase 4 | 1 | 3h | End-to-end scenario testing |
| Phase 5 | 1 | 2h | Documentation updates |
| **Total** | **7** | **16h** | **Complete project** |

## Critical Path

The critical path through the project:

```
DBFALLBK-1001 (Phase 1)
    ↓
DBFALLBK-2001 (Phase 2 - Implementation)
    ↓
DBFALLBK-2901 (Phase 2 - Tests)
    ↓
DBFALLBK-3001 (Phase 3 - Implementation)
    ↓
DBFALLBK-3901 (Phase 3 - Tests)
    ↓
DBFALLBK-4001 (Phase 4 - E2E Testing)
    ↓
DBFALLBK-5001 (Phase 5 - Documentation)
```

## Success Criteria

The project is complete when:

- [x] All 7 tickets completed
- [x] All tests passing (Rust unit tests, Rust integration tests, Node.js tests)
- [x] All 4 end-to-end scenarios verified
- [x] Documentation updated and consistent
- [x] No references to old devcontainer postgres remain
- [x] Single database architecture (maproom-postgres only)
- [x] Consistent fallback behavior across Rust and Node.js

## Files Modified Summary

**Configuration**:
- `.devcontainer/docker-compose.yml` - Remove postgres service

**Rust Code**:
- `crates/maproom/src/db/connection.rs` - New fallback module
- `crates/maproom/src/db/pool.rs` - Use fallback
- `crates/maproom/src/db/queries.rs` - Use fallback
- `crates/maproom/src/db/mod.rs` - Export connection module

**Rust Tests**:
- `crates/maproom/src/db/connection.rs` - Unit tests
- `crates/maproom/tests/connection_fallback_test.rs` - Integration test

**Node.js Code**:
- `packages/maproom-mcp/bin/cli.cjs` - Respect DATABASE_URL

**Node.js Tests**:
- `packages/maproom-mcp/tests/connection-fallback.test.js` - CLI tests

**Documentation**:
- `CLAUDE.md` - Update database architecture
- `docs/architecture/DATABASE_ARCHITECTURE.md` - Remove devcontainer postgres
- `packages/maproom-mcp/README.md` - Document fallback behavior

## Next Steps

1. Run `/single-ticket DBFALLBK-1001` to start Phase 1
2. After completion, proceed sequentially through tickets
3. Ensure each phase completes before starting next phase
4. Run full test suite after Phase 2 and Phase 3
5. Execute all E2E scenarios in Phase 4 before documentation
6. Final verification: All acceptance criteria met

## Related Projects

This project is standalone but supports:
- Future MCP server improvements
- Development workflow simplification
- Database migration tooling
