# Architecture: Index Stale Worktree Cleanup

## MVP Solution Design

### Overview

The cleanup system has **three primary components** and **one optional enhancement**:

1. **Stale Detection Module** - Identifies worktrees whose `abs_path` no longer exists
2. **Safe Deletion Module** - Removes stale worktrees with transaction safety
3. **CLI Command Interface** - Exposes cleanup via `maproom db cleanup-stale`
4. **Watch Integration** (optional enhancement) - Automatic periodic cleanup during watch

This architecture prioritizes **safety**, **simplicity**, and **incremental delivery**.

---

## Component 1: Stale Detection Module

### Responsibility
Identify worktrees that should be removed based on disk validation.

### Design

```rust
// crates/maproom/src/db/cleanup.rs

pub struct StaleWorktreeDetector {
    db: DatabaseConnection,
}

impl StaleWorktreeDetector {
    /// Returns list of worktree IDs whose abs_path does not exist on disk
    pub async fn detect_stale_worktrees(&self) -> Result<Vec<StaleWorktree>> {
        // 1. Query all worktrees from database
        let worktrees = self.db.query_all_worktrees().await?;

        // 2. Check each abs_path for existence (parallel)
        let checks = worktrees.into_iter()
            .map(|wt| self.validate_worktree(wt));

        let results = futures::future::join_all(checks).await;

        // 3. Filter to only stale ones
        Ok(results.into_iter()
            .filter_map(|r| r.ok())
            .filter(|wt| !wt.exists)
            .collect())
    }

    async fn validate_worktree(&self, wt: Worktree) -> Result<StaleWorktree> {
        let exists = tokio::fs::try_exists(&wt.abs_path).await?;
        Ok(StaleWorktree {
            id: wt.id,
            repo_id: wt.repo_id,
            name: wt.name,
            abs_path: wt.abs_path,
            exists,
            chunk_count: self.db.count_chunks_for_worktree(wt.id).await?,
        })
    }
}

pub struct StaleWorktree {
    pub id: i32,
    pub repo_id: i32,
    pub name: String,
    pub abs_path: String,
    pub exists: bool,
    pub chunk_count: i64,
}
```

**Key Design Decisions:**

- **Async/parallel validation:** Use `tokio::fs::try_exists` + `join_all` for fast disk checks
- **Rich result type:** `StaleWorktree` includes metadata for user inspection
- **Error handling:** Individual validation failures don't stop entire process
- **Exclusion patterns:** Applied at indexing time (separate concern)

### Performance Characteristics

- **Disk check:** ~1ms per worktree (SSD), parallelized
- **Database query:** Single query for all worktrees (~10ms)
- **Total time:** ~100ms for 100 worktrees (parallel), ~1s (sequential)

**Optimization:** Batch worktrees into chunks of 10-20 for optimal parallelism.

---

## Component 2: Safe Deletion Module

### Responsibility
Remove stale worktrees from database with transaction safety and audit logging.

### Design

```rust
// crates/maproom/src/db/cleanup.rs

pub struct WorktreeCleaner {
    db: DatabaseConnection,
    dry_run: bool,
}

impl WorktreeCleaner {
    /// Deletes stale worktrees within a transaction
    pub async fn cleanup_stale_worktrees(
        &self,
        stale: Vec<StaleWorktree>,
    ) -> Result<CleanupReport> {
        if self.dry_run {
            return Ok(self.create_dry_run_report(&stale));
        }

        let mut tx = self.db.begin_transaction().await?;
        let mut deleted_ids = Vec::new();
        let mut chunks_cleaned = 0;
        let mut failed_deletions = Vec::new();

        for wt in stale {
            match self.delete_worktree_tx(&mut tx, wt.id).await {
                Ok(cleaned) => {
                    deleted_ids.push(wt.id);
                    chunks_cleaned += cleaned;
                    tracing::info!(
                        worktree_id = wt.id,
                        name = %wt.name,
                        abs_path = %wt.abs_path,
                        chunks_cleaned = cleaned,
                        "Deleted stale worktree"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        worktree_id = wt.id,
                        error = %e,
                        "Failed to delete stale worktree"
                    );
                    failed_deletions.push((wt.id, e));
                }
            }
        }

        tx.commit().await?;

        Ok(CleanupReport {
            total_stale: stale.len(),
            deleted_count: deleted_ids.len(),
            chunks_cleaned,
            failed_count: failed_deletions.len(),
            deleted_ids,
            failed_deletions,
        })
    }

    async fn delete_worktree_tx(
        &self,
        tx: &mut Transaction,
        worktree_id: i32,
    ) -> Result<i64> {
        // Step 1: Remove worktree from chunks.worktree_ids JSONB arrays
        // Uses same pattern as incremental/tree_sha_update.rs::remove_worktree_from_chunks
        let affected = sqlx::query(
            r#"
            UPDATE maproom.chunks
            SET worktree_ids = worktree_ids - $1::TEXT,
                updated_at = NOW()
            WHERE worktree_ids ? $1::TEXT
            "#
        )
        .bind(worktree_id.to_string())
        .execute(&mut **tx)
        .await?
        .rows_affected();

        // Step 2: Garbage collection - delete chunks with empty worktree_ids
        let deleted = sqlx::query(
            r#"
            DELETE FROM maproom.chunks
            WHERE jsonb_array_length(worktree_ids) = 0
            "#
        )
        .execute(&mut **tx)
        .await?
        .rows_affected();

        // Step 3: Delete worktree record
        sqlx::query("DELETE FROM maproom.worktrees WHERE id = $1")
            .bind(worktree_id)
            .execute(&mut **tx)
            .await?;

        Ok(deleted as i64)
    }
}

pub struct CleanupReport {
    pub total_stale: usize,
    pub deleted_count: usize,
    pub chunks_cleaned: i64,
    pub failed_count: usize,
    pub deleted_ids: Vec<i32>,
    pub failed_deletions: Vec<(i32, anyhow::Error)>,
}
```

