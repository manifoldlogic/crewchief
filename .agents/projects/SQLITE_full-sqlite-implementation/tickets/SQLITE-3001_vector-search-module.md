# Ticket: SQLITE-3001: Vector Search Module

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
Implement vector similarity search using sqlite-vec to find chunks with embeddings similar to a query embedding.

## Background
Vector search enables semantic similarity - finding code that's conceptually similar even without exact keyword matches. sqlite-vec provides KNN search via the MATCH operator. Results need to be joined back to chunks and filtered by repo/worktree.

Implements: Plan Phase 3 - Vector Search

## Acceptance Criteria
- [ ] `vector.rs` module created with search functionality
- [ ] `search_vector(repo, worktree, query_embedding, limit)` returns similar chunks
- [ ] Results include chunk_id, distance, and can be joined to full chunk data
- [ ] L2 distance converted to similarity score (0-1, higher = better)
- [ ] Worktree filtering works via junction table JOIN
- [ ] Empty results returned (not error) when extension missing
- [ ] Empty results returned when no embeddings indexed
- [ ] Results sorted by similarity (best first)

## Technical Requirements
Create `crates/maproom/src/db/sqlite/vector.rs`:

```rust
pub struct VectorResult {
    pub chunk_id: i64,
    pub distance: f64,
    pub similarity: f64,  // Normalized 0-1
}

impl SqliteStore {
    /// Search for similar chunks by embedding
    pub async fn search_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<VectorResult>> {
        if !self.has_vec_extension() {
            return Ok(vec![]);
        }

        let query_blob = vec_to_blob(query_embedding);

        self.run(move |conn| {
            let sql = r#"
                SELECT c.id, v.distance
                FROM vec_code v
                JOIN code_embeddings e ON e.id = v.rowid
                JOIN chunks c ON c.blob_sha = e.blob_sha
                JOIN files f ON f.id = c.file_id
                JOIN repos r ON r.id = f.repo_id
                LEFT JOIN chunk_worktrees cw ON cw.chunk_id = c.id
                LEFT JOIN worktrees w ON w.id = cw.worktree_id
                WHERE v.embedding MATCH ?1
                  AND r.name = ?2
                  AND (?3 IS NULL OR w.name = ?3)
                ORDER BY v.distance ASC
                LIMIT ?4
            "#;

            let mut stmt = conn.prepare(sql)?;
            let results = stmt.query_map(
                params![query_blob, repo, worktree, limit],
                |row| {
                    let chunk_id: i64 = row.get(0)?;
                    let distance: f64 = row.get(1)?;
                    Ok(VectorResult {
                        chunk_id,
                        distance,
                        similarity: distance_to_similarity(distance),
                    })
                }
            )?;

            results.collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow::anyhow!("{}", e))
        }).await
    }
}

/// Convert L2 distance to similarity score (0-1, higher = better)
fn distance_to_similarity(distance: f64) -> f64 {
    // L2 distance: 0 = identical, larger = more different
    // Convert to 0-1 where 1 = identical
    1.0 / (1.0 + distance)
}
```

## Implementation Notes
- sqlite-vec MATCH operator returns results sorted by distance (ascending)
- Distance is L2 (Euclidean) - lower is more similar
- Query embedding must be same dimension (1536) as indexed embeddings
- The join path is: vec_code.rowid → code_embeddings.id → chunks.blob_sha
- Worktree filter is optional (NULL = all worktrees)
- Over-fetch slightly for hybrid search (will combine with FTS results)

## Dependencies
- SQLITE-2002 (Vector Table Population) - vec_code must be populated
- SQLITE-1002 (CRUD Junction Table) - worktree filtering via junction

## Risk Assessment
- **Risk**: Query embedding dimension mismatch
  - **Mitigation**: Validate dimension matches 1536; return clear error
- **Risk**: Slow queries on large indexes
  - **Mitigation**: sqlite-vec is optimized for KNN; limit results; profile if slow

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/vector.rs` (NEW)
- `crates/maproom/src/db/sqlite/mod.rs` (export vector module)
