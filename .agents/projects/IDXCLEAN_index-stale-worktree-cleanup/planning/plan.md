# Execution Plan: Index Stale Worktree Cleanup

## Project Overview

**Goal:** Implement automated detection and removal of stale worktrees from the maproom index to eliminate search result duplication and improve search quality.

**Problem:** Database contains 100+ worktrees, 95% of which no longer exist on disk, causing 15x result duplication and making search unusable.

**Solution:** Three-component system (detection, deletion, CLI) with optional watch integration for automatic cleanup.

**Scope:**
- Manual cleanup command (MVP)
- Optional watch integration (enhancement)
- Safety-first design with dry-run, confirmation, audit logging

**Success Metrics:**
- Worktree count reduced from 100+ to <10
- Search result duplication reduced from 15x to 1x
- Cleanup completes in <2 seconds
- Zero data loss for valid worktrees

---

## Project Phases

### Phase 1: Core Cleanup Infrastructure (Week 1)
**Goal:** Build foundational modules for detection and deletion

**Deliverables:**
- Stale detection module (identify worktrees with non-existent paths)
- Safe deletion module (transaction-based removal)
- Data models and error types
- Unit tests for core logic

**Agent Assignment:** rust-indexer-engineer
**Risk Level:** Low (no user-facing changes)

### Phase 2: CLI Command Interface (Week 1-2)
**Goal:** Expose cleanup via maproom CLI with complete main.rs integration

**Deliverables:**
- `maproom db cleanup-stale` subcommand
- Dry-run mode (default behavior)
- Confirmation flag (--confirm)
- User-friendly output and error messages
- Full CLI routing in main.rs (DbCommand enum and match arms)

**Tickets:** 4 (IDXCLEAN-2001 to 2004)
**Agent Assignment:** rust-indexer-engineer
**Risk Level:** Medium (user-facing, data deletion)

### Phase 3: Integration Testing and Safety Validation (Week 2)
**Goal:** Ensure cleanup is safe and correct

**Deliverables:**
- Integration test suite (database interactions)
- Safety validation tests (no false positives)
- Transaction rollback tests
- Manual validation on staging database

**Agent Assignment:** integration-tester
**Risk Level:** High (testing data deletion)

### Phase 4: Watch Integration (Week 3+) [Optional Enhancement]
**Goal:** Automatic cleanup during watch command

**✅ STATUS:** Analysis complete. No refactoring required. Ready for implementation.

**Analysis Results:**
- Watch architecture is well-suited for cleanup integration (pool-based DB access, background task pattern)
- Two integration hook points identified: Startup cleanup (line ~1140) + Status task extension (line 1432)
- Recommended approach: Option A (extend existing tasks, minimal code changes ~30-50 LOC)
- No structural changes needed - add tokio::spawn for startup, extend status_task for periodic cleanup

**Deliverables:**
- Startup cleanup (non-blocking background task on watch start)
- Periodic cleanup via status_task extension (every 30 minutes if queue idle)
- Rate limiting and safety checks (skip if indexer busy or cleanup ran recently)
- Configuration via environment variable (MAPROOM_AUTO_CLEANUP)

**Tickets:** 3 (IDXCLEAN-4001 to 4003) - **Analysis complete, all tickets are implementation**
**Agent Assignment:** rust-indexer-engineer
**Risk Level:** Low (minimal changes, non-blocking, no refactoring needed)
**Timeline:** 2-4 days (simple integration, well-understood hooks)

### Phase 5: Production Deployment (Week 4)
**Goal:** Deploy to production with monitoring

**Deliverables:**
- Documentation updates
- Deployment procedure
- Monitoring and alerting
- Incident response playbook

**Agent Assignment:** verify-ticket, commit-ticket
**Risk Level:** Medium (production deployment)

---

## Detailed Ticket Breakdown

### Phase 1: Core Cleanup Infrastructure

#### Ticket IDXCLEAN-1001: Stale Detection Module
**Description:** Implement module to identify worktrees whose abs_path no longer exists on disk.

**Acceptance Criteria:**
- [ ] `StaleWorktreeDetector` struct created
- [ ] `detect_stale_worktrees()` method queries database and validates paths
- [ ] Parallel validation using tokio (< 1s for 100 worktrees)
- [ ] Returns list of `StaleWorktree` with metadata (id, name, path, chunk_count)
- [ ] Error handling for permission denied (treat as exists)
- [ ] Unit tests for detection logic

