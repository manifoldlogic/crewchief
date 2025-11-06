# Project Plan: Watch Change Detection Fix

## Project Overview

Fix the maproom watch command's file change detection logic to correctly classify modified files and successfully re-index them. The bug causes all modified files to be misclassified as NEW files, leading to indexing failures and infinite retry loops.

## Success Criteria

1. ✅ Modified files are correctly classified as `ChangeType::Modified` (not New)
2. ✅ Multiple files changed simultaneously are all successfully indexed
3. ✅ Database timestamps and content are updated correctly
4. ✅ No infinite retry loops
5. ✅ Performance: < 1s per file for typical changes
6. ✅ All tests pass (unit + integration + E2E)
7. ✅ No regressions in scan/upsert commands

## Phases and Deliverables

### Phase 1: Foundation - Path Normalization Utility

**Goal**: Create robust path normalization function that prevents the root cause (path format mismatches).

**Agent**: rust-indexer-engineer

**Deliverables**:

1. **New Module**: `crates/maproom/src/incremental/path_utils.rs`
   - `normalize_to_relpath()` function
   - Clear documentation
   - Handles edge cases (trailing slashes, Windows paths)
   - Rejects paths outside repo root
   - Rejects parent directory components (`..`)

2. **Unit Tests**: `crates/maproom/src/incremental/path_utils.rs` (tests module)
   - Simple absolute → relative conversion
   - Nested paths
   - Paths outside repo (error case)
   - Paths with trailing slashes
   - Windows paths (if applicable)
   - Paths with `..` components (error case)
   - **Target**: 100% code coverage

3. **Module Export**: Update `crates/maproom/src/incremental/mod.rs`
   - Add `pub mod path_utils;`
   - Export normalize_to_relpath function

**Acceptance Criteria**:
- [ ] normalize_to_relpath() correctly converts paths
- [ ] Rejects malicious paths (traversal attempts)
- [ ] All unit tests pass
- [ ] Cross-platform compatible (Unix + Windows)

**Estimated Effort**: 4 hours

**Ticket**: WATCHFIX-1001

---

### Phase 2: Core Fix - processor_task Refactoring

**Goal**: Fix the processor_task logic to always use ChangeDetector for Modified events, eliminating misclassification.

**Agent**: rust-indexer-engineer

**Deliverables**:

1. **Refactored processor_task**: `crates/maproom/src/indexer/mod.rs` (lines 658-724)
   - Normalize path ONCE at event entry using `normalize_to_relpath()`
   - Use normalized relpath for all `get_file_id_by_path()` calls
   - ALWAYS call `ChangeDetector.detect_change()` when file_id found
   - Only fall back to `ChangeType::New` when file_id is None
   - Add logging for path normalization failures
   - Add logging for database lookup failures

