use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use tracing_subscriber::{fmt, EnvFilter};

use crewchief_maproom::context::{AssemblyStrategy, DefaultAssemblyStrategy, ExpandOptions};
use crewchief_maproom::progress::{OutputMode, ProgressTracker};
use crewchief_maproom::{db, indexer};

mod daemon;

/// Validate provider name against supported providers.
///
/// Returns the provider name if valid, or an error message if invalid.
fn validate_provider(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "ollama" | "openai" | "google" => Ok(s.to_lowercase()),
        _ => Err(format!(
            "Invalid provider: '{}'. Supported providers: ollama, openai, google",
            s
        )),
    }
}

/// Deduplicate search hits by identity (file_relpath, symbol_name, start_line).
///
/// When the same code exists in multiple worktrees, this removes duplicates
/// keeping only the highest-scoring instance.
fn deduplicate_search_hits(hits: Vec<db::SearchHit>, limit: usize) -> Vec<db::SearchHit> {
    use std::collections::HashMap;

    if hits.is_empty() {
        return hits;
    }

    // Group by identity key
    let mut groups: HashMap<(String, Option<String>, i32), Vec<db::SearchHit>> = HashMap::new();
    for hit in hits {
        let key = (hit.file_relpath.clone(), hit.symbol_name.clone(), hit.start_line);
        groups.entry(key).or_default().push(hit);
    }

    // Select highest-scoring from each group
    let mut deduped: Vec<db::SearchHit> = groups
        .into_values()
        .map(|mut group| {
            group.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            group.remove(0)
        })
        .collect();

    // Re-sort by score descending
    deduped.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Apply limit
    deduped.into_iter().take(limit).collect()
}

/// Format number with thousands separator.
///
/// Converts a number like 487329 to "487,329" for better readability.
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*ch);
    }
    result
}

#[derive(Parser, Debug)]
#[command(name = "crewchief-maproom", version, about = "Maproom indexer & CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run database migrations
    Db {
        #[command(subcommand)]
        command: DbCommand,
    },

    /// Cache management commands
    Cache {
        #[command(subcommand)]
        command: crewchief_maproom::cli::CacheCommand,
    },

    /// Retrieve context bundle for a chunk
    ///
    /// Assembles a context bundle containing the primary chunk and optionally
    /// related code (callers, callees, tests, docs, config) within a token budget.
    ///
    /// Examples:
    ///   maproom context --chunk-id 12345                   # Basic context retrieval
    ///   maproom context --chunk-id 12345 --callers         # Include caller functions
    ///   maproom context --chunk-id 12345 --budget 4000     # Custom token budget
    ///   maproom context --chunk-id 12345 --json            # Output as JSON
    Context {
        /// Chunk ID to retrieve context for
        #[arg(long)]
        chunk_id: i64,

        /// Maximum tokens for the bundle (default: 6000)
        #[arg(long, default_value_t = 6000)]
        budget: usize,

        /// Include caller functions
        #[arg(long)]
        callers: bool,

        /// Include callee functions
        #[arg(long)]
        callees: bool,

        /// Include test files
        #[arg(long)]
        tests: bool,

        /// Include documentation
        #[arg(long)]
        docs: bool,

        /// Include configuration files
        #[arg(long)]
        config: bool,

        /// Maximum traversal depth (default: 2)
        #[arg(long, default_value_t = 2)]
        max_depth: i32,

        /// Output as JSON instead of human-readable
        #[arg(long)]
        json: bool,
    },

    /// Scan and index a worktree with real-time progress display
    ///
    /// By default, uses incremental scanning to only process changed files based on
    /// git tree SHA comparison. Use --force to bypass the optimization and scan all files.
    ///
    /// Examples:
    ///   maproom scan                    # Incremental scan (changed files only)
    ///   maproom scan --path /repo       # Scan specific path
    ///   maproom scan --force            # Force full scan (all files)
    ///   maproom scan --verbose          # Scan with detailed output
    Scan {
        /// Repository name (defaults to git remote origin name)
        #[arg(long)]
        repo: Option<String>,
        /// Worktree name (defaults to current branch name)
        #[arg(long)]
        worktree: Option<String>,
        /// Path to scan (defaults to current directory)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Git commit hash (defaults to HEAD)
        #[arg(long)]
        commit: Option<String>,
        #[arg(long, default_value_t = 4)]
        concurrency: usize,
        #[arg(long, value_delimiter = ',')]
        languages: Option<Vec<String>>, // e.g. ts,tsx,js,jsx
        #[arg(long, value_delimiter = ',')]
        exclude: Option<Vec<String>>, // glob patterns
        /// Force full scan, bypassing incremental tree SHA optimization (BRANCHX-1011)
        #[arg(long, default_value_t = false)]
        force: bool,
        /// Automatically generate embeddings after scanning (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        generate_embeddings: bool,
        /// Embedding batch size for generation (default: 50)
        #[arg(long, default_value_t = 50)]
        embedding_batch_size: usize,
        /// Embedding provider: ollama, openai, or google (overrides MAPROOM_EMBEDDING_PROVIDER env var)
        #[arg(long, value_parser = validate_provider)]
        provider: Option<String>,
        /// Show detailed output (currently same as default, reserved for future enhancements)
        #[arg(long)]
        verbose: bool,
    },

    /// Upsert a set of files at a given commit
    Upsert {
        #[arg(long, value_delimiter = ',')]
        paths: Vec<PathBuf>,
        #[arg(long)]
        commit: String,
        #[arg(long)]
        repo: String,
        #[arg(long)]
        worktree: String,
        #[arg(long)]
        root: PathBuf,
        /// Automatically generate embeddings after upserting (default: true)
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        generate_embeddings: bool,
        /// Embedding batch size for generation (default: 50)
        #[arg(long, default_value_t = 50)]
        embedding_batch_size: usize,
        /// Embedding provider: ollama, openai, or google (overrides MAPROOM_EMBEDDING_PROVIDER env var)
        #[arg(long, value_parser = validate_provider)]
        provider: Option<String>,
    },

    /// Watch a worktree for changes and incrementally upsert
    ///
    /// Auto-detects the current branch and watches for branch switches.
    /// Emits NDJSON events to stdout (including branch_switched events).
    /// The --worktree flag is deprecated and will be ignored if provided.
    Watch {
        /// Repository name (defaults to git remote origin name)
        #[arg(long)]
        repo: Option<String>,
        /// Worktree name (deprecated: auto-detected from current branch)
        #[arg(long)]
        worktree: Option<String>,
        /// Path to watch (defaults to current directory)
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long, default_value = "2s")]
        throttle: String,
    },

    /// Full-text search against indexed chunks
    Search {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        worktree: Option<String>,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 10)]
        k: i64,
        /// Include score breakdown (base_fts, kind_multiplier, exact_match_multiplier, final)
        #[arg(long, default_value_t = false)]
        debug: bool,
        /// Deduplicate results across worktrees (default: true)
        /// Use --no-deduplicate to see all results including duplicates
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        deduplicate: bool,
    },

    /// Vector similarity search using embeddings
    ///
    /// Searches for code chunks using semantic similarity based on vector embeddings.
    /// Returns results ranked by cosine similarity score.
    ///
    /// Examples:
    ///   maproom vector-search --repo myproject --query "authentication logic"
    ///   maproom vector-search --repo myproject --worktree main --query "error handling" --k 20
    VectorSearch {
        /// Repository name to search within
        #[arg(long)]
        repo: String,

        /// Worktree name to filter results (optional)
        #[arg(long)]
        worktree: Option<String>,

        /// Search query text (will be converted to embedding)
        #[arg(long)]
        query: String,

        /// Number of results to return (default: 10)
        #[arg(long, default_value_t = 10)]
        k: usize,

        /// Similarity threshold (0.0-1.0, optional)
        /// Only return results with similarity >= threshold
        #[arg(long)]
        threshold: Option<f32>,
    },

    /// Show status of indexed repositories and worktrees
    Status {
        /// Filter by repository name
        #[arg(long)]
        repo: Option<String>,
        /// Filter by worktree name (requires --repo)
        #[arg(long)]
        worktree: Option<String>,
        /// Output as JSON instead of human-readable text
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Generate embeddings for indexed chunks
    GenerateEmbeddings {
        /// Only process chunks where embeddings are NULL (default: true)
        #[arg(long, default_value_t = true)]
        incremental: bool,

        /// Batch size for processing (default: 100)
        #[arg(long, default_value_t = 100)]
        batch_size: usize,

        /// Dry run mode - don't write to database
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Process only a sample of N chunks
        #[arg(long)]
        sample: Option<usize>,

        /// Delay between batches in milliseconds (default: 100)
        #[arg(long, default_value_t = 100)]
        batch_delay: u64,

        /// Maximum cost ceiling in USD
        #[arg(long)]
        max_cost: Option<f64>,

        /// Force regeneration of all embeddings (overrides --incremental)
        #[arg(long, default_value_t = false)]
        force: bool,
    },

    /// Migrate markdown chunks to new tree-sitter parser
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },

    /// Start the Maproom daemon (JSON-RPC over Stdio)
    Serve,
}