**Technical Notes:**
```rust
// crates/maproom/src/db/cleanup.rs
pub struct StaleWorktreeDetector {
    db: DatabaseConnection,
}

impl StaleWorktreeDetector {
    pub async fn detect_stale_worktrees(&self) -> Result<Vec<StaleWorktree>> {
        // 1. Query all worktrees
        // 2. Validate each abs_path exists (parallel)
        // 3. Return stale ones
    }
}
```

**Files Modified:**
- `crates/maproom/src/db/cleanup.rs` (new file)
- `crates/maproom/src/db/mod.rs` (export cleanup module)

**Dependencies:** None

**Estimated Effort:** 1-2 days

---

#### Ticket IDXCLEAN-1002: Safe Deletion Module
**Description:** Implement transaction-safe deletion of stale worktrees using array-based JSONB removal (not CASCADE).

**Acceptance Criteria:**
- [ ] `WorktreeCleaner` struct created
- [ ] `cleanup_stale_worktrees()` method deletes within transaction
- [ ] Uses array-based deletion: removes worktree from `worktree_ids` JSONB arrays
- [ ] Garbage collection: deletes chunks with empty `worktree_ids` arrays
- [ ] Reuses SQL pattern from `incremental/tree_sha_update.rs::remove_worktree_from_chunks()`
- [ ] Multi-worktree chunks preserved (chunks with multiple worktree IDs)
- [ ] Dry-run mode supported (no actual deletion)
- [ ] Returns `CleanupReport` with statistics (includes `chunks_cleaned` count)
- [ ] Audit logging for every deletion
- [ ] Transaction rollback on any error
- [ ] Unit tests for deletion logic

**Technical Notes:**
```rust
pub struct WorktreeCleaner {
    db: DatabaseConnection,
    dry_run: bool,
}

impl WorktreeCleaner {
    pub async fn cleanup_stale_worktrees(
        &self,
        stale: Vec<StaleWorktree>,
    ) -> Result<CleanupReport> {
        // All deletions in single transaction
        // CASCADE will delete associated chunks
    }
}
```

**Files Modified:**
- `crates/maproom/src/db/cleanup.rs` (extend existing file)

**Dependencies:** IDXCLEAN-1001

**Estimated Effort:** 1-2 days

---

#### Ticket IDXCLEAN-1003: Data Models and Error Types
**Description:** Define data structures and error types for cleanup operations.

**Acceptance Criteria:**
- [ ] `StaleWorktree` struct with all necessary fields
- [ ] `CleanupReport` struct with statistics
- [ ] `CleanupError` enum for specific error cases
- [ ] Serde serialization support (for logging)
- [ ] Unit tests for data models

**Technical Notes:**
```rust
#[derive(Debug, Clone)]
pub struct StaleWorktree {
    pub id: i32,
    pub repo_id: i32,
    pub name: String,
    pub abs_path: String,
    pub exists: bool,
    pub chunk_count: i64,
}

#[derive(Debug)]
pub struct CleanupReport {
    pub total_stale: usize,
    pub deleted_count: usize,
    pub failed_count: usize,
    pub deleted_ids: Vec<i32>,
    pub failed_deletions: Vec<(i32, anyhow::Error)>,
}
```

**Files Modified:**
- `crates/maproom/src/db/cleanup.rs` (extend)
- `crates/maproom/src/error.rs` (add cleanup errors)

**Dependencies:** None (can be done in parallel with 1001/1002)

**Estimated Effort:** 0.5-1 day

---

### Phase 2: CLI Command Interface

#### Ticket IDXCLEAN-2001: CLI Subcommand Structure
**Description:** Add `maproom db cleanup-stale` subcommand to CLI.

**Acceptance Criteria:**
- [ ] New subcommand `cleanup-stale` under `db` command
- [ ] `--confirm` flag (defaults to false)
- [ ] `--verbose` flag (show detailed output)
- [ ] Command integrated into main CLI routing
- [ ] Help text and usage examples

**Technical Notes:**
```rust
// crates/maproom/src/cli/commands/db.rs
#[derive(Parser)]
pub struct CleanupStaleCommand {
    #[arg(long, help = "Actually delete (default is dry-run)")]
    confirm: bool,

    #[arg(long, short, help = "Show detailed information")]
    verbose: bool,
}
```

**Files Modified:**
- `crates/maproom/src/cli/commands/db.rs` (add subcommand)
- `crates/maproom/src/cli/mod.rs` (routing)

**Dependencies:** IDXCLEAN-1001, 1002

**Estimated Effort:** 0.5-1 day

---

