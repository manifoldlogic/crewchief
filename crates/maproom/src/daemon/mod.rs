pub mod protocol;
pub mod server;
pub mod session;
pub mod types;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

use crate::context::{AssemblyStrategy, DefaultAssemblyStrategy, ExpandOptions};
use crate::db::{connect, SearchHit, Store};
use crate::embedding::EmbeddingService;
use crate::search::confidence::compute_result_confidence;
use crate::search::errors::SearchErrorDetails;
use crate::search::executor_types::SearchSource;
use crate::search::fusion::FusedResult;

use self::types::{
    ContextParams, JsonRpcRequest, JsonRpcResponse, RepoStatus, SearchParams, StatusParams,
    StatusResult, WorktreeStatus,
};

/// Create SearchErrorDetails from anyhow::Error by analyzing error message.
///
/// This function pattern-matches error messages to infer the appropriate error type
/// when we can't extract a concrete PipelineError from the error chain.
fn error_details_from_anyhow(error: &anyhow::Error) -> SearchErrorDetails {
    use crate::search::errors::{ErrorType, PipelineStage};
    use std::collections::HashMap;

    let error_str = error.to_string();

    // Check for embedding-related errors
    if error_str.contains("embed") || error_str.contains("Embed") {
        if error_str.contains("timeout") || error_str.contains("Timeout") {
            return SearchErrorDetails {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([(
                    "error".to_string(),
                    "Embedding request timeout".to_string(),
                )]),
                suggestions: vec![
                    "Check your embedding provider connectivity".to_string(),
                    "Try FTS mode while debugging: --mode fts".to_string(),
                ],
            };
        } else if error_str.contains("API")
            || error_str.contains("api")
            || error_str.contains("credential")
        {
            return SearchErrorDetails {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([(
                    "error".to_string(),
                    "Embedding provider authentication failed".to_string(),
                )]),
                suggestions: vec![
                    "Check your API credentials (OPENAI_API_KEY, GOOGLE_API_KEY, etc.)".to_string(),
                    "Verify your API key is valid and has not expired".to_string(),
                ],
            };
        } else {
            return SearchErrorDetails {
                error_type: ErrorType::EmbeddingProvider,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([("error".to_string(), error_str.clone())]),
                suggestions: vec![
                    "Check your embedding provider configuration".to_string(),
                    "Try FTS mode while debugging: --mode fts".to_string(),
                ],
            };
        }
    }

    // Check for database-related errors
    if error_str.contains("not indexed")
        || error_str.contains("not found")
        || error_str.contains("No such")
    {
        return SearchErrorDetails {
            error_type: ErrorType::NotFound,
            stage: PipelineStage::SearchExecution,
            context: HashMap::from([("error".to_string(), error_str.clone())]),
            suggestions: vec![
                "Check that the repository is indexed: maproom status".to_string(),
                "Run a scan to index the repository: maproom scan".to_string(),
            ],
        };
    }

    if error_str.contains("database") || error_str.contains("Database") || error_str.contains("SQL")
    {
        if error_str.contains("timeout") || error_str.contains("Timeout") {
            return SearchErrorDetails {
                error_type: ErrorType::Database,
                stage: PipelineStage::SearchExecution,
                context: HashMap::from([("error".to_string(), error_str.clone())]),
                suggestions: vec![
                    "Check database connectivity".to_string(),
                    "Restart the maproom daemon: maproom serve".to_string(),
                ],
            };
        } else {
            return SearchErrorDetails {
                error_type: ErrorType::Database,
                stage: PipelineStage::SearchExecution,
                context: HashMap::from([("error".to_string(), error_str.clone())]),
                suggestions: vec![
                    "Check database connectivity and permissions".to_string(),
                    "Verify repository is indexed: maproom status".to_string(),
                ],
            };
        }
    }

    // Check for timeout errors
    if error_str.contains("timeout") || error_str.contains("Timeout") {
        return SearchErrorDetails {
            error_type: ErrorType::Timeout,
            stage: PipelineStage::SearchExecution,
            context: HashMap::from([("error".to_string(), error_str.clone())]),
            suggestions: vec![
                "Try narrowing your search scope with more specific terms".to_string(),
                "Use a simpler query or reduce the result limit".to_string(),
            ],
        };
    }

    // Check for search execution errors
    if error_str.contains("search") || error_str.contains("Search") {
        return SearchErrorDetails {
            error_type: ErrorType::Database,
            stage: PipelineStage::SearchExecution,
            context: HashMap::from([("error".to_string(), error_str.clone())]),
            suggestions: vec![
                "Check that the repository is indexed".to_string(),
                "Try a different search mode (fts, vector, or hybrid)".to_string(),
            ],
        };
    }

    // Default: unknown error
    SearchErrorDetails {
        error_type: ErrorType::Unknown,
        stage: PipelineStage::SearchExecution,
        context: HashMap::from([("error".to_string(), error_str)]),
        suggestions: vec!["Please report this error with full details".to_string()],
    }
}

