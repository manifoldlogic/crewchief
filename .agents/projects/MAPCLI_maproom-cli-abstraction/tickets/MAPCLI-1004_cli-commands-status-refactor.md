# Ticket: MAPCLI-1004: Update CLI Commands and Refactor status.rs

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update CLI commands (search, vector-search, cleanup-stale) to use VectorStore trait methods, and **critically refactor `status.rs` to remove its direct PostgreSQL connection** and use the VectorStore abstraction instead.

## Background
The `status.rs` module (lines 28-34) creates its own PostgreSQL connection via `tokio_postgres::connect()`, completely bypassing the factory pattern. It also uses PostgreSQL-specific JSONB operators (`@>`) that don't work with SQLite. This is a critical blocker for SQLite support.

Additionally, CLI commands like `search`, `vector-search`, and `db cleanup-stale` need to be updated to use VectorStore trait methods instead of direct database calls.

**Plan Reference**: Phase 1: CLI Command Updates (MAPCLI-1004) in plan.md

## Acceptance Criteria
- [ ] `search` command works with SQLite backend using `store.search_chunks_fts()`
- [ ] `vector-search` command works with SQLite (or shows graceful fallback message)
- [ ] **`status` command works with SQLite backend** (critical refactor complete)
- [ ] **`status.rs` no longer creates its own PostgreSQL connection** (no `tokio_postgres::connect()`)
- [ ] **`status.rs` no longer uses JSONB operators** (no `@>` or `jsonb_build_array`)
- [ ] `db cleanup-stale` detects stale worktrees using `store.detect_stale_worktrees()`
- [ ] `db migrate` skips for SQLite with informative message
- [ ] scan/upsert/watch show helpful "Phase 2" error for SQLite

## Technical Requirements
- Refactor `status.rs` to accept `Arc<dyn VectorStore>` parameter
- Use `store.list_repos()` and `store.list_worktrees()` for status data
- Remove all direct `tokio_postgres` imports from `status.rs`
- Update command handlers in main.rs to pass store to status functions
- Handle vector search fallback when sqlite-vec is unavailable

## Implementation Notes

### CRITICAL: status.rs Refactoring

#### Current Implementation (BROKEN for SQLite)
```rust
// status.rs lines 28-34
pub async fn get_status(...) -> Result<StatusResponse> {
    let database_url = env::var("MAPROOM_DATABASE_URL")?;
    let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
    tokio::spawn(async move { if let Err(e) = connection.await { ... } });

    // PostgreSQL-specific query with JSONB operators
    let rows = client.query(r#"
        SELECT r.name, w.name, w.root_path,
               COUNT(DISTINCT c.id) FILTER (WHERE c.worktree_ids @> jsonb_build_array(w.id))
        ...
    "#, &[]).await?;
}
```

#### New Implementation
```rust
// status.rs
use crate::db::VectorStore;
use std::sync::Arc;

pub async fn get_status(
    store: Arc<dyn VectorStore>,
    repo_filter: Option<&str>,
    worktree_filter: Option<&str>,
) -> Result<StatusResponse> {
    // Use trait methods instead of direct queries
    let repos = store.list_repos().await?;

    let mut repo_statuses = Vec::new();
    for repo in repos {
        if let Some(filter) = repo_filter {
            if repo.name != filter { continue; }
        }

        let worktrees = store.list_worktrees(&repo.name).await?;
        let mut worktree_statuses = Vec::new();

        for worktree in worktrees {
            if let Some(filter) = worktree_filter {
                if worktree.name != filter { continue; }
            }
            worktree_statuses.push(WorktreeStatus {
                name: worktree.name.clone(),
                root_path: worktree.root_path.clone(),
                // Note: chunk_count may be unavailable for SQLite MVP
                chunk_count: None, // Or use worktree metadata if available
            });
        }

        repo_statuses.push(RepoStatus {
            name: repo.name.clone(),
            worktrees: worktree_statuses,
        });
    }

    Ok(StatusResponse { repos: repo_statuses })
}
```

### Update main.rs status command handler
```rust
Commands::Status { repo, worktree, json } => {
    let store = get_store().await?;
    let status = status::get_status(
        store,
        repo.as_deref(),
        worktree.as_deref(),
    ).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        // Format as table...
    }
}
```

### Update search command
```rust
Commands::Search { repo, query, limit, worktree, debug } => {
    let store = get_store().await?;
    let hits = store.search_chunks_fts(
        &repo,
        worktree.as_deref(),
        &query,
        limit as i64,
        debug
    ).await?;

    for hit in hits {
        println!("{}:{}:{}", hit.file_path, hit.start_line, hit.symbol_name.unwrap_or_default());
        if debug {
            println!("  Score: {:.4}", hit.score);
        }
    }
}
```

