use anyhow::Context;
use tokio_postgres::{types::ToSql, Client, NoTls};
use serde::Serialize;

pub async fn connect() -> anyhow::Result<Client> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL env var is required (tip: use a .env file)")?;
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls).await?;
    // Spawn the connection driver
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("postgres connection error: {e}");
        }
    });

    // Configure ivfflat.probes for vector search optimization
    // This setting controls the accuracy/speed tradeoff for vector similarity queries
    // probes=10 provides ~80-85% recall with <25ms p95 latency
    client.execute("SET ivfflat.probes = 10", &[]).await?;

    Ok(client)
}

pub async fn migrate(client: &Client) -> anyhow::Result<()> {
    // Minimal migration runner: execute all migrations in order
    // IMPORTANT: batch_execute() wraps statements in an implicit transaction,
    // which prevents CREATE INDEX CONCURRENTLY from working.
    // Solution: Execute CREATE INDEX CONCURRENTLY statements separately using simple_query()

    let migrations = vec![
        include_str!("./../../migrations/0001_init.sql"),
        include_str!("./../../migrations/0002_markdown_support.sql"),
        include_str!("./../../migrations/0003_yaml_toml_support.sql"),
        include_str!("./../../migrations/0004_optimize_vector_indices.sql"),
        include_str!("./../../migrations/0005_create_materialized_views.sql"),
        include_str!("./../../migrations/0006_optimize_gin_index.sql"),
        include_str!("./../../migrations/0007_ab_testing_schema.sql"),
        include_str!("./../../migrations/0011_python_symbol_kinds.sql"),
        include_str!("./../../migrations/0014_add_enhanced_symbol_kinds.sql"),
    ];

    for sql in migrations {
        client.batch_execute(sql).await?;
    }

    // Migrations with CREATE INDEX CONCURRENTLY or multi-line string concatenation (||)
    // must use simple_query instead of batch_execute.
    // simple_query does NOT wrap in transactions, allowing CONCURRENTLY to work,
    // and handles complex SQL constructs that batch_execute may not parse correctly.

    // Migration 0008: Context query optimizations (CREATE INDEX CONCURRENTLY)
    eprintln!("Running migration 0008...");
    // Test: execute just the first CREATE INDEX statement
    client.simple_query("CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_test_links_target ON maproom.test_links(target_chunk_id);").await?;
    eprintln!("Test statement complete");
    return Ok(()); // Early return for testing

    // Migration 0009: Context cache (uses || in COMMENT statements)
    client.simple_query(include_str!("./../../migrations/0009_create_context_cache.sql")).await?;

    // Migration 0010: Add blake3_hash column (CREATE INDEX CONCURRENTLY)
    execute_with_concurrent_indexes(
        client,
        include_str!("./../../migrations/0010_add_blake3_hash.sql"),
    ).await?;

    // Migration 0012: Optimize indices (CREATE INDEX CONCURRENTLY)
    execute_with_concurrent_indexes(
        client,
        include_str!("./../../migrations/0012_optimize_indices.sql"),
    ).await?;

    // Migration 0013: Query tuning (uses || in COMMENT statements)
    client.simple_query(include_str!("./../../migrations/0013_query_tuning.sql")).await?;

    // Migration 0015: Add Ollama/Google embedding columns (CREATE INDEX CONCURRENTLY)
    execute_with_concurrent_indexes(
        client,
        include_str!("./../../migrations/0015_add_ollama_columns.sql"),
    ).await?;

    Ok(())
}

/// Execute a migration that contains CREATE INDEX CONCURRENTLY statements.
/// Uses simple_query which does NOT wrap in implicit transactions.
async fn execute_with_concurrent_indexes(client: &Client, sql: &str) -> anyhow::Result<()> {
    // After extensive testing, discovered that simple_query in tokio-postgres
    // uses the PostgreSQL "simple query protocol" which does NOT wrap statements
    // in an implicit transaction. This allows CREATE INDEX CONCURRENTLY to work.
    //
    // We just pass the entire migration SQL to simple_query and let PostgreSQL
    // handle the parsing and execution. PostgreSQL correctly handles:
    // - Multi-line string literals
    // - Comments
    // - CREATE INDEX CONCURRENTLY outside transactions
    client.simple_query(sql).await?;
    Ok(())
}

pub async fn get_or_create_repo(client: &Client, name: &str, root_path: &str) -> anyhow::Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.repos(name, root_path) VALUES ($1, $2)
             ON CONFLICT(name) DO UPDATE SET root_path = EXCLUDED.root_path
             RETURNING id",
            &[&name, &root_path],
        )
        .await?;
    let id: i64 = row.get(0);
    Ok(id)
}

