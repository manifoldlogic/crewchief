# TESTISO Ticket Index

**Project**: Test Database Isolation
**Total Tickets**: 6
**Status**: Ready for execution

## Ticket Overview

All tickets created and ready for implementation. Execute sequentially using `/work-on-project TESTISO` or individually using `/single-ticket TESTISO-XXXX`.

---

## Phase 1: Docker Infrastructure (1 ticket)

### TESTISO-1001: Add postgres-test service to Docker Compose
- **File**: `TESTISO-1001_add-postgres-test-service.md`
- **Agent**: docker-engineer
- **Status**: ⏳ Not Started
- **Dependencies**: None (first ticket)
- **Blocks**: TESTISO-1002, 1003, 1004, 1005
- **Summary**: Add postgres-test service to docker-compose.yml with port 5434, separate volume (maproom-test-data), and manual schema initialization
- **Plan Reference**: Phase 1 (lines 18-85)
- **Estimated Effort**: ~30 minutes

---

## Phase 2: Test Configuration (2 tickets)

### TESTISO-1002: Update vitest configuration for TEST_MAPROOM_DATABASE_URL
- **File**: `TESTISO-1002_update-vitest-config-test-database.md`
- **Agent**: general-implementation
- **Status**: ⏳ Not Started
- **Dependencies**: TESTISO-1001
- **Blocks**: TESTISO-1003, 1004
- **Summary**: Configure vitest.config.ts to use TEST_MAPROOM_DATABASE_URL with fallback, using container hostname (maproom-postgres-test:5432)
- **Plan Reference**: Phase 2 (lines 88-150)
- **Estimated Effort**: ~30 minutes

### TESTISO-1003: Update package.json test scripts
- **File**: `TESTISO-1003_update-package-json-test-scripts.md`
- **Agent**: general-implementation
- **Status**: ⏳ Not Started
- **Dependencies**: TESTISO-1001, 1002
- **Blocks**: TESTISO-1004
- **Summary**: Update test scripts to set TEST_MAPROOM_DATABASE_URL=postgresql://maproom:maproom@localhost:5434/maproom_test
- **Plan Reference**: Phase 2 (lines 88-150)
- **Estimated Effort**: ~30 minutes

---

## Phase 3: Manual Validation (1 ticket)

### TESTISO-1004: Create manual validation script
- **File**: `TESTISO-1004_create-manual-validation-script.md`
- **Agent**: general-implementation
- **Status**: ⏳ Not Started
- **Dependencies**: TESTISO-1001, 1002, 1003
- **Blocks**: TESTISO-1005
- **Summary**: Create `/workspace/scripts/validate-test-isolation.sh` to verify database isolation by running tests and comparing chunk counts
- **Plan Reference**: Phase 3 (lines 153-202)
- **Estimated Effort**: ~30 minutes

---

## Phase 4: CI/CD Integration (1 ticket)

### TESTISO-1005: Create GitHub Actions test workflow
- **File**: `TESTISO-1005_create-github-actions-test-workflow.md`
- **Agent**: github-actions-specialist
- **Status**: ⏳ Not Started
- **Dependencies**: TESTISO-1001, 1002, 1003, 1004
- **Blocks**: TESTISO-1006
- **Summary**: Create `.github/workflows/test.yml` with postgres-test service container, schema initialization, and TEST_MAPROOM_DATABASE_URL
- **Plan Reference**: Phase 4 (lines 205-267)
- **Estimated Effort**: ~45 minutes

---

## Phase 5: Documentation (1 ticket)

### TESTISO-1006: Update documentation
- **File**: `TESTISO-1006_update-documentation.md`
- **Agent**: general-implementation
- **Status**: ⏳ Not Started
- **Dependencies**: TESTISO-1001, 1002, 1003, 1004, 1005
- **Blocks**: None (final ticket)
- **Summary**: Update README.md and create comprehensive TEST_DATABASE_SETUP.md with troubleshooting, volume management, and workflow examples
- **Plan Reference**: Phase 5 (lines 270-323)
- **Estimated Effort**: ~1 hour