/// Deduplicate search hits by identity (file_relpath, symbol_name, start_line).
fn deduplicate_search_hits(hits: Vec<SearchHit>, limit: usize) -> Vec<SearchHit> {
    if hits.is_empty() {
        return hits;
    }

    let mut groups: HashMap<(String, Option<String>, i32), Vec<SearchHit>> = HashMap::new();
    for hit in hits {
        let key = (
            hit.file_relpath.clone(),
            hit.symbol_name.clone(),
            hit.start_line,
        );
        groups.entry(key).or_default().push(hit);
    }

    let mut deduped: Vec<SearchHit> = groups
        .into_values()
        .map(|mut group| {
            group.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            group.remove(0)
        })
        .collect();

    deduped.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    deduped.into_iter().take(limit).collect()
}

struct DaemonState {
    store: Arc<dyn Store + Send + Sync>,
    /// Lazily-initialized embedding service (R16 / fix spec R-LAZY-1, OD-5).
    /// `serve` used to hard-require `EmbeddingService::from_env()` at startup,
    /// making the daemon DOA in provider-less environments even though FTS /
    /// context / status / ping need no embeddings. `get_or_try_init` does not
    /// cache failures, so a provider that comes up later is picked up.
    embedding: tokio::sync::OnceCell<EmbeddingService>,
    /// Negative cache for failed lazy init (R-LAZY-8): hybrid is the daemon's
    /// DEFAULT mode, and in a provider-less environment every default-mode
    /// search would otherwise pay the full provider auto-detection probe cost
    /// before falling back to FTS.
    embed_failed_at: std::sync::Mutex<Option<std::time::Instant>>,
    context_assembler: DefaultAssemblyStrategy,
}

impl DaemonState {
    fn new(store: Arc<dyn Store + Send + Sync>) -> Self {
        Self {
            store: store.clone(),
            embedding: tokio::sync::OnceCell::new(),
            embed_failed_at: std::sync::Mutex::new(None),
            context_assembler: DefaultAssemblyStrategy::new(store),
        }
    }

    /// Lazy accessor for the embedding service (R-LAZY-1). The
    /// `.context("Failed to initialize embedding service")` is load-bearing
    /// twice over: it coerces `EmbeddingError` into the accessor's
    /// `anyhow::Result`, and its "embedding" substring guarantees
    /// `error_details_from_anyhow` classifies failures as EmbeddingProvider.
    async fn embedding_service(&self) -> Result<&EmbeddingService> {
        if let Some(svc) = self.embedding.get() {
            return Ok(svc);
        }
        {
            let guard = self.embed_failed_at.lock().unwrap();
            if let Some(at) = *guard {
                if at.elapsed() < std::time::Duration::from_secs(30) {
                    anyhow::bail!(
                        "Failed to initialize embedding service (retry suppressed for 30s after last failure)"
                    );
                }
            }
        }
        match self
            .embedding
            .get_or_try_init(|| async {
                EmbeddingService::from_env()
                    .await
                    .context("Failed to initialize embedding service")
            })
            .await
        {
            Ok(svc) => Ok(svc),
            Err(e) => {
                *self.embed_failed_at.lock().unwrap() = Some(std::time::Instant::now());
                Err(e)
            }
        }
    }
}

