use anyhow::Context;
use tokio_postgres::{types::ToSql, Client, NoTls};

use super::SearchHit;

pub async fn connect() -> anyhow::Result<Client> {
    let database_url = crate::db::connection::get_database_url()
        .context("Failed to determine database connection URL")?;
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
    // Use batch_execute (simple query protocol) to avoid starting an implicit transaction
    client.batch_execute("SET ivfflat.probes = 10").await?;

    Ok(client)
}

pub async fn migrate(client: &Client) -> anyhow::Result<()> {
    // Migration runner with tracking and idempotency support
    // IMPORTANT: Migrations are tracked in maproom.schema_migrations table
    // CONCURRENT indexes are executed statement-by-statement to avoid transaction context issues

    // Step 1: Ensure schema_migrations table exists (migration 0000)
    let migration_0000 = include_str!("./../../migrations/0000_schema_migrations.sql");
    client
        .batch_execute(migration_0000)
        .await
        .context("Failed to create schema_migrations table")?;

    // Step 2: Get list of applied migrations
    let applied_migrations = get_applied_migrations(client).await?;

    // Step 3: Define all migrations in order with their version numbers
    // Format: (version, filename, sql_content, use_concurrent_handler)
    // use_concurrent_handler=true for migrations with CREATE INDEX CONCURRENTLY statements
    let all_migrations: Vec<(i32, &str, &str, bool)> = vec![
        (
            1,
            "0001_init.sql",
            include_str!("./../../migrations/0001_init.sql"),
            false,
        ),
        (
            2,
            "0002_markdown_support.sql",
            include_str!("./../../migrations/0002_markdown_support.sql"),
            false,
        ),
        (
            3,
            "0003_yaml_toml_support.sql",
            include_str!("./../../migrations/0003_yaml_toml_support.sql"),
            false,
        ),
        (
            4,
            "0004_optimize_vector_indices.sql",
            include_str!("./../../migrations/0004_optimize_vector_indices.sql"),
            false,
        ),
        (
            5,
            "0005_create_materialized_views.sql",
            include_str!("./../../migrations/0005_create_materialized_views.sql"),
            false,
        ),
        (
            6,
            "0006_optimize_gin_index.sql",
            include_str!("./../../migrations/0006_optimize_gin_index.sql"),
            false,
        ),
        (
            7,
            "0007_ab_testing_schema.sql",
            include_str!("./../../migrations/0007_ab_testing_schema.sql"),
            false,
        ),
        (
            8,
            "0008_context_query_optimizations.sql",
            include_str!("./../../migrations/0008_context_query_optimizations.sql"),
            true,
        ),
        (
            9,
            "0009_create_context_cache.sql",
            include_str!("./../../migrations/0009_create_context_cache.sql"),
            false,
        ),
        (
            10,
            "0010_add_blake3_hash.sql",
            include_str!("./../../migrations/0010_add_blake3_hash.sql"),
            true,
        ),
        (
            11,
            "0011_python_symbol_kinds.sql",
            include_str!("./../../migrations/0011_python_symbol_kinds.sql"),
            false,
        ),
        (
            12,
            "0012_optimize_indices.sql",
            include_str!("./../../migrations/0012_optimize_indices.sql"),
            true,
        ),
        (
            13,
            "0013_query_tuning.sql",
            include_str!("./../../migrations/0013_query_tuning.sql"),
            false,
        ),
        (
            14,
            "0014_add_enhanced_symbol_kinds.sql",
            include_str!("./../../migrations/0014_add_enhanced_symbol_kinds.sql"),
            false,
        ),
        (
            15,
            "0015_add_ollama_columns.sql",
            include_str!("./../../migrations/0015_add_ollama_columns.sql"),
            true,
        ),
        (
            16,
            "0016_add_updated_at_to_chunks.sql",
            include_str!("./../../migrations/0016_add_updated_at_to_chunks.sql"),
            false,
        ),
        (
            17,
            "0017_fix_index_size_limits.sql",
            include_str!("./../../migrations/0017_fix_index_size_limits.sql"),
            true,
        ),
        (
            18,
            "0018_add_blob_sha.sql",
            include_str!("./../../migrations/0018_add_blob_sha.sql"),
            false,
        ),
        (
            19,
            "0019_create_code_embeddings.sql",
            include_str!("./../../migrations/0019_create_code_embeddings.sql"),
            false,
        ),
        (
            20,
            "0020_add_worktree_tracking.sql",
            include_str!("./../../migrations/0020_add_worktree_tracking.sql"),
            false,
        ),
    ];

    // Step 4: Apply each unapplied migration
    for (version, filename, sql, use_concurrent_handler) in all_migrations {
        // Skip if already applied
        if applied_migrations.contains(&version) {
            println!(
                "⏭️  Skipping migration {}: {} (already applied)",
                version, filename
            );
            continue;
        }

        println!("🔄 Applying migration {}: {}", version, filename);

        // Execute migration based on whether it contains CONCURRENT indexes
        if use_concurrent_handler {
            execute_with_concurrent_indexes(client, sql)
                .await
                .with_context(|| format!("Failed to apply migration {}: {}", version, filename))?;
        } else {
            client
                .batch_execute(sql)
                .await
                .with_context(|| format!("Failed to apply migration {}: {}", version, filename))?;
        }

        // Record successful application
        record_migration(client, version, filename)
            .await
            .with_context(|| format!("Failed to record migration {}: {}", version, filename))?;

        println!("✅ Applied migration {}: {}", version, filename);
    }

    println!("🎉 All migrations applied successfully");
    Ok(())
}

