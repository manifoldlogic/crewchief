# Ticket: IDXABS-3001: Clean Up main.rs

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Binary should compile: `cargo build --bin crewchief-maproom`
- Commands should run (may fail at runtime if tests aren't updated yet)
- Focus on compilation and removal of backend switching

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Remove all backend switching logic and SQLite blockers from `main.rs`, updating all command handlers to use `db::connect()`.

## Background
The main.rs file currently contains backend type detection, SQLite blockers (rejecting commands for SQLite), and the `--parallel` flag. With SQLite as the only backend, all this complexity can be removed.

**Reference**: Phase 3, Ticket 3001 of `planning/plan.md` - "Clean Up main.rs"
**Architecture**: See `planning/architecture.md` - Section 4.5 "main.rs"

## Acceptance Criteria
- [ ] `BackendType` enum usage removed
- [ ] `get_store_with_type()` function removed
- [ ] All `if backend_type == BackendType::SQLite` blocks removed
- [ ] All command handlers use `db::connect()` (returns SqliteStore)
- [ ] `--parallel` flag removed or made a no-op
- [ ] `auto_generate_embeddings()` uses SqliteStore
- [ ] `scan` command works
- [ ] `upsert` command works
- [ ] `watch` command works
- [ ] `generate-embeddings` command works
- [ ] `search` command works
- [ ] Binary compiles: `cargo build --bin crewchief-maproom`

## Technical Requirements
- Remove all backend type switching logic
- Update command handlers to call refactored functions:
  - `indexer::scan_worktree(&store, ...)`
  - `indexer::upsert_files(&store, ...)`
  - `embedding::pipeline::run(&store, ...)`
  - `search::search(&store, ...)`
- Remove blockers that prevented SQLite from being used for indexing

## Implementation Notes

### Code to Remove
```rust
// DELETE these from main.rs:

// 1. BackendType usage
let (store, backend_type) = get_store_with_type().await?;

// 2. SQLite blockers
if backend_type == BackendType::SQLite {
    anyhow::bail!("Scan command not supported with SQLite backend");
}

// 3. Parallel flag handling
#[arg(long)]
parallel: bool,

if parallel {
    indexer::scan_worktree_parallel(&pool, ...).await?;
} else {
    indexer::scan_worktree(&client, ...).await?;
}
```

### Target Code Pattern
```rust
// Simplified command handler:
Commands::Scan { path, repo, worktree } => {
    let store = db::connect().await?;
    let stats = indexer::scan_worktree(&store, repo, worktree, &path).await?;
    println!("Indexed {} files, {} chunks", stats.files, stats.chunks);
}

Commands::Upsert { paths, repo, worktree } => {
    let store = db::connect().await?;
    indexer::upsert_files(&store, repo, worktree, &paths).await?;
}

Commands::Watch { path, repo, worktree } => {
    let store = db::connect().await?;
    indexer::watch_worktree(&store, repo, worktree, &path).await?;
}

Commands::GenerateEmbeddings { model } => {
    let store = db::connect().await?;
    let pipeline = EmbeddingPipeline::new(model);
    pipeline.run(&store).await?;
}

Commands::Search { query, repo, worktree, limit } => {
    let store = db::connect().await?;
    let results = search::search(&store, query, repo, worktree, limit).await?;
    // ... display results
}
```

### CLI Argument Updates
```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Scan {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        repo: String,
        #[arg(long)]
        worktree: Option<String>,
        // REMOVE: #[arg(long)] parallel: bool,
    },
    // ... other commands
}
```

### Verification
```bash
# Build binary
cargo build --bin crewchief-maproom

# Test commands (will fail without database, but should not crash)
cargo run --bin crewchief-maproom -- --help
cargo run --bin crewchief-maproom -- scan --help

# Verify no backend type references
grep -n "BackendType\|get_store_with_type\|parallel" crates/maproom/src/main.rs
# Should return nothing (or very minimal)
```

## Dependencies
- IDXABS-1001 through IDXABS-2005 (all Phase 1 and 2 tickets)

## Risk Assessment
- **Risk**: Command handlers have complex logic beyond database calls
  - **Mitigation**: Focus on database-related changes only
  - **Mitigation**: Keep non-database logic intact
- **Risk**: Environment variable handling differs
  - **Mitigation**: `db::connect()` handles env vars internally
  - **Mitigation**: Test with MAPROOM_DATABASE_URL set and unset
- **Risk**: Progress/output handling needs updates
  - **Mitigation**: Progress callbacks should be database-agnostic

## Files/Packages Affected
Files to MODIFY:
- `crates/maproom/src/main.rs` - Major refactoring

Items to REMOVE:
- `BackendType` imports and usage
- `get_store_with_type()` calls
- `--parallel` flag
- All SQLite blocker checks
- Any `Arc<dyn VectorStore>` usage (use concrete `SqliteStore`)