#### Ticket IDXCLEAN-2002: CLI Execution Logic
**Description:** Implement command execution with dry-run, detection, and deletion phases.

**Acceptance Criteria:**
- [ ] Execute detection phase (find stale worktrees)
- [ ] Display findings to user (formatted output)
- [ ] Execute deletion phase if --confirm provided
- [ ] Show progress indicators and results
- [ ] Handle errors gracefully with clear messages
- [ ] Exit codes: 0 = success, 1 = error, 2 = no stale found

**Technical Notes:**
```rust
impl CleanupStaleCommand {
    pub async fn execute(&self, cfg: &Config) -> Result<()> {
        // Phase 1: Detection
        println!("🔍 Detecting stale worktrees...");

        // Phase 2: Report
        println!("📊 Found {} stale worktrees", stale.len());

        // Phase 3: Deletion (if confirmed)
        if self.confirm {
            println!("🗑️  Deleting stale worktrees...");
        } else {
            println!("⚠️  This was a dry-run. Use --confirm to actually delete.");
        }
    }
}
```

**Files Modified:**
- `crates/maproom/src/cli/commands/db.rs` (implement execute method)

**Dependencies:** IDXCLEAN-2001

**Estimated Effort:** 1 day

---

#### Ticket IDXCLEAN-2003: User Output Formatting
**Description:** Implement user-friendly output with progress indicators and summary.

**Acceptance Criteria:**
- [ ] Emoji indicators for different phases (🔍 📊 🗑️ ✅)
- [ ] Progress messages during detection
- [ ] Table/list format for stale worktrees
- [ ] Summary statistics (total chunks, time taken)
- [ ] Verbose mode shows additional details
- [ ] Clear warning for dry-run vs. actual deletion

**Technical Notes:**
```rust
// Example output format
🔍 Detecting stale worktrees...

📊 Found 95 stale worktrees:
  • experiment-1 (worktree_id=42, chunks=5230)
  • experiment-2 (worktree_id=43, chunks=4821)
  ...

💾 Total chunks to delete: 487,329

⚠️  This was a dry-run. Use --confirm to actually delete.
```

**Files Modified:**
- `crates/maproom/src/cli/commands/db.rs` (add formatting helpers)

**Dependencies:** IDXCLEAN-2002

**Estimated Effort:** 0.5 day

---

#### Ticket IDXCLEAN-2004: Integrate Cleanup Command with main.rs CLI
**Description:** Wire cleanup command into main.rs CLI routing (DbCommand enum and match arms).

**Acceptance Criteria:**
- [ ] Extend `DbCommand` enum with `CleanupStale { confirm: bool, verbose: bool }` variant
- [ ] Add match arm in `main()` to handle `Commands::Db { DbCommand::CleanupStale }`
- [ ] Wire up to `cleanup::StaleWorktreeDetector` and `cleanup::WorktreeCleaner`
- [ ] Implement dry-run vs. confirm logic
- [ ] Error handling with user-friendly messages
- [ ] Export cleanup types from `db/mod.rs`: `pub use cleanup::{...}`
- [ ] Integration test: CLI command invocation works correctly
- [ ] Help text: `maproom db cleanup-stale --help` shows usage

**Technical Notes:**
```rust
// crates/maproom/src/main.rs
#[derive(Subcommand, Debug)]
enum DbCommand {
    Migrate,
    CleanupStale {
        #[arg(long)]
        confirm: bool,
        #[arg(long, short)]
        verbose: bool,
    },
}

// In main() match block:
Commands::Db { command } => match command {
    DbCommand::Migrate => { /* existing */ }
    DbCommand::CleanupStale { confirm, verbose } => {
        use crewchief_maproom::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

        let client = db::connect().await?;
        let detector = StaleWorktreeDetector::new(client.clone());
        let stale = detector.detect_stale_worktrees().await?;

        if stale.is_empty() {
            println!("✅ No stale worktrees found");
            return Ok(());
        }

        // ... dry-run/confirm logic ...
    }
},
```

**Files Modified:**
- `crates/maproom/src/main.rs` (extend DbCommand, add match arm)
- `crates/maproom/src/db/mod.rs` (pub use cleanup types)

**Dependencies:** IDXCLEAN-2001, 2002, 2003

**Estimated Effort:** 2-4 hours

---

### Phase 3: Integration Testing and Safety Validation

#### Ticket IDXCLEAN-3001: Detection Accuracy Tests
**Description:** Integration tests verifying correct identification of stale vs. valid worktrees.

