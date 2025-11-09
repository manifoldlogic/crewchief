# Ticket: BRWATCH-3001: Add watch command to CLI

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `maproom watch` CLI command that starts the branch watcher for automatic indexing on branch switches.

## Background
This ticket implements Step 3.1 from the implementation plan (plan.md - Phase 3). The CLI command provides the user-facing interface for BRWATCH, allowing developers to start the background watcher with:

```bash
maproom watch --repo /path/to/repo [--verbose]
```

From architecture.md lines 197-248, the command handles:
- Argument parsing (repo path, verbose flag)
- Database pool setup
- BranchWatcher initialization
- Logging configuration
- Long-running process management

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 3.1

## Acceptance Criteria
- [ ] `WatchArgs` struct added to CLI with `--repo` and `--verbose` flags
- [ ] `watch_command()` async function implemented
- [ ] Command registered in main CLI dispatcher
- [ ] Database pool initialized from DATABASE_URL
- [ ] BranchWatcher created and started
- [ ] Logging configured (info level default, debug if --verbose)
- [ ] Command runs until interrupted
- [ ] Help text clear and helpful
- [ ] Command compiles and runs without errors

## Technical Requirements
- Add to `/workspace/crates/maproom/src/cli.rs` (or create if needed)
- Use clap for argument parsing
- Import BranchWatcher from watcher module
- Get DATABASE_URL from environment
- Use env_logger for logging configuration
- Handle database connection errors gracefully
- Run watcher in foreground (blocking)
- Return proper exit codes

## Implementation Notes

From architecture.md lines 197-248:

```rust
use clap::{Args, Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "maproom")]
#[command(about = "Semantic code search and indexing")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Watch for branch switches and auto-index
    Watch(WatchArgs),
    // ... other commands
}

#[derive(Args)]
pub struct WatchArgs {
    /// Path to git repository
    #[arg(long)]
    repo: PathBuf,

    /// Show verbose logging
    #[arg(short, long)]
    verbose: bool,
}

async fn watch_command(args: WatchArgs) -> Result<()> {
    // Setup logging
    if args.verbose {
        env::set_var("RUST_LOG", "maproom=debug");
    } else {
        env::set_var("RUST_LOG", "maproom=info");
    }
    env_logger::init();

    info!("Starting branch watcher for {}", args.repo.display());

    // Validate repository path
    if !args.repo.join(".git/HEAD").exists() {
        anyhow::bail!("Not a git repository: {}", args.repo.display());
    }

    // Setup database
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    info!("Connected to database");

    // Create and start watcher
    let mut watcher = BranchWatcher::new(args.repo, pool)?;

    info!("Watching for branch switches (Ctrl+C to stop)");

    // Run watcher (blocks until error or shutdown)
    watcher.start().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Watch(args) => watch_command(args).await?,
        // ... other commands
    }

    Ok(())
}
```

### Usage Examples

```bash
# Start watching repository
$ maproom watch --repo /path/to/myproject

[INFO] Starting branch watcher for /path/to/myproject
[INFO] Connected to database
[INFO] Watching /path/to/myproject/.git/HEAD for branch switches
[INFO] Indexing current branch...
[INFO] Branch switch detected: main
[INFO] Index updated in 0.1s:
[INFO]   Files processed: 0
[INFO]   Chunks processed: 0
[INFO]   Cache hit rate: 100.0%
[INFO] Waiting for changes...

# Verbose mode
$ maproom watch --repo /path/to/myproject --verbose

[DEBUG] Initializing file watcher
[DEBUG] Creating database pool
[DEBUG] Watching .git/HEAD
[INFO] Branch switch detected: feature-auth
[DEBUG] Extracting branch name from HEAD
[DEBUG] Creating worktree record
[INFO] Index updated in 45.2s:
...
```

### Error Handling

```rust
async fn watch_command(args: WatchArgs) -> Result<()> {
    // Validate repository
    if !args.repo.exists() {
        eprintln!("Error: Repository path does not exist: {}", args.repo.display());
        std::process::exit(1);
    }

    if !args.repo.join(".git").exists() {
        eprintln!("Error: Not a git repository: {}", args.repo.display());
        eprintln!("Expected .git directory in: {}", args.repo.display());
        std::process::exit(1);
    }

    // Database connection
    let database_url = match env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            eprintln!("Error: DATABASE_URL environment variable not set");
            eprintln!("Example: export DATABASE_URL=postgresql://user:pass@localhost/db");
            std::process::exit(1);
        }
    };

    let pool = match PgPool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Error: Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    // ... rest of command
}
```

## Dependencies
- BRWATCH-1002 complete (BranchWatcher struct)
- BRWATCH-2001 complete (handle_branch_switch)
- BRWATCH-2002 complete (error handling)
- clap crate added to dependencies (if not already present)
- env_logger crate added to dependencies

## Risk Assessment
- **Risk**: Database URL not configured in environment
  - **Mitigation**: Clear error message with example, check before starting watcher
- **Risk**: Repository path invalid
  - **Mitigation**: Validate path exists and has .git directory before starting
- **Risk**: Permission errors accessing .git/HEAD
  - **Mitigation**: File watcher will error with clear message, suggest checking permissions

## Files/Packages Affected
- `/workspace/crates/maproom/src/cli.rs` (add watch command)
- `/workspace/crates/maproom/src/main.rs` (register command if separate file)
- `/workspace/crates/maproom/Cargo.toml` (add clap, env_logger if needed)

## Implementation Notes

### Naming Decision
The command was implemented as `branch-watch` instead of `watch` because there was already an existing `Watch` command in the CLI (line 124-137 of main.rs) that performs file watching for incremental updates within a worktree. To avoid naming conflicts and maintain both functionalities:

- **`watch`** - Watches files in a worktree for changes (existing functionality)
- **`branch-watch`** - Watches for git branch switches (new functionality from BRWATCH)

### Implementation Details
- Command variant: `BranchWatch` in Commands enum (line 139-148)
- Handler function: `branch_watch_command()` (line 342-393)
- Arguments: `--repo` (optional PathBuf), `--verbose` (boolean flag)
- Database: Uses `db::connect()` which reads DATABASE_URL from environment
- Logging: Configured via RUST_LOG environment variable (debug level if --verbose)
- Error handling: Comprehensive error context for validation and database connection
- Validation: Checks repository path exists and contains .git/HEAD before starting

### Usage
```bash
# Watch current directory
crewchief-maproom branch-watch

# Watch specific repository
crewchief-maproom branch-watch --repo /path/to/repo

# Verbose logging
crewchief-maproom branch-watch --repo /path/to/repo --verbose
```
