# Analysis: Watch Command Change Detection Bug

## Executive Summary

The maproom watch command has a critical bug in its file change detection logic that prevents modified files from being correctly indexed. When multiple files are changed simultaneously, the system detects all changes but **misclassifies existing modified files as NEW files**, causing indexing to fail with "File not found in database" errors. All three test files were detected, queued, and processed, but zero were successfully indexed.

## Problem Statement

### Observed Behavior

When modifying 3 files simultaneously while watch command is running:

**✅ Detection Works:**
- All 3 files detected by file watcher
- All 3 enqueued for processing
- Debouncing works correctly (2s throttle)

**❌ Indexing Fails:**
- All 3 files dequeued and processed
- All 3 classified as `ChangeType::New` (WRONG)
- All 3 fail with "File not found in database" error
- Files enter retry loop, never successfully indexed
- Database timestamps unchanged (no re-indexing occurred)

### Expected Behavior

Modified files should be classified as `ChangeType::Modified{old, new}` and successfully re-indexed with updated content and timestamps in the database.

## Root Cause Analysis

### The Bug Location

**File**: `crates/maproom/src/indexer/mod.rs`
**Function**: `watch_worktree()` - `processor_task` (lines 658-724)
**Specific Issue**: Lines 678-694

### The Problematic Code

```rust
// Detect change type
let change_type = match file_event {
    FileEvent::Modified(ref path) => {
        // Try to get file_id from database
        if let Ok(Some(file_id)) = get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, relpath.to_str().unwrap()).await {
            detector_clone.lock().await.detect_change(file_id, path).await.ok()
        } else {
            // New file - compute hash
            if path.exists() {
                if let Ok(hash) = crate::incremental::FileHasher::hash_file(path) {
                    Some(crate::incremental::ChangeType::New(hash))  // ❌ MISCLASSIFICATION
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
    // ...
};
```

### Why It Fails

1. **`get_file_id_by_path()` lookup uses wrong relpath**
   - Converts: `/workspace/packages/cli/src/agents/discovery.ts`
   - Using: `path.strip_prefix(&root_clone)` (line 675)
   - Should produce: `packages/cli/src/agents/discovery.ts`
   - But database query fails to find it

2. **Falls through to "New file" branch**
   - When `get_file_id_by_path()` returns `Ok(None)`, assumes file is new
   - Computes hash and creates `ChangeType::New(hash)`
   - This is WRONG for existing files

3. **`index_new_file()` expects file record to NOT exist**
   - Queries: `SELECT id FROM maproom.files WHERE relpath = $1` (processor.rs:211)
   - Uses absolute path `/workspace/...` instead of relpath
   - File DOES exist (with relpath `packages/cli/...`)
   - Query fails, hits error at line 222: `"File not found in database"`

### The Cascade of Failures

```
Modified file event
    ↓
get_file_id_by_path() uses wrong relpath format
    ↓
Returns Ok(None) - "file not found"
    ↓
Misclassified as ChangeType::New
    ↓
Sent to index_new_file()
    ↓
index_new_file() queries with wrong path format
    ↓
File exists but query uses different path format
    ↓
Fails with "File not found in database"
    ↓
File re-queued, enters retry loop
    ↓
Never successfully indexed
```

## Evidence from Investigation

### Test Scenario
- Modified 3 TypeScript files: `discovery.ts`, `registry.ts`, `runner.ts`
- Added comment lines to each file
- Watched with `RUST_LOG=debug`

### Log Analysis

**File Detection (Lines working correctly):**
```
2025-11-05T16:12:45.837679Z DEBUG Enqueueing new task for path: /workspace/packages/cli/src/agents/discovery.ts (priority: Low)
2025-11-05T16:12:45.838355Z DEBUG Enqueueing new task for path: /workspace/packages/cli/src/agents/runner.ts (priority: Low)
2025-11-05T16:12:45.839053Z DEBUG Enqueueing new task for path: /workspace/packages/cli/src/agents/registry.ts (priority: Low)
```

**File Processing (Lines showing bug):**
```
2025-11-05T16:12:45.913509Z DEBUG Dequeued task for path: /workspace/packages/cli/src/agents/discovery.ts (priority: Low)
2025-11-05T16:12:45.913523Z DEBUG Processing update task path=/workspace/packages/cli/src/agents/discovery.ts
    change_type=New(Hash("c380ead..."))  # ❌ WRONG - should be Modified
2025-11-05T16:12:45.914327Z WARN Failed to process file path=/workspace/packages/cli/src/agents/discovery.ts
    error=Failed to index new file: /workspace/packages/cli/src/agents/discovery.ts
```

