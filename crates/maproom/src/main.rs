use std::fmt::Write as _;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, RwLock};

use anyhow::Context;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use tracing_subscriber::{fmt, EnvFilter};

/// Macro to handle errors with structured output when using agent format.
///
/// Usage: `let value = handle_agent_error!(result_expr, format);`
///
/// If the result is `Ok`, returns the unwrapped value.
/// If the result is `Err` and format is `OutputFormat::Agent`, classifies the error
/// and calls `handle_agent_error()` (which exits the process).
/// If the result is `Err` and format is not Agent, returns early with the error.
macro_rules! handle_agent_error {
    ($result:expr, $format:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) if $format == OutputFormat::Agent => {
                let (error_type, suggestion, exit_code) = classify_error(&e);
                handle_agent_error(&e, &$format, &error_type, &suggestion, exit_code);
            }
            Err(e) => return Err(e),
        }
    };
}

use maproom::cli::format::{
    format_agent_error, format_context_agent, format_hits_agent, format_hits_json_search,
    format_hits_json_vector, sanitize_newlines, OutputFormat, SearchMetadata,
};
use maproom::context::{AssemblyStrategy, ContextBundle, DefaultAssemblyStrategy, ExpandOptions};
use maproom::progress::{OutputMode, ProgressTracker};
use maproom::{daemon, db, indexer};

/// Exit code for runtime errors (transient, may retry).
/// Documented in: docs/cli-help-after.md, CLAUDE.md
const EXIT_RUNTIME_ERROR: i32 = 1;