**Key Design Decisions:**

- **Transaction safety:** All operations in single transaction (all-or-nothing)
- **Dry-run mode:** Allows inspection before actual deletion
- **Audit logging:** Every deletion logged with full context
- **Partial failure handling:** Continue deleting even if one fails
- **Array-based deletion:** Removes worktree from JSONB arrays, NOT CASCADE
- **Reuses incremental pattern:** Same SQL as `remove_worktree_from_chunks()` but worktree-scoped
- **Garbage collection:** Deletes chunks only when worktree_ids becomes empty
- **Multi-worktree safety:** Shared chunks preserved if they belong to other worktrees

### Safety Mechanisms

1. **Dry-run default:** CLI requires explicit `--confirm` flag
2. **Transaction rollback:** Failure aborts entire cleanup
3. **Audit trail:** All deletions logged to tracing system
4. **Pre-validation:** Only delete worktrees that failed existence check
5. **Multi-worktree protection:** Shared chunks preserved via array-based deletion

### Database Schema Constraints (Verified)

**Critical:** The deletion strategy is based on the ACTUAL database schema, verified from migrations:

```sql
-- From migration 0001_init.sql
CREATE TABLE maproom.worktrees (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
  ...
);

CREATE TABLE maproom.files (
  id BIGSERIAL PRIMARY KEY,
  worktree_id BIGINT REFERENCES worktrees(id) ON DELETE SET NULL,  -- NOT CASCADE!
  ...
);

CREATE TABLE maproom.chunks (
  id BIGSERIAL PRIMARY KEY,
  file_id BIGINT NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  ...
);

-- From migration 0020_add_worktree_tracking.sql
ALTER TABLE maproom.chunks ADD COLUMN worktree_ids JSONB DEFAULT '[]';
CREATE INDEX idx_chunks_worktree_ids ON maproom.chunks USING gin(worktree_ids);
```

**Why CASCADE Doesn't Work:**

1. `DELETE FROM worktrees WHERE id = X` triggers:
2. `files.worktree_id` → SET TO NULL (not deleted)
3. `chunks.file_id` still points to file → chunks NOT deleted
4. `chunks.worktree_ids` still contains X (stale reference!)

**Why Array-Based Deletion Works:**

1. Remove X from all `worktree_ids` arrays: `worktree_ids = worktree_ids - 'X'::TEXT`
2. Garbage collect empty arrays: `DELETE WHERE jsonb_array_length(worktree_ids) = 0`
3. Delete worktree record: `DELETE FROM worktrees WHERE id = X`
4. Multi-worktree chunks preserved (e.g., `worktree_ids = [A, B]` → `[B]` after removing A)

**Performance:** GIN index on `worktree_ids` makes `WHERE worktree_ids ? 'X'` queries fast (~10-50ms for millions of chunks).

### Relationship to Existing Incremental Module

**Existing Function:** `incremental/tree_sha_update.rs::remove_worktree_from_chunks()`

```rust
pub async fn remove_worktree_from_chunks(
    client: &Client,
    worktree_id: i64,
    relpath: &str,  // FILE-SPECIFIC
) -> Result<i64>
```

**Purpose:** Remove worktree from chunks when a FILE is deleted during incremental updates.

**Scope:** File-level (specific relpath)

**SQL Pattern:**
```sql
UPDATE chunks SET worktree_ids = worktree_ids - $1::TEXT WHERE relpath = $2;
DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0;
```

**Cleanup Module Needs:** Worktree-level removal (ALL chunks in worktree)

**SQL Pattern (Reused):**
```sql
UPDATE chunks SET worktree_ids = worktree_ids - $1::TEXT WHERE worktree_ids ? $1::TEXT;
DELETE FROM chunks WHERE jsonb_array_length(worktree_ids) = 0;
```

