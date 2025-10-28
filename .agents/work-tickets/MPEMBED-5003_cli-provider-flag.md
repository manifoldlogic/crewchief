# Ticket: MPEMBED-5003: Add --provider CLI flag to Rust binary

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- rust-test-runner
- verify-ticket
- commit-ticket

## Summary
Add --provider CLI flag to scan and upsert commands in the Rust binary. Validate provider name and pass to embedding service factory.

## Background
This ticket implements the Rust-side support for the --provider flag added by the MCP TypeScript wrapper in MPEMBED-5002. The Rust binary needs to accept the provider flag, validate it, and pass it to the embedding provider factory.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-5-mcp-documentation.md

## Acceptance Criteria
- [ ] --provider flag added to scan command
- [ ] --provider flag added to upsert command
- [ ] Flag validates provider name (ollama, openai, google)
- [ ] Flag overrides EMBEDDING_PROVIDER env var if both set
- [ ] Provider passed to create_provider() factory
- [ ] Help text updated with provider options
- [ ] Error message for invalid provider name
- [ ] Unit tests for CLI parsing
- [ ] Integration test with --provider flag

## Technical Requirements
- Use clap for CLI argument parsing
- Make --provider optional (defaults to env var)
- Validate against known providers: ["ollama", "openai", "google"]
- Pass provider to embedding service initialization
- Maintain backward compatibility (works without --provider)
- Update CLI help documentation
- Log provider selection for debugging

## Implementation Notes
```rust
// crates/maproom/src/main.rs
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "crewchief-maproom")]
#[clap(about = "Semantic code search and indexing")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Scan {
        #[clap(long)]
        repo: String,

        #[clap(long)]
        worktree: String,

        #[clap(long)]
        root: PathBuf,

        /// Embedding provider: ollama, openai, google
        #[clap(long, value_parser = validate_provider)]
        provider: Option<String>,

        #[clap(long)]
        generate_embeddings: bool,
    },
    Upsert {
        #[clap(long)]
        repo: String,

        #[clap(long)]
        worktree: String,

        #[clap(long)]
        root: PathBuf,

        #[clap(long)]
        commit: String,

        /// Embedding provider: ollama, openai, google
        #[clap(long, value_parser = validate_provider)]
        provider: Option<String>,

        #[clap(long)]
        paths: Vec<PathBuf>,
    },
    // ... other commands
}

fn validate_provider(s: &str) -> Result<String, String> {
    match s {
        "ollama" | "openai" | "google" => Ok(s.to_string()),
        _ => Err(format!(
            "Invalid provider: '{}'. Supported: ollama, openai, google",
            s
        )),
    }
}

async fn run_scan(
    repo: String,
    worktree: String,
    root: PathBuf,
    provider: Option<String>,
    generate_embeddings: bool,
    pool: PgPool,
) -> Result<()> {
    // Determine provider (CLI flag overrides env var)
    let provider_name = provider
        .or_else(|| std::env::var("EMBEDDING_PROVIDER").ok())
        .unwrap_or_else(|| "openai".to_string()); // Default fallback

    tracing::info!(
        "Starting scan: repo={}, worktree={}, provider={}",
        repo,
        worktree,
        provider_name
    );

    // Create provider
    let embedding_provider = create_provider(&provider_name)
        .context(format!("Failed to create provider: {}", provider_name))?;

    tracing::info!(
        "Using embedding provider: {} ({} dimensions)",
        embedding_provider.name(),
        embedding_provider.dimension()
    );

    // Continue with scan...
    let scanner = Scanner::new(pool.clone());
    let scan_result = scanner.scan(&root, &repo, &worktree).await?;

    if generate_embeddings {
        let pipeline = EmbeddingPipeline::new(embedding_provider, pool);
        let stats = pipeline.process_chunks(scan_result.chunk_ids).await?;
        println!("{}", stats);
    }

    Ok(())
}

async fn run_upsert(
    repo: String,
    worktree: String,
    root: PathBuf,
    commit: String,
    provider: Option<String>,
    paths: Vec<PathBuf>,
    pool: PgPool,
) -> Result<()> {
    let provider_name = provider
        .or_else(|| std::env::var("EMBEDDING_PROVIDER").ok())
        .unwrap_or_else(|| "openai".to_string());

    tracing::info!(
        "Starting upsert: repo={}, paths={}, provider={}",
        repo,
        paths.len(),
        provider_name
    );

    let embedding_provider = create_provider(&provider_name)?;

    let upserter = Upserter::new(pool.clone());
    let upsert_result = upserter.upsert(&root, &repo, &worktree, &commit, &paths).await?;

    let pipeline = EmbeddingPipeline::new(embedding_provider, pool);
    let stats = pipeline.process_chunks(upsert_result.chunk_ids).await?;
    println!("{}", stats);

    Ok(())
}
```

## Dependencies
- MPEMBED-2004 (Provider factory must exist)

## Risk Assessment
- **Risk**: CLI flag may conflict with future flags
  - **Mitigation**: Use explicit --provider name, avoid abbreviations

## Files/Packages Affected
- crates/maproom/src/main.rs (modify - add CLI flags)
- crates/maproom/tests/cli_test.rs (create)
