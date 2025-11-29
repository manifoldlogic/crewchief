# Deployment Documentation: Migration 0017

**Migration**: `0017_fix_index_size_limits.sql`
**Project**: IDXSIZE - Index Size Limits
**Purpose**: Production deployment documentation and procedures

---

## Overview

This directory contains all documentation required for safe production deployment of migration 0017, which fixes PostgreSQL B-tree index size limit errors by implementing a multi-index strategy.

**Migration Purpose**: Replace the failing `idx_chunks_search_covering` index with two specialized indexes that handle all chunk sizes without hitting PostgreSQL's 2704-byte index entry limit.

---

## Document Index

### 1. Pre-Deployment Checklist
**File**: `pre-deployment-checklist.md`
**Purpose**: Systematic verification that all testing is complete and production migration is ready
**Use When**: Before executing production migration (IDXSIZE-3002)

**Contents**:
- Testing verification (Phase 1 and Phase 2)
- Production clone testing execution confirmation
- Production database backup procedures
- Resource verification (disk space, PostgreSQL health)
- Baseline metrics capture
- Migration window planning
- Rollback readiness verification
- Final go/no-go checklist
- Sign-off and approval section

**Key Feature**: MANUAL verification checklist - must be completed by humans before production migration.

### 2. Migration Execution Guide
**File**: `migration-execution-guide.md`
**Purpose**: Step-by-step commands for executing production migration with verification
**Use When**: During production migration execution (IDXSIZE-3002)

**Contents**:
- Pre-execution verification steps
- Migration execution commands with timing
- Post-migration verification queries
- Success criteria evaluation
- Rollback decision tree
- Rollback procedures (if needed)
- Post-deployment tasks
- Troubleshooting guide

**Key Feature**: Exact commands provided for every step - copy/paste ready.

---

## Deployment Workflow

```
┌─────────────────────────────────────────────────────────────┐
│ PHASE 1: Pre-Deployment Verification                       │
│ Document: pre-deployment-checklist.md                      │
├─────────────────────────────────────────────────────────────┤
│ 1. Verify all Phase 1 tickets complete (IDXSIZE-1001-1004) │
│ 2. Verify all Phase 2 tickets complete (IDXSIZE-2001-2004) │
│ 3. Execute production clone test (manual)                  │
│ 4. Create and verify production backup                     │
│ 5. Verify disk space and resources                         │
│ 6. Capture baseline metrics                                │
│ 7. Schedule migration window                               │
│ 8. Verify rollback readiness                               │
│ 9. Complete final go/no-go checklist                       │
│ 10. Sign-off and approval                                  │
│                                                             │
│ Result: Signed checklist confirming readiness              │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ PHASE 2: Migration Execution                               │
│ Document: migration-execution-guide.md                     │
├─────────────────────────────────────────────────────────────┤
│ 1. Pre-execution verification (checklist complete?)        │
│ 2. Confirm migration window                                │
│ 3. Final safety checks                                     │
│ 4. Capture pre-migration timestamp                         │
│ 5. Execute migration SQL                                   │
│ 6. Capture post-migration timestamp                        │
│ 7. Verify index changes                                    │
│ 8. Verify data integrity                                   │
│ 9. Test large preview support (critical)                   │
│ 10. Verify storage impact                                  │
│ 11. Test query performance                                 │
│ 12. Check PostgreSQL logs                                  │
│ 13. Evaluate success criteria                              │
│                                                             │
│ Result: Migration success/failure determination            │
└─────────────────────────────────────────────────────────────┘
                            ↓
                ┌───────────────────────┐
                │ Migration Succeeded?  │
                └───────────────────────┘
                     │              │
                    YES            NO
                     │              │
                     ↓              ↓
    ┌─────────────────────────┐  ┌──────────────────────────┐
    │ PHASE 3: Post-Deployment│  │ PHASE 3-ALT: Rollback    │
    │ Document: migration-    │  │ Document: migration-     │
    │   execution-guide.md    │  │   execution-guide.md     │
    │   (Step 20-24)          │  │   (Step 17-19)           │
    ├─────────────────────────┤  ├──────────────────────────┤
    │ 1. Document results     │  │ 1. Execute rollback SQL  │
    │ 2. Send success notice  │  │ 2. Verify rollback       │
    │ 3. Archive artifacts    │  │ 3. Send rollback notice  │
    │ 4. Update tracking      │  │ 4. Root cause analysis   │
    │ 5. Schedule monitoring  │  │ 5. Plan retry            │
    └─────────────────────────┘  └──────────────────────────┘
```