**Acceptance Criteria:**
- [ ] Test: Detects worktree with non-existent path
- [ ] Test: Does not detect worktree with valid path
- [ ] Test: Handles multiple stale worktrees
- [ ] Test: Empty database returns no stale worktrees
- [ ] Test: Permission denied treated as exists
- [ ] Test: Handles special characters in paths
- [ ] All tests pass

**Technical Notes:**
```rust
#[tokio::test]
async fn test_detects_stale_worktree() {
    let db = setup_test_db().await;
    let stale_id = create_stale_worktree(&db, "/tmp/nonexistent").await;

    let detector = StaleWorktreeDetector::new(db);
    let stale = detector.detect_stale_worktrees().await.unwrap();

    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].id, stale_id);
}
```

**Files Created:**
- `crates/maproom/tests/cleanup_detection_test.rs` (new test file)

**Dependencies:** IDXCLEAN-1001

**Estimated Effort:** 1 day

---

#### Ticket IDXCLEAN-3002: Deletion Safety Tests
**Description:** Integration tests verifying safe deletion with transaction integrity and multi-worktree protection.

**Acceptance Criteria:**
- [ ] Test: Deletes only stale worktrees (not valid ones)
- [ ] Test: Transaction rollback on error
- [ ] Test: **Multi-worktree chunk safety** (chunk in 2 worktrees, delete 1, verify chunk preserved)
- [ ] Test: **Single-worktree garbage collection** (chunk in 1 worktree, delete it, verify chunk deleted)
- [ ] Test: Array-based removal updates `worktree_ids` correctly
- [ ] Test: Dry-run mode makes no changes
- [ ] Test: Audit logging captures all deletions
- [ ] Test: Handles concurrent operations safely
- [ ] All tests pass

**Technical Notes:**
```rust
#[tokio::test]
async fn test_deletes_only_stale_worktrees() {
    let db = setup_test_db().await;
    let valid_id = create_valid_worktree(&db).await;
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;

    let cleaner = WorktreeCleaner::new(db.clone(), false);
    cleaner.cleanup_stale_worktrees(vec![...]).await.unwrap();

    assert!(db.get_worktree(stale_id).await.is_err()); // Deleted
    assert!(db.get_worktree(valid_id).await.is_ok()); // Preserved
}
```

**Files Created:**
- `crates/maproom/tests/cleanup_deletion_test.rs` (new test file)

**Dependencies:** IDXCLEAN-1002

**Estimated Effort:** 1 day

---

#### Ticket IDXCLEAN-3003: CLI Integration Tests
**Description:** End-to-end tests of CLI command behavior.

**Acceptance Criteria:**
- [ ] Test: Default execution is dry-run
- [ ] Test: --confirm flag actually deletes
- [ ] Test: Output format is correct
- [ ] Test: Exit codes are correct
- [ ] Test: Error handling works (database failure, no stale found)
- [ ] All tests pass

**Technical Notes:**
```rust
#[tokio::test]
async fn test_cli_default_is_dry_run() {
    let db = setup_test_db().await;
    let stale_id = create_stale_worktree(&db, "/tmp/stale").await;

    let cmd = CleanupStaleCommand { confirm: false, verbose: false };
    cmd.execute(&test_config()).await.unwrap();

    // Verify: worktree still exists (dry-run)
    assert!(db.get_worktree(stale_id).await.is_ok());
}
```

**Files Created:**
- `crates/maproom/tests/cleanup_cli_test.rs` (new test file)

**Dependencies:** IDXCLEAN-2002

**Estimated Effort:** 1 day

---

#### Ticket IDXCLEAN-3004: Manual Validation on Staging
**Description:** Run cleanup on staging database and manually verify results.

**Acceptance Criteria:**
- [ ] Dry-run executed on staging database
- [ ] Output reviewed for accuracy (all reported stale are actually stale)
- [ ] Cleanup with --confirm executed successfully
- [ ] Search quality improved (result duplication reduced)
- [ ] No valid worktrees accidentally deleted
- [ ] Performance acceptable (< 2 seconds)
- [ ] Validation report documented

**Technical Notes:**
- Connect to staging database
- Run: `maproom db cleanup-stale` (dry-run)
- Review output: Are all paths actually non-existent?
- Run: `maproom db cleanup-stale --confirm`
- Verify: Search for known symbols, check duplication

**Files Modified:**
- Update ticket with validation report

**Dependencies:** IDXCLEAN-2003, 3001, 3002, 3003

**Estimated Effort:** 0.5 day

---