pub async fn get_or_create_worktree(
    client: &Client,
    repo_id: i64,
    name: &str,
    abs_path: &str,
) -> anyhow::Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.worktrees(repo_id, name, abs_path) VALUES ($1,$2,$3)
             ON CONFLICT(repo_id, name) DO UPDATE SET abs_path = EXCLUDED.abs_path
             RETURNING id",
            &[&repo_id, &name, &abs_path],
        )
        .await?;
    let id: i64 = row.get(0);
    Ok(id)
}

pub async fn get_or_create_commit(
    client: &Client,
    repo_id: i64,
    sha: &str,
    committed_at: Option<chrono::DateTime<chrono::Utc>>,
) -> anyhow::Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.commits(repo_id, sha, committed_at) VALUES ($1,$2,$3)
             ON CONFLICT(repo_id, sha) DO UPDATE SET committed_at = COALESCE(maproom.commits.committed_at, EXCLUDED.committed_at)
             RETURNING id",
            &[&repo_id as &(dyn ToSql + Sync), &sha, &committed_at],
        )
        .await?;
    Ok(row.get(0))
}

pub async fn upsert_file(
    client: &Client,
    repo_id: i64,
    worktree_id: i64,
    commit_id: i64,
    relpath: &str,
    language: Option<&str>,
    content_hash: &str,
    size_bytes: i32,
    last_modified: Option<chrono::DateTime<chrono::Utc>>,
) -> anyhow::Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.files (repo_id, worktree_id, commit_id, relpath, language, content_hash, size_bytes, last_modified)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT(commit_id, relpath, content_hash) DO UPDATE SET
               language = COALESCE(EXCLUDED.language, maproom.files.language),
               size_bytes = EXCLUDED.size_bytes,
               last_modified = EXCLUDED.last_modified
             RETURNING id",
            &[&repo_id as &(dyn ToSql + Sync), &worktree_id, &commit_id, &relpath, &language, &content_hash, &size_bytes, &last_modified],
        )
        .await?;
    Ok(row.get(0))
}

pub async fn insert_chunk(
    client: &Client,
    file_id: i64,
    symbol_name: Option<&str>,
    kind: &str,
    signature: Option<&str>,
    docstring: Option<&str>,
    start_line: i32,
    end_line: i32,
    preview: &str,
    ts_doc_text: &str,
    recency_score: f32,
    churn_score: f32,
    metadata: Option<&serde_json::Value>,
) -> anyhow::Result<i64> {
    let row = client
        .query_one(
             "INSERT INTO maproom.chunks (
               file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, metadata
             ) VALUES (
               $1, $2::text, ($3::text)::maproom.symbol_kind, $4::text, $5::text, $6, $7, $8::text, to_tsvector('simple', unaccent($9::text)), $10, $11, $12::jsonb
             )
             ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
               symbol_name = EXCLUDED.symbol_name,
               kind = EXCLUDED.kind,
               signature = EXCLUDED.signature,
               docstring = EXCLUDED.docstring,
               preview = EXCLUDED.preview,
               ts_doc = EXCLUDED.ts_doc,
               metadata = EXCLUDED.metadata
             RETURNING id",
            &[&file_id, &symbol_name, &kind, &signature, &docstring, &start_line, &end_line, &preview, &ts_doc_text, &recency_score, &churn_score, &metadata],
        )
        .await?;
    Ok(row.get(0))
}