**Database Queries (Lines showing path mismatch):**
```
executing statement s3 with parameters: ["crewchief", "main", "packages/cli/src/agents/discovery.ts.tmp.98369.1762359155590"]
executing statement s5 with parameters: ["crewchief", "main", "packages/cli/src/agents/discovery.ts"]
```

**Database State (Proving no indexing occurred):**
```sql
SELECT f.relpath, MAX(c.updated_at) as last_update
FROM files f LEFT JOIN chunks c ON f.id = c.file_id
WHERE f.relpath IN (...)

# Result: timestamps from Nov 4 (unchanged)
packages/cli/src/agents/discovery.ts | 2025-11-04 21:19:47.742946+00
packages/cli/src/agents/registry.ts  | 2025-11-04 21:19:47.742263+00
packages/cli/src/agents/runner.ts    | 2025-11-04 21:19:48.606887+00
```

### Status Report Analysis
```
Watch status files_watched=5194 watcher_state="running"
queue_size=0 processing=3 dead_letter=0 total_processed=9
```

- `processing=3`: All 3 files stuck in processing state
- `total_processed=9`: 3 files × 3 retry attempts = 9 total attempts
- `queue_size=0`: No new work, just retries
- `dead_letter=0`: Files not dead, just looping

## Path Format Inconsistency

### The Core Issue: Three Different Path Representations

The system uses three incompatible path representations:

1. **Absolute filesystem path**: `/workspace/packages/cli/src/agents/discovery.ts`
   - Used by: file watcher, event processing, IncrementalProcessor

2. **Repository-relative path**: `packages/cli/src/agents/discovery.ts`
   - Stored in: `maproom.files.relpath` column
   - Used by: database queries, get_file_id_by_path()

3. **Temporary file path**: `packages/cli/src/agents/discovery.ts.tmp.98369.1762359155590`
   - Created by: text editors during save operations
   - Detected by: file watcher before rename

### Where Conversions Fail

**processor_task (line 675):**
```rust
let relpath = indexing_event.path.strip_prefix(&root_clone).unwrap_or(&indexing_event.path);
```
- Correctly strips `/workspace/` → `packages/cli/...`
- Passed to `get_file_id_by_path()`

**get_file_id_by_path() (line 849):**
```rust
WHERE r.name = $1 AND w.name = $2 AND f.relpath = $3
```
- Receives relpath: `packages/cli/src/agents/discovery.ts`
- Should find record in database ✅
- But sometimes receives absolute path instead ❌

**index_new_file() (line 206-211):**
```rust
let relpath = path.to_string_lossy();  // ❌ Uses absolute path!

let file_row = client.query_opt(
    "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
    &[&relpath.as_ref()],  // ❌ Absolute path doesn't match DB relpath
).await?;
```
- Uses absolute path: `/workspace/packages/cli/...`
- Database has relpath: `packages/cli/...`
- Query returns no rows
- Fails at line 222: "File not found in database"

## Change Detection Architecture

### How It Should Work

The `ChangeDetector` uses a three-tier comparison strategy:

1. **In-memory cache**: Fast HashMap lookup (< 1μs)
2. **Database lookup**: Query file hash by file_id (~1-5ms)
3. **Filesystem hash**: Compute blake3 hash (~5-10ms)

```rust
// detector.rs:152-196
pub async fn detect_change(&mut self, file_id: i64, path: &Path) -> Result<ChangeType> {
    let current_hash = FileHasher::hash_file(path)?;  // Step 3

    if let Some(cached_hash) = self.cache.get(path) { // Step 1
        if *cached_hash == current_hash {
            return Ok(ChangeType::None);
        } else {
            return Ok(ChangeType::Modified { old: *cached_hash, new: current_hash });
        }
    }

    let db_hash = get_hash_from_db(&self.pool, file_id).await?;  // Step 2

    match db_hash {
        Some(old_hash) => {
            if old_hash == current_hash {
                ChangeType::None
            } else {
                ChangeType::Modified { old: old_hash, new: current_hash }
            }
        }
        None => ChangeType::New(current_hash)
    }
}
```

### How It Actually Fails

**In watch_worktree() processor_task:**
```rust
// Line 681: Try to get file_id
if let Ok(Some(file_id)) = get_file_id_by_path(...).await {
    // SUCCESS PATH: Call detector
    detector_clone.lock().await.detect_change(file_id, path).await.ok()
} else {
    // FAILURE PATH: Assume new file
    if path.exists() {
        if let Ok(hash) = FileHasher::hash_file(path) {
            Some(ChangeType::New(hash))  // ❌ WRONG ASSUMPTION
        }
    }
}
```

