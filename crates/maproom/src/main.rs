use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use tracing_subscriber::{fmt, EnvFilter};

use crewchief_maproom::cli::format::{
    format_hits_agent, format_hits_json_search, format_hits_json_vector, OutputFormat,
};
use crewchief_maproom::context::{
    AssemblyStrategy, ContextBundle, DefaultAssemblyStrategy, ExpandOptions,
};
use crewchief_maproom::db::StoreCore;
use crewchief_maproom::db::StoreSearch;
use crewchief_maproom::progress::{OutputMode, ProgressTracker};
use crewchief_maproom::{daemon, db, indexer};

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
        let key = (
            hit.file_relpath.clone(),
            hit.symbol_name.clone(),
            hit.start_line,
        );
        groups.entry(key).or_default().push(hit);
    }

    // Select highest-scoring from each group
    let mut deduped: Vec<db::SearchHit> = groups
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

    // Re-sort by score descending
    deduped.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

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

/// Get 8-character short commit SHA for detached HEAD state.
///
/// Used when the branch name is "HEAD" (detached state) to identify the worktree.
fn get_short_commit_sha(path: &Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .current_dir(path)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git rev-parse: {}. Is git installed?", e))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "git rev-parse failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Handle a branch switch detected by the HEAD watcher.
///
/// NEW IMPLEMENTATION for SQLite (original PostgreSQL version removed in IDXABS-2001).
/// Debounces rapid switches, updates dynamic state, triggers re-indexing, and emits NDJSON.
async fn handle_branch_switch(
    watch_path: &Path,
    store: &db::SqliteStore,
    repo: &str,
    repo_id: i64,
    current_branch: &Arc<RwLock<String>>,
    worktree_id: &Arc<RwLock<i64>>,
    debouncer: &indexer::DebouncedHandler,
) -> anyhow::Result<()> {
    use crewchief_maproom::git::get_current_branch;
    use crewchief_maproom::incremental::incremental_update;
    use crewchief_maproom::indexer::BranchSwitchEvent;

    // 0. Check debounce (skip if rapid switch)
    if !debouncer.should_handle() {
        tracing::debug!("Debouncing rapid branch switch");
        return Ok(());
    }

    // 1. Detect new branch
    let new_branch = match get_current_branch(watch_path) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to get current branch: {}", e);
            return Ok(()); // Continue watching, retry on next event
        }
    };

    // 2. Handle detached HEAD (use 8-char SHA as branch name)
    let effective_branch = if new_branch == "HEAD" {
        match get_short_commit_sha(watch_path) {
            Ok(sha) => sha,
            Err(e) => {
                tracing::warn!("Failed to get commit SHA for detached HEAD: {}", e);
                return Ok(()); // Continue watching
            }
        }
    } else {
        new_branch
    };

    // 3. Check if actually changed
    let old_branch = current_branch.read().unwrap().clone();
    let old_wt_id = *worktree_id.read().unwrap();
    if old_branch == effective_branch {
        tracing::debug!("Same branch '{}', skipping", effective_branch);
        return Ok(()); // Same branch, skip
    }

    tracing::info!("Branch switch: '{}' -> '{}'", old_branch, effective_branch);

    // 4. Get/create worktree record
    let watch_path_str = watch_path.to_string_lossy().to_string();
    let new_wt_id = match store
        .get_or_create_worktree(repo_id, &effective_branch, &watch_path_str)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(
                "Failed to get/create worktree: {}. Continuing with old worktree_id.",
                e
            );
            return Ok(()); // Continue with old worktree_id
        }
    };
    let worktree_created = new_wt_id != old_wt_id;

    // 5. Update state (brief lock hold)
    {
        *current_branch.write().unwrap() = effective_branch.clone();
        *worktree_id.write().unwrap() = new_wt_id;
    }

    // 6. Re-index (log errors, don't crash)
    if let Err(e) = incremental_update(store, new_wt_id, watch_path).await {
        tracing::warn!("Incremental update after branch switch failed: {}", e);
    }

    // 7. Emit NDJSON event to stdout (for VSCode extension)
    let event = BranchSwitchEvent {
        event_type: "branch_switched",
        timestamp: chrono::Utc::now().to_rfc3339(),
        repo: repo.to_string(),
        old_branch,
        new_branch: effective_branch,
        old_worktree_id: old_wt_id,
        new_worktree_id: new_wt_id,
        worktree_created,
    };
    if let Ok(json) = serde_json::to_string(&event) {
        println!("{}", json);
    }

    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    name = "crewchief-maproom",
    version,
    about = "Maproom indexer & CLI",
    after_help = include_str!("../docs/cli-help-after.md")
)]
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
        /// Generate embeddings for vector search (default: true).
        /// Embeddings enable semantic search but require embedding provider configuration.
        /// Use --generate-embeddings=false (or --no-generate-embeddings) to skip if:
        /// - Embedding provider is not configured
        /// - Only using full-text search
        /// - Troubleshooting configuration issues
        #[arg(
            long,
            default_value_t = true,
            action = clap::ArgAction::Set,
            help = "Generate embeddings for vector search (default: true)",
            long_help = "Generate embeddings for vector search.\n\
                         Embeddings enable semantic search via vector-search command.\n\
                         Full-text search works without embeddings.\n\n\
                         Skip embeddings with --generate-embeddings=false or --no-generate-embeddings when:\n\
                         - Embedding provider is not configured\n\
                         - Only using full-text search\n\
                         - Troubleshooting configuration issues"
        )]
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
        /// Output progress as JSON events (for programmatic consumption by VSCode extension)
        #[arg(long)]
        json: bool,
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
        /// Output as JSON events only (suppress human-readable messages)
        #[arg(long, default_value_t = false)]
        json: bool,
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
        /// Filter by chunk kind (comma-separated: func,class,method,heading_2). Case-sensitive.
        #[arg(long, value_delimiter = ',')]
        kind: Option<Vec<String>>,
        /// Filter by file language (comma-separated: py,ts,rs). Case-sensitive. Use file extensions.
        #[arg(long, value_delimiter = ',')]
        lang: Option<Vec<String>>,
        /// Include content preview in search results
        #[arg(long, default_value_t = false)]
        preview: bool,
        /// Maximum length of preview text in characters (default: 200, or 120 for agent format)
        #[arg(long)]
        preview_length: Option<usize>,
        /// Output format: json (default, backward compatible) or agent (compact, LLM-optimized)
        ///
        /// Available formats:
        /// - json: Full JSON output (default, backward compatible)
        /// - agent: Compact one-line-per-result for LLM agents
        ///
        /// The agent format implicitly enables preview with 120-char default.
        /// Use --preview-length to customize the preview length.
        ///
        /// Examples:
        ///   # Agent format with default preview (120 chars)
        ///   maproom search --repo X --query Y --format agent
        ///
        ///   # Agent format with custom preview length
        ///   maproom search --repo X --query Y --format agent --preview-length 50
        ///
        ///   # JSON format (explicit, same as default)
        ///   maproom search --repo X --query Y --format json
        ///
        ///   # Agent format filtered by kind
        ///   maproom search --repo X --query Y --format agent --kind func
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
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

        /// Filter by chunk kind (comma-separated: func,class,method,heading_2). Case-sensitive.
        #[arg(long, value_delimiter = ',')]
        kind: Option<Vec<String>>,

        /// Filter by file language (comma-separated: py,ts,rs). Case-sensitive. Use file extensions.
        #[arg(long, value_delimiter = ',')]
        lang: Option<Vec<String>>,

        /// Include content preview in search results
        #[arg(long, default_value_t = false)]
        preview: bool,

        /// Maximum length of preview text in characters (default: 200, or 120 for agent format)
        #[arg(long)]
        preview_length: Option<usize>,

        /// Output format: json (default, backward compatible) or agent (compact, LLM-optimized)
        ///
        /// Available formats:
        /// - json: Full JSON output (default, backward compatible)
        /// - agent: Compact one-line-per-result for LLM agents
        ///
        /// The agent format implicitly enables preview with 120-char default.
        /// Use --preview-length to customize the preview length.
        ///
        /// Examples:
        ///   # Agent format with default preview (120 chars)
        ///   maproom vector-search --repo X --query Y --format agent
        ///
        ///   # Agent format with custom preview length
        ///   maproom vector-search --repo X --query Y --format agent --preview-length 50
        ///
        ///   # JSON format (explicit, same as default)
        ///   maproom vector-search --repo X --query Y --format json
        ///
        ///   # Agent format filtered by kind
        ///   maproom vector-search --repo X --query Y --format agent --kind func
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
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
        /// Show all languages instead of top 5
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },

    /// Show encoding (embedding generation) progress
    ///
    /// Displays chunk/embedding counts, percentage complete, and active encoding run info.
    ///
    /// Examples:
    ///   maproom encoding-progress                  # Global progress
    ///   maproom encoding-progress --repo myrepo    # Repo-specific progress
    ///   maproom encoding-progress --json           # JSON output
    EncodingProgress {
        /// Filter by repository name
        #[arg(long)]
        repo: Option<String>,
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

    /// Start the Maproom daemon (JSON-RPC over Stdio or Unix socket)
    Serve {
        /// Use Unix socket mode instead of stdio (experimental)
        #[arg(long)]
        socket: bool,

        /// Socket path (default: /tmp/maproom-{uid}.sock)
        #[arg(long)]
        socket_path: Option<PathBuf>,

        /// Idle timeout in seconds (default: 300 = 5 minutes)
        #[arg(long, default_value_t = 300)]
        idle_timeout: u64,
    },

    /// Delete indexed chunks matching patterns in .maproomignore
    ///
    /// Removes chunks from the database where their file path matches any pattern
    /// in the .maproomignore file at the repository root. This is useful for cleaning
    /// up stale entries after adding new ignore patterns.
    ///
    /// Examples:
    ///   maproom clean-ignored --repo myproject --worktree main
    ///   maproom clean-ignored --repo myproject --worktree main --dry-run
    CleanIgnored {
        /// Repository name
        #[arg(long, required = true)]
        repo: String,

        /// Worktree name
        #[arg(long, required = true)]
        worktree: String,

        /// Dry run - show what would be deleted without deleting
        #[arg(long, default_value_t = false)]
        dry_run: bool,
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
/// * `json_mode` - If true, suppress human-readable output
async fn auto_generate_embeddings(
    batch_size: usize,
    provider: Option<String>,
    json_mode: bool,
) -> anyhow::Result<crewchief_maproom::embedding::PipelineStats> {
    use crewchief_maproom::embedding::{EmbeddingPipeline, EmbeddingService, PipelineConfig};

    tracing::info!("Starting auto-embedding generation");
    if !json_mode {
        println!("\n🔄 Generating embeddings for new chunks...");
    }

    // Set provider in environment if specified via CLI (overrides env var)
    if let Some(ref provider_name) = provider {
        tracing::info!("Using provider from CLI flag: {}", provider_name);
        std::env::set_var("MAPROOM_EMBEDDING_PROVIDER", provider_name);
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
        if !json_mode {
            println!("   ✓ All chunks already have embeddings");
        }
        return Ok(crewchief_maproom::embedding::PipelineStats::default());
    }

    if !json_mode {
        println!("   Found {} chunks needing embeddings", chunk_count);
    }

    // Create progress tracker with appropriate output mode
    let output_mode = if json_mode {
        crewchief_maproom::progress::OutputMode::Json
    } else {
        crewchief_maproom::progress::OutputMode::Minimal
    };
    let progress = crewchief_maproom::progress::ProgressTracker::new(output_mode);
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
        ms_str
            .parse::<u64>()
            .with_context(|| format!("Invalid throttle value: {}", throttle))
    } else if let Some(s_str) = throttle.strip_suffix("s") {
        let secs: u64 = s_str
            .parse()
            .with_context(|| format!("Invalid throttle value: {}", throttle))?;
        Ok(secs * 1000)
    } else {
        // Default to treating as seconds if no suffix
        let secs: u64 = throttle.parse().with_context(|| {
            format!(
                "Invalid throttle value: {}. Use format like '2s' or '500ms'",
                throttle
            )
        })?;
        Ok(secs * 1000)
    }
}

/// Map context item role to an emoji for human-readable output.
fn role_emoji(role: &str) -> &'static str {
    match role.to_lowercase().as_str() {
        "primary" => "📄",
        "caller" => "🔗",
        "callee" => "📤",
        "test" => "🧪",
        "doc" => "📚",
        "config" => "⚙️",
        "hook" => "🪝",
        "jsx_parent" => "⬆️",
        "jsx_child" => "⬇️",
        _ => "📎",
    }
}