pub async fn run() -> Result<()> {
    info!("Daemon mode starting...");

    // Initialize the configured backend (SQLite or Postgres per the DSN).
    let store = connect()
        .await
        .context("Failed to initialize database store")?;

    // R16 / R-LAZY-2: the embedding service initializes lazily on the first
    // vector/hybrid request — serve MUST NOT fail (or block) on provider
    // configuration at startup; FTS/context/status/ping need no embeddings.
    // (The MAY-level eager warn-only init was skipped deliberately: Google/
    // Vertex credential resolution can block for seconds at startup; the
    // hybrid arm degrades to FTS and surfaces the reason per-request.)
    let state = Arc::new(DaemonState::new(store));

    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_request(request, state.clone()).await,
            Err(e) => {
                error!("Failed to parse request: {}", e);
                JsonRpcResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    "Parse error".to_string(),
                    Some(serde_json::json!(e.to_string())),
                )
            }
        };

        let mut response_json = serde_json::to_string(&response)?;
        response_json.push('\n');
        stdout.write_all(response_json.as_bytes()).await?;
        stdout.flush().await?;
    }

    info!("Daemon mode exiting...");
    Ok(())
}

async fn handle_request(request: JsonRpcRequest, state: Arc<DaemonState>) -> JsonRpcResponse {
    let id = request.id.clone().unwrap_or(serde_json::Value::Null);

    match request.method.as_str() {
        "ping" => JsonRpcResponse::success(id, serde_json::Value::String("pong".to_string())),
        "search" => {
            let params: SearchParams = match serde_json::from_value(
                request.params.clone().unwrap_or(serde_json::Value::Null),
            ) {
                Ok(p) => p,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        "Invalid params".to_string(),
                        Some(serde_json::json!(e.to_string())),
                    )
                }
            };

            // R18 / R-WTF-1: validate the worktree name BEFORE execute_search
            // (all execute_search errors collapse to -32000). An unknown
            // worktree used to be silently unscoped -> wrong-scope results.
            if let Some(ref w) = params.worktree {
                let known = match worktree_exists(&state, &params.repo, w).await {
                    Ok(known) => known,
                    Err(e) => {
                        error!("Worktree validation failed: {}", e);
                        false
                    }
                };
                if !known {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        "Invalid params".to_string(),
                        Some(serde_json::json!(format!(
                            "unknown worktree '{}' for repo '{}'",
                            w, params.repo
                        ))),
                    );
                }
            }

            match execute_search(state, params).await {
                Ok(results) => JsonRpcResponse::success(id, results),
                Err(e) => {
                    error!("Search failed: {}", e);

                    // Try to extract PipelineError from anyhow error chain
                    let error_details = if let Some(pipeline_err) =
                        e.downcast_ref::<crate::search::pipeline::PipelineError>()
                    {
                        // Direct PipelineError found in error chain
                        SearchErrorDetails::from_pipeline_error(pipeline_err)
                    } else {
                        // Fall back to error message analysis for other error types
                        // This handles database errors, embedding errors, etc. wrapped in anyhow
                        error_details_from_anyhow(&e)
                    };

                    // Serialize error details, with fallback to simple string on error
                    let error_data = match serde_json::to_value(&error_details) {
                        Ok(value) => Some(value),
                        Err(ser_err) => {
                            tracing::warn!("Failed to serialize error details: {}", ser_err);
                            Some(serde_json::json!(e.to_string()))
                        }
                    };

                    JsonRpcResponse::error(
                        id,
                        -32000,
                        e.to_string(), // Preserve human-readable message
                        error_data,
                    )
                }
            }
        }
        "context" => {
            let params: ContextParams = match serde_json::from_value(
                request.params.clone().unwrap_or(serde_json::Value::Null),
            ) {
                Ok(p) => p,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        "Invalid params".to_string(),
                        Some(serde_json::json!(e.to_string())),
                    )
                }
            };

            match execute_context(state, params).await {
                Ok(bundle) => JsonRpcResponse::success(id, bundle),
                Err(e) => {
                    error!("Context assembly failed: {}", e);
                    // Use -32000 for "chunk not found" or general errors
                    JsonRpcResponse::error(
                        id,
                        -32000,
                        "Context assembly failed".to_string(),
                        Some(serde_json::json!(e.to_string())),
                    )
                }
            }
        }
        "status" => {
            let params: StatusParams =
                serde_json::from_value(request.params.clone().unwrap_or(serde_json::Value::Null))
                    .unwrap_or_default();

            match execute_status(state, params).await {
                Ok(result) => JsonRpcResponse::success(id, serde_json::to_value(result).unwrap()),
                Err(e) => {
                    error!("Status query failed: {}", e);
                    JsonRpcResponse::error(
                        id,
                        -32000,
                        "Status query failed".to_string(),
                        Some(serde_json::json!(e.to_string())),
                    )
                }
            }
        }
        _ => JsonRpcResponse::error(
            id,
            -32601,
            "Method not found".to_string(),
            Some(serde_json::json!(request.method)),
        ),
    }
}

