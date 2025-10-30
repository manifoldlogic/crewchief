# CFGVER: Config Version Management - Ticket Index

## Project Overview

**Goal:** Implement version-based configuration management to prevent config drift in Maproom MCP CLI

**Timeline:** 12-17 days (3 weeks)

**Status:** Ready for Implementation

---

## Phase 0: Prerequisites (CRITICAL - COMPLETE FIRST)

**Objective:** Establish testing infrastructure before any test tickets

### Setup Tickets

**CFGVER-0001: Testing Infrastructure Setup**
- **File:** `CFGVER-0001_testing-infrastructure.md`
- **Agent:** database-engineer
- **Summary:** Install and configure Vitest testing framework (REQUIRED for all test tickets)
- **Deliverables:** Vitest installed, vitest.config.js created, sample tests passing
- **Dependencies:** None (prerequisite for all test tickets)
- **Blocks:** CFGVER-1901, CFGVER-2902, CFGVER-3903, CFGVER-5001, CFGVER-5002
- **Estimated:** 2-3 hours
- **Priority:** CRITICAL - Must complete before ANY test ticket

**Phase 0 Total:** 1 ticket, 2-3 hours (0.5 day)

---

## Phase 1: Core Version Management (2-3 days)

**Objective:** Implement version tracking and detection logic

### Implementation Tickets

**CFGVER-1001: Version File Schema and Creation Logic**
- **File:** `CFGVER-1001_version-file-schema.md`
- **Agent:** database-engineer
- **Summary:** Create `.maproom-version` file schema with package version, file hashes, timestamps
- **Deliverables:** `createVersionFile()`, `computeFileHash()`, `readVersionFile()` functions
- **Dependencies:** None (first ticket)
- **Estimated:** 4-6 hours

**CFGVER-1002: Version Comparison Function**
- **File:** `CFGVER-1002_version-comparison.md`
- **Agent:** database-engineer
- **Summary:** Implement version comparison logic to detect first run, version mismatch, same version
- **Deliverables:** `needsConfigUpdate()` function with reason codes
- **Dependencies:** CFGVER-1001
- **Estimated:** 3-4 hours

**CFGVER-1003: File Integrity Checking**
- **File:** `CFGVER-1003_file-integrity.md`
- **Agent:** database-engineer
- **Summary:** Implement SHA-256 hash verification with TOCTOU and symlink attack prevention
- **Deliverables:** `verifyFileIntegrity()` function
- **Dependencies:** CFGVER-1001
- **Estimated:** 4-5 hours

### Testing Tickets

**CFGVER-1901: Unit Tests for Core Version Logic**
- **File:** `CFGVER-1901_unit-tests-core.md`
- **Agent:** unit-test-runner
- **Summary:** Comprehensive unit tests using Vitest + memfs for version detection, comparison, integrity
- **Deliverables:** Test file with 80%+ coverage
- **Dependencies:** CFGVER-0001 (testing infrastructure), CFGVER-1001, 1002, 1003
- **Estimated:** 4-6 hours

**Phase 1 Total:** 4 tickets, 15-21 hours (2-3 days)

---

## Phase 2: Safe Update Process (3-4 days)

**Objective:** Implement backup, update, and rollback mechanisms

### Implementation Tickets

**CFGVER-2001: Backup Creation Logic**
- **File:** `CFGVER-2001_backup-creation.md`
- **Agent:** database-engineer
- **Summary:** Create timestamped backups of all config files with symlink attack prevention
- **Deliverables:** `backupConfigs()` function with permissions 0o700/0o600
- **Dependencies:** CFGVER-1001
- **Estimated:** 4-6 hours

**CFGVER-2002: Config Update Logic**
- **File:** `CFGVER-2002_config-update.md`
- **Agent:** database-engineer
- **Summary:** Copy new configs from package template while preserving user .env file
- **Deliverables:** `copyNewConfigs()` function with hash computation
- **Dependencies:** CFGVER-1001, CFGVER-2001
- **Estimated:** 5-7 hours

**CFGVER-2003: Rollback Mechanism**
- **File:** `CFGVER-2003_rollback-mechanism.md`
- **Agent:** database-engineer
- **Summary:** Restore backup when update fails (the critical safety net)
- **Deliverables:** `rollbackConfigs()` function with error recovery
- **Dependencies:** CFGVER-2001
- **Estimated:** 4-5 hours

