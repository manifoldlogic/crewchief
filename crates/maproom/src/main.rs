use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use clap::{Parser, Subcommand};
use dotenvy::dotenv;
use tracing_subscriber::{fmt, EnvFilter};

mod db;
mod indexer;

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

    /// Scan a worktree and index files into Postgres
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
        exclude: Option<Vec<String>>,   // glob patterns
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
}

#[derive(Subcommand, Debug)]
enum DbCommand {
    /// Apply SQL migrations to the configured database
    Migrate,
}

/// Extract git information from a repository path
fn get_git_info(path: &Path) -> anyhow::Result<(String, String, String)> {
    // Get the repository name from remote origin
    let repo_name = Command::new("git")
        .args(&["-C", path.to_str().unwrap_or("."), "remote", "get-url", "origin"])
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
        .args(&["-C", path.to_str().unwrap_or("."), "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
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
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
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

        Commands::Scan { repo, worktree, path, commit, concurrency, languages, exclude } => {
            // Get git defaults if not provided
            let path = path.unwrap_or_else(|| PathBuf::from("."));
            
            // Get git information from the path
            let (repo_name, branch_name, commit_hash) = get_git_info(&path)?;
            
            let repo = repo.unwrap_or(repo_name);
            let worktree = worktree.unwrap_or(branch_name);
            let commit = commit.unwrap_or(commit_hash);
            
            tracing::info!("Scanning repo: {}, worktree: {}, commit: {}", repo, worktree, commit);
            
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
            )
            .await
            .with_context(|| format!("scan failed for {}@{}", worktree, commit))?;
        }

        Commands::Upsert { paths, commit, repo, worktree, root } => {
            let client = db::connect().await?;
            indexer::upsert_files(&client, &repo, &worktree, &root, &commit, &paths)
                .await
                .with_context(|| "upsert failed")?;
        }

        Commands::Watch { repo, worktree, path, throttle } => {
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

        Commands::Search { repo, worktree, query, k } => {
            let client = db::connect().await?;
            let hits = db::search_chunks_fts(&client, &repo, worktree.as_deref(), &query, k).await?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({"hits": hits}))?);
        }
    }

    Ok(())
}


