# Ticket: SRCHREL-1001 - Modify Database Query

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (1135/1135 maproom tests pass)
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary

Implement quality-weighted SQL query in the database layer with hardcoded edge quality weights. Modify `calculate_graph_importance()` to support both legacy and enhanced modes via feature flag parameter.

## Background

The current graph importance calculation uses simple edge counting with logarithmic scaling. This ticket enhances it with quality weights:
- Production code edges: weight 1.0
- Test code edges: weight 0.5 (penalty)
- Edge type (calls): weight 1.0 (Phase 1 only supports calls)

The implementation must preserve the existing SQL path for backward compatibility, using the `enable_quality` parameter to switch between modes.

## Acceptance Criteria

- [ ] Add `enable_quality: bool` parameter to `calculate_graph_importance()` signature
- [ ] Implement quality-weighted SQL query with hardcoded weights
- [ ] Keep existing SQL as fallback when `enable_quality = false`
- [ ] SQL query uses test detection patterns (file path LIKE patterns)
- [ ] SQL query performs quality weight multiplication (edge type × source type)
- [ ] SQL query applies LOG scaling to quality-weighted sum
- [ ] Both old and new SQL paths compile and execute
- [ ] Results differ between old/new modes (quality weights applied)
- [ ] SQL query returns Vec<(i64, f32)> (chunk_id, graph_score)
- [ ] Query handles NULL values gracefully (COALESCE)
- [ ] Query limits results (ORDER BY score DESC LIMIT ?)

## Technical Requirements

**Modified Function Signature:**

```rust
// In crates/maproom/src/db/sqlite/mod.rs
impl SqliteStore {
    pub fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool, // NEW PARAMETER
    ) -> Result<Vec<(i64, f32)>, DbError> {
        if !enable_quality {
            // Call existing legacy implementation
            return self.calculate_graph_importance_legacy(repo_id, worktree_id, limit);
        }

        // New quality-weighted implementation
        self.calculate_graph_importance_quality(repo_id, worktree_id, limit)
    }

    // Rename existing implementation
    fn calculate_graph_importance_legacy(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<(i64, f32)>, DbError> {
        // Existing SQL query (unchanged)
        // ...
    }

    // New quality-weighted implementation
    fn calculate_graph_importance_quality(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<(i64, f32)>, DbError> {
        // Quality-weighted SQL query (see below)
        // ...
    }
}
```

**Quality-Weighted SQL Query:**

```sql
-- Quality-weighted graph importance calculation
WITH quality_edges AS (
  SELECT
    ce.dst_chunk_id as chunk_id,
    -- Edge type weight (Phase 1: calls only, hardcoded to 1.0)
    CASE ce.type
      WHEN 'calls' THEN 1.0
      ELSE 1.0  -- Default for any other edge type
    END *
    -- Source code type weight (test detection via file path)
    CASE
      WHEN src_file.relpath LIKE '%/test/%'
        OR src_file.relpath LIKE '%/tests/%'
        OR src_file.relpath LIKE '%/__tests__/%'
        OR src_file.relpath LIKE '%.test.ts%'
        OR src_file.relpath LIKE '%.test.js%'
        OR src_file.relpath LIKE '%.test.tsx%'
        OR src_file.relpath LIKE '%.test.jsx%'
        OR src_file.relpath LIKE '%.spec.ts%'
        OR src_file.relpath LIKE '%.spec.js%'
        OR src_file.relpath LIKE '%_test.rs%'
        OR src_file.relpath LIKE '%_test.py%'
        OR src_chunk.kind LIKE '%test%'
      THEN 0.5  -- Test code penalty (hardcoded)
      ELSE 1.0  -- Production code baseline (hardcoded)
    END as edge_quality
  FROM chunk_edges ce
  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
  JOIN files src_file ON src_file.id = src_chunk.file_id
  WHERE ce.dst_chunk_id IN (
    SELECT c.id FROM chunks c
    JOIN files f ON f.id = c.file_id
    WHERE f.repo_id = ?1
      AND (?2 IS NULL OR f.worktree_id = ?2)
  )
),
importance_scores AS (
  SELECT
    chunk_id,
    SUM(edge_quality) as quality_weighted_sum
  FROM quality_edges
  GROUP BY chunk_id
)
SELECT
  chunk_id,
  LOG(2.0 + COALESCE(quality_weighted_sum, 0.0)) as graph_score
FROM importance_scores
ORDER BY graph_score DESC
LIMIT ?3;
```

**Rust Implementation:**

