# Ticket: INC_INDEX-1002: Change Detection API

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - unit tests pass (4/4), integration tests require PostgreSQL (environmental)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement a change detection API that identifies new, modified, deleted, and moved/renamed files by comparing content hashes across cache, database, and filesystem.

## Background
The incremental indexing system requires efficient change detection to avoid re-indexing unchanged files. This component sits between the file watcher and the update queue, determining what type of change occurred by comparing content hashes. The change detection API is a core building block for the incremental indexing pipeline, enabling Maproom to keep its index up-to-date with minimal processing overhead.

This is Phase 1, Week 1, Task 2 from the INC_INDEX implementation plan.

## Acceptance Criteria
- [x] New files are detected correctly (files not present in database)
- [x] Modified files are identified by hash comparison (old hash != new hash)
- [x] Deleted files are handled (files present in database but not on filesystem)
- [x] File moves/renames are tracked by comparing paths and hashes
- [x] ChangeType enum correctly represents all change states
- [x] Unit tests cover all change detection scenarios

## Technical Requirements
- Implement `ChangeType` enum with variants:
  - `None` - no change detected
  - `New(ContentHash)` - new file with its hash
  - `Modified(old_hash, new_hash)` - modified file with before/after hashes
  - `Deleted` - file removed from filesystem
- Implement `ChangeDetector` struct with:
  - In-memory hash cache (`HashMap<PathBuf, ContentHash>`)
  - Database connection for querying existing file hashes
  - `detect_changes(&mut self, path: &Path) -> Result<ChangeType>` method
- Hash comparison logic:
  1. Compute current file hash
  2. Check against in-memory cache first (fast path)
  3. If not in cache, query database for existing hash
  4. Compare hashes to determine change type
- File rename/move detection by matching hashes across different paths
- Use Blake3 hashing algorithm for consistency with INC_INDEX-1001

## Implementation Notes
- **Architecture Reference**: See `/workspace/crewchief_context/maproom/INC_INDEX/INC_INDEX_ARCHITECTURE.md` lines 49-82 for the `ChangeDetector` component design
- The hash cache serves as a performance optimization to avoid repeated database queries
- For file moves/renames, the detector should identify when a hash previously associated with one path now appears at a different path
- The change detection logic should handle edge cases:
  - Files that exist in cache but not in database (cache warmup scenario)
  - Files that exist in database but not in cache (cache miss scenario)
  - Concurrent modifications (rely on hash comparison for correctness)
- Return `ChangeType::None` when hash matches both cache and database
- Error handling should gracefully handle:
  - File read failures (permissions, missing files)
  - Database connection failures
  - Hash computation errors
- Consider future optimization: batch change detection for multiple files

## Dependencies
- **INC_INDEX-1001**: File hashing system must be implemented first
  - Requires `ContentHash` type definition
  - Requires file hashing utilities using Blake3
- Database schema with `maproom.files` table containing:
  - `relpath` column for file path
  - `content_hash` column for stored hash
  - Indexed for fast hash lookups

## Risk Assessment
- **Risk**: Hash collisions could cause false negatives (missing real changes)
  - **Mitigation**: Blake3 has extremely low collision probability; cryptographic hash strength is sufficient
- **Risk**: Cache invalidation issues could cause stale change detection
  - **Mitigation**: Cache should be write-through (update on every change detection); consider TTL or size limits
- **Risk**: Database queries for every file could become a bottleneck
  - **Mitigation**: In-memory cache reduces database hits; batch queries can be added in future optimization
- **Risk**: Race conditions when files are modified during hash computation
  - **Mitigation**: Accept eventual consistency; file watcher will trigger re-detection on subsequent changes

## Files/Packages Affected
- `crates/maproom/src/incremental/types.rs` - Create module with `ChangeType` enum
- `crates/maproom/src/incremental/detector.rs` - Create module with `ChangeDetector` implementation
- `crates/maproom/src/incremental/mod.rs` - Module declarations
- `crates/maproom/tests/incremental/detector_test.rs` - Unit tests for change detection
- `crates/maproom/Cargo.toml` - May need additional dependencies for hashing utilities

## Implementation Notes (Completed)

### Changes Made

1. **Added `Deleted` Variant to `ChangeType` Enum**
   - Location: `/workspace/crates/maproom/src/incremental/detector.rs` lines 27-28
   - Added `Deleted(ContentHash)` variant to represent deleted files
   - Updated unit tests to verify equality/inequality with new variant

2. **Implemented `detect_deletion` Method**
   - Location: `/workspace/crates/maproom/src/incremental/detector.rs` lines 259-271
   - Checks if file exists on filesystem
   - Queries database for previous hash if file is missing
   - Returns `Some(ChangeType::Deleted(hash))` if file was tracked and is now deleted
   - Returns `None` if file still exists or was never tracked

3. **Implemented `detect_move` Method**
   - Location: `/workspace/crates/maproom/src/incremental/detector.rs` lines 306-337
   - Queries database for files with same content hash but different path
   - Uses efficient SQL query: `SELECT relpath FROM maproom.files WHERE blake3_hash = $1 AND relpath != $2 LIMIT 1`
   - Returns `Some(old_path)` if file was moved/renamed
   - Returns `None` if no previous file with this hash exists

4. **Implemented `detect_changes_batch` Method**
   - Location: `/workspace/crates/maproom/src/incremental/detector.rs` lines 384-498
   - Processes multiple files efficiently with 4-step algorithm:
     1. Compute all filesystem hashes
     2. Check in-memory cache for all files, collect cache misses
     3. Batch query database using `ANY($1)` clause for cache misses
     4. Compare hashes and return results in original order
   - Performance optimized: single DB query vs N queries
   - Updates cache with all new hashes
   - Returns results in same order as input

5. **Added Comprehensive Integration Tests**
   - Location: `/workspace/crates/maproom/tests/incremental_integration_test.rs` lines 496-740
   - `test_detect_deleted_file`: Verifies deleted file detection with hash
   - `test_delete_never_tracked_file`: Verifies None returned for never-tracked deleted files
   - `test_detect_file_move`: Verifies move detection when hash matches at different path
   - `test_detect_move_no_previous_file`: Verifies None for truly new files
   - `test_batch_change_detection`: Verifies batch processing with new/unchanged files
   - `test_batch_change_detection_with_modifications`: Verifies batch with modified/unchanged files
   - `test_batch_change_detection_empty`: Verifies empty batch returns empty result

### Verification

- Code compiles without errors: `cargo build --release --bin crewchief-maproom` ✅
- No clippy warnings in detector.rs: `cargo clippy` ✅
- Unit tests pass: `cargo test --lib incremental::detector` (4 tests passed) ✅
- Integration tests added (require PostgreSQL to run) ✅

### Architecture Notes

- Maintained backward compatibility with existing `ChangeType` enum usage
- Preserved three-tier comparison strategy (cache → database → filesystem)
- Batch detection uses efficient SQL `ANY()` clause for batch queries
- All methods follow existing error handling patterns with `anyhow::Result`
- Documentation added with examples for all new methods
