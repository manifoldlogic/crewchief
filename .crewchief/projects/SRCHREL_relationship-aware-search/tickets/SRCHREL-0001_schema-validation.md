# Ticket: SRCHREL-0001 - Database Schema Validation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary

Validate that the database schema supports quality-weighted graph scoring by confirming edge types, chunk fields, and test detection patterns. This prerequisite ensures the SQL query will work before implementation begins.

## Background

The EDGEEXT project has populated the `chunk_edges` table with `calls` edges for TypeScript, JavaScript, and Rust. Before implementing quality-weighted scoring, we must verify that:
1. The database has the necessary edge data (✅ partially complete - calls edges exist)
2. Chunk and file fields support test detection heuristics
3. Actual `kind` values match our assumptions for test detection

This is a BLOCKING prerequisite - Phase 1 implementation cannot begin until validation completes.

## Acceptance Criteria

- [ ] Query actual chunk `kind` values from the database (sample 100 chunks)
- [ ] Document distinct chunk `kind` values in architecture.md
- [ ] Verify `chunks.relpath` is accessible via JOIN with files table
- [ ] Verify `files.relpath` contains file paths suitable for test detection
- [ ] Sample 50 file paths with "test" patterns and confirm they are actual test files
- [ ] Document test detection pattern accuracy expectations in architecture.md
- [ ] Confirm `chunk_edges` table has populated `calls` edges (count > 0)
- [ ] Verify edge table schema matches Edge struct (src_chunk_id, dst_chunk_id, type columns)
- [ ] Document findings in `.crewchief/projects/SRCHREL_relationship-aware-search/planning/validation-results.md`

## Technical Requirements

**Database Queries to Run:**

```sql
-- 1. Verify edge data exists
SELECT COUNT(*) FROM chunk_edges WHERE type = 'calls';

-- 2. Sample distinct chunk kinds
SELECT DISTINCT kind FROM chunks LIMIT 100;

-- 3. Sample file paths for test pattern validation
SELECT f.relpath
FROM files f
JOIN chunks c ON c.file_id = f.id
WHERE f.relpath LIKE '%test%' OR f.relpath LIKE '%spec%'
LIMIT 50;

-- 4. Verify JOIN accessibility
SELECT c.id, c.kind, f.relpath
FROM chunks c
JOIN files f ON f.id = c.file_id
LIMIT 10;

-- 5. Count edges by type
SELECT type, COUNT(*) as count
FROM chunk_edges
GROUP BY type;
```

**Test Detection Pattern Validation:**
- Review sampled file paths to confirm test detection heuristics will work
- Patterns to validate:
  - `/test/` directories
  - `/tests/` directories
  - `/__tests__/` directories
  - `.test.ts`, `.test.js` files
  - `.spec.ts`, `.spec.js` files
  - `_test.rs`, `_test.py` files

**Documentation Requirements:**
- Create `planning/validation-results.md` with:
  - Edge count by type
  - Sample chunk kinds (first 20 distinct values)
  - Test path pattern accuracy assessment
  - Any schema issues discovered
  - Recommendations for Phase 1 implementation

## Implementation Notes

**Database Access:**
- Use `~/.maproom/maproom.db` (default location)
- Can override with `MAPROOM_DATABASE_URL` environment variable
- Use SQLite CLI: `sqlite3 ~/.maproom/maproom.db`
- Or use Rust code: `crates/maproom/src/db/sqlite/mod.rs`

**Expected Findings:**
- Chunk kinds: likely tree-sitter node types (e.g., "function_declaration", "method_definition")
- Edge count: 458 calls edges confirmed in prerequisites (from EDGEEXT validation)
- Test patterns: Expect 90%+ accuracy for file path-based detection

**Success Indicators:**
- All SQL queries run successfully without errors
- Edge data exists (count > 0 for `calls` type)
- File paths contain enough information for test detection
- No blocking schema issues discovered

## Dependencies

**Prerequisites:**
- EDGEEXT project completed (edge extraction infrastructure exists)
- Database populated with edge data

**Blocks:**
- SRCHREL-0002 (SQL performance validation needs schema confirmed)
- SRCHREL-0003 (test detection validation needs patterns confirmed)

## Risk Assessment

**Risk:** Chunk `kind` values are opaque and don't support test detection
**Mitigation:** Use file path as primary test detection signal (already validated in EDGEEXT). Chunk kind is secondary validation only.

**Risk:** File paths are relative and lack directory structure
**Mitigation:** Query confirms relpath includes full directory path. If not, adjust test detection to work with available data.

**Risk:** Edge table schema differs from assumptions
**Mitigation:** Early validation prevents late-stage surprises. Can adapt SQL query to match actual schema.

## Files/Packages Affected

**Documentation Created:**
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/validation-results.md` (new)

**Documentation Updated:**
- `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (document actual kind values and test detection accuracy)

**Database Queries:**
- Read-only queries on `chunks`, `files`, `chunk_edges` tables

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Prerequisite 1, lines 32-55)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (Test detection design, lines 211-245)