### Update vector-search command with fallback
```rust
Commands::VectorSearch { repo, query, limit, worktree } => {
    let store = get_store().await?;

    match store.search_chunks_vector(&repo, worktree.as_deref(), &query, limit as i64).await {
        Ok(hits) => {
            for hit in hits {
                println!("{}:{}:{} (score: {:.4})", hit.file_path, hit.start_line,
                         hit.symbol_name.unwrap_or_default(), hit.score);
            }
        }
        Err(e) if e.to_string().contains("sqlite-vec") || e.to_string().contains("vector") => {
            eprintln!("Vector search unavailable: {}", e);
            eprintln!("Tip: Use 'search' command for full-text search instead");
        }
        Err(e) => return Err(e),
    }
}
```

### Update db cleanup-stale command
```rust
Commands::Db { command: DbCommand::CleanupStale { dry_run } } => {
    let store = get_store().await?;
    let stale = store.detect_stale_worktrees().await?;

    if stale.is_empty() {
        println!("No stale worktrees detected");
        return Ok(());
    }

    println!("Found {} stale worktrees:", stale.len());
    for worktree in &stale {
        println!("  - {} ({})", worktree.name, worktree.root_path);
    }

    if !dry_run {
        for worktree in stale {
            store.remove_worktree(&worktree.repo_name, &worktree.name).await?;
            println!("Removed: {}", worktree.name);
        }
    } else {
        println!("\nDry run - no changes made. Use --no-dry-run to remove.");
    }
}
```

### Commands Summary Table
| Command | Implementation | SQLite Support |
|---------|---------------|----------------|
| `search` | `store.search_chunks_fts()` | ✅ MVP |
| `vector-search` | `store.search_chunks_vector()` with fallback | ✅ MVP (with fallback) |
| `status` | Refactored to use `store.list_repos()`, `store.list_worktrees()` | ✅ MVP |
| `db cleanup-stale` | `store.detect_stale_worktrees()` | ✅ MVP |
| `db migrate` | Check backend type, skip for SQLite | ✅ MVP |
| `scan` | Graceful error for SQLite | ⏸️ Phase 2 |
| `upsert` | Graceful error for SQLite | ⏸️ Phase 2 |
| `watch` | Graceful error for SQLite | ⏸️ Phase 2 |

## Dependencies
- **MAPCLI-1001**: main.rs factory pattern must be established
- **MAPCLI-1000**: BackendType enum (indirect, through 1001)

## Risk Assessment
- **Risk**: Status output format changes
  - **Mitigation**: Keep same output structure; chunk_count can be optional/null for SQLite
- **Risk**: Breaking status command for PostgreSQL users
  - **Mitigation**: Test PostgreSQL path before and after; trait methods work for both backends
- **Risk**: Vector search silently failing
  - **Mitigation**: Explicit error message with suggestion to use FTS instead

## Files/Packages Affected
- `crates/maproom/src/status.rs` - **COMPLETE REFACTOR**
  - Remove `tokio_postgres` imports
  - Change function signature to accept `Arc<dyn VectorStore>`
  - Replace SQL queries with trait method calls
  - Remove JSONB operator usage
- `crates/maproom/src/main.rs` - Update command handlers
  - Update status command to pass store
  - Update search command to use trait methods
  - Update vector-search with fallback
  - Update db cleanup-stale

## Testing
```bash
# Test status with PostgreSQL
MAPROOM_DATABASE_URL="postgresql://..." cargo run --bin crewchief-maproom -- status

# Test status with SQLite
MAPROOM_DATABASE_URL="sqlite:///tmp/test.db" cargo run --features sqlite --bin crewchief-maproom -- status

# Test search with both backends
cargo run --bin crewchief-maproom -- search --repo test --query "function"
cargo run --features sqlite --bin crewchief-maproom -- search --repo test --query "function"

# Test vector-search fallback on SQLite without sqlite-vec
cargo run --features sqlite --bin crewchief-maproom -- vector-search --repo test --query "function"
# Should show: "Vector search unavailable... Use 'search' command..."

# Test cleanup-stale
cargo run --features sqlite --bin crewchief-maproom -- db cleanup-stale --dry-run

# Verify status.rs no longer imports tokio_postgres
grep -r "tokio_postgres" crates/maproom/src/status.rs  # Should return nothing

# Run all tests
cargo test
cargo test --features sqlite
```
