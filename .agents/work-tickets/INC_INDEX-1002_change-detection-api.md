# Ticket: INC_INDEX-1002: Change Detection API

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

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
- [ ] New files are detected correctly (files not present in database)
- [ ] Modified files are identified by hash comparison (old hash != new hash)
- [ ] Deleted files are handled (files present in database but not on filesystem)
- [ ] File moves/renames are tracked by comparing paths and hashes
- [ ] ChangeType enum correctly represents all change states
- [ ] Unit tests cover all change detection scenarios

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