/// Execute a migration that contains CREATE INDEX CONCURRENTLY statements.
/// Parses SQL and executes CONCURRENT statements individually to avoid transaction context issues.
async fn execute_with_concurrent_indexes(client: &Client, sql: &str) -> anyhow::Result<()> {
    // The problem: When PostgreSQL receives multiple statements in a single message
    // (even via simple_query protocol), it may execute them in a pseudo-transaction
    // context that blocks CREATE INDEX CONCURRENTLY operations.
    //
    // Solution: Parse the SQL into individual statements and execute:
    // - CREATE INDEX CONCURRENTLY statements individually (one at a time)
    // - Other statements can be batched together

    let statements = parse_sql_statements(sql);

    for stmt in statements {
        let trimmed = stmt.trim();

        // Skip empty statements
        if trimmed.is_empty() {
            continue;
        }

        // Check if this is a CREATE INDEX CONCURRENTLY statement
        let is_concurrent = trimmed.to_uppercase().contains("CREATE INDEX CONCURRENTLY")
            || trimmed
                .to_uppercase()
                .contains("CREATE UNIQUE INDEX CONCURRENTLY");

        if is_concurrent {
            // Execute CONCURRENT indexes individually using simple_query
            // This ensures they run outside any transaction context
            client.simple_query(trimmed).await.with_context(|| {
                format!(
                    "Failed to execute CONCURRENT index statement: {}",
                    truncate_for_display(trimmed, 100)
                )
            })?;
        } else {
            // Execute other statements using batch_execute
            // This is safer for regular DDL as it provides transaction boundaries
            client.batch_execute(trimmed).await.with_context(|| {
                format!(
                    "Failed to execute statement: {}",
                    truncate_for_display(trimmed, 100)
                )
            })?;
        }
    }

    Ok(())
}

