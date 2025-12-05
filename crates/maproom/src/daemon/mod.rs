pub mod protocol;
pub mod session;
pub mod types;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

use crate::context::{AssemblyStrategy, DefaultAssemblyStrategy, ExpandOptions};
use crate::db::{connect, SearchHit, SqliteStore};
use crate::embedding::EmbeddingService;

use self::types::{
    ContextParams, JsonRpcRequest, JsonRpcResponse, RepoStatus, SearchParams, StatusParams,
    StatusResult, WorktreeStatus,
};

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
    store: Arc<SqliteStore>,
    embedding_service: EmbeddingService,
    context_assembler: DefaultAssemblyStrategy,
}

impl DaemonState {
    fn new(store: Arc<SqliteStore>, embedding_service: EmbeddingService) -> Self {
        Self {
            store: store.clone(),
            embedding_service,
            context_assembler: DefaultAssemblyStrategy::new(store),
        }
    }
}

pub async fn run() -> Result<()> {
    info!("Daemon mode starting...");

    // Initialize SqliteStore
    let store = Arc::new(
        connect()
            .await
            .context("Failed to initialize database store")?,
    );
    info!("Database backend: SQLite");

    // Initialize Embedding Service
    let embedding_service = EmbeddingService::from_env()
        .await
        .context("Failed to initialize embedding service")?;

    let state = Arc::new(DaemonState::new(store, embedding_service));

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

            match execute_search(state, params).await {
                Ok(results) => JsonRpcResponse::success(id, results),
                Err(e) => {
                    error!("Search failed: {}", e);
                    JsonRpcResponse::error(
                        id,
                        -32000,
                        "Search failed".to_string(),
                        Some(serde_json::json!(e.to_string())),
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
            let params: StatusParams = match serde_json::from_value(
                request.params.clone().unwrap_or(serde_json::Value::Null),
            ) {
                Ok(p) => p,
                Err(_) => StatusParams::default(),
            };

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
            state
                .store
                .search_chunks_fts(
                    &params.repo,
                    params.worktree.as_deref(),
                    &params.query,
                    fetch_k,
                    false, // debug
                )
                .await
                .context("FTS search execution failed")?
        }
        "vector" => {
            // Vector mode: Semantic search using embeddings
            let query_embedding = state
                .embedding_service
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
                )
                .await
                .context("Vector search execution failed")?
        }
        "hybrid" => {
            // Hybrid mode: Try to get embedding for hybrid search
            // Falls back gracefully if embedding service unavailable
            let query_embedding_result = state.embedding_service.embed_text(&params.query).await;

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
                        )
                        .await
                        .unwrap_or_else(|_| {
                            // Hybrid failed, will fall back to FTS below
                            Vec::new()
                        })
                }
                Err(_) => {
                    // No embeddings available, use FTS directly
                    state
                        .store
                        .search_chunks_fts(
                            &params.repo,
                            params.worktree.as_deref(),
                            &params.query,
                            fetch_k,
                            false, // debug
                        )
                        .await
                        .context("FTS search execution failed")?
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

    // Format response - SearchHit already contains all needed fields
    let formatted_hits: Vec<serde_json::Value> = hits
        .iter()
        .filter(|hit| {
            // Apply threshold filter if specified
            if let Some(thresh) = params.threshold {
                hit.score >= thresh as f64
            } else {
                true
            }
        })
        .map(|hit| {
            serde_json::json!({
                "score": hit.score,
                "start_line": hit.start_line,
                "end_line": hit.end_line,
                "symbol_name": hit.symbol_name,
                "kind": hit.kind,
                "file_path": hit.file_relpath,
            })
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
