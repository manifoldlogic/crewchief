# Ticket: SEMRANK-2003: Implement Kind-Based Multiplier in SQL

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
Add CASE statement for kind multiplier using actual database enum values, map chunk kinds to multiplier values, update SELECT query to compute kind_mult column.

## Background
Current FTS treats all chunks equally regardless of their semantic kind (func, test, heading_1 all receive same ranking). This ticket implements the first layer of semantic enhancement: kind awareness. Functions and classes (implementations) should rank higher than tests, which should rank higher than documentation headings.

The database uses specific enum values defined in `init.sql:44` (maproom.symbol_kind enum). We must verify these actual values rather than assume 'function' or other names. This is the foundation for the semantic ranking system that will be combined with exact-match multipliers in later tickets.

References SEMRANK plan Section 3.2.1 (Kind-Based Multiplier Implementation).

## Acceptance Criteria
- [ ] **PREREQUISITE VERIFIED**: Query `SELECT DISTINCT kind FROM maproom.chunks` executed and actual database enum values documented in implementation
- [ ] SQL CASE statement created using ACTUAL database enum values (not assumed 'function')
- [ ] Kind multipliers mapped correctly per specification:
  - 'func' → 2.5
  - 'class', 'component' → 2.0
  - 'hook' → 1.8
  - 'module', 'type' → 1.5
  - 'var' → 1.0
  - 'heading_1' → 0.6, 'heading_2' → 0.5, 'heading_3' → 0.4
  - 'other' → 1.0, NULL → 1.0
- [ ] SELECT query updated to compute `kind_mult` column
- [ ] Query compiles without SQL errors
- [ ] Comment added noting enum source: `-- Source: init.sql:44 (maproom.symbol_kind enum)`
- [ ] Tested with queries that return different kinds (func vs heading_1) showing different multiplier values

## Technical Requirements
- Location: Rust FTS implementation `/crates/maproom/src/search/fts.rs`
- Current query at lines 77-99, modify to add kind_mult column
- SQL CASE statement implementation:
  ```sql
  CASE
    WHEN c.kind = 'func' THEN 2.5
    WHEN c.kind IN ('class', 'component') THEN 2.0
    WHEN c.kind = 'hook' THEN 1.8
    WHEN c.kind IN ('module', 'type') THEN 1.5
    WHEN c.kind = 'var' THEN 1.0
    WHEN c.kind = 'heading_1' THEN 0.6
    WHEN c.kind = 'heading_2' THEN 0.5
    WHEN c.kind = 'heading_3' THEN 0.4
    WHEN c.kind = 'other' THEN 1.0
    WHEN c.kind IS NULL THEN 1.0
    ELSE 1.0
  END AS kind_mult
  ```
- Preserve existing `ts_rank_cd()` as `base_score` for debug mode visibility
- Do NOT yet multiply into final score (that's SEMRANK-2005)
- Add kind_mult as separate column initially for visibility and debugging

## Implementation Notes
**Step 1: Verify Database Enum Values**
- First, document actual kind values from database query
- Verify all values match enum definition in init.sql:44
- Document any discrepancies between expected and actual values

**Step 2: Implement CASE Statement**
- Add kind_mult as separate column initially for visibility
- Follow existing SQL query patterns in fts.rs
- Ensure proper formatting and indentation

**Step 3: Testing**
- Test with queries that return different kinds (func vs heading_1)
- Validate multipliers apply correctly via debug mode
- Verify NULL handling returns 1.0 (neutral baseline)

## Dependencies
- SEMRANK-0001 (search tool exists)
- SEMRANK-1004 (test corpus indexed with kind values)

## Risk Assessment
- **Risk**: Kind enum mismatch between assumption and actual database values
  - **Mitigation**: PREREQUISITE query verification before implementation
- **Risk**: SQL syntax errors in CASE statement
  - **Mitigation**: Test query compilation before committing
- **Risk**: Null kinds causing crashes or unexpected behavior
  - **Mitigation**: Explicit CASE handling with ELSE 1.0 clause

## Files/Packages Affected
- `/crates/maproom/src/search/fts.rs`
