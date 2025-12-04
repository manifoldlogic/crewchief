# MRMIGNR-2001: Add clean-ignored CLI Command

**Status:** 🔴 Not Started
**Phase:** 2 (Maintenance Features)
**Complexity:** Medium
**Estimate:** 4-6 hours

## Overview

Implement a `clean-ignored` CLI command that deletes indexed chunks matching patterns in `.maproomignore`. This provides users a way to remove noise from search results after adding patterns to `.maproomignore`, without requiring a full rescan.

## Context

After implementing `.maproomignore` support in Phase 1, users can prevent new files from being indexed. However, files that were already indexed before patterns were added remain in the database. This ticket adds a maintenance command to clean up those stale entries.

**User Story:**
"As a developer, when I add entries to `.maproomignore` to reduce noise in search results, I want a simple command to delete already-indexed chunks matching those patterns, so I don't have to wait for a full rescan."

## Acceptance Criteria

- [x] **Task completed** - CLI command implementation finished
- [x] **Tests pass** - All unit and integration tests pass
- [x] **Verified** - verify-ticket agent confirms all requirements met
- [ ] **Committed** - Changes committed to repository

### Functional Requirements

- [x] `crewchief-maproom clean-ignored` command exists in CLI
- [x] Command accepts `--repo <name>` and `--worktree <name>` flags (required)
- [x] Command loads `.maproomignore` patterns from repository root
- [x] Command deletes chunks where `relpath` matches any pattern
- [x] Command outputs count of deleted chunks
- [x] Command handles missing `.maproomignore` gracefully (no-op, informative message)
- [x] Command returns exit code 0 on success, non-zero on error

### Technical Requirements

- [x] New subcommand in `crates/maproom/src/main.rs`
- [x] Implementation module at `crates/maproom/src/cli/clean_ignored.rs`
- [x] Database method `delete_chunks_by_ids()` in `crates/maproom/src/db/sqlite/mod.rs`
- [x] Reuses `load_ignore_patterns()` from `crates/maproom/src/incremental/ignore.rs`
- [x] Uses `IgnorePatternMatcher` to test paths against patterns
- [x] Proper error handling for database errors and invalid patterns

### Testing Requirements

- [x] Unit test: `test_delete_chunks_by_ids()` - database method works correctly
- [x] Unit test: `test_clean_ignored_missing_file()` - handles missing `.maproomignore`
- [x] Unit test: `test_clean_ignored_empty_file()` - handles empty `.maproomignore`
- [x] Integration test: `test_clean_ignored_removes_matching_chunks()` - end-to-end CLI test
- [x] Integration test: `test_clean_ignored_preserves_non_matching()` - doesn't delete wrong chunks

### Documentation Requirements

- [x] Help text for `clean-ignored` command added
- [x] CLAUDE.md section documenting the command with examples
- [x] Example workflow: add pattern → run clean-ignored → verify with search

## Implementation Plan

### 1. Add CLI Subcommand (`crates/maproom/src/main.rs`)

```rust
#[derive(Parser)]
pub enum SubCommand {
    // ... existing commands

    /// Delete indexed chunks matching patterns in .maproomignore
    #[command(name = "clean-ignored")]
    CleanIgnored {
        /// Repository name
        #[arg(long, required = true)]
        repo: String,

        /// Worktree name
        #[arg(long, required = true)]
        worktree: String,

        /// Dry run - show what would be deleted without deleting
        #[arg(long, default_value = "false")]
        dry_run: bool,
    },
}
```

### 2. Implementation Module (`crates/maproom/src/cli/clean_ignored.rs`)

```rust
use crate::config::Config;
use crate::db::DatabaseHandle;
use crate::incremental::ignore::{load_ignore_patterns, IgnorePatternMatcher};
use anyhow::{Context, Result};
use tracing::{info, warn};

pub async fn clean_ignored(
    config: &Config,
    db: &DatabaseHandle,
    repo_name: &str,
    worktree_name: &str,
    dry_run: bool,
) -> Result<()> {
    // 1. Resolve repository and worktree IDs
    let repo_id = db
        .get_repository_by_name(repo_name)
        .await
        .context("Failed to get repository")?
        .ok_or_else(|| anyhow::anyhow!("Repository '{}' not found", repo_name))?
        .id;

    let worktree = db
        .get_worktree_by_name(repo_id, worktree_name)
        .await
        .context("Failed to get worktree")?
        .ok_or_else(|| anyhow::anyhow!("Worktree '{}' not found", worktree_name))?;

    // 2. Load .maproomignore patterns
    let root = PathBuf::from(&worktree.path);
    let patterns = load_ignore_patterns(&root)?;

    if patterns.is_empty() {
        info!("No patterns in .maproomignore, nothing to clean");
        return Ok(());
    }

    // 3. Create pattern matcher
    let matcher = IgnorePatternMatcher::from_patterns(&patterns)
        .context("Failed to compile ignore patterns")?;

    // 4. Get all chunks for this worktree
    let chunks = db.get_chunks_by_worktree(worktree.id).await?;

    // 5. Filter chunks that match patterns
    let mut to_delete = Vec::new();
    for chunk in chunks {
        if matcher.should_ignore(&chunk.relpath) {
            to_delete.push(chunk.id);
        }
    }

    // 6. Delete or report
    if dry_run {
        info!("Dry run: would delete {} chunks", to_delete.len());
        for chunk_id in to_delete {
            let chunk = db.get_chunk(chunk_id).await?;
            info!("  Would delete: {}", chunk.relpath);
        }
    } else {
        let count = db.delete_chunks_matching_patterns(worktree.id, &to_delete).await?;
        info!("Deleted {} chunks matching .maproomignore patterns", count);
    }

    Ok(())
}
```