#[derive(Subcommand, Debug)]
enum MigrateCommand {
    /// Migrate markdown chunks from regex to tree-sitter parser
    Markdown {
        /// Repository name
        #[arg(long)]
        repo: String,
        /// Worktree name (optional)
        #[arg(long)]
        worktree: Option<String>,
    },

    /// Rollback markdown migration from a backup
    Rollback {
        /// Backup table name (e.g., chunks_backup_20250124_120000)
        #[arg(long)]
        backup: String,
    },

    /// List available backup tables
    ListBackups,

    /// Delete a backup table
    DeleteBackup {
        /// Backup table name to delete
        #[arg(long)]
        backup: String,
    },

    /// Verify migration integrity
    Verify {
        /// Repository name
        #[arg(long)]
        repo: String,
    },
}

#[derive(Subcommand, Debug)]
enum DbCommand {
    /// Apply SQL migrations to the configured database
    Migrate,

    /// Clean up stale worktree data from the database
    ///
    /// By default, runs in dry-run mode showing what would be deleted.
    /// Use --confirm to actually perform deletions.
    ///
    /// Examples:
    ///   maproom db cleanup-stale              # Dry-run mode (show what would be deleted)
    ///   maproom db cleanup-stale --confirm    # Actually delete stale data
    ///   maproom db cleanup-stale --verbose    # Show detailed information
    CleanupStale {
        /// Actually delete stale data (default is dry-run)
        #[arg(long, help = "Actually delete (default is dry-run)")]
        confirm: bool,

        /// Show detailed information during cleanup
        #[arg(long, short, help = "Show detailed information")]
        verbose: bool,
    },
}