**CFGVER-2004: Backup Cleanup Logic**
- **File:** `CFGVER-2004_backup-cleanup.md`
- **Agent:** database-engineer
- **Summary:** Automatically remove old backups, keeping 5 most recent
- **Deliverables:** `cleanupOldBackups()` function with best-effort cleanup
- **Dependencies:** CFGVER-2001
- **Estimated:** 2-3 hours

### Testing Tickets

**CFGVER-2902: Integration Tests for Update Flow**
- **File:** `CFGVER-2902_integration-tests-update.md`
- **Agent:** integration-tester
- **Summary:** End-to-end tests for backup → update → rollback → cleanup
- **Deliverables:** Integration test file with 5 scenarios
- **Dependencies:** CFGVER-0001 (testing infrastructure), CFGVER-2001, 2002, 2003, 2004
- **Estimated:** 6-8 hours

**Phase 2 Total:** 5 tickets, 21-29 hours (3-4 days)

---

## Phase 3: Docker Integration (2-3 days)

**Objective:** Handle Docker containers during updates

### Implementation Tickets

**CFGVER-3001: Container Stop Logic**
- **File:** `CFGVER-3001_container-stop.md`
- **Agent:** docker-engineer
- **Summary:** Safe Docker container shutdown using execFile (no shell injection)
- **Deliverables:** `stopContainers()` function with 30-second timeout
- **Dependencies:** CFGVER-2001
- **Estimated:** 3-4 hours

**CFGVER-3002: Volume Cleanup Logic**
- **File:** `CFGVER-3002_volume-cleanup.md`
- **Agent:** docker-engineer
- **Summary:** Safe volume cleanup using strict label filtering
- **Deliverables:** `cleanupOldVolumes()` function, docker-compose.yml labels
- **Dependencies:** CFGVER-3001
- **Estimated:** 3-4 hours

**CFGVER-3003: Docker Error Handling**
- **File:** `CFGVER-3003_docker-error-handling.md`
- **Agent:** docker-engineer
- **Summary:** Comprehensive Docker error detection with actionable messages
- **Deliverables:** `checkDockerAvailable()` function with error scenarios
- **Dependencies:** CFGVER-3001
- **Estimated:** 4-5 hours

### Testing Tickets

**CFGVER-3903: Integration Tests for Docker Operations**
- **File:** `CFGVER-3903_integration-tests-docker.md`
- **Agent:** integration-tester
- **Summary:** Real Docker tests with graceful skip when unavailable
- **Deliverables:** Integration test file with container/volume scenarios
- **Dependencies:** CFGVER-0001 (testing infrastructure), CFGVER-3001, 3002, 3003
- **Estimated:** 5-7 hours

**Phase 3 Total:** 4 tickets, 15-20 hours (2-3 days)

---

## Phase 4: CLI Integration (1-2 days)

**Objective:** Integrate version management into CLI entry point

### Implementation Tickets

**CFGVER-4001: CLI Entry Point Integration**
- **File:** `CFGVER-4001_cli-entry-point.md`
- **Agent:** mcp-tools-engineer
- **Summary:** Integrate version checking into CLI startup flow
- **Deliverables:** Modified `cli.cjs` with automatic update check
- **Dependencies:** CFGVER-1002, CFGVER-2002
- **Estimated:** 2-3 hours

**CFGVER-4002: Progress Messages**
- **File:** `CFGVER-4002_progress-messages.md`
- **Agent:** mcp-tools-engineer
- **Summary:** Add clear, emoji-based progress messages during updates
- **Deliverables:** Console.log calls for each update step
- **Dependencies:** CFGVER-4001
- **Estimated:** 2-3 hours

**CFGVER-4003: Environment Variable Support**
- **File:** `CFGVER-4003_env-var-support.md`
- **Agent:** mcp-tools-engineer
- **Summary:** Add MAPROOM_CACHE_DIR env var for isolated testing
- **Deliverables:** Environment variable support in config-manager.js
- **Dependencies:** CFGVER-1001
- **Estimated:** 1-2 hours

### Testing Tickets

