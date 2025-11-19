# Ticket: SEMRANK-2005: Combine Multipliers into Final Score

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
Multiply base FTS score by kind_mult and exact_mult, update ORDER BY clause to use final_score, preserve base_score for debug mode.

## Background
At this point, kind_mult (SEMRANK-2003) and exact_mult (SEMRANK-2004a) are computed as separate columns but not yet combined into ranking. This ticket implements the core semantic ranking formula: `final_score = ts_rank_cd() × kind_mult × exact_mult`.

This multiplicative approach means a function with exact match gets 2.5 × 3.0 = 7.5× boost over baseline, while a heading with no exact match gets 0.6× (demotion). The separate multipliers compound to create strong semantic differentiation in rankings.

Preserving base_score enables comparison and debugging of the semantic enhancement effects.

References SEMRANK plan Section 3.2.3 (Final Score Computation).

## Acceptance Criteria
- [ ] Final score computed: `base_score × kind_mult × exact_mult`
- [ ] ORDER BY clause updated to use `final_score DESC`
- [ ] base_score preserved as separate column for debug mode
- [ ] Results ordered correctly: Higher final_score ranks higher
- [ ] Query tested with known cases: func + exact match should rank highest
- [ ] Edge case verified: Null symbol_name doesn't crash (exact_mult = 1.0, multiplication safe)

## Technical Requirements
- Location: Rust FTS implementation `/crates/maproom/src/search/fts.rs`
- SQL modification to add final_score column:
  ```sql
  SELECT
    c.id,
    c.symbol_name,
    c.kind,
    c.relpath,
    c.preview,
    ts_rank_cd(c.ts_doc, query) AS base_score,
    [kind_mult CASE from SEMRANK-2003],
    [exact_mult CASE from SEMRANK-2004a],
    ts_rank_cd(c.ts_doc, query) * [kind_mult] * [exact_mult] AS final_score
  FROM maproom.chunks c
  WHERE c.ts_doc @@ query
  ORDER BY final_score DESC
  LIMIT $limit
  ```
- Ensure multiplication happens correctly (no integer truncation)
- Preserve all intermediate values (base_score, kind_mult, exact_mult) for debug mode
- Update ORDER BY from `base_score DESC` to `final_score DESC`

## Implementation Notes
**Step 1: Add Final Score Computation**
- Build on top of SEMRANK-2003 (kind_mult) and SEMRANK-2004a (exact_mult)
- Multiply ts_rank_cd() × kind_mult × exact_mult
- Verify no type conversion issues (all should be floating point)

**Step 2: Update ORDER BY**
- Change ORDER BY clause to use final_score DESC
- Verify results are now ordered by combined score

**Step 3: Testing**
- Test with query "authenticate" on test corpus
- Expected: func with exact match should rank first (2.5 × 3.0 = 7.5× boost)
- Verify tests rank below implementations (0.8× vs 2.5×)
- Check that docs rank below both func and tests (0.6× heading penalty)

**Validation Checklist**:
- [ ] func + exact match ranks highest
- [ ] func without exact match ranks above tests
- [ ] tests rank above docs
- [ ] headings rank lowest

## Dependencies
- SEMRANK-2003 (kind_mult column exists)
- SEMRANK-2004a (exact_mult column exists)

## Risk Assessment
- **Risk**: Multiplication overflow with large scores
  - **Mitigation**: Unlikely with these multiplier values (max ~7.5×), but verify with tests
- **Risk**: ORDER BY using wrong column
  - **Mitigation**: Double-check ORDER BY uses final_score, not base_score
- **Risk**: Null handling causing multiplication errors
  - **Mitigation**: All CASEs have ELSE 1.0, so multiplication always safe

## Files/Packages Affected
- `/crates/maproom/src/search/fts.rs`
