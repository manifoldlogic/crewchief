# Ticket: SQLITE-4003: Semantic Ranking

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement semantic ranking that applies domain-specific score adjustments (kind multipliers, exact match boost) to hybrid search results.

## Background
Raw search scores don't account for code semantics. Functions and classes are typically more important than variables. Exact symbol name matches should rank higher. Semantic ranking applies these domain-specific adjustments.

Implements: Plan Phase 4 - Hybrid Search

## Acceptance Criteria
- [x] `SemanticRanking` struct with configurable multipliers
- [x] Kind multipliers applied (function=1.2, class=1.1, variable=0.8, etc.)
- [x] Exact match boost applied when symbol name matches query
- [x] Uses existing `normalize_for_exact_match()` from `src/search/fts.rs`
- [x] Recency score factored in (if available)
- [x] Integrated into hybrid search pipeline
- [x] Tests verify multipliers apply correctly

## Technical Requirements
Add to `crates/maproom/src/db/sqlite/hybrid.rs`:

```rust
use crate::search::fts::normalize_for_exact_match;

pub struct SemanticRanking {
    pub kind_multipliers: HashMap<String, f64>,
    pub exact_match_boost: f64,
    pub recency_weight: f64,  // How much recency affects score
}

impl Default for SemanticRanking {
    fn default() -> Self {
        let mut kind_multipliers = HashMap::new();
        kind_multipliers.insert("function".to_string(), 1.2);
        kind_multipliers.insert("method".to_string(), 1.2);
        kind_multipliers.insert("class".to_string(), 1.1);
        kind_multipliers.insert("struct".to_string(), 1.1);
        kind_multipliers.insert("interface".to_string(), 1.1);
        kind_multipliers.insert("trait".to_string(), 1.1);
        kind_multipliers.insert("enum".to_string(), 1.0);
        kind_multipliers.insert("module".to_string(), 1.0);
        kind_multipliers.insert("constant".to_string(), 0.9);
        kind_multipliers.insert("variable".to_string(), 0.8);
        kind_multipliers.insert("import".to_string(), 0.7);

        Self {
            kind_multipliers,
            exact_match_boost: 1.5,
            recency_weight: 0.1,  // Small boost for recent changes
        }
    }
}

/// Extended search hit with chunk metadata for ranking
pub struct RankedSearchHit {
    pub chunk_id: i64,
    pub score: f64,
    pub fts_rank: Option<usize>,
    pub vector_rank: Option<usize>,
    pub kind: String,
    pub symbol_name: Option<String>,
    pub recency_score: f64,
}

/// Apply semantic ranking to search results
pub fn apply_semantic_ranking(
    results: &mut [RankedSearchHit],
    query: &str,
    ranking: &SemanticRanking,
) {
    let normalized_query = normalize_for_exact_match(query);

    for hit in results.iter_mut() {
        let mut multiplier = 1.0;

        // Apply kind multiplier
        if let Some(&kind_mult) = ranking.kind_multipliers.get(&hit.kind) {
            multiplier *= kind_mult;
        }

        // Apply exact match boost
        if let Some(ref symbol) = hit.symbol_name {
            let normalized_symbol = normalize_for_exact_match(symbol);
            if normalized_symbol.to_lowercase().contains(&normalized_query.to_lowercase()) {
                multiplier *= ranking.exact_match_boost;
            }
        }

        // Apply recency factor (small boost for recently modified)
        // recency_score is 0-1 where 1 = most recent
        let recency_boost = 1.0 + (hit.recency_score * ranking.recency_weight);
        multiplier *= recency_boost;

        hit.score *= multiplier;
    }

    // Re-sort after applying multipliers
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
}
```

Update `search_hybrid()` to fetch chunk metadata and apply ranking:
```rust
pub async fn search_hybrid_ranked(
    &self,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    query_embedding: &[f32],
    limit: usize,
    weights: HybridWeights,
    ranking: SemanticRanking,
) -> Result<Vec<RankedSearchHit>> {
    // 1. Get base hybrid results
    let hits = self.search_hybrid(repo, worktree, query, query_embedding, limit * 2, weights).await?;

    // 2. Fetch chunk metadata (kind, symbol_name, recency)
    let chunk_ids: Vec<i64> = hits.iter().map(|h| h.chunk_id).collect();
    let metadata = self.get_chunks_metadata(&chunk_ids).await?;

    // 3. Build ranked hits with metadata
    let mut ranked: Vec<RankedSearchHit> = hits.into_iter()
        .filter_map(|h| {
            let meta = metadata.get(&h.chunk_id)?;
            Some(RankedSearchHit {
                chunk_id: h.chunk_id,
                score: h.score,
                fts_rank: h.fts_rank,
                vector_rank: h.vector_rank,
                kind: meta.kind.clone(),
                symbol_name: meta.symbol_name.clone(),
                recency_score: meta.recency_score,
            })
        })
        .collect();

    // 4. Apply semantic ranking
    apply_semantic_ranking(&mut ranked, query, &ranking);

    // 5. Take top N after re-ranking
    ranked.truncate(limit);
    Ok(ranked)
}
```