**The problem:**
- `get_file_id_by_path()` fails due to path format mismatch
- Falls through to "file not found" branch
- Incorrectly assumes file is new
- Never calls `ChangeDetector.detect_change()`
- Bypasses the three-tier comparison entirely

## IncrementalProcessor Expectations

### What `index_new_file()` Expects

Per processor.rs:204-223, `index_new_file()` has this logic:

```rust
// Query to find the file record
let file_row = client.query_opt(
    "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
    &[&relpath.as_ref()],
).await?;

let file_id = match file_row {
    Some(row) => row.get::<_, i64>(0),
    None => {
        // File doesn't exist in DB yet - this is an error condition
        // The file should have been created by the watcher before queueing
        anyhow::bail!("File not found in database: {}", path.display());
    }
};
```

**Comment at line 204 is revealing:**
> "For new files, we need to find the repo/worktree/commit context"

**Comment at line 220-221:**
> "File doesn't exist in DB yet - this is an error condition
> The file should have been created by the watcher before queueing the update"

### The Semantic Mismatch

`ChangeType::New` has TWO different meanings:

1. **ChangeDetector meaning**: "File has no previous hash in database"
   - Could be truly new file
   - Could be existing file with no blake3_hash set yet

2. **IncrementalProcessor meaning**: "File record exists in database"
   - Expects to find file_id by querying relpath
   - Will parse and insert chunks
   - Will update blake3_hash column

**The mismatch:**
- watch_worktree sends `ChangeType::New` for **existing files with records**
- index_new_file() queries for file record, finds it, but uses wrong path format
- Fails with "File not found" even though file exists

## Industry Solutions & Best Practices

### File Watching Patterns

**1. Chokidar (Node.js)** - Event debouncing with ready state:
```javascript
watcher
  .on('ready', () => console.log('Initial scan complete'))
  .on('change', (path) => handleChange(path))
  .on('add', (path) => handleAdd(path))
```
- Separate add/change events
- Ready event after initial scan
- Clear path normalization

**2. Watchman (Facebook)** - State-based file tracking:
```json
{
  "expression": ["allof",
    ["type", "f"],
    ["not", "empty"]
  ],
  "fields": ["name", "size", "mtime", "exists", "new"]
}
```
- Explicit "new" field vs modified
- State tracking in database
- Subscription-based updates

**3. fswatch (Cross-platform)** - Event filtering:
```bash
fswatch -r /path --event Updated --event Created
```
- Separate event types
- Batch processing with latency
- Path canonicalization

**4. notify-rs (Rust)** - Used by maproom:
```rust
match event.kind {
    EventKind::Create(_) => handle_create(),
    EventKind::Modify(ModifyKind::Data(_)) => handle_modify(),
    EventKind::Remove(_) => handle_remove(),
}
```
- Fine-grained event types
- Path normalization required by consumer
- No built-in debouncing

### Change Detection Patterns

**1. Git** - Hash-based change detection:
```
Index state: SHA1 of file content
Working tree: Compute SHA1, compare
```
- Single source of truth (hash)
- No ambiguity about file state
- Database lookup by path → hash

**2. Rsync** - Modification time + size:
```
if (mtime differs || size differs):
    compute checksum
    compare checksums
```
- Quick pre-check before hashing
- Fallback to content hash
- Clear new vs modified logic

**3. Bazel** - Content-addressable storage:
```
path → file_id (stable)
file_id → content_hash (changes)
```
- Stable identifiers
- Hash comparisons only
- No path-based lookups in hot path

## Current Project State

### Existing Infrastructure

**1. File Watcher (`crates/maproom/src/incremental/watcher.rs`)**:
- Uses `notify` crate
- Handles Create/Modify/Remove events
- Debouncing with configurable throttle (default 500ms)
- Works correctly ✅

**2. ChangeDetector (`crates/maproom/src/incremental/detector.rs`)**:
- Three-tier comparison (cache/DB/filesystem)
- Returns: None, New, Modified, Deleted
- Fast and accurate ✅
- **BUT:** Not being called for Modified events in watch ❌

**3. IncrementalProcessor (`crates/maproom/src/incremental/processor.rs`)**:
- Atomic transactions
- Handles New/Modified/Deleted
- Edge relationship updates
- Works correctly when called with right ChangeType ✅
- **BUT:** Assumes certain preconditions about path formats ❌

