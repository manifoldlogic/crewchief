# Ticket: BRWATCH-4002: Add Rustdoc comments to watcher code

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - `cargo doc` builds without warnings
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Add comprehensive Rustdoc comments to the watcher module, documenting public APIs, implementation details, and usage examples.

## Background
This ticket implements Step 4.2 from the implementation plan (plan.md - Phase 4). Good code documentation helps future maintainers understand the implementation and aids IDE autocomplete/tooltips.

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 4.2

## Acceptance Criteria
- [ ] Module-level doc comment for watcher.rs
- [ ] Rustdoc comments for BranchWatcher struct
- [ ] Rustdoc comments for all public methods
- [ ] Code examples in doc comments
- [ ] Doc comments for DebouncedHandler if public
- [ ] `cargo doc` builds without warnings
- [ ] Generated docs viewable with `cargo doc --open`

## Technical Requirements
- Add `///` doc comments (outer docs) for public items
- Add `//!` doc comments (inner docs) for module
- Include examples in doc comments using ` ```rust` blocks
- Document parameters and return values
- Explain error conditions
- Link to related functions with `[method_name]`
- Follow Rust documentation conventions

## Implementation Notes

### Module-Level Documentation

```rust
//! Branch switch detection and automatic indexing.
//!
//! This module implements automatic detection of git branch switches by watching
//! the `.git/HEAD` file for changes. When a branch switch is detected, it triggers
//! incremental indexing using the BRANCHX infrastructure.
//!
//! # Architecture
//!
//! The watcher uses the [`notify`] crate to monitor `.git/HEAD` for file system events.
//! When a write event occurs (indicating a branch switch), the watcher:
//!
//! 1. Extracts the new branch name from `.git/HEAD`
//! 2. Gets or creates a worktree record in the database
//! 3. Triggers incremental update for the new branch
//! 4. Logs indexing metrics
//!
//! # Performance
//!
//! - Detection latency: <1 second (OS file events)
//! - CPU usage while idle: <5%
//! - Memory usage: ~10-20MB
//!
//! # Example
//!
//! ```rust
//! use crewchief_maproom::watcher::BranchWatcher;
//! use sqlx::PgPool;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let pool = PgPool::connect(&std::env::var("DATABASE_URL")?).await?;
//!     let repo_path = PathBuf::from("/path/to/repo");
//!
//!     let mut watcher = BranchWatcher::new(repo_path, pool)?;
//!     watcher.start().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Related
//!
//! - [`incremental_update`]: The BRANCHX function called on branch switches
//! - [`get_or_create_worktree`]: Database function for worktree records

use notify::{Watcher, RecursiveMode, watcher, DebouncedEvent};
// ...
```

### Struct Documentation

```rust
/// Watches `.git/HEAD` for branch switches and triggers automatic indexing.
///
/// The `BranchWatcher` uses OS-level file system events to detect when a developer
/// switches branches. This avoids polling and ensures minimal resource usage while idle.
///
/// # Lifecycle
///
/// 1. Create watcher with [`BranchWatcher::new`]
/// 2. Start watching with [`BranchWatcher::start`]
/// 3. Watcher runs until error or shutdown signal
///
/// # Error Handling
///
/// The watcher is designed to be fault-tolerant. Errors during indexing (database
/// connection failures, git errors) are logged but don't crash the watcher. Transient
/// errors trigger retry logic with exponential backoff.
///
/// # Example
///
/// ```rust
/// let mut watcher = BranchWatcher::new(repo_path, pool)?;
///
/// // Runs until Ctrl+C or error
/// watcher.start().await?;
/// ```
pub struct BranchWatcher {
    /// Path to the git repository root
    repo_path: PathBuf,

    /// Database connection pool for worktree and chunk queries
    pool: PgPool,

    /// File system watcher from notify crate
    watcher: RecommendedWatcher,

    /// Event receiver channel
    rx: Receiver<DebouncedEvent>,

    /// Debouncer to prevent rapid successive triggers
    debouncer: DebouncedHandler,
}
```

### Method Documentation

```rust
impl BranchWatcher {
    /// Creates a new branch watcher for the specified repository.
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the git repository root (containing `.git/`)
    /// * `pool` - PostgreSQL connection pool
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File watcher initialization fails
    /// - Repository path is invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// let pool = PgPool::connect(&database_url).await?;
    /// let repo = PathBuf::from("/workspace/myproject");
    /// let watcher = BranchWatcher::new(repo, pool)?;
    /// ```
    pub fn new(repo_path: PathBuf, pool: PgPool) -> Result<Self> {
        // ...
    }

    /// Starts watching for branch switches and indexes the current branch.
    ///
    /// This method:
    /// 1. Validates that `.git/HEAD` exists
    /// 2. Begins watching the file for changes
    /// 3. Indexes the current branch initially
    /// 4. Enters the watch loop (blocks until error/shutdown)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Not a git repository (no `.git/HEAD`)
    /// - File watcher fails to start
    /// - Initial indexing fails (non-retryable error)
    ///
    /// # Example
    ///
    /// ```rust
    /// let mut watcher = BranchWatcher::new(repo_path, pool)?;
    ///
    /// // Blocks until Ctrl+C or error
    /// watcher.start().await?;
    /// ```
    pub async fn start(&mut self) -> Result<()> {
        // ...
    }

    /// Handles a detected branch switch by triggering incremental update.
    ///
    /// Called automatically when `.git/HEAD` changes. Extracts the new branch name,
    /// gets/creates the worktree record, and triggers incremental indexing.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Cannot read `.git/HEAD`
    /// - Database operations fail
    /// - Incremental update fails
    ///
    /// Errors are logged and may trigger retry logic in [`handle_branch_switch_with_retry`].
    async fn handle_branch_switch(&self) -> Result<()> {
        // ...
    }
}
```

### Function Documentation

```rust
/// Extracts the current branch name from a git repository.
///
/// Reads `.git/HEAD` and parses either a branch reference or detached HEAD.
///
/// # Branch Formats
///
/// - Standard branch: `ref: refs/heads/main` → `"main"`
/// - Feature branch: `ref: refs/heads/feature/auth` → `"feature/auth"`
/// - Detached HEAD: `abc123def...` → `"abc123de"` (first 8 chars)
///
/// # Arguments
///
/// * `repo_path` - Path to repository root
///
/// # Errors
///
/// Returns error if:
/// - `.git/HEAD` doesn't exist
/// - Cannot read file
/// - Content has invalid format
///
/// # Example
///
/// ```rust
/// let branch = get_current_branch(&repo_path)?;
/// assert_eq!(branch, "main");
/// ```
fn get_current_branch(repo_path: &Path) -> Result<String> {
    // ...
}
```

## Dependencies
- BRWATCH implementation complete (watcher.rs exists with code)

## Risk Assessment
- **Risk**: Doc comments become outdated
  - **Mitigation**: Update docs when changing function signatures, code review checks
- **Risk**: Examples in docs don't compile
  - **Mitigation**: `cargo test --doc` validates doc examples

## Files/Packages Affected
- `/workspace/crates/maproom/src/watcher.rs` (add Rustdoc comments)
