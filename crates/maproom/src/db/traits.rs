//! Store trait hierarchy for backend-agnostic database operations.
//!
//! Defines 9 sub-traits grouped by functional responsibility, combined into
//! a single `Store` supertrait via blanket impl. This enables both `SqliteStore`
//! and future `PostgresStore` to implement the same interface.
//!
//! # Sub-traits
//!
//! - [`StoreCore`] - Repository, worktree, commit, file CRUD and stats queries
//! - [`StoreChunks`] - Chunk CRUD and worktree associations
//! - [`StoreSearch`] - FTS, vector, and hybrid search
//! - [`StoreGraph`] - Graph traversal and importance scoring
//! - [`StoreEmbeddings`] - Embedding storage and retrieval
//! - [`StoreMigration`] - Schema migration management
//! - [`StoreCleanup`] - Stale worktree detection and cleanup
//! - [`StoreIndexState`] - Index state tracking
//! - [`StoreEncoding`] - Encoding run lifecycle

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;

use crate::config::EdgeQualityWeights;
use crate::db::types::{
    ChunkMetadata, EmbeddingRecord, EncodingRunRow, GraphResult, HybridResult, HybridWeights,
    ImportDirection, RankedSearchHit, SemanticRanking,
};
use crate::db::{
    ChunkContext, ChunkForEmbedding, ChunkFull, ChunkRecord, ChunkSummary, FileRecord, RepoInfo,
    SearchHit, StaleWorktree, WorktreeCleanupResult, WorktreeInfo,
};
use crate::db::index_state::UpdateStats;

// =============================================================================
// StoreCore - Repository, worktree, commit, file CRUD and stats
// =============================================================================

#[async_trait]
pub trait StoreCore: Send + Sync {
    /// Check if sqlite-vec (or equivalent vector extension) is available.
    fn has_vec_extension(&self) -> bool;

    /// Get or create a repository by name and root path. Returns repo ID.
    async fn get_or_create_repo(&self, name: &str, root_path: &str) -> anyhow::Result<i64>;

    /// Get or create a worktree for a repository. Returns worktree ID.
    async fn get_or_create_worktree(
        &self,
        repo_id: i64,
        name: &str,
        abs_path: &str,
    ) -> anyhow::Result<i64>;

