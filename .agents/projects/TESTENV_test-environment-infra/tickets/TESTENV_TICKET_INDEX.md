# TESTENV Ticket Index

**Project**: Test Environment Infrastructure
**Total Tickets**: 9
**Created**: 2025-11-25

## Overview

This project provides reliable test infrastructure through two phases:
- **Phase 1**: SQL test fixtures (make all 397 tests pass)
- **Phase 2**: Dockerized daemon (enable true E2E testing)

---

## Phase 1: SQL Test Fixtures (1xxx)

| ID | Title | Agent | Status | Effort |
|----|-------|-------|--------|--------|
| [TESTENV-1001](./TESTENV-1001_design-test-corpus.md) | Design test corpus with known query results | database-engineer | Not Started | S |
| [TESTENV-1002](./TESTENV-1002_create-fixture-script.md) | Create fixture generation script | database-engineer | Not Started | M |
| [TESTENV-1003](./TESTENV-1003_generate-fixtures.md) | Generate initial test fixtures | database-engineer | Not Started | S |
| [TESTENV-1004](./TESTENV-1004_integrate-fixtures.md) | Integrate fixtures into test setup | typescript-engineer | Not Started | M |
| [TESTENV-1005](./TESTENV-1005_update-tests.md) | Update tests to use fixtures | typescript-engineer | Not Started | M |
| [TESTENV-1006](./TESTENV-1006_verify-tests-pass.md) | Verify all fixture-compatible tests pass | unit-test-runner | Not Started | S |

**Phase 1 Success Criteria:**
- [ ] All 397 integration tests pass
- [ ] Test suite completes in <30 seconds
- [ ] Fixtures load in <50ms
- [ ] No daemon required for standard test runs
- [ ] CI pipeline passes

---

## Phase 2: Dockerized Daemon (2xxx)

| ID | Title | Agent | Status | Effort |
|----|-------|-------|--------|--------|
| [TESTENV-2001](./TESTENV-2001_add-daemon-compose.md) | Add daemon service to Docker Compose | docker-engineer | Not Started | M |
| [TESTENV-2002](./TESTENV-2002_daemon-helpers-e2e.md) | Create daemon test helpers and configure E2E suite | typescript-engineer | Not Started | M |
| [TESTENV-2003](./TESTENV-2003_document-e2e.md) | Document E2E testing workflow | typescript-engineer | Not Started | S |

**Phase 2 Success Criteria:**
- [ ] Daemon container builds successfully
- [ ] Daemon starts and responds to health checks
- [ ] E2E tests can index real files
- [ ] E2E tests pass in CI with daemon
- [ ] Documentation complete

---

## Dependency Graph

```
Phase 1 (Sequential):
TESTENV-1001 → TESTENV-1002 → TESTENV-1003 → TESTENV-1004 → TESTENV-1005 → TESTENV-1006

Phase 2 (Sequential, after Phase 1):
TESTENV-2001 → TESTENV-2002 → TESTENV-2003
```

---

## Agent Assignments

| Agent | Tickets |
|-------|---------|
| database-engineer | 1001, 1002, 1003 |
| typescript-engineer | 1004, 1005, 2002, 2003 |
| docker-engineer | 2001 |
| unit-test-runner | 1006 |

---

## Plan Reference

See [planning/plan.md](../planning/plan.md) for full implementation details.