### 3. Database Method (`crates/maproom/src/db/sqlite/mod.rs`)

```rust
/// Delete chunks by their IDs
pub async fn delete_chunks_matching_patterns(
    &self,
    worktree_id: i64,
    chunk_ids: &[Uuid],
) -> Result<usize> {
    // Convert UUIDs to strings for SQL
    let id_strings: Vec<String> = chunk_ids.iter().map(|id| id.to_string()).collect();

    if id_strings.is_empty() {
        return Ok(0);
    }

    // Create placeholders for SQL
    let placeholders = id_strings.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "DELETE FROM chunks WHERE worktree_id = ? AND id IN ({})",
        placeholders
    );

    let mut params = vec![worktree_id.to_string()];
    params.extend(id_strings);

    let result = sqlx::query(&query)
        .bind(worktree_id)
        // Bind all chunk IDs
        // ... (bind remaining params)
        .execute(&self.pool)
        .await?;

    Ok(result.rows_affected() as usize)
}
```

### 4. Update `main.rs` Match Arm

```rust
SubCommand::CleanIgnored { repo, worktree, dry_run } => {
    clean_ignored::clean_ignored(&config, &db, &repo, &worktree, dry_run).await?;
}
```

## Testing Strategy

### Unit Tests (`crates/maproom/src/cli/clean_ignored.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_clean_ignored_missing_file() {
        // Create test repo without .maproomignore
        // Run clean_ignored
        // Assert: succeeds with message "No patterns"
    }

    #[tokio::test]
    async fn test_clean_ignored_empty_file() {
        // Create test repo with empty .maproomignore
        // Run clean_ignored
        // Assert: succeeds with message "No patterns"
    }

    #[tokio::test]
    async fn test_clean_ignored_deletes_matching() {
        // Create test repo with .maproomignore containing "test/**"
        // Index files: src/main.rs, test/unit.rs
        // Run clean_ignored
        // Assert: test/unit.rs deleted, src/main.rs remains
    }

    #[tokio::test]
    async fn test_clean_ignored_dry_run() {
        // Create test repo with patterns
        // Run clean_ignored with dry_run=true
        // Assert: no chunks deleted, count reported correctly
    }
}
```

### Integration Test (`crates/maproom/tests/clean_ignored_test.rs`)

```rust
#[tokio::test]
async fn test_clean_ignored_cli_integration() {
    // 1. Create test repository with files
    // 2. Run scan to index files
    // 3. Create .maproomignore with pattern
    // 4. Run clean-ignored command
    // 5. Verify chunks deleted via search
}
```

## Dependencies

- Phase 1 implementation (MRMIGNR-1001 through MRMIGNR-1006) must be complete
- `load_ignore_patterns()` function from `ignore.rs`
- `IgnorePatternMatcher` for pattern matching
- Database access methods

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Accidentally delete non-matching chunks | Low | High | Thorough testing with pattern matching edge cases |
| Pattern matching inconsistency with scan/watch | Medium | Medium | Reuse same `IgnorePatternMatcher` code |
| Performance issues with large chunk sets | Low | Medium | Batch deletions, add progress reporting |
| Database transaction failures | Low | High | Proper error handling, consider transactions |

## Success Criteria

- [x] Command runs successfully: `crewchief-maproom clean-ignored --repo test --worktree main`
- [x] Chunks matching `.maproomignore` patterns are deleted
- [x] Chunks not matching patterns are preserved
- [x] Dry-run mode works correctly (reports without deleting)
- [x] All unit tests pass
- [x] Integration test demonstrates end-to-end workflow
- [x] Documentation includes working examples

## Notes

**Why not just rescan?**
Rescanning is expensive (parses all files, regenerates embeddings). The `clean-ignored` command is a surgical operation that only removes matching entries, preserving all other indexed data and embeddings.

**Edge cases to test:**
- Empty `.maproomignore`
- Missing `.maproomignore`
- Invalid patterns (should fail with clear error)
- Patterns matching no chunks (should succeed with 0 deleted)
- Patterns matching all chunks (should delete all)

**Future enhancements** (NOT in scope for this ticket):
- Progress bar for large deletions
- Option to clean across all worktrees in a repo
- Option to output deleted chunk details to JSON file