/// Format a context bundle for human-readable CLI output.
///
/// Displays a header with budget/usage information followed by each context item
/// with its role, file path, line range, reason (for non-primary items), and token count.
fn format_context_bundle(bundle: &ContextBundle, chunk_id: i64, budget: usize) -> String {
    let mut output = String::new();

    // Header
    let _ = writeln!(output, "📦 Context Bundle for chunk #{}", chunk_id);
    let _ = writeln!(
        output,
        "   Budget: {} tokens | Used: {} tokens | Truncated: {}",
        budget,
        bundle.total_tokens,
        if bundle.truncated { "Yes" } else { "No" }
    );
    let _ = writeln!(output);

    // Items grouped by display
    for item in &bundle.items {
        let emoji = role_emoji(&item.role);
        let role_upper = item.role.to_uppercase();

        // File path with line range
        let _ = writeln!(
            output,
            "{} {}: {}:{}-{}",
            emoji, role_upper, item.relpath, item.range.start, item.range.end
        );

        // For primary chunks, show a content preview
        if item.role.to_lowercase() == "primary" {
            let _ = writeln!(output, "   ─────────────────────────────────────────");
            // Show first few lines of content (max 10 lines, max 80 chars per line)
            for (i, line) in item.content.lines().take(10).enumerate() {
                let truncated_line = if line.len() > 80 {
                    format!("{}...", &line[..77])
                } else {
                    line.to_string()
                };
                let _ = writeln!(output, "   {}", truncated_line);
                if i >= 9 {
                    let _ = writeln!(output, "   ... (content truncated)");
                    break;
                }
            }
            let _ = writeln!(output, "   ─────────────────────────────────────────");
        } else {
            // For non-primary items, show the reason
            if !item.reason.is_empty() {
                let _ = writeln!(output, "   Reason: {}", item.reason);
            }
        }

        // Token count for every item
        let _ = writeln!(output, "   Tokens: {}", item.tokens);
        let _ = writeln!(output);
    }

    // Handle empty bundle
    if bundle.items.is_empty() {
        let _ = writeln!(output, "   (No context items found)");
    }

    output
}