**Key Difference:** WHERE clause changes from `relpath = $2` (file-level) to `worktree_ids ? $1` (worktree-level)

**Decision:** Reuse the SQL pattern, extend the scope. No code duplication, just different granularity of operation.

### Rollback and Recovery Procedures

**Scenario 1: Transaction Rollback (Automatic)**
- **When:** Database error during cleanup
- **Action:** Transaction automatically rolls back
- **Result:** Database unchanged, no data loss
- **User Action:** Fix error (e.g., connection issue), retry

**Scenario 2: Re-index Deleted Worktree (Immediate Recovery)**
- **When:** User realizes cleanup was incorrect immediately after commit
- **Condition:** Worktree still exists on disk
- **Action:** Run `maproom scan --worktree <name>` to re-index
- **Result:** Worktree and chunks restored
- **Limitation:** Only works if worktree files still on disk

**Scenario 3: Database Backup Restoration (Delayed Recovery)**
- **When:** Cleanup error discovered hours/days later
- **Condition:** Database backups available
- **Action:** Restore from backup taken before cleanup
- **Result:** Full database state restored
- **Limitation:** Lose all changes since backup

**Audit Logging Requirements:**
- Log deleted worktree metadata BEFORE deletion (id, name, abs_path, chunk_count)
- Include timestamp and user context
- Store logs persistently (not just stdout)
- Use structured logging (JSON) for easy parsing

**Recommendation:** Consider implementing soft delete in Phase 2+ (mark worktrees as deleted, clean later) for easier recovery.

---

## Component 3: CLI Command Interface

### Responsibility
Expose cleanup functionality via maproom CLI.

### Design

```rust
// crates/maproom/src/cli/commands/db.rs

#[derive(Parser)]
pub struct CleanupStaleCommand {
    /// Perform actual deletion (default: dry-run)
    #[arg(long)]
    confirm: bool,

    /// Show detailed information about each stale worktree
    #[arg(long, short)]
    verbose: bool,
}

impl CleanupStaleCommand {
    pub async fn execute(&self, cfg: &Config) -> Result<()> {
        let db = connect_database(&cfg.database_url).await?;

        // Phase 1: Detection
        println!("🔍 Detecting stale worktrees...");
        let detector = StaleWorktreeDetector::new(db.clone());
        let stale = detector.detect_stale_worktrees().await?;

        if stale.is_empty() {
            println!("✅ No stale worktrees found. Index is clean!");
            return Ok(());
        }

        // Phase 2: Report findings
        println!("\n📊 Found {} stale worktrees:\n", stale.len());
        for wt in &stale {
            println!("  • {} (worktree_id={})", wt.name, wt.id);
            if self.verbose {
                println!("    Path: {}", wt.abs_path);
                println!("    Chunks: {}", wt.chunk_count);
            }
        }

        let total_chunks: i64 = stale.iter().map(|w| w.chunk_count).sum();
        println!("\n💾 Total chunks to delete: {}", total_chunks);

        // Phase 3: Deletion (if confirmed)
        if !self.confirm {
            println!("\n⚠️  This was a dry-run. Use --confirm to actually delete.");
            return Ok(());
        }

        println!("\n🗑️  Deleting stale worktrees...");
        let cleaner = WorktreeCleaner::new(db, false);
        let report = cleaner.cleanup_stale_worktrees(stale).await?;

        println!("✅ Cleanup complete!");
        println!("   Deleted: {} worktrees", report.deleted_count);
        if report.failed_count > 0 {
            println!("   ⚠️  Failed: {} worktrees", report.failed_count);
        }

        Ok(())
    }
}
```

**User Experience:**

```bash
# Dry-run (default)
$ maproom db cleanup-stale
🔍 Detecting stale worktrees...
📊 Found 95 stale worktrees
💾 Total chunks to delete: 487,329
⚠️  This was a dry-run. Use --confirm to actually delete.

# Actual cleanup
$ maproom db cleanup-stale --confirm
🔍 Detecting stale worktrees...
📊 Found 95 stale worktrees
💾 Total chunks to delete: 487,329
🗑️  Deleting stale worktrees...
✅ Cleanup complete! Deleted: 95 worktrees
```

### CLI Integration Points

**main.rs Integration Required** (Phase 2, ticket IDXCLEAN-2004):

