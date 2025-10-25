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
    let migrations = vec![
        include_str!("./../../migrations/0001_init.sql"),
        include_str!("./../../migrations/0002_markdown_support.sql"),
        include_str!("./../../migrations/0003_yaml_toml_support.sql"),
        include_str!("./../../migrations/0004_optimize_vector_indices.sql"),
        include_str!("./../../migrations/0005_create_materialized_views.sql"),
        include_str!("./../../migrations/0006_optimize_gin_index.sql"),
        include_str!("./../../migrations/0007_ab_testing_schema.sql"),
        include_str!("./../../migrations/0008_context_query_optimizations.sql"),
        include_str!("./../../migrations/0009_create_context_cache.sql"),
        include_str!("./../../migrations/0010_add_blake3_hash.sql"),
        include_str!("./../../migrations/0011_python_symbol_kinds.sql"),
        include_str!("./../../migrations/0012_optimize_indices.sql"),
        include_str!("./../../migrations/0013_query_tuning.sql"),
    ];

    for sql in migrations {
        client.batch_execute(sql).await?;
    }
    Ok(())
}

pub async fn get_or_create_repo(client: &Client, name: &str, root_path: &str) -> anyhow::Result<i64> {
    let row = client
        .query_one(
            "INSERT INTO maproom.repos(name, root_path) VALUES ($1,$2)
             ON CONFLICT(name) DO UPDATE SET root_path = EXCLUDED.root_path
             RETURNING id",
            &[&name, &root_path],
        )
        .await?;
    Ok(row.get(0))
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
    Ok(row.get(0))
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