### Phase 4: Watch Integration (Optional Enhancement)

**Analysis Completed:** Watch command architecture analyzed. No refactoring needed. Simple integration via Option A (extend existing tasks).

#### Ticket IDXCLEAN-4001: Startup Cleanup Integration
**Description:** Add non-blocking startup cleanup to watch_worktree() function (based on Watch analysis findings).

**Acceptance Criteria:**
- [ ] Add tokio::spawn for startup cleanup after pool creation (line ~1140 in indexer/mod.rs)
- [ ] Cleanup runs in background (non-blocking, < 200ms startup delay)
- [ ] Uses existing StaleWorktreeDetector and WorktreeCleaner from db::cleanup module
- [ ] Controlled by MAPROOM_AUTO_CLEANUP environment variable (default: false)
- [ ] Errors logged with tracing::warn! but don't break watch startup
- [ ] Cleanup logs with emoji indicators (🧹 starting, ✅ success, ⚠️ failure)
- [ ] Integration test: watch starts immediately even if cleanup running

**Technical Notes:**
```rust
// In indexer/mod.rs::watch_worktree() after pool creation (line ~1142)

let enable_auto_cleanup = std::env::var("MAPROOM_AUTO_CLEANUP")
    .unwrap_or_else(|_| "false".to_string())
    .parse::<bool>()
    .unwrap_or(false);

if enable_auto_cleanup {
    let pool_cleanup = pool.clone();
    tokio::spawn(async move {
        use crate::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

        match StaleWorktreeDetector::new(pool_cleanup.clone()).detect_stale_worktrees().await {
            Ok(stale) if !stale.is_empty() => {
                tracing::info!("🧹 Startup cleanup: found {} stale worktrees", stale.len());
                match WorktreeCleaner::new(pool_cleanup, false).cleanup_stale_worktrees(stale).await {
                    Ok(report) => tracing::info!("✅ Cleanup complete: {} deleted", report.deleted_count),
                    Err(e) => tracing::warn!("⚠️  Cleanup failed: {}", e),
                }
            }
            Ok(_) => tracing::debug!("No stale worktrees found"),
            Err(e) => tracing::warn!("Cleanup detection failed: {}", e),
        }
    });
}
```

**Files Modified:**
- `crates/maproom/src/indexer/mod.rs` (add startup cleanup spawn)

**Dependencies:** IDXCLEAN-1001, 1002

**Estimated Effort:** 0.5-1 day

---

#### Ticket IDXCLEAN-4002: Periodic Cleanup via Status Task Extension
**Description:** Extend existing status_task loop with periodic cleanup checks (based on Watch analysis findings).

**Acceptance Criteria:**
- [ ] Extend status_task loop (line ~1432 in indexer/mod.rs) with cleanup check
- [ ] Cleanup runs every 30 minutes (configurable interval)
- [ ] Rate limiting: skip if cleanup ran in last 15 minutes
- [ ] Queue idle detection: only run if `stats.pending == 0 && stats.processing == 0`
- [ ] Cleanup spawned as tokio::spawn (non-blocking)
- [ ] Track last_cleanup timestamp (Option<Instant>)
- [ ] Controlled by same MAPROOM_AUTO_CLEANUP env variable
- [ ] Integration test: cleanup defers when indexer busy

**Technical Notes:**
```rust
// In status_task (around line 1432)
let mut interval = tokio::time::interval(Duration::from_secs(10));
let mut last_cleanup: Option<Instant> = None;
let cleanup_interval = Duration::from_secs(30 * 60); // 30 minutes

let enable_auto_cleanup = std::env::var("MAPROOM_AUTO_CLEANUP")
    .unwrap_or_else(|_| "false".to_string())
    .parse::<bool>()
    .unwrap_or(false);

loop {
    interval.tick().await;
    let stats = queue_clone.lock().await.stats();

    // ... existing status logging ...

    // Periodic cleanup check (if enabled)
    if enable_auto_cleanup {
        let should_cleanup = match last_cleanup {
            None => false, // Don't run on first check (startup already did)
            Some(instant) => instant.elapsed() > cleanup_interval,
        };

        // Only cleanup if queue idle and enough time passed
        if should_cleanup && stats.pending == 0 && stats.processing == 0 {
            let pool_cleanup = pool_clone.clone();
            tokio::spawn(async move {
                // Run cleanup (same logic as startup)
            });
            last_cleanup = Some(Instant::now());
        }
    }
}
```

**Files Modified:**
- `crates/maproom/src/indexer/mod.rs` (extend status_task)