/// Extract git information from a repository path
fn get_git_info(path: &Path) -> anyhow::Result<(String, String, String)> {
    // Get the repository name from remote origin in owner/repo format
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
            // Extract owner/repo from URL
            // Handles both HTTPS (https://github.com/owner/repo.git) and SSH (git@github.com:owner/repo.git)
            let url = url.trim();
            // Match owner/repo pattern at the end of the URL
            let re = regex::Regex::new(r"[:/]([^/]+/[^/]+?)(?:\.git)?$").ok()?;
            re.captures(url).map(|cap| cap[1].to_string())
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
                // connect() auto-runs migrations, so this command just ensures
                // the database exists and is fully migrated
                let _store = db::connect().await?;
                println!("✅ SQLite database is up to date");
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
            json,
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
                repo,
                worktree,
                commit,
                force,
                generate_embeddings
            );

            // Log scan mode for user awareness (suppress human output in JSON mode)
            if force {
                tracing::info!("🔄 Force flag enabled - performing full repository scan");
                if !json {
                    println!("🔄 Full scan mode (--force flag enabled)");
                }
            } else {
                tracing::info!(
                    "⚡ Incremental mode - only scanning changed files (use --force for full scan)"
                );
                if !json {
                    println!("⚡ Incremental scan mode (use --force for full scan)");
                }
            }

            // Create progress tracker
            let mode = if json {
                OutputMode::Json
            } else if verbose {
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
                let repo_id = match store
                    .get_or_create_repo(&repo, root_abs.to_string_lossy().as_ref())
                    .await
                {
                    Ok(id) => Some(id),
                    Err(e) => {
                        tracing::warn!("Could not get repo ID: {}, proceeding with full scan", e);
                        None
                    }
                };

                if let Some(repo_id) = repo_id {
                    let worktree_id = match store
                        .get_or_create_worktree(
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
                                if json {
                                    // In JSON mode, emit complete event with 0 files
                                    println!(
                                        r#"{{"type":"complete","files":0,"duration":0,"elapsed":0,"timestamp":"{}"}}"#,
                                        chrono::Utc::now().to_rfc3339()
                                    );
                                } else {
                                    println!(
                                        "✓ No changes detected (tree SHA match), skipping scan"
                                    );
                                }
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
                match auto_generate_embeddings(embedding_batch_size, provider, json).await {
                    Ok(stats) => {
                        if stats.total_chunks > 0 && !json {
                            println!("\n📊 Embedding Generation Summary:");
                            println!("   {}", stats.summary());
                        }
                    }
                    Err(e) => {
                        // Don't fail the entire scan if embeddings fail
                        tracing::warn!("Embedding generation failed: {}", e);
                        if json {
                            // In JSON mode, emit error event
                            println!(
                                r#"{{"type":"error","message":"Embedding generation failed: {}","error_type":"embedding"}}"#,
                                e
                            );
                        } else {
                            println!("\n⚠️  Warning: Embedding generation failed: {}", e);
                            println!("   You can generate embeddings later with: crewchief-maproom generate-embeddings");
                        }
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
                        tracing::warn!("Could not canonicalize path for state update: {}", e);
                        path.clone()
                    }
                };

                // Get repo ID
                let repo_id = match store
                    .get_or_create_repo(&repo, root_abs.to_string_lossy().as_ref())
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
                    let worktree_id = match store
                        .get_or_create_worktree(
                            repo_id,
                            &worktree,
                            root_abs.to_string_lossy().as_ref(),
                        )
                        .await
                    {
                        Ok(id) => Some(id),
                        Err(e) => {
                            tracing::warn!("Could not get worktree ID for state update: {}", e);
                            None
                        }
                    };

                    if let Some(wt_id) = worktree_id {
                        // Update index state with current tree SHA and stats
                        match store
                            .update_index_state(wt_id, current_tree_sha, &scan_stats)
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
                                tracing::warn!(
                                    "Scan completed successfully, but next scan may be slower"
                                );
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
                match auto_generate_embeddings(embedding_batch_size, provider, false).await {
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
            json,
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
                if !json {
                    eprintln!("Warning: --worktree flag is deprecated and ignored.");
                    eprintln!("The watch command now auto-detects branch switches.");
                    eprintln!("Using auto-detected branch: {}", detected_branch);
                }
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
            let watch_path = path
                .canonicalize()
                .with_context(|| format!("Failed to canonicalize path: {}", path.display()))?;
            let watch_path_str = watch_path.to_string_lossy().to_string();

            // Ensure repo and worktree exist
            let repo_id = store.get_or_create_repo(&repo, &watch_path_str).await?;
            let initial_worktree_id = store
                .get_or_create_worktree(repo_id, &worktree, &watch_path_str)
                .await?;

            // Wrap worktree_id and current_branch in thread-safe state for dynamic updates
            // Uses std::sync::RwLock (not tokio::sync) to match existing codebase patterns
            let worktree_id: Arc<RwLock<i64>> = Arc::new(RwLock::new(initial_worktree_id));
            let current_branch: Arc<RwLock<String>> = Arc::new(RwLock::new(worktree.clone()));

            // Create debouncer for branch switch events (2-second window)
            use crewchief_maproom::indexer::DebouncedHandler;
            let branch_debouncer = DebouncedHandler::new(std::time::Duration::from_secs(2));

            // Create and start the file watcher
            use crewchief_maproom::incremental::{MultiWatcher, WatcherConfig};

            let debounce_ms = parse_throttle(&throttle)?;
            let config = WatcherConfig {
                debounce_ms,
                ..Default::default()
            };

            let (mut multi_watcher, mut event_rx) = MultiWatcher::new(config);

            // Add the worktree to watch
            multi_watcher
                .add_worktree(worktree.clone(), watch_path.clone())
                .await?;

            // Set up HEAD file watcher to detect branch switches
            use crewchief_maproom::indexer::setup_head_watcher;
            let git_head = watch_path.join(".git/HEAD");
            let (head_tx, mut head_rx) = tokio::sync::mpsc::channel(10);
            // Store watcher handle to prevent premature drop (watcher stops if dropped)
            let _head_watcher = setup_head_watcher(&git_head, head_tx)?;

            if !json {
                println!("👀 Watching {} for changes...", watch_path.display());
                println!("   Repository: {}", repo);
                println!("   Worktree: {}", worktree);
                println!("   Throttle: {}", throttle);
                println!();
                println!("Press Ctrl+C to stop.");
            }

            // Handle events
            use crewchief_maproom::incremental::incremental_update;
            use tokio::signal;

            loop {
                tokio::select! {
                    // Handle shutdown signal
                    _ = signal::ctrl_c() => {
                        if !json {
                            println!("\n🛑 Shutting down watch...");
                        }
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

                        if json {
                            // Emit JSON event
                            println!(
                                r#"{{"type":"file_event","event":"{}","path":"{}","timestamp":"{}"}}"#,
                                event_type,
                                event.path.display(),
                                chrono::Utc::now().to_rfc3339()
                            );
                        } else {
                            println!("📁 {} {}", event_type, event.path.display());
                        }

                        // Read worktree_id from lock (copy value, drop lock, then use)
                        let wt_id = *worktree_id.read().unwrap();

                        // Trigger incremental update for the worktree
                        match incremental_update(&store, wt_id, &watch_path).await {
                            Ok(stats) => {
                                if stats.files_processed > 0 {
                                    if json {
                                        println!(
                                            r#"{{"type":"update_complete","files_processed":{},"timestamp":"{}"}}"#,
                                            stats.files_processed,
                                            chrono::Utc::now().to_rfc3339()
                                        );
                                    } else {
                                        println!("   ✅ Processed {} files", stats.files_processed);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Incremental update failed: {}", e);
                                if json {
                                    println!(
                                        r#"{{"type":"update_error","error":"{}","timestamp":"{}"}}"#,
                                        e,
                                        chrono::Utc::now().to_rfc3339()
                                    );
                                } else {
                                    println!("   ⚠️  Update failed: {}", e);
                                }
                            }
                        }
                    }

                    // Handle HEAD file changes (branch switches)
                    Some(_head_event) = head_rx.recv() => {
                        if let Err(e) = handle_branch_switch(
                            &watch_path,
                            &store,
                            &repo,
                            repo_id,
                            &current_branch,
                            &worktree_id,
                            &branch_debouncer,
                        ).await {
                            tracing::warn!("Branch switch handler error: {}", e);
                        }
                    }
                }
            }

            if !json {
                println!("Watch complete.");
            }
        }

        Commands::Search {
            repo,
            worktree,
            query,
            k,
            debug,
            deduplicate,
            kind,
            lang,
            preview,
            preview_length,
            format,
        } => {
            // MRIMP-5: Implicit preview enable for agent format (parameter preprocessing)
            // Agent format always needs preview data; default length is 120 chars (token-optimized).
            // Explicit --preview-length overrides the agent default.
            let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
                (true, preview_length.unwrap_or(120))
            } else {
                (preview, preview_length.unwrap_or(200))
            };

            let store = db::connect().await?;
            // Fetch extra results if deduplication is enabled
            let fetch_k = if deduplicate { k * 3 } else { k };
            let hits = store
                .search_chunks_fts(
                    &repo,
                    worktree.as_deref(),
                    &query,
                    fetch_k,
                    debug,
                    kind.as_deref(),
                    lang.as_deref(),
                )
                .await?;

            // Apply deduplication if enabled
            let hits = if deduplicate {
                deduplicate_search_hits(hits, k as usize)
            } else {
                hits
            };

            // Post-process preview field
            let hits: Vec<_> = hits
                .into_iter()
                .map(|mut hit| {
                    if preview_enabled {
                        // Truncate preview to preview_len
                        if let Some(preview_text) = hit.preview.take() {
                            hit.preview = Some(db::truncate_preview(&preview_text, preview_len));
                        }
                    } else {
                        // Strip preview if flag not set
                        hit.preview = None;
                    }
                    hit
                })
                .collect();

            // MRIMP-5: Route output through format module
            match format {
                OutputFormat::Json => {
                    let output = format_hits_json_search(&hits)?;
                    println!("{}", output);
                }
                OutputFormat::Agent => {
                    let output = format_hits_agent(&hits);
                    if !output.is_empty() {
                        println!("{}", output);
                    }
                }
            }
        }

        Commands::VectorSearch {
            repo,
            worktree,
            query,
            k,
            threshold,
            kind,
            lang,
            preview,
            preview_length,
            format,
        } => {
            use crewchief_maproom::embedding::EmbeddingService;

            // MRIMP-5: Implicit preview enable for agent format (parameter preprocessing)
            // Agent format always needs preview data; default length is 120 chars (token-optimized).
            // Explicit --preview-length overrides the agent default.
            let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
                (true, preview_length.unwrap_or(120))
            } else {
                (preview, preview_length.unwrap_or(200))
            };

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
            let search_hits = match store
                .search_chunks_vector(
                    &repo,
                    worktree.as_deref(),
                    &query_embedding,
                    k as i64,
                    false,
                    kind.as_deref(),
                    lang.as_deref(),
                )
                .await
            {
                Ok(hits) => hits,
                Err(e) => {
                    // Check for SQLite-specific errors (no vector support)
                    let err_str = e.to_string();
                    if err_str.contains("sqlite-vec")
                        || err_str.contains("vector")
                        || err_str.contains("not available")
                    {
                        eprintln!("Vector search unavailable: {}", e);
                        eprintln!("Tip: Use 'search' command for full-text search instead");
                        std::process::exit(1);
                    }
                    return Err(e);
                }
            };

            // Apply threshold filtering and preview processing on SearchHit objects
            let hits: Vec<_> = search_hits
                .into_iter()
                .filter(|hit| {
                    if let Some(thresh) = threshold {
                        hit.score >= thresh as f64
                    } else {
                        true
                    }
                })
                .map(|mut hit| {
                    if preview_enabled {
                        // Truncate preview to preview_len
                        if let Some(preview_text) = hit.preview.take() {
                            hit.preview = Some(db::truncate_preview(&preview_text, preview_len));
                        }
                    } else {
                        // Strip preview if flag not set
                        hit.preview = None;
                    }
                    hit
                })
                .collect();

            // MRIMP-5: Route output through format module
            match format {
                OutputFormat::Json => {
                    // Convert SearchHit objects to serde_json::Value for JSON format
                    let json_hits: Vec<serde_json::Value> = hits
                        .iter()
                        .map(|hit| {
                            let mut obj = serde_json::json!({
                                "chunk_id": hit.chunk_id,
                                "score": hit.score,
                                "start_line": hit.start_line,
                                "end_line": hit.end_line,
                                "symbol_name": hit.symbol_name,
                                "kind": hit.kind,
                                "file_path": hit.file_relpath,
                            });

                            // Conditionally add preview if enabled and preview exists
                            if let Some(ref preview_text) = hit.preview {
                                obj.as_object_mut()
                                    .unwrap()
                                    .insert("preview".to_string(), serde_json::json!(preview_text));
                            }

                            obj
                        })
                        .collect();

                    let output = format_hits_json_vector(
                        &json_hits,
                        json_hits.len(),
                        &query,
                        "vector",
                        k,
                        threshold,
                    )?;
                    println!("{}", output);
                }
                OutputFormat::Agent => {
                    let output = format_hits_agent(&hits);
                    if !output.is_empty() {
                        println!("{}", output);
                    }
                }
            }
        }

        Commands::Status {
            repo,
            worktree,
            json,
            verbose,
        } => {
            use crewchief_maproom::status;

            // Validate worktree filter requires repo filter
            if worktree.is_some() && repo.is_none() {
                anyhow::bail!("--worktree requires --repo to be specified");
            }

            tracing::debug!("status: connecting to database...");
            let store = db::connect().await?;
            tracing::debug!("status: connected, querying status...");

            match status::get_status(Arc::new(store), repo, worktree, verbose).await {
                Ok(status_data) => {
                    tracing::debug!("status: query complete, formatting output...");
                    if json {
                        let output = status::format_json(&status_data)?;
                        println!("{}", output);
                    } else {
                        let output = status::format_text(&status_data, verbose);
                        print!("{}", output);
                    }
                }
                Err(e) => {
                    eprintln!("Error querying status: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::EncodingProgress { repo, json } => {
            use crewchief_maproom::encoding_progress;

            tracing::debug!("encoding-progress: connecting to database...");
            let store = db::connect().await?;
            tracing::debug!("encoding-progress: connected, querying progress...");

            match encoding_progress::get_encoding_progress(Arc::new(store), repo).await {
                Ok(progress_data) => {
                    tracing::debug!("encoding-progress: query complete, formatting output...");
                    if json {
                        let output = encoding_progress::format_json(&progress_data)?;
                        println!("{}", output);
                    } else {
                        let output = encoding_progress::format_text(&progress_data);
                        print!("{}", output);
                    }
                }
                Err(e) => {
                    eprintln!("Error querying encoding progress: {}", e);
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
        Commands::Serve {
            socket,
            socket_path,
            idle_timeout,
        } => {
            if socket {
                // Socket mode (experimental)
                use crewchief_maproom::daemon::server::{
                    run_with_signal_handling, ServerConfig, SocketServer,
                };

                let mut config = ServerConfig::default_for_user()?;

                if let Some(path) = socket_path {
                    config.socket_path = path;
                }

                config.idle_timeout = std::time::Duration::from_secs(idle_timeout);

                tracing::info!(
                    socket_path = %config.socket_path.display(),
                    idle_timeout_secs = idle_timeout,
                    "Starting socket server with signal handling"
                );

                let server = SocketServer::new(config)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create socket server: {}", e))?;

                run_with_signal_handling(server)
                    .await
                    .map_err(|e| anyhow::anyhow!("Socket server error: {}", e))?;
            } else {
                // Stdio mode (default)
                tracing::info!("Starting stdio daemon");
                daemon::run().await?;
            }
        }

        Commands::CleanIgnored {
            repo,
            worktree,
            dry_run,
        } => {
            use crewchief_maproom::cli::clean_ignored;
            let store = db::connect().await?;
            clean_ignored::clean_ignored(&store, &repo, &worktree, dry_run).await?;
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
            let store = db::connect().await.context("Database connection failed")?;

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
                print!("{}", format_context_bundle(&bundle, chunk_id, budget));
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
            "maproom",
            "context",
            "--chunk-id",
            "99999",
            "--budget",
            "4000",
            "--callers",
            "--callees",
            "--tests",
            "--max-depth",
            "5",
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
            "maproom",
            "context",
            "--chunk-id",
            "42",
            "--budget",
            "8000",
            "--callers",
            "--callees",
            "--tests",
            "--docs",
            "--config",
            "--max-depth",
            "3",
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

    #[test]
    fn test_role_emoji_known_roles() {
        assert_eq!(super::role_emoji("primary"), "📄");
        assert_eq!(super::role_emoji("caller"), "🔗");
        assert_eq!(super::role_emoji("callee"), "📤");
        assert_eq!(super::role_emoji("test"), "🧪");
        assert_eq!(super::role_emoji("doc"), "📚");
        assert_eq!(super::role_emoji("config"), "⚙️");
        assert_eq!(super::role_emoji("hook"), "🪝");
        assert_eq!(super::role_emoji("jsx_parent"), "⬆️");
        assert_eq!(super::role_emoji("jsx_child"), "⬇️");
    }

    #[test]
    fn test_role_emoji_unknown_role() {
        assert_eq!(super::role_emoji("unknown"), "📎");
        assert_eq!(super::role_emoji("foobar"), "📎");
    }

    #[test]
    fn test_role_emoji_case_insensitive() {
        assert_eq!(super::role_emoji("PRIMARY"), "📄");
        assert_eq!(super::role_emoji("Caller"), "🔗");
        assert_eq!(super::role_emoji("TEST"), "🧪");
    }

    #[test]
    fn test_format_context_bundle_basic() {
        use crewchief_maproom::context::{ContextBundle, ContextItem, LineRange};

        let mut bundle = ContextBundle::new();
        bundle.add_item(ContextItem {
            relpath: "src/auth.ts".to_string(),
            range: LineRange::new(10, 30),
            role: "primary".to_string(),
            reason: "".to_string(),
            content: "async function authenticate(user: User) {\n  return token;\n}".to_string(),
            tokens: 150,
        });

        let output = super::format_context_bundle(&bundle, 12345, 6000);

        assert!(output.contains("📦 Context Bundle for chunk #12345"));
        assert!(output.contains("Budget: 6000 tokens"));
        assert!(output.contains("Used: 150 tokens"));
        assert!(output.contains("Truncated: No"));
        assert!(output.contains("📄 PRIMARY: src/auth.ts:10-30"));
        assert!(output.contains("Tokens: 150"));
        assert!(output.contains("authenticate")); // content preview
    }

    #[test]
    fn test_format_context_bundle_empty() {
        use crewchief_maproom::context::ContextBundle;

        let bundle = ContextBundle::new();
        let output = super::format_context_bundle(&bundle, 99999, 6000);

        assert!(output.contains("📦 Context Bundle for chunk #99999"));
        assert!(output.contains("Used: 0 tokens"));
        assert!(output.contains("(No context items found)"));
    }

    #[test]
    fn test_format_context_bundle_truncated() {
        use crewchief_maproom::context::{ContextBundle, ContextItem, LineRange};

        let mut bundle = ContextBundle::new();
        bundle.truncated = true;
        bundle.add_item(ContextItem {
            relpath: "src/main.rs".to_string(),
            range: LineRange::new(1, 10),
            role: "primary".to_string(),
            reason: "".to_string(),
            content: "fn main() {}".to_string(),
            tokens: 5500,
        });

        let output = super::format_context_bundle(&bundle, 42, 6000);

        assert!(output.contains("Truncated: Yes"));
    }

    #[test]
    fn test_format_context_bundle_with_related_items() {
        use crewchief_maproom::context::{ContextBundle, ContextItem, LineRange};

        let mut bundle = ContextBundle::new();
        bundle.add_item(ContextItem {
            relpath: "src/auth.ts".to_string(),
            range: LineRange::new(10, 30),
            role: "primary".to_string(),
            reason: "".to_string(),
            content: "function authenticate() {}".to_string(),
            tokens: 100,
        });
        bundle.add_item(ContextItem {
            relpath: "src/login.ts".to_string(),
            range: LineRange::new(40, 60),
            role: "caller".to_string(),
            reason: "Calls authenticate function".to_string(),
            content: "function login() { authenticate(); }".to_string(),
            tokens: 120,
        });
        bundle.add_item(ContextItem {
            relpath: "src/__tests__/auth.test.ts".to_string(),
            range: LineRange::new(5, 25),
            role: "test".to_string(),
            reason: "Test file for primary function".to_string(),
            content: "test('auth', () => {});".to_string(),
            tokens: 80,
        });

        let output = super::format_context_bundle(&bundle, 12345, 6000);

        // Check all items are present
        assert!(output.contains("📄 PRIMARY: src/auth.ts:10-30"));
        assert!(output.contains("🔗 CALLER: src/login.ts:40-60"));
        assert!(output.contains("🧪 TEST: src/__tests__/auth.test.ts:5-25"));

        // Check reasons are shown for non-primary items
        assert!(output.contains("Reason: Calls authenticate function"));
        assert!(output.contains("Reason: Test file for primary function"));

        // Check total tokens
        assert!(output.contains("Used: 300 tokens")); // 100 + 120 + 80
    }

    // ==================== Filter Flag CLI Parsing Tests ====================

    #[test]
    fn test_search_with_kind_single_value() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--kind", "func",
        ]);
        match cli.command {
            Commands::Search { kind, .. } => {
                assert_eq!(kind, Some(vec!["func".to_string()]));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_with_kind_multiple_values() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--kind",
            "func,class,method",
        ]);
        match cli.command {
            Commands::Search { kind, .. } => {
                assert_eq!(
                    kind,
                    Some(vec![
                        "func".to_string(),
                        "class".to_string(),
                        "method".to_string(),
                    ])
                );
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_with_lang_single_value() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--lang", "py",
        ]);
        match cli.command {
            Commands::Search { lang, .. } => {
                assert_eq!(lang, Some(vec!["py".to_string()]));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_with_lang_multiple_values() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--lang", "py,ts,rs",
        ]);
        match cli.command {
            Commands::Search { lang, .. } => {
                assert_eq!(
                    lang,
                    Some(vec!["py".to_string(), "ts".to_string(), "rs".to_string(),])
                );
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_with_both_kind_and_lang() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--kind",
            "func,class",
            "--lang",
            "py,rs",
        ]);
        match cli.command {
            Commands::Search { kind, lang, .. } => {
                assert_eq!(kind, Some(vec!["func".to_string(), "class".to_string()]));
                assert_eq!(lang, Some(vec!["py".to_string(), "rs".to_string()]));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_with_no_filters() {
        let cli = Cli::parse_from(&["maproom", "search", "--repo", "test", "--query", "foo"]);
        match cli.command {
            Commands::Search { kind, lang, .. } => {
                assert_eq!(kind, None);
                assert_eq!(lang, None);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_vector_search_with_kind_and_lang() {
        let cli = Cli::parse_from(&[
            "maproom",
            "vector-search",
            "--repo",
            "test",
            "--query",
            "authentication logic",
            "--kind",
            "func,method",
            "--lang",
            "ts,rs",
        ]);
        match cli.command {
            Commands::VectorSearch { kind, lang, .. } => {
                assert_eq!(kind, Some(vec!["func".to_string(), "method".to_string()]));
                assert_eq!(lang, Some(vec!["ts".to_string(), "rs".to_string()]));
            }
            _ => panic!("Expected VectorSearch command"),
        }
    }

    #[test]
    fn test_vector_search_with_no_filters() {
        let cli = Cli::parse_from(&[
            "maproom",
            "vector-search",
            "--repo",
            "test",
            "--query",
            "authentication logic",
        ]);
        match cli.command {
            Commands::VectorSearch { kind, lang, .. } => {
                assert_eq!(kind, None);
                assert_eq!(lang, None);
            }
            _ => panic!("Expected VectorSearch command"),
        }
    }

    #[test]
    fn test_status_default_no_verbose() {
        let cli = Cli::parse_from(&["maproom", "status"]);
        match cli.command {
            Commands::Status {
                repo,
                worktree,
                json,
                verbose,
            } => {
                assert_eq!(repo, None);
                assert_eq!(worktree, None);
                assert!(!json);
                assert!(!verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_status_with_verbose() {
        let cli = Cli::parse_from(&["maproom", "status", "--verbose"]);
        match cli.command {
            Commands::Status { verbose, .. } => {
                assert!(verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_status_verbose_with_json() {
        let cli = Cli::parse_from(&["maproom", "status", "--verbose", "--json"]);
        match cli.command {
            Commands::Status { json, verbose, .. } => {
                assert!(json);
                assert!(verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_status_verbose_with_repo() {
        let cli = Cli::parse_from(&["maproom", "status", "--verbose", "--repo", "myrepo"]);
        match cli.command {
            Commands::Status { repo, verbose, .. } => {
                assert_eq!(repo, Some("myrepo".to_string()));
                assert!(verbose);
            }
            _ => panic!("Expected Status command"),
        }
    }

    // ==================== Preview Flag CLI Parsing Tests (MRIMP-3.2001) ====================

    #[test]
    fn test_search_preview_flag() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--preview",
        ]);
        match cli.command {
            Commands::Search { preview, .. } => {
                assert!(preview);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_preview_length_flag() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--preview-length",
            "150",
        ]);
        match cli.command {
            Commands::Search { preview_length, .. } => {
                assert_eq!(preview_length, Some(150));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_defaults() {
        let cli = Cli::parse_from(&["maproom", "search", "--repo", "test", "--query", "foo"]);
        match cli.command {
            Commands::Search {
                preview,
                preview_length,
                ..
            } => {
                assert_eq!(preview, false);
                assert_eq!(preview_length, None);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_vector_search_preview_flag() {
        let cli = Cli::parse_from(&[
            "maproom",
            "vector-search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--preview",
        ]);
        match cli.command {
            Commands::VectorSearch { preview, .. } => {
                assert!(preview);
            }
            _ => panic!("Expected VectorSearch command"),
        }
    }

    #[test]
    fn test_vector_search_preview_length_flag() {
        let cli = Cli::parse_from(&[
            "maproom",
            "vector-search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--preview-length",
            "150",
        ]);
        match cli.command {
            Commands::VectorSearch { preview_length, .. } => {
                assert_eq!(preview_length, Some(150));
            }
            _ => panic!("Expected VectorSearch command"),
        }
    }

    #[test]
    fn test_search_combined_flags_with_preview() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--preview",
            "--kind",
            "func",
            "--lang",
            "py",
        ]);
        match cli.command {
            Commands::Search {
                preview,
                kind,
                lang,
                ..
            } => {
                assert!(preview);
                assert_eq!(kind, Some(vec!["func".to_string()]));
                assert_eq!(lang, Some(vec!["py".to_string()]));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_preview_length_without_preview() {
        // Flag interaction: --preview-length without --preview is valid
        // (length is parsed but ignored when preview=false per architecture.md Decision 4)
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--preview-length",
            "150",
        ]);
        match cli.command {
            Commands::Search {
                preview,
                preview_length,
                ..
            } => {
                assert_eq!(preview, false);
                assert_eq!(preview_length, Some(150));
            }
            _ => panic!("Expected Search command"),
        }
    }

    // ==================== Encoding Progress CLI Parsing Tests (ENCPROG.1001) ====================

    // Test Case #26: encoding-progress with no args
    #[test]
    fn test_encoding_progress_no_args() {
        let cli = Cli::parse_from(&["maproom", "encoding-progress"]);
        match cli.command {
            Commands::EncodingProgress { repo, json } => {
                assert_eq!(repo, None);
                assert!(!json);
            }
            _ => panic!("Expected EncodingProgress command"),
        }
    }

    // Test Case #27: encoding-progress --json
    #[test]
    fn test_encoding_progress_json() {
        let cli = Cli::parse_from(&["maproom", "encoding-progress", "--json"]);
        match cli.command {
            Commands::EncodingProgress { repo, json } => {
                assert_eq!(repo, None);
                assert!(json);
            }
            _ => panic!("Expected EncodingProgress command"),
        }
    }

    // Test Case #28: encoding-progress --repo myrepo
    #[test]
    fn test_encoding_progress_repo() {
        let cli = Cli::parse_from(&["maproom", "encoding-progress", "--repo", "myrepo"]);
        match cli.command {
            Commands::EncodingProgress { repo, json } => {
                assert_eq!(repo, Some("myrepo".to_string()));
                assert!(!json);
            }
            _ => panic!("Expected EncodingProgress command"),
        }
    }

    // Test Case #29: encoding-progress --repo myrepo --json
    #[test]
    fn test_encoding_progress_repo_and_json() {
        let cli = Cli::parse_from(&["maproom", "encoding-progress", "--repo", "myrepo", "--json"]);
        match cli.command {
            Commands::EncodingProgress { repo, json } => {
                assert_eq!(repo, Some("myrepo".to_string()));
                assert!(json);
            }
            _ => panic!("Expected EncodingProgress command"),
        }
    }

    // ==================== Format Flag CLI Parsing Tests (MRIMP-5.2002) ====================

    #[test]
    fn test_search_format_agent() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--format", "agent",
        ]);
        match cli.command {
            Commands::Search { format, .. } => {
                assert_eq!(format, OutputFormat::Agent);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_json() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--format", "json",
        ]);
        match cli.command {
            Commands::Search { format, .. } => {
                assert_eq!(format, OutputFormat::Json);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_default_is_json() {
        // No --format flag should default to OutputFormat::Json
        let cli = Cli::parse_from(&["maproom", "search", "--repo", "test", "--query", "foo"]);
        match cli.command {
            Commands::Search { format, .. } => {
                assert_eq!(format, OutputFormat::Json);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_invalid_produces_error() {
        // --format invalid should cause a clap parse error
        let result = Cli::try_parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--format", "invalid",
        ]);
        assert!(
            result.is_err(),
            "Expected clap error for invalid format value"
        );
    }

    #[test]
    fn test_vector_search_format_agent() {
        let cli = Cli::parse_from(&[
            "maproom",
            "vector-search",
            "--repo",
            "test",
            "--query",
            "auth logic",
            "--format",
            "agent",
        ]);
        match cli.command {
            Commands::VectorSearch { format, .. } => {
                assert_eq!(format, OutputFormat::Agent);
            }
            _ => panic!("Expected VectorSearch command"),
        }
    }

    #[test]
    fn test_vector_search_format_default_is_json() {
        let cli = Cli::parse_from(&[
            "maproom",
            "vector-search",
            "--repo",
            "test",
            "--query",
            "auth logic",
        ]);
        match cli.command {
            Commands::VectorSearch { format, .. } => {
                assert_eq!(format, OutputFormat::Json);
            }
            _ => panic!("Expected VectorSearch command"),
        }
    }

    // ==================== Format + Flag Interaction Tests (MRIMP-5.2002) ====================

    #[test]
    fn test_search_format_agent_with_preview() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--format",
            "agent",
            "--preview",
        ]);
        match cli.command {
            Commands::Search {
                format, preview, ..
            } => {
                assert_eq!(format, OutputFormat::Agent);
                assert!(preview);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_agent_with_preview_length() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--format",
            "agent",
            "--preview-length",
            "50",
        ]);
        match cli.command {
            Commands::Search {
                format,
                preview_length,
                ..
            } => {
                assert_eq!(format, OutputFormat::Agent);
                assert_eq!(preview_length, Some(50));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_json_with_preview() {
        let cli = Cli::parse_from(&[
            "maproom",
            "search",
            "--repo",
            "test",
            "--query",
            "foo",
            "--format",
            "json",
            "--preview",
        ]);
        match cli.command {
            Commands::Search {
                format, preview, ..
            } => {
                assert_eq!(format, OutputFormat::Json);
                assert!(preview);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_agent_with_kind() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--format", "agent", "--kind",
            "func",
        ]);
        match cli.command {
            Commands::Search { format, kind, .. } => {
                assert_eq!(format, OutputFormat::Agent);
                assert_eq!(kind, Some(vec!["func".to_string()]));
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_search_format_agent_with_lang() {
        let cli = Cli::parse_from(&[
            "maproom", "search", "--repo", "test", "--query", "foo", "--format", "agent", "--lang",
            "py",
        ]);
        match cli.command {
            Commands::Search { format, lang, .. } => {
                assert_eq!(format, OutputFormat::Agent);
                assert_eq!(lang, Some(vec!["py".to_string()]));
            }
            _ => panic!("Expected Search command"),
        }
    }

    // ==================== Implicit Preview Enable Tests (MRIMP-5.2002) ====================

    #[test]
    fn test_agent_format_implicit_preview_enabled() {
        // Agent format should implicitly enable preview with default length 120
        let format = OutputFormat::Agent;
        let preview = false; // Not explicitly set
        let preview_length: Option<usize> = None;

        let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
            (true, preview_length.unwrap_or(120))
        } else {
            (preview, preview_length.unwrap_or(200))
        };

        assert!(
            preview_enabled,
            "Agent format must implicitly enable preview"
        );
        assert_eq!(
            preview_len, 120,
            "Agent format default preview length must be 120"
        );
    }

    #[test]
    fn test_agent_format_explicit_preview_length_override() {
        // Agent format with explicit --preview-length should use that length
        let format = OutputFormat::Agent;
        let preview = false;
        let preview_length: Option<usize> = Some(50);

        let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
            (true, preview_length.unwrap_or(120))
        } else {
            (preview, preview_length.unwrap_or(200))
        };

        assert!(
            preview_enabled,
            "Agent format must implicitly enable preview"
        );
        assert_eq!(
            preview_len, 50,
            "Explicit preview-length must override agent default"
        );
    }

    #[test]
    fn test_json_format_no_implicit_preview() {
        // JSON format without --preview should NOT enable preview
        let format = OutputFormat::Json;
        let preview = false;
        let preview_length: Option<usize> = None;

        let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
            (true, preview_length.unwrap_or(120))
        } else {
            (preview, preview_length.unwrap_or(200))
        };

        assert!(
            !preview_enabled,
            "JSON format must NOT implicitly enable preview"
        );
        assert_eq!(
            preview_len, 200,
            "JSON format default preview length must be 200"
        );
    }

    #[test]
    fn test_json_format_with_explicit_preview() {
        // JSON format with --preview should enable preview with default 200
        let format = OutputFormat::Json;
        let preview = true;
        let preview_length: Option<usize> = None;

        let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
            (true, preview_length.unwrap_or(120))
        } else {
            (preview, preview_length.unwrap_or(200))
        };

        assert!(
            preview_enabled,
            "JSON format with --preview must enable preview"
        );
        assert_eq!(
            preview_len, 200,
            "JSON format default preview length must be 200"
        );
    }

    #[test]
    fn test_json_format_explicit_preview_length_override() {
        // JSON format with explicit --preview-length should use that length
        let format = OutputFormat::Json;
        let preview = true;
        let preview_length: Option<usize> = Some(300);

        let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
            (true, preview_length.unwrap_or(120))
        } else {
            (preview, preview_length.unwrap_or(200))
        };

        assert!(preview_enabled);
        assert_eq!(
            preview_len, 300,
            "Explicit preview-length must override JSON default"
        );
    }
}