/// Batch insert multiple chunks for improved performance.
///
/// This function addresses the primary indexing bottleneck identified in PERF_OPT-1002:
/// - Individual INSERT operations consume 90-95% of indexing time
/// - Network round-trip latency (~1-2ms per call) dominates
/// - Batching reduces N inserts to 1 insert, expected 5-10x speedup
///
/// # Parameters
/// - `client`: Database client from connection pool
/// - `chunks`: Vector of chunk data tuples (file_id, symbol_name, kind, ...)
///
/// # Performance
/// - Expected improvement: 5-10x faster than individual inserts
/// - Batch size recommendation: 50-100 chunks per batch
/// - Transaction overhead: Single transaction per batch
pub async fn insert_chunks_batch(
    client: &Client,
    chunks: &[(
        i64,                          // file_id
        Option<String>,               // symbol_name
        String,                       // kind
        Option<String>,               // signature
        Option<String>,               // docstring
        i32,                          // start_line
        i32,                          // end_line
        String,                       // preview
        String,                       // ts_doc_text
        f32,                          // recency_score
        f32,                          // churn_score
        Option<serde_json::Value>,    // metadata
    )],
) -> anyhow::Result<Vec<i64>> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    // Build VALUES clause with parameter placeholders
    // Each chunk has 12 parameters
    let mut values_clauses = Vec::with_capacity(chunks.len());
    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(chunks.len() * 12);

    for (idx, chunk) in chunks.iter().enumerate() {
        let base = idx * 12;
        values_clauses.push(format!(
            "(${}, ${}::text, (${}::text)::maproom.symbol_kind, ${}::text, ${}::text, ${}, ${}, ${}::text, to_tsvector('simple', unaccent(${}::text)), ${}, ${}, ${}::jsonb)",
            base + 1, base + 2, base + 3, base + 4, base + 5,
            base + 6, base + 7, base + 8, base + 9, base + 10, base + 11, base + 12
        ));

        params.push(&chunk.0);  // file_id
        params.push(&chunk.1);  // symbol_name
        params.push(&chunk.2);  // kind
        params.push(&chunk.3);  // signature
        params.push(&chunk.4);  // docstring
        params.push(&chunk.5);  // start_line
        params.push(&chunk.6);  // end_line
        params.push(&chunk.7);  // preview
        params.push(&chunk.8);  // ts_doc_text
        params.push(&chunk.9);  // recency_score
        params.push(&chunk.10); // churn_score
        params.push(&chunk.11); // metadata
    }

    let query = format!(
        "INSERT INTO maproom.chunks (
           file_id, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, metadata
         ) VALUES {}
         ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
           symbol_name = EXCLUDED.symbol_name,
           kind = EXCLUDED.kind,
           signature = EXCLUDED.signature,
           docstring = EXCLUDED.docstring,
           preview = EXCLUDED.preview,
           ts_doc = EXCLUDED.ts_doc,
           metadata = EXCLUDED.metadata
         RETURNING id",
        values_clauses.join(", ")
    );

    let rows = client.query(&query, &params).await?;
    Ok(rows.iter().map(|row| row.get(0)).collect())
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub score: f32,
    pub file_relpath: String,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
}

/// Insert a chunk edge representing a relationship between two chunks
pub async fn insert_chunk_edge(
    client: &Client,
    src_chunk_id: i64,
    dst_chunk_id: i64,
    edge_type: &str,
) -> anyhow::Result<()> {
    client
        .execute(
            "INSERT INTO maproom.chunk_edges (src_chunk_id, dst_chunk_id, type)
             VALUES ($1, $2, ($3::text)::maproom.edge_type)
             ON CONFLICT (src_chunk_id, dst_chunk_id, type) DO NOTHING",
            &[&src_chunk_id, &dst_chunk_id, &edge_type],
        )
        .await?;
    Ok(())
}

/// Find a chunk by symbol name within a specific file or repository
/// This is used to resolve import targets for creating edges
pub async fn find_chunk_by_symbol(
    client: &Client,
    repo_id: i64,
    worktree_id: Option<i64>,
    symbol_name: &str,
    relpath: Option<&str>,
) -> anyhow::Result<Option<i64>> {
    let row = if let Some(wid) = worktree_id {
        if let Some(path) = relpath {
            // Find in specific file
            client
                .query_opt(
                    "SELECT c.id FROM maproom.chunks c
                     JOIN maproom.files f ON f.id = c.file_id
                     WHERE f.repo_id = $1 AND f.worktree_id = $2
                       AND f.relpath = $3 AND c.symbol_name = $4
                     ORDER BY c.id DESC LIMIT 1",
                    &[&repo_id, &wid, &path, &symbol_name],
                )
                .await?
        } else {
            // Find anywhere in worktree
            client
                .query_opt(
                    "SELECT c.id FROM maproom.chunks c
                     JOIN maproom.files f ON f.id = c.file_id
                     WHERE f.repo_id = $1 AND f.worktree_id = $2 AND c.symbol_name = $3
                     ORDER BY c.id DESC LIMIT 1",
                    &[&repo_id, &wid, &symbol_name],
                )
                .await?
        }
    } else {
        // Find anywhere in repo
        client
            .query_opt(
                "SELECT c.id FROM maproom.chunks c
                 JOIN maproom.files f ON f.id = c.file_id
                 WHERE f.repo_id = $1 AND c.symbol_name = $2
                 ORDER BY c.id DESC LIMIT 1",
                &[&repo_id, &symbol_name],
            )
            .await?
    };

    Ok(row.map(|r| r.get(0)))
}

