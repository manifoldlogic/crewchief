# WATCHFIX Ticket Index

## Project Overview

**Project**: WATCHFIX - Watch Change Detection Fix
**Slug**: WATCHFIX
**Total Tickets**: 6
**Estimated Total Effort**: 26 hours (2 weeks)

**Problem**: The maproom watch command misclassifies modified files as NEW files due to path format mismatches, causing indexing failures and infinite retry loops.

**Solution**: Create path normalization utility and fix processor_task to correctly classify modified files using ChangeDetector.

## Ticket Organization

### Phase 1: Foundation (4 hours)
- **WATCHFIX-1001**: Create Path Normalization Utility Module
  - Status: Not Started
  - Agent: rust-indexer-engineer
  - Priority: HIGH (blocks all other tickets)
  - Files: CREATE `path_utils.rs`, MODIFY `incremental/mod.rs`
  - Dependencies: None
  - Blocks: 1002, 1003

### Phase 2: Core Fix (6 hours)
- **WATCHFIX-1002**: Refactor processor_task to Fix Change Detection Logic
  - Status: Not Started
  - Agent: rust-indexer-engineer
  - Priority: CRITICAL
  - Files: MODIFY `indexer/mod.rs` (processor_task)
  - Dependencies: 1001
  - Blocks: 1005

### Phase 3: Processor Path Handling (4 hours)
- **WATCHFIX-1003**: Fix IncrementalProcessor Path Handling
  - Status: Not Started
  - Agent: rust-indexer-engineer
  - Priority: HIGH
  - Files: MODIFY `incremental/processor.rs` (3 methods)
  - Dependencies: 1001, 1002
  - Blocks: 1005

### Phase 4: Security & Performance (2 hours)
- **WATCHFIX-1004**: Add Security and Performance Safeguards
  - Status: Not Started
  - Agent: rust-indexer-engineer
  - Priority: MEDIUM
  - Files: MODIFY `incremental/processor.rs` (add size limits)
  - Dependencies: None
  - Blocks: None

### Phase 5: Integration Testing (8 hours)
- **WATCHFIX-1005**: Write Integration Tests for Watch Command Fix
  - Status: Not Started
  - Agent: rust-indexer-engineer
  - Priority: HIGH
  - Files: CREATE `tests/watch_integration.rs`, `tests/test_utils.rs`
  - Dependencies: 1002, 1003
  - Blocks: None

### Phase 6: Documentation & Polish (2 hours)
- **WATCHFIX-1006**: Documentation and Code Polish
  - Status: Not Started
  - Agent: rust-indexer-engineer
  - Priority: MEDIUM
  - Files: MODIFY all changed files (add doc comments)
  - Dependencies: 1001-1005 (all implementation complete)
  - Blocks: None

## Dependency Graph

```
1001 (Path Normalization)
 ├─→ 1002 (processor_task Fix)
 │    └─→ 1005 (Integration Tests)
 └─→ 1003 (Processor Path Handling)
      └─→ 1005 (Integration Tests)

1004 (Security) ─────────────────→ [Independent]

1005 (Integration Tests) ────────→ 1006 (Documentation)
```

## Execution Order

**Sequential execution (recommended):**
1. WATCHFIX-1001 (foundation - required first)
2. WATCHFIX-1002 (core fix - depends on 1001)
3. WATCHFIX-1003 (processor fix - depends on 1001, 1002)
4. WATCHFIX-1004 (security - can be done anytime, but fits here)
5. WATCHFIX-1005 (tests - depends on 1002, 1003)
6. WATCHFIX-1006 (docs - should be last)

**Parallel opportunities:**
- WATCHFIX-1004 can be done in parallel with 1002/1003 (independent)

## Success Criteria (Project-wide)

After all tickets complete:
- [x] Bug reproduced (3 files fail to index) ✅ DONE in investigation
- [ ] Bug fixed (3 files successfully indexed)
- [ ] Modified files classified as `ChangeType::Modified` (not New)
- [ ] Database timestamps updated correctly
- [ ] No infinite retry loops
- [ ] Performance < 1s per file
- [ ] All tests pass (unit + integration)
- [ ] No regressions in scan/upsert commands