**Dependencies:** IDXCLEAN-4001

**Estimated Effort:** 0.5-1 day

---

#### Ticket IDXCLEAN-4003: Configuration Documentation and Testing
**Description:** Document configuration options and add comprehensive integration tests for watch cleanup.

**Acceptance Criteria:**
- [ ] Environment variable documented in README.md
- [ ] Example usage in crates/maproom/CLAUDE.md
- [ ] Integration test: Watch startup cleanup runs in background
- [ ] Integration test: Periodic cleanup respects rate limiting
- [ ] Integration test: Cleanup skips when indexer busy
- [ ] Integration test: MAPROOM_AUTO_CLEANUP=false disables cleanup
- [ ] Integration test: MAPROOM_AUTO_CLEANUP=true enables cleanup
- [ ] Performance test: Watch startup delay < 200ms with cleanup enabled

**Technical Notes:**
```md
# README.md addition

## Auto-Cleanup Configuration

Enable automatic cleanup during `maproom watch`:

```bash
export MAPROOM_AUTO_CLEANUP=true
maproom watch
```

Behavior:
- Runs quick cleanup on watch startup (non-blocking)
- Periodic cleanup every 30 minutes (only when indexer idle)
- Rate limited to prevent excessive operations
```

**Files Modified:**
- `README.md` (add configuration section)
- `crates/maproom/CLAUDE.md` (add watch cleanup notes)
- `crates/maproom/tests/watch_cleanup_test.rs` (new integration tests)

**Dependencies:** IDXCLEAN-4002