**4. watch_worktree() (`crates/maproom/src/indexer/mod.rs:561-834`)**:
- Async architecture with 3 tasks:
  - processor_task: Event → ChangeType conversion
  - processing_task: ChangeType → Database updates
  - status_task: Periodic reporting
- **BUG HERE:** processor_task misclassifies Modified as New ❌

### Why Previous Work (MRPROG) Avoided This

The MRPROG project (Maproom Progress UX) specifically **skipped** watch command improvements:

From `.agents/archive/projects/MRPROG_maproom-progress-ux/tickets/MRPROG-2001_watch-minimal-output.md`:

> **Decision: Close this ticket**
> The current `watch_worktree()` implementation uses a sophisticated async architecture with separate event processor, task queue, and status reporting tasks. Output is handled via `info!()` logging macros, not direct `println!()` statements.
>
> Implementing minimal vs verbose output modes would require architectural changes to thread output through the async task system. This is significant refactoring that goes beyond the scope of a UX polish ticket.

**Key insight**: They recognized the watch command has architectural complexity and avoided touching it. We now face the actual logic bug that causes the symptoms they observed.

## Complexity Assessment

### Why This is Not Simple

**1. Async Task Architecture**
- Three separate async tasks communicate via channels
- State shared via Arc<Mutex<>>
- Error handling across async boundaries
- Can't just "add a line" - need to understand flow

**2. Path Normalization Inconsistency**
- Absolute paths from file watcher
- Relative paths in database
- Temporary file paths from editors
- Multiple conversion points with different assumptions

**3. Semantic Overloading**
- `ChangeType::New` means different things in different contexts
- `index_new_file()` expects file record to exist for "new" files
- ChangeDetector returns New when no hash exists (but file record might)

**4. Database State Assumptions**
- Files must exist in DB before indexing
- But "new file" workflow expects to find them
- Unclear who creates file records and when

**5. Error Recovery**
- Retry logic re-queues failed tasks
- But doesn't fix root cause
- Creates infinite retry loops

### Risk Areas

**1. Race Conditions**
- File modified between detection and processing
- Multiple events for same file (create → modify)
- Temp file → rename sequences

**2. Data Integrity**
- Partial index updates if transaction fails
- Orphaned chunks if file deleted during processing
- Hash cache inconsistency

**3. Performance**
- Lock contention on shared ChangeDetector
- Database query for every file event
- No batching of updates

## Key Insights

1. **The bug is NOT in file detection** - watcher works perfectly
2. **The bug is NOT in ChangeDetector** - three-tier comparison is sound
3. **The bug is NOT in IncrementalProcessor** - indexing logic is correct
4. **The bug IS in watch_worktree processor_task** - wrong ChangeType classification

5. **Root cause**: Path format mismatch causes `get_file_id_by_path()` to fail, leading to misclassification as new file

6. **Compounding factor**: `index_new_file()` has inconsistent path handling, expects file record to exist

7. **This is NOT a simple fix** - requires understanding async architecture, path normalization, and semantic contracts between components

8. **Previous attempts avoided this** - MRPROG team recognized complexity and skipped watch improvements

## Recommended Investigation Areas

Before designing a fix, we need to understand:

1. **Path normalization strategy**
   - Where should absolute → relative conversion happen?
   - Who owns path format for each subsystem?
   - How to handle temp files from editors?

2. **File record lifecycle**
   - When are file records created?
   - Who creates them during watch?
   - What's the contract between watcher and processor?

3. **ChangeType semantics**
   - What does "New" really mean?
   - Should it mean "no file record" or "no hash"?
   - How to distinguish truly new files from first-time indexing?

4. **Error handling strategy**
   - Should we retry on path mismatch?
   - How to detect vs recover from this class of error?
   - What's the correct escalation path?

5. **Testing strategy**
   - How to test multi-file changes reliably?
   - How to test temp file → rename sequences?
   - How to verify database state after failures?

## Success Criteria

A successful fix must:

1. ✅ Correctly detect file changes (already works)
2. ✅ Correctly classify as Modified, not New
3. ✅ Successfully re-index all changed files
4. ✅ Update database timestamps and content
5. ✅ Handle multiple simultaneous changes
6. ✅ Handle temp file → rename sequences from editors
7. ✅ Maintain transaction integrity
8. ✅ No infinite retry loops
9. ✅ Performance: < 1s per file for typical changes
10. ✅ Clear logging at info level (not just debug)

## Conclusion

This is a well-bounded, architecturally isolated bug with clear root cause and evidence. The fix requires careful attention to path normalization and semantic contracts, but doesn't require changing external APIs or refactoring the entire watch system. The complexity is manageable for a focused effort with proper testing.
