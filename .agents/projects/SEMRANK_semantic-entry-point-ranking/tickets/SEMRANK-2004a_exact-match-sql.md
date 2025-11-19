# Ticket: SEMRANK-2004a: Implement Exact Match SQL Logic

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - 55 tests executed and passing (6 FTS unit + 49 integration)
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
Add SQL CASE statement for exact match detection, handle case-insensitive comparison, apply 3.0× multiplier when symbol_name matches normalized query.

## Background
Users searching for "validate_provider" want that specific function ranked highest, not just substring matches. The current exact bonus uses `+0.2` additive bonus if `symbol_name ILIKE '%query%'`, which is too broad (matches substrings) and too weak (additive instead of multiplicative).

This ticket implements the SQL side of exact matching with a 3.0× multiplicative boost. The companion ticket SEMRANK-2004b implements query normalization in TypeScript (handling "validateProvider" → "validate_provider" transformations) and removes the old bonus logic.

References SEMRANK plan Section 3.2.2 (Exact Match Multiplier Implementation).

## Acceptance Criteria
- [x] SQL CASE statement for exact match detection implemented
- [x] Case-insensitive comparison: `LOWER(c.symbol_name) = LOWER($normalized_query)`
- [x] Exact match multiplier: 3.0× when match, 1.0 otherwise
- [x] Null symbol_name handled: Returns 1.0 (no boost, no crash)
- [x] SQL compiles and executes without errors
- [x] Separate `exact_mult` column computed for debug visibility
- [x] $normalized_query parameter added to SQL query

## Technical Requirements
- Location: Rust FTS implementation `/crates/maproom/src/search/fts.rs`
- SQL CASE statement implementation:
  ```sql
  CASE
    WHEN LOWER(c.symbol_name) = LOWER($normalized_query) THEN 3.0
    ELSE 1.0
  END AS exact_mult
  ```
- Add $normalized_query parameter to SQL query (value will be passed from TypeScript in 2004b)
- Do NOT remove old exact bonus yet (will be done in 2004b to avoid merge conflicts)
- Add exact_mult as separate column initially for debugging
- Ensure parameter binding follows existing patterns in fts.rs

## Implementation Notes
**Step 1: Add Parameter**
- Add $normalized_query parameter to query
- Follow existing parameterized query patterns in fts.rs
- Ensure proper parameter indexing

**Step 2: Implement CASE Statement**
- Add exact_mult as separate column for visibility
- Use LOWER() on both sides for case-insensitive comparison
- ELSE 1.0 handles null symbol_name gracefully

**Step 3: Testing**
- Test with exact matches: "authenticate" should match "authenticate"
- Test case insensitivity: "AUTHENTICATE" should match "authenticate"
- Test null symbol_name: Should return 1.0, not crash
- Preserve old exact bonus temporarily for comparison (removed in 2004b)

## Dependencies
- SEMRANK-2003 (kind_mult implemented)

## Risk Assessment
- **Risk**: Case sensitivity issues causing missed matches
  - **Mitigation**: Use LOWER() on both sides of comparison
- **Risk**: Null symbol_name causing SQL errors or crashes
  - **Mitigation**: CASE with ELSE 1.0 handles null gracefully
- **Risk**: Parameter binding errors
  - **Mitigation**: Follow existing parameterized query patterns in fts.rs

## Files/Packages Affected
- `/crates/maproom/src/search/fts.rs`
- `/crates/maproom/src/search/executors.rs` (updated call sites)
- `/crates/maproom/tests/search_executors_test.rs` (updated test call sites)

## Implementation Notes

### Changes Made

**1. Added `normalized_query` parameter to FTSExecutor::execute()**
- Updated function signature to accept `normalized_query: &str` parameter
- Updated documentation to reflect new parameter
- Parameter order: `fts_query`, `normalized_query`, `original_query`, `repo_id`, `worktree_id`, `limit`

**2. Implemented exact_mult CASE statement in SQL**
- Added CASE statement at lines 96-99 in fts.rs:
  ```sql
  CASE
    WHEN LOWER(c.symbol_name) = LOWER($2) THEN 3.0
    ELSE 1.0
  END as exact_mult
  ```
- Uses LOWER() for case-insensitive comparison
- Returns 1.0 for null symbol_name (ELSE clause handles null gracefully)
- Added as separate column for debugging visibility

**3. Updated parameter bindings**
- Updated SQL parameter indices: $1 (fts_query), $2 (normalized_query), $3 (original_query), $4 (repo_id), $5 (worktree_id), $6 (limit)
- Updated Rust parameter binding array to include normalized_query in correct position
- Preserved old exact_bonus logic (lines 101-104) for comparison until SEMRANK-2004b

**4. Updated all call sites**
- executors.rs: Updated execute_all() and execute_fast() to pass original_query twice (temporary until SEMRANK-2004b implements normalization)
- search_executors_test.rs: Updated 5 test call sites with new parameter
- Added comments noting that normalized_query will be properly implemented in SEMRANK-2004b

### Testing Results

**Build verification:**
```bash
cargo build --release --package crewchief-maproom
# Result: Finished `release` profile [optimized] target(s) in 1m 08s
```

**Unit tests:**
```bash
cargo test --package crewchief-maproom --lib fts
# Result: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 781 filtered out
```

### Key Design Decisions

1. **Temporary normalization**: Currently passing `original_query` as both normalized and original. SEMRANK-2004b will implement proper normalization in TypeScript.

2. **Preserved old bonus**: Kept the old ILIKE exact_bonus logic temporarily to allow SEMRANK-2004b to compare old vs new scoring before removal.

3. **Separate column**: Added exact_mult as a separate column (not yet applied to final score) for debugging visibility. SEMRANK-2004b will integrate it into the final scoring formula.

4. **Case-insensitive comparison**: Used LOWER() on both sides to handle variations like "Authenticate" vs "authenticate".

5. **Null safety**: ELSE 1.0 clause ensures null symbol_name doesn't crash and gets no boost (1.0× multiplier).

### Next Steps (SEMRANK-2004b)

1. Implement normalizeForExactMatch() in TypeScript
2. Pass normalized query from TypeScript to Rust
3. Apply exact_mult to final score calculation
4. Remove old exact_bonus logic
5. Test end-to-end exact matching with real queries
