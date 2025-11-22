# Test Database Isolation

**Project**: TESTISO
**Status**: Planning Complete, Ready for Implementation
**Created**: 2025-11-20

## Overview

Set up isolated test database infrastructure that runs alongside the development database without interference. This enables true test isolation while maintaining developer ergonomics.

## Problem Statement

Currently, development and test environments share the same PostgreSQL database instance, causing:
- Data contamination between dev and test workflows
- State leakage from failed tests
- Cannot develop and run tests simultaneously
- Lack of production parity (production uses isolated databases)

## Solution

Implement dual-database architecture with:
- **Development Database**: Port 5433, database `maproom`
- **Test Database**: Port 5434, database `maproom_test`
- **Single Docker Compose**: One `docker compose up` starts both
- **Environment Variable Priority**: `TEST_MAPROOM_DATABASE_URL || MAPROOM_DATABASE_URL`

## Key Benefits

✅ Tests and development can run in parallel
✅ Clean test database state
✅ Production parity
✅ Backward compatible (existing tests still work)
✅ Zero-config experience (`docker compose up && pnpm test`)

## Planning Documents

### 📊 [Analysis](planning/analysis.md)
Current state assessment, gap identification, and industry solution research.

**Key Findings**:
- Test helpers already support TEST_MAPROOM_DATABASE_URL (excellent!)
- Vitest and package.json hardcode dev database (need fixes)
- No test-specific PostgreSQL service exists
- Recommended: Single compose file with both databases

### 🏗️ [Architecture](planning/architecture.md)
Technical design decisions and component architecture.

**Key Decisions**:
- Single docker-compose.yml with dual databases (better DX)
- Port allocation: 5433 (dev), 5434 (test)
- Separate volumes for data isolation
- Same schema (init.sql) for both databases

### 🧪 [Quality Strategy](planning/quality-strategy.md)
Testing approach and validation milestones.

**Approach**:
- Manual validation script for isolation verification
- Smoke tests for configuration correctness
- CI integration validation
- Backward compatibility testing

### 🔒 [Security Review](planning/security-review.md)
Security analysis for development infrastructure.

**Conclusion**: No new security risks
- Development/test infrastructure only
- No sensitive data in test databases
- Follows Docker Compose security conventions

### 📋 [Implementation Plan](planning/plan.md)
Comprehensive implementation plan with phases and ticket breakdown.

**Phases**:
1. Docker Infrastructure (add postgres-test)
2. Test Configuration (vitest, package.json)
3. Manual Validation (verification script)
4. CI/CD Integration (GitHub Actions)
5. Documentation (README, guides)

## Tickets

### Phase 1: Docker Infrastructure
- **TESTISO-1001**: Add postgres-test service to Docker Compose

### Phase 2: Test Configuration
- **TESTISO-1002**: Update vitest configuration for TEST_MAPROOM_DATABASE_URL
- **TESTISO-1003**: Update package.json test scripts

### Phase 3: Manual Validation
- **TESTISO-1004**: Create manual validation script

### Phase 4: CI/CD Integration
- **TESTISO-1005**: Configure CI to use test database

### Phase 5: Documentation
- **TESTISO-1006**: Update documentation

## Success Criteria

Implementation complete when:
- ✅ Two PostgreSQL containers running (dev: 5433, test: 5434)
- ✅ Tests use TEST_MAPROOM_DATABASE_URL, dev uses MAPROOM_DATABASE_URL
- ✅ Test data isolated from dev data (separate volumes)
- ✅ `docker compose up && pnpm test` works out of the box
- ✅ CI tests pass using isolated test database
- ✅ Backward compatible (existing tests work without TEST_MAPROOM_DATABASE_URL)

## Files Impacted

### Modified
- `packages/maproom-mcp/config/docker-compose.yml` - Add postgres-test service
- `packages/maproom-mcp/vitest.config.ts` - Use TEST_MAPROOM_DATABASE_URL with fallback
- `packages/maproom-mcp/package.json` - Update test scripts
- `.github/workflows/test.yml` - Add postgres-test service to CI
- `packages/maproom-mcp/README.md` - Document database setup

### Created
- `/workspace/scripts/validate-test-isolation.sh` - Manual validation script (project root)
- `docs/development/TEST_DATABASE_SETUP.md` - Test setup guide

### No Changes Needed
- `packages/maproom-mcp/tests/helpers/database.ts` - Already supports TEST_MAPROOM_DATABASE_URL!

## Timeline Estimate

**Total**: 6 tickets, ~3.75 hours of implementation work

**Breakdown**:
- Phase 1: 1 ticket, ~30 min
- Phase 2: 2 tickets, ~1 hour
- Phase 3: 1 ticket, ~30 min
- Phase 4: 1 ticket, ~45 min
- Phase 5: 1 ticket, ~1 hour

## Dependencies

Sequential execution required:
```
Docker Infrastructure
    ↓
Test Configuration
    ↓
Manual Validation
    ↓
CI/CD Integration
    ↓
Documentation
```

## Risk Mitigation

**Low Risk Project** - Infrastructure changes only, backward compatible

**Identified Risks**:
- Port 5434 in use → Check before starting, use alternative port
- Schema drift → Both use same init.sql
- Test failures → Manual validation catches issues early
- CI flakiness → Health checks ensure readiness

## Next Steps

1. Review planning documents
2. Run `/create-project-tickets TESTISO` to generate ticket files
3. Run `/review-tickets TESTISO` to validate ticket quality
4. Run `/work-on-project TESTISO` to execute all tickets sequentially

## Related Documentation

- [Docker Compose Configuration](../../packages/maproom-mcp/config/docker-compose.yml)
- [Maproom MCP README](../../packages/maproom-mcp/README.md)
- [Database Architecture](../../docs/architecture/DATABASE_ARCHITECTURE.md)

---

**Project Lead**: Claude
**Complexity**: Low (Infrastructure configuration)
**Impact**: High (Enables reliable test isolation)