/// Parse SQL into individual statements.
/// This is a simple parser that handles:
/// - Semicolon-terminated statements
/// - Single-line comments (--)
/// - Multi-line comments (/* */)
/// - String literals (single quotes)
/// - Dollar-quoted strings ($$...$$)
fn parse_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current_stmt = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;
    let mut in_dollar_quote = false;
    let mut dollar_tag = String::new();

    while let Some(ch) = chars.next() {
        // Handle single-line comments
        if ch == '-' && chars.peek() == Some(&'-') && !in_single_quote && !in_dollar_quote {
            // Skip until end of line
            while let Some(c) = chars.next() {
                if c == '\n' {
                    current_stmt.push(c);
                    break;
                }
            }
            continue;
        }

        // Handle multi-line comments
        if ch == '/' && chars.peek() == Some(&'*') && !in_single_quote && !in_dollar_quote {
            current_stmt.push(ch);
            current_stmt.push(chars.next().unwrap()); // consume '*'
                                                      // Skip until */
            while let Some(c) = chars.next() {
                current_stmt.push(c);
                if c == '*' && chars.peek() == Some(&'/') {
                    current_stmt.push(chars.next().unwrap());
                    break;
                }
            }
            continue;
        }

        // Handle dollar-quoted strings ($$...$$, $tag$...$tag$)
        if ch == '$' && !in_single_quote {
            if in_dollar_quote {
                // Check if this ends the dollar quote
                let mut potential_tag = String::from("$");
                let mut temp_chars = chars.clone();

                while let Some(&c) = temp_chars.peek() {
                    if c == '$' {
                        potential_tag.push(c);
                        temp_chars.next();
                        break;
                    } else if c.is_alphanumeric() || c == '_' {
                        potential_tag.push(c);
                        temp_chars.next();
                    } else {
                        break;
                    }
                }

                if potential_tag == dollar_tag {
                    // End of dollar quote
                    current_stmt.push_str(&potential_tag);
                    // Consume the characters we checked
                    for _ in 1..potential_tag.len() {
                        chars.next();
                    }
                    in_dollar_quote = false;
                    dollar_tag.clear();
                    continue;
                }
            } else {
                // Start of dollar quote
                let mut tag = String::from("$");
                while let Some(&c) = chars.peek() {
                    if c == '$' {
                        tag.push(c);
                        chars.next();
                        break;
                    } else if c.is_alphanumeric() || c == '_' {
                        tag.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                current_stmt.push_str(&tag);
                in_dollar_quote = true;
                dollar_tag = tag;
                continue;
            }
        }

        // Handle single quotes (string literals)
        if ch == '\'' && !in_dollar_quote {
            in_single_quote = !in_single_quote;
            current_stmt.push(ch);
            continue;
        }

        // Handle semicolon (statement terminator)
        if ch == ';' && !in_single_quote && !in_dollar_quote {
            current_stmt.push(ch);
            statements.push(current_stmt.trim().to_string());
            current_stmt.clear();
            continue;
        }

        // Regular character
        current_stmt.push(ch);
    }

    // Add any remaining statement (might not end with semicolon)
    let final_stmt = current_stmt.trim();
    if !final_stmt.is_empty() {
        statements.push(final_stmt.to_string());
    }

    statements
}

/// Truncate a string for display purposes
fn truncate_for_display(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Get list of applied migration versions from schema_migrations table
pub async fn get_applied_migrations(client: &Client) -> anyhow::Result<std::collections::HashSet<i32>> {
    // First check if the schema_migrations table exists
    let table_exists = client
        .query_opt(
            "SELECT 1 FROM information_schema.tables
             WHERE table_schema = 'maproom' AND table_name = 'schema_migrations'",
            &[],
        )
        .await?
        .is_some();

    if !table_exists {
        // Table doesn't exist yet, return empty set
        return Ok(std::collections::HashSet::new());
    }

    // Query the applied migrations
    let rows = client
        .query("SELECT version FROM maproom.schema_migrations", &[])
        .await?;

    let applied: std::collections::HashSet<i32> =
        rows.iter().map(|row| row.get::<_, i32>(0)).collect();

    Ok(applied)
}

/// Record a successfully applied migration in schema_migrations table
async fn record_migration(client: &Client, version: i32, filename: &str) -> anyhow::Result<()> {
    client
        .execute(
            "INSERT INTO maproom.schema_migrations (version, filename)
             VALUES ($1, $2)
             ON CONFLICT (version) DO NOTHING",
            &[&version, &filename],
        )
        .await?;
    Ok(())
}

pub async fn get_or_create_repo(
    client: &Client,
    name: &str,
    root_path: &str,
) -> anyhow::Result<i64> {
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

/// Get repository by name
pub async fn get_repo_by_name(
    client: &Client,
    name: &str,
) -> anyhow::Result<Option<super::RepoInfo>> {
    let row = client
        .query_opt(
            "SELECT id, name, root_path FROM maproom.repos WHERE name = $1",
            &[&name],
        )
        .await?;

    Ok(row.map(|r| super::RepoInfo {
        id: r.get(0),
        name: r.get(1),
        root_path: r.get(2),
    }))
}

/// Get worktree by name within a repository
pub async fn get_worktree_by_name(
    client: &Client,
    repo_id: i64,
    name: &str,
) -> anyhow::Result<Option<super::WorktreeInfo>> {
    let row = client
        .query_opt(
            "SELECT id, repo_id, name, abs_path FROM maproom.worktrees WHERE repo_id = $1 AND name = $2",
            &[&repo_id, &name],
        )
        .await?;

    Ok(row.map(|r| super::WorktreeInfo {
        id: r.get(0),
        repo_id: r.get(1),
        name: r.get(2),
        abs_path: r.get(3),
    }))
}

/// List all repositories
pub async fn list_repos(client: &Client) -> anyhow::Result<Vec<super::RepoInfo>> {
    let rows = client
        .query("SELECT id, name, root_path FROM maproom.repos ORDER BY name", &[])
        .await?;

    let repos = rows
        .into_iter()
        .map(|r| super::RepoInfo {
            id: r.get(0),
            name: r.get(1),
            root_path: r.get(2),
        })
        .collect();

    Ok(repos)
}

/// List all worktrees for a repository
pub async fn list_worktrees(
    client: &Client,
    repo_id: i64,
) -> anyhow::Result<Vec<super::WorktreeInfo>> {
    let rows = client
        .query(
            "SELECT id, repo_id, name, abs_path FROM maproom.worktrees WHERE repo_id = $1 ORDER BY name",
            &[&repo_id],
        )
        .await?;

    let worktrees = rows
        .into_iter()
        .map(|r| super::WorktreeInfo {
            id: r.get(0),
            repo_id: r.get(1),
            name: r.get(2),
            abs_path: r.get(3),
        })
        .collect();

    Ok(worktrees)
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
    blob_sha: &str,
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
    worktree_id: i64,
) -> anyhow::Result<i64> {
    let row = client
        .query_one(
             "INSERT INTO maproom.chunks (
               file_id, blob_sha, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, metadata, worktree_ids
             ) VALUES (
               $1, $2::text, $3::text, ($4::text)::maproom.symbol_kind, $5::text, $6::text, $7, $8, $9::text, to_tsvector('simple', unaccent($10::text)), $11, $12, $13::jsonb, jsonb_build_array($14::BIGINT)
             )
             ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
               blob_sha = EXCLUDED.blob_sha,
               symbol_name = EXCLUDED.symbol_name,
               kind = EXCLUDED.kind,
               signature = EXCLUDED.signature,
               docstring = EXCLUDED.docstring,
               preview = EXCLUDED.preview,
               ts_doc = EXCLUDED.ts_doc,
               metadata = EXCLUDED.metadata,
               worktree_ids = CASE
                   WHEN maproom.chunks.worktree_ids @> jsonb_build_array($14::BIGINT) THEN maproom.chunks.worktree_ids
                   ELSE maproom.chunks.worktree_ids || jsonb_build_array($14::BIGINT)
               END
             RETURNING id",
            &[&file_id, &blob_sha, &symbol_name, &kind, &signature, &docstring, &start_line, &end_line, &preview, &ts_doc_text, &recency_score, &churn_score, &metadata, &worktree_id],
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
        i64,                       // file_id
        String,                    // blob_sha
        Option<String>,            // symbol_name
        String,                    // kind
        Option<String>,            // signature
        Option<String>,            // docstring
        i32,                       // start_line
        i32,                       // end_line
        String,                    // preview
        String,                    // ts_doc_text
        f32,                       // recency_score
        f32,                       // churn_score
        Option<serde_json::Value>, // metadata
        i64,                       // worktree_id
    )],
) -> anyhow::Result<Vec<i64>> {
    if chunks.is_empty() {
        return Ok(Vec::new());
    }

    // Build VALUES clause with parameter placeholders
    // Each chunk has 14 parameters (added worktree_id)
    let mut values_clauses = Vec::with_capacity(chunks.len());
    let mut params: Vec<&(dyn ToSql + Sync)> = Vec::with_capacity(chunks.len() * 14);

    for (idx, chunk) in chunks.iter().enumerate() {
        let base = idx * 14;
        values_clauses.push(format!(
            "(${}, ${}::text, ${}::text, (${}::text)::maproom.symbol_kind, ${}::text, ${}::text, ${}, ${}, ${}::text, to_tsvector('simple', unaccent(${}::text)), ${}, ${}, ${}::jsonb, jsonb_build_array(${}::BIGINT))",
            base + 1, base + 2, base + 3, base + 4, base + 5, base + 6,
            base + 7, base + 8, base + 9, base + 10, base + 11, base + 12, base + 13, base + 14
        ));

        params.push(&chunk.0); // file_id
        params.push(&chunk.1); // blob_sha
        params.push(&chunk.2); // symbol_name
        params.push(&chunk.3); // kind
        params.push(&chunk.4); // signature
        params.push(&chunk.5); // docstring
        params.push(&chunk.6); // start_line
        params.push(&chunk.7); // end_line
        params.push(&chunk.8); // preview
        params.push(&chunk.9); // ts_doc_text
        params.push(&chunk.10); // recency_score
        params.push(&chunk.11); // churn_score
        params.push(&chunk.12); // metadata
        params.push(&chunk.13); // worktree_id
    }

    let query = format!(
        "INSERT INTO maproom.chunks (
           file_id, blob_sha, symbol_name, kind, signature, docstring, start_line, end_line, preview, ts_doc, recency_score, churn_score, metadata, worktree_ids
         ) VALUES {}
         ON CONFLICT(file_id, start_line, end_line) DO UPDATE SET
           blob_sha = EXCLUDED.blob_sha,
           symbol_name = EXCLUDED.symbol_name,
           kind = EXCLUDED.kind,
           signature = EXCLUDED.signature,
           docstring = EXCLUDED.docstring,
           preview = EXCLUDED.preview,
           ts_doc = EXCLUDED.ts_doc,
           metadata = EXCLUDED.metadata,
           worktree_ids = CASE
               WHEN maproom.chunks.worktree_ids @> EXCLUDED.worktree_ids THEN maproom.chunks.worktree_ids
               ELSE maproom.chunks.worktree_ids || EXCLUDED.worktree_ids
           END
         RETURNING id",
        values_clauses.join(", ")
    );

    let rows = client.query(&query, &params).await?;
    Ok(rows.iter().map(|row| row.get(0)).collect())
}


