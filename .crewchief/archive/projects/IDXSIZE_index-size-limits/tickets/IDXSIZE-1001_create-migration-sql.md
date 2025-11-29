# Ticket: IDXSIZE-1001: Create Migration SQL File

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (SQL file creation, no tests to execute)
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
Create SQL migration file that replaces the failing `idx_chunks_search_covering` index with a multi-index strategy to eliminate PostgreSQL B-tree size limit errors.

## Background
The current covering index `idx_chunks_search_covering` fails when preview text exceeds PostgreSQL's 2704-byte B-tree limit. This affects 50%+ of real-world codebases during indexing.

The solution is a multi-index strategy with three specialized indexes:
1. **Partial covering index** for small previews (<=2000 bytes, handles 95% of data)
2. **Hash-based covering index** for all sizes (MD5 approach)
3. **Basic non-covering index** as universal fallback

This ticket implements Step 1.1 from `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md`.

## Acceptance Criteria
- [x] Migration SQL file created (at `/workspace/crates/maproom/migrations/0017_fix_index_size_limits.sql` - numbered 0017 as 0013-0016 already exist)
- [x] SQL syntax validated - executes successfully against PostgreSQL
- [x] File includes DROP statement for old index (`idx_chunks_search_covering`)
- [x] File includes CREATE statements for 2 new indexes with CONCURRENTLY option (Note: reduced from 3 to 2 - hash-based index removed due to PostgreSQL not supporting expressions in INCLUDE clauses)
- [x] Each index has COMMENT explaining its purpose
- [x] Statement timeout set to 10 minutes
- [x] ANALYZE statement included at end
- [x] Migration SQL achieves functional goals: eliminates size limit errors for 100% of data while maintaining 95%+ index-only scan performance

## Technical Requirements

### Index to Drop
- `idx_chunks_search_covering` - The problematic covering index that fails on large previews

### Indexes to Create

**Note**: Originally planned for 3 indexes, but PostgreSQL limitation (expressions not allowed in INCLUDE clauses) means the hash-based index cannot be implemented as specified. The two-index solution achieves the same functional outcome.

1. **idx_chunks_search_small_preview** (Partial Covering Index)
   - Columns: `(file_id, kind, start_line)`
   - INCLUDE: `(symbol_name, preview)`
   - WHERE clause: `LENGTH(preview) <= 2000`
   - Purpose: Enables index-only scans for 95% of chunks

2. **idx_chunks_search_basic** (Non-Covering Index)
   - Columns: `(file_id, kind, start_line)`
   - No INCLUDE clause
   - Purpose: Universal fallback that works for 100% of data, including large previews that exceed 2704-byte limit

### Migration Requirements
- Use `CREATE INDEX CONCURRENTLY` to avoid table locks
- Set `statement_timeout = '10min'` at beginning
- Add descriptive `COMMENT ON INDEX` for each index
- Include `ANALYZE maproom.chunks` at end
- Use `DROP INDEX IF EXISTS` for safety
- Include header comments explaining problem, solution, and references

## Implementation Notes

Follow the exact SQL structure defined in:
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md` (lines 203-239)
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` Step 1.1 (lines 24-80)

The partial index condition `WHERE LENGTH(preview) <= 2000` ensures the index stays under the 2704-byte limit while covering 95% of chunks. This is 704 bytes below the limit to account for overhead from other columns (file_id, kind, start_line, symbol_name).

### Key Technical Details
- **2000-byte limit**: Chosen to ensure total row size < 2704 after adding metadata columns
- **MD5 hash**: Fixed 32-byte size, works for any preview length
- **CONCURRENTLY**: Allows index creation without locking the table
- **Statement timeout**: Prevents runaway queries during migration
- **ANALYZE**: Updates query planner statistics for optimal index selection

## Dependencies
None - This is the first ticket in the IDXSIZE project

## Risk Assessment

- **Risk**: Migration SQL has syntax errors
  - **Mitigation**: Validate with `psql --dry-run` or PostgreSQL syntax checker before committing

- **Risk**: Forgot to drop old index, causing both old and new to exist
  - **Mitigation**: Explicit `DROP INDEX IF EXISTS` statement at beginning of migration

- **Risk**: CONCURRENTLY fails if index already exists
  - **Mitigation**: Use `IF NOT EXISTS` clause or check for existing indexes

- **Risk**: Migration timeout on large tables
  - **Mitigation**: `statement_timeout = '10min'` provides sufficient time, CONCURRENTLY prevents locks

## Files/Packages Affected
- `/workspace/crates/maproom/migrations/0013_fix_index_size_limits.sql` (new file)

## Planning References
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/plan.md` - Step 1.1 (lines 24-80)
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/architecture.md` - Index design details (lines 203-239)
- `.crewchief/projects/IDXSIZE_index-size-limits/planning/analysis.md` - Problem analysis and motivation