**Estimated Effort:** 0.5-1 day
```

**Files Modified:**
- `crates/maproom/src/config.rs` (add CleanupConfig struct)
- `config/maproom.toml.example` (add example config)

**Dependencies:** IDXCLEAN-4001

**Estimated Effort:** 0.5 day

---

### Phase 5: Production Deployment

#### Ticket IDXCLEAN-5001: Documentation Updates
**Description:** Update README, CHANGELOG, and add user guide for cleanup command.

**Acceptance Criteria:**
- [ ] README: Add cleanup command to usage section
- [ ] CHANGELOG: Document new cleanup feature
- [ ] User guide: Step-by-step cleanup instructions
- [ ] Administrator guide: Watch integration setup
- [ ] Recovery procedures: How to restore if accident
- [ ] Security considerations: Backup recommendations

**Technical Notes:**
- Document: `maproom db cleanup-stale` usage
- Document: Watch auto-cleanup configuration
- Document: Dry-run workflow
- Document: Recovery from accidental deletion

**Files Modified:**
- `README.md` (add cleanup section)
- `CHANGELOG.md` (add IDXCLEAN entries)
- `docs/user-guide-cleanup.md` (new file)
- `docs/admin-guide-watch-cleanup.md` (new file)

**Dependencies:** All previous tickets complete

**Estimated Effort:** 1 day

---

#### Ticket IDXCLEAN-5002: Deployment Procedure
**Description:** Create deployment procedure and monitoring setup.

**Acceptance Criteria:**
- [ ] Deployment checklist created
- [ ] Rollback procedure documented
- [ ] Monitoring configured (error alerts)
- [ ] Log aggregation configured
- [ ] Performance baseline established
- [ ] Incident response playbook created

**Technical Notes:**
- Deploy to staging first
- Run manual validation
- Monitor for 24 hours
- Deploy to production (phased rollout)
- Enable watch integration after stability proven

**Files Created:**
- `docs/deployment-cleanup.md` (new file)
- `docs/incident-response-cleanup.md` (new file)

**Dependencies:** IDXCLEAN-5001

**Estimated Effort:** 0.5 day

---

#### Ticket IDXCLEAN-5003: Production Verification
**Description:** Verify cleanup works correctly in production environment.

**Acceptance Criteria:**
- [ ] Dry-run executed on production database
- [ ] Results reviewed for accuracy
- [ ] Cleanup with --confirm executed successfully
- [ ] Search quality improved measurably
- [ ] No errors in logs
- [ ] Performance within acceptable limits
- [ ] Monitoring shows healthy metrics

**Technical Notes:**
- Run dry-run on production
- Get team approval on results
- Execute cleanup with confirmation
- Monitor for 48 hours
- Verify search result duplication reduced
- Document actual results vs. expected

**Files Modified:**
- Update ticket with production verification report

**Dependencies:** IDXCLEAN-5002

**Estimated Effort:** 0.5 day

---

## Agent Assignments

### rust-indexer-engineer
**Responsibilities:**
- All Rust code implementation (detection, deletion, CLI, watch)
- Core modules and data structures
- Error handling and logging
- Performance optimization

**Tickets Assigned:**
- Phase 1: IDXCLEAN-1001, 1002, 1003
- Phase 2: IDXCLEAN-2001, 2002, 2003
- Phase 4: IDXCLEAN-4001, 4002, 4003, 4004

### integration-tester
**Responsibilities:**
- Comprehensive test suite creation
- Integration tests for database operations
- CLI behavior tests
- Manual validation on staging

**Tickets Assigned:**
- Phase 3: IDXCLEAN-3001, 3002, 3003, 3004

### verify-ticket
**Responsibilities:**
- Verify acceptance criteria met for each ticket
- Check test coverage and quality
- Validate documentation completeness

**Tickets Assigned:**
- All phases (verification step for each ticket)

### commit-ticket
**Responsibilities:**
- Create Conventional Commits for completed work
- Ensure commit messages reference tickets
- Maintain git history quality

**Tickets Assigned:**
- All phases (commit step for each ticket)

---

## Timeline and Milestones

### Week 1: Core Infrastructure
**Milestone:** Manual cleanup command working

**Deliverables:**
- Detection and deletion modules complete
- CLI command interface implemented
- Unit tests passing

**Success Criteria:**
- Can detect stale worktrees via CLI
- Dry-run shows accurate results
- Deletion works with --confirm

### Week 2: Safety Validation
**Milestone:** Production-ready cleanup command

**Deliverables:**
- Integration test suite complete
- Manual validation on staging successful
- Documentation updated

**Success Criteria:**
- All tests passing
- Staging validation shows no false positives
- User guide complete

### Week 3: Watch Integration (Optional)
**Milestone:** Automatic cleanup during watch

**Deliverables:**
- Cleanup scheduler module complete
- Startup and periodic cleanup working
- Configuration options implemented

**Success Criteria:**
- Watch startup time acceptable (<200ms)
- Background cleanup doesn't interfere
- Rate limiting works correctly

### Week 4: Production Deployment
**Milestone:** Deployed and verified in production

**Deliverables:**
- Production deployment complete
- Monitoring and alerting configured
- Production verification successful

**Success Criteria:**
- Production cleanup executed successfully
- Search quality improved measurably
- No incidents or errors

---

## Testing Milestones

### Checkpoint 1: Unit Tests (End of Week 1)
- All core modules have unit tests
- Unit test coverage > 80%
- All unit tests passing

### Checkpoint 2: Integration Tests (End of Week 2)
- Detection accuracy tests passing
- Deletion safety tests passing
- CLI integration tests passing

### Checkpoint 3: Manual Validation (Week 2)
- Staging database validation complete
- Results reviewed and approved by team
- No false positives detected

### Checkpoint 4: Production Verification (Week 4)
- Production cleanup successful
- Search quality improvement confirmed
- No errors in production logs

---

## Security Checkpoints

### Security Review 1: Design Phase (Before Implementation)
- [ ] Threat model reviewed and approved
- [ ] Security requirements defined
- [ ] Defense-in-depth strategy confirmed

### Security Review 2: Implementation Phase (End of Week 1)
- [ ] Code review for deletion logic
- [ ] Transaction safety verified
- [ ] Audit logging implemented

### Security Review 3: Testing Phase (End of Week 2)
- [ ] Security tests passing
- [ ] No false positives in validation
- [ ] Recovery procedures tested

### Security Review 4: Production Deployment (Week 4)
- [ ] Database backups confirmed
- [ ] Incident response playbook ready
- [ ] Monitoring and alerting configured

---

## Risk Mitigation Plan

### Risk: Accidental deletion of valid worktree
**Impact:** High (data loss)
**Likelihood:** Low (multiple safety checks)

**Mitigation:**
- Dry-run default (explicit confirmation required)
- Manual validation on staging before production
- Audit logging for recovery
- Database backups available

**Contingency:**
- If occurs: Restore from database backup
- Add test case to prevent regression
- Review validation logic for bug

### Risk: Performance degradation during watch
**Impact:** Medium (user experience)
**Likelihood:** Low (background execution, rate limiting)

**Mitigation:**
- Performance testing on staging
- Rate limiting (max once per 15 min)
- Busy detection (defer if indexer active)
- Configuration to disable if needed

**Contingency:**
- If occurs: Increase cleanup interval
- Add more aggressive rate limiting
- Disable auto-cleanup temporarily

### Risk: Database corruption
**Impact:** Critical (data loss)
**Likelihood:** Very Low (transaction safety)

**Mitigation:**
- Transaction-based deletion (ACID guarantees)
- Rollback on error
- Foreign key constraints (CASCADE)
- Database backups before production cleanup

**Contingency:**
- If occurs: Transaction auto-rolls back
- Restore from backup if needed
- Review transaction handling code

### Risk: Watch integration interference
**Impact:** Medium (indexing delays)
**Likelihood:** Low (priority scheduling, busy detection)

**Mitigation:**
- Startup cleanup is optional and background
- Periodic cleanup defers if indexer busy
- Rate limiting prevents excessive overhead
- Configuration to disable

**Contingency:**
- If occurs: Disable auto-cleanup via config
- Increase cooldown period
- Add more aggressive busy detection

---

## Success Criteria (Project-Level)

### Functional Success
- [ ] Manual cleanup command works (dry-run and --confirm)
- [ ] Stale worktrees detected accurately (100% accuracy)
- [ ] Deletion is safe (0 false positives)
- [ ] Watch integration works without interference
- [ ] Configuration options work correctly

### Quality Success
- [ ] All unit tests passing (>80% coverage)
- [ ] All integration tests passing (100% critical paths)
- [ ] Manual validation successful (staging and production)
- [ ] Documentation complete and accurate
- [ ] Code review approved

### Performance Success
- [ ] Manual cleanup completes in <2 seconds (100 worktrees)
- [ ] Watch startup delay <200ms (background cleanup)
- [ ] Periodic cleanup completes in <500ms
- [ ] No indexing performance degradation

### Business Success
- [ ] Worktree count reduced from 100+ to <10
- [ ] Search result duplication reduced from 15x to 1x
- [ ] Search quality improved measurably
- [ ] User feedback positive
- [ ] Zero incidents or data loss

---

## Deployment Strategy

### Phase 1: Staging Deployment (Week 2)
1. Deploy to staging environment
2. Run dry-run and review results
3. Execute cleanup with --confirm
4. Monitor for 24 hours
5. Verify search quality improvement

### Phase 2: Production Manual Cleanup (Week 3)
1. Deploy CLI command to production
2. Run dry-run on production database
3. Get team approval on results
4. Execute cleanup with --confirm
5. Monitor for 48 hours
6. Verify search quality improvement

### Phase 3: Watch Integration Rollout (Week 4)
1. Enable startup cleanup only
2. Monitor for 48 hours
3. Enable periodic cleanup (conservative interval)
4. Monitor for 1 week
5. Tune configuration based on metrics

### Phase 4: Full Production (Ongoing)
1. Cleanup runs automatically via watch
2. Continuous monitoring for errors
3. Periodic validation of results
4. Configuration tuning as needed

---

## Monitoring and Metrics

### Operational Metrics
- Cleanup execution count (how often run)
- Stale worktrees detected per cleanup
- Worktrees deleted per cleanup
- Cleanup execution time
- Error rate (failed cleanups)

### Performance Metrics
- Watch startup time (with/without cleanup)
- Periodic cleanup execution time
- Indexing latency (ensure no degradation)
- Database query performance

### Business Metrics
- Total worktree count over time
- Search result duplication factor
- Search quality scores
- User satisfaction with search

### Alerting Thresholds
- Error rate > 5% → Alert
- Cleanup time > 2 seconds → Warning
- Worktree count increasing → Alert
- Search duplication > 5x → Warning

---

## Summary

**Project Scope:** 3-4 weeks for MVP (manual cleanup) + 1 week for watch integration

**Ticket Count:**
- Phase 1: 3 tickets (core infrastructure)
- Phase 2: 3 tickets (CLI interface)
- Phase 3: 4 tickets (testing and validation)
- Phase 4: 4 tickets (watch integration, optional)
- Phase 5: 3 tickets (deployment and verification)
- **Total: 17 tickets**

**Key Success Factors:**
1. Safety first: Multiple layers of defense against data loss
2. Incremental delivery: Manual cleanup first, then automation
3. Thorough testing: Integration tests more valuable than unit tests
4. Production validation: Test on staging, monitor closely in production

**Dependencies:**
- None external (self-contained project)
- PostgreSQL database (existing)
- Rust toolchain (existing)
- maproom codebase (existing)

**Risk Level:** Medium (data deletion always risky, but mitigated with safety mechanisms)

**Confidence Level:** High (90%) for MVP, Medium (75%) for watch integration
