# Config Version Management - Implementation Plan

## Project Overview

Implement version-based configuration management to prevent config drift in the Maproom MCP CLI. This ensures users always have up-to-date configurations when running `npx -y @crewchief/maproom-mcp@latest`.

**Goal:** Zero config drift incidents, automatic updates, safe rollback on failure.

## Implementation Phases

### Phase 1: Core Version Management (Week 1)

**Objective:** Implement version tracking and detection logic

**Deliverables:**
1. Version file schema and creation logic
2. Version comparison function
3. Update detection logic
4. File integrity checking (SHA-256 hashing)

**Acceptance Criteria:**
- Can detect when config needs update (version mismatch, missing file, corrupted file)
- Can create version file with accurate metadata
- Can compute and verify file hashes
- Unit tests for all core logic (80% coverage)

**Estimated Effort:** 2-3 days

**Agent Assignments:**
- **Implementation:** database-engineer (handles file management and integrity logic)
- **Testing:** unit-test-runner (creates and runs unit tests)
- **Review:** code-reviewer (verifies correctness and edge cases)

---

### Phase 2: Safe Update Process (Week 1-2)

**Objective:** Implement backup, update, and rollback mechanisms

**Deliverables:**
1. Backup creation logic (copy all configs to timestamped directory)
2. Config update logic (copy new files from package)
3. Rollback mechanism (restore from backup on failure)
4. Cleanup logic (remove old backups, keep last 5)

**Acceptance Criteria:**
- Can create backup before update
- Can copy new config files
- Can rollback on failure
- Can cleanup old backups
- Integration tests for update flow

**Estimated Effort:** 3-4 days

**Agent Assignments:**
- **Implementation:** database-engineer (file operations, backup logic)
- **Testing:** integration-tester (end-to-end update scenarios)
- **Review:** code-reviewer (verify safety and error handling)

---

### Phase 3: Docker Integration (Week 2)

**Objective:** Handle Docker containers during updates

**Deliverables:**
1. Container stop logic (docker compose down)
2. Volume cleanup logic (prune with filters)
3. Error handling for Docker not running
4. Container restart verification

**Acceptance Criteria:**
- Stops containers before update
- Cleans up old volumes safely
- Handles Docker not available gracefully
- Doesn't affect user's other containers
- Integration tests with Docker

**Estimated Effort:** 2-3 days

**Agent Assignments:**
- **Implementation:** docker-engineer (Docker operations and cleanup)
- **Testing:** integration-tester (Docker scenarios)
- **Review:** code-reviewer (verify container safety)

---

### Phase 4: CLI Integration (Week 2)

**Objective:** Integrate version management into CLI entry point

**Deliverables:**
1. Update CLI entry point to check version on startup
2. User-friendly progress messages
3. Error messages with recovery steps
4. Environment variable support for cache directory (testing)

**Acceptance Criteria:**
- CLI checks for updates on every run
- Shows clear progress messages during update
- Provides actionable error messages
- Can run in test mode (custom cache directory)
- Manual testing checklist complete

**Estimated Effort:** 1-2 days

**Agent Assignments:**
- **Implementation:** mcp-tools-engineer (CLI integration)
- **Testing:** integration-tester (manual testing scenarios)
- **Review:** code-reviewer (UX and error handling)

---

### Phase 5: Testing and Validation (Week 3)

**Objective:** Comprehensive testing and validation

**Deliverables:**
1. Complete unit test suite (80%+ coverage)
2. Integration test suite (all critical paths)
3. Manual testing on macOS and Linux
4. Documentation updates
5. CI/CD pipeline updates

**Acceptance Criteria:**
- All unit tests pass
- All integration tests pass
- Manual testing checklist complete
- CI pipeline green
- Documentation updated

**Estimated Effort:** 3-4 days

**Agent Assignments:**
- **Unit Testing:** unit-test-runner (vitest tests)
- **Integration Testing:** integration-tester (end-to-end scenarios)
- **Manual Testing:** mcp-tools-engineer (user experience validation)
- **Documentation:** documentation-engineer (update docs)

---

### Phase 6: Release and Monitoring (Week 3)

**Objective:** Ship to production and monitor for issues

**Deliverables:**
1. Version bump (patch: 1.2.3)
2. Publish to npm registry
3. Update GitHub release notes
4. Monitor for user-reported issues

**Acceptance Criteria:**
- Package published to npm
- Release notes accurate
- No critical issues within 48 hours
- User feedback collected

**Estimated Effort:** 1 day

**Agent Assignments:**
- **Release:** database-engineer (version bump, npm publish)
- **Documentation:** documentation-engineer (release notes)
- **Monitoring:** None (manual user feedback collection)

---

## Dependencies and Blockers

### External Dependencies

- **Docker** - Must be available for integration tests
- **npm Registry** - For publishing package
- **GitHub Actions** - For CI/CD pipeline

### Internal Dependencies

