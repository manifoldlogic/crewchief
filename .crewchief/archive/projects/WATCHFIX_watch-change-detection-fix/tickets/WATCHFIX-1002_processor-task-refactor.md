# Ticket: WATCHFIX-1002: Refactor processor_task to Fix Change Detection Logic

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
Refactor the `processor_task` async function in `watch_worktree()` to correctly classify modified files by normalizing paths once at entry and always calling `ChangeDetector.detect_change()` for Modified events. This is the core fix that eliminates the misclassification bug.

## Background
Currently, `processor_task` (lines 658-724 in `crates/maproom/src/indexer/mod.rs`) fails to normalize paths before database lookups. When `get_file_id_by_path()` is called with an absolute path but the database has relative paths, it returns `None`. The code then incorrectly assumes the file is NEW and computes a hash, bypassing `ChangeDetector` entirely. This misclassification causes `index_new_file()` to fail since the file record actually exists.

The fix involves normalizing paths immediately at event entry, using the normalized relpath for all database queries, and always calling `ChangeDetector.detect_change()` for Modified events where a file_id is found.

This ticket implements **Phase 2: Core Fix - processor_task Refactoring** from the WATCHFIX project plan.

## Acceptance Criteria
- [x] `processor_task` normalizes paths once at event entry using `normalize_to_relpath()`
- [x] All `get_file_id_by_path()` calls use normalized relpath (not absolute path)
- [x] `ChangeDetector.detect_change()` called for every Modified event with valid file_id
- [x] Only fall back to `ChangeType::New` when `get_file_id_by_path()` returns `Ok(None)`
- [x] Path normalization failures logged as warnings, event skipped gracefully
- [x] Database lookup errors logged as warnings, event skipped gracefully
- [x] Code compiles without warnings

## Technical Requirements

### File to Modify
- **File**: `crates/maproom/src/indexer/mod.rs`
- **Function**: `watch_worktree()` -> `processor_task` closure (lines 658-724)

### Required Changes

1. **Import normalize_to_relpath** at top of file:
   ```rust
   use crate::incremental::path_utils::normalize_to_relpath;
   ```

2. **Normalize path at event entry** (beginning of processor_task match arm):
   ```rust
   let relpath = match normalize_to_relpath(&indexing_event.path, &root_clone) {
       Ok(p) => p,
       Err(e) => {
           warn!(path = %indexing_event.path.display(), error = %e, "Path outside repo");
           continue; // Skip event
       }
   };
   ```

3. **Update get_file_id_by_path calls** to use relpath:
   ```rust
   match get_file_id_by_path(&pool_clone, &repo_clone, &worktree_clone, relpath.to_str().unwrap()).await {
       Ok(Some(file_id)) => {
           // ALWAYS call ChangeDetector for Modified events
           detector_clone.lock().await.detect_change(file_id, &indexing_event.path).await.ok()
       }
       Ok(None) => {
           // Truly new file - compute hash directly
           if indexing_event.path.exists() {
               FileHasher::hash_file(&indexing_event.path).ok().map(|h| ChangeType::New(h))
           } else { None }
       }
       Err(e) => {
           warn!(path = %indexing_event.path.display(), error = %e, "DB lookup failed");
           None
       }
   }
   ```

4. **Add code comments** explaining:
   - Why we normalize paths (database uses relative paths)
   - The decision logic for Modified vs New
   - Reference to the bug this fixes

5. **Error handling**:
   - Use `warn!` level logging (visible without RUST_LOG=debug)
   - Continue processing other files on individual failures
   - Don't crash the watch task

## Implementation Notes

### Key Principles
- **Use absolute path** for ChangeDetector (filesystem operations like hashing)
- **Use relpath** for database queries (database stores relative paths)
- **Don't change async architecture** - just fix the logic
- **Preserve existing error handling patterns** - enhance, don't replace

### Path Flow
1. Event arrives with absolute path
2. Normalize to relpath for database lookup
3. If file_id found, pass absolute path to ChangeDetector
4. ChangeDetector uses absolute path for filesystem operations

### Critical Fix
The core bug is that we were calling `get_file_id_by_path()` with an absolute path when the database expects a relative path. This caused the lookup to return `None` even for existing files, leading to misclassification as NEW.

