# Ticket: SQLITE-4001: FTS Module Extraction

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Extract FTS5 search logic from `mod.rs` into a dedicated `fts.rs` module with improved rank normalization and worktree filtering.

## Background
The existing FTS5 search in `mod.rs` works but needs refactoring to support hybrid search. The rank needs normalization to a 0-1 scale for RRF fusion, and worktree filtering needs to use the new junction table.

Implements: Plan Phase 4 - Hybrid Search

## Acceptance Criteria
- [ ] `fts.rs` module created with extracted FTS logic
- [ ] `search_chunks_fts()` returns `FtsResult` with chunk_id, rank, and position
- [ ] FTS5 rank normalized to 0-1 scale (higher = better)
- [ ] Query building handles edge cases (special chars, empty query)
- [ ] Worktree filtering uses junction table JOIN
- [ ] Existing FTS tests continue to pass
- [ ] New tests for rank normalization

## Technical Requirements
Create `crates/maproom/src/db/sqlite/fts.rs`:

```rust
pub struct FtsResult {
    pub chunk_id: i64,
    pub rank: f64,           // Original FTS5 rank (negative)
    pub normalized_rank: f64, // 0-1 scale (higher = better)
    pub position: usize,      // Position in result set (0-indexed)
}

/// Normalize FTS5 rank to 0-1 scale
/// FTS5 rank: negative values, more negative = better match
pub fn normalize_fts_rank(rank: f64) -> f64 {
    1.0 / (1.0 + rank.abs())
}

/// Build FTS5 query from user input
/// Handles special characters, multi-word queries
pub fn build_fts_query(query: &str) -> String {
    // Sanitize special FTS5 characters
    let sanitized = query
        .replace('"', "")
        .replace('\'', "")
        .replace('*', "")
        .replace('(', "")
        .replace(')', "")
        .replace('-', " ")
        .replace(':', " ");

    // Build OR query for multiple words
    let words: Vec<&str> = sanitized.split_whitespace().collect();
    if words.is_empty() {
        return String::new();
    }

    // Each word with prefix matching
    words.iter()
        .map(|w| format!("{}*", w))
        .collect::<Vec<_>>()
        .join(" OR ")
}

impl SqliteStore {
    pub async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        limit: usize,
    ) -> Result<Vec<FtsResult>> {
        let fts_query = build_fts_query(query);
        if fts_query.is_empty() {
            return Ok(vec![]);
        }

        self.run(move |conn| {
            let sql = r#"
                SELECT c.id, fts.rank
                FROM fts_chunks fts
                JOIN chunks c ON c.id = fts.rowid
                JOIN files f ON f.id = c.file_id
                JOIN repos r ON r.id = f.repo_id
                LEFT JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                LEFT JOIN worktrees w ON w.id = cw.worktree_id
                WHERE fts_chunks MATCH ?1
                  AND r.name = ?2
                  AND (?3 IS NULL OR w.name = ?3)
                ORDER BY fts.rank ASC
                LIMIT ?4
            "#;

            let mut stmt = conn.prepare(sql)?;
            let results = stmt.query_map(
                params![fts_query, repo, worktree, limit],
                |row| {
                    let chunk_id: i64 = row.get(0)?;
                    let rank: f64 = row.get(1)?;
                    Ok(FtsResult {
                        chunk_id,
                        rank,
                        normalized_rank: normalize_fts_rank(rank),
                        position: 0,  // Will be set after collecting
                    })
                }
            )?;

            let mut results: Vec<_> = results.collect::<Result<Vec<_>, _>>()?;
            for (i, result) in results.iter_mut().enumerate() {
                result.position = i;
            }
            Ok(results)
        }).await
    }
}
```

## Implementation Notes
- FTS5 rank is negative (more negative = better), ORDER BY ASC for best first
- Normalization formula: `1 / (1 + abs(rank))` gives 0-1 where 1 is best
- Position is 0-indexed rank in result set, used for RRF
- Query sanitization prevents FTS5 syntax errors
- Use prefix matching (`word*`) for better partial matches

## Dependencies
- SQLITE-1002 (CRUD Junction Table) - worktree filtering via junction

## Risk Assessment
- **Risk**: Breaking existing FTS tests
  - **Mitigation**: Run existing tests after refactoring; preserve behavior
- **Risk**: Query sanitization removes valid search terms
  - **Mitigation**: Only remove FTS5 syntax characters, keep alphanumeric

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/fts.rs` (NEW)
- `crates/maproom/src/db/sqlite/mod.rs` (refactor to use fts module)