2. **Updated Error Handling**:
   - Log warnings for paths outside repo (don't crash)
   - Log warnings for database errors (don't skip silently)
   - Continue processing other files on individual failures

3. **Code Comments**:
   - Document why we normalize paths
   - Explain the Modified vs New decision logic
   - Reference the bug this fixes

**Acceptance Criteria**:
- [ ] processor_task uses normalized paths for all database queries
- [ ] ChangeDetector called for all Modified events with valid file_id
- [ ] Error cases handled gracefully (log + skip, don't crash)
- [ ] Code compiles without warnings

**Estimated Effort**: 6 hours

**Ticket**: WATCHFIX-1002

---

### Phase 3: Processor Path Handling

**Goal**: Update IncrementalProcessor to handle both absolute and relative paths correctly.

**Agent**: rust-indexer-engineer

**Deliverables**:

1. **Updated index_new_file()**: `crates/maproom/src/incremental/processor.rs` (lines 191-257)
   - Normalize path to relpath for database queries
   - Use absolute path for filesystem operations
   - Update query to use correct relpath format
   - Add comment explaining path handling

2. **Updated update_file()**: Same file (lines 259-330)
   - Normalize path to relpath for database queries
   - Use absolute path for filesystem operations
   - Ensure consistency with index_new_file()

3. **Updated remove_file()**: Same file (if needed)
   - Check if path normalization needed

**Acceptance Criteria**:
- [ ] index_new_file() queries database with relpath, not absolute path
- [ ] update_file() uses correct path formats
- [ ] Filesystem operations use absolute paths
- [ ] Database operations use relative paths
- [ ] Code compiles without warnings

**Estimated Effort**: 4 hours

**Ticket**: WATCHFIX-1003

---

### Phase 4: Security & Performance Improvements

**Goal**: Add file size limits and optional symlink detection per security review.

**Agent**: rust-indexer-engineer

**Deliverables**:

1. **File Size Check**: `crates/maproom/src/incremental/processor.rs`
   - Check file size before reading in index_new_file()
   - Reject files > 10MB with warning
   - Return Ok() gracefully (skip file, don't error)

2. **Symlink Detection** (Optional): Same file
   - Detect symlinks using `fs::symlink_metadata()`
   - Log warning when indexing symlinks
   - Allow indexing (don't reject)

3. **Configuration** (Future-proof):
   - Add constant `MAX_FILE_SIZE_BYTES = 10 * 1024 * 1024`
   - Document in code comments
   - Easy to adjust later if needed

**Acceptance Criteria**:
- [ ] Files > 10MB are skipped with warning
- [ ] Symlinks logged but not rejected
- [ ] No panics or crashes on large files
- [ ] Performance unaffected for normal files

**Estimated Effort**: 2 hours

**Ticket**: WATCHFIX-1004

---

### Phase 5: Integration Testing

**Goal**: Write comprehensive integration tests that verify the fix works end-to-end.

**Agent**: rust-indexer-engineer

**Deliverables**:

1. **Multi-File Test**: `crates/maproom/tests/watch_integration.rs`
   - Setup test database with fixtures
   - Start watch command
   - Modify 3 files simultaneously
   - Wait for processing
   - Assert all 3 files re-indexed
   - Assert database timestamps updated
   - Cleanup

2. **Single File Test**: Same file
   - Modify single file
   - Assert ChangeType::Modified (not New)
   - Assert successful indexing

3. **New File Test**: Same file
   - Create truly new file
   - Assert ChangeType::New
   - Assert indexing succeeds (clarify who creates file record)

4. **Test Utilities**: `crates/maproom/tests/test_utils.rs`
   - `setup_test_db()` - Create and migrate test database
   - `seed_test_data()` - Insert repo/worktree/files
   - `modify_file()` - Helper to modify test files
   - `assert_chunks_updated()` - Verify database state
   - `cleanup_test_db()` - Clean up after tests

**Acceptance Criteria**:
- [ ] Multi-file test passes (3 files indexed)
- [ ] Single file test passes
- [ ] New file test passes (or documented as TODO if file creation unclear)
- [ ] Test utilities are reusable
- [ ] Tests run in < 10s total

**Estimated Effort**: 8 hours

**Ticket**: WATCHFIX-1005

---

### Phase 6: Documentation & Polish

**Goal**: Document the fix, update comments, and ensure code is maintainable.

**Agent**: rust-indexer-engineer

**Deliverables**:

1. **Code Comments**: Review all changed files
   - Add doc comments to public functions
   - Explain non-obvious logic
   - Reference security considerations
   - Link to GitHub issue (if exists)

2. **Module Documentation**: Update lib.rs or relevant module docs
   - Document path normalization strategy
   - Explain when to use absolute vs relative paths
   - Note the bug this fix addresses

3. **CHANGELOG**: Update if project uses one
   - Add entry: "fix(watch): correct file change detection for modified files"
   - Explain impact: "Watch now correctly re-indexes modified files"

4. **README** (if watch has dedicated docs):
   - Update any watch command documentation
   - Note file size limits (10MB)
   - Mention symlink behavior

**Acceptance Criteria**:
- [ ] All public functions have doc comments
- [ ] Code is self-documenting
- [ ] CHANGELOG updated
- [ ] No TODO comments left unaddressed

**Estimated Effort**: 2 hours

**Ticket**: WATCHFIX-1006

---

## Risk Mitigation

### Risk 1: Async Architecture Complexity

**Mitigation**: Use existing async patterns, don't reinvent. Test thoroughly.

**Contingency**: If async changes cause deadlocks, simplify to synchronous path normalization.

### Risk 2: Database Path Format Unknown

**Mitigation**: Query database before implementing to confirm path format.

**Contingency**: If path format varies, handle both formats in queries.

### Risk 3: File Record Creation Unclear

**Mitigation**: Investigate during Phase 1 how new files get file records.

**Contingency**: If unclear, document as limitation and create separate ticket.

### Risk 4: Test Environment Setup

**Mitigation**: Use docker-compose for PostgreSQL, ensure CI has database service.

**Contingency**: If CI database fails, run tests locally and document manual test results.

### Risk 5: Windows Path Compatibility

**Mitigation**: Test path normalization on Windows paths in unit tests.

**Contingency**: If Windows fails, document as known limitation, fix in follow-up.

## Timeline

```
Week 1:
├─ Day 1-2: Phase 1 (Path Normalization)
├─ Day 3-4: Phase 2 (processor_task Fix)
└─ Day 5:   Phase 3 (Processor Path Handling)

Week 2:
├─ Day 1-2: Phase 4 (Security/Performance)
├─ Day 3-4: Phase 5 (Integration Testing)
└─ Day 5:   Phase 6 (Documentation)

Total: 2 weeks (10 working days)
```

**Critical Path**: Phase 2 depends on Phase 1. All other phases can proceed in order.

## Testing Strategy Summary

- **Phase 1**: Unit tests for path normalization (100% coverage)
- **Phase 2**: Compile-time verification (Rust type system)
- **Phase 3**: Compile-time verification
- **Phase 4**: Security tests (large files, symlinks)
- **Phase 5**: Integration tests (end-to-end watch workflow)
- **Phase 6**: Manual regression testing (scan/upsert)

**Total Test Count**: ~15-20 tests (10 unit, 5 integration, 3-5 manual)

## Deployment Plan

1. **Build**: `cargo build --release --bin crewchief-maproom`
2. **Platform Binaries**: Run `./scripts/build-and-package.sh` for all platforms
3. **Copy**: Replace binaries in `packages/cli/bin/<platform>/`
4. **Test**: Run maproom commands manually to verify
5. **Commit**: Single commit with all changes (or per-phase commits)
6. **Push**: To main branch after verification

**Rollback**: Revert commit if regressions found, restore previous binaries.

## Success Metrics

**Functional**:
- [x] Bug reproduced (3 files fail to index) ✅ DONE
- [ ] Bug fixed (3 files successfully index)
- [ ] No regression in scan command
- [ ] No regression in upsert command

**Quality**:
- [ ] 100% of unit tests pass
- [ ] 100% of integration tests pass
- [ ] Code coverage > 85% for changed files

**Performance**:
- [ ] No performance regression vs baseline
- [ ] < 1s per file indexing time
- [ ] < 10s for 10 files simultaneously

## Open Questions (To Resolve During Implementation)

1. **File record creation**: Who creates maproom.files records during watch? Need to investigate.
2. **Temporary file handling**: Do we need special logic for .tmp → rename sequences?
3. **Windows testing**: Can we test Windows paths in Linux CI?
4. **Batch optimization**: Should we batch get_file_id_by_path() calls?

## Agent Assignments

| Phase | Agent | Effort | Ticket |
|-------|-------|--------|--------|
| 1. Path Normalization | rust-indexer-engineer | 4h | WATCHFIX-1001 |
| 2. processor_task Fix | rust-indexer-engineer | 6h | WATCHFIX-1002 |
| 3. Processor Paths | rust-indexer-engineer | 4h | WATCHFIX-1003 |
| 4. Security/Performance | rust-indexer-engineer | 2h | WATCHFIX-1004 |
| 5. Integration Tests | rust-indexer-engineer | 8h | WATCHFIX-1005 |
| 6. Documentation | rust-indexer-engineer | 2h | WATCHFIX-1006 |
| **Total** | | **26h** | **6 tickets** |

Testing/verification for each ticket handled by standard workflow agents (unit-test-runner, verify-ticket, commit-ticket).

## Next Steps

1. ✅ Create project planning documents (DONE)
2. **Create individual tickets** from this plan (run /create-project-tickets WATCHFIX)
3. **Review tickets** for completeness (run /review-tickets WATCHFIX)
4. **Execute tickets** sequentially (run /work-on-project WATCHFIX)
5. **Manual testing** after all tickets complete
6. **Deploy** to production (replace binaries)
7. **Monitor** for regressions in real usage

## Conclusion

This is a well-scoped, achievable project with clear deliverables and success criteria. The fix is architecturally sound, minimally invasive, and thoroughly tested. Estimated completion: 2 weeks with a single rust-indexer-engineer agent.

Risk is low, impact is high (fixes critical bug), and the solution maintains the existing architecture while preventing future path-related issues.