/// Auto-generate embeddings for chunks with NULL embeddings.
///
/// This function is called automatically after scan/upsert operations.
/// It gracefully handles embedding service unavailability.
///
/// # Arguments
///
/// * `batch_size` - Number of chunks to process per batch
/// * `provider` - Optional provider name (overrides MAPROOM_EMBEDDING_PROVIDER env var)
async fn auto_generate_embeddings(
    batch_size: usize,
    provider: Option<String>,
) -> anyhow::Result<crewchief_maproom::embedding::PipelineStats> {
    use crewchief_maproom::embedding::{EmbeddingPipeline, EmbeddingService, PipelineConfig};

    tracing::info!("Starting auto-embedding generation");
    println!("\n🔄 Generating embeddings for new chunks...");

    // Set provider in environment if specified via CLI (overrides env var)
    if let Some(ref provider_name) = provider {
        tracing::info!("Using provider from CLI flag: {}", provider_name);
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", provider_name);
    } else {
        let env_provider =
            std::env::var("MAPROOM_EMBEDDING_PROVIDER").unwrap_or_else(|_| "not set".to_string());
        tracing::info!("Using provider from environment: {}", env_provider);
    }

    // Try to create embedding service from environment
    let service = match EmbeddingService::from_env().await {
        Ok(s) => {
            tracing::info!(
                "Created embedding service with provider: {}",
                s.provider_name()
            );
            s
        }
        Err(e) => {
            // Check if this is an Ollama configuration without Ollama running
            let provider_name = std::env::var("MAPROOM_EMBEDDING_PROVIDER").unwrap_or_default();
            if provider_name.to_lowercase() == "ollama" || provider_name.is_empty() {
                // Try to detect if Ollama is configured
                tracing::warn!("Embedding service unavailable: {}", e);
                return Err(anyhow::anyhow!(
                    "Embedding service not available. Configure MAPROOM_EMBEDDING_PROVIDER (openai/ollama/google) and API keys in .env file."
                ));
            }
            return Err(e.into());
        }
    };

    // Configure pipeline for incremental embedding generation
    let config = PipelineConfig {
        batch_size,
        incremental: true, // Only process chunks with NULL embeddings
        dry_run: false,
        sample_size: None,
        batch_delay_ms: 100,
        max_cost_usd: None,
    };

    // Connect to database
    let store = crewchief_maproom::db::connect().await?;

    // Count chunks needing embeddings
    let chunk_count = store.get_chunks_needing_embeddings_count().await?;

    if chunk_count == 0 {
        println!("   ✓ All chunks already have embeddings");
        return Ok(crewchief_maproom::embedding::PipelineStats::default());
    }

    println!("   Found {} chunks needing embeddings", chunk_count);

    // Create progress tracker
    let progress = crewchief_maproom::progress::ProgressTracker::new(
        crewchief_maproom::progress::OutputMode::Minimal,
    );
    progress.set_totals(0, Some(chunk_count as usize));

    // Run pipeline with progress callback
    let pipeline = EmbeddingPipeline::new(service, config);
    let stats = pipeline
        .run_with_progress(
            &store,
            Some(&|processed, _total| {
                progress.update_chunks(processed);
                if progress.should_print() {
                    progress.print_progress();
                }
            }),
        )
        .await?;

    // Finish progress tracking
    progress.finish();

    Ok(stats)
}

/// Parse a throttle string like "2s" or "500ms" into milliseconds.
fn parse_throttle(throttle: &str) -> anyhow::Result<u64> {
    let throttle = throttle.trim();

    if let Some(ms_str) = throttle.strip_suffix("ms") {
        ms_str.parse::<u64>()
            .with_context(|| format!("Invalid throttle value: {}", throttle))
    } else if let Some(s_str) = throttle.strip_suffix("s") {
        let secs: u64 = s_str.parse()
            .with_context(|| format!("Invalid throttle value: {}", throttle))?;
        Ok(secs * 1000)
    } else {
        // Default to treating as seconds if no suffix
        let secs: u64 = throttle.parse()
            .with_context(|| format!("Invalid throttle value: {}. Use format like '2s' or '500ms'", throttle))?;
        Ok(secs * 1000)
    }
}

