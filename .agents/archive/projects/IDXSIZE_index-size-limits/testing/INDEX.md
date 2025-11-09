# Testing Documentation Index

**Project**: IDXSIZE - Index Size Limits
**Migration**: 0017_fix_index_size_limits.sql
**Ticket**: IDXSIZE-2003

## Quick Navigation

### For Test Executors

1. **Start Here**: [README.md](README.md) - Overview and quick start
2. **Detailed Procedure**: [production-clone-test-procedure.md](production-clone-test-procedure.md) - Complete step-by-step guide
3. **Quick Reference**: [test-execution-checklist.md](test-execution-checklist.md) - Printable checklist
4. **Results Template**: [test-results-template.txt](test-results-template.txt) - Structured results recording

### For Reviewers

- **Test Overview**: [README.md](README.md) - Purpose, scope, and expected results
- **Test Results**: `migration_test_results_*.txt` (created during test execution)

### For Developers

- **Migration Source**: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- **Architecture**: `../planning/architecture.md`
- **Quality Strategy**: `../planning/quality-strategy.md`

## Document Purposes

### README.md (115 lines)
**Purpose**: Testing directory overview and quick start guide
**Use When**: First time reading testing documentation
**Contains**:
- Overview of testing approach
- Document structure
- Quick start instructions
- Success criteria summary
- Expected results reference

### production-clone-test-procedure.md (1,035 lines)
**Purpose**: Complete step-by-step manual test procedure
**Use When**: Executing production clone test
**Contains**:
- Prerequisites and required tools
- Environment setup (backup, restore, isolation)
- Pre-migration baseline measurements
- Migration execution with timing
- Post-migration validation (indexes, data, storage)
- Critical path query testing
- PostgreSQL log verification
- Rollback procedure
- Cleanup instructions
- Troubleshooting guide

### test-execution-checklist.md (240 lines)
**Purpose**: Quick reference checklist during test execution
**Use When**: Actively executing test (print and use as checklist)
**Contains**:
- Pre-test setup checklist
- Environment setup verification
- Pre-migration baseline checklist
- Migration execution checklist
- Post-migration validation checklist
- Critical path testing checklist
- Success criteria quick check
- Final result approval section

### test-results-template.txt (367 lines)
**Purpose**: Structured template for recording test results
**Use When**: Documenting test execution results
**Contains**:
- Test information section
- Environment details
- Pre-migration measurements
- Migration execution results
- Post-migration verification
- Query performance results
- Index usage statistics
- Success criteria evaluation
- Issues and observations
- Final approval section

## Test Execution Workflow

```
┌─────────────────────────────────────────┐
│ 1. Read README.md                       │
│    Understand scope and expectations    │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│ 2. Review production-clone-test-       │
│    procedure.md completely              │
│    Understand all steps before starting │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│ 3. Print test-execution-checklist.md   │
│    Use as reference during execution    │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│ 4. Create results file from template   │
│    cp test-results-template.txt         │
│       migration_test_results_DATE.txt   │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│ 5. Execute test following procedure    │
│    - Setup environment                  │
│    - Capture baseline                   │
│    - Run migration                      │
│    - Validate results                   │
│    - Test queries                       │
│    - Check logs                         │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│ 6. Document results                     │
│    Fill in test-results-template.txt    │
└───────────────┬─────────────────────────┘
                │
                ▼
┌─────────────────────────────────────────┐
│ 7. Review and approve                   │
│    Evaluate success criteria            │
│    Decide: Ready for production?        │
└─────────────────────────────────────────┘
```

## File Sizes and Line Counts

| File | Lines | Size | Purpose |
|------|-------|------|---------|
| README.md | 115 | 4.1K | Overview |
| production-clone-test-procedure.md | 1,035 | 31K | Full procedure |
| test-execution-checklist.md | 240 | 5.7K | Quick reference |
| test-results-template.txt | 367 | 13K | Results template |
| **Total** | **1,757** | **54K** | Complete test documentation |

## Migration Details Reference

**What Changes**:
- **Drops**: `idx_chunks_search_covering` (1 index)
- **Creates**: `idx_chunks_search_small_preview` and `idx_chunks_search_basic` (2 indexes)

**Why**:
- Fix: PostgreSQL 2704-byte index entry size limit
- Problem: Large preview fields (>2704 bytes) cause index creation failures
- Solution: Partial index for small previews + fallback index for all data

**Expected Impact**:
- Storage: +31% (+~155MB typical)
- Performance: 5-10ms (small), 15-30ms (large)
- Data: Zero data loss
- Queries: No application changes required

## Success Criteria Quick Reference

### MUST PASS (Blocking)
✅ Migration completes without errors
✅ Zero data loss (chunk count matches)
✅ Old index dropped, 2 new indexes created
✅ Large preview queries succeed (core fix)
✅ Query performance within ±30% baseline

### SHOULD PASS (Investigate)
✅ Storage increase < 40%
✅ Migration duration < 10 minutes
✅ No PostgreSQL errors in logs

## Related Documentation

### Project Documentation
- Analysis: `../planning/analysis.md`
- Architecture: `../planning/architecture.md`
- Quality Strategy: `../planning/quality-strategy.md`
- Project Plan: `../planning/plan.md`

### Migration Files
- Migration SQL: `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql`
- Rollback SQL: (included in production-clone-test-procedure.md)

### Ticket Documentation
- Implementation: `../tickets/IDXSIZE-2003_production-clone-testing.md`
- Related tickets: `../tickets/IDXSIZE-*.md`

## Contact and Support

**Questions about testing procedures?**
- See: production-clone-test-procedure.md (complete details)
- See: README.md (quick start)

**Questions about migration design?**
- See: ../planning/architecture.md (technical design)
- See: ../planning/analysis.md (problem analysis)

**Need help during test execution?**
- See: Troubleshooting section in production-clone-test-procedure.md
- See: test-execution-checklist.md for quick verification steps

---

**Last Updated**: 2025-11-09
**Document Version**: 1.0
**Maintained By**: IDXSIZE Project Team