/// R18 / R-WTF-1: does `worktree` exist for `repo`? Repo resolution uses the
/// same exact-or-suffix fuzzy match as execute_status. Unknown REPO returns
/// true (out of scope here — existing repo-error behavior is preserved).
async fn worktree_exists(state: &Arc<DaemonState>, repo: &str, worktree: &str) -> Result<bool> {
    let all_repos = state
        .store
        .list_repos()
        .await
        .context("Failed to list repos")?;
    let matched: Vec<_> = all_repos
        .into_iter()
        .filter(|r| r.name == repo || r.name.ends_with(&format!("/{}", repo)))
        .collect();
    if matched.is_empty() {
        return Ok(true); // unknown repo keeps existing error behavior downstream
    }
    for r in matched {
        let worktrees = state
            .store
            .list_worktrees(r.id)
            .await
            .context("Failed to list worktrees")?;
        if worktrees.iter().any(|w| w.name == worktree) {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn execute_search(
    state: Arc<DaemonState>,
    params: SearchParams,
) -> Result<serde_json::Value> {
    // Determine search mode (default to "hybrid" for backward compatibility)
    let mode = params.mode.as_deref().unwrap_or("hybrid");

    // Validate mode
    if !matches!(mode, "fts" | "vector" | "hybrid") {
        anyhow::bail!(
            "Invalid search mode: '{}'. Valid modes: fts, vector, hybrid",
            mode
        );
    }

    let k = params.limit.unwrap_or(10) as i64;
    let deduplicate = params.deduplicate.unwrap_or(true);

    // Fetch extra results when deduplication is enabled
    let fetch_k = if deduplicate { k * 3 } else { k };

    // Use VectorStore trait methods for all search operations
    // The trait methods handle repo/worktree resolution internally
    let raw_hits: Vec<SearchHit> = match mode {
        "fts" => {
            // FTS mode: Full-text search only (no embeddings required)
            let (hits, _total_count) = state
                .store
                .search_chunks_fts(
                    &params.repo,
                    params.worktree.as_deref(),
                    &params.query,
                    fetch_k,
                    false, // debug
                    params.kind.as_deref(),
                    params.lang.as_deref(),
                )
                .await
                .context("FTS search execution failed")?;
            hits
        }
        "vector" => {
            // Vector mode: Semantic search using embeddings. R-LAZY-3: the
            // lazy accessor's failure flows into the existing structured error
            // path (classified EmbeddingProvider via its context string).
            let query_embedding = state
                .embedding_service()
                .await?
                .embed_text(&params.query)
                .await
                .context("Failed to generate query embedding")?;

            state
                .store
                .search_chunks_vector(
                    &params.repo,
                    params.worktree.as_deref(),
                    &query_embedding,
                    fetch_k,
                    false, // debug
                    params.kind.as_deref(),
                    params.lang.as_deref(),
                )
                .await
                .context("Vector search execution failed")?
        }
        "hybrid" => {
            // Hybrid mode: Try to get embedding for hybrid search.
            // R-LAZY-4: a lazy-init failure folds into the same FTS fallback
            // as an embed_text failure — the daemon's default mode degrades
            // gracefully in provider-less environments.
            let query_embedding_result = match state.embedding_service().await {
                Ok(svc) => svc.embed_text(&params.query).await.map_err(anyhow::Error::from),
                Err(e) => Err(e),
            };

            match query_embedding_result {
                Ok(query_embedding) => {
                    // Embeddings available, use hybrid search
                    state
                        .store
                        .search_chunks_hybrid(
                            &params.repo,
                            params.worktree.as_deref(),
                            &params.query,
                            &query_embedding,
                            fetch_k,
                            false, // debug
                            params.kind.as_deref(),
                            params.lang.as_deref(),
                        )
                        .await
                        .unwrap_or_else(|_| {
                            // Hybrid failed, will fall back to FTS below
                            Vec::new()
                        })
                }
                Err(_) => {
                    // No embeddings available, use FTS directly
                    let (hits, _total_count) = state
                        .store
                        .search_chunks_fts(
                            &params.repo,
                            params.worktree.as_deref(),
                            &params.query,
                            fetch_k,
                            false, // debug
                            params.kind.as_deref(),
                            params.lang.as_deref(),
                        )
                        .await
                        .context("FTS search execution failed")?;
                    hits
                }
            }
        }
        _ => unreachable!("Mode validation should prevent this"),
    };

    // Apply deduplication if enabled
    let hits = if deduplicate {
        deduplicate_search_hits(raw_hits, k as usize)
    } else {
        raw_hits
    };

    let include_confidence = params.include_confidence.unwrap_or(false);

    // Apply threshold filter first so we have the filtered list for confidence computation
    let filtered_hits: Vec<&SearchHit> = hits
        .iter()
        .filter(|hit| {
            // Apply threshold filter if specified
            if let Some(thresh) = params.threshold {
                hit.score >= thresh as f64
            } else {
                true
            }
        })
        .collect();

    // Build all_fused once for score_gap calculation (only when confidence is requested)
    // Note: In daemon mode, source_count will be 1 (fts/vector) or 2 (hybrid),
    // not 1-4 like the full pipeline. This is acceptable because score_gap and
    // is_exact_match are the most actionable signals.
    let all_fused: Vec<FusedResult> = if include_confidence {
        filtered_hits
            .iter()
            .map(|h| FusedResult::new(h.chunk_id, h.score as f32, HashMap::new()))
            .collect()
    } else {
        Vec::new()
    };

    // Format response - SearchHit already contains all needed fields
    let formatted_hits: Vec<serde_json::Value> = filtered_hits
        .iter()
        .enumerate()
        .map(|(index, hit)| {
            let mut json = serde_json::json!({
                "chunk_id": hit.chunk_id,
                "score": hit.score,
                "start_line": hit.start_line,
                "end_line": hit.end_line,
                "symbol_name": hit.symbol_name,
                "kind": hit.kind,
                "file_relpath": hit.file_relpath,
                // DEPRECATED(AFM-02): Use file_relpath. Retained for backward compatibility.
                "file_path": hit.file_relpath,
            });

            if include_confidence {
                // Convert SearchHit to FusedResult using adapter function
                let fused_result = searchhit_to_fused_result(hit, mode);

                // Call existing confidence function - zero new computation logic
                let confidence = compute_result_confidence(
                    &fused_result,
                    &all_fused,
                    index,
                    fused_result.exact_match_multiplier,
                );

                json["confidence"] =
                    serde_json::to_value(&confidence).unwrap_or(serde_json::Value::Null);
            }

            json
        })
        .collect();

    Ok(serde_json::json!({
        "hits": formatted_hits,
        "total": formatted_hits.len(),
        "query": params.query,
        "mode": mode,
        "k": k,
        "threshold": params.threshold,
        "deduplicate": deduplicate,
    }))
}

/// Execute a context assembly request.
///
/// Converts ContextParams to ExpandOptions and assembles a context bundle
/// using the DefaultAssemblyStrategy stored in DaemonState.
async fn execute_context(
    state: Arc<DaemonState>,
    params: ContextParams,
) -> Result<serde_json::Value> {
    // Parse chunk_id from string to i64
    let chunk_id = params
        .chunk_id
        .parse::<i64>()
        .context("Invalid chunk_id: must be a valid integer")?;

    // Convert ExpandConfig to ExpandOptions
    let options = ExpandOptions {
        callers: params.expand.callers,
        callees: params.expand.callees,
        tests: params.expand.tests,
        docs: params.expand.docs,
        config: params.expand.config,
        max_depth: params.expand.max_depth,
        routes: params.expand.routes,
        hooks: params.expand.hooks,
        jsx_parents: params.expand.jsx_parents,
        jsx_children: params.expand.jsx_children,
    };

    // Use the state's context assembler (enables caching across requests)
    let bundle = state
        .context_assembler
        .assemble(chunk_id, params.budget_tokens, options)
        .await
        .context("Failed to assemble context bundle")?;

    // Serialize the bundle to JSON
    serde_json::to_value(bundle).context("Failed to serialize context bundle")
}

/// Execute a status request.
///
/// Queries the database for repository and worktree statistics.
async fn execute_status(state: Arc<DaemonState>, params: StatusParams) -> Result<StatusResult> {
    // Get all repos
    let all_repos = state
        .store
        .list_repos()
        .await
        .context("Failed to list repos")?;

    // Filter by repo name if specified
    let repos_to_query: Vec<_> = if let Some(ref repo_filter) = params.repo {
        all_repos
            .into_iter()
            .filter(|r| r.name == *repo_filter || r.name.ends_with(&format!("/{}", repo_filter)))
            .collect()
    } else {
        all_repos
    };

    let mut repo_statuses = Vec::new();
    let mut total_files: i64 = 0;
    let mut total_chunks: i64 = 0;

    for repo in &repos_to_query {
        // Get worktrees for this repo
        let worktrees = state
            .store
            .list_worktrees(repo.id)
            .await
            .context("Failed to list worktrees")?;

        let mut worktree_statuses = Vec::new();

        for wt in worktrees {
            // Get chunk count for this worktree
            let chunk_count = state
                .store
                .get_worktree_chunk_count(wt.id)
                .await
                .unwrap_or(0);

            // Get file count (we need to add this method or use a raw query)
            let file_count = state
                .store
                .get_worktree_file_count(wt.id)
                .await
                .unwrap_or(0);

            total_files += file_count;
            total_chunks += chunk_count;

            worktree_statuses.push(WorktreeStatus {
                name: wt.name,
                path: wt.abs_path,
                file_count,
                chunk_count,
            });
        }

        repo_statuses.push(RepoStatus {
            name: repo.name.clone(),
            worktrees: worktree_statuses,
        });
    }

    Ok(StatusResult {
        total_repos: repo_statuses.len(),
        repos: repo_statuses,
        total_files,
        total_chunks,
    })
}

/// Convert a SearchHit to a FusedResult for confidence computation.
///
/// This is the adapter pattern that bridges daemon SearchHit results
/// to the existing confidence computation infrastructure in search/confidence.rs.
///
/// # Parameters
/// - `hit`: The daemon SearchHit to convert
/// - `mode`: Search mode string ("fts", "vector", "hybrid") to determine source_scores
///
/// # Returns
/// A FusedResult suitable for passing to `compute_result_confidence()`
fn searchhit_to_fused_result(hit: &SearchHit, mode: &str) -> FusedResult {
    let mut source_scores = HashMap::new();
    match mode {
        "fts" => {
            source_scores.insert(SearchSource::FTS, hit.score as f32);
        }
        "vector" => {
            source_scores.insert(SearchSource::Vector, hit.score as f32);
        }
        "hybrid" => {
            source_scores.insert(SearchSource::FTS, hit.score as f32);
            source_scores.insert(SearchSource::Vector, hit.score as f32);
        }
        _ => {}
    }

    FusedResult::with_exact_match(
        hit.chunk_id,
        hit.score as f32,
        source_scores,
        hit.exact_mult.map(|m| m as f32),
    )
}

#[cfg(test)]
mod r16_lazy_embedding_tests {
    use super::*;

    async fn memory_state() -> Arc<DaemonState> {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let db = format!("file:memdb_daemon_r16_{n}?mode=memory&cache=shared");
        let store = crate::db::SqliteStore::connect(&db).await.unwrap();
        use crate::db::traits::StoreMigration;
        store.migrate().await.unwrap();
        Arc::new(DaemonState::new(Arc::new(store)))
    }

    /// R16 / R-LAZY-1: constructing daemon state must not touch the
    /// embedding environment at all.
    #[tokio::test]
    #[serial_test::serial]
    async fn daemon_state_constructs_without_embedding_env() {
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", "google");
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "/nonexistent");
        let state = memory_state().await;
        // ping-equivalent: any non-embedding request works.
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "ping".to_string(),
            params: None,
            id: Some(serde_json::json!(1)),
        };
        let resp = handle_request(req, state).await;
        assert_eq!(resp.result, Some(serde_json::json!("pong")));
        std::env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        std::env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
    }

    /// R16 / R-LAZY-3: a vector-mode search without a provider yields a
    /// structured JSON-RPC error (not a dead daemon, not a raw chain).
    #[tokio::test]
    #[serial_test::serial]
    async fn vector_search_returns_structured_error_without_provider() {
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", "invalid-provider");
        let state = memory_state().await;
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "search".to_string(),
            params: Some(serde_json::json!({
                "query": "anything",
                "repo": "nope",
                "mode": "vector"
            })),
            id: Some(serde_json::json!(2)),
        };
        let resp = handle_request(req, state).await;
        std::env::remove_var("MAPROOM_EMBEDDING_PROVIDER");
        let err = resp.error.expect("vector search without provider must be a JSON-RPC error");
        assert_eq!(err.code, -32000);
        // Classified as an embedding-provider problem, with the process alive.
        let data = serde_json::to_string(&err.data).unwrap_or_default();
        assert!(
            data.to_lowercase().contains("embedding"),
            "error data should classify the embedding provider failure: {data}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::SearchHit;
    use crate::search::confidence::compute_result_confidence;
    use crate::search::executor_types::SearchSource;
    use crate::search::fusion::FusedResult;

    /// Helper to create a SearchHit with test data.
    fn make_search_hit(chunk_id: i64, score: f64, exact_mult: Option<f64>) -> SearchHit {
        SearchHit {
            chunk_id,
            score,
            file_relpath: format!("src/test_{}.rs", chunk_id),
            symbol_name: Some(format!("test_fn_{}", chunk_id)),
            kind: "function".to_string(),
            start_line: 1,
            end_line: 10,
            base_score: None,
            kind_mult: None,
            exact_mult,
            preview: None,
        }
    }

    #[test]
    fn test_searchhit_to_fusedresult_fts_mode() {
        let hit = make_search_hit(42, 0.95, Some(3.0));
        let fused = searchhit_to_fused_result(&hit, "fts");

        assert_eq!(fused.chunk_id, 42);
        assert!((fused.score - 0.95).abs() < 0.001);
        assert_eq!(fused.exact_match_multiplier, Some(3.0));
        assert_eq!(fused.source_scores.len(), 1);
        assert!(fused.source_scores.contains_key(&SearchSource::FTS));
        assert!(!fused.source_scores.contains_key(&SearchSource::Vector));
    }

    #[test]
    fn test_searchhit_to_fusedresult_vector_mode() {
        let hit = make_search_hit(99, 0.80, None);
        let fused = searchhit_to_fused_result(&hit, "vector");

        assert_eq!(fused.chunk_id, 99);
        assert!((fused.score - 0.80).abs() < 0.001);
        assert_eq!(fused.exact_match_multiplier, None);
        assert_eq!(fused.source_scores.len(), 1);
        assert!(fused.source_scores.contains_key(&SearchSource::Vector));
        assert!(!fused.source_scores.contains_key(&SearchSource::FTS));
    }

    #[test]
    fn test_searchhit_to_fusedresult_hybrid_mode() {
        let hit = make_search_hit(7, 0.88, Some(1.0));
        let fused = searchhit_to_fused_result(&hit, "hybrid");

        assert_eq!(fused.chunk_id, 7);
        assert!((fused.score - 0.88).abs() < 0.001);
        assert_eq!(fused.exact_match_multiplier, Some(1.0));
        // Hybrid mode has 2 sources: FTS + Vector
        assert_eq!(fused.source_scores.len(), 2);
        assert!(fused.source_scores.contains_key(&SearchSource::FTS));
        assert!(fused.source_scores.contains_key(&SearchSource::Vector));
    }

    #[test]
    fn test_confidence_computed_from_adapter_fts() {
        let hits = vec![
            make_search_hit(1, 0.95, Some(3.0)),
            make_search_hit(2, 0.82, None),
            make_search_hit(3, 0.70, Some(1.0)),
        ];

        let all_fused: Vec<FusedResult> = hits
            .iter()
            .map(|h| FusedResult::new(h.chunk_id, h.score as f32, HashMap::new()))
            .collect();

        // Compute confidence for first hit (FTS mode)
        let fused = searchhit_to_fused_result(&hits[0], "fts");
        let confidence =
            compute_result_confidence(&fused, &all_fused, 0, fused.exact_match_multiplier);

        // source_count: 1 for FTS mode
        assert_eq!(confidence.source_count, 1);
        // score_gap: 0.95 - 0.82 = 0.13
        assert!((confidence.score_gap - 0.13).abs() < 0.01);
        // is_exact_match: exact_mult 3.0 >= 2.9 threshold
        assert!(confidence.is_exact_match);
    }

    #[test]
    fn test_confidence_computed_from_adapter_hybrid() {
        let hits = vec![
            make_search_hit(1, 0.90, None),
            make_search_hit(2, 0.85, None),
        ];

        let all_fused: Vec<FusedResult> = hits
            .iter()
            .map(|h| FusedResult::new(h.chunk_id, h.score as f32, HashMap::new()))
            .collect();

        // Compute confidence for first hit (hybrid mode)
        let fused = searchhit_to_fused_result(&hits[0], "hybrid");
        let confidence =
            compute_result_confidence(&fused, &all_fused, 0, fused.exact_match_multiplier);

        // source_count: 2 for hybrid mode (FTS + Vector)
        assert_eq!(confidence.source_count, 2);
        // score_gap: 0.90 - 0.85 = 0.05
        assert!((confidence.score_gap - 0.05).abs() < 0.01);
        // is_exact_match: None exact_mult means false
        assert!(!confidence.is_exact_match);
    }

    #[test]
    fn test_confidence_last_result_zero_gap() {
        let hits = vec![
            make_search_hit(1, 0.90, None),
            make_search_hit(2, 0.85, None),
        ];

        let all_fused: Vec<FusedResult> = hits
            .iter()
            .map(|h| FusedResult::new(h.chunk_id, h.score as f32, HashMap::new()))
            .collect();

        // Last result should have score_gap = 0.0
        let fused = searchhit_to_fused_result(&hits[1], "fts");
        let confidence =
            compute_result_confidence(&fused, &all_fused, 1, fused.exact_match_multiplier);

        assert_eq!(confidence.score_gap, 0.0);
    }

    #[test]
    fn test_confidence_exact_mult_below_threshold() {
        let hit = make_search_hit(1, 0.90, Some(2.8));
        let all_fused = vec![FusedResult::new(
            hit.chunk_id,
            hit.score as f32,
            HashMap::new(),
        )];

        let fused = searchhit_to_fused_result(&hit, "fts");
        let confidence =
            compute_result_confidence(&fused, &all_fused, 0, fused.exact_match_multiplier);

        // 2.8 < 2.9 threshold, so NOT an exact match
        assert!(!confidence.is_exact_match);
    }

    #[test]
    fn test_confidence_signals_json_serialization() {
        let hit = make_search_hit(1, 0.95, Some(3.0));
        let all_fused = vec![FusedResult::new(
            hit.chunk_id,
            hit.score as f32,
            HashMap::new(),
        )];

        let fused = searchhit_to_fused_result(&hit, "fts");
        let confidence =
            compute_result_confidence(&fused, &all_fused, 0, fused.exact_match_multiplier);

        // Verify serialization produces all 3 required fields
        let json = serde_json::to_value(&confidence).unwrap();
        assert!(json.get("source_count").is_some());
        assert!(json.get("score_gap").is_some());
        assert!(json.get("is_exact_match").is_some());

        assert_eq!(json["source_count"], 1);
        assert_eq!(json["is_exact_match"], true);
    }
}