---

## Quick Start

### For First-Time Readers

1. **Read this README** - Understand deployment structure
2. **Review pre-deployment-checklist.md** - Understand what needs verification
3. **Review migration-execution-guide.md** - Understand execution steps
4. **Check testing documentation** - Verify all tests passed
   - Location: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/`

### For Deployment Executors

**Before Migration Day**:
1. Complete `pre-deployment-checklist.md` sections 1-7
2. Execute production clone test (section 2)
3. Schedule migration window (section 6)
4. Get approval sign-off (section 9)

**On Migration Day**:
1. Complete final checks from `pre-deployment-checklist.md` section 3-4
2. Follow `migration-execution-guide.md` steps 1-24
3. Evaluate success criteria (guide step 15)
4. Execute post-deployment or rollback as appropriate

---

## Document Summary

| Document | Lines | Purpose | Key Sections |
|----------|-------|---------|--------------|
| pre-deployment-checklist.md | ~800 | Verify readiness | Testing, Backup, Resources, Approval |
| migration-execution-guide.md | ~900 | Execute migration | Commands, Verification, Rollback, Results |
| README.md (this file) | ~350 | Overview | Workflow, Quick start, Reference |
| **Total** | **~2,050** | **Complete deployment docs** | **Production-ready** |

---

## Key Migration Details

### What Changes

**Drops**:
- `idx_chunks_search_covering` (1 index) - problematic covering index that fails on large previews

**Creates**:
- `idx_chunks_search_small_preview` - Partial covering index for previews ≤2000 bytes
- `idx_chunks_search_basic` - Non-covering index for all data (universal fallback)

### Why This Change

**Problem**: PostgreSQL B-tree indexes have a 2704-byte size limit per entry. The current covering index includes the `preview` column which can exceed this limit, causing index creation failures on ~50% of real-world codebases.

**Solution**: Multi-index strategy that routes queries based on preview size:
- Small previews (≤2000 bytes, ~95% of data): Use partial covering index for fast index-only scans
- Large previews (>2000 bytes, ~5% of data): Use basic index that always works

**Result**: 100% of chunks can be indexed successfully while maintaining 95%+ index-only scan performance.

### Expected Impact

- **Storage**: +31% (+~155MB for typical 500MB database)
- **Performance**: 5-10ms (small previews), 15-30ms (large previews)
- **Downtime**: None (CONCURRENTLY allows queries during migration)
- **Duration**: 5-10 minutes
- **Data Loss**: Zero (verified in all testing phases)

### Success Criteria

**MUST PASS** (blocking - rollback if fail):
1. Migration completes without errors
2. Zero data loss (chunk count matches)
3. Old index dropped, 2 new indexes created
4. Large preview queries succeed (>2704 bytes)
5. Query performance within ±30% baseline

**SHOULD PASS** (investigate if fail):
1. Storage increase < 40%
2. Migration duration < 10 minutes
3. No PostgreSQL errors in logs
4. Correct index selection by query planner

---

## Testing Summary

All testing phases complete before deployment:

### Phase 1: Migration Development
- ✅ IDXSIZE-1001: Migration SQL created and validated
- ✅ IDXSIZE-1002: Rollback script created and tested
- ✅ IDXSIZE-1003: Documentation updated
- ✅ IDXSIZE-1004: Phase 1 validation passed

### Phase 2: Testing Validation
- ✅ IDXSIZE-2001: Automated test suite (30/30 tests passed)
  - L1 (Syntax): 9/9 passed
  - L2 (Empty DB): 12/12 passed
  - L3 (Data): 9/9 passed - **large preview INSERT confirmed**
- ✅ IDXSIZE-2002: Query performance tests (17/17 tests passed)
  - Small previews: 0.037ms (target: <20ms)
  - Large previews: 15-30ms (target: <50ms)
- ✅ IDXSIZE-2003: Production clone test procedure documented (5 files, 1,757 lines)
- ✅ IDXSIZE-2004: Phase 2 validation passed

**Production Clone Test**: Must be executed manually before production deployment (documented in testing/).

---

## Rollback Information

### Rollback Availability

**Rollback script**: `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql`

**Rollback capability**:
- ✅ **Available**: If database contains only small previews (≤2704 bytes)
- ❌ **NOT AVAILABLE**: If database contains any large previews (>2704 bytes)

**Recommendation**: Rollback is provided for completeness but is NOT RECOMMENDED. This is effectively a forward-only migration once large previews are indexed.

### Rollback Decision Tree

The migration execution guide (step 15-19) provides a detailed decision tree for:
1. When to rollback (data loss, core fix failure)
2. When to investigate (performance variation)
3. When to keep (acceptable variation)

**Critical**: Follow the decision tree systematically. Do not rollback without consulting the decision matrix.

---

## Critical File Locations

### Migration Files
- **Migration SQL**: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- **Rollback SQL**: `/workspace/crates/maproom/migrations/rollback/0017_rollback.sql`

### Documentation
- **This directory**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/deployment/`
- **Testing docs**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/`
- **Project planning**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/planning/`
- **Tickets**: `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/tickets/`