- Phase 2 depends on Phase 1 (version detection needed for update logic)
- Phase 3 depends on Phase 2 (Docker cleanup happens during update)
- Phase 4 depends on Phase 1-3 (CLI integration needs all components)
- Phase 5 depends on Phase 1-4 (testing validates complete system)
- Phase 6 depends on Phase 5 (can't release without passing tests)

### Known Blockers

None identified. If Docker integration proves complex, Phase 3 can be simplified (skip volume cleanup initially).

---

## Risk Management

### High Risks

1. **Rollback Failure** - Backup corrupted or missing
   - **Mitigation:** Verify backup immediately after creation
   - **Contingency:** Document manual recovery steps

2. **Docker Conflicts** - Update affects user's other containers
   - **Mitigation:** Use label filters for cleanup
   - **Contingency:** Document volume recovery

3. **Permission Errors** - Can't write to ~/.maproom-mcp/
   - **Mitigation:** Check permissions before update
   - **Contingency:** Clear error message with fix command

### Medium Risks

1. **Concurrent Updates** - Two terminals running npx simultaneously
   - **Mitigation:** Document that it's not supported (acceptable for MVP)
   - **Future:** Add file locking

2. **Disk Space** - Not enough space for backup
   - **Mitigation:** Check available space (future enhancement)
   - **Contingency:** Error message mentions disk space

3. **Test Coverage** - Edge cases not covered
   - **Mitigation:** Focus on critical paths (first run, update, rollback)
   - **Acceptance:** Document known limitations

---

## Success Metrics

### Functional Metrics

- **Zero config drift incidents** reported by users after release
- **100% success rate** for first-run config creation
- **95%+ success rate** for version updates
- **100% success rate** for rollback when triggered

### Quality Metrics

- **80%+ code coverage** for config-manager module
- **All critical paths covered** by integration tests
- **Zero high-severity security issues** in security review
- **All manual test cases passing**

### User Experience Metrics

- **Clear progress messages** during update (validated by manual testing)
- **Actionable error messages** for common failures
- **No user intervention required** for normal updates
- **Positive user feedback** (monitor GitHub issues/discussions)

---

## Timeline Summary

| Phase | Duration | Completion Criteria |
|-------|----------|-------------------|
| 1. Core Version Management | 2-3 days | Version detection working |
| 2. Safe Update Process | 3-4 days | Update with rollback working |
| 3. Docker Integration | 2-3 days | Container management working |
| 4. CLI Integration | 1-2 days | CLI integration complete |
| 5. Testing and Validation | 3-4 days | All tests passing |
| 6. Release and Monitoring | 1 day | Package published |
| **Total** | **12-17 days** | **MVP shipped** |

**Target Ship Date:** 3 weeks from project start

---

## Post-MVP Enhancements

### Phase 2 Features (Future)

1. **File Locking** - Prevent concurrent updates
2. **Config Migrations** - Support schema changes between versions
3. **Partial Updates** - Only update changed files
4. **Rollback Command** - Manual rollback: `npx maproom-mcp rollback`
5. **Update Notifications** - Show changelog when updating
6. **Dry Run Mode** - Preview updates: `--dry-run`
7. **Encrypted Backups** - Encrypt backups at rest
8. **Audit Logging** - Log all config operations

**Estimated Effort:** 1-2 weeks per feature

**Priority:** Based on user feedback and reported issues

---

## Resource Requirements

### Team

- **Primary Developer:** 1 full-time
- **Reviewers:** 1 part-time (code review)
- **Testers:** Automated + manual (primary developer)

### Tools and Infrastructure

- **Development:** Node.js 18+, Docker, Git
- **Testing:** Vitest, memfs, real Docker containers
- **CI/CD:** GitHub Actions (existing)
- **Publishing:** npm registry access

### Documentation

- **User-Facing:** Update README.md, add troubleshooting guide
- **Developer-Facing:** Code comments, JSDoc annotations
- **Process:** This plan, architecture, quality strategy, security review

---

## Communication Plan

### Internal Updates

- **Daily:** Quick status update in team channel
- **Weekly:** Progress report (phases complete, blockers, next steps)
- **Phase Completion:** Demo to team, review feedback

### External Communication

- **Release Notes:** Detailed changelog with upgrade instructions
- **GitHub Issue:** Close config drift issues with reference to fix
- **npm Deprecation:** Deprecate old versions with security issues

---

## Acceptance and Sign-Off

### Definition of Done

A phase is complete when:
1. ✅ All deliverables implemented
2. ✅ All acceptance criteria met
3. ✅ Tests passing (unit + integration)
4. ✅ Code reviewed and approved
5. ✅ Documentation updated

Project is complete when:
1. ✅ All phases complete
2. ✅ Package published to npm
3. ✅ Release notes published
4. ✅ Zero critical issues within 48 hours

---

## References

- **Analysis:** `analysis.md` - Problem space and industry solutions
- **Architecture:** `architecture.md` - Technical design and components
- **Quality Strategy:** `quality-strategy.md` - Testing approach
- **Security Review:** `security-review.md` - Security considerations