/// Upsert embeddings for a chunk, selecting columns based on dimension.
///
/// This function dynamically selects the appropriate database columns for embeddings
/// based on the provider's dimension: 768-dim uses *_ollama columns, 1536-dim uses
/// original columns.
///
/// # Arguments
/// * `client` - Database client from connection pool
/// * `chunk_id` - Chunk ID to update
/// * `code_embedding` - Optional code embedding vector
/// * `text_embedding` - Optional text embedding vector
/// * `dimension` - Embedding dimension (768 or 1536)
///
/// # Errors
/// * Returns error if dimension is unsupported
/// * Returns error if embedding length doesn't match dimension
/// * Returns error if database update fails
///
/// # Safety
/// Column names come from compile-time constants (ColumnSet), preventing SQL injection.
/// All vector values use parameterized queries ($1, $2, etc.).
pub async fn upsert_embeddings(
    client: &Client,
    chunk_id: i64,
    code_embedding: Option<&[f32]>,
    text_embedding: Option<&[f32]>,
    dimension: usize,
) -> anyhow::Result<()> {
    use crate::db::select_columns_for_dimension;

    // Validate embedding dimensions
    if let Some(vec) = code_embedding {
        if vec.len() != dimension {
            anyhow::bail!(
                "Code embedding length {} does not match dimension {}",
                vec.len(),
                dimension
            );
        }
    }
    if let Some(vec) = text_embedding {
        if vec.len() != dimension {
            anyhow::bail!(
                "Text embedding length {} does not match dimension {}",
                vec.len(),
                dimension
            );
        }
    }

    // Select columns based on dimension
    let columns = select_columns_for_dimension(dimension)?;

    // Convert slices to Vec<f32> for parameter binding
    // tokio-postgres supports Vec<f32> -> vector conversion natively
    let code_vec = code_embedding.map(|emb| emb.to_vec());
    let text_vec = text_embedding.map(|emb| emb.to_vec());

    // Build SQL query with dynamic column names (from constants, safe from injection)
    // and parameterized vector bindings ($1, $2, etc.)
    match (code_vec, text_vec) {
        (Some(code), Some(text)) => {
            let sql = format!(
                "UPDATE maproom.chunks
                 SET {} = $1::vector,
                     {} = $2::vector,
                     updated_at = NOW()
                 WHERE id = $3",
                columns.code_embedding, columns.doc_embedding
            );
            client
                .execute(&sql, &[&code, &text, &chunk_id])
                .await
                .context("Failed to upsert embeddings")?;
        }
        (Some(code), None) => {
            let sql = format!(
                "UPDATE maproom.chunks
                 SET {} = $1::vector,
                     updated_at = NOW()
                 WHERE id = $2",
                columns.code_embedding
            );
            client
                .execute(&sql, &[&code, &chunk_id])
                .await
                .context("Failed to upsert embeddings")?;
        }
        (None, Some(text)) => {
            let sql = format!(
                "UPDATE maproom.chunks
                 SET {} = $1::vector,
                     updated_at = NOW()
                 WHERE id = $2",
                columns.doc_embedding
            );
            client
                .execute(&sql, &[&text, &chunk_id])
                .await
                .context("Failed to upsert embeddings")?;
        }
        (None, None) => {
            // Nothing to update
            return Ok(());
        }
    };

    Ok(())
}

