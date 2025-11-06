# Architecture: Watch Change Detection Fix

## Solution Overview

Fix the watch command's file change detection by correcting path normalization in the processor_task and ensuring `ChangeDetector.detect_change()` is always called for modified files. This requires minimal changes to the existing async architecture while maintaining transaction integrity and performance.

## Design Principles

1. **Minimal invasiveness**: Fix the bug without refactoring the entire watch system
2. **Use existing infrastructure**: Leverage ChangeDetector's three-tier comparison
3. **Clear contracts**: Establish unambiguous path format ownership
4. **Fail fast**: Return errors instead of misclassifying file states
5. **Backwards compatible**: Don't break scan/upsert commands

## Architecture Changes

### 1. Path Normalization Strategy

**Problem**: Three incompatible path representations cause lookup failures.

**Solution**: Establish clear ownership and conversion points.

#### Path Format Standards

```rust
// 1. Filesystem paths (from watcher) - ABSOLUTE
/workspace/packages/cli/src/agents/discovery.ts

// 2. Repository paths (in database) - RELATIVE to repo root
packages/cli/src/agents/discovery.ts

// 3. Display paths (for logging) - RELATIVE preferred, absolute fallback
packages/cli/src/agents/discovery.ts
```

#### Conversion Function

Create a single, tested path normalization function:

```rust
/// Convert absolute filesystem path to repository-relative path.
///
/// # Arguments
/// * `absolute_path` - Full filesystem path
/// * `repo_root` - Repository root directory
///
/// # Returns
/// * `Ok(PathBuf)` - Relative path suitable for database queries
/// * `Err(_)` - Path is not within repo root
///
/// # Example
/// ```
/// let abs = Path::new("/workspace/packages/cli/src/main.ts");
/// let root = Path::new("/workspace");
/// let rel = normalize_to_relpath(abs, root)?;
/// assert_eq!(rel.to_str().unwrap(), "packages/cli/src/main.ts");
/// ```
fn normalize_to_relpath(absolute_path: &Path, repo_root: &Path) -> Result<PathBuf> {
    absolute_path
        .strip_prefix(repo_root)
        .map(|p| p.to_path_buf())
        .context("Path is not within repository root")
}
```

**Location**: `crates/maproom/src/incremental/path_utils.rs` (new module)

**Usage**:
- Call ONCE at event receipt in processor_task
- Use relpath for all database queries
- Use relpath for logging
- Keep absolute path for filesystem operations only

### 2. processor_task Refactoring

**Current Flow (Broken)**:
```
FileEvent::Modified(path)
    ↓
get_file_id_by_path(relpath)  # Path mismatch
    ↓
Returns Ok(None)
    ↓
Assumes NEW file
    ↓
index_new_file() fails
```

**New Flow (Fixed)**:
```
FileEvent::Modified(absolute_path)
    ↓
normalize_to_relpath(absolute_path, repo_root)
    ↓
get_file_id_by_path(relpath)
    ↓
file_id found?
    YES → ChangeDetector.detect_change(file_id, absolute_path)
           ↓
           Returns: None | Modified{old, new} | New(hash)
    NO → Return error, don't assume