**CFGVER-4904: Manual Testing Checklist**
- **File:** `CFGVER-4904_manual-testing.md`
- **Agent:** mcp-tools-engineer
- **Summary:** Execute comprehensive manual testing on macOS and Linux
- **Deliverables:** Manual test report with results
- **Dependencies:** All Phase 1-4 tickets
- **Estimated:** 4-6 hours

**Phase 4 Total:** 4 tickets, 9-14 hours (1-2 days)

---

## Phase 5: Testing and Validation (3-4 days)

**Objective:** Comprehensive testing and validation

### Testing Tickets

**CFGVER-5001: Complete Unit Test Suite**
- **File:** `CFGVER-5001_complete-unit-tests.md`
- **Agent:** unit-test-runner
- **Summary:** Consolidate unit tests and achieve 80%+ coverage
- **Deliverables:** Complete test suite with coverage report
- **Dependencies:** CFGVER-0001 (testing infrastructure), CFGVER-1901, all Phase 1-4 tickets
- **Estimated:** 6-8 hours

**CFGVER-5002: Complete Integration Test Suite**
- **File:** `CFGVER-5002_complete-integration-tests.md`
- **Agent:** integration-tester
- **Summary:** Validate all critical paths with end-to-end tests
- **Deliverables:** Complete integration test suite
- **Dependencies:** CFGVER-0001 (testing infrastructure), CFGVER-2902, CFGVER-3903, all Phase 1-4 tickets
- **Estimated:** 6-8 hours

### Documentation Tickets

**CFGVER-5003: Documentation Updates**
- **File:** `CFGVER-5003_documentation-updates.md`
- **Agent:** documentation-engineer
- **Summary:** Update README, create troubleshooting guide, add JSDoc
- **Deliverables:** README.md, TROUBLESHOOTING.md, CHANGELOG.md, JSDoc comments
- **Dependencies:** All Phase 1-4 tickets
- **Estimated:** 6-8 hours

**CFGVER-5004: CI/CD Pipeline Updates**
- **File:** `CFGVER-5004_ci-cd-updates.md`
- **Agent:** database-engineer
- **Summary:** Add GitHub Actions workflow for automated testing
- **Deliverables:** `.github/workflows/test-config-manager.yml`, npm scripts
- **Dependencies:** CFGVER-5001, CFGVER-5002
- **Estimated:** 4-5 hours

**Phase 5 Total:** 4 tickets, 22-29 hours (3-4 days)

---

## Phase 6: Release and Monitoring (1 day)

**Objective:** Ship to production and monitor for issues

### Release Tickets

**CFGVER-6001: Version Bump and Package Preparation**
- **File:** `CFGVER-6001_version-bump.md`
- **Agent:** database-engineer
- **Summary:** Bump version to 1.2.3, create git tag, verify pre-release checklist
- **Deliverables:** Updated package.json, git tag v1.2.3
- **Dependencies:** ALL previous tickets (Phase 1-5 complete)
- **Estimated:** 2-3 hours

**CFGVER-6002: npm Publish and Release Notes**
- **File:** `CFGVER-6002_npm-publish.md`
- **Agent:** database-engineer + documentation-engineer
- **Summary:** Publish to npm registry and create GitHub release
- **Deliverables:** Package on npm, GitHub release with notes
- **Dependencies:** CFGVER-6001
- **Estimated:** 2-3 hours

**CFGVER-6003: Post-Release Monitoring**
- **File:** `CFGVER-6003_post-release-monitoring.md`
- **Agent:** documentation-engineer
- **Summary:** Monitor GitHub issues and npm stats for 48 hours
- **Deliverables:** Post-release report
- **Dependencies:** CFGVER-6002
- **Estimated:** 4-6 hours (spread over 48 hours)

**Phase 6 Total:** 3 tickets, 8-12 hours (1 day)

---

## Summary Statistics

| Phase | Tickets | Est. Hours | Est. Days |
|-------|---------|------------|-----------|
| Phase 0: Prerequisites | 1 | 2-3 | 0.5 |
| Phase 1: Core Version Management | 4 | 15-21 | 2-3 |
| Phase 2: Safe Update Process | 5 | 21-29 | 3-4 |
| Phase 3: Docker Integration | 4 | 15-20 | 2-3 |
| Phase 4: CLI Integration | 4 | 9-14 | 1-2 |
| Phase 5: Testing and Validation | 4 | 22-29 | 3-4 |
| Phase 6: Release and Monitoring | 3 | 8-12 | 1 |
| **Total** | **25** | **92-128** | **12-17** |

