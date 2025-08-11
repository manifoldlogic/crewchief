use std::path::PathBuf;

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
        #[arg(long)]
        repo: String,
        #[arg(long)]
        worktree: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        commit: String,
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
        #[arg(long)]
        repo: String,
        #[arg(long)]
        worktree: String,
        #[arg(long)]
        path: PathBuf,
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
            tracing::warn!(?repo, ?worktree, ?path, ?throttle, "watch is not yet implemented; exiting");
        }

        Commands::Search { repo, worktree, query, k } => {
            let client = db::connect().await?;
            let hits = db::search_chunks_fts(&client, &repo, worktree.as_deref(), &query, k).await?;
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({"hits": hits}))?);
        }
    }

    Ok(())
}