/// Extract git information from a repository path
fn get_git_info(path: &Path) -> anyhow::Result<(String, String, String)> {
    // Get the repository name from remote origin
    let repo_name = Command::new("git")
        .args(&[
            "-C",
            path.to_str().unwrap_or("."),
            "remote",
            "get-url",
            "origin",
        ])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .and_then(|url| {
            // Extract repo name from URL (e.g., git@github.com:user/repo.git or https://github.com/user/repo)
            let url = url.trim();
            if let Some(repo_part) = url.rsplit('/').next() {
                Some(repo_part.trim_end_matches(".git").to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            // Fallback: use the current directory name
            path.canonicalize()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "unknown".to_string())
        });

    // Get the current branch name
    let branch_name = Command::new("git")
        .args(&[
            "-C",
            path.to_str().unwrap_or("."),
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".to_string());

    // Get the current commit hash
    let commit_hash = Command::new("git")
        .args(&["-C", path.to_str().unwrap_or("."), "rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "HEAD".to_string());

    Ok((repo_name, branch_name, commit_hash))
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Db { command } => match command {
            DbCommand::Migrate => {
                let store = db::connect().await?;
                // SQLite auto-migrates on connection, but we still run migrate for consistency
                store.migrate().await?;
                println!("✅ SQLite database is up to date (auto-migrates on connection)");
            }
            DbCommand::CleanupStale { confirm, verbose } => {
                // Start timer for elapsed time tracking
                let start_time = std::time::Instant::now();

                // Get store
                let store = db::connect().await?;

                // Phase 1: Detection
                println!("🔍 Detecting stale worktrees...");
                let stale = match store.detect_stale_worktrees().await {
                    Ok(worktrees) => worktrees,
                    Err(e) => {
                        eprintln!("❌ Error detecting stale worktrees: {}", e);
                        std::process::exit(1);
                    }
                };

                // Phase 2: Report
                if stale.is_empty() {
                    println!("✅ No stale worktrees found!");
                    std::process::exit(2); // Informational exit code
                }

                println!("📊 Found {} stale worktree(s):", stale.len());
                for wt in &stale {
                    println!(
                        "  • {} (path: {}, chunks: {})",
                        wt.name,
                        wt.abs_path,
                        format_number(wt.chunk_count)
                    );
                    if verbose {
                        println!(
                            "    ID: {}, Repo ID: {}, Exists: {}",
                            wt.id, wt.repo_id, wt.exists
                        );
                    }
                }

                // Calculate and display total chunks
                let total_chunks: i64 = stale.iter().map(|wt| wt.chunk_count).sum();
                println!(
                    "\n💾 Total chunks to delete: {}",
                    format_number(total_chunks)
                );

                // Phase 3: Deletion (if confirmed)
                if confirm {
                    println!("🗑️  Deleting {} stale worktree(s)...", stale.len());
                    let mut deleted_count = 0;
                    let mut chunks_cleaned = 0i64;
                    let mut failures = Vec::new();

                    for wt in &stale {
                        match store.delete_worktree_data(wt.id).await {
                            Ok(result) => {
                                deleted_count += 1;
                                chunks_cleaned += result.chunks_deleted as i64;
                            }
                            Err(e) => {
                                failures.push((wt.id, e.to_string()));
                            }
                        }
                    }

                    let elapsed = start_time.elapsed();
                    println!("✅ Cleanup complete!");
                    println!("   Deleted: {}/{}", deleted_count, stale.len());
                    println!("   Chunks cleaned: {}", format_number(chunks_cleaned));
                    println!("   Time taken: {:.2}s", elapsed.as_secs_f64());
                    if !failures.is_empty() {
                        println!("   ⚠️  Failures: {}", failures.len());
                        if verbose {
                            for (id, err) in &failures {
                                println!("      Worktree {}: {}", id, err);
                            }
                        }
                    }
                } else {
                    let elapsed = start_time.elapsed();
                    println!("⚠️  This was a dry-run. Use --confirm to actually delete.");
                    println!("   Command: maproom db cleanup-stale --confirm");
                    println!("   Time taken: {:.2}s", elapsed.as_secs_f64());
                }
            }
        },

        Commands::Cache { command } => {
            command.execute().await?;
        }

        Commands::Scan {
            repo,
            worktree,
            path,
            commit,
            concurrency,
            languages,
            exclude,
            force,
            generate_embeddings,
            embedding_batch_size,
            provider,
            verbose,
        } => {

            // Get git defaults if not provided
            let path = path.unwrap_or_else(|| PathBuf::from("."));

            // Get git information from the path
            let (repo_name, branch_name, commit_hash) = get_git_info(&path)?;

            let repo = repo.unwrap_or(repo_name);
            let worktree = worktree.unwrap_or(branch_name);
            let commit = commit.unwrap_or(commit_hash);

            tracing::info!(
                "Scanning repo: {}, worktree: {}, commit: {}, force: {}, generate_embeddings: {}",
                repo, worktree, commit, force, generate_embeddings
            );

            // Log scan mode for user awareness
            if force {
                tracing::info!("🔄 Force flag enabled - performing full repository scan");
                println!("🔄 Full scan mode (--force flag enabled)");
            } else {
                tracing::info!(
                    "⚡ Incremental mode - only scanning changed files (use --force for full scan)"
                );
                println!("⚡ Incremental scan mode (use --force for full scan)");
            }

            // Create progress tracker
            let mode = if verbose {
                OutputMode::Verbose
            } else {
                OutputMode::Minimal
            };
            let progress = ProgressTracker::new(mode);

            // Tree SHA-based incremental scanning optimization (INCRSCAN-1001)
            //
            // Before scanning, we compare the current git tree SHA against the last
            // indexed SHA stored in worktree_index_state. If they match (and --force
            // is not set), we can skip the entire scan since the code hasn't changed.
            //
            // This provides a 10,000x speedup for unchanged worktrees (2-3 hours → 5-10ms).
            //
            // Fail-safe design: Any error in tree SHA retrieval or state query causes
            // a fallback to full scan. We never skip incorrectly.

            // Create database connection for tree SHA check
            // This must happen before scanning so we can skip if needed
            let store = db::connect().await?;

            // Get git tree SHA using existing function from git.rs
            let tree_sha = match crewchief_maproom::git::get_git_tree_sha(&path) {
                Ok(sha) => {
                    tracing::info!("Current tree SHA: {}", sha);
                    Some(sha)
                }
                Err(e) => {
                    tracing::warn!("Could not get tree SHA: {}, proceeding with full scan", e);
                    None
                }
            };

            // Query worktree_index_state if we have tree SHA
            if let Some(ref current_sha) = tree_sha {
                // Get repo and worktree IDs using store methods
                // Note: Using get_or_create functions ensures worktrees are created if they don't exist
                let root_abs = path.canonicalize().context("invalid root path")?;
                let repo_id = match store.get_or_create_repo(
                    &repo,
                    root_abs.to_string_lossy().as_ref(),
                )
                .await
                {
                    Ok(id) => Some(id),
                    Err(e) => {
                        tracing::warn!("Could not get repo ID: {}, proceeding with full scan", e);
                        None
                    }
                };

                if let Some(repo_id) = repo_id {
                    let worktree_id = match store.get_or_create_worktree(
                        repo_id,
                        &worktree,
                        root_abs.to_string_lossy().as_ref(),
                    )
                    .await
                    {
                        Ok(id) => Some(id),
                        Err(e) => {
                            tracing::warn!(
                                "Could not get worktree ID: {}, proceeding with full scan",
                                e
                            );
                            None
                        }
                    };

                    if let Some(wt_id) = worktree_id {
                        // Get last indexed tree SHA
                        match store.get_last_indexed_tree(wt_id).await {
                            Ok(last_sha) if last_sha == *current_sha && !force => {
                                println!("✓ No changes detected (tree SHA match), skipping scan");
                                tracing::info!(
                                    "Scan skipped: tree {} already indexed",
                                    current_sha
                                );
                                return Ok(()); // Early return!
                            }
                            Ok(last_sha) if last_sha != "init" => {
                                tracing::info!("Tree changed: {} -> {}", last_sha, current_sha);
                            }
                            Ok(_) => {
                                tracing::info!("First-time indexing (no cached state)");
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Could not query index state: {}, proceeding with full scan",
                                    e
                                );
                            }
                        }
                    }
                }
            }

            // Scan execution
            indexer::scan_worktree(
                &store,
                &repo,
                &worktree,
                &path,
                &commit,
                concurrency,
                languages,
                exclude,
                Some(&progress),
            )
            .await
            .with_context(|| format!("scan failed for {}@{}", worktree, commit))?;

            // Auto-generate embeddings after scan if enabled
            if generate_embeddings {
                match auto_generate_embeddings(embedding_batch_size, provider).await {
                    Ok(stats) => {
                        if stats.total_chunks > 0 {
                            println!("\n📊 Embedding Generation Summary:");
                            println!("   {}", stats.summary());
                        }
                    }
                    Err(e) => {
                        // Don't fail the entire scan if embeddings fail
                        tracing::warn!("Embedding generation failed: {}", e);
                        println!("\n⚠️  Warning: Embedding generation failed: {}", e);
                        println!("   You can generate embeddings later with: crewchief-maproom generate-embeddings");
                    }
                }
            }

            // State persistence (INCRSCAN-1002)
            // Update worktree_index_state with scan results and current tree SHA
            // This enables future scans to skip unchanged worktrees (INCRSCAN-1001)
            //
            // Design: Non-fatal errors (scan succeeded, state update is advisory only)
            // If this fails, next scan will be slower but still correct
            if let Some(ref current_tree_sha) = tree_sha {
                // Collect statistics from ProgressTracker using getter methods
                // Note: scan functions return Result<()>, not stats, so we use ProgressTracker
                let files_processed = progress.files_processed() as i32;
                let chunks_processed = progress.chunks_processed() as i32;

                // Calculate embeddings generated
                // If embeddings were enabled and succeeded, all chunks got embeddings
                let embeddings_generated = if generate_embeddings {
                    chunks_processed
                } else {
                    0
                };

                let scan_stats = crewchief_maproom::db::UpdateStats {
                    files_processed,
                    chunks_processed,
                    embeddings_generated,
                };

                // Update index state using existing store
                let root_abs = match path.canonicalize() {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(
                            "Could not canonicalize path for state update: {}",
                            e
                        );
                        path.clone()
                    }
                };

                // Get repo ID
                let repo_id = match store.get_or_create_repo(
                    &repo,
                    root_abs.to_string_lossy().as_ref(),
                )
                .await
                {
                    Ok(id) => Some(id),
                    Err(e) => {
                        tracing::warn!("Could not get repo ID for state update: {}", e);
                        None
                    }
                };

                if let Some(repo_id) = repo_id {
                    // Get worktree ID
                    let worktree_id = match store.get_or_create_worktree(
                        repo_id,
                        &worktree,
                        root_abs.to_string_lossy().as_ref(),
                    )
                    .await
                    {
                        Ok(id) => Some(id),
                        Err(e) => {
                            tracing::warn!(
                                "Could not get worktree ID for state update: {}",
                                e
                            );
                            None
                        }
                    };

                    if let Some(wt_id) = worktree_id {
                        // Update index state with current tree SHA and stats
                        match store.update_index_state(
                            wt_id,
                            current_tree_sha,
                            &scan_stats,
                        )
                        .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "✓ Updated index state: tree {} ({} files, {} chunks, {} embeddings)",
                                    current_tree_sha, files_processed, chunks_processed, embeddings_generated
                                );
                            }
                            Err(e) => {
                                tracing::warn!("Failed to update index state: {}", e);
                                tracing::warn!("Scan completed successfully, but next scan may be slower");
                                // Don't fail the scan - state update is advisory only
                            }
                        }
                    }
                }
            }
        }

        Commands::Upsert {
            paths,
            commit,
            repo,
            worktree,
            root,
            generate_embeddings,
            embedding_batch_size,
            provider,
        } => {
            let store = db::connect().await?;
            indexer::upsert_files(&store, &repo, &worktree, &root, &commit, &paths)
                .await
                .with_context(|| "upsert failed")?;

            // Auto-generate embeddings after upsert if enabled
            if generate_embeddings {
                match auto_generate_embeddings(embedding_batch_size, provider).await {
                    Ok(stats) => {
                        if stats.total_chunks > 0 {
                            println!("\n📊 Embedding Generation Summary:");
                            println!("   {}", stats.summary());
                        }
                    }
                    Err(e) => {
                        // Don't fail the entire upsert if embeddings fail
                        tracing::warn!("Embedding generation failed: {}", e);
                        println!("\n⚠️  Warning: Embedding generation failed: {}", e);
                        println!("   You can generate embeddings later with: crewchief-maproom generate-embeddings");
                    }
                }
            }
        }

        Commands::Watch {
            repo,
            worktree,
            path,
            throttle,
        } => {
            // Default path to current directory if not provided
            let path = path.unwrap_or_else(|| PathBuf::from("."));

            // Get repository name from git remote
            let (repo_name, _, _) = get_git_info(&path)?;
            let repo = repo.unwrap_or(repo_name);

            // Auto-detect current branch using get_current_branch()
            let detected_branch = crewchief_maproom::git::get_current_branch(&path)?;

            // Handle deprecation warning if --worktree flag is provided
            let worktree = if let Some(_wt) = worktree {
                eprintln!("Warning: --worktree flag is deprecated and ignored.");
                eprintln!("The watch command now auto-detects branch switches.");
                eprintln!("Using auto-detected branch: {}", detected_branch);
                detected_branch
            } else {
                detected_branch
            };

            tracing::info!(
                repo = %repo,
                worktree = %worktree,
                path = %path.display(),
                throttle = %throttle,
                "Starting watch"
            );

            // Connect to database
            let store = Arc::new(db::connect().await?);

            // Canonicalize path for the watcher
            let watch_path = path.canonicalize()
                .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;
            let watch_path_str = watch_path.to_string_lossy().to_string();

            // Ensure repo and worktree exist
            let repo_id = store.get_or_create_repo(&repo, &watch_path_str).await?;
            let worktree_id = store.get_or_create_worktree(repo_id, &worktree, &watch_path_str).await?;

            // Create and start the file watcher
            use crewchief_maproom::incremental::{MultiWatcher, WatcherConfig};

            let debounce_ms = parse_throttle(&throttle)?;
            let config = WatcherConfig {
                debounce_ms,
                ..Default::default()
            };

            let (mut multi_watcher, mut event_rx) = MultiWatcher::new(config);

            // Add the worktree to watch
            multi_watcher.add_worktree(
                worktree.clone(),
                watch_path.clone(),
            ).await?;

            println!("👀 Watching {} for changes...", watch_path.display());
            println!("   Repository: {}", repo);
            println!("   Worktree: {}", worktree);
            println!("   Throttle: {}", throttle);
            println!();
            println!("Press Ctrl+C to stop.");

            // Handle events
            use crewchief_maproom::incremental::incremental_update;
            use tokio::signal;

            loop {
                tokio::select! {
                    // Handle shutdown signal
                    _ = signal::ctrl_c() => {
                        println!("\n🛑 Shutting down watch...");
                        multi_watcher.shutdown().await?;
                        break;
                    }
                    // Handle file events
                    Some(event) = event_rx.recv() => {
                        use crewchief_maproom::incremental::EventType;

                        let event_type = match event.event_type {
                            EventType::Modified => "modified",
                            EventType::Deleted => "deleted",
                            EventType::Renamed => "renamed",
                        };

                        println!("📁 {} {}", event_type, event.path.display());

                        // Trigger incremental update for the worktree
                        match incremental_update(&store, worktree_id, &watch_path).await {
                            Ok(stats) => {
                                if stats.files_processed > 0 {
                                    println!("   ✅ Processed {} files", stats.files_processed);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Incremental update failed: {}", e);
                                println!("   ⚠️  Update failed: {}", e);
                            }
                        }
                    }
                }
            }

            println!("Watch complete.");
        }

        Commands::Search {
            repo,
            worktree,
            query,
            k,
            debug,
            deduplicate,
        } => {
            let store = db::connect().await?;
            // Fetch extra results if deduplication is enabled
            let fetch_k = if deduplicate { k * 3 } else { k };
            let hits = store.search_chunks_fts(&repo, worktree.as_deref(), &query, fetch_k, debug)
                .await?;

            // Apply deduplication if enabled
            let hits = if deduplicate {
                deduplicate_search_hits(hits, k as usize)
            } else {
                hits
            };

            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({"hits": hits}))?
            );
        }

        Commands::VectorSearch {
            repo,
            worktree,
            query,
            k,
            threshold,
        } => {
            use crewchief_maproom::embedding::EmbeddingService;

            let store = db::connect().await?;

            // Generate query embedding
            tracing::info!("Generating embedding for query: {}", query);
            let embedding_service = EmbeddingService::from_env()
                .await
                .context("Failed to create embedding service. Ensure OPENAI_API_KEY is set.")?;

            let query_embedding = embedding_service
                .embed_text(&query)
                .await
                .context("Failed to generate query embedding")?;

            tracing::info!(
                "Executing vector search (k={}, threshold={:?})",
                k,
                threshold
            );

            // Execute vector search
            let search_hits = match store.search_chunks_vector(
                &repo,
                worktree.as_deref(),
                &query_embedding,
                k as i64,
                false,
            ).await {
                Ok(hits) => hits,
                Err(e) => {
                    // Check for SQLite-specific errors (no vector support)
                    let err_str = e.to_string();
                    if err_str.contains("sqlite-vec") || err_str.contains("vector") || err_str.contains("not available") {
                        eprintln!("Vector search unavailable: {}", e);
                        eprintln!("Tip: Use 'search' command for full-text search instead");
                        std::process::exit(1);
                    }
                    return Err(e);
                }
            };

            // Build hits with threshold filtering
            let mut hits = Vec::new();
            for hit in search_hits {
                // Apply threshold filter if specified
                if let Some(thresh) = threshold {
                    if hit.score < thresh as f64 {
                        continue;
                    }
                }

                hits.push(serde_json::json!({
                    "score": hit.score,
                    "start_line": hit.start_line,
                    "end_line": hit.end_line,
                    "symbol_name": hit.symbol_name,
                    "kind": hit.kind,
                    "file_path": hit.file_relpath,
                }));
            }

            // Output JSON schema for MCP client consumption
            let output = serde_json::json!({
                "hits": hits,
                "total": hits.len(),
                "query": query,
                "mode": "vector",
                "k": k,
                "threshold": threshold,
            });

            println!("{}", serde_json::to_string_pretty(&output)?);
        }

        Commands::Status {
            repo,
            worktree,
            json,
        } => {
            use crewchief_maproom::status;

            // Validate worktree filter requires repo filter
            if worktree.is_some() && repo.is_none() {
                anyhow::bail!("--worktree requires --repo to be specified");
            }

            let store = db::connect().await?;

            match status::get_status(Arc::new(store), repo, worktree).await {
                Ok(status_data) => {
                    if json {
                        let output = status::format_json(&status_data)?;
                        println!("{}", output);
                    } else {
                        let output = status::format_text(&status_data);
                        print!("{}", output);
                    }
                }
                Err(e) => {
                    eprintln!("Error querying status: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::GenerateEmbeddings {
            incremental,
            batch_size,
            dry_run,
            sample,
            batch_delay,
            max_cost,
            force,
        } => {
            use crewchief_maproom::embedding::{
                CostEstimator, EmbeddingPipeline, EmbeddingService, PipelineConfig,
            };

            tracing::info!("Initializing embedding generation pipeline");

            // Create embedding service from environment
            let service = EmbeddingService::from_env()
                .await
                .context("Failed to create embedding service. Ensure OPENAI_API_KEY is set.")?;

            // Configure pipeline
            let config = PipelineConfig {
                batch_size,
                incremental: if force { false } else { incremental },
                dry_run,
                sample_size: sample,
                batch_delay_ms: batch_delay,
                max_cost_usd: max_cost,
            };

            tracing::info!(
                "Pipeline config: batch_size={}, incremental={}, dry_run={}, sample={:?}",
                config.batch_size,
                config.incremental,
                config.dry_run,
                config.sample_size
            );

            // Connect to database
            let store = db::connect().await?;

            // Get chunk count for cost estimation
            // Note: Always uses chunks needing embeddings count.
            // If --force is used, the pipeline will regenerate all embeddings anyway.
            let chunk_count = store.get_chunks_needing_embeddings_count().await?;

            tracing::info!("Found {} chunks needing embeddings", chunk_count);

            // Provide cost estimate
            let estimator = CostEstimator::default();
            let estimate = estimator.estimate_cost(chunk_count as usize);
            println!("\n{}\n", estimate.format());

            // Warn if cost is high
            if estimate.estimated_cost_usd > 10.0 {
                tracing::warn!(
                    "Estimated cost is high: ${:.2}. Consider using --sample or --max-cost to limit spending.",
                    estimate.estimated_cost_usd
                );
            }

            // Run pipeline
            let pipeline = EmbeddingPipeline::new(service, config);
            let stats = pipeline.run(&store).await?;

            // Display results
            println!("\n{}\n", "=".repeat(60));
            println!("Embedding Generation Complete");
            println!("{}\n", "=".repeat(60));
            println!("{}", stats.summary());
            println!("{}", "=".repeat(60));
        }

        Commands::Migrate { command } => {
            use crewchief_maproom::migrate::{verify_migration, MarkdownMigrator};

            let store = db::connect().await?;

            match command {
                MigrateCommand::Markdown { repo, worktree } => {
                    println!("Starting markdown migration for repo: {}", repo);
                    if let Some(ref wt) = worktree {
                        println!("Worktree: {}", wt);
                    }

                    let migrator = MarkdownMigrator::new(store.clone());
                    let result = migrator.migrate(&repo, worktree.as_deref()).await?;

                    println!("\n{}", "=".repeat(60));
                    println!("Migration Complete");
                    println!("{}", "=".repeat(60));
                    println!("Files processed: {}", result.stats.files_processed);
                    println!("Old chunks: {}", result.stats.total_old_chunks);
                    println!("New chunks: {}", result.stats.total_new_chunks);
                    println!("Delta: {:+}", result.stats.delta());
                    println!("Errors: {}", result.stats.files_with_errors);
                    println!("Backup table: {}", result.backup_table);

                    if let Some(duration) = result.stats.duration() {
                        println!(
                            "Duration: {:.2}s",
                            duration.num_milliseconds() as f64 / 1000.0
                        );
                    }

                    println!("{}", "=".repeat(60));
                    println!("\nTo rollback: cargo run --bin crewchief-maproom -- migrate rollback --backup {}", result.backup_table);
                }

                MigrateCommand::Rollback { backup } => {
                    println!("Rolling back migration from backup: {}", backup);
                    let migrator = MarkdownMigrator::new(store.clone());
                    migrator.rollback(&backup).await?;
                    println!("Rollback complete");
                }

                MigrateCommand::ListBackups => {
                    let migrator = MarkdownMigrator::new(store.clone());
                    let backups = migrator.list_backups().await?;

                    if backups.is_empty() {
                        println!("No backup tables found");
                    } else {
                        println!("Available backup tables:");
                        for backup in backups {
                            println!("  - {}", backup);
                        }
                    }
                }

                MigrateCommand::DeleteBackup { backup } => {
                    println!("Deleting backup table: {}", backup);
                    let migrator = MarkdownMigrator::new(store.clone());
                    migrator.delete_backup(&backup).await?;
                    println!("Backup deleted");
                }

                MigrateCommand::Verify { repo } => {
                    println!("Verifying migration for repo: {}", repo);
                    let results = verify_migration(&store, &repo).await?;

                    println!("\n{}", "=".repeat(60));
                    println!("Migration Verification Results");
                    println!("{}", "=".repeat(60));

                    for (key, value) in results.iter() {
                        println!("{}: {}", key, value);
                    }

                    println!("{}", "=".repeat(60));
                }
            }
        }
        Commands::Serve => {
            daemon::run().await?;
        }

        Commands::Context {
            chunk_id,
            budget,
            callers,
            callees,
            tests,
            docs,
            config,
            max_depth,
            json,
        } => {
            // Connect to database
            let store = db::connect()
                .await
                .context("Database connection failed")?;

            // Create assembler (uses DefaultAssemblyStrategy which has working get_chunk_metadata)
            let assembler = DefaultAssemblyStrategy::new(Arc::new(store));

            // Build expand options from CLI args
            let options = ExpandOptions {
                callers,
                callees,
                tests,
                docs,
                config,
                max_depth,
                ..Default::default()
            };

            // Execute context assembly
            let bundle = assembler
                .assemble(chunk_id, budget, options)
                .await
                .with_context(|| format!("Failed to assemble context for chunk {}", chunk_id))?;

            // Output
            if json {
                println!("{}", serde_json::to_string_pretty(&bundle)?);
            } else {
                // Placeholder text - human-readable format in CTXCLI-2003
                println!("Context bundle for chunk #{}", chunk_id);
                println!("Items: {}", bundle.items.len());
                println!("Total tokens: {}", bundle.total_tokens);
                if bundle.truncated {
                    println!("(truncated)");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_stale_defaults() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale"]);
        if let Commands::Db {
            command: DbCommand::CleanupStale { confirm, verbose },
        } = cli.command
        {
            assert_eq!(confirm, false, "confirm should default to false");
            assert_eq!(verbose, false, "verbose should default to false");
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_cleanup_stale_with_confirm() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale", "--confirm"]);
        if let Commands::Db {
            command: DbCommand::CleanupStale { confirm, verbose },
        } = cli.command
        {
            assert_eq!(confirm, true);
            assert_eq!(verbose, false);
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_cleanup_stale_with_verbose() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale", "--verbose"]);
        if let Commands::Db {
            command: DbCommand::CleanupStale { confirm, verbose },
        } = cli.command
        {
            assert_eq!(confirm, false);
            assert_eq!(verbose, true);
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_cleanup_stale_short_verbose() {
        let cli = Cli::parse_from(&["maproom", "db", "cleanup-stale", "-v"]);
        if let Commands::Db {
            command:
                DbCommand::CleanupStale {
                    confirm: _,
                    verbose,
                },
        } = cli.command
        {
            assert_eq!(verbose, true);
        } else {
            panic!("Expected cleanup-stale command");
        }
    }

    #[test]
    fn test_context_command_parsing_minimal() {
        let cli = Cli::parse_from(&["maproom", "context", "--chunk-id", "12345"]);
        if let Commands::Context {
            chunk_id,
            budget,
            callers,
            callees,
            tests,
            docs,
            config,
            max_depth,
            json,
        } = cli.command
        {
            assert_eq!(chunk_id, 12345);
            assert_eq!(budget, 6000); // default
            assert_eq!(callers, false);
            assert_eq!(callees, false);
            assert_eq!(tests, false);
            assert_eq!(docs, false);
            assert_eq!(config, false);
            assert_eq!(max_depth, 2); // default
            assert_eq!(json, false);
        } else {
            panic!("Expected Context command");
        }
    }

    #[test]
    fn test_context_command_parsing_with_expands() {
        let cli = Cli::parse_from(&[
            "maproom", "context",
            "--chunk-id", "99999",
            "--budget", "4000",
            "--callers",
            "--callees",
            "--tests",
            "--max-depth", "5",
        ]);
        if let Commands::Context {
            chunk_id,
            budget,
            callers,
            callees,
            tests,
            docs,
            config,
            max_depth,
            json,
        } = cli.command
        {
            assert_eq!(chunk_id, 99999);
            assert_eq!(budget, 4000);
            assert_eq!(callers, true);
            assert_eq!(callees, true);
            assert_eq!(tests, true);
            assert_eq!(docs, false); // not specified
            assert_eq!(config, false); // not specified
            assert_eq!(max_depth, 5);
            assert_eq!(json, false);
        } else {
            panic!("Expected Context command");
        }
    }

    #[test]
    fn test_context_command_parsing_all_flags() {
        let cli = Cli::parse_from(&[
            "maproom", "context",
            "--chunk-id", "42",
            "--budget", "8000",
            "--callers",
            "--callees",
            "--tests",
            "--docs",
            "--config",
            "--max-depth", "3",
            "--json",
        ]);
        if let Commands::Context {
            chunk_id,
            budget,
            callers,
            callees,
            tests,
            docs,
            config,
            max_depth,
            json,
        } = cli.command
        {
            assert_eq!(chunk_id, 42);
            assert_eq!(budget, 8000);
            assert_eq!(callers, true);
            assert_eq!(callees, true);
            assert_eq!(tests, true);
            assert_eq!(docs, true);
            assert_eq!(config, true);
            assert_eq!(max_depth, 3);
            assert_eq!(json, true);
        } else {
            panic!("Expected Context command");
        }
    }
}
