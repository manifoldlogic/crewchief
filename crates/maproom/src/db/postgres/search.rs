//! `StoreSearch` — Phase-2 deliverable (FTS + vector + hybrid parity, §6.6).
//!
//! Phase-1 stubs: verbatim signatures, empty/default returns, `// PARITY-TODO`.
//! Per the spec's Phase-1 stub rule these compile with no `todo!()`; their
//! behavior is not asserted by Phase-1 parity tests.

use std::collections::HashMap;

use async_trait::async_trait;

use super::PostgresStore;
use crate::db::traits::StoreSearch;
use crate::db::types::{
    ChunkMetadata, HybridResult, HybridWeights, RankedSearchHit, SemanticRanking,
};
use crate::db::SearchHit;

#[allow(unused_variables)]
#[async_trait]
impl StoreSearch for PostgresStore {
    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<(Vec<SearchHit>, usize)> {
        // PARITY-TODO(Phase 2): tsquery OR-of-prefix + ts_rank + total_count.
        Ok((Vec::new(), 0))
    }

    async fn search_fts_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query: &str,
        normalized_query: &str,
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 2).
        Ok(Vec::new())
    }

    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 2): pgvector <-> L2 KNN; FTS-only degrade when !has_vector_extension.
        Ok(Vec::new())
    }

    async fn search_vector_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query_embedding: &[f32],
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 2).
        Ok(Vec::new())
    }

    async fn search_chunks_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>> {
        // PARITY-TODO(Phase 2).
        Ok(Vec::new())
    }

    async fn search_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
    ) -> anyhow::Result<Vec<HybridResult>> {
        // PARITY-TODO(Phase 2): reuse shared RRF fusion.
        Ok(Vec::new())
    }

    async fn search_hybrid_ranked(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
        ranking: SemanticRanking,
    ) -> anyhow::Result<Vec<RankedSearchHit>> {
        // PARITY-TODO(Phase 2): reuse shared apply_semantic_ranking.
        Ok(Vec::new())
    }

    async fn get_chunks_metadata(
        &self,
        chunk_ids: &[i64],
    ) -> anyhow::Result<HashMap<i64, ChunkMetadata>> {
        // PARITY-TODO(Phase 2).
        Ok(HashMap::new())
    }
}