## File Impact Summary

**Files Created:**
- `crates/maproom/src/incremental/path_utils.rs` (~150 lines)
- `crates/maproom/tests/watch_integration.rs` (~200 lines)
- `crates/maproom/tests/test_utils.rs` (~150 lines)

**Files Modified:**
- `crates/maproom/src/incremental/mod.rs` (+2 lines for module export)
- `crates/maproom/src/indexer/mod.rs` (~70 lines changed in processor_task)
- `crates/maproom/src/incremental/processor.rs` (~60 lines changed in 3 methods + size limits)
- `CHANGELOG.md` (if exists - add entry)

**Total Lines Changed**: ~600 lines (new + modified)

## Testing Strategy

**Unit Tests** (WATCHFIX-1001):
- Path normalization: 6-8 tests
- Target: 100% coverage

**Integration Tests** (WATCHFIX-1005):
- Multi-file modification: 1 test
- Single file modification: 1 test
- New file handling: 1 test (optional)
- Test utilities: ~10 helper functions

**Manual Testing** (after 1005):
- Run watch command
- Modify 3 files simultaneously
- Verify all 3 indexed in database
- Check logs for warnings
- Test scan/upsert (regression check)

## Agent Workflow

For each ticket:
```
1. rust-indexer-engineer implements
2. rust-indexer-engineer writes tests (if applicable)
3. unit-test-runner executes tests
4. If tests fail → return to step 1
5. verify-ticket checks acceptance criteria
6. If verification fails → return to step 1
7. commit-ticket creates commit
8. Move to next ticket
```

## References

**Planning Documents:**
- `README.md` - Project overview and context
- `planning/analysis.md` - Deep bug investigation (15k words)
- `planning/architecture.md` - Solution design (8k words)
- `planning/plan.md` - Phases and timeline (5k words)
- `planning/quality-strategy.md` - Testing approach (4k words)
- `planning/security-review.md` - Security assessment (4k words)
- `planning/agent-suggestions.md` - Agent assignments (1.5k words)

**Investigation Evidence:**
- Test scenario: 3 files modified simultaneously
- Log analysis: All detected, all failed, zero indexed
- Database verification: No timestamp updates
- Root cause: Path format mismatch in processor_task

## Quick Start

**To work on this project:**

1. **Single ticket**: `/single-ticket WATCHFIX-1001`
2. **All tickets**: `/work-on-project WATCHFIX`
3. **Review tickets**: `/review-tickets WATCHFIX`

**To verify progress:**

```bash
# Check ticket status
ls -la .agents/projects/WATCHFIX_watch-change-detection-fix/tickets/

# Check database state
docker exec maproom-postgres psql -U maproom -d maproom -c \
  "SELECT relpath, MAX(updated_at) FROM chunks GROUP BY relpath LIMIT 5"

# Test manually
cd /workspace
cargo build --release --bin crewchief-maproom
RUST_LOG=info ./packages/cli/bin/crewchief-maproom watch --repo crewchief --worktree main
```

## Timeline

**Week 1:**
- Days 1-2: Tickets 1001, 1002
- Days 3-4: Ticket 1003
- Day 5: Ticket 1004

**Week 2:**
- Days 1-2: Ticket 1005 (testing)
- Days 3-4: Ticket 1005 (completion)
- Day 5: Ticket 1006 (documentation)

**Total**: 10 working days

## Notes

- This is a **bug fix**, not a feature - minimize scope
- Focus on **correctness**, then performance
- **Test thoroughly** - this is critical path code
- **Document well** - subtle bugs need clear explanations
- No **breaking changes** - backwards compatible

---

**Status**: All tickets created, ready for execution
**Last Updated**: 2025-11-05
**Next Action**: Execute tickets sequentially starting with WATCHFIX-1001
