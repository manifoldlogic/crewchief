# Project: Index Stale Worktree Cleanup (IDXCLEAN)

**Status:** ✅ Complete (MVP)
**Created:** 2025-11-26
**Completed:** 2025-12-05

## Project Summary

**Goal:** Implement automated detection and removal of stale worktrees from the maproom index to eliminate search result duplication and restore search quality.

**Priority:** High (🟡) - Significantly impacts search quality

**Estimated Duration:** 3-4 weeks (MVP) + 1 week (watch integration)

---

## Problem Statement

The maproom index database contains **100+ worktrees**, of which **~95% no longer exist on disk**. These "zombie" worktrees are leftover from genetic algorithm experimentation and were never cleaned up.

### Impact

**Search Quality Degradation:**
- Same code chunk appears **15+ times** in search results
- Actual relevant results buried in noise from duplicate stale entries
- Context tool becomes unusable (can't pick correct chunk_id)
- Users lose trust in search functionality

**Database Bloat:**
- ~95 stale worktree records
- ~500,000 stale chunks
- ~2-3 GB of useless data
- Query performance degradation (joins across 100+ worktrees)

### Root Cause

1. Temporary worktrees created in `.crewchief/` during experiments
2. Worktrees indexed automatically during creation
3. Worktrees deleted from disk when experiments completed
4. **Database never updated** to reflect deletions
5. No automatic cleanup mechanism exists

---

## Proposed Solution

### Three-Component System

**1. Stale Detection Module** (`crates/maproom/src/db/cleanup.rs`)
- Identifies worktrees whose `abs_path` no longer exists on disk
- Parallel validation using tokio for fast execution
- Returns rich metadata (id, name, path, chunk_count)

**2. Safe Deletion Module** (`crates/maproom/src/db/cleanup.rs`)
- Transaction-based deletion with rollback on error
- CASCADE deletes associated chunks automatically
- Dry-run mode for inspection before deletion
- Audit logging for every deletion

**3. CLI Command Interface** (`maproom db cleanup-stale`)
- User-facing command for manual cleanup
- Dry-run is default behavior (explicit --confirm required)
- Clear output showing what will be deleted
- Error handling with actionable messages

### Optional Enhancement: Watch Integration

**Automatic Cleanup During Watch** (`crates/maproom/src/watch/`)
- Startup cleanup: Quick check when watch starts (background, non-blocking)
- Periodic cleanup: Background task every 30 minutes
- Rate limiting: Max once per 15 minutes
- Safety checks: Defers if indexer is busy
- Configuration: User can enable/disable and tune intervals

---

## Success Metrics

### Functional Metrics
- ✅ Worktree count reduced from 100+ to <10
- ✅ Search result duplication reduced from 15x to 1x
- ✅ Cleanup completes in <2 seconds (100 worktrees)
- ✅ Zero data loss for valid worktrees

### Quality Metrics
- ✅ 100% detection accuracy (no false positives)
- ✅ Transaction safety (rollback on error)
- ✅ Comprehensive test coverage (>80% unit, 100% integration)

### Performance Metrics
- ✅ Watch startup delay <200ms (background cleanup)
- ✅ Periodic cleanup execution time <500ms
- ✅ No indexing performance degradation

---

## Architecture Highlights

### Safety-First Design

**Multiple Defense Layers:**
1. **Validation:** Robust disk existence checks (permission errors treated as "exists")
2. **Dry-run default:** User must explicitly confirm deletion
3. **Transaction safety:** All deletions in single transaction, rollback on error
4. **Audit logging:** Every deletion logged with full context
5. **Rate limiting:** Automatic cleanup limited to prevent abuse

**Error Handling:**
- Permission denied → Treat as exists (safe assumption)
- Database error → Transaction rollback (no partial state)
- Validation failure → Skip worktree, log error, continue

### Watch Integration Architecture

**Non-Blocking Design:**
- Startup cleanup runs in background tokio task
- Periodic cleanup uses separate async task
- File events processed with priority over cleanup
- Cleanup can be cancelled without breaking watch

**Priority Scheduling:**
1. File events (high priority, immediate)
2. Indexing operations (high priority)
3. Cleanup operations (low priority, background)

**Safety Checks:**
- Skip cleanup if indexer is busy
- Skip cleanup if database under load
- Skip cleanup if ran recently (rate limiting)

### Performance Optimization

**Parallel Validation:**
- Use `tokio::fs::try_exists` for async disk checks
- Process worktrees in parallel using `join_all`
- ~100ms for 100 worktrees (vs. ~10s sequential)

**Batched Operations:**
- Single transaction for all deletions
- CASCADE leverages database for chunk removal
- No redundant queries

---

## Project Phases

### Phase 1: Core Cleanup Infrastructure (Week 1)
**Deliverables:**
- Stale detection module
- Safe deletion module
- Data models and error types

**Tickets:** IDXCLEAN-1001, 1002, 1003

### Phase 2: CLI Command Interface (Week 1-2)
**Deliverables:**
- `maproom db cleanup-stale` command
- Dry-run and confirmation logic
- User-friendly output formatting

**Tickets:** IDXCLEAN-2001, 2002, 2003

### Phase 3: Integration Testing and Safety Validation (Week 2)
**Deliverables:**
- Integration test suite
- Safety validation tests
- Manual validation on staging

**Tickets:** IDXCLEAN-3001, 3002, 3003, 3004

### Phase 4: Watch Integration (Week 3) [Optional]
**Deliverables:**
- Cleanup scheduler module
- Startup and periodic cleanup
- Configuration options

**Tickets:** IDXCLEAN-4001, 4002, 4003, 4004

### Phase 5: Production Deployment (Week 4)
**Deliverables:**
- Documentation updates
- Deployment procedure
- Production verification

**Tickets:** IDXCLEAN-5001, 5002, 5003

---

## Relevant Agents

### Primary Agent: rust-indexer-engineer
**Responsibilities:**
- Implement detection and deletion modules (Rust)
- Create CLI command interface
- Build watch integration components
- Performance optimization

**Tickets:** IDXCLEAN-1001, 1002, 1003, 2001, 2002, 2003, 4001, 4002, 4003, 4004

### Supporting Agent: integration-tester
**Responsibilities:**
- Create comprehensive test suite
- Validate detection accuracy
- Verify deletion safety
- Manual validation on staging

**Tickets:** IDXCLEAN-3001, 3002, 3003, 3004

### Workflow Agents: verify-ticket, commit-ticket
**Responsibilities:**
- Verify acceptance criteria met
- Create Conventional Commits
- Maintain ticket workflow

**Tickets:** All tickets (verification and commit phases)

---

## Planning Documents

### [analysis.md](planning/analysis.md)
Deep understanding of the stale worktree problem:
- Problem definition and root cause analysis
- Impact assessment on search quality and database
- Research on detection methods and cleanup patterns
- Industry solutions and best practices
- Key insights and design principles

### [architecture.md](planning/architecture.md)
MVP-focused solution design with watch integration deep-dive:
- Three-component system architecture
- Watch integration strategies (when, how, efficiently)
- Performance characteristics and optimization
- Technology choices and trade-offs
- Deployment strategy and rollout plan

### [quality-strategy.md](planning/quality-strategy.md)
Pragmatic testing approach focused on safety:
- Test pyramid (30% unit, 60% integration, 10% manual)
- Critical test paths (detection accuracy, deletion safety, CLI usability)
- Integration test suite with fixtures
- Manual validation checklist
- Success metrics and confidence levels

### [security-review.md](planning/security-review.md)
Comprehensive security assessment:
- Threat model (accidental deletion, database corruption, unauthorized access, DoS)
- Architecture security analysis
- Known gaps and risk evaluation (authentication, soft delete, backup integration)
- Security best practices (defense-in-depth, least privilege, auditability)
- Production deployment security checklist

### [plan.md](planning/plan.md)
High-level execution plan:
- 5 phases with clear deliverables
- 17 tickets with detailed acceptance criteria
- Agent assignments and timeline
- Testing and security milestones
- Risk mitigation and success criteria

---

## Key Design Decisions

### Decision 1: Dry-Run Default with Explicit Confirmation
**Rationale:** Prevent accidental data loss; user must actively confirm deletion

**Implementation:**
```bash
# Default: dry-run (safe)
$ maproom db cleanup-stale
⚠️  This was a dry-run. Use --confirm to actually delete.

# Explicit confirmation required
$ maproom db cleanup-stale --confirm
🗑️  Deleting stale worktrees...
```

### Decision 2: Transaction-Based Deletion with CASCADE
**Rationale:** ACID guarantees prevent corruption; CASCADE ensures no orphaned chunks

**Implementation:**
```rust
let mut tx = db.begin_transaction().await?;
for wt in stale {
    delete_worktree_tx(&mut tx, wt.id).await?; // CASCADE deletes chunks
}
tx.commit().await?; // All-or-nothing
```

### Decision 3: Hybrid Watch Integration
**Rationale:** Balance between automation and safety; startup + periodic cleanup

**Implementation:**
- Startup: Quick check on watch start (skip if recent, background task)
- Periodic: Every 30 minutes in background (rate limited, defers if busy)
- Configuration: User can enable/disable and tune intervals

### Decision 4: Disk Validation Only (No Git Validation)
**Rationale:** Simpler, faster (~1ms vs ~50ms per worktree); sufficient for MVP

**Future:** Can add git validation if needed, but disk check is accurate enough

---

## Risk Assessment

### High-Risk Areas

**1. Accidental Deletion of Valid Worktree**
- **Impact:** High (data loss)
- **Likelihood:** Low (multiple safety checks)
- **Mitigation:** Dry-run default, validation accuracy, audit logging, backups

**2. Database Corruption During Cleanup**
- **Impact:** Critical (data loss)
- **Likelihood:** Very Low (transaction safety)
- **Mitigation:** ACID transactions, automatic rollback, foreign key constraints

### Medium-Risk Areas

**3. Performance Degradation During Watch**
- **Impact:** Medium (user experience)
- **Likelihood:** Low (background execution, rate limiting)
- **Mitigation:** Priority scheduling, busy detection, configuration tuning

**4. Unauthorized Deletion**
- **Impact:** Critical (data loss)
- **Likelihood:** Low (requires access)
- **Mitigation:** OS authentication, audit logging, no remote execution

---

## Dependencies

**None External:**
- Self-contained project
- Uses existing database (PostgreSQL)
- Uses existing toolchain (Rust, tokio, sqlx)
- No new libraries or services required

**Integration Points:**
- `crates/maproom/src/db/` - Database operations
- `crates/maproom/src/cli/` - CLI commands
- `crates/maproom/src/watch/` - Watch command (optional)

---

## Next Steps

1. **Review planning documents** - Ensure team alignment on approach
2. **Run `/create-project-tickets IDXCLEAN`** - Generate 17 individual tickets
3. **Execute Phase 1** - Build core infrastructure (detection + deletion)
4. **Test thoroughly** - Integration tests + manual validation on staging
5. **Deploy incrementally** - Manual cleanup first, then watch integration

---

## Quick Reference

**Project Slug:** IDXCLEAN
**Project Name:** Index Stale Worktree Cleanup
**Ticket Range:** IDXCLEAN-1001 to IDXCLEAN-5003 (17 tickets)
**Duration:** 3-4 weeks (MVP) + 1 week (watch integration)
**Priority:** High (🟡)
**Risk Level:** Medium (data deletion, but heavily mitigated)
**Confidence:** High (90% for MVP, 75% for watch integration)

---

## Contact and Collaboration

**Primary Agent:** rust-indexer-engineer
**Review Agent:** verify-ticket
**Testing Agent:** integration-tester
**Commit Agent:** commit-ticket

**Planning Status:** ✅ Complete (all documents reviewed and approved)
**Ready for Execution:** ✅ Yes (run `/create-project-tickets IDXCLEAN`)