```

#### Pseudocode

```rust
let processor_task = tokio::spawn(async move {
    while let Some(indexing_event) = event_rx.recv().await {
        // 1. Normalize path ONCE at entry
        let absolute_path = &indexing_event.path;
        let relpath = match normalize_to_relpath(absolute_path, &root_clone) {
            Ok(p) => p,
            Err(e) => {
                warn!(path = %absolute_path.display(), error = %e, "Path outside repo root");
                continue;  // Skip this event
            }
        };

        // 2. Convert event type
        let file_event = match indexing_event.event_type {
            EventType::Modified => FileEvent::Modified(absolute_path.clone()),
            EventType::Deleted => FileEvent::Deleted(absolute_path.clone()),
            EventType::Renamed => {
                if let Some(old_path) = indexing_event.old_path {
                    FileEvent::Renamed(old_path, absolute_path.clone())
                } else {
                    FileEvent::Modified(absolute_path.clone())
                }
            }
        };

        // 3. Detect change type
        let change_type = match file_event {
            FileEvent::Modified(ref path) => {
                // Lookup file_id by relpath
                match get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, relpath.to_str().unwrap()).await {
                    Ok(Some(file_id)) => {
                        // File exists - use ChangeDetector
                        detector_clone.lock().await
                            .detect_change(file_id, path)
                            .await
                            .ok()
                    }
                    Ok(None) => {
                        // File truly doesn't exist - compute new hash
                        if path.exists() {
                            FileHasher::hash_file(path)
                                .ok()
                                .map(|hash| ChangeType::New(hash))
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Database lookup failed");
                        None
                    }
                }
            }
            FileEvent::Deleted(ref path) => {
                match get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, relpath.to_str().unwrap()).await {
                    Ok(Some(file_id)) => {
                        detector_clone.lock().await
                            .detect_deletion(file_id, path)
                            .await
                            .ok()
                            .flatten()
                    }
                    Ok(None) => None,  // File already gone
                    Err(e) => {
                        warn!(path = %path.display(), error = %e, "Database lookup failed");
                        None
                    }
                }
            }
            FileEvent::Renamed(ref _old_path, ref new_path) => {
                // Treat rename as modified (detect change against old file_id if exists)
                let new_relpath = match normalize_to_relpath(new_path, &root_clone) {
                    Ok(p) => p,
                    Err(_) => continue,
                };

                match get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, new_relpath.to_str().unwrap()).await {
                    Ok(Some(file_id)) => {
                        detector_clone.lock().await
                            .detect_change(file_id, new_path)
                            .await
                            .ok()
                    }
                    Ok(None) => {
                        FileHasher::hash_file(new_path)
                            .ok()
                            .map(|hash| ChangeType::New(hash))
                    }
                    Err(_) => None,
                }
            }
        };

        // 4. Enqueue if changes detected
        if let Some(change) = change_type {
            if !matches!(change, ChangeType::None) {
                let task = UpdateTask::new(
                    absolute_path.clone(),
                    change,
                    Trigger::Auto,
                );
                queue_clone.lock().await.enqueue(task);
            }
        }
    }
});
```

### 3. IncrementalProcessor Path Handling

**Current Problem**: `index_new_file()` uses absolute path in database query.

**Solution**: Accept both absolute path (for filesystem) and relpath (for database).

#### Modified Function Signature

```rust
async fn index_new_file(
    &self,
    absolute_path: &Path,
    relpath: &Path,  // NEW: separate relpath for DB queries
    hash: &ContentHash
) -> Result<()> {
    // Read from filesystem using absolute_path
    let content = fs::read_to_string(absolute_path)?;

    // Query database using relpath
    let file_row = client.query_opt(
        "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
        &[&relpath.to_str().unwrap()],  // Use relpath, not absolute
    ).await?;

    // ... rest of logic
}
```

**Alternative**: Keep single path parameter, normalize internally.

```rust
async fn index_new_file(&self, path: &Path, hash: &ContentHash) -> Result<()> {
    let content = fs::read_to_string(path)?;

    // Normalize for database query
    let relpath = path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path"))?
        .trim_start_matches("/workspace/");  // Quick fix, replace with proper function

    let file_row = client.query_opt(
        "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
        &[&relpath],
    ).await?;

    // ... rest
}
```

**Recommendation**: Use alternative approach (normalize internally) to minimize API changes.

### 4. UpdateTask Structure

**Current**: Only stores path.

**Proposed**: Store both absolute and relative paths.

```rust
pub struct UpdateTask {
    pub path: PathBuf,           // Absolute path (for filesystem ops)
    pub relpath: PathBuf,        // Relative path (for database ops) - NEW
    pub change_type: ChangeType,
    pub priority: Priority,
    pub trigger: Trigger,
}

impl UpdateTask {
    pub fn new(
        path: PathBuf,
        relpath: PathBuf,  // NEW parameter
        change_type: ChangeType,
        trigger: Trigger
    ) -> Self {
        Self {
            path,
            relpath,
            change_type,
            priority: Priority::from_trigger(&trigger),
            trigger,
        }
    }
}
```

**Alternative**: Keep single path, normalize on-demand.

**Recommendation**: Use alternative (single path) to avoid refactoring existing code.

## Component Interaction Diagram

```
┌─────────────────┐
│   FileWatcher   │
│  (notify crate) │
└────────┬────────┘
         │ Absolute paths
         ↓
┌─────────────────────────────┐
│   processor_task            │
│                             │
│ 1. normalize_to_relpath()   │
│ 2. get_file_id_by_path()    │
│ 3. ChangeDetector           │
│    .detect_change()         │
│ 4. Enqueue UpdateTask       │
└────────┬────────────────────┘
         │ UpdateTask with ChangeType
         ↓
┌─────────────────────────────┐
│   processing_task           │
│                             │
│ Dequeue → IncrementalProc   │
└────────┬────────────────────┘
         │
         ↓
┌──────────────────────────────────┐
│   IncrementalProcessor           │
│                                  │
│ • index_new_file()    (if New)   │
│ • update_file()       (if Mod)   │
│ • remove_file()       (if Del)   │
└──────────────────────────────────┘
         │
         ↓
