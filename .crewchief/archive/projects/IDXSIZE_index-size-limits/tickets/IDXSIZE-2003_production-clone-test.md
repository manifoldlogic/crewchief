# Ticket: IDXSIZE-2003: Production clone test (manual)

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (manual test documentation created, execution is deferred)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Manually test the migration on a clone of the production maproom database to validate it works with real data and measure actual migration time, storage impact, and query performance.

## Background
This is L4 testing (production-like environment) - the final validation before production deployment. We need to verify the migration works on real production data with actual code chunk distributions, large previews, and production data volume.

This ticket implements Step 2.3 from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` (lines 258-298).

## Acceptance Criteria
- [x] Test procedure document created with step-by-step instructions
- [x] Validation checklist created for consistent execution
- [x] Test results template created for structured recording
- [x] Documentation covers production database backup procedure (pg_dump)
- [x] Documentation covers test PostgreSQL instance setup and restore
- [x] Documentation specifies baseline measurements (chunk count, table size, index size)
- [x] Documentation includes migration execution instructions (<10 minutes target)
- [x] Documentation verifies 2 new indexes created (corrected from 3)
- [x] Documentation includes zero data loss verification (chunk count comparison)
- [x] Documentation includes sample queries and expected results
- [x] Documentation specifies query performance targets (within ±30% of expectations)
- [x] Documentation includes PostgreSQL log verification procedures

**Note**: This ticket creates DOCUMENTATION for manual testing. Actual test execution is deferred until production deployment preparation.

## Technical Requirements
- Create production database backup using pg_dump
- Restore backup to isolated test PostgreSQL instance (different port)
- Run baseline measurements: `SELECT COUNT(*) FROM maproom.chunks`, table size, existing index sizes
- Apply migration: `time psql < migrations/0013_fix_index_size_limits.sql`
- Verify using: `\di maproom.idx_chunks_search_*`
- Run critical path test queries from quality-strategy.md (lines 169-211)
- Check PostgreSQL logs for any warnings or errors
- Document actual storage increase percentage
- Cleanup test instance when complete

## Implementation Notes
Follow the production clone procedure from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` Step 2.3 (lines 258-298) and quality-strategy.md L4 testing (lines 128-165).

This is MANUAL testing - requires human execution and observation. The agent should document the test procedure clearly but execution happens manually during migration deployment preparation.

The database-engineer agent should create:
1. **Test procedure document**: Step-by-step instructions for executing the production clone test
2. **Validation checklist**: Specific measurements to capture and success criteria
3. **Test results template**: Structured format for recording observations

This ensures the manual test can be executed consistently and results are properly documented.

## Dependencies
- IDXSIZE-2001 (automated tests passed)
- IDXSIZE-2002 (query performance validated on test data)
- Access to production maproom-postgres database for backup

## Risk Assessment
- **Risk**: Production backup contains sensitive data
  - **Mitigation**: Use isolated test environment, delete backup after testing
- **Risk**: Backup/restore takes too long
  - **Mitigation**: Plan test during low-activity period
- **Risk**: Migration fails on production data structure
  - **Mitigation**: This is why we test! Investigate and fix before production

## Files/Packages Affected
- `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/INDEX.md` (NEW - master navigation document, 234 lines)
- `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/README.md` (NEW - testing directory overview, 115 lines)
- `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/production-clone-test-procedure.md` (NEW - complete test procedure, 1,035 lines)
- `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/test-execution-checklist.md` (NEW - quick reference checklist, 240 lines)
- `/workspace/.crewchief/projects/IDXSIZE_index-size-limits/testing/test-results-template.txt` (NEW - results recording template, 367 lines)

**Total**: 5 files created (1,991 lines, 64KB)

**When Test is Executed** (future):
- Temporary production backup file: `prod_backup_YYYYMMDD.sql`
- Test PostgreSQL Docker container (temporary, port 5433)
- Results file: `test-results-YYYYMMDD.txt` (copy of template with actual measurements)

## Documentation Summary

### Created Files

**1. INDEX.md** - Master Navigation Document
- Document purposes and use cases
- Test execution workflow diagram
- Quick reference for all stakeholders
- Navigation guide to other documents

**2. README.md** - Testing Directory Overview
- Quick start guide
- Success criteria summary
- Expected results reference
- Document relationships

**3. production-clone-test-procedure.md** - Complete Test Procedure (MAIN DOCUMENT)
- Step-by-step manual testing procedure (13 steps)
- Environment setup (backup, restore, isolation)
- Pre-migration baseline measurements
- Migration execution with timing
- Post-migration validation (indexes, data integrity, storage impact)
- Critical path testing (4 test queries with performance targets)
- PostgreSQL log verification
- Rollback procedure (if migration fails)
- Cleanup and archival instructions
- Comprehensive troubleshooting guide

**4. test-execution-checklist.md** - Quick Reference Checklist
- Printable quick reference
- All validation steps in checkbox format
- Success criteria quick evaluation
- Final approval section with signatures

**5. test-results-template.txt** - Structured Results Recording
- Template for recording all test measurements
- Pre/post-migration comparison fields
- Query performance tracking
- Issue and observation documentation
- Follow-up action items
- Approval workflow with signatures

### Key Documentation Features

✅ **Comprehensive Coverage**:
- Production database backup and restore
- Isolated test environment setup
- Pre/post-migration measurements
- Migration execution and timing
- Index verification (2 new, 1 dropped)
- Zero data loss verification
- Storage impact measurement (+31% expected, 25-40% acceptable)
- Critical path query testing (4 queries)
- PostgreSQL log review
- Index usage statistics

✅ **Clear Success Criteria**:
- **MUST PASS**: Migration success, zero data loss, correct indexes, large previews work, performance ±30%
- **SHOULD PASS**: Storage <40%, duration <10 min, no PostgreSQL errors
- **NICE TO HAVE**: Index-only scans >90%, planning time <5ms

✅ **Safety & Rollback**:
- Backup verification steps
- Isolated test environment (port 5433)
- Rollback procedure if migration fails
- Cleanup instructions

✅ **Expected Results**:
- Storage: +31% (+~155MB typical)
- Query performance: 5-10ms (small), 15-30ms (large)
- Migration duration: 2-5 min (<10 min acceptable)
- Data integrity: Zero data loss

### Usage

**For Test Executors**:
1. Start with INDEX.md
2. Read README.md for overview
3. Review production-clone-test-procedure.md completely
4. Print test-execution-checklist.md for reference
5. Copy test-results-template.txt to dated file
6. Execute test step-by-step
7. Evaluate success criteria and obtain approval

**For Reviewers**:
1. Read README.md for scope
2. Review completed test-results-*.txt file
3. Verify all measurements documented
4. Check success criteria evaluation
5. Approve or reject for production deployment
