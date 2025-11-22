pub mod types;

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info};

use crewchief_maproom::db::{create_pool, PgPool};
use crewchief_maproom::embedding::EmbeddingService;
use crewchief_maproom::search::fts::{normalize_for_exact_match, FTSExecutor};
use crewchief_maproom::search::types::SearchMode;
use crewchief_maproom::search::vector::VectorExecutor;

use self::types::{JsonRpcRequest, JsonRpcResponse, SearchParams};

struct DaemonState {
    pool: PgPool,
    embedding_service: EmbeddingService,
}

pub async fn run() -> Result<()> {
    info!("Daemon mode starting...");

    // Initialize DB pool
    let pool = create_pool().await?;

    // Initialize Embedding Service
    let embedding_service = EmbeddingService::from_env()
        .await
        .context("Failed to initialize embedding service")?;

    let state = Arc::new(DaemonState {
        pool,
        embedding_service,
    });

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
    let client = state.pool.get().await?;

    // Resolve repo_id
    let repo_row = client
        .query_one(
            "SELECT id FROM maproom.repos WHERE name = $1",
            &[&params.repo],
        )
        .await
        .context(format!("Repository '{}' not found", params.repo))?;
    let repo_id: i64 = repo_row.get(0);

    // Resolve worktree_id
    let worktree_id: Option<i64> = if let Some(w) = &params.worktree {
        let row = client
            .query_opt(
                "SELECT id FROM maproom.worktrees WHERE repo_id = $1 AND name = $2",
                &[&repo_id, w],
            )
            .await?;
        row.map(|r| r.get(0))
    } else {
        None
    };

    // Determine search mode (default to "hybrid" for backward compatibility)
    let mode = params.mode.as_deref().unwrap_or("hybrid");

    // Validate mode
    if !matches!(mode, "fts" | "vector" | "hybrid") {
        anyhow::bail!(
            "Invalid search mode: '{}'. Valid modes: fts, vector, hybrid",
            mode
        );
    }

    let k = params.limit.unwrap_or(10);

    // Route to appropriate executor based on mode
    let ranked_results = match mode {
        "fts" => {
            // FTS mode: Full-text search only (no embeddings required)
            let normalized_query = normalize_for_exact_match(&params.query);
            let fts_query = params.query.split_whitespace().collect::<Vec<_>>().join(" & ");

            FTSExecutor::execute(
                &client,
                &fts_query,
                &normalized_query,
                repo_id,
                worktree_id,
                k,
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

            VectorExecutor::execute(
                &client,
                &query_embedding,
                SearchMode::Code,
                repo_id,
                worktree_id,
                k,
            )
            .await
            .context("Vector search execution failed")?
        }
        "hybrid" => {
            // Hybrid mode: Try vector search first, fall back to FTS if it fails
            // This gracefully handles the case where embeddings are not available
            let query_embedding_result = state
                .embedding_service
                .embed_text(&params.query)
                .await;

            match query_embedding_result {
                Ok(query_embedding) => {
                    // Embeddings available, try vector search
                    let vector_result = VectorExecutor::execute(
                        &client,
                        &query_embedding,
                        SearchMode::Code,
                        repo_id,
                        worktree_id,
                        k,
                    )
                    .await;

                    match vector_result {
                        Ok(results) => results,
                        Err(_) => {
                            // Vector search failed, fall back to FTS
                            let normalized_query = normalize_for_exact_match(&params.query);
                            let fts_query = params.query.split_whitespace().collect::<Vec<_>>().join(" & ");
                            FTSExecutor::execute(
                                &client,
                                &fts_query,
                                &normalized_query,
                                repo_id,
                                worktree_id,
                                k,
                            )
                            .await
                            .unwrap_or_else(|_| {
                                crewchief_maproom::search::executor_types::RankedResults::empty(
                                    crewchief_maproom::search::executor_types::SearchSource::FTS
                                )
                            })
                        }
                    }
                }
                Err(_) => {
                    // No embeddings available, use FTS directly
                    let normalized_query = normalize_for_exact_match(&params.query);
                    let fts_query = params.query.split_whitespace().collect::<Vec<_>>().join(" & ");
                    FTSExecutor::execute(
                        &client,
                        &fts_query,
                        &normalized_query,
                        repo_id,
                        worktree_id,
                        k,
                    )
                    .await
                    .context("FTS search execution failed")?
                }
            }
        }
        _ => unreachable!("Mode validation should prevent this"),
    };

    // Fetch details
    let mut hits = Vec::new();
    for result in ranked_results.results {
        if let Some(thresh) = params.threshold {
            if result.score < thresh {
                continue;
            }
        }

        let chunk_row = client
            .query_opt(
                r#"
                SELECT
                    c.start_line,
                    c.end_line,
                    c.symbol_name,
                    c.kind::text,
                    f.relpath
                FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE c.id = $1
                "#,
                &[&result.chunk_id],
            )
            .await?;

        if let Some(row) = chunk_row {
            hits.push(serde_json::json!({
                "chunk_id": result.chunk_id,
                "score": result.score,
                "start_line": row.get::<_, i32>(0),
                "end_line": row.get::<_, i32>(1),
                "symbol_name": row.get::<_, Option<String>>(2),
                "kind": row.get::<_, String>(3),
                "file_path": row.get::<_, String>(4),
            }));
        }
    }

    Ok(serde_json::json!({
        "hits": hits,
        "total": hits.len(),
        "query": params.query,
        "mode": mode,
        "k": k,
        "threshold": params.threshold,
    }))
}