### Backups and Logs
- **Backup location**: `/var/backups/maproom/migration-0017/`
- **Migration log**: `/tmp/migration_0017_[TIMESTAMP].log`
- **Results summary**: `/tmp/migration_0017_results_[TIMESTAMP].txt`
- **Archive**: `/var/backups/maproom/migration-0017/artifacts/`

---

## Deployment Checklist Quick Reference

### Before Migration Day
- [ ] All Phase 1 tickets complete
- [ ] All Phase 2 tickets complete
- [ ] Production clone test executed and approved
- [ ] Migration window scheduled
- [ ] Team notified

### On Migration Day (Pre-Execution)
- [ ] Pre-deployment checklist completed
- [ ] Production backup created and verified
- [ ] Disk space verified sufficient
- [ ] Baseline metrics captured
- [ ] Rollback script accessible
- [ ] Approval sign-off obtained

### During Migration
- [ ] Pre-execution safety checks passed
- [ ] Migration SQL executed
- [ ] Post-migration verification completed
- [ ] Success criteria evaluated
- [ ] Decision made (keep/rollback)

### After Migration
- [ ] Results documented
- [ ] Team notified (success/rollback)
- [ ] Artifacts archived
- [ ] Tracking updated
- [ ] Monitoring scheduled

---

## Support and Contacts

### During Deployment

**Primary Contact**: [DEPLOYMENT LEAD NAME]
- Role: Migration executor
- Contact: [EMAIL/SLACK]
- Availability: During migration window

**Backup Contact**: [BACKUP NAME]
- Role: Database administrator
- Contact: [EMAIL/SLACK]
- Availability: During migration window

**Escalation**: [ESCALATION PATH]

### Documentation Questions

- **Pre-deployment questions**: Review `pre-deployment-checklist.md`
- **Execution questions**: Review `migration-execution-guide.md`
- **Testing questions**: Review `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/`
- **Architecture questions**: Review `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md`

---

## Revision History

| Date | Version | Changes | Author |
|------|---------|---------|--------|
| 2025-11-09 | 1.0 | Initial deployment documentation created | IDXSIZE Team |

---

## Next Steps

1. **Complete pre-deployment checklist** - Verify all testing and readiness
2. **Execute production clone test** - Manual test on production data clone
3. **Schedule migration window** - Coordinate with team
4. **Execute migration** - Follow migration execution guide
5. **Monitor post-deployment** - IDXSIZE-3003 (7-day monitoring period)

---

**Ready to proceed?** Start with `pre-deployment-checklist.md`.

**Questions?** Contact IDXSIZE project lead.

**Emergency?** Follow rollback procedures in `migration-execution-guide.md` step 17-19.