/// Normalize query for exact match detection (SEMRANK-2004b)
///
/// Handles acronym-aware camelCase to snake_case conversion:
/// - "validateProvider" → "validate_provider"
/// - "XMLParser" → "xml_parser"
/// - "validateHTTPRequest" → "validate_http_request"
/// - "HTTPSHandler" → "https_handler"
/// - "Base64Encoder" → "base64_encoder"
/// - "validate-provider" → "validate_provider"
fn normalize_for_exact_match(query: &str) -> String {
    use regex::Regex;

    let mut normalized = query.to_string();

    // Step 1: Handle consecutive uppercase (acronyms) before lowercase
    // "XMLParser" → "XML_Parser", "HTTPSHandler" → "HTTPS_Handler"
    let re1 = Regex::new(r"([A-Z]+)([A-Z][a-z])").unwrap();
    normalized = re1.replace_all(&normalized, "${1}_${2}").to_string();

    // Step 2: Handle transition from lowercase to multiple capitals (acronym after lowercase)
    // "validateHTTP" → "validate_HTTP"
    let re2 = Regex::new(r"([a-z\d])([A-Z]{2,})").unwrap();
    normalized = re2.replace_all(&normalized, "${1}_${2}").to_string();

    // Step 3: Handle camelCase → snake_case (single capital after lowercase)
    // "validateProvider" → "validate_Provider"
    let re3 = Regex::new(r"([a-z\d])([A-Z])").unwrap();
    normalized = re3.replace_all(&normalized, "${1}_${2}").to_string();

    // Step 4: Handle kebab-case, spaces, and dots → snake_case
    let re4 = Regex::new(r"[\s\-\.]").unwrap();
    normalized = re4.replace_all(&normalized, "_").to_string();

    // Step 5: Lowercase everything
    normalized = normalized.to_lowercase();

    // Step 6: Clean up multiple/trailing/leading underscores
    let re5 = Regex::new(r"_+").unwrap();
    normalized = re5.replace_all(&normalized, "_").to_string();
    let re6 = Regex::new(r"^_|_$").unwrap();
    normalized = re6.replace_all(&normalized, "").to_string();

    normalized
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

    // Convert slices to pgvector::Vector for PostgreSQL compatibility
    // The pgvector crate provides proper serialization to PostgreSQL's vector type
    let code_vec = code_embedding.map(|emb| pgvector::Vector::from(emb.to_vec()));
    let text_vec = text_embedding.map(|emb| pgvector::Vector::from(emb.to_vec()));

    // Build SQL query with dynamic column names (from constants, safe from injection)
    // and parameterized vector bindings ($1, $2, etc.)
    match (code_vec, text_vec) {
        (Some(code), Some(text)) => {
            let sql = format!(
                "UPDATE maproom.chunks
                 SET {} = $1,
                     {} = $2,
                     updated_at = NOW()
                 WHERE id = $3",
                columns.code_embedding, columns.text_embedding
            );
            client
                .execute(&sql, &[&code, &text, &chunk_id])
                .await
                .context("Failed to upsert embeddings")?;
        }
        (Some(code), None) => {
            let sql = format!(
                "UPDATE maproom.chunks
                 SET {} = $1,
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
                 SET {} = $1,
                     updated_at = NOW()
                 WHERE id = $2",
                columns.text_embedding
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

        // Convert to pgvector::Vector for PostgreSQL compatibility
        let code_vec = code_emb.as_ref().map(|v| pgvector::Vector::from(v.clone()));
        let text_vec = text_emb.as_ref().map(|v| pgvector::Vector::from(v.clone()));

        // Build SQL query with dynamic column names (from constants, safe from injection)
        // and parameterized vector bindings ($1, $2, etc.)
        match (code_vec, text_vec) {
            (Some(code), Some(text)) => {
                let sql = format!(
                    "UPDATE maproom.chunks
                     SET {} = $1,
                         {} = $2,
                         updated_at = NOW()
                     WHERE id = $3",
                    columns.code_embedding, columns.text_embedding
                );
                tx.execute(&sql, &[&code, &text, chunk_id]).await?;
            }
            (Some(code), None) => {
                let sql = format!(
                    "UPDATE maproom.chunks
                     SET {} = $1,
                         updated_at = NOW()
                     WHERE id = $2",
                    columns.code_embedding
                );
                tx.execute(&sql, &[&code, chunk_id]).await?;
            }
            (None, Some(text)) => {
                let sql = format!(
                    "UPDATE maproom.chunks
                     SET {} = $1,
                         updated_at = NOW()
                     WHERE id = $2",
                    columns.text_embedding
                );
                tx.execute(&sql, &[&text, chunk_id]).await?;
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

/// Search chunks using vector similarity with SEMRANK enhancements
///
/// Implements pgvector cosine distance search with semantic ranking:
/// - Kind-based multipliers (SEMRANK-2003): Functions/classes rank higher than tests/docs
/// - Distance to similarity conversion: 1.0 - (distance / 2.0) for normalized vectors
/// - Combined final score (SEMRANK-2005): similarity × kind_mult
///
/// # Parameters
/// - `client`: Database client
/// - `repo`: Repository name
/// - `worktree`: Optional worktree filter
/// - `embedding`: Query embedding vector (768 or 1536 dimensions)
/// - `k`: Maximum number of results
/// - `debug`: If true, include base_score (similarity) and kind_mult in results
///
/// # Returns
/// Vector of SearchHit results ordered by final_score DESC
pub async fn search_chunks_vector(
    client: &Client,
    repo: &str,
    worktree: Option<&str>,
    embedding: &[f32],
    k: i64,
    debug: bool,
) -> anyhow::Result<Vec<SearchHit>> {
    use crate::db::select_columns_for_dimension;

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
    } else {
        None
    };

    // Select columns based on embedding dimension
    let dimension = embedding.len();
    let columns = select_columns_for_dimension(dimension)?;

    // Convert embedding to pgvector::Vector
    let query_vec = pgvector::Vector::from(embedding.to_vec());

    // SEMRANK-enhanced SQL query with kind multiplier and cosine distance
    // pgvector's <=> operator computes cosine distance (0 = identical, 2 = opposite)
    // Similarity = 1.0 - (distance / 2.0) for normalized range [0, 1]
    let sql = format!(
        r#"
        WITH vector_results AS (
          SELECT
            c.id,
            c.start_line,
            c.end_line,
            c.symbol_name,
            c.kind::text,
            f.relpath,
            (1.0 - (c.{} <=> $1) / 2.0) as similarity,
            -- SEMRANK-2003: Kind-based multiplier
            CASE
              WHEN c.kind IN ('func', 'async_func') THEN 2.5
              WHEN c.kind IN ('class', 'component') THEN 2.0
              WHEN c.kind = 'hook' THEN 1.8
              WHEN c.kind IN ('module', 'type', 'struct', 'trait', 'enum') THEN 1.5
              WHEN c.kind IN ('var', 'variable', 'constant', 'method', 'async_method') THEN 1.0
              WHEN c.kind = 'heading_1' THEN 0.6
              WHEN c.kind = 'heading_2' THEN 0.5
              WHEN c.kind = 'heading_3' THEN 0.4
              WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN 0.3
              WHEN c.kind = 'other' THEN 1.0
              WHEN c.kind IS NULL THEN 1.0
              ELSE 1.0
            END as kind_mult
          FROM maproom.chunks c
          JOIN maproom.files f ON f.id = c.file_id
          WHERE c.{} IS NOT NULL
            AND f.repo_id = $2
            AND ($3::bigint IS NULL OR f.worktree_id = $3)
          ORDER BY c.{} <=> $1
          LIMIT $4
        )
        SELECT
          start_line,
          end_line,
          symbol_name,
          kind,
          relpath,
          similarity::float8,
          kind_mult::float8,
          (similarity * kind_mult)::float8 as final_score
        FROM vector_results
        ORDER BY final_score DESC
    "#,
        columns.code_embedding, columns.code_embedding, columns.code_embedding
    );

    let rows = client
        .query(&sql, &[&query_vec, &repo_id, &worktree_id, &k])
        .await?;

    // Extract results with optional debug fields
    let hits = rows
        .into_iter()
        .map(|r| {
            let final_score: f64 = r.get(7);
            SearchHit {
                start_line: r.get(0),
                end_line: r.get(1),
                symbol_name: r.get(2),
                kind: r.get(3),
                file_relpath: r.get(4),
                score: final_score,
                base_score: if debug { Some(r.get(5)) } else { None },
                kind_mult: if debug { Some(r.get(6)) } else { None },
                exact_mult: None, // Not applicable for vector search
            }
        })
        .collect();
    Ok(hits)
}

/// Search chunks using full-text search with SEMRANK enhancements
///
/// Implements SEMRANK semantic ranking with:
/// - Kind-based multipliers (SEMRANK-2003): Functions/classes rank higher than tests/docs
/// - Exact match multipliers (SEMRANK-2004a/b): Symbol name exact matches get 3.0× boost
/// - Combined final score (SEMRANK-2005): base_score × kind_mult × exact_mult
///
/// # Parameters
/// - `client`: Database client
/// - `repo`: Repository name
/// - `worktree`: Optional worktree filter
/// - `query`: Search query string
/// - `k`: Maximum number of results
/// - `debug`: If true, include base_score, kind_mult, exact_mult in results (SEMRANK-2006)
///
/// # Returns
/// Vector of SearchHit results ordered by final_score DESC
pub async fn search_chunks_fts(
    client: &Client,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    k: i64,
    debug: bool,
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
    } else {
        None
    };

    // Build FTS query string (simple tokenization with prefix matching)
    let fts_query = query
        .split_whitespace()
        .map(|t| format!("{}:*", t.replace("'", "")))
        .collect::<Vec<_>>()
        .join(" & ");

    // Normalize query for exact match detection (SEMRANK-2004b)
    let normalized_query = normalize_for_exact_match(query);

    // SEMRANK-enhanced SQL query with kind multiplier, exact match multiplier, and final score
    // Based on FTSExecutor implementation in src/search/fts.rs
    let sql = r#"
        WITH fts_results AS (
          SELECT
            c.id,
            c.start_line,
            c.end_line,
            c.symbol_name,
            c.kind::text,
            f.relpath,
            ts_rank_cd(c.ts_doc, to_tsquery('simple', $1), 32) as base_score,
            -- SEMRANK-2004a/b: Exact match multiplier (3.0× boost for exact symbol_name match)
            CASE
              WHEN LOWER(c.symbol_name) = LOWER($2) THEN 3.0
              ELSE 1.0
            END as exact_mult,
            -- SEMRANK-2003: Kind-based multiplier (source: 0001_init.sql:45 maproom.symbol_kind enum)
            CASE
              WHEN c.kind IN ('func', 'async_func') THEN 2.5
              WHEN c.kind IN ('class', 'component') THEN 2.0
              WHEN c.kind = 'hook' THEN 1.8
              WHEN c.kind IN ('module', 'type', 'struct', 'trait', 'enum') THEN 1.5
              WHEN c.kind IN ('var', 'variable', 'constant', 'method', 'async_method') THEN 1.0
              WHEN c.kind = 'heading_1' THEN 0.6
              WHEN c.kind = 'heading_2' THEN 0.5
              WHEN c.kind = 'heading_3' THEN 0.4
              WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN 0.3
              WHEN c.kind = 'other' THEN 1.0
              WHEN c.kind IS NULL THEN 1.0
              ELSE 1.0
            END as kind_mult
          FROM maproom.chunks c
          JOIN maproom.files f ON f.id = c.file_id
          WHERE c.ts_doc @@ to_tsquery('simple', $1)
            AND f.repo_id = $3
            AND ($4::bigint IS NULL OR f.worktree_id = $4)
        )
        SELECT
          start_line,
          end_line,
          symbol_name,
          kind,
          relpath,
          base_score::float8,
          kind_mult::float8,
          exact_mult::float8,
          (base_score * kind_mult * exact_mult)::float8 as final_score
        FROM fts_results
        ORDER BY final_score DESC
        LIMIT $5
    "#;

    let rows = client
        .query(
            sql,
            &[&fts_query, &normalized_query, &repo_id, &worktree_id, &k],
        )
        .await?;

    // Extract results with optional debug fields (SEMRANK-2006)
    let hits = rows
        .into_iter()
        .map(|r| {
            let final_score: f64 = r.get(8);
            SearchHit {
                start_line: r.get(0),
                end_line: r.get(1),
                symbol_name: r.get(2),
                kind: r.get(3),
                file_relpath: r.get(4),
                score: final_score,
                base_score: if debug { Some(r.get(5)) } else { None },
                kind_mult: if debug { Some(r.get(6)) } else { None },
                exact_mult: if debug { Some(r.get(7)) } else { None },
            }
        })
        .collect();
    Ok(hits)
}

/// Get a single chunk by ID with full content
pub async fn get_chunk_by_id(
    client: &Client,
    chunk_id: i64,
) -> anyhow::Result<Option<super::ChunkFull>> {
    let row = client
        .query_opt(
            "SELECT c.id, c.file_id, c.blob_sha, c.symbol_name, c.kind::text, c.signature,
                    c.docstring, c.start_line, c.end_line, c.preview, f.relpath
             FROM maproom.chunks c
             JOIN maproom.files f ON f.id = c.file_id
             WHERE c.id = $1",
            &[&chunk_id],
        )
        .await?;

    Ok(row.map(|r| super::ChunkFull {
        id: r.get(0),
        file_id: r.get(1),
        blob_sha: r.get(2),
        symbol_name: r.get(3),
        kind: r.get(4),
        signature: r.get(5),
        docstring: r.get(6),
        start_line: r.get(7),
        end_line: r.get(8),
        preview: r.get(9),
        file_path: r.get(10),
    }))
}

/// Get all chunks for a file ordered by line number
pub async fn get_file_chunks(
    client: &Client,
    file_id: i64,
) -> anyhow::Result<Vec<super::ChunkSummary>> {
    let rows = client
        .query(
            "SELECT c.id, c.symbol_name, c.kind::text, c.start_line, c.end_line, f.relpath
             FROM maproom.chunks c
             JOIN maproom.files f ON f.id = c.file_id
             WHERE c.file_id = $1
             ORDER BY c.start_line ASC",
            &[&file_id],
        )
        .await?;

    let chunks = rows
        .into_iter()
        .map(|r| super::ChunkSummary {
            id: r.get(0),
            symbol_name: r.get(1),
            kind: r.get(2),
            start_line: r.get(3),
            end_line: r.get(4),
            file_path: r.get(5),
        })
        .collect();

    Ok(chunks)
}

/// Get chunk with surrounding context (N chunks before/after by line proximity)
pub async fn get_chunk_context(
    client: &Client,
    chunk_id: i64,
    surrounding: usize,
) -> anyhow::Result<Option<super::ChunkContext>> {
    // First, get the target chunk
    let chunk = get_chunk_by_id(client, chunk_id).await?;

    let chunk = match chunk {
        Some(c) => c,
        None => return Ok(None),
    };

    // Get surrounding chunks from the same file, ordered by line proximity
    // Use ABS(start_line - target_start) to find closest chunks
    let rows = client
        .query(
            "SELECT c.id, c.symbol_name, c.kind::text, c.start_line, c.end_line, f.relpath
             FROM maproom.chunks c
             JOIN maproom.files f ON f.id = c.file_id
             WHERE c.file_id = $1 AND c.id != $2
             ORDER BY ABS(c.start_line - $3)
             LIMIT $4",
            &[&chunk.file_id, &chunk_id, &chunk.start_line, &(surrounding as i64 * 2)],
        )
        .await?;

    let surrounding_chunks = rows
        .into_iter()
        .map(|r| super::ChunkSummary {
            id: r.get(0),
            symbol_name: r.get(1),
            kind: r.get(2),
            start_line: r.get(3),
            end_line: r.get(4),
            file_path: r.get(5),
        })
        .collect();

    Ok(Some(super::ChunkContext {
        file_path: chunk.file_path.clone(),
        chunk,
        surrounding_chunks,
    }))
}

/// Hybrid search combining FTS and vector similarity using RRF fusion
///
/// Implements Reciprocal Rank Fusion to combine keyword matching (FTS) with
/// semantic similarity (vector search). This provides the best of both approaches:
/// - FTS for exact keyword matches and term relevance
/// - Vector search for semantic similarity and conceptual matching
///
/// RRF formula: score = Σ(1.0 / (60 + rank)) across both result sets
/// Items appearing in both lists naturally rank higher due to combined contributions.
///
/// # Parameters
/// - `client`: Database client
/// - `repo`: Repository name
/// - `worktree`: Optional worktree filter
/// - `query`: Search query string (for FTS)
/// - `embedding`: Query embedding vector (for vector search)
/// - `k`: Maximum number of results
/// - `debug`: If true, include base_score, kind_mult in results
///
/// # Returns
/// Vector of SearchHit results ordered by RRF score DESC
pub async fn search_chunks_hybrid(
    client: &Client,
    repo: &str,
    worktree: Option<&str>,
    query: &str,
    embedding: &[f32],
    k: i64,
    debug: bool,
) -> anyhow::Result<Vec<SearchHit>> {
    use crate::db::select_columns_for_dimension;

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
    } else {
        None
    };

    // Select columns based on embedding dimension
    let dimension = embedding.len();
    let columns = select_columns_for_dimension(dimension)?;

    // Convert embedding to pgvector::Vector
    let query_vec = pgvector::Vector::from(embedding.to_vec());

    // Build FTS query string (simple tokenization with prefix matching)
    let fts_query = query
        .split_whitespace()
        .map(|t| format!("{}:*", t.replace("'", "")))
        .collect::<Vec<_>>()
        .join(" & ");

    // Normalize query for exact match detection (SEMRANK-2004b)
    let normalized_query = normalize_for_exact_match(query);

    // Hybrid search SQL using RRF (Reciprocal Rank Fusion)
    // Standard RRF formula: score = 1/(k + rank) where k=60 is constant
    // Combines FTS and vector results with FULL OUTER JOIN for coverage
    let sql = format!(
        r#"
        WITH fts_results AS (
          SELECT
            c.id,
            c.start_line,
            c.end_line,
            c.symbol_name,
            c.kind::text,
            f.relpath,
            ROW_NUMBER() OVER (ORDER BY ts_rank_cd(c.ts_doc, to_tsquery('simple', $1), 32) DESC) - 1 as fts_rank,
            -- SEMRANK-2004a/b: Exact match multiplier
            CASE
              WHEN LOWER(c.symbol_name) = LOWER($2) THEN 3.0
              ELSE 1.0
            END as exact_mult,
            -- SEMRANK-2003: Kind-based multiplier
            CASE
              WHEN c.kind IN ('func', 'async_func') THEN 2.5
              WHEN c.kind IN ('class', 'component') THEN 2.0
              WHEN c.kind = 'hook' THEN 1.8
              WHEN c.kind IN ('module', 'type', 'struct', 'trait', 'enum') THEN 1.5
              WHEN c.kind IN ('var', 'variable', 'constant', 'method', 'async_method') THEN 1.0
              WHEN c.kind = 'heading_1' THEN 0.6
              WHEN c.kind = 'heading_2' THEN 0.5
              WHEN c.kind = 'heading_3' THEN 0.4
              WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN 0.3
              WHEN c.kind = 'other' THEN 1.0
              WHEN c.kind IS NULL THEN 1.0
              ELSE 1.0
            END as kind_mult
          FROM maproom.chunks c
          JOIN maproom.files f ON f.id = c.file_id
          WHERE c.ts_doc @@ to_tsquery('simple', $1)
            AND f.repo_id = $3
            AND ($4::bigint IS NULL OR f.worktree_id = $4)
          ORDER BY ts_rank_cd(c.ts_doc, to_tsquery('simple', $1), 32) DESC
          LIMIT $5 * 2
        ),
        vec_results AS (
          SELECT
            c.id,
            c.start_line,
            c.end_line,
            c.symbol_name,
            c.kind::text,
            f.relpath,
            ROW_NUMBER() OVER (ORDER BY c.{} <=> $6) - 1 as vec_rank,
            -- SEMRANK-2003: Kind-based multiplier (same as FTS)
            CASE
              WHEN c.kind IN ('func', 'async_func') THEN 2.5
              WHEN c.kind IN ('class', 'component') THEN 2.0
              WHEN c.kind = 'hook' THEN 1.8
              WHEN c.kind IN ('module', 'type', 'struct', 'trait', 'enum') THEN 1.5
              WHEN c.kind IN ('var', 'variable', 'constant', 'method', 'async_method') THEN 1.0
              WHEN c.kind = 'heading_1' THEN 0.6
              WHEN c.kind = 'heading_2' THEN 0.5
              WHEN c.kind = 'heading_3' THEN 0.4
              WHEN c.kind IN ('heading_4', 'heading_5', 'heading_6') THEN 0.3
              WHEN c.kind = 'other' THEN 1.0
              WHEN c.kind IS NULL THEN 1.0
              ELSE 1.0
            END as kind_mult
          FROM maproom.chunks c
          JOIN maproom.files f ON f.id = c.file_id
          WHERE c.{} IS NOT NULL
            AND f.repo_id = $3
            AND ($4::bigint IS NULL OR f.worktree_id = $4)
          ORDER BY c.{} <=> $6
          LIMIT $5 * 2
        )
        SELECT
          COALESCE(f.start_line, v.start_line) as start_line,
          COALESCE(f.end_line, v.end_line) as end_line,
          COALESCE(f.symbol_name, v.symbol_name) as symbol_name,
          COALESCE(f.kind, v.kind) as kind,
          COALESCE(f.relpath, v.relpath) as relpath,
          COALESCE(f.kind_mult, v.kind_mult, 1.0)::float8 as kind_mult,
          COALESCE(f.exact_mult, 1.0)::float8 as exact_mult,
          (
            COALESCE(1.0 / (60.0 + f.fts_rank::float8), 0.0) +
            COALESCE(1.0 / (60.0 + v.vec_rank::float8), 0.0)
          ) * COALESCE(f.kind_mult, v.kind_mult, 1.0) * COALESCE(f.exact_mult, 1.0) as rrf_score
        FROM fts_results f
        FULL OUTER JOIN vec_results v ON f.id = v.id
        ORDER BY rrf_score DESC
        LIMIT $5
    "#,
        columns.code_embedding, columns.code_embedding, columns.code_embedding
    );

    let rows = client
        .query(
            &sql,
            &[&fts_query, &normalized_query, &repo_id, &worktree_id, &k, &query_vec],
        )
        .await?;

    // Extract results with optional debug fields
    let hits = rows
        .into_iter()
        .map(|r| {
            let rrf_score: f64 = r.get(7);
            SearchHit {
                start_line: r.get(0),
                end_line: r.get(1),
                symbol_name: r.get(2),
                kind: r.get(3),
                file_relpath: r.get(4),
                score: rrf_score,
                base_score: if debug { Some(rrf_score) } else { None },
                kind_mult: if debug { Some(r.get(5)) } else { None },
                exact_mult: if debug { Some(r.get(6)) } else { None },
            }
        })
        .collect();
    Ok(hits)
}
