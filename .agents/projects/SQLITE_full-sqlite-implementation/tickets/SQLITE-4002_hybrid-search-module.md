# Ticket: SQLITE-4002: Hybrid Search Module

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
Implement hybrid search that combines FTS5 and vector search results using Reciprocal Rank Fusion (RRF).

## Background
Hybrid search provides the best of both worlds: keyword matching (FTS5) and semantic similarity (vectors). RRF is a proven algorithm for combining ranked lists from different sources without requiring score normalization.

Implements: Plan Phase 4 - Hybrid Search

## Acceptance Criteria
- [ ] `hybrid.rs` module created with RRF fusion logic
- [ ] `search_hybrid()` combines FTS and vector results
- [ ] RRF score calculated correctly with k=60 constant
- [ ] Configurable weights for FTS vs vector contribution
- [ ] Falls back to FTS-only when no embeddings/extension available
- [ ] Returns `SearchHit` with combined scores and source info
- [ ] Results deduped by chunk_id (same chunk may appear in both sources)
- [ ] Tests verify RRF calculation and fallback behavior

## Technical Requirements
Create `crates/maproom/src/db/sqlite/hybrid.rs`:

```rust
const RRF_K: f64 = 60.0;  // Standard RRF constant

pub struct HybridWeights {
    pub fts_weight: f64,     // Default 0.3
    pub vector_weight: f64,  // Default 0.7
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            fts_weight: 0.3,
            vector_weight: 0.7,
        }
    }
}

pub struct SearchHit {
    pub chunk_id: i64,
    pub score: f64,           // Combined RRF score
    pub fts_rank: Option<usize>,    // Position in FTS results
    pub vector_rank: Option<usize>, // Position in vector results
}

/// Calculate RRF score for a single result
fn rrf_score(
    fts_rank: Option<usize>,
    vec_rank: Option<usize>,
    weights: &HybridWeights,
) -> f64 {
    let fts_contribution = fts_rank
        .map(|r| weights.fts_weight / (RRF_K + r as f64))
        .unwrap_or(0.0);

    let vec_contribution = vec_rank
        .map(|r| weights.vector_weight / (RRF_K + r as f64))
        .unwrap_or(0.0);

    fts_contribution + vec_contribution
}

impl SqliteStore {
    pub async fn search_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
    ) -> Result<Vec<SearchHit>> {
        // Over-fetch from each source for better fusion
        let fetch_limit = limit * 3;

        // Run FTS and vector search in parallel
        let (fts_results, vec_results) = tokio::join!(
            self.search_chunks_fts(repo, worktree, query, fetch_limit),
            self.search_vector(repo, worktree, query_embedding, fetch_limit),
        );

        let fts_results = fts_results?;
        let vec_results = vec_results?;

        // Build lookup maps: chunk_id -> rank
        let fts_ranks: HashMap<i64, usize> = fts_results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.chunk_id, i))
            .collect();

        let vec_ranks: HashMap<i64, usize> = vec_results
            .iter()
            .enumerate()
            .map(|(i, r)| (r.chunk_id, i))
            .collect();

        // Get all unique chunk_ids
        let all_chunk_ids: HashSet<i64> = fts_ranks.keys()
            .chain(vec_ranks.keys())
            .copied()
            .collect();

        // Calculate RRF scores
        let mut hits: Vec<SearchHit> = all_chunk_ids
            .into_iter()
            .map(|chunk_id| {
                let fts_rank = fts_ranks.get(&chunk_id).copied();
                let vec_rank = vec_ranks.get(&chunk_id).copied();
                SearchHit {
                    chunk_id,
                    score: rrf_score(fts_rank, vec_rank, &weights),
                    fts_rank,
                    vector_rank: vec_rank,
                }
            })
            .collect();

        // Sort by score (descending) and take top N
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        hits.truncate(limit);

        Ok(hits)
    }
}
```

## Implementation Notes
- RRF formula: `1 / (k + rank)` where rank is 0-indexed position
- k=60 is standard constant that balances contribution across ranks
- Over-fetch by 3x to ensure good coverage after fusion
- Dedup by chunk_id since same chunk may match both FTS and vector
- Fall back to FTS-only automatically when vec_results is empty

## Dependencies
- SQLITE-4001 (FTS Module Extraction) - FTS search with proper rank
- SQLITE-3001 (Vector Search Module) - vector similarity search

## Risk Assessment
- **Risk**: Different result counts from FTS vs vector skew fusion
  - **Mitigation**: RRF handles this naturally; items in both lists boosted
- **Risk**: Over-fetching too many results is slow
  - **Mitigation**: 3x is reasonable; profile and adjust if needed

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/hybrid.rs` (NEW)
- `crates/maproom/src/db/sqlite/mod.rs` (export hybrid module)
