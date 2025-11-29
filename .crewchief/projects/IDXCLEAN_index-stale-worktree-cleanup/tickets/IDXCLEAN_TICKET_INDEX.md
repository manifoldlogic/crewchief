# IDXCLEAN Ticket Index

**Project**: Index Stale Worktree Cleanup
**Project Slug**: IDXCLEAN
**Total Tickets**: 18 (across 5 phases)
**Status**: All tickets created ✅ (IDXCLEAN-3005 added for SQLite migration)

---

## Phase 1: Core Cleanup Infrastructure (Week 1)

**Goal**: Build foundational modules for detection and deletion

| Ticket ID | Title | Agent | Status | Files |
|-----------|-------|-------|--------|-------|
| IDXCLEAN-1001 | Stale Detection Module | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/db/cleanup.rs` (new) |
| IDXCLEAN-1002 | Safe Deletion Module | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/db/cleanup.rs` (extend) |
| IDXCLEAN-1003 | Data Models and Error Types | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/db/cleanup.rs` (extend) |

**Dependencies**: None (foundational)
**Risk Level**: Low (no user-facing changes)

---

## Phase 2: CLI Command Interface (Week 1-2)

**Goal**: Expose cleanup via maproom CLI with complete main.rs integration

| Ticket ID | Title | Agent | Status | Files |
|-----------|-------|-------|--------|-------|
| IDXCLEAN-2001 | CLI Subcommand Structure | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/cli/commands/db.rs` |
| IDXCLEAN-2002 | CLI Execution Logic | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/cli/commands/db.rs` |
| IDXCLEAN-2003 | User Output Formatting | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/cli/commands/db.rs` |
| IDXCLEAN-2004 | Integrate with main.rs | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/main.rs` |

**Dependencies**: Phase 1 complete
**Risk Level**: Medium (user-facing, data deletion)

---

## Phase 3: Integration Testing and Safety Validation (Week 2)

**Goal**: Ensure cleanup is safe and correct

**⚠️ STATUS**: Tests exist but need migration from PostgreSQL to SQLite (IDXCLEAN-3005)

| Ticket ID | Title | Agent | Status | Files |
|-----------|-------|-------|--------|-------|
| IDXCLEAN-3001 | Detection Accuracy Tests | integration-tester | ⚠️ Needs Migration | `crates/maproom/tests/cleanup_detection_test.rs` |
| IDXCLEAN-3002 | Deletion Safety Tests | integration-tester | ⚠️ Needs Migration | `crates/maproom/tests/cleanup_deletion_test.rs` |
| IDXCLEAN-3003 | CLI Integration Tests | integration-tester | ⚠️ Needs Migration | `crates/maproom/tests/cleanup_cli_test.rs` |
| IDXCLEAN-3004 | Manual Validation on Staging | integration-tester | 🟡 Pending | This ticket (validation report) |
| **IDXCLEAN-3005** | **Migrate Integration Tests to SQLite** | rust-indexer-engineer | 🔴 **Blocker** | All test files above |

**Dependencies**: Phase 2 complete, IDXCLEAN-3005 must be done first
**Risk Level**: Medium (straightforward migration)

**Critical Tests** (after SQLite migration):
- Multi-worktree chunk safety (Scenario 4) - uses `chunk_worktrees` junction table
- Garbage collection accuracy (Scenario 5)
- Transaction rollback verification

---

## Phase 4: Watch Integration (Week 3+) [Optional Enhancement]

**Goal**: Automatic cleanup during watch command

| Ticket ID | Title | Agent | Status | Files |
|-----------|-------|-------|--------|-------|
| IDXCLEAN-4001 | Startup Cleanup Integration | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/indexer/mod.rs` |
| IDXCLEAN-4002 | Periodic Cleanup via Status Task | rust-indexer-engineer | 🟡 Pending | `crates/maproom/src/indexer/mod.rs` |
| IDXCLEAN-4003 | Configuration Documentation and Testing | rust-indexer-engineer | 🟡 Pending | `README.md`, `tests/watch_cleanup_test.rs` |

**Dependencies**: Phase 1 complete (uses cleanup modules)
**Risk Level**: Low (minimal changes, non-blocking, no refactoring needed)
**Timeline**: 2-4 days (simple integration, well-understood hooks)

**Watch Analysis Complete**: ✅ No refactoring required, integration points identified at lines ~1140 and ~1432