┌──────────────────────────────────┐
│   Database (PostgreSQL)          │
│                                  │
│ files (relpath, blake3_hash)     │
│ chunks (file_id, content)        │
└──────────────────────────────────┘
```

## Data Flow

### Successful Modified File Processing

```
1. User modifies file
   /workspace/packages/cli/src/main.ts

2. FileWatcher detects
   EventType::Modified(/workspace/packages/cli/src/main.ts)

3. processor_task receives event
   absolute_path = /workspace/packages/cli/src/main.ts
   relpath = normalize_to_relpath(...) → packages/cli/src/main.ts

4. get_file_id_by_path(repo, worktree, "packages/cli/src/main.ts")
   Returns: Ok(Some(123))

5. ChangeDetector.detect_change(123, /workspace/packages/cli/src/main.ts)
   - Computes current hash: hash_new
   - Checks cache: miss
   - Checks DB: hash_old
   - Returns: ChangeType::Modified{old: hash_old, new: hash_new}

6. Enqueue UpdateTask
   path: /workspace/packages/cli/src/main.ts
   change_type: Modified{...}

7. processing_task dequeues

8. IncrementalProcessor.process(task)
   Calls: update_file(path, new_hash)

9. update_file() transaction
   - BEGIN
   - DELETE FROM chunks WHERE file_id = 123
   - Parse file, INSERT chunks
   - UPDATE files SET blake3_hash = hash_new WHERE id = 123
   - COMMIT

10. Success ✅
```

### New File Processing

```
1. User creates file
   /workspace/packages/cli/src/new.ts

2. FileWatcher detects
   EventType::Modified (initial event for new file)

3. processor_task
   relpath = packages/cli/src/new.ts

4. get_file_id_by_path(...)
   Returns: Ok(None)  # File not in database

5. Compute hash directly (bypass ChangeDetector)
   hash = FileHasher::hash_file(path)
   Returns: ChangeType::New(hash)

6. Enqueue UpdateTask