## Implementation Notes
- Import `normalize_for_exact_match` from existing `src/search/fts.rs`
- The function handles camelCase, XMLParser, HTTPSHandler, kebab-case
- Over-fetch by 2x before ranking to ensure good results after re-ordering
- Recency score should be 0-1 from chunks.recency_score column

## Dependencies
- SQLITE-4002 (Hybrid Search Module) - base hybrid search to enhance

## Risk Assessment
- **Risk**: Multipliers significantly change ranking order
  - **Mitigation**: Conservative multipliers (1.2 max); can tune based on feedback
- **Risk**: Performance overhead from fetching metadata
  - **Mitigation**: Single batch query for all chunk metadata

## Files/Packages Affected
- `crates/maproom/src/db/sqlite/hybrid.rs` (add semantic ranking)

---

## Implementation Notes (rust-indexer-engineer)

### Summary
Implemented semantic ranking that applies domain-specific score adjustments to hybrid search results. Added configurable kind multipliers, exact match boost, and recency weighting.

### Files Modified

**Modified: `crates/maproom/src/db/sqlite/hybrid.rs` (~200 lines added)**
- `SemanticRanking` struct with configurable multipliers and weights
- `ChunkMetadata` struct for batch metadata retrieval
- `RankedSearchHit` struct with full metadata for ranked results
- `apply_semantic_ranking()` function applies multipliers and re-sorts
- 13 unit tests for semantic ranking functionality

**Modified: `crates/maproom/src/db/sqlite/mod.rs` (~115 lines added)**
- `get_chunks_metadata()` batch query for chunk kind/symbol/recency
- `search_hybrid_ranked()` combines hybrid search with semantic ranking
- 3 integration tests for metadata retrieval and ranked search

### Key Implementation Details

**SemanticRanking Configuration:**
- Default multipliers: function/method=1.2, class/struct/interface/trait=1.1, enum/module=1.0, constant=0.9, variable=0.8, import=0.7
- Exact match boost: 1.5 (when normalized symbol contains normalized query)
- Recency weight: 0.1 (small boost for recently modified chunks)
- `SemanticRanking::identity()` provides no-op ranking for testing

**Exact Match Detection:**
- Uses `normalize_for_exact_match()` from `crate::search::fts`
- Normalizes camelCase, kebab-case, acronyms to snake_case lowercase
- Case-insensitive substring match after normalization
- Example: query "user" matches symbol "validateUserCredentials"

**Score Calculation:**
```
final_score = base_score * kind_multiplier * exact_match_boost * recency_boost
recency_boost = 1.0 + (recency_score * recency_weight)
```

**Pipeline Integration:**
1. `search_hybrid()` returns base RRF results (over-fetched by 2x)
2. `get_chunks_metadata()` batch fetches kind/symbol/recency for all chunk_ids
3. `apply_semantic_ranking()` applies multipliers and re-sorts
4. Final results truncated to requested limit

### Test Results
```
running 78 tests
test db::sqlite::hybrid::tests::test_semantic_ranking_defaults ... ok
test db::sqlite::hybrid::tests::test_semantic_ranking_identity ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_kind_multipliers ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_exact_match_boost ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_exact_match_camel_case ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_exact_match_partial_name ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_recency_factor ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_combined_factors ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_reorders_results ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_unknown_kind ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_empty_results ... ok
test db::sqlite::hybrid::tests::test_apply_semantic_ranking_no_symbol_name ... ok
test db::sqlite::hybrid::tests::test_semantic_ranking_custom ... ok
test db::sqlite::tests::test_get_chunks_metadata ... ok
test db::sqlite::tests::test_search_hybrid_ranked_integration ... ok
test db::sqlite::tests::test_search_hybrid_ranked_identity_ranking ... ok
...
test result: ok. 78 passed; 0 failed; 0 ignored
```

### Acceptance Criteria Verification

| Criterion | Evidence |
|-----------|----------|
| SemanticRanking struct | Lines 89-144 in hybrid.rs |
| Kind multipliers applied | test_apply_semantic_ranking_kind_multipliers |
| Exact match boost applied | test_apply_semantic_ranking_exact_match_boost |
| Uses normalize_for_exact_match | Line 191 in hybrid.rs |
| Recency score factored in | test_apply_semantic_ranking_recency_factor |
| Integrated into hybrid search | search_hybrid_ranked() in mod.rs |
| Tests verify multipliers | 13 unit tests + 3 integration tests |
