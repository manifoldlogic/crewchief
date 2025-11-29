# Ticket: IDXCLEAN-1003: Define Data Models and Error Types for Cleanup Operations

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - unit tests executed and passing (7/7 passed)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create well-defined data structures (`StaleWorktree`, `CleanupReport`, `CleanupError`) with serde serialization support for logging and error handling throughout the cleanup system.

## Background
This is the third ticket in Phase 1 of the IDXCLEAN project. While IDXCLEAN-1001 and IDXCLEAN-1002 implement core detection and deletion logic, this ticket defines the foundational data structures and error types used throughout the cleanup system.

This ticket can be done in parallel with 1001/1002 as it defines interfaces without implementation logic.

Good type design is critical for a data deletion system. This ticket ensures:
1. Clear contracts between detection and deletion modules
2. Structured error messages for debugging and user feedback
3. Serializable reports for audit logging
4. Type safety prevents passing wrong data between components

This aligns with Rust's "Make invalid states unrepresentable" principle - if the types are correct, many bugs become impossible.

## Acceptance Criteria
- [ ] `StaleWorktree` struct defined with all necessary fields
- [ ] `CleanupReport` struct defined with statistics and outcome data
- [ ] `CleanupError` enum defined for specific error cases
- [ ] All structs derive: `Debug`, `Clone` (where appropriate)
- [ ] All structs support serde serialization (for logging): `#[derive(Serialize)]`
- [ ] `CleanupReport` has helper methods: `success_rate()`, `has_failures()`
- [ ] Error types have clear, actionable error messages
- [ ] Unit tests for struct creation and serialization
- [ ] Unit tests for error message formatting
- [ ] Documentation comments on all public types

## Technical Requirements
- Use `#[derive(Debug, Clone, Serialize)]` for data structs
- Use `thiserror` for error type definitions (clear error messages)
- `StaleWorktree.exists` field must be bool (for clear semantics)
- `CleanupReport` statistics must use appropriate numeric types (usize for counts, i64 for chunk counts)
- Error messages must include context (worktree ID, path, etc.)
- All types must be in `crates/maproom/src/db/cleanup.rs` module

## Implementation Notes

```rust
// crates/maproom/src/db/cleanup.rs (extend or standalone section)

use serde::Serialize;
use thiserror::Error;

/// Represents a worktree that may be stale (path doesn't exist on disk)
#[derive(Debug, Clone, Serialize)]
pub struct StaleWorktree {
    /// Database ID of the worktree
    pub id: i32,
    /// Repository ID this worktree belongs to
    pub repo_id: i32,
    /// Worktree name (e.g., "main", "feature-branch")
    pub name: String,
    /// Absolute path on disk (may not exist)
    pub abs_path: String,
    /// Whether the path exists on disk (false = stale)
    pub exists: bool,
    /// Number of chunks associated with this worktree
    pub chunk_count: i64,
}

/// Report of cleanup operation results
#[derive(Debug, Serialize)]
pub struct CleanupReport {
    /// Total number of stale worktrees detected
    pub total_stale: usize,
    /// Number successfully deleted
    pub deleted_count: usize,
    /// Number of chunks cleaned (garbage collected)
    pub chunks_cleaned: i64,
    /// Number that failed to delete
    pub failed_count: usize,
    /// IDs of successfully deleted worktrees
    pub deleted_ids: Vec<i32>,
    /// Failed deletions with error details
    #[serde(skip)]  // Errors not serializable
    pub failed_deletions: Vec<(i32, anyhow::Error)>,
}

impl CleanupReport {
    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        if self.total_stale == 0 {
            1.0
        } else {
            self.deleted_count as f64 / self.total_stale as f64
        }
    }

    /// Check if any deletions failed
    pub fn has_failures(&self) -> bool {
        self.failed_count > 0
    }

    /// Get total database impact (worktrees + chunks removed)
    pub fn total_removed(&self) -> i64 {
        self.deleted_count as i64 + self.chunks_cleaned
    }
}

/// Errors specific to cleanup operations
#[derive(Error, Debug)]
pub enum CleanupError {
    #[error("Database transaction failed during cleanup: {0}")]
    TransactionFailed(#[source] sqlx::Error),

    #[error("Failed to validate worktree path {path}: {source}")]
    ValidationFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Worktree {id} not found in database")]
    WorktreeNotFound { id: i32 },

    #[error("Database connection failed: {0}")]
    ConnectionFailed(#[from] sqlx::Error),

    #[error("Cleanup operation cancelled by user")]
    Cancelled,
}
```

**Testing Strategy**:
```rust
#[test]
fn test_stale_worktree_struct() {
    let stale = StaleWorktree {
        id: 1,
        repo_id: 1,
        name: "test".into(),
        abs_path: "/tmp/test".into(),
        exists: false,
        chunk_count: 42,
    };

    assert!(!stale.exists);
    assert_eq!(stale.chunk_count, 42);

    // Test serialization
    let json = serde_json::to_string(&stale).unwrap();
    assert!(json.contains("\"exists\":false"));
}

#[test]
fn test_cleanup_report_success_rate() {
    let report = CleanupReport {
        total_stale: 100,
        deleted_count: 95,
        chunks_cleaned: 50000,
        failed_count: 5,
        deleted_ids: vec![],
        failed_deletions: vec![],
    };

    assert_eq!(report.success_rate(), 0.95);
    assert!(report.has_failures());
    assert_eq!(report.total_removed(), 50095);
}

#[test]
fn test_cleanup_error_messages() {
    let err = CleanupError::WorktreeNotFound { id: 42 };
    assert!(err.to_string().contains("Worktree 42 not found"));

    let err = CleanupError::Cancelled;
    assert!(err.to_string().contains("cancelled by user"));
}
```

## Dependencies
None (can be done in parallel with IDXCLEAN-1001 and IDXCLEAN-1002)

## Risk Assessment
- **Risk**: None (pure data definition, no logic)
  - **Mitigation**: N/A

## Files/Packages Affected
- `crates/maproom/src/db/cleanup.rs` (if not already created, or extend existing)
- `crates/maproom/Cargo.toml` (ensure `serde`, `thiserror` dependencies present)
