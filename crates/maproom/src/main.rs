use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use tokio::sync::oneshot;
use tracing_subscriber::{fmt, EnvFilter};

use crewchief_maproom::progress::{OutputMode, ProgressTracker};
use crewchief_maproom::{db, indexer};

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
        /// Enable parallel batch processing for improved performance (PERF_OPT-3001)
        #[arg(long, default_value_t = false)]
        parallel: bool,
        /// Number of parallel database workers (only with --parallel)
        #[arg(long, default_value_t = 4)]
        parallel_workers: usize,
        /// Batch size for database inserts (only with --parallel)
        #[arg(long, default_value_t = 50)]
        batch_size: usize,
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
    Watch {
        /// Repository name (defaults to git remote origin name)
        #[arg(long)]
        repo: Option<String>,
        /// Worktree name (defaults to current branch name)
        #[arg(long)]
        worktree: Option<String>,
        /// Path to watch (defaults to current directory)
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long, default_value = "2s")]
        throttle: String,
    },

    /// Watch for branch switches and auto-index
    BranchWatch {
        /// Path to git repository (defaults to current directory)
        #[arg(long)]
        repo: Option<PathBuf>,

        /// Show verbose logging
        #[arg(short, long)]
        verbose: bool,
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
    let client = crewchief_maproom::db::connect().await?;

    // Count chunks needing embeddings
    let count_row = client
        .query_one(
            "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NULL OR text_embedding IS NULL",
            &[],
        )
        .await?;
    let chunk_count: i64 = count_row.get(0);

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
            &client,
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

/// Branch watch command handler
///
/// Starts the BranchWatcher for automatic indexing on branch switches.
async fn branch_watch_command(repo: Option<PathBuf>, verbose: bool) -> anyhow::Result<()> {
    use crewchief_maproom::watcher::BranchWatcher;

    // Set up logging based on verbose flag
    if verbose {
        std::env::set_var("RUST_LOG", "crewchief_maproom=debug");
    } else if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "crewchief_maproom=info");
    }

    // Default to current directory if no repo path provided
    let repo_path = repo.unwrap_or_else(|| PathBuf::from("."));

    // Validate repository path exists
    if !repo_path.exists() {
        anyhow::bail!("Repository path does not exist: {}", repo_path.display());
    }

    // Validate it's a git repository
    let git_head = repo_path.join(".git/HEAD");
    if !git_head.exists() {
        anyhow::bail!(
            "Not a git repository: {} (expected .git/HEAD)",
            repo_path.display()
        );
    }

    tracing::info!("Starting branch watcher for {}", repo_path.display());

    // Connect to database
    let client = db::connect()
        .await
        .context("Failed to connect to database")?;

    tracing::info!("Connected to database");

    // Create and start watcher
    let mut watcher = BranchWatcher::new(repo_path, client)?;

    tracing::info!("Watching for branch switches (Ctrl+C to stop)");

    // Create shutdown channel
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Wrap sender in Mutex to allow FnMut closure to take ownership
    let shutdown_tx = std::sync::Mutex::new(Some(shutdown_tx));

    // Setup Ctrl+C handler
    ctrlc::set_handler(move || {
        tracing::info!("Shutting down...");
        if let Ok(mut tx) = shutdown_tx.lock() {
            if let Some(tx) = tx.take() {
                let _ = tx.send(());
            }
        }
    })?;

    // Run watcher with shutdown signal
    let result = watcher.start(Some(shutdown_rx)).await;

    match result {
        Ok(_) => {
            tracing::info!("Watcher stopped normally");
        }
        Err(e) => {
            tracing::error!("Watcher error: {}", e);
            return Err(e);
        }
    }

    tracing::info!("Branch watcher stopped");
    Ok(())
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
                let client = db::connect().await?;
                db::migrate(&client).await?;
                tracing::info!("migrations applied");
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
            parallel,
            parallel_workers,
            batch_size,
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
                "Scanning repo: {}, worktree: {}, commit: {}, parallel: {}, force: {}, generate_embeddings: {}",
                repo, worktree, commit, parallel, force, generate_embeddings
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

            if parallel {
                // Use parallel batch processing pipeline (PERF_OPT-3001)
                use crewchief_maproom::indexer::parallel::ParallelConfig;

                let pool = db::create_pool().await?;
                let config = ParallelConfig {
                    batch_size,
                    parallel_workers,
                    max_file_size: 10 * 1024 * 1024, // 10MB
                    file_queue_capacity: 1000,
                    chunk_queue_capacity: 10000,
                };

                indexer::scan_worktree_parallel(
                    &pool,
                    &repo,
                    &worktree,
                    &path,
                    &commit,
                    languages,
                    exclude,
                    config,
                    Some(&progress),
                )
                .await
                .with_context(|| format!("parallel scan failed for {}@{}", worktree, commit))?;
            } else {
                // Use sequential single-client processing
                let client = db::connect().await?;
                indexer::scan_worktree(
                    &client,
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
            }

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
            let client = db::connect().await?;
            indexer::upsert_files(&client, &repo, &worktree, &root, &commit, &paths)
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

            // Derive repo/worktree defaults from git if not provided
            let (repo_name, branch_name, _commit_hash) = get_git_info(&path)?;
            let repo = repo.unwrap_or(repo_name);
            let worktree = worktree.unwrap_or(branch_name);

            tracing::info!(
                repo = %repo,
                worktree = %worktree,
                path = %path.display(),
                throttle = %throttle,
                "Starting watch"
            );

            let client = db::connect().await?;
            indexer::watch_worktree(&client, &repo, &worktree, &path, &throttle).await?;
        }

        Commands::BranchWatch { repo, verbose } => {
            branch_watch_command(repo, verbose).await?;
        }

        Commands::Search {
            repo,
            worktree,
            query,
            k,
        } => {
            let client = db::connect().await?;
            let hits =
                db::search_chunks_fts(&client, &repo, worktree.as_deref(), &query, k).await?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({"hits": hits}))?
            );
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
            let client = db::connect().await?;

            // Get chunk count for cost estimation
            let count_query = if config.incremental {
                "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NULL OR text_embedding IS NULL"
            } else {
                "SELECT COUNT(*) FROM maproom.chunks"
            };

            let count_row = client.query_one(count_query, &[]).await?;
            let chunk_count: i64 = count_row.get(0);

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
            let stats = pipeline.run(&client).await?;

            // Display results
            println!("\n{}\n", "=".repeat(60));
            println!("Embedding Generation Complete");
            println!("{}\n", "=".repeat(60));
            println!("{}", stats.summary());
            println!("{}", "=".repeat(60));
        }

        Commands::Migrate { command } => {
            use crewchief_maproom::migrate::{verify_migration, MarkdownMigrator};

            let client = db::connect().await?;

            match command {
                MigrateCommand::Markdown { repo, worktree } => {
                    println!("Starting markdown migration for repo: {}", repo);
                    if let Some(ref wt) = worktree {
                        println!("Worktree: {}", wt);
                    }

                    let mut migrator = MarkdownMigrator::new(client);
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
                    let mut migrator = MarkdownMigrator::new(client);
                    migrator.rollback(&backup).await?;
                    println!("Rollback complete");
                }

                MigrateCommand::ListBackups => {
                    let migrator = MarkdownMigrator::new(client);
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
                    let mut migrator = MarkdownMigrator::new(client);
                    migrator.delete_backup(&backup).await?;
                    println!("Backup deleted");
                }

                MigrateCommand::Verify { repo } => {
                    println!("Verifying migration for repo: {}", repo);
                    let results = verify_migration(&client, &repo).await?;

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
    }

    Ok(())
}