```rust
fn calculate_graph_importance_quality(
    &self,
    repo_id: i64,
    worktree_id: Option<i64>,
    limit: usize,
) -> Result<Vec<(i64, f32)>, DbError> {
    let conn = self.pool.get()?;

    let mut stmt = conn.prepare(
        r#"
        WITH quality_edges AS (
          SELECT
            ce.dst_chunk_id as chunk_id,
            CASE ce.type
              WHEN 'calls' THEN 1.0
              ELSE 1.0
            END *
            CASE
              WHEN src_file.relpath LIKE '%/test/%'
                OR src_file.relpath LIKE '%/tests/%'
                OR src_file.relpath LIKE '%/__tests__/%'
                OR src_file.relpath LIKE '%.test.ts%'
                OR src_file.relpath LIKE '%.test.js%'
                OR src_file.relpath LIKE '%.test.tsx%'
                OR src_file.relpath LIKE '%.test.jsx%'
                OR src_file.relpath LIKE '%.spec.ts%'
                OR src_file.relpath LIKE '%.spec.js%'
                OR src_file.relpath LIKE '%_test.rs%'
                OR src_file.relpath LIKE '%_test.py%'
                OR src_chunk.kind LIKE '%test%'
              THEN 0.5
              ELSE 1.0
            END as edge_quality
          FROM chunk_edges ce
          JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
          JOIN files src_file ON src_file.id = src_chunk.file_id
          WHERE ce.dst_chunk_id IN (
            SELECT c.id FROM chunks c
            JOIN files f ON f.id = c.file_id
            WHERE f.repo_id = ?1
              AND (?2 IS NULL OR f.worktree_id = ?2)
          )
        ),
        importance_scores AS (
          SELECT
            chunk_id,
            SUM(edge_quality) as quality_weighted_sum
          FROM quality_edges
          GROUP BY chunk_id
        )
        SELECT
          chunk_id,
          LOG(2.0 + COALESCE(quality_weighted_sum, 0.0)) as graph_score
        FROM importance_scores
        ORDER BY graph_score DESC
        LIMIT ?3
        "#,
    )?;

    let rows = stmt.query_map(
        params![repo_id, worktree_id, limit as i64],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,   // chunk_id
                row.get::<_, f64>(1)? as f32,   // graph_score (convert from f64)
            ))
        },
    )?;

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
```

## Implementation Notes

**Hardcoded Weights (Phase 1):**
- Production code: `1.0`
- Test code: `0.5`
- Calls edge type: `1.0`
- These will be configurable in Phase 2

**Test Detection Patterns:**
Based on SRCHREL-0003 validation results. Patterns cover:
- JavaScript/TypeScript: `.test.ts`, `.test.js`, `.spec.ts`, `.spec.js`, `__tests__/`
- Rust: `_test.rs`, `/tests/`
- Python: `_test.py`, `/test/`
- Generic: `/test/`, `/tests/` directories

**SQL Parameter Binding:**
- `?1` = `repo_id` (i64)
- `?2` = `worktree_id` (Option<i64>)
- `?3` = `limit` (i64)

**Error Handling:**
- Database connection errors: Propagate via `?` operator
- SQL errors: Propagate via `?` operator
- Row parsing errors: Propagate via `?` operator
- NULL values: Handled by `COALESCE(quality_weighted_sum, 0.0)`

**Performance:**
- Expected latency: 20-30ms p95 (validated in SRCHREL-0002)
- Uses existing indexes on `chunk_edges(dst_chunk_id)`, `chunks(file_id)`, `files(id)`
- No new indexes required

**Logging:**
Add debug logging to track query execution:
```rust
use tracing::debug;

debug!(
    repo_id = repo_id,
    worktree_id = ?worktree_id,
    limit = limit,
    enable_quality = enable_quality,
    "Calculating graph importance"
);
```

## Dependencies

**Prerequisites:**
- SRCHREL-0001 (schema validation complete)
- SRCHREL-0002 (SQL performance validated)
- SRCHREL-0003 (test detection patterns validated)

**Blocks:**
- SRCHREL-1003 (graph executor needs database layer to be ready)

## Risk Assessment

**Risk:** SQL syntax error in complex query
**Mitigation:** Test with SQLite CLI first, validate with unit test

**Risk:** Performance worse than expected
**Mitigation:** SRCHREL-0002 validated performance, fallback to legacy mode if issues

**Risk:** Test detection patterns produce unexpected results
**Mitigation:** SRCHREL-0003 validated patterns, can tune in follow-up if needed

## Files/Packages Affected

**Modified Files:**
- `crates/maproom/src/db/sqlite/mod.rs` (modify `calculate_graph_importance()`, add quality-weighted implementation)

**Dependencies:**
- `rusqlite` (already in Cargo.toml)
- `tracing` (already in Cargo.toml)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Task 1.1, lines 171-181)
- Architecture: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/architecture.md` (SQL query design, lines 248-356)
- Performance validation: Results from SRCHREL-0002