**Target Ship Date:** 3 weeks from project start

**CRITICAL:** Phase 0 (CFGVER-0001) MUST be completed before starting any test tickets

---

## Ticket Workflow

For each ticket:

1. **Assign** to primary agent specified in ticket
2. **Implement** following technical requirements and implementation notes
3. **Test** locally and verify acceptance criteria
4. **Verify** using verify-ticket agent (checks all acceptance criteria)
5. **Commit** using commit-ticket agent (creates Conventional Commit)

---

## Dependencies Graph

```
Phase 0 (CRITICAL - BLOCKS ALL TEST TICKETS):
  0001 (testing infrastructure)
    └─→ Blocks: 1901, 2902, 3903, 5001, 5002

Phase 1:
  1001 (version schema)
    ├─→ 1002 (version comparison)
    └─→ 1003 (file integrity)
  0001, 1001, 1002, 1003 → 1901 (unit tests)

Phase 2:
  1001 → 2001 (backup)
  1001, 2001 → 2002 (update)
  2001 → 2003 (rollback)
  2001 → 2004 (cleanup)
  0001, 2001, 2002, 2003, 2004 → 2902 (integration tests)

Phase 3:
  2001 → 3001 (container stop)
  3001 → 3002 (volume cleanup)
  3001 → 3003 (error handling)
  0001, 3001, 3002, 3003 → 3903 (integration tests)

Phase 4:
  1002, 2002 → 4001 (CLI integration)
  4001 → 4002 (progress messages)
  1001 → 4003 (env var support)
  All Phase 1-4 → 4904 (manual testing)

Phase 5:
  0001, 1901 + All Phase 1-4 → 5001 (complete unit tests)
  0001, 2902, 3903 + All Phase 1-4 → 5002 (complete integration tests)
  All Phase 1-4 → 5003 (documentation)
  5001, 5002 → 5004 (CI/CD)

Phase 6:
  All Phase 1-5 → 6001 (version bump)
  6001 → 6002 (npm publish)
  6002 → 6003 (monitoring)
```

---

## Critical Path

The following tickets are on the critical path (must complete before others can proceed):

0. **CFGVER-0001** - Testing infrastructure (CRITICAL - blocks ALL test tickets: 1901, 2902, 3903, 5001, 5002)
1. **CFGVER-1001** - Version file schema (blocks 1002, 1003, 2001, 4003)
2. **CFGVER-2001** - Backup creation (blocks 2002, 2003, 2004, 3001)
3. **CFGVER-2002** - Config update (blocks 4001)
4. **CFGVER-3001** - Container stop (blocks 3002, 3003)
5. **CFGVER-4001** - CLI integration (blocks 4002, 4904)
6. **All Phase 1-5** - Must complete before Phase 6 release

**Recommendation:** Start with CFGVER-0001 immediately, then prioritize critical path tickets to prevent bottlenecks.

---

## Planning References

- **Analysis:** `.agents/projects/CFGVER_config-version-management/planning/analysis.md`
- **Architecture:** `.agents/projects/CFGVER_config-version-management/planning/architecture.md`
- **Quality Strategy:** `.agents/projects/CFGVER_config-version-management/planning/quality-strategy.md`
- **Security Review:** `.agents/projects/CFGVER_config-version-management/planning/security-review.md`
- **Plan:** `.agents/projects/CFGVER_config-version-management/planning/plan.md`
- **README:** `.agents/projects/CFGVER_config-version-management/README.md`

---

## Success Criteria

**Functional:**
- Zero config drift incidents after release
- 100% success rate for first-run config creation
- 95%+ success rate for version updates
- 100% success rate for rollback when triggered

**Quality:**
- 80%+ code coverage for config-manager module
- All critical paths covered by integration tests
- Zero high-severity security issues
- All manual test cases passing

**User Experience:**
- Clear progress messages during updates
- Actionable error messages for failures
- No user intervention required for normal updates
- Positive user feedback (monitored post-release)

---

*All tickets are ready for implementation. Use `/work-on-project CFGVER` to begin sequential execution.*