/// Batch upsert embeddings for multiple chunks.
///
/// This function processes multiple chunks in a single transaction, improving performance
/// for bulk embedding updates. Uses the same column selection logic as `upsert_embeddings`.
///
/// # Arguments
/// * `client` - Database client from connection pool (requires mutable reference for transactions)
/// * `embeddings` - Vector of tuples: (chunk_id, code_embedding, text_embedding)
/// * `dimension` - Embedding dimension (768 or 1536)
///
/// # Errors
/// * Returns error if dimension is unsupported
/// * Returns error if any embedding length doesn't match dimension
/// * Returns error if database update fails
///
/// # Transaction Safety
/// All updates occur within a single transaction. If any update fails, all changes are rolled back.
pub async fn batch_upsert_embeddings(
    client: &mut Client,
    embeddings: &[(i64, Option<Vec<f32>>, Option<Vec<f32>>)],
    dimension: usize,
) -> anyhow::Result<()> {
    use crate::db::select_columns_for_dimension;

    if embeddings.is_empty() {
        return Ok(());
    }

    let columns = select_columns_for_dimension(dimension)?;

    // Use transaction for batch operation
    let tx = client.transaction().await?;

    for (chunk_id, code_emb, text_emb) in embeddings {
        // Validate dimensions
        if let Some(ref vec) = code_emb {
            if vec.len() != dimension {
                anyhow::bail!(
                    "Code embedding dimension mismatch for chunk {}: expected {}, got {}",
                    chunk_id,
                    dimension,
                    vec.len()
                );
            }
        }
        if let Some(ref vec) = text_emb {
            if vec.len() != dimension {
                anyhow::bail!(
                    "Text embedding dimension mismatch for chunk {}: expected {}, got {}",
                    chunk_id,
                    dimension,
                    vec.len()
                );
            }
        }

        // Build SQL query with dynamic column names (from constants, safe from injection)
        // and parameterized vector bindings ($1, $2, etc.)
        match (code_emb, text_emb) {
            (Some(code), Some(text)) => {
                let sql = format!(
                    "UPDATE maproom.chunks
                     SET {} = $1::vector,
                         {} = $2::vector,
                         updated_at = NOW()
                     WHERE id = $3",
                    columns.code_embedding, columns.doc_embedding
                );
                tx.execute(&sql, &[code, text, chunk_id]).await?;
            }
            (Some(code), None) => {
                let sql = format!(
                    "UPDATE maproom.chunks
                     SET {} = $1::vector,
                         updated_at = NOW()
                     WHERE id = $2",
                    columns.code_embedding
                );
                tx.execute(&sql, &[code, chunk_id]).await?;
            }
            (None, Some(text)) => {
                let sql = format!(
                    "UPDATE maproom.chunks
                     SET {} = $1::vector,
                         updated_at = NOW()
                     WHERE id = $2",
                    columns.doc_embedding
                );
                tx.execute(&sql, &[text, chunk_id]).await?;
            }
            (None, None) => {
                // Nothing to update for this chunk
                continue;
            }
        };
    }

    tx.commit().await?;
    Ok(())
}

pub async fn search_chunks_fts(
    client: &Client,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    k: i64,
) -> anyhow::Result<Vec<SearchHit>> {
    // Resolve repo/worktree ids
    let repo_row = client
        .query_one("SELECT id FROM maproom.repos WHERE name = $1", &[&repo])
        .await?;
    let repo_id: i64 = repo_row.get(0);
    let worktree_id: Option<i64> = if let Some(w) = worktree {
        let row = client
            .query_opt(
                "SELECT id FROM maproom.worktrees WHERE repo_id = $1 AND name = $2",
                &[&repo_id, &w],
            )
            .await?;
        row.map(|r| r.get(0))
    } else { None };

    let ts = query
        .split_whitespace()
        .map(|t| format!("{}:*", t.replace("'", "")))
        .collect::<Vec<_>>()
        .join(" & ");

    let rows = if let Some(wid) = worktree_id {
        client
            .query(
                "SELECT c.start_line, c.end_line, c.symbol_name, c.kind::text, f.relpath,
                        CASE 
                            WHEN c.kind IN ('heading_1', 'heading_2') THEN 
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $4)) * 2.0
                            WHEN c.kind = 'heading_3' THEN
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $4)) * 1.5
                            WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $4)) * 1.2
                            WHEN c.kind = 'json_key' THEN
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $4)) * 1.3
                            ELSE 
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $4))
                        END AS score
                 FROM maproom.chunks c
                 JOIN maproom.files f ON f.id = c.file_id
                 WHERE f.repo_id = $1 AND f.worktree_id = $2 AND c.ts_doc @@ to_tsquery('simple', $4)
                 ORDER BY score DESC
                 LIMIT $3",
                &[&repo_id, &wid, &k, &ts],
            )
            .await?
    } else {
        client
            .query(
                "SELECT c.start_line, c.end_line, c.symbol_name, c.kind::text, f.relpath,
                        CASE 
                            WHEN c.kind IN ('heading_1', 'heading_2') THEN 
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $3)) * 2.0
                            WHEN c.kind = 'heading_3' THEN
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $3)) * 1.5
                            WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $3)) * 1.2
                            WHEN c.kind = 'json_key' THEN
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $3)) * 1.3
                            ELSE 
                                ts_rank_cd(c.ts_doc, to_tsquery('simple', $3))
                        END AS score
                 FROM maproom.chunks c
                 JOIN maproom.files f ON f.id = c.file_id
                 WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $3)
                 ORDER BY score DESC
                 LIMIT $2",
                &[&repo_id, &k, &ts],
            )
            .await?
    };

    let hits = rows
        .into_iter()
        .map(|r| SearchHit {
            start_line: r.get(0),
            end_line: r.get(1),
            symbol_name: r.get::<_, Option<String>>(2),
            kind: r.get::<_, String>(3),
            file_relpath: r.get::<_, String>(4),
            score: r.get::<_, f32>(5),
        })
        .collect();
    Ok(hits)
}


