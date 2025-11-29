# Implementation Plan: Test Environment Infrastructure

## Executive Summary

Two-phase implementation to provide reliable test infrastructure:
- **Phase 1**: SQL test fixtures (make all current tests pass)
- **Phase 2**: Dockerized daemon (enable true E2E testing)

## Phase 1: SQL Test Fixtures

**Goal**: Make all 397 integration tests pass using pre-indexed fixtures

**Duration**: ~1-2 days

### Deliverables

1. **Test Corpus Design Document**
   - Document expected query results
   - Define chunk distribution (languages, kinds)
   - Specify ranking expectations

2. **Fixture Generation Script**
   - `packages/maproom-mcp/scripts/create-test-fixtures.sh`
   - Based on existing `crates/maproom/scripts/create_fixture.sh`
   - Generates `test-fixtures.sql`

3. **Test Fixtures SQL**
   - `packages/maproom-mcp/tests/setup/test-fixtures.sql`
   - Pre-indexed test-corpus data
   - ~100 chunks across TypeScript, Python, Rust, Markdown

4. **Enhanced Test Setup**
   - Update `ensure-test-db.ts` to load fixtures
   - Add fixture verification step
   - Update `database.ts` helpers

5. **Test Updates**
   - Remove daemon-dependent logic from fixture-compatible tests
   - Mark true E2E tests for Phase 2

### Tickets (Phase 1)

| ID | Title | Agent | Effort |
|----|-------|-------|--------|
| TESTENV-1001 | Design test corpus with known query results | database-engineer | S |
| TESTENV-1002 | Create fixture generation script | database-engineer | M |
| TESTENV-1003 | Generate initial test fixtures | database-engineer | S |
| TESTENV-1004 | Integrate fixtures into test setup | typescript-engineer | M |
| TESTENV-1005 | Update tests to use fixtures | typescript-engineer | M |
| TESTENV-1006 | Verify all fixture-compatible tests pass | unit-test-runner | S |

### Success Criteria (Phase 1)

- [ ] All 397 integration tests pass
- [ ] Test suite completes in <30 seconds
- [ ] Fixtures load in <50ms
- [ ] No daemon required for standard test runs
- [ ] CI pipeline passes

---

## Phase 2: Dockerized Daemon

**Goal**: Enable true E2E testing with real indexing

**Duration**: ~1-2 days (reduced - leveraging existing Dockerfile)

### Deliverables

1. **Docker Compose Updates** (using existing Dockerfile)
   - Add `maproom-daemon` service referencing `/workspace/Dockerfile.maproom`
   - Configure `e2e` profile
   - Set up container networking
   - **NOTE**: Dockerfile already exists at `/workspace/Dockerfile.maproom` with:
     - Multi-stage build (rust:1.82-slim → debian:bookworm-slim)
     - Non-root user (`maproom`, uid 1000)
     - Health check endpoint (port 3000)
     - All security best practices implemented

3. **E2E Test Helpers**
   - `tests/helpers/daemon.ts`
   - Health check waiter
   - Daemon availability detection

4. **E2E Test Suite**
   - Separate E2E tests that require real indexing
   - Skip conditions for non-daemon environments
   - CI job configuration

### Tickets (Phase 2) - Consolidated

| ID | Title | Agent | Effort |
|----|-------|-------|--------|
| TESTENV-2001 | Add daemon service to Docker Compose (using existing Dockerfile.maproom) | docker-engineer | M |
| TESTENV-2002 | Create daemon test helpers and configure E2E test suite | typescript-engineer | M |
| TESTENV-2003 | Document E2E testing workflow | typescript-engineer | S |

**Note**: Original 6 tickets consolidated to 3:
- TESTENV-2001 now uses existing `Dockerfile.maproom` (no new Dockerfile needed)
- TESTENV-2002 combines daemon helpers + E2E configuration
- TESTENV-2003 covers documentation
- CI E2E job deferred to future enhancement (current CI already validates daemon builds)

### Success Criteria (Phase 2)

- [ ] Daemon container builds successfully
- [ ] Daemon starts and responds to health checks
- [ ] E2E tests can index real files
- [ ] E2E tests pass in CI with daemon
- [ ] Documentation complete

---

## Implementation Order

```
Phase 1 (Fixtures):
├── TESTENV-1001: Design test corpus
├── TESTENV-1002: Create fixture script
├── TESTENV-1003: Generate fixtures
├── TESTENV-1004: Integrate into test setup
├── TESTENV-1005: Update tests
└── TESTENV-1006: Verify passing

Phase 2 (Daemon):
├── TESTENV-2001: Add daemon to Docker Compose
├── TESTENV-2002: Create helpers + E2E suite
└── TESTENV-2003: Document workflow
```

## Agent Assignments

| Agent | Tickets |
|-------|---------|
| database-engineer | 1001, 1002, 1003 |
| typescript-engineer | 1004, 1005, 2002, 2003 |
| docker-engineer | 2001 |
| unit-test-runner | 1006 |

**Total: 9 tickets** (reduced from original 12)

## Dependencies

### External Dependencies
- PostgreSQL 16 with pgvector (existing)
- Docker Compose (existing)
- Rust toolchain (for daemon build)

### Internal Dependencies
- Schema initialization (MCPSIMP-4003) - COMPLETED
- Docker Compose project name fix - COMPLETED

## Risks and Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Fixture schema drift | High | Medium | CI validation job |
| Daemon build failures | Medium | Low | Multi-platform build matrix |
| Flaky E2E tests | Medium | Medium | Health check waits, retries |
| CI timeout | Low | Low | Parallel test jobs |

## Rollback Plan

**Phase 1**: If fixtures cause issues, revert to current state (5 failing tests)

**Phase 2**: Daemon is opt-in via profile, can be removed without affecting Phase 1

## Documentation Updates

After implementation, update:
- `packages/maproom-mcp/CLAUDE.md` - Test environment setup
- `packages/maproom-mcp/tests/README.md` - Test running instructions
- `.github/CLAUDE.md` - CI pipeline documentation
