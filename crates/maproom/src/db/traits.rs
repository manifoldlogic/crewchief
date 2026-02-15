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
use crate::db::index_state::UpdateStats;
use crate::db::types::{
    ChunkMetadata, EmbeddingRecord, EncodingRunRow, GraphResult, HybridResult, HybridWeights,
    ImportDirection, RankedSearchHit, SemanticRanking,
};
use crate::db::{
    ChunkContext, ChunkForEmbedding, ChunkFull, ChunkRecord, ChunkSummary, FileRecord, RepoInfo,
    SearchHit, StaleWorktree, WorktreeCleanupResult, WorktreeInfo,
};

// =============================================================================
// StoreCore - Repository, worktree, commit, file CRUD and stats
// =============================================================================

#[async_trait]
pub trait StoreCore: Send + Sync {
    /// Check if sqlite-vec (or equivalent vector extension) is available.
    ///
    /// Returns `true` if the backend supports vector similarity search. When
    /// `false`, vector search degrades gracefully (FTS-only results).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// fn choose_search_mode(store: &dyn Store) -> &'static str {
    ///     if store.has_vector_extension() {
    ///         "hybrid"
    ///     } else {
    ///         "fts"
    ///     }
    /// }
    /// ```
    fn has_vector_extension(&self) -> bool;

    /// Get or create a repository by name and root path. Returns repo ID.
    ///
    /// If a repository with the given name already exists, its ID is returned
    /// without creating a duplicate. This is the standard entry point for
    /// registering a repo before indexing.
    ///
    /// # Arguments
    ///
    /// * `name` - Short name for the repository (e.g., `"crewchief"`)
    /// * `root_path` - Absolute filesystem path to the repository root
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// async fn setup_repo(store: &dyn Store) -> anyhow::Result<i64> {
    ///     let repo_id = store
    ///         .get_or_create_repo("my-project", "/home/user/repos/my-project")
    ///         .await?;
    ///     // repo_id can now be used with get_or_create_worktree, etc.
    ///     Ok(repo_id)
    /// }
    /// ```
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
    ///
    /// Stores a parsed code chunk (function, class, etc.) extracted by
    /// tree-sitter. The chunk is associated with a file via `chunk.file_id`
    /// and a worktree via `chunk.worktree_id`.
    ///
    /// # Arguments
    ///
    /// * `chunk` - The chunk record containing symbol metadata, line range,
    ///   preview text, and content hash (`blob_sha`)
    ///
    /// # Returns
    ///
    /// The database ID of the inserted chunk.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::{ChunkRecord, Store};
    ///
    /// async fn index_chunk(store: &dyn Store, file_id: i64, worktree_id: i64) -> anyhow::Result<i64> {
    ///     let chunk = ChunkRecord {
    ///         file_id,
    ///         blob_sha: "abc123".to_string(),
    ///         symbol_name: Some("process_request".to_string()),
    ///         kind: "function".to_string(),
    ///         signature: Some("fn process_request(req: Request) -> Response".to_string()),
    ///         docstring: None,
    ///         start_line: 10,
    ///         end_line: 25,
    ///         preview: "fn process_request(req: Request) -> Response { ... }".to_string(),
    ///         ts_doc_text: "process_request function request response".to_string(),
    ///         recency_score: 1.0,
    ///         churn_score: 0.5,
    ///         metadata: None,
    ///         worktree_id,
    ///     };
    ///     store.insert_chunk(&chunk).await
    /// }
    /// ```
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
    ///
    /// Returns the full chunk data along with neighboring chunks from the
    /// same file. The `surrounding` parameter controls how many chunks
    /// before and after are included, providing spatial context for code
    /// navigation and LLM context assembly.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The database ID of the target chunk
    /// * `surrounding` - Number of neighboring chunks to include on each side
    ///
    /// # Returns
    ///
    /// `None` if the chunk ID does not exist; otherwise a [`ChunkContext`]
    /// containing the chunk and its neighbors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// async fn show_context(store: &dyn Store, chunk_id: i64) -> anyhow::Result<()> {
    ///     if let Some(ctx) = store.get_chunk_context(chunk_id, 2).await? {
    ///         println!("File: {}", ctx.file_path);
    ///         println!("Chunk: {} (lines {}-{})",
    ///             ctx.chunk.kind, ctx.chunk.start_line, ctx.chunk.end_line);
    ///         println!("Surrounding chunks: {}", ctx.surrounding_chunks.len());
    ///     }
    ///     Ok(())
    /// }
    /// ```
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
    async fn get_chunks_for_worktree(&self, worktree_id: i64)
        -> anyhow::Result<Vec<(i64, String)>>;