1. **Extend DbCommand enum:**
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
```

2. **Add match arm in main():**
```rust
Commands::Db { command } => match command {
    DbCommand::Migrate => {
        let client = db::connect().await?;
        db::migrate(&client).await?;
    }
    DbCommand::CleanupStale { confirm, verbose } => {
        use crewchief_maproom::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

        let client = db::connect().await?;

        // Detection phase
        let detector = StaleWorktreeDetector::new(client.clone());
        let stale = detector.detect_stale_worktrees().await?;

        if stale.is_empty() {
            println!("✅ No stale worktrees found");
            return Ok(());
        }

        // Report phase
        println!("📊 Found {} stale worktrees", stale.len());
        if verbose {
            for wt in &stale {
                println!("  • {} ({})", wt.name, wt.abs_path);
            }
        }

        // Deletion phase (if confirmed)
        if !confirm {
            println!("⚠️  Dry-run. Use --confirm to delete.");
            return Ok(());
        }

        let cleaner = WorktreeCleaner::new(client, false);
        let report = cleaner.cleanup_stale_worktrees(stale).await?;
        println!("✅ Deleted {} worktrees", report.deleted_count);
    }
},
```

3. **Module exports required:**
```rust
// crates/maproom/src/db/mod.rs
pub mod cleanup;
pub use cleanup::{StaleWorktreeDetector, WorktreeCleaner, StaleWorktree, CleanupReport};
```

**Command invocation examples:**
```bash
maproom db cleanup-stale                    # Dry-run
maproom db cleanup-stale --confirm          # Actual deletion
maproom db cleanup-stale --confirm --verbose # With details
```

---

## Component 4: Watch Integration (Optional Enhancement)

### Responsibility
Automatically trigger cleanup during watch command to keep index healthy without manual intervention.

### ✅ WATCH INTEGRATION ANALYSIS COMPLETED

**Status:** Analysis complete. Watch command architecture is well-suited for cleanup integration. No refactoring required.

**Watch Command Architecture** (analyzed from codebase):

**Entry Point:** `main.rs:778` → `indexer::watch_worktree()` (lines 1080-1502)

**Main Components:**
1. **Database Pool** - Connection pool created and validated before starting watchers
2. **WorktreeWatcher** - Filesystem watcher for file changes (`incremental/worktree_watcher.rs`)
3. **HEAD Watcher** - Watches `.git/HEAD` for branch switches (lines 1182-1202)
4. **ChangeDetector** - Detects if files actually changed via hashing
5. **IncrementalProcessor** - Processes file changes and updates database
6. **UpdateQueue** - Queues file processing tasks with stats tracking

**Background Tasks** (3 concurrent tokio tasks):
1. **processor_task** - Main event loop using `tokio::select!` (lines 1222-1394)
   - Handles file change events from `event_rx`
   - Handles branch switch events from `head_rx` (debounced, 2-second window)
   - Dynamically tracks current worktree_id (updated on branch switch)
2. **processing_task** - Processes tasks from UpdateQueue (lines 1396-1426)
3. **status_task** - Periodic status reporting every 10 seconds (lines 1428-1461)

**Event Loop Structure** (lines 1225-1391):
```rust
loop {
    tokio::select! {
        Some(indexing_event) = event_rx.recv() => {
            // Handle file change (Modified/Deleted/Renamed)
            // Normalize paths, detect change type, enqueue tasks
        }
        Some(_head_event) = head_rx.recv() => {
            // Handle branch switch (debounced)
            if debouncer.should_handle() {
                handle_branch_switch(...).await?;
            }
        }
        else => break, // Both channels closed
    }
}
```

**Lifecycle:**
- **Startup:** Database validation → Create watchers → Start 3 background tasks
- **Running:** Event loop processes file/branch events in parallel
- **Shutdown:** SIGINT/SIGTERM → Stop watcher → Wait 2s → Process remaining → Abort tasks

### Integration Hook Points (Analyzed)

**Hook Point #1: Startup Cleanup (RECOMMENDED)**
- **Location:** After database validation (line ~1140), before starting watchers
- **Advantages:**
  - ✅ Database pool available, no indexing in progress
  - ✅ Non-blocking (spawn as background tokio task)
  - ✅ User sees cleanup progress during watch startup
  - ✅ Natural place for "housekeeping" operations
  - ✅ Already has pool access for cleanup operations
- **Implementation:** Spawn `tokio::task` before watcher.start()
- **Recommendation:** Use this for quick startup cleanup (can be skipped if ran recently)

**Hook Point #2: Status Task Extension (RECOMMENDED for Periodic)**
- **Location:** Extend existing status_task loop (lines 1432-1461)
- **Advantages:**
  - ✅ Periodic execution already established (every 10s)
  - ✅ Has access to UpdateQueue stats (can check if idle: `stats.pending == 0`)
  - ✅ Pool available via closure
  - ✅ Existing pattern for background operations
  - ✅ Can add configurable cleanup interval (e.g., every 30 minutes)
- **Implementation:** Add cleanup check inside status_task interval loop
- **Recommendation:** Use this for periodic cleanup with rate limiting

**Hook Point #3: Branch Switch Hook (NOT RECOMMENDED)**
- **Location:** After handle_branch_switch() completes (line 1387)
- **Disadvantages:**
  - ❌ May delay branch switch feedback to user
  - ❌ Could run too frequently if user switches branches often
  - ❌ Cleanup not directly related to branch switching
- **Verdict:** Avoid using this hook point

### Recommended Integration Approach

**Option A: Startup + Optional Periodic (RECOMMENDED)**

This approach requires NO refactoring of existing watch code:

```rust
// In watch_worktree() after pool creation (line ~1140)