---

## Phase 5: Production Deployment (Week 4)

**Goal**: Deploy to production with monitoring

| Ticket ID | Title | Agent | Status | Files |
|-----------|-------|-------|--------|-------|
| IDXCLEAN-5001 | Documentation Updates | rust-indexer-engineer | 🟡 Pending | `README.md`, `CHANGELOG.md`, `docs/` |
| IDXCLEAN-5002 | Deployment Procedure | verify-ticket | 🟡 Pending | `docs/deployment-cleanup.md` (new) |
| IDXCLEAN-5003 | Production Verification | verify-ticket | 🟡 Pending | This ticket (verification report) |

**Dependencies**: All Phase 1-4 tickets complete
**Risk Level**: Medium (production deployment)

---

## Execution Order

### Sequential (Must Complete in Order)

**Week 1:**
1. Phase 1: IDXCLEAN-1001 → 1002 → 1003
2. Phase 2: IDXCLEAN-2001 → 2002 → 2003 → 2004

**Week 2:**
3. Phase 3: IDXCLEAN-3001, 3002, 3003 (parallel) → 3004 (after all pass)

**Week 3 (Optional):**
4. Phase 4: IDXCLEAN-4001 → 4002 → 4003

**Week 4:**
5. Phase 5: IDXCLEAN-5001 → 5002 → 5003

### Parallel Opportunities

- **Phase 1**: IDXCLEAN-1003 can be done in parallel with 1001/1002
- **Phase 3**: IDXCLEAN-3001, 3002, 3003 can be done in parallel
- **Phase 4**: Optional, can be skipped for MVP

---

## Success Metrics

### Functional Metrics
- ✅ Worktree count reduced from 100+ to <10
- ✅ Search result duplication reduced from 15x to 1x
- ✅ Cleanup completes in <2 seconds
- ✅ Zero data loss for valid worktrees

### Quality Metrics
- ✅ 100% detection accuracy (no false positives)
- ✅ Transaction safety (rollback on error)
- ✅ Test coverage >85% overall, >90% for cleanup module

### Performance Metrics
- ✅ Watch startup delay <200ms (background cleanup)
- ✅ Periodic cleanup <500ms
- ✅ No indexing performance degradation

---

## Risk Mitigation

### High-Risk Areas
1. **Accidental deletion of valid worktree**
   - Mitigation: Dry-run default, validation accuracy, audit logging, backups
2. **Database corruption**
   - Mitigation: ACID transactions, automatic rollback, foreign key constraints

### Medium-Risk Areas
3. **Performance degradation during watch**
   - Mitigation: Priority scheduling, busy detection, configuration tuning
4. **Multi-worktree chunk deletion**
   - Mitigation: Array-based deletion pattern (not CASCADE), comprehensive tests

---

## Agent Assignments

### rust-indexer-engineer
- Phase 1: All tickets (1001, 1002, 1003)
- Phase 2: All tickets (2001, 2002, 2003, 2004)
- Phase 4: All tickets (4001, 4002, 4003)
- Phase 5: Documentation (5001)

### integration-tester
- Phase 3: All tickets (3001, 3002, 3003, 3004)

### verify-ticket
- All phases: Verification step for each ticket
- Phase 5: Deployment procedure review (5002), production verification (5003)

### commit-ticket
- All phases: Commit step for each ticket

---

## Planning Documents

All planning documents are complete in `/workspace/.crewchief/projects/IDXCLEAN_index-stale-worktree-cleanup/planning/`:

- **analysis.md**: Problem definition and research findings
- **architecture.md**: MVP-focused solution design with complete Watch integration analysis
- **quality-strategy.md**: Pragmatic testing approach focused on safety
- **plan.md**: High-level execution plan with all ticket details
- **review-updates.md**: Tracking document for review updates (all critical issues resolved)

---

## Next Steps

1. ✅ **Phase 4 numbering issue resolved**: File renamed to `IDXCLEAN-4002_periodic-cleanup-status-task.md`
2. **Begin Phase 1 execution**: Run `/single-ticket IDXCLEAN-1001` to start first ticket
3. **Follow ticket workflow**: Implementation → Tests → Verification → Commit
4. **Sequential progression**: Complete each phase before moving to next

---

**Document Version**: 1.1
**Last Updated**: 2025-11-18
**Status**: Ready for Execution ✅
**All Issues Resolved**: ✅