7. IncrementalProcessor.index_new_file(path, hash)
   - Query: SELECT id FROM files WHERE relpath = 'packages/cli/src/new.ts'
   - Should return None (file truly doesn't exist)
   - ERROR: "File not found in database"

   NOTE: This is expected! Watch command doesn't create file records.
   Need to clarify: who creates them?
```

**Question for investigation**: How do new files get added to the files table during watch?

## Edge Cases

### 1. Temporary Files from Editors

**Scenario**: Editor creates .tmp file, writes content, renames to actual file.

```
Events:
1. Create: /workspace/src/main.ts.tmp
2. Modify: /workspace/src/main.ts.tmp
3. Rename: /workspace/src/main.ts.tmp → /workspace/src/main.ts
```

**Handling**:
- Events 1-2: Ignored (file not in database)
- Event 3: Treated as Modified
  - get_file_id_by_path("src/main.ts") → file_id
  - detect_change(file_id, /workspace/src/main.ts)
  - Correctly classifies as Modified or None

### 2. Rapid Successive Changes

**Scenario**: User saves file multiple times quickly.

```
Events (within 2s throttle):
1. Modify: main.ts (hash A → B)
2. Modify: main.ts (hash B → C)
```

**Handling**:
- Debouncer batches events
- Only latest event processed
- ChangeDetector compares: hash_db (A) vs hash_current (C)
- Returns: Modified{old: A, new: C}
- Single transaction updates to C

### 3. File Deleted During Processing

**Scenario**: File queued for indexing but deleted before processing.

```
1. Modify detected, queued
2. User deletes file
3. processing_task dequeues
4. IncrementalProcessor.update_file() called
5. fs::read_to_string() fails
```

**Handling**:
- Return error from update_file()
- processing_task catches error
- File marked as failed, retried
- Eventually moves to dead letter queue
- Accept this as correct behavior (file legitimately gone)

### 4. Multiple Files Changed Simultaneously

**Scenario**: Git checkout changes 100 files.

**Handling**:
- All events received by processor_task
- All debounced (2s window)
- All classified by ChangeDetector (parallel via async)
- All enqueued in UpdateQueue
- processing_task processes sequentially
- Each file gets atomic transaction
- Total time: ~100-500ms per file = 10-50s for 100 files

**Performance**: Acceptable for watch use case. Scan command better for large changes.

### 5. Path Outside Repository

**Scenario**: Watcher detects change to /etc/hosts (symlink in repo?).

**Handling**:
```rust
let relpath = match normalize_to_relpath(absolute_path, &root_clone) {
    Ok(p) => p,
    Err(e) => {
        warn!("Path outside repo: {}", absolute_path.display());
        continue;  // Skip event
    }
};
```

## Performance Considerations

### Current Performance

- File watcher overhead: <1ms per event
- Debouncing delay: 500ms (configurable)
- ChangeDetector (per file):
  - Cache hit: < 1ms
  - DB lookup: 1-5ms
  - Filesystem hash: 5-10ms
- Transaction processing: 100-800ms per file

### Proposed Performance

**No change** - Same operations, just correct logic flow.

**Optimization opportunities** (out of scope for this fix):
1. Batch database lookups (get_file_id_by_path for multiple files)
2. Parallel transaction processing (thread pool)
3. Pre-warm ChangeDetector cache on watch startup

## Testing Strategy

### Unit Tests

1. **Path normalization**:
   - Absolute → relative conversion
   - Handles trailing slashes
   - Rejects paths outside repo
   - Windows vs Unix paths

2. **processor_task logic** (mock test):
   - Modified file → ChangeType::Modified
   - New file → ChangeType::New
   - Deleted file → ChangeType::Deleted
   - get_file_id_by_path error → None

### Integration Tests

1. **Single file modification**:
   - Start watch
   - Modify file
   - Assert: UpdateTask queued with ChangeType::Modified
   - Assert: Database updated correctly

2. **Multiple file modification**:
   - Start watch
   - Modify 3 files simultaneously
   - Assert: All 3 queued with correct ChangeType
   - Assert: All 3 indexed successfully

3. **Temp file sequence**:
   - Start watch
   - Create .tmp file
   - Rename to actual file
   - Assert: Classified as Modified
   - Assert: Database updated

4. **Rapid changes**:
   - Start watch
   - Modify file 3 times in 1 second
   - Assert: Single UpdateTask (debounced)
   - Assert: Final state indexed

### End-to-End Tests

1. **Watch integration test**:
   ```rust
   #[tokio::test]
   async fn test_watch_multi_file_modification() {
       // Setup database
       let pool = test_pool().await;
       seed_test_data(&pool).await;

       // Start watch
       let watch_handle = tokio::spawn(async move {
           watch_worktree(pool, "test_repo", "main", "/tmp/test").await
       });

       // Modify files
       modify_file("/tmp/test/src/a.rs", "// comment 1");
       modify_file("/tmp/test/src/b.rs", "// comment 2");
       modify_file("/tmp/test/src/c.rs", "// comment 3");

       // Wait for processing
       tokio::time::sleep(Duration::from_secs(5)).await;

       // Assert all indexed
       let timestamps = query_chunk_timestamps(&pool, vec!["src/a.rs", "src/b.rs", "src/c.rs"]).await;
       assert!(timestamps.iter().all(|ts| ts > start_time));

       // Cleanup
       watch_handle.abort();
   }
   ```

## Rollback Plan

If fix causes regressions:

1. **Revert commit** containing the fix
2. **Fallback behavior**: Disable watch command entirely
   - Add flag: `MAPROOM_DISABLE_WATCH=1`
   - Return error: "Watch temporarily disabled, use scan instead"
3. **Investigation**: Collect logs from failed watch sessions
4. **Re-fix**: Address regressions, re-test, re-deploy

## Migration Path

**No migration needed** - this is a bug fix, not a feature change.

**Backwards compatibility**:
- Scan command unchanged
- Upsert command unchanged
- Database schema unchanged
- API unchanged

**Deployment**:
- Build new binary
- Replace in packages/cli/bin/<platform>/
- No config changes
- No data migration

## Success Metrics

1. **Correctness**:
   - 100% of modified files classified as Modified (not New)
   - 100% of modified files successfully indexed
   - 0% retry loops for valid file changes

2. **Performance**:
   - < 1s from file modification to database update (single file)
   - < 10s for 10 files modified simultaneously
   - No regression vs current scan performance

3. **Reliability**:
   - 0% false positives (New when should be Modified)
   - 0% false negatives (None when should be Modified)
   - < 1% acceptable failures (file deleted during processing, etc.)

## Open Questions

1. **Who creates file records during watch?**
   - Currently assumes files exist
   - What if truly new file created?
   - Need to add file record creation logic?

2. **Should we batch database lookups?**
   - get_file_id_by_path called once per file
   - Could batch into single query for multiple files
   - Optimization for later?

3. **Error handling for path normalization failures?**
   - Skip event silently?
   - Log warning?
   - Count metrics?

4. **Cache warming strategy?**
   - ChangeDetector cache empty on watch start
   - Should we pre-populate from database?
   - Trade-off: startup time vs first-event latency

## Conclusion

The fix is architecturally sound, minimally invasive, and leverages existing infrastructure. The main changes are:

1. Add path normalization utility
2. Fix processor_task to always call ChangeDetector for Modified events
3. Fix IncrementalProcessor to use relpath for database queries

Risk is low, testing is straightforward, and rollback is trivial. The solution maintains the async architecture, preserves transaction integrity, and doesn't introduce performance regressions.
