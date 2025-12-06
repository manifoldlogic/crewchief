# WTSCAN Ticket Index

## Project: Worktree Use Auto-Scan Control

**Total Tickets**: 4 (3 in Phase 1, 1 in Phase 2)

## Phase 1: Config Schema and Core Logic (4-6 hours)

### WTSCAN-1001: Add Config Schema Field
**File**: `WTSCAN-1001_add-config-schema-field.md`
**Agent**: typescript-dev
**Estimated**: 1-2 hours
**Status**: Not Started

**Summary**: Add `autoScanOnWorktreeUse` boolean field to `WorktreeSchema` with default value of `false` and comprehensive unit tests for validation.

**Dependencies**: None (first ticket)

---

### WTSCAN-1002: Implement Conditional Scan Logic
**File**: `WTSCAN-1002_implement-conditional-scan-logic.md`
**Agent**: typescript-dev
**Estimated**: 2-3 hours
**Status**: Not Started

**Summary**: Modify `WorktreeService.createWorktree()` to load config once and conditionally call `runMaproomScan()` based on the config field, with graceful error handling.

**Dependencies**: WTSCAN-1001 (requires config schema field)

---

### WTSCAN-1003: Add Integration Tests
**File**: `WTSCAN-1003_add-integration-tests.md`
**Agent**: typescript-dev
**Estimated**: 1-2 hours
**Status**: Not Started

**Summary**: Add comprehensive integration tests covering all auto-scan scenarios: default (off), explicitly disabled, explicitly enabled, and error handling.

**Dependencies**: WTSCAN-1001, WTSCAN-1002 (requires implementation complete)

---

## Phase 2: Documentation and Breaking Change Communication (2-4 hours)

### WTSCAN-2001: Add Documentation and Migration Guide
**File**: `WTSCAN-2001_add-documentation-and-migration-guide.md`
**Agent**: docs-writer
**Estimated**: 2-4 hours
**Status**: Not Started

**Summary**: Document the auto-scan configuration feature, explain trade-offs, provide migration guide for existing users, and create prominent changelog entry for breaking change.

**Dependencies**: WTSCAN-1001, WTSCAN-1002, WTSCAN-1003 (Phase 1 must be complete)

---

## Execution Order

**Sequential Execution** (no parallel work possible):
1. WTSCAN-1001 → WTSCAN-1002 → WTSCAN-1003 (Phase 1)
2. WTSCAN-2001 (Phase 2, after Phase 1 complete)

**Critical Path**: All tickets are on the critical path (no parallel work)

---

## Success Criteria

**Phase 1 Complete When**:
- [ ] All 3 tickets verified and committed
- [ ] Config field exists and validates correctly
- [ ] Conditional scan logic implemented and tested
- [ ] All tests pass (new and existing)

**Phase 2 Complete When**:
- [ ] Documentation complete and accurate
- [ ] Migration guide clear and actionable
- [ ] Changelog entry prominent and comprehensive
- [ ] verify-ticket confirms documentation quality

**Project Complete When**:
- [ ] All 4 tickets complete
- [ ] Breaking change clearly communicated
- [ ] Ready for release with confidence
- [ ] Performance improvement verified (worktree creation <1s by default)

---

## Key Metrics

**Functional**:
- Config validates boolean field correctly
- Scan runs only when enabled
- Error handling prevents failures

**Performance**:
- Worktree creation: <1 second (default, down from 5-30s)
- Test suite: No regression in execution time

**Documentation**:
- Migration requires one line of config
- Self-service migration possible in <5 minutes
- Breaking change clearly noted in multiple places

---

## Risk Summary

| Risk | Level | Mitigation |
|------|-------|------------|
| Breaking change impacts users | Medium | Clear migration docs, trivial config fix |
| Config validation edge cases | Low | Zod handles validation, comprehensive tests |
| Test coverage gaps | Low | Follow existing patterns, verify all paths |
| Users miss migration guide | Medium | Prominent changelog, example in README |

---

## Notes

- This is a simple, focused change: one config field + conditional logic
- Total estimated time: 6-10 hours (1-2 days)
- Breaking change by design - necessary for better UX
- All existing functionality preserved (opt-in model)
- Foundation for future enhancements (CLI flags, smart defaults, etc.)