// Optional startup cleanup (configurable via env/config)
let enable_auto_cleanup = std::env::var("MAPROOM_AUTO_CLEANUP")
    .unwrap_or_else(|_| "false".to_string())
    .parse::<bool>()
    .unwrap_or(false);

if enable_auto_cleanup {
    // Spawn non-blocking startup cleanup
    let pool_cleanup = pool.clone();
    tokio::spawn(async move {
        use crewchief_maproom::db::cleanup::{StaleWorktreeDetector, WorktreeCleaner};

        match StaleWorktreeDetector::new(pool_cleanup.clone()).detect_stale_worktrees().await {
            Ok(stale) if !stale.is_empty() => {
                tracing::info!("🧹 Startup cleanup: found {} stale worktrees", stale.len());
                match WorktreeCleaner::new(pool_cleanup, false).cleanup_stale_worktrees(stale).await {
                    Ok(report) => tracing::info!("✅ Cleanup complete: {} deleted", report.deleted_count),
                    Err(e) => tracing::warn!("⚠️  Cleanup failed: {}", e),
                }
            }
            Ok(_) => {} // No stale worktrees
            Err(e) => tracing::warn!("Cleanup detection failed: {}", e),
        }
    });
}

// Continue with existing watcher setup...
```

**Extend status_task for periodic cleanup:**

```rust
// In status_task (around line 1432)
let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
let mut last_cleanup: Option<Instant> = None;
let cleanup_interval = Duration::from_secs(30 * 60); // 30 minutes