    /// Get chunks by blob SHA.
    async fn get_chunks_by_blob_sha(&self, blob_sha: &str) -> anyhow::Result<Vec<ChunkSummary>>;

    /// Add a chunk to an additional worktree.
    async fn add_chunk_to_worktree(&self, chunk_id: i64, worktree_id: i64) -> anyhow::Result<()>;

    /// Get all worktree IDs containing a chunk.
    async fn get_chunk_worktrees(&self, chunk_id: i64) -> anyhow::Result<Vec<i64>>;
}

// =============================================================================
// StoreSearch - FTS, vector, and hybrid search
// =============================================================================

#[async_trait]
pub trait StoreSearch: Send + Sync {
    /// Full-text search for chunks, resolving repo/worktree by name.
    ///
    /// Performs FTS5 keyword search across indexed chunks in the specified
    /// repository and optional worktree. Results are ranked by BM25 with
    /// optional semantic ranking adjustments.
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository name (e.g., `"crewchief"`)
    /// * `worktree` - Optional worktree name to scope the search
    /// * `query` - FTS query string (supports SQLite FTS5 syntax)
    /// * `k` - Maximum number of results to return
    /// * `debug` - If `true`, populate debug fields (`base_score`, `kind_mult`, etc.)
    /// * `kind_filter` - Optional filter to restrict chunk kinds (e.g., `["function", "method"]`)
    /// * `lang_filter` - Optional filter to restrict languages (e.g., `["rust", "typescript"]`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// async fn find_functions(store: &dyn Store) -> anyhow::Result<()> {
    ///     let hits = store.search_chunks_fts(
    ///         "my-project",
    ///         Some("main"),
    ///         "authentication",
    ///         10,
    ///         false,
    ///         Some(&["function".to_string(), "method".to_string()]),
    ///         None,
    ///     ).await?;
    ///
    ///     for hit in &hits {
    ///         println!("{} ({}) score={:.3}", hit.file_relpath, hit.kind, hit.score);
    ///     }
    ///     Ok(())
    /// }
    /// ```
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
    ///
    /// Executes both FTS and vector searches, then merges them using
    /// Reciprocal Rank Fusion (RRF). Each result's final score blends
    /// keyword relevance and semantic similarity according to `weights`.
    ///
    /// # Arguments
    ///
    /// * `repo` - Repository name
    /// * `worktree` - Optional worktree name to scope the search
    /// * `query` - FTS query string for the keyword component
    /// * `query_embedding` - Pre-computed embedding vector for the semantic component
    /// * `limit` - Maximum number of results to return
    /// * `weights` - Relative importance of FTS vs. vector scores
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::{HybridWeights, Store};
    ///
    /// async fn hybrid_search(
    ///     store: &dyn Store,
    ///     query: &str,
    ///     embedding: &[f32],
    /// ) -> anyhow::Result<()> {
    ///     let results = store.search_hybrid(
    ///         "my-project",
    ///         Some("main"),
    ///         query,
    ///         embedding,
    ///         20,
    ///         HybridWeights::default(), // 0.3 FTS, 0.7 vector
    ///     ).await?;
    ///
    ///     for r in &results {
    ///         println!("chunk {} score={:.3} source={}", r.chunk_id, r.score, r.source);
    ///     }
    ///     Ok(())
    /// }
    /// ```
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
    ///
    /// Performs a recursive graph traversal following incoming "calls" edges
    /// to discover callers up to `max_depth` levels. Uses cycle detection to
    /// avoid infinite loops. Default depth is 3; hard maximum is 10.
    ///
    /// # Arguments
    ///
    /// * `target_chunk_id` - The chunk whose callers to find
    /// * `max_depth` - Maximum traversal depth (`None` for default of 3)
    ///
    /// # Returns
    ///
    /// A list of [`GraphResult`] entries, each containing the caller's chunk ID,
    /// traversal depth, and the edge path from source to target.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// async fn who_calls(store: &dyn Store, chunk_id: i64) -> anyhow::Result<()> {
    ///     let callers = store.find_callers(chunk_id, Some(2)).await?;
    ///     for caller in &callers {
    ///         println!(
    ///             "chunk {} at depth {} via {:?}",
    ///             caller.chunk_id, caller.depth, caller.path
    ///         );
    ///     }
    ///     Ok(())
    /// }
    /// ```
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
    ///
    /// Returns only depth-1 relationships (no transitive traversal). Useful
    /// for building local dependency views or populating UI adjacency lists.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The chunk to query edges for
    /// * `direction` - [`ImportDirection::Outgoing`] for edges leaving the chunk,
    ///   [`ImportDirection::Incoming`] for edges arriving at the chunk
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::{ImportDirection, Store};
    ///
    /// async fn list_dependencies(store: &dyn Store, chunk_id: i64) -> anyhow::Result<()> {
    ///     let outgoing = store.get_direct_edges(chunk_id, ImportDirection::Outgoing).await?;
    ///     println!("Chunk {} has {} outgoing edges", chunk_id, outgoing.len());
    ///
    ///     let incoming = store.get_direct_edges(chunk_id, ImportDirection::Incoming).await?;
    ///     println!("Chunk {} has {} incoming edges", chunk_id, incoming.len());
    ///     Ok(())
    /// }
    /// ```
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
    ///
    /// Embeddings are deduplicated by `blob_sha` -- if the same content hash
    /// already has an embedding, it is updated in place. This avoids
    /// redundant storage when the same chunk appears in multiple worktrees.
    ///
    /// # Arguments
    ///
    /// * `blob_sha` - Content hash identifying the chunk text
    /// * `embedding` - The embedding vector (dimension must match the configured table)
    /// * `model_version` - Identifier for the model that produced the embedding
    ///   (e.g., `"mxbai-embed-large"`)
    ///
    /// # Returns
    ///
    /// The database ID of the upserted embedding row.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// async fn store_embedding(
    ///     store: &dyn Store,
    ///     blob_sha: &str,
    ///     vector: &[f32],
    /// ) -> anyhow::Result<i64> {
    ///     store.upsert_embedding(blob_sha, vector, "mxbai-embed-large").await
    /// }
    /// ```
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
    ///
    /// Returns chunks whose `blob_sha` does not yet have a corresponding
    /// embedding record. Used by the embedding pipeline to discover work.
    ///
    /// # Arguments
    ///
    /// * `incremental` - If `true`, only return chunks added since the last
    ///   encoding run; if `false`, return all chunks without embeddings
    /// * `sample_size` - Optional cap on the number of chunks returned
    ///   (useful for batched processing)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use crewchief_maproom::db::Store;
    ///
    /// async fn embedding_pipeline(store: &dyn Store) -> anyhow::Result<()> {
    ///     let chunks = store.fetch_chunks_needing_embeddings(true, Some(500)).await?;
    ///     println!("{} chunks need embeddings", chunks.len());
    ///
    ///     for chunk in &chunks {
    ///         // Generate embedding for chunk.preview + chunk.signature ...
    ///         println!("chunk {} blob_sha={}", chunk.id, chunk.blob_sha);
    ///     }
    ///     Ok(())
    /// }
    /// ```
    async fn fetch_chunks_needing_embeddings(
        &self,
        incremental: bool,
        sample_size: Option<usize>,
    ) -> anyhow::Result<Vec<ChunkForEmbedding>>;
}

// =============================================================================
// StoreMigration - Schema migration management
// =============================================================================

/// Migration management for database schema versioning.
///
/// # Version-Based Migration Tracking
///
/// This trait uses integer-based migration versions (`i32`) for tracking
/// applied migrations. This design aligns with SQLite's simple migration
/// system where migrations are ordered by version number.
///
/// # PostgreSQL Compatibility Note
///
/// PostgreSQL's `sqlx` uses string-based migration identifiers (e.g.,
/// `"20240101_initial_schema.sql"`). Future `PostgresStore` implementations
/// may need an adapter layer to convert between string-based sqlx migration
/// names and integer-based version numbers used by this trait.
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
    async fn delete_worktree_data(&self, worktree_id: i64)
        -> anyhow::Result<WorktreeCleanupResult>;
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