    /// Get or create a commit record. Returns commit ID.
    async fn get_or_create_commit(
        &self,
        repo_id: i64,
        sha: &str,
        committed_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> anyhow::Result<i64>;

    /// Look up a repository by name.
    async fn get_repo_by_name(&self, name: &str) -> anyhow::Result<Option<RepoInfo>>;

    /// Look up a worktree by repo ID and name.
    async fn get_worktree_by_name(
        &self,
        repo_id: i64,
        name: &str,
    ) -> anyhow::Result<Option<WorktreeInfo>>;

    /// List all repositories.
    async fn list_repos(&self) -> anyhow::Result<Vec<RepoInfo>>;

    /// List all worktrees for a repository.
    async fn list_worktrees(&self, repo_id: i64) -> anyhow::Result<Vec<WorktreeInfo>>;

    /// Upsert a file record. Returns file ID.
    async fn upsert_file(&self, file: &FileRecord) -> anyhow::Result<i64>;

    /// Delete a file record by ID. Returns true if the file was found and deleted.
    async fn delete_file(&self, file_id: i64) -> anyhow::Result<bool>;

    /// Look up a file ID by relative path and worktree ID.
    async fn get_file_id_by_relpath(
        &self,
        relpath: &str,
        worktree_id: i64,
    ) -> anyhow::Result<Option<i64>>;

    /// Get the count of chunks associated with a worktree.
    async fn get_worktree_chunk_count(&self, worktree_id: i64) -> anyhow::Result<i64>;

    /// Get the count of files in a worktree.
    async fn get_worktree_file_count(&self, worktree_id: i64) -> anyhow::Result<i64>;

    /// Get the count of chunks with embeddings in a worktree.
    async fn get_worktree_embedding_count(&self, worktree_id: i64) -> anyhow::Result<i64>;

    /// Get the language breakdown for files in a worktree.
    async fn get_worktree_language_breakdown(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<Vec<(String, i64)>>;

    /// Get the last scan timestamp for a worktree.
    async fn get_worktree_last_scan(&self, worktree_id: i64) -> anyhow::Result<Option<String>>;

    /// Get the global count of distinct chunks (by blob_sha).
    async fn get_global_chunk_count(&self) -> anyhow::Result<i64>;

    /// Get the global count of embeddings.
    async fn get_global_embedding_count(&self) -> anyhow::Result<i64>;

    /// Get the count of distinct chunks for a specific repo by name.
    async fn get_repo_chunk_count(&self, repo_name: &str) -> anyhow::Result<i64>;

    /// Get the count of embeddings for a specific repo by name.
    async fn get_repo_embedding_count(&self, repo_name: &str) -> anyhow::Result<i64>;
}

// =============================================================================
// StoreChunks - Chunk CRUD and worktree associations
// =============================================================================

#[async_trait]
pub trait StoreChunks: Send + Sync {
    /// Insert a single chunk. Returns chunk ID.
    async fn insert_chunk(&self, chunk: &ChunkRecord) -> anyhow::Result<i64>;

    /// Insert multiple chunks in a batch. Returns chunk IDs.
    async fn insert_chunks_batch(&self, chunks: &[ChunkRecord]) -> anyhow::Result<Vec<i64>>;

    /// Insert a directed edge between two chunks.
    async fn insert_chunk_edge(
        &self,
        src_chunk_id: i64,
        dst_chunk_id: i64,
        edge_type: &str,
    ) -> anyhow::Result<()>;

    /// Get full chunk data by ID.
    async fn get_chunk_by_id(&self, chunk_id: i64) -> anyhow::Result<Option<ChunkFull>>;

    /// Get all chunks for a file, ordered by start line.
    async fn get_file_chunks(&self, file_id: i64) -> anyhow::Result<Vec<ChunkSummary>>;

    /// Get a chunk with surrounding context.
    async fn get_chunk_context(
        &self,
        chunk_id: i64,
        surrounding: usize,
    ) -> anyhow::Result<Option<ChunkContext>>;

    /// Find a chunk by symbol name, returning the chunk ID.
    async fn find_chunk_by_symbol(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        symbol_name: &str,
        relpath: Option<&str>,
    ) -> anyhow::Result<Option<i64>>;

    /// Delete all chunks associated with a file. Returns count of chunks deleted.
    async fn delete_chunks_by_file(&self, file_id: i64) -> anyhow::Result<u64>;

    /// Delete chunks by their IDs. Returns count of chunks deleted.
    async fn delete_chunks_by_ids(
        &self,
        worktree_id: i64,
        chunk_ids: &[i64],
    ) -> anyhow::Result<usize>;

    /// Get all chunks for a worktree with their file paths.
    /// Returns (chunk_id, file_relpath) tuples.
    async fn get_chunks_for_worktree(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<Vec<(i64, String)>>;

    /// Get chunks by blob SHA.
    async fn get_chunks_by_blob_sha(&self, blob_sha: &str) -> anyhow::Result<Vec<ChunkSummary>>;

    /// Add a chunk to an additional worktree.
    async fn add_chunk_to_worktree(
        &self,
        chunk_id: i64,
        worktree_id: i64,
    ) -> anyhow::Result<()>;

    /// Get all worktree IDs containing a chunk.
    async fn get_chunk_worktrees(&self, chunk_id: i64) -> anyhow::Result<Vec<i64>>;
}

// =============================================================================
// StoreSearch - FTS, vector, and hybrid search
// =============================================================================

#[async_trait]
pub trait StoreSearch: Send + Sync {
    /// Full-text search for chunks, resolving repo/worktree by name.
    async fn search_chunks_fts(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// FTS search by repo_id and worktree_id (for search executors).
    async fn search_fts_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query: &str,
        normalized_query: &str,
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Vector search for chunks, resolving repo/worktree by name.
    async fn search_chunks_vector(
        &self,
        repo: &str,
        worktree: Option<&str>,
        embedding: &[f32],
        k: i64,
        debug: bool,
        kind_filter: Option<&[String]>,
        lang_filter: Option<&[String]>,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Vector search by repo_id and worktree_id (for search executors).
    async fn search_vector_by_id(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        query_embedding: &[f32],
        k: i64,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Hybrid search combining FTS and vector, resolving repo/worktree by name.
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
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Hybrid search using RRF to combine FTS and vector results.
    async fn search_hybrid(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
    ) -> anyhow::Result<Vec<HybridResult>>;

    /// Hybrid search with semantic ranking applied.
    async fn search_hybrid_ranked(
        &self,
        repo: &str,
        worktree: Option<&str>,
        query: &str,
        query_embedding: &[f32],
        limit: usize,
        weights: HybridWeights,
        ranking: SemanticRanking,
    ) -> anyhow::Result<Vec<RankedSearchHit>>;

    /// Get metadata for multiple chunks (batch query for semantic ranking).
    async fn get_chunks_metadata(
        &self,
        chunk_ids: &[i64],
    ) -> anyhow::Result<HashMap<i64, ChunkMetadata>>;
}

// =============================================================================
// StoreGraph - Graph traversal and importance scoring
// =============================================================================

#[async_trait]
pub trait StoreGraph: Send + Sync {
    /// Find all chunks that call the target chunk (directly or transitively).
    async fn find_callers(
        &self,
        target_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>>;

    /// Find all chunks called by the source chunk (directly or transitively).
    async fn find_callees(
        &self,
        source_chunk_id: i64,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>>;

    /// Find import relationships for a chunk.
    async fn find_imports(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>>;

    /// Find extension/inheritance relationships for a chunk.
    async fn find_extensions(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<GraphResult>>;

    /// Get all direct edges from or to a chunk (without recursion).
    async fn get_direct_edges(
        &self,
        chunk_id: i64,
        direction: ImportDirection,
    ) -> anyhow::Result<Vec<GraphResult>>;

    /// Calculate graph importance scores for chunks in a repo/worktree.
    async fn calculate_graph_importance(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        limit: usize,
        enable_quality: bool,
        weights: &EdgeQualityWeights,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Calculate graph importance for specific chunk IDs.
    async fn calculate_graph_importance_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Calculate signal scores (recency + churn) for chunks in a repo/worktree.
    async fn calculate_signal_scores(
        &self,
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
        limit: usize,
    ) -> anyhow::Result<Vec<SearchHit>>;

    /// Calculate signal scores for specific chunk IDs.
    async fn calculate_signal_scores_for_chunks(
        &self,
        chunk_ids: &[i64],
        repo_id: i64,
        worktree_id: Option<i64>,
        recency_weight: f32,
        churn_weight: f32,
    ) -> anyhow::Result<Vec<SearchHit>>;
}

// =============================================================================
// StoreEmbeddings - Embedding storage and retrieval
// =============================================================================

#[async_trait]
pub trait StoreEmbeddings: Send + Sync {
    /// Store or update an embedding by content hash. Returns embedding ID.
    async fn upsert_embedding(
        &self,
        blob_sha: &str,
        embedding: &[f32],
        model_version: &str,
    ) -> anyhow::Result<i64>;

    /// Batch upsert embeddings with deduplication.
    async fn upsert_embeddings_batch_new(
        &self,
        embeddings: &[EmbeddingRecord],
    ) -> anyhow::Result<()>;

    /// Check if an embedding exists for a blob SHA.
    async fn has_embedding(&self, blob_sha: &str) -> anyhow::Result<bool>;

    /// Get an embedding vector by blob SHA.
    async fn get_embedding(&self, blob_sha: &str) -> anyhow::Result<Option<Vec<f32>>>;

    /// Sync a single embedding to the vector search table.
    async fn sync_embedding_to_vec(
        &self,
        embedding_id: i64,
        embedding: &[f32],
    ) -> anyhow::Result<()>;

    /// Sync all un-synced embeddings to the vector search table.
    /// Returns the number of embeddings synced.
    async fn sync_all_embeddings_to_vec(&self) -> anyhow::Result<usize>;

    /// Count chunks that need embeddings generated.
    async fn get_chunks_needing_embeddings_count(&self) -> anyhow::Result<i64>;

    /// Copy existing embeddings from cache (no-op for SQLite).
    async fn copy_existing_embeddings_from_cache(&self) -> anyhow::Result<i64>;

    /// Fetch chunks that need embeddings generated.
    async fn fetch_chunks_needing_embeddings(
        &self,
        incremental: bool,
        sample_size: Option<usize>,
    ) -> anyhow::Result<Vec<ChunkForEmbedding>>;
}

// =============================================================================
// StoreMigration - Schema migration management
// =============================================================================

#[async_trait]
pub trait StoreMigration: Send + Sync {
    /// Run all pending database migrations.
    async fn migrate(&self) -> anyhow::Result<()>;

    /// Get the set of already-applied migration versions.
    async fn get_applied_migrations(&self) -> anyhow::Result<HashSet<i32>>;
}

// =============================================================================
// StoreCleanup - Stale worktree detection and cleanup
// =============================================================================

#[async_trait]
pub trait StoreCleanup: Send + Sync {
    /// Detect worktrees whose paths no longer exist on disk.
    async fn detect_stale_worktrees(&self) -> anyhow::Result<Vec<StaleWorktree>>;

    /// Delete all data associated with a worktree.
    async fn delete_worktree_data(
        &self,
        worktree_id: i64,
    ) -> anyhow::Result<WorktreeCleanupResult>;
}

// =============================================================================
// StoreIndexState - Index state tracking
// =============================================================================

#[async_trait]
pub trait StoreIndexState: Send + Sync {
    /// Get the last indexed tree SHA for a worktree.
    /// Returns "init" if never indexed.
    async fn get_last_indexed_tree(&self, worktree_id: i64) -> anyhow::Result<String>;

    /// Update the index state for a worktree.
    async fn update_index_state(
        &self,
        worktree_id: i64,
        tree_sha: &str,
        stats: &UpdateStats,
    ) -> anyhow::Result<()>;
}

// =============================================================================
// StoreEncoding - Encoding run lifecycle
// =============================================================================

#[async_trait]
pub trait StoreEncoding: Send + Sync {
    /// Create a new encoding run record. Returns the run ID.
    async fn create_encoding_run(
        &self,
        total_chunks: i64,
        provider: Option<&str>,
        dimension: Option<i32>,
    ) -> anyhow::Result<i64>;

    /// Update the progress of an encoding run.
    async fn update_encoding_run_progress(
        &self,
        run_id: i64,
        chunks_completed: i64,
        chunks_per_second: Option<f64>,
    ) -> anyhow::Result<()>;

    /// Complete an encoding run, setting its final status.
    async fn complete_encoding_run(&self, run_id: i64, status: &str) -> anyhow::Result<()>;

    /// Mark all running encoding runs as failed (cleanup on startup).
    async fn mark_stale_runs_as_failed(&self) -> anyhow::Result<()>;

    /// Get the currently active (running) encoding run, if any.
    async fn get_active_encoding_run(&self) -> anyhow::Result<Option<EncodingRunRow>>;
}

// =============================================================================
// Store - Supertrait combining all 9 sub-traits
// =============================================================================

/// The unified Store supertrait combining all database operation categories.
///
/// Any type implementing all 9 sub-traits automatically implements `Store`
/// via the blanket impl below.
pub trait Store:
    StoreCore
    + StoreChunks
    + StoreSearch
    + StoreGraph
    + StoreEmbeddings
    + StoreMigration
    + StoreCleanup
    + StoreIndexState
    + StoreEncoding
{
}

/// Blanket implementation: any type implementing all 9 sub-traits is a Store.
impl<T> Store for T where
    T: StoreCore
        + StoreChunks
        + StoreSearch
        + StoreGraph
        + StoreEmbeddings
        + StoreMigration
        + StoreCleanup
        + StoreIndexState
        + StoreEncoding
{
}

// =============================================================================
// Object safety verification tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // These functions verify that each trait is object-safe by accepting
    // a reference to a trait object. If any trait is not object-safe,
    // these functions will fail to compile.

    fn _assert_object_safe_core(_: &dyn StoreCore) {}
    fn _assert_object_safe_chunks(_: &dyn StoreChunks) {}
    fn _assert_object_safe_search(_: &dyn StoreSearch) {}
    fn _assert_object_safe_graph(_: &dyn StoreGraph) {}
    fn _assert_object_safe_embeddings(_: &dyn StoreEmbeddings) {}
    fn _assert_object_safe_migration(_: &dyn StoreMigration) {}
    fn _assert_object_safe_cleanup(_: &dyn StoreCleanup) {}
    fn _assert_object_safe_index_state(_: &dyn StoreIndexState) {}
    fn _assert_object_safe_encoding(_: &dyn StoreEncoding) {}
    fn _assert_object_safe_store(_: &dyn Store) {}
}