---

## Dependency Graph

```
TESTISO-1001 (Docker Infrastructure)
    ↓
TESTISO-1002 (vitest config)
    ↓
TESTISO-1003 (package.json)
    ↓
TESTISO-1004 (validation script)
    ↓
TESTISO-1005 (CI workflow)
    ↓
TESTISO-1006 (documentation)
```

**Execution Strategy**: Sequential execution required due to dependencies

---

## Completion Tracking

| Ticket | Task | Test | Verify | Commit | Status |
|--------|------|------|--------|--------|--------|
| TESTISO-1001 | ⏳ | ⏳ | ⏳ | ⏳ | Not Started |
| TESTISO-1002 | ⏳ | ⏳ | ⏳ | ⏳ | Not Started |
| TESTISO-1003 | ⏳ | ⏳ | ⏳ | ⏳ | Not Started |
| TESTISO-1004 | ⏳ | ⏳ | ⏳ | ⏳ | Not Started |
| TESTISO-1005 | ⏳ | ⏳ | ⏳ | ⏳ | Not Started |
| TESTISO-1006 | ⏳ | ⏳ | ⏳ | ⏳ | Not Started |

**Legend**: ⏳ Not Started | 🔄 In Progress | ✅ Complete | ❌ Failed

---

## Files Modified/Created by Project

### Modified Files
- `packages/maproom-mcp/config/docker-compose.yml` (TESTISO-1001)
- `packages/maproom-mcp/vitest.config.ts` (TESTISO-1002)
- `packages/maproom-mcp/package.json` (TESTISO-1003)
- `packages/maproom-mcp/README.md` (TESTISO-1006)

### Created Files
- `/workspace/scripts/validate-test-isolation.sh` (TESTISO-1004)
- `.github/workflows/test.yml` (TESTISO-1005)
- `docs/development/TEST_DATABASE_SETUP.md` (TESTISO-1006)

### No Changes Needed
- `packages/maproom-mcp/tests/helpers/database.ts` (already supports TEST_MAPROOM_DATABASE_URL!)

---

## Quality Gates

Before marking project complete, verify:

1. ✅ Two PostgreSQL containers running (dev: 5433, test: 5434)
2. ✅ Tests use TEST_MAPROOM_DATABASE_URL, dev uses MAPROOM_DATABASE_URL
3. ✅ Test data isolated from dev data (separate volumes)
4. ✅ `docker compose up && pnpm test` works out of the box
5. ✅ CI tests pass using isolated test database
6. ✅ Backward compatible (existing tests work without TEST_MAPROOM_DATABASE_URL)

---

## Project Execution Commands

### Review Tickets (Before Starting)
```bash
/review-tickets TESTISO
```

### Execute All Tickets
```bash
/work-on-project TESTISO
```

### Execute Single Ticket
```bash
/single-ticket TESTISO-1001  # Start with first ticket
/single-ticket TESTISO-1002  # Continue sequentially
# ... etc
```

### Manual Validation (After Implementation)
```bash
/workspace/scripts/validate-test-isolation.sh
```

---

## Planning Documents Reference

- **Analysis**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/analysis.md`
- **Architecture**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/architecture.md`
- **Plan**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/plan.md`
- **Quality Strategy**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/quality-strategy.md`
- **Security Review**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/security-review.md`
- **Project Review**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/project-review.md`
- **Review Updates**: `/workspace/.agents/projects/TESTISO_test-database-isolation/planning/review-updates.md`

---

## Total Estimated Effort

**Implementation**: ~3.75 hours
- Phase 1: 30 minutes
- Phase 2: 1 hour
- Phase 3: 30 minutes
- Phase 4: 45 minutes
- Phase 5: 1 hour

**Note**: Actual time may vary based on:
- Docker environment issues
- Existing data in databases
- CI/CD configuration complexity
- Documentation review iterations

---

**Last Updated**: 2025-11-20
**Ticket Creation**: Complete ✅
**Ready for Execution**: Yes ✅