### Logging Strategy
- **warn!** for path normalization failures (paths outside repo)
- **warn!** for database lookup failures
- **info!** or **debug!** for successful operations (don't spam logs)

## Dependencies
- **WATCHFIX-1001** - Requires `normalize_to_relpath()` function from path_utils module

## Risk Assessment

### Risk: Async locking on ChangeDetector could cause contention
- **Mitigation**: Existing `Arc<Mutex<ChangeDetector>>` architecture handles this correctly. The mutex is only held briefly during `detect_change()` calls, which are already async-aware.

### Risk: Error handling changes might affect retry logic
- **Mitigation**: Test thoroughly with integration tests (WATCHFIX-1005). Current implementation uses `ok()` to convert errors to `None`, which skips the event. This is appropriate for unrecoverable errors (path outside repo, database failure).

### Risk: Windows path compatibility
- **Mitigation**: `normalize_to_relpath()` (from WATCHFIX-1001) handles cross-platform path normalization. Unit tests in WATCHFIX-1001 verify Windows path handling.

## Files/Packages Affected

### Modified Files
- `crates/maproom/src/indexer/mod.rs` (~70 lines changed in processor_task closure)
  - Lines 658-724: processor_task refactoring
  - Top of file: Add import for normalize_to_relpath

### Unchanged Files (Reference Only)
- `crates/maproom/src/incremental/path_utils.rs` (created in WATCHFIX-1001)
- `crates/maproom/src/incremental/processor.rs` (will be updated in WATCHFIX-1003)

## Planning References
- **Analysis**: `/workspace/.crewchief/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md` (Root Cause Analysis section, lines 658-724 analysis)
- **Architecture**: `/workspace/.crewchief/projects/WATCHFIX_watch-change-detection-fix/planning/architecture.md` (processor_task Refactoring section with pseudocode)
- **Plan**: `/workspace/.crewchief/projects/WATCHFIX_watch-change-detection-fix/planning/plan.md` (Phase 2 deliverables, lines 59-95)

## Estimated Effort
6 hours

## Priority
CRITICAL - This is the core bug fix that resolves the misclassification issue

## Verification Notes
After implementation, verify:
1. Code compiles without warnings (`cargo build`)
2. Unit tests pass (`cargo test`)
3. Logging output shows normalized paths being used
4. No panics or crashes during watch operations
5. Integration tests (WATCHFIX-1005) will verify end-to-end functionality

## Implementation Notes

**Completed**: 2025-11-06

**Changes Made**:

1. **Added import** (line 9):
   - `use crate::incremental::path_utils::normalize_to_relpath;`

2. **Refactored processor_task** (lines 660-779):
   - Path normalization happens ONCE at event entry (lines 666-676)
   - Added error handling for path normalization failures with `warn!` logging
   - Added UTF-8 validation for paths (lines 679-688)
   - All `get_file_id_by_path()` calls now use normalized `relpath_str` instead of absolute paths
   - Modified event handling now ALWAYS calls `ChangeDetector.detect_change()` when file_id is found (lines 710-714)
   - Only falls back to `ChangeType::New` when database lookup returns `Ok(None)` (lines 716-727)
   - Added comprehensive error handling with `warn!` logging for database failures (lines 729-737, 747-755)
   - Added code comments explaining the critical fix and referencing WATCHFIX-1002

**Key Fix**:
The core bug was on line 681 (old code) where `get_file_id_by_path()` was called with `relpath` derived from `strip_prefix().unwrap_or()`, which would sometimes fail silently and return the original absolute path. The database expects relative paths like "packages/cli/src/main.ts" but was receiving "/workspace/packages/cli/src/main.ts", causing the lookup to return `None` and files to be misclassified as NEW.

The fix normalizes paths using `normalize_to_relpath()` which:
- Explicitly strips the repo root prefix
- Returns a proper error if the path is outside the repo
- Prevents security issues with path traversal
- Ensures consistent path format for database lookups

**Testing**:
- Code compiles successfully with `cargo build --release`
- No new clippy warnings introduced (verified with `cargo clippy`)
- All acceptance criteria met:
  - Path normalized once at event entry using `normalize_to_relpath()` ✓
  - All `get_file_id_by_path()` calls use normalized relpath ✓
  - `ChangeDetector.detect_change()` called for every Modified event with valid file_id ✓
  - Falls back to `ChangeType::New` only when `get_file_id_by_path()` returns `Ok(None)` ✓
  - Path normalization failures logged as warnings, events skipped gracefully ✓
  - Database lookup errors logged as warnings, events skipped gracefully ✓
  - Code compiles without warnings ✓