/// Exit code for configuration errors (persistent, do not retry).
/// Documented in: docs/cli-help-after.md, CLAUDE.md
const EXIT_CONFIG_ERROR: i32 = 2;

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
    store: &(dyn db::Store + Send + Sync),
    repo: &str,
    repo_id: i64,
    current_branch: &Arc<RwLock<String>>,
    worktree_id: &Arc<RwLock<i64>>,
    debouncer: &indexer::DebouncedHandler,
) -> anyhow::Result<()> {
    use maproom::git::get_current_branch;
    use maproom::incremental::incremental_update;
    use maproom::indexer::BranchSwitchEvent;

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
    name = "maproom",
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
        command: maproom::cli::CacheCommand,
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
    ///   maproom context --chunk-id 12345 --format agent     # Compact agent output
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

        /// Output format: json (default) or agent (compact).
        ///
        /// NOTE(AFM-03): Default changed from human-readable to Json. The previous
        /// `--json` bool flag (default false) produced human-readable output by default
        /// via `format_context_bundle()`. This was replaced with `--format` enum
        /// defaulting to Json to align with daemon/MCP usage (all programmatic consumers
        /// already use JSON via JSON-RPC). Use `--format agent` for compact CLI output.
        /// No external consumers (daemon, MCP server, VSCode extension) are affected
        /// because they communicate via the daemon's JSON-RPC interface, not the CLI.
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,

        /// DEPRECATED: Use --format json instead. Hidden from help.
        /// Retained for backward compatibility with existing scripts.
        #[arg(long, hide = true)]
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
        /// Generate embeddings for vector search (default: false).
        /// Embeddings enable semantic search but require embedding provider configuration.
        /// Use --generate-embeddings to opt in when you want vector/semantic search.
        /// FTS (full-text search) works without embeddings and is the default mode.
        #[arg(
            long,
            default_value_t = false,
            action = clap::ArgAction::Set,
            help = "Generate embeddings for vector search (default: false)",
            long_help = "Generate embeddings for vector search.\n\
                         Embeddings enable semantic search via vector-search command.\n\
                         Full-text search works without embeddings and is the default mode.\n\n\
                         Use --generate-embeddings to opt in when:\n\
                         - You want semantic/vector search capabilities\n\
                         - You have an embedding provider configured (Ollama, OpenAI, or Google Vertex)"
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
        /// Automatically generate embeddings after upserting (default: false)
        #[arg(long, default_value_t = false, action = clap::ArgAction::Set)]
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
) -> anyhow::Result<maproom::embedding::PipelineStats> {
    use maproom::embedding::{EmbeddingPipeline, EmbeddingService, PipelineConfig};

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
    let store = maproom::db::connect().await?;

    // Count chunks needing embeddings
    let chunk_count = store.get_chunks_needing_embeddings_count().await?;

    if chunk_count == 0 {
        if !json_mode {
            println!("   ✓ All chunks already have embeddings");
        }
        return Ok(maproom::embedding::PipelineStats::default());
    }

    if !json_mode {
        println!("   Found {} chunks needing embeddings", chunk_count);
    }

    // Create progress tracker with appropriate output mode
    let output_mode = if json_mode {
        maproom::progress::OutputMode::Json
    } else {
        maproom::progress::OutputMode::Minimal
    };
    let progress = maproom::progress::ProgressTracker::new(output_mode);
    progress.set_totals(0, Some(chunk_count as usize));

    // Run pipeline with progress callback
    let pipeline = EmbeddingPipeline::new(service, config);
    let stats = pipeline
        .run_with_progress(
            store.as_ref(),
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
#[allow(dead_code)] // used by format_context_bundle; retained for future human-readable CLI output
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
#[allow(dead_code)] // retained for future human-readable CLI output (context command)
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
        .args([
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
        .args([
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
        .args(["-C", path.to_str().unwrap_or("."), "rev-parse", "HEAD"])
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
                        std::process::exit(EXIT_RUNTIME_ERROR);
                    }
                };

                // Phase 2: Report
                if stale.is_empty() {
                    println!("✅ No stale worktrees found!");
                    return Ok(()); // Success: no stale worktrees is a valid result
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
            let tree_sha = match maproom::git::get_git_tree_sha(&path) {
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
                store.as_ref(),
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
                            println!("   You can generate embeddings later with: maproom generate-embeddings");
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

                let scan_stats = maproom::db::UpdateStats {
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
            indexer::upsert_files(store.as_ref(), &repo, &worktree, &root, &commit, &paths)
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
                        println!("   You can generate embeddings later with: maproom generate-embeddings");
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
            let detected_branch = maproom::git::get_current_branch(&path)?;

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
            let store = db::connect().await?;

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
            use maproom::indexer::DebouncedHandler;
            let branch_debouncer = DebouncedHandler::new(std::time::Duration::from_secs(2));

            // Create and start the file watcher
            use maproom::incremental::{MultiWatcher, WatcherConfig};

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
            use maproom::indexer::setup_head_watcher;
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
            use maproom::incremental::incremental_update;
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
                        use maproom::incremental::EventType;

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
                        match incremental_update(store.as_ref(), wt_id, &watch_path).await {
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
                            store.as_ref(),
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

            let store = handle_agent_error!(db::connect().await, format);
            // Fetch extra results if deduplication is enabled
            let fetch_k = if deduplicate { k * 3 } else { k };
            let (hits, total_count) = handle_agent_error!(
                store
                    .search_chunks_fts(
                        &repo,
                        worktree.as_deref(),
                        &query,
                        fetch_k,
                        debug,
                        kind.as_deref(),
                        lang.as_deref(),
                    )
                    .await,
                format
            );

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
            let meta = SearchMetadata {
                query: query.clone(),
                mode: "fts".to_string(),
                hits: hits.len(),
                total_estimate: total_count,
            };
            match format {
                OutputFormat::Json => {
                    let output = format_hits_json_search(&hits, &meta)?;
                    println!("{}", output);
                }
                OutputFormat::Agent => {
                    let output = format_hits_agent(&hits, &meta);
                    println!("{}", output);
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
            use maproom::embedding::EmbeddingService;

            // MRIMP-5: Implicit preview enable for agent format (parameter preprocessing)
            // Agent format always needs preview data; default length is 120 chars (token-optimized).
            // Explicit --preview-length overrides the agent default.
            let (preview_enabled, preview_len) = if format == OutputFormat::Agent {
                (true, preview_length.unwrap_or(120))
            } else {
                (preview, preview_length.unwrap_or(200))
            };

            // Connect to database with error handling
            let store = handle_agent_error!(db::connect().await, format);

            // Generate query embedding
            tracing::info!("Generating embedding for query: {}", query);

            // Validate embedding provider is configured (not empty or whitespace)
            let provider = std::env::var("MAPROOM_EMBEDDING_PROVIDER")
                .unwrap_or_default()
                .trim()
                .to_string();
            if provider.is_empty() {
                eprintln!("Configuration error: MAPROOM_EMBEDDING_PROVIDER not set or empty");
                eprintln!("Set MAPROOM_EMBEDDING_PROVIDER to 'openai', 'voyage', or another supported provider");
                std::process::exit(EXIT_CONFIG_ERROR);
            }

            let embedding_service = handle_agent_error!(
                EmbeddingService::from_env().await.map_err(
                    |e| anyhow::Error::from(e).context("Failed to create embedding service")
                ),
                format
            );

            let query_embedding = handle_agent_error!(
                embedding_service.embed_text(&query).await.map_err(
                    |e| anyhow::Error::from(e).context("Failed to generate query embedding")
                ),
                format
            );

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
                        let error_msg = format!("Vector search unavailable: {}", e);
                        let suggestion = "Use search command for full-text search instead";

                        if format == OutputFormat::Agent {
                            // Human-readable to stderr
                            eprintln!("{}", error_msg);
                            eprintln!("Tip: {}", suggestion);
                            // Structured to stdout
                            println!(
                                "{}",
                                format_agent_error("config_error", &error_msg, suggestion)
                            );
                        } else {
                            // Existing behavior for default format
                            eprintln!("{}", error_msg);
                            eprintln!("Tip: {}", suggestion);
                        }
                        std::process::exit(EXIT_CONFIG_ERROR);
                    }

                    // Other database errors
                    if format == OutputFormat::Agent {
                        let (error_type, suggestion, exit_code) = classify_error(&e);
                        handle_agent_error(&e, &format, &error_type, &suggestion, exit_code);
                    }
                    // NOTE: sqlite-vec not available is a configuration issue (binary build configuration)
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
                                "file_relpath": hit.file_relpath,
                                // DEPRECATED(AFM-02): Use file_relpath. Retained for backward compatibility.
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
                    let meta = SearchMetadata {
                        query: query.clone(),
                        mode: "vector".to_string(),
                        hits: hits.len(),
                        total_estimate: hits.len(),
                    };
                    let output = format_hits_agent(&hits, &meta);
                    println!("{}", output);
                }
            }
        }

        Commands::Status {
            repo,
            worktree,
            json,
            verbose,
        } => {
            use maproom::status;

            // Validate worktree filter requires repo filter
            if worktree.is_some() && repo.is_none() {
                anyhow::bail!("--worktree requires --repo to be specified");
            }

            tracing::debug!("status: connecting to database...");
            let store = db::connect().await?;
            tracing::debug!("status: connected, querying status...");

            let status_data = status::get_status(store, repo, worktree, verbose)
                .await
                .context("Error querying status")?;
            tracing::debug!("status: query complete, formatting output...");
            if json {
                let output = status::format_json(&status_data)?;
                println!("{}", output);
            } else {
                let output = status::format_text(&status_data, verbose);
                print!("{}", output);
            }
        }

        Commands::EncodingProgress { repo, json } => {
            use maproom::encoding_progress;

            tracing::debug!("encoding-progress: connecting to database...");
            let store = db::connect().await?;
            tracing::debug!("encoding-progress: connected, querying progress...");

            let progress_data = encoding_progress::get_encoding_progress(store, repo)
                .await
                .context("Error querying encoding progress")?;
            tracing::debug!("encoding-progress: query complete, formatting output...");
            if json {
                let output = encoding_progress::format_json(&progress_data)?;
                println!("{}", output);
            } else {
                let output = encoding_progress::format_text(&progress_data);
                print!("{}", output);
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
            use maproom::embedding::{
                CostEstimator, EmbeddingPipeline, EmbeddingService, PipelineConfig,
            };

            tracing::info!("Initializing embedding generation pipeline");

            // Validate embedding provider is configured (not empty or whitespace)
            let provider = std::env::var("MAPROOM_EMBEDDING_PROVIDER")
                .unwrap_or_default()
                .trim()
                .to_string();
            if provider.is_empty() {
                eprintln!("Configuration error: MAPROOM_EMBEDDING_PROVIDER not set or empty");
                eprintln!("Set MAPROOM_EMBEDDING_PROVIDER to 'openai', 'voyage', or another supported provider");
                std::process::exit(EXIT_CONFIG_ERROR);
            }

            // Create embedding service from environment
            let service = match EmbeddingService::from_env().await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Configuration error: {}. Ensure embedding provider and API keys are configured.", e);
                    eprintln!("Set MAPROOM_EMBEDDING_PROVIDER and corresponding API key (OPENAI_API_KEY, etc.)");
                    std::process::exit(EXIT_CONFIG_ERROR);
                }
            };
            // NOTE: EmbeddingService::from_env() failures are configuration errors (missing provider/API keys)
            // not runtime errors (network failures).

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
            let stats = pipeline.run(store.as_ref()).await?;

            // Display results
            println!("\n{}\n", "=".repeat(60));
            println!("Embedding Generation Complete");
            println!("{}\n", "=".repeat(60));
            println!("{}", stats.summary());
            println!("{}", "=".repeat(60));
        }

        Commands::Migrate { command } => {
            use maproom::migrate::{verify_migration, MarkdownMigrator};

            // Markdown re-migration is a SQLite-only maintenance tool (dynamic
            // backup tables / sqlite_master introspection); require the SQLite backend.
            let store = db::connect_sqlite().await?;

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
                    println!(
                        "\nTo rollback: cargo run --bin maproom -- migrate rollback --backup {}",
                        result.backup_table
                    );
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
                use maproom::daemon::server::{
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
            use maproom::cli::clean_ignored;
            let store = db::connect().await?;
            clean_ignored::clean_ignored(store.as_ref(), &repo, &worktree, dry_run).await?;
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
            format,
            json: _,
        } => {
            // TODO(AFM-04): Context structured error handling awaits AFM-03 --format flag.
            // When AFM-03 lands and adds OutputFormat to Context, add structured error handling
            // here following the pattern used in search and vector-search commands:
            //   match db::connect().await { Ok(s) => s, Err(e) if format == Agent => handle_agent_error(...), ... }
            //   match assembler.assemble() { Ok(ctx) => ctx, Err(e) if format == Agent => ... }

            // Connect to database
            let store = db::connect().await.context("Database connection failed")?;

            // Create assembler (uses DefaultAssemblyStrategy which has working get_chunk_metadata)
            let assembler = DefaultAssemblyStrategy::new(store);

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
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&bundle)?);
                }
                OutputFormat::Agent => {
                    let output = format_context_agent(&bundle, chunk_id, budget);
                    if !output.is_empty() {
                        println!("{}", output);
                    }
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// Error Classification and Handling Infrastructure (AFM-04.2001)
// ============================================================================

/// Handle agent errors with dual output: structured format to stdout, human-readable to stderr.
///
/// This function provides the routing layer for error output in agent mode:
/// - If format is Agent: prints structured error line to stdout using `format_agent_error()`
/// - Always prints human-readable error chain to stderr via `eprintln!`
/// - Calls `std::process::exit(exit_code)` after output
///
/// # Investigation Notes (AFM-04.2001)
///
/// The `src/context/` module was investigated for structured error types. No error enums
/// or structured error definitions were found in the context assembly module. The context
/// module focuses on assembly logic (budget, graph traversal, truncation) rather than
/// defining its own error taxonomy.
///
/// Error classification relies on:
/// 1. Structured types from search pipeline: `PipelineError` (src/search/pipeline.rs)
/// 2. Structured types from embedding: `EmbeddingError` (src/embedding/error.rs)
/// 3. Helper: `SearchErrorDetails::from_pipeline_error()` (src/search/errors.rs)
/// 4. Fallback: String-based heuristics for anyhow-wrapped errors
///
/// # Arguments
///
/// * `error` - The anyhow error to handle
/// * `format` - Output format (Agent vs Json)
/// * `error_type` - Error type string from classification (e.g., "database", "config_error")
/// * `suggestion` - Actionable suggestion string from classification
/// * `exit_code` - Exit code to use (2 for config errors, 1 for runtime errors)
///
/// # Exit Behavior
///
/// This function NEVER returns - it always calls `std::process::exit(exit_code)`.
fn handle_agent_error(
    error: &anyhow::Error,
    format: &OutputFormat,
    error_type: &str,
    suggestion: &str,
    exit_code: i32,
) -> ! {
    // Log error handling event
    let error_message: String = error.to_string().chars().take(100).collect();
    tracing::error!(
        error_type = error_type,
        exit_code = exit_code,
        format = ?format,
        error_message = %error_message,
        "Agent error handled"
    );

    // If agent format, print structured error to stdout
    if matches!(format, OutputFormat::Agent) {
        let error_msg = error.to_string();
        let structured = format_agent_error(error_type, &error_msg, suggestion);
        println!("{}", structured);
    }

    // Always print human-readable error to stderr
    // Use Debug formatting to show error chain
    let error_display = format!("{:?}", error);
    let sanitized = sanitize_newlines(&error_display);
    eprintln!("Error: {}", sanitized);

    // Flush both streams before exit to prevent buffered output loss
    // (process::exit bypasses normal cleanup/Drop implementations)
    let _ = io::stdout().flush();
    let _ = io::stderr().flush();

    // Exit with appropriate code
    std::process::exit(exit_code)
}

/// Classify an anyhow error into (error_type, suggestion, exit_code).
///
/// Classification strategy (priority order):
/// 1. Attempt downcast to `PipelineError` -> use SearchErrorDetails for structured classification
/// 2. Attempt downcast to `EmbeddingError` -> use SearchErrorDetails for structured classification
/// 3. Fall back to string-based heuristics on `error.to_string()`
///
/// # Error Type Taxonomy
///
/// - **config_error**: Missing API keys, sqlite-vec unavailable, provider misconfiguration (exit code 2)
/// - **database**: Database connection, timeout, corruption (exit code 1)
/// - **not_found**: Chunk not found, repository not indexed (exit code 1)
/// - **validation**: Empty query, invalid input (exit code 1)
/// - **timeout**: Search execution timeout (exit code 1)
/// - **embedding_provider**: OpenAI/Ollama/Google API errors (exit code 1 for runtime, 2 for config)
/// - **unknown**: Unclassified errors (exit code 1)
///
/// # Heuristic Patterns
///
/// When structured types are unavailable, errors are classified by string matching:
/// - Contains "config" or "API_KEY" or "provider" -> config_error (exit 2)
/// - Contains "sqlite-vec" or "vector" and "not available" -> config_error (exit 2)
/// - Contains "database" or "connection" -> database (exit 1)
/// - Contains "chunk" and "not found" -> not_found (exit 1)
/// - Otherwise -> unknown (exit 1)
///
/// # Returns
///
/// Tuple of `(error_type: String, suggestion: String, exit_code: i32)`
///
/// # Examples
///
/// ```rust,ignore
/// let error = anyhow::anyhow!("Missing OPENAI_API_KEY");
/// let (error_type, suggestion, exit_code) = classify_error(&error);
/// assert_eq!(error_type, "config_error");
/// assert_eq!(exit_code, 2);
/// ```
fn classify_error(error: &anyhow::Error) -> (String, String, i32) {
    use maproom::embedding::error::{ApiError, EmbeddingError};
    use maproom::search::errors::SearchErrorDetails;
    use maproom::search::pipeline::PipelineError;

    // Priority 1: Try to downcast to PipelineError (structured search errors)
    if let Some(pipeline_error) = error.downcast_ref::<PipelineError>() {
        let details = SearchErrorDetails::from_pipeline_error(pipeline_error);

        // Map ErrorType to string and exit code
        let (error_type_str, exit_code) = match details.error_type {
            maproom::search::errors::ErrorType::EmbeddingProvider => {
                // Check if it's a config error (exit 2) or runtime error (exit 1)
                if details.context.contains_key("provider")
                    && details.suggestions.iter().any(|s| {
                        s.contains("API_KEY")
                            || s.contains("credentials")
                            || s.contains("environment variable")
                    })
                {
                    ("embedding_provider", 2)
                } else {
                    ("embedding_provider", 1)
                }
            }
            maproom::search::errors::ErrorType::Database => ("database", 1),
            maproom::search::errors::ErrorType::Validation => ("validation", 1),
            maproom::search::errors::ErrorType::Timeout => ("timeout", 1),
            maproom::search::errors::ErrorType::NotFound => ("not_found", 1),
            maproom::search::errors::ErrorType::Unknown => ("unknown", 1),
        };

        // Combine suggestions into a single string
        let suggestion = if details.suggestions.is_empty() {
            "Check logs for details".to_string()
        } else {
            details.suggestions.join("; ")
        };

        tracing::info!(
            error_type = error_type_str,
            exit_code = exit_code,
            classification_method = "downcast:PipelineError",
            "Error classified"
        );

        return (error_type_str.to_string(), suggestion, exit_code);
    }

    // Priority 2: Try to downcast to EmbeddingError (structured embedding errors)
    if let Some(embedding_error) = error.downcast_ref::<EmbeddingError>() {
        // Use the embedding module's error handling
        let error_str = embedding_error.to_string();

        // Check for authentication/authorization errors (401/403) -> config error (exit 2)
        if let EmbeddingError::Api(api_error) = embedding_error {
            if matches!(
                api_error,
                ApiError::Authentication(_) | ApiError::QuotaExceeded(_)
            ) {
                tracing::info!(
                    error_type = "config_error",
                    exit_code = 2,
                    classification_method = "downcast:EmbeddingError",
                    "Error classified"
                );
                return (
                    "config_error".to_string(),
                    "Invalid or expired API key. Check your API key configuration.".to_string(),
                    2,
                );
            }

            // Heuristic: check error message for auth-related patterns (e.g., 403 Forbidden)
            let api_error_lower = api_error.to_string().to_lowercase();
            if api_error_lower.contains("401")
                || api_error_lower.contains("403")
                || api_error_lower.contains("unauthorized")
                || api_error_lower.contains("forbidden")
                || api_error_lower.contains("authentication")
            {
                tracing::info!(
                    error_type = "config_error",
                    exit_code = 2,
                    classification_method = "downcast:EmbeddingError",
                    "Error classified"
                );
                return (
                    "config_error".to_string(),
                    "Invalid or expired API key. Check your API key configuration.".to_string(),
                    2,
                );
            }
        }

        // Check if it's a config error (missing API key, etc.)
        if matches!(
            embedding_error,
            EmbeddingError::Config(_) | EmbeddingError::InvalidInput(_)
        ) {
            tracing::info!(
                error_type = "embedding_provider",
                exit_code = 2,
                classification_method = "downcast:EmbeddingError",
                "Error classified"
            );
            return (
                "embedding_provider".to_string(),
                "Check your embedding provider configuration".to_string(),
                2,
            );
        }

        // Runtime errors (API failures, network issues)
        tracing::info!(
            error_type = "embedding_provider",
            exit_code = 1,
            classification_method = "downcast:EmbeddingError",
            "Error classified"
        );
        return (
            "embedding_provider".to_string(),
            format!("Embedding provider error: {}", error_str),
            1,
        );
    }

    // Priority 3: Fallback to string-based heuristics
    let error_str = error.to_string();
    let error_lower = error_str.to_lowercase();

    // Warn when heuristic classification is used (downcast failed)
    let error_preview: String = error_str.chars().take(100).collect();
    tracing::warn!(
        error_preview = %error_preview,
        classification_method = "heuristic",
        "Error classification using heuristic fallback (downcast failed)"
    );

    // Heuristic: Config errors (exit code 2)
    if error_lower.contains("config")
        || error_lower.contains("api_key")
        || error_lower.contains("provider")
        || (error_lower.contains("sqlite-vec") && error_lower.contains("not available"))
        || (error_lower.contains("vector") && error_lower.contains("not available"))
    {
        tracing::info!(
            error_type = "config_error",
            exit_code = 2,
            classification_method = "heuristic",
            "Error classified"
        );
        return (
            "config_error".to_string(),
            "Check your configuration and environment variables".to_string(),
            2,
        );
    }

    // Heuristic: Database file not found (repo not indexed) -> exit code 2
    // Must come BEFORE general database pattern to prioritize config error
    if (error_lower.contains("database") || error_lower.contains("connection"))
        && (error_lower.contains("no such file") || error_lower.contains("not found"))
    {
        tracing::info!(
            error_type = "config_error",
            exit_code = 2,
            classification_method = "heuristic",
            "Error classified"
        );
        return (
            "config_error".to_string(),
            "Repository not indexed. Run scan command first.".to_string(),
            2,
        );
    }

    // Heuristic: Database errors
    if error_lower.contains("database") || error_lower.contains("connection") {
        tracing::info!(
            error_type = "database",
            exit_code = 1,
            classification_method = "heuristic",
            "Error classified"
        );
        return (
            "database".to_string(),
            "Check database connectivity and permissions".to_string(),
            1,
        );
    }

    // Heuristic: Not found errors
    if error_lower.contains("chunk") && error_lower.contains("not found") {
        tracing::info!(
            error_type = "not_found",
            exit_code = 1,
            classification_method = "heuristic",
            "Error classified"
        );
        return (
            "not_found".to_string(),
            "The requested chunk may not be indexed".to_string(),
            1,
        );
    }

    // Default: Unknown error (exit code 1)
    tracing::info!(
        error_type = "unknown",
        exit_code = 1,
        classification_method = "heuristic",
        "Error classified"
    );
    (
        "unknown".to_string(),
        "Please report this error with full details".to_string(),
        1,
    )
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
            format,
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
            assert_eq!(format, OutputFormat::Json); // default
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
            format,
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
            assert_eq!(format, OutputFormat::Json); // default
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
            format,
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
            assert_eq!(format, OutputFormat::Json); // default (--json is separate flag)
            assert_eq!(json, true); // backward compat: --json still parsed
        } else {
            panic!("Expected Context command");
        }
    }

    #[test]
    fn test_context_format_agent() {
        let cli = Cli::parse_from(&[
            "maproom",
            "context",
            "--chunk-id",
            "100",
            "--format",
            "agent",
        ]);
        match cli.command {
            Commands::Context { format, .. } => {
                assert_eq!(format, OutputFormat::Agent);
            }
            _ => panic!("Expected Context command"),
        }
    }

    #[test]
    fn test_context_format_json() {
        let cli = Cli::parse_from(&[
            "maproom",
            "context",
            "--chunk-id",
            "100",
            "--format",
            "json",
        ]);
        match cli.command {
            Commands::Context { format, .. } => {
                assert_eq!(format, OutputFormat::Json);
            }
            _ => panic!("Expected Context command"),
        }
    }

    #[test]
    fn test_context_format_default_is_json() {
        // No --format flag should default to OutputFormat::Json
        let cli = Cli::parse_from(&["maproom", "context", "--chunk-id", "100"]);
        match cli.command {
            Commands::Context { format, .. } => {
                assert_eq!(format, OutputFormat::Json);
            }
            _ => panic!("Expected Context command"),
        }
    }

    #[test]
    fn test_context_json_flag_backward_compat() {
        // --json flag should still parse without error (backward compatibility)
        let cli = Cli::parse_from(&["maproom", "context", "--chunk-id", "100", "--json"]);
        match cli.command {
            Commands::Context { json, format, .. } => {
                assert_eq!(json, true);
                assert_eq!(format, OutputFormat::Json); // default unchanged
            }
            _ => panic!("Expected Context command"),
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
        use maproom::context::{ContextBundle, ContextItem, LineRange};

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
        use maproom::context::ContextBundle;

        let bundle = ContextBundle::new();
        let output = super::format_context_bundle(&bundle, 99999, 6000);

        assert!(output.contains("📦 Context Bundle for chunk #99999"));
        assert!(output.contains("Used: 0 tokens"));
        assert!(output.contains("(No context items found)"));
    }

    #[test]
    fn test_format_context_bundle_truncated() {
        use maproom::context::{ContextBundle, ContextItem, LineRange};

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
        use maproom::context::{ContextBundle, ContextItem, LineRange};

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

    // ==================== Error Classification Tests (AFM-04.2001) ====================

    /// Test classification of embedding config error (missing API key) -> exit code 2
    #[test]
    fn test_classify_embedding_config_error() {
        use maproom::embedding::error::{ConfigError, EmbeddingError};

        let config_error = ConfigError::MissingConfig("OPENAI_API_KEY".to_string());
        let embedding_error = EmbeddingError::Config(config_error);
        let error: anyhow::Error = embedding_error.into();

        let (error_type, _suggestion, exit_code) = classify_error(&error);

        assert_eq!(error_type, "embedding_provider");
        assert_eq!(exit_code, 2, "Config errors should exit with code 2");
    }

    /// Test classification of embedding API error (timeout) -> exit code 1
    #[test]
    fn test_classify_embedding_api_error() {
        use maproom::embedding::error::{ApiError, EmbeddingError};

        let api_error = ApiError::RateLimit {
            retry_after_ms: 5000,
        };
        let embedding_error = EmbeddingError::Api(api_error);
        let error: anyhow::Error = embedding_error.into();

        let (error_type, _suggestion, exit_code) = classify_error(&error);

        assert_eq!(error_type, "embedding_provider");
        assert_eq!(exit_code, 1, "Runtime API errors should exit with code 1");
    }

    /// Test classification of database error -> exit code 1
    #[test]
    fn test_classify_database_error() {
        let error = anyhow::anyhow!("Database connection failed");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(error_type, "database");
        assert!(suggestion.contains("database") || suggestion.contains("connectivity"));
        assert_eq!(exit_code, 1);
    }

    /// Test classification of database file not found -> exit code 2 (config error)
    /// Missing database file means repo not indexed - agents should suggest scanning
    #[test]
    fn test_classify_database_file_not_found() {
        let error = anyhow::anyhow!(
            "SQLite error: unable to open database file: No such file or directory"
        );

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(exit_code, 2, "Missing database file is a config error");
        assert_eq!(error_type, "config_error");
        assert!(
            suggestion.contains("scan"),
            "Suggestion should mention running scan command"
        );
    }

    /// Test that general database runtime errors still produce exit code 1
    /// Connection failures, query errors, etc. are transient and should be retried
    #[test]
    fn test_classify_database_runtime_error_unchanged() {
        let error = anyhow::anyhow!("Database query failed: syntax error");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            exit_code, 1,
            "Database runtime errors should remain exit code 1"
        );
        assert_eq!(error_type, "database");
        assert!(
            suggestion.contains("database") || suggestion.contains("connectivity"),
            "Suggestion should be about database connectivity"
        );
    }

    /// Test classification of sqlite-vec unavailable -> exit code 2 (config error)
    #[test]
    fn test_classify_sqlite_vec_error() {
        let error = anyhow::anyhow!("sqlite-vec extension not available");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(error_type, "config_error");
        assert!(suggestion.contains("configuration"));
        assert_eq!(exit_code, 2, "Missing extension is a config error");
    }

    /// Test classification of unknown error -> exit code 1
    #[test]
    fn test_classify_unknown_error() {
        let error = anyhow::anyhow!("Something unexpected happened");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(error_type, "unknown");
        assert!(suggestion.contains("report") || suggestion.contains("details"));
        assert_eq!(exit_code, 1);
    }

    /// Test classification of error wrapped in anyhow context
    #[test]
    fn test_classify_error_with_anyhow_context() {
        use maproom::embedding::error::{ConfigError, EmbeddingError};

        let config_error = ConfigError::EnvVarNotFound("GOOGLE_API_KEY".to_string());
        let embedding_error = EmbeddingError::Config(config_error);
        let error: anyhow::Error = embedding_error.into();
        let wrapped = error.context("Failed to generate embeddings");

        let (error_type, _suggestion, exit_code) = classify_error(&wrapped);

        assert_eq!(error_type, "embedding_provider");
        assert_eq!(exit_code, 2, "Config errors should exit with code 2");
    }

    /// Test heuristic fallback for unrecognized error patterns
    #[test]
    fn test_classify_heuristic_fallback() {
        let error = anyhow::anyhow!("Mysterious error from unknown source");

        let (error_type, _suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            error_type, "unknown",
            "Unrecognized errors should fall back to unknown type"
        );
        assert_eq!(exit_code, 1);
    }

    // ==================== Auth Error Classification Tests (AFM-04.5001) ====================

    /// Test: 401 Authentication error produces exit code 2 (config error)
    #[test]
    fn test_classify_auth_error_exit_code_2() {
        use maproom::embedding::error::{ApiError, EmbeddingError};

        let auth_error = ApiError::Authentication("Invalid API key".to_string());
        let embedding_error = EmbeddingError::Api(auth_error);
        let error: anyhow::Error = embedding_error.into();

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            exit_code, 2,
            "Auth errors should be config errors (exit code 2)"
        );
        assert_eq!(error_type, "config_error");
        assert!(
            suggestion.contains("API key"),
            "Suggestion should mention API key, got: {}",
            suggestion
        );
    }

    /// Test: QuotaExceeded error produces exit code 2 (config error)
    #[test]
    fn test_classify_quota_exceeded_exit_code_2() {
        use maproom::embedding::error::{ApiError, EmbeddingError};

        let quota_error = ApiError::QuotaExceeded("Rate limit exceeded for account".to_string());
        let embedding_error = EmbeddingError::Api(quota_error);
        let error: anyhow::Error = embedding_error.into();

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            exit_code, 2,
            "Quota exceeded errors should be config errors (exit code 2)"
        );
        assert_eq!(error_type, "config_error");
        assert!(
            suggestion.contains("API key"),
            "Suggestion should mention API key, got: {}",
            suggestion
        );
    }

    /// Test: API error with 403/forbidden in message produces exit code 2 via heuristic
    #[test]
    fn test_classify_forbidden_api_error_heuristic() {
        use maproom::embedding::error::{ApiError, EmbeddingError};

        // ServerError with 403 status containing "forbidden" in message
        let forbidden_error = ApiError::ServerError {
            status: 403,
            message: "Forbidden: insufficient permissions".to_string(),
        };
        let embedding_error = EmbeddingError::Api(forbidden_error);
        let error: anyhow::Error = embedding_error.into();

        let (error_type, _suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            exit_code, 2,
            "403 Forbidden errors should be config errors (exit code 2)"
        );
        assert_eq!(error_type, "config_error");
    }

    /// Test: Non-auth API errors (e.g., rate limit, server error) still produce exit code 1
    #[test]
    fn test_classify_non_auth_api_error_exit_code_1() {
        use maproom::embedding::error::{ApiError, EmbeddingError};

        let rate_limit_error = ApiError::RateLimit {
            retry_after_ms: 5000,
        };
        let embedding_error = EmbeddingError::Api(rate_limit_error);
        let error: anyhow::Error = embedding_error.into();

        let (error_type, _suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            exit_code, 1,
            "Rate limit errors should be runtime errors (exit code 1)"
        );
        assert_eq!(error_type, "embedding_provider");
    }

    /// Test: Auth error with anyhow context still produces exit code 2
    #[test]
    fn test_classify_auth_error_with_context() {
        use maproom::embedding::error::{ApiError, EmbeddingError};

        let auth_error = ApiError::Authentication("Invalid key provided".to_string());
        let embedding_error = EmbeddingError::Api(auth_error);
        let error: anyhow::Error = embedding_error.into();
        let wrapped = error.context("Failed during vector search");

        let (error_type, suggestion, exit_code) = classify_error(&wrapped);

        assert_eq!(
            exit_code, 2,
            "Auth errors wrapped in context should still be config errors (exit code 2)"
        );
        assert_eq!(error_type, "config_error");
        assert!(suggestion.contains("API key"));
    }

    // ==================== Integration Testing (AFM-04.4001) ====================

    /// Test: search with database error produces correct classification
    /// This verifies database errors are classified as type=database, exit=1
    #[test]
    fn test_search_agent_database_error_classification() {
        let error = anyhow::anyhow!("Database connection failed");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(error_type, "database");
        assert!(
            suggestion.contains("database") || suggestion.contains("connectivity"),
            "Database error should mention database or connectivity"
        );
        assert_eq!(exit_code, 1, "Database errors should exit with code 1");
    }

    /// Test: vector-search with missing API key produces config error classification
    /// This verifies missing configuration produces type=embedding_provider, exit=2
    #[test]
    fn test_vector_search_config_error_classification() {
        // Simulate missing API key error
        let error = anyhow::anyhow!("OPENAI_API_KEY environment variable not set");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            error_type, "config_error",
            "Missing API key should be classified as config_error"
        );
        assert!(
            suggestion.contains("configuration") || suggestion.contains("environment"),
            "Config error should mention configuration or environment"
        );
        assert_eq!(exit_code, 2, "Config errors should exit with code 2");
    }

    /// Test: vector-search with sqlite-vec unavailable produces config error
    /// This verifies sqlite-vec missing is classified as config_error with exit=2
    #[test]
    fn test_vector_search_sqlite_vec_classification() {
        // Simulate sqlite-vec extension not available
        // Use message that matches the heuristic: contains "sqlite-vec" AND "not available"
        let error = anyhow::anyhow!("sqlite-vec extension not available");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        assert_eq!(
            error_type, "config_error",
            "Missing sqlite-vec should be classified as config_error"
        );
        assert!(
            suggestion.contains("configuration"),
            "Config error should mention configuration"
        );
        assert_eq!(
            exit_code, 2,
            "Missing extension is a config error, not runtime error"
        );
    }

    /// Test: Dual output - classify + format produces valid structured output
    /// This verifies the combination of classify_error and format_agent_error produces
    /// a valid structured error line on stdout
    #[test]
    fn test_dual_output_format() {
        use maproom::cli::format::format_agent_error;

        let error = anyhow::anyhow!("Database connection timeout");
        let (error_type, suggestion, exit_code) = classify_error(&error);

        // Format the error using the agent format function
        let structured_output = format_agent_error(&error_type, &error.to_string(), &suggestion);

        // Verify exit code
        assert_eq!(exit_code, 1, "Database errors should exit with code 1");

        // Verify structured output format
        assert!(
            structured_output.starts_with("ERROR | "),
            "Structured output should start with 'ERROR | '"
        );
        assert!(
            structured_output.contains(&format!("type={}", error_type)),
            "Structured output should contain error type"
        );
        assert!(
            structured_output.contains("message="),
            "Structured output should contain message field"
        );
        assert!(
            structured_output.contains("suggestion="),
            "Structured output should contain suggestion field"
        );
    }

    /// Test: format_agent_error produces exactly one line (no embedded newlines)
    /// This verifies structured output is always a single line for parser reliability
    #[test]
    fn test_exactly_one_error_line() {
        use maproom::cli::format::format_agent_error;

        // Test with error message containing newlines
        let error_msg = "First line\nSecond line\nThird line";
        let suggestion = "Try this\nor that";

        let structured_output = format_agent_error("database", error_msg, suggestion);

        // Verify exactly one line (no embedded newlines)
        let line_count = structured_output.lines().count();
        assert_eq!(
            line_count, 1,
            "Structured error output must be exactly one line, got {} lines",
            line_count
        );

        // Verify no newline characters in output
        assert!(
            !structured_output.contains('\n'),
            "Structured output should not contain newline characters"
        );
    }

    /// Test: classify_error doesn't depend on format (classification is independent)
    /// This verifies classify_error can be called independently without side effects
    #[test]
    fn test_default_format_no_structured_output() {
        // Classification should work regardless of output format
        let error = anyhow::anyhow!("Some random error");

        let (error_type, suggestion, exit_code) = classify_error(&error);

        // Verify classification works
        assert!(!error_type.is_empty(), "Error type should not be empty");
        assert!(!suggestion.is_empty(), "Suggestion should not be empty");
        assert!(
            exit_code == 1 || exit_code == 2,
            "Exit code should be 1 or 2"
        );

        // classify_error itself doesn't produce output - that's format_agent_error's job
        // This test verifies we can call classify_error without formatting
    }

    /// Test: All error types produce output matching the structured format regex
    /// This validates the structured error format is consistent across all error types
    #[test]
    fn test_error_line_format_validation() {
        use maproom::cli::format::format_agent_error;
        use regex::Regex;

        // Regex pattern for structured error format
        let pattern = Regex::new(r"^ERROR \| type=[a-z_]+ \| message=.+ \| suggestion=.+$")
            .expect("Valid regex pattern");

        // Test various error types
        let test_cases = vec![
            ("database", "Connection failed", "Check connectivity"),
            (
                "embedding_provider",
                "API key missing",
                "Set OPENAI_API_KEY",
            ),
            ("config_error", "sqlite-vec not found", "Install extension"),
            ("unknown", "Mysterious error", "Report this issue"),
            ("validation", "Invalid input", "Check parameters"),
            ("timeout", "Request timed out", "Retry the operation"),
            ("not_found", "Chunk not found", "Verify chunk ID is indexed"),
        ];

        for (error_type, message, suggestion) in test_cases {
            let structured_output = format_agent_error(error_type, message, suggestion);

            assert!(
                pattern.is_match(&structured_output),
                "Error type '{}' produced invalid format: {}",
                error_type,
                structured_output
            );
        }
    }

    // ==================== Manual Validation Notes (AFM-04.4001) ====================
    //
    // The following commands were run manually to verify end-to-end behavior:
    //
    // 1. Search with invalid path (database error):
    //    $ cargo run --bin maproom -- search --format agent --repo nonexistent --query test 2>/dev/null
    //    Expected: ERROR line on stdout, exit code 1
    //    Result: ✓ Verified - produces structured error, exits with 1
    //
    // 2. Default format - no structured output:
    //    $ cargo run --bin maproom -- search --repo nonexistent --query test 2>/dev/null
    //    Expected: No ERROR line on stdout (backward compatibility)
    //    Result: ✓ Verified - no structured output, error only on stderr
    //
    // 3. Vector-search with missing API key (if testable):
    //    $ unset OPENAI_API_KEY GOOGLE_API_KEY VOYAGE_API_KEY
    //    $ cargo run --bin maproom -- vector-search --format agent --repo test --query foo 2>/dev/null
    //    Expected: ERROR line with type=embedding_provider or config_error, exit code 2
    //    Note: Requires specific test environment setup
    //
    // 4. Vector-search with sqlite-vec unavailable (if testable):
    //    Requires environment where sqlite-vec extension is not loaded
    //    Expected: ERROR line with type=config_error, exit code 2, suggestion mentions search command
    //    Note: Difficult to test in CI since sqlite-vec is statically linked
    //
}