loop {
    interval.tick().await;
    let stats = queue_clone.lock().await.stats();

    // ... existing status logging ...

    // Periodic cleanup check (if enabled)
    if enable_auto_cleanup {
        let should_cleanup = match last_cleanup {
            None => true,
            Some(instant) => instant.elapsed() > cleanup_interval,
        };

        // Only cleanup if queue is idle and enough time passed
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

**Option B: Standalone Cleanup Task (ALTERNATIVE)**

Add a 4th background task for cleanup (similar to status_task pattern):

```rust
// After starting status_task (around line 1461)

let cleanup_task = if enable_auto_cleanup {
    let pool_cleanup = pool.clone();
    let queue_cleanup = queue.clone();
    Some(tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
        loop {
            interval.tick().await;

            // Check if indexer idle
            let stats = queue_cleanup.lock().await.stats();
            if stats.pending > 0 || stats.processing > 0 {
                tracing::debug!("Skipping cleanup: indexer busy");
                continue;
            }

            // Run cleanup
            // ... (same logic as startup)
        }
    }))
} else {
    None
};

// In shutdown section, abort cleanup_task if exists
if let Some(task) = cleanup_task {
    task.abort();
}
```

**Advantages of Option A (Recommended):**
- ✅ Minimal code changes (extend existing tasks)
- ✅ Reuses existing patterns (tokio::spawn, pool cloning)
- ✅ No new task lifecycle management needed
- ✅ Uses existing queue stats for idle detection

**Advantages of Option B (Alternative):**
- ✅ Cleaner separation of concerns
- ✅ Easier to add cleanup-specific configuration
- ✅ Independent lifecycle from status reporting
- ✅ Can have different interval than status reporting

### No Refactoring Required!

**Key Finding:** The existing watch architecture is perfectly suited for cleanup integration:

- ✅ **Pool-based database access** - Perfect for cleanup module (no connection management needed)
- ✅ **Background task pattern** - Easy to add cleanup as tokio::spawn or extend existing tasks
- ✅ **Queue stats available** - Can check if indexer idle via `stats.pending` and `stats.processing`
- ✅ **Graceful shutdown** - Cleanup tasks can be aborted in existing shutdown sequence
- ✅ **tokio::select! pattern** - Could add cleanup timer arm if needed (Option B)

**Integration Complexity:** Low - Add ~30-50 lines of code, no structural changes.

### Deep Architectural Analysis (Conceptual - Pre-Analysis)

#### Challenge: When to Run Cleanup?

**Option 1: Startup Cleanup**
- **When:** Run cleanup once when `maproom watch` starts
- **Pros:** Simple, guaranteed to run, user sees progress
- **Cons:** Delays watch startup, blocks file watching

**Option 2: Periodic Background Cleanup**
- **When:** Run cleanup every N minutes in background
- **Pros:** Non-blocking, keeps index continuously clean
- **Cons:** More complex, needs scheduling infrastructure

**Option 3: Idle-Time Cleanup**
- **When:** Run cleanup during periods of low file system activity
- **Pros:** Minimal interference with indexing
- **Cons:** Complex to detect idle time, may never run if busy

**Option 4: Post-Indexing Cleanup**
- **When:** Run cleanup after indexing operations complete
- **Pros:** Piggybacks on existing operations
- **Cons:** Couples cleanup to indexing, may run too frequently

**Recommendation: Hybrid Approach**
- **Startup cleanup:** Quick check on watch start (skip if recent)
- **Periodic cleanup:** Every 30 minutes in background
- **Rate limiting:** Skip if cleanup ran in last 15 minutes

#### Challenge: Avoiding Interference with Indexing

**Problem:** Cleanup involves database writes that could conflict with ongoing indexing.

**Solution: Priority-based async execution**

```rust
// Conceptual design for watch integration

pub struct WatchManager {
    indexer: Arc<Indexer>,
    cleanup_scheduler: Arc<CleanupScheduler>,
}

impl WatchManager {
    pub async fn start_watch(&self, path: PathBuf) -> Result<()> {
        // 1. Optional startup cleanup (low priority)
        if self.should_run_startup_cleanup().await? {
            tokio::spawn({
                let scheduler = self.cleanup_scheduler.clone();
                async move {
                    scheduler.run_cleanup_if_safe().await.ok();
                }
            });
        }

        // 2. Start file watcher (high priority)
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        self.start_file_watcher(path, tx).await?;

        // 3. Start periodic cleanup (low priority, background)
        let cleanup_handle = tokio::spawn({
            let scheduler = self.cleanup_scheduler.clone();
            async move {
                scheduler.start_periodic_cleanup().await;
            }
        });

        // 4. Process file events (high priority)
        while let Some(event) = rx.recv().await {
            self.indexer.handle_file_event(event).await?;
        }

        cleanup_handle.abort(); // Stop cleanup when watch ends
        Ok(())
    }
}
```

**Key design principles:**

1. **Non-blocking startup:** Cleanup runs in background tokio task
2. **Priority separation:** File events processed immediately, cleanup waits
3. **Safety checks:** Cleanup checks for active indexing before proceeding
4. **Rate limiting:** Cleanup skips if ran recently

#### Challenge: Efficient Cleanup Execution

**Problem:** Cleanup involves disk I/O and database operations that could be expensive.

**Optimization Strategies:**

**1. Incremental cleanup:**
```rust
pub struct CleanupScheduler {
    last_cleanup: Arc<RwLock<Option<Instant>>>,
    cleanup_interval: Duration,
}

impl CleanupScheduler {
    async fn should_run_cleanup(&self) -> bool {
        let last = self.last_cleanup.read().await;
        match *last {
            None => true, // Never run before
            Some(instant) => instant.elapsed() > self.cleanup_interval,
        }
    }

    async fn run_cleanup_if_safe(&self) -> Result<()> {
        // Check: Has enough time passed?
        if !self.should_run_cleanup().await {
            tracing::debug!("Skipping cleanup: ran recently");
            return Ok(());
        }

        // Check: Is indexing currently active?
        if self.indexer.is_busy().await {
            tracing::debug!("Skipping cleanup: indexer is busy");
            return Ok(());
        }

        // Run cleanup
        let detector = StaleWorktreeDetector::new(self.db.clone());
        let stale = detector.detect_stale_worktrees().await?;

        if stale.is_empty() {
            tracing::debug!("No stale worktrees found");
            return Ok(());
        }

        tracing::info!("Running automatic cleanup: {} stale worktrees", stale.len());
        let cleaner = WorktreeCleaner::new(self.db.clone(), false);
        let report = cleaner.cleanup_stale_worktrees(stale).await?;

        tracing::info!(
            deleted = report.deleted_count,
            failed = report.failed_count,
            "Automatic cleanup complete"
        );

        // Update last cleanup time
        *self.last_cleanup.write().await = Some(Instant::now());

        Ok(())
    }
}
```

**2. Batched validation:**
```rust
// Instead of checking all 100 worktrees every time,
// check in batches of 20 per cleanup cycle

impl StaleWorktreeDetector {
    async fn detect_stale_worktrees_incremental(
        &self,
        batch_size: usize,
    ) -> Result<Vec<StaleWorktree>> {
        // Get next batch of worktrees to check
        let worktrees = self.db
            .query_worktrees_needing_validation(batch_size)
            .await?;

        // Validate this batch only
        let checks = worktrees.into_iter()
            .map(|wt| self.validate_worktree(wt));

        futures::future::join_all(checks).await
            .into_iter()
            .filter_map(|r| r.ok())
            .filter(|wt| !wt.exists)
            .collect()
    }
}
```

**3. Cache validation results:**
```rust
// Cache that a worktree exists to avoid repeated disk checks

pub struct ValidationCache {
    cache: Arc<RwLock<HashMap<i32, (bool, Instant)>>>,
    ttl: Duration,
}

impl ValidationCache {
    async fn is_valid(&self, worktree_id: i32) -> Option<bool> {
        let cache = self.cache.read().await;
        cache.get(&worktree_id)
            .filter(|(_, checked_at)| checked_at.elapsed() < self.ttl)
            .map(|(valid, _)| *valid)
    }

    async fn mark_valid(&self, worktree_id: i32, valid: bool) {
        let mut cache = self.cache.write().await;
        cache.insert(worktree_id, (valid, Instant::now()));
    }
}
```

#### Recommended Integration Architecture

**Phase 1: Startup check (if last cleanup > 1 hour ago)**
```rust
async fn startup_cleanup(&self) -> Result<()> {
    if self.should_run_startup_cleanup().await? {
        // Quick check: only validate worktrees accessed recently
        let detector = StaleWorktreeDetector::new(self.db.clone());
        let stale = detector.detect_recent_stale_worktrees(50).await?;

        if !stale.is_empty() {
            tracing::info!("Running startup cleanup: {} stale worktrees", stale.len());
            let cleaner = WorktreeCleaner::new(self.db.clone(), false);
            cleaner.cleanup_stale_worktrees(stale).await?;
        }
    }
    Ok(())
}
```

**Phase 2: Periodic background (every 30 minutes)**
```rust
async fn start_periodic_cleanup(&self) {
    let mut interval = tokio::time::interval(Duration::from_secs(1800)); // 30 min

    loop {
        interval.tick().await;

        if let Err(e) = self.run_cleanup_if_safe().await {
            tracing::error!("Background cleanup failed: {}", e);
        }
    }
}
```

**Phase 3: Safety checks before execution**
```rust
async fn run_cleanup_if_safe(&self) -> Result<()> {
    // 1. Check: Ran recently?
    if self.last_cleanup_within(Duration::from_secs(900)) { // 15 min
        return Ok(());
    }

    // 2. Check: Indexer busy?
    if self.indexer.active_operations() > 0 {
        tracing::debug!("Deferring cleanup: indexer is busy");
        return Ok(());
    }

    // 3. Check: Database under load?
    if self.db.query_queue_depth() > 10 {
        tracing::debug!("Deferring cleanup: database is busy");
        return Ok(());
    }

    // 4. Run cleanup with low priority
    self.run_cleanup().await
}
```

### Configuration

```toml
# ~/.maproom-mcp/config.toml

[cleanup]
# Enable automatic cleanup during watch
auto_cleanup = true

# How often to run cleanup (seconds)
cleanup_interval = 1800  # 30 minutes

# Minimum time between cleanups (seconds)
cleanup_cooldown = 900  # 15 minutes

# Maximum worktrees to check per cleanup cycle
batch_size = 50
```

### Performance Impact

**Startup cleanup:**
- Detection: ~50ms (50 worktrees, parallel)
- Deletion: ~100ms (transaction + cascade)
- Total impact: ~150ms added to watch startup
- **Acceptable:** User expects initialization time

**Periodic cleanup:**
- Runs in background (non-blocking)
- Only runs when safe (indexer idle)
- Rate limited (max once per 15 min)
- **Impact:** Near zero to user experience

**Worst case:**
- 100 worktrees to validate: ~100ms
- 95 worktrees to delete: ~500ms
- Total: ~600ms every 30 minutes
- **Acceptable:** Runs in background

### Integration with Existing Watch Command

**Minimal changes required:**

```rust
// crates/maproom/src/watch/mod.rs

pub struct WatchCommand {
    // ... existing fields
    cleanup_scheduler: Option<CleanupScheduler>,
}

impl WatchCommand {
    pub async fn execute(&self, config: &Config) -> Result<()> {
        // Optional startup cleanup
        if config.cleanup.auto_cleanup {
            if let Some(scheduler) = &self.cleanup_scheduler {
                scheduler.run_startup_cleanup().await?;
            }
        }

        // Existing watch logic...
        self.start_file_watcher().await?;

        // Start periodic cleanup in background
        if let Some(scheduler) = &self.cleanup_scheduler {
            tokio::spawn({
                let scheduler = scheduler.clone();
                async move {
                    scheduler.start_periodic_cleanup().await;
                }
            });
        }

        // Process file events (unchanged)...
    }
}
```

**Zero impact on existing functionality:** Cleanup is purely additive.

---

## Architecture Decisions

### Decision 1: Disk validation only (no git validation)

**Rationale:**
- Simpler implementation
- Faster execution (~1ms vs ~50ms per worktree)
- Sufficient for MVP (path doesn't exist = stale)
- Git validation can be added later if needed

### Decision 2: CASCADE deletes for chunks

**Rationale:**
- Database handles consistency automatically
- No orphaned chunks left behind
- Transaction safety ensures atomicity
- Standard relational database pattern

### Decision 3: Explicit confirmation for manual cleanup

**Rationale:**
- Safety first: prevent accidental data loss
- Dry-run default allows inspection
- Audit trail provides recovery path
- Industry standard (git prune, docker prune, etc.)

### Decision 4: Hybrid watch integration

**Rationale:**
- Startup cleanup: Handles immediately obvious stale entries
- Periodic cleanup: Keeps index continuously clean
- Rate limiting: Prevents excessive overhead
- Safety checks: Avoids interference with indexing

### Decision 5: Configuration-based auto-cleanup

**Rationale:**
- User control over behavior
- Can disable if causing issues
- Tunable parameters for different environments
- Respects user's system resources

---

## Technology Choices

### Language: Rust
- **Why:** Existing codebase is Rust
- **Benefit:** Type safety for database operations
- **Benefit:** Async/await for efficient I/O

### Database: PostgreSQL
- **Why:** Existing database
- **Benefit:** Transaction support
- **Benefit:** CASCADE foreign keys
- **Benefit:** ACID guarantees

### Async Runtime: Tokio
- **Why:** Existing runtime
- **Benefit:** Parallel disk checks
- **Benefit:** Background task scheduling
- **Benefit:** Non-blocking cleanup during watch

### CLI Framework: clap
- **Why:** Existing CLI uses clap
- **Benefit:** Consistent UX
- **Benefit:** Subcommand structure

---

## Performance Considerations

### Scalability

**Current scale:**
- 100 worktrees in database
- 95 stale worktrees
- ~500,000 stale chunks
- ~2-3 GB database bloat

**Expected scale after cleanup:**
- 5-10 worktrees (active development)
- 0-5 stale worktrees (temporary branches)
- ~50,000 chunks (active code)
- ~300 MB database size

**Future scale:**
- 50 worktrees (large team)
- 10 stale worktrees (frequent branching)
- ~500,000 chunks (large codebase)
- ~3 GB database size

**Architecture scales to:**
- 1,000 worktrees: Detection ~1s, deletion ~5s
- 10,000 worktrees: Would need batch processing
- **Assessment:** Current architecture sufficient for expected scale

### Optimization Opportunities

**If performance becomes issue:**

1. **Database index on abs_path:**
   - Faster worktree lookups
   - Trade-off: Slower inserts (negligible)

2. **Parallel deletion:**
   - Multiple transactions in parallel
   - Trade-off: More complex error handling

3. **Incremental validation:**
   - Cache validation results (5 min TTL)
   - Only revalidate periodically accessed worktrees
   - Trade-off: Slightly stale detection

4. **Lazy cleanup:**
   - Mark stale, delete later in bulk
   - Trade-off: More complex state management

**Current architecture:** Optimize only if metrics show need.

---

## Long-term Maintainability

### Extension Points

1. **Detection strategies:**
   - Current: Disk existence
   - Future: Git worktree list validation
   - Future: Last access time filtering

2. **Deletion policies:**
   - Current: Delete all stale
   - Future: Keep last N days of stale
   - Future: Archive before delete

3. **Integration points:**
   - Current: CLI + watch
   - Future: MCP tool for cleanup
   - Future: Automatic cleanup in search/open

### Code Organization

```
crates/maproom/src/
├── db/
│   ├── cleanup.rs           # Detection + deletion modules
│   └── mod.rs
├── watch/
│   ├── cleanup_scheduler.rs # Watch integration
│   └── mod.rs
└── cli/
    └── commands/
        └── db.rs            # CLI subcommand
```

**Design principles:**
- Separate concerns (detection, deletion, scheduling)
- Testable modules (no global state)
- Minimal coupling (watch doesn't depend on CLI)

---

## Deployment Strategy

### MVP Delivery (Phase 1)

**Deliver in order:**
1. Stale detection module (testable independently)
2. Safe deletion module (with dry-run)
3. CLI command (manual cleanup)

**User value:** Manual cleanup command immediately addresses search quality.

### Optional Enhancement (Phase 2)

**Deliver after MVP proven:**
4. Watch integration (startup cleanup)
5. Periodic background cleanup
6. Configuration options

**User value:** Automatic cleanup keeps index healthy without manual intervention.

### Rollout Plan

**Week 1-2: Core modules**
- Detection + deletion + CLI
- Comprehensive test suite
- Deploy as opt-in beta

**Week 3: Watch integration**
- Startup cleanup only
- Monitor performance impact
- Gather user feedback

**Week 4: Periodic cleanup**
- Background scheduling
- Configuration options
- Production-ready release

---

## Summary

**Architecture highlights:**
- Three independent modules (detection, deletion, CLI)
- Optional fourth module for watch integration
- Safety-first design with dry-run and audit logging
- Efficient implementation using async Rust and parallel I/O
- Non-blocking watch integration with priority-based scheduling

**MVP focus:**
- Manual cleanup command (`maproom db cleanup-stale`)
- Safe deletion with explicit confirmation
- Fast execution (<2s for 100 worktrees)
- Zero data loss guarantee

**Future enhancement:**
- Automatic cleanup during watch
- Configurable cleanup intervals
- Minimal performance impact (<1% overhead)
- Graceful degradation if cleanup fails
