# Ticket: WATCHFIX-1003: Fix IncrementalProcessor Path Handling

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
Update `IncrementalProcessor` methods (`index_new_file`, `update_file`, `remove_file`) to correctly handle path formats: use absolute paths for filesystem operations and relative paths for database queries. This fixes the "File not found in database" errors in `index_new_file()`.

## Background
This ticket implements Phase 3 (Processor Path Handling) from the WATCHFIX project plan.

Currently, `index_new_file()` (line 206 in `processor.rs`) converts the path using `path.to_string_lossy()` which produces an absolute path like `/workspace/packages/cli/src/main.ts`. It then queries the database with this absolute path, but the database stores relative paths like `packages/cli/src/main.ts`. The query returns no rows, causing the error at line 222: "File not found in database".

The fix requires normalizing paths for database queries while keeping absolute paths for filesystem reads. This ensures the processor can correctly locate files in the database while still being able to read their contents from the filesystem.

Reference: `.agents/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md` (IncrementalProcessor Expectations section, lines 204-223) and `architecture.md` (IncrementalProcessor Path Handling section).

## Acceptance Criteria
- [ ] `index_new_file()` queries database using relative path (not absolute)
- [ ] `update_file()` queries database using relative path
- [ ] `remove_file()` queries database using relative path (if applicable)
- [ ] Filesystem operations (`fs::read_to_string`, etc.) continue using absolute paths
- [ ] All three methods compile without warnings
- [ ] Existing functionality preserved (no behavior changes except path handling)

## Technical Requirements

**File to modify**: `crates/maproom/src/incremental/processor.rs`

**Method 1: index_new_file()** (lines 191-257):
```rust
async fn index_new_file(&self, path: &Path, hash: &ContentHash) -> Result<()> {
    // Read from filesystem using absolute path
    let content = fs::read_to_string(path)?;

    // Normalize for database query
    let relpath_str = path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path"))?
        .trim_start_matches("/workspace/"); // Quick normalization

    // Query with relpath
    let file_row = client.query_opt(
        "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
        &[&relpath_str],
    ).await?;
    // ... rest of logic
}
```

**Method 2: update_file()** (lines 259-330):
- Similar normalization for database queries
- Keep absolute path for filesystem reads

**Method 3: remove_file()** (if path queries exist):
- Check if normalization needed
- Apply same pattern

**Alternative approach** (if better): Accept both `absolute_path` and `relpath` as separate parameters
- Requires updating call sites in processor_task
- More explicit but more invasive

## Implementation Notes

**Temporary solution**: Use `trim_start_matches("/workspace/")` for quick path normalization.

**Better solution**: Import and use `normalize_to_relpath()` from path_utils module for more robust handling.

**Trade-off**: Simple trim is less robust but avoids potential circular dependencies. Choose based on module structure.

**Important**: Add code comments explaining the path handling pattern - absolute paths for filesystem, relative paths for database queries.

**Path handling pattern**:
1. Receive absolute path as parameter
2. Use absolute path for filesystem operations (reading file content)
3. Convert to relative path for database queries (strip repo root prefix)
4. Ensure error messages clearly indicate which path format is being used

## Dependencies
- WATCHFIX-1001 (uses normalize_to_relpath pattern)
- WATCHFIX-1002 (processor_task calls these methods)

## Blocks
- WATCHFIX-1005 (integration tests verify end-to-end indexing)

## Risk Assessment

- **Risk**: Hard-coded repo root path ("/workspace/") is fragile and may break in different environments
  - **Mitigation**: Document limitation in code comments, or pass repo_root as parameter to processor methods

- **Risk**: Might miss some query locations where path normalization is needed
  - **Mitigation**: Grep for all relpath queries in the file to ensure comprehensive coverage

- **Risk**: Alternative approach (separate parameters) requires updating all call sites
  - **Mitigation**: If choosing this approach, carefully review all call sites in processor_task.rs

## Files/Packages Affected
- **MODIFY**: `crates/maproom/src/incremental/processor.rs` (~40 lines changed across 3 methods)
  - `index_new_file()` method (lines 191-257)
  - `update_file()` method (lines 259-330)
  - `remove_file()` method (if applicable)

**Estimated Effort**: 4 hours

**Priority**: HIGH (required for fix to work)

**References**:
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/analysis.md` - IncrementalProcessor Expectations section (lines 204-223)
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/architecture.md` - IncrementalProcessor Path Handling section
- `/workspace/.agents/projects/WATCHFIX_watch-change-detection-fix/planning/plan.md` - Phase 3 deliverables

## Implementation Notes

**Completed**: 2025-11-06

**Summary**: Successfully updated `IncrementalProcessor` to use robust path normalization for all database operations while maintaining absolute paths for filesystem operations.

**Changes Made**:

1. **Updated struct and constructor** (`crates/maproom/src/incremental/processor.rs`):
   - Added `repo_root: PathBuf` field to `IncrementalProcessor` struct (line 82)
   - Updated constructor `new()` to accept `repo_root` parameter (lines 94-100)
   - Updated documentation examples to reflect new constructor signature

2. **Added imports**:
   - Imported `PathBuf` from `std::path` (line 34)
   - Imported `normalize_to_relpath` from `super::path_utils` (line 43)

3. **Fixed `index_new_file()` method** (lines 197-234):
   - Added comment: "CRITICAL: Read file content using absolute path (filesystem operation)"
   - Added comment: "CRITICAL: Normalize path for database query (database stores relative paths)"
   - Uses `normalize_to_relpath(path, &self.repo_root)` to convert absolute path to relative path
   - Added UTF-8 validation with clear error message
   - Database query now uses normalized `relpath_str` instead of `path.to_string_lossy()`
   - Updated error message to include `relpath_str` for better debugging

4. **Fixed `update_file()` method** (lines 286-322):
   - Same pattern as `index_new_file()`:
     - Filesystem read uses absolute path
     - Path normalized for database query
     - Database query uses `relpath_str`
   - Enhanced warning log to include both absolute path and relpath

5. **Fixed `remove_file()` method** (lines 379-409):
   - Same pattern as other methods:
     - Path normalized at start
     - Database query uses `relpath_str`
   - Enhanced debug log to include both absolute path and relpath

6. **Updated call site** (`crates/maproom/src/indexer/mod.rs`, line 649):
   - Changed `IncrementalProcessor::new(pool.clone())` to `IncrementalProcessor::new(pool.clone(), root_abs.clone())`
   - Uses existing `root_abs` variable which is already available in `watch_worktree()`

**Key Design Decisions**:

1. **Chose to add `repo_root` parameter**: More robust than hardcoded string trimming, works across environments
2. **Used `normalize_to_relpath()` from path_utils**: Provides security (prevents path traversal), cross-platform support, and comprehensive error handling
3. **Added clear comments**: Marked filesystem operations vs database operations with "CRITICAL" prefix to prevent future confusion
4. **Preserved absolute paths for filesystem ops**: Ensures file reading continues to work correctly
5. **Enhanced error messages**: Include both absolute and relative paths in logs for easier debugging

**Path Handling Pattern**:
- Input: Absolute path from filesystem watcher (e.g., `/workspace/packages/cli/src/main.ts`)
- Filesystem operations: Use absolute path directly
- Database queries: Normalize to relative path (e.g., `packages/cli/src/main.ts`) using `normalize_to_relpath()`
- Error messages: Include both formats for clarity

**Testing**:
- Code compiles successfully: `cargo build --release` (0 errors, 0 warnings in modified files)
- Clippy passes: `cargo clippy` (no new warnings introduced)
- Unit tests pass: `cargo test incremental::processor` (4/4 tests passing)

**Acceptance Criteria Status**:
- [x] `index_new_file()` queries database using relative path (lines 212-223)
- [x] `update_file()` queries database using relative path (lines 301-314)
- [x] `remove_file()` queries database using relative path (lines 387-400)
- [x] Filesystem operations use absolute paths (lines 203, 292)
- [x] All three methods compile without warnings
- [x] Existing functionality preserved (same logic, just fixed path handling)

**Code Examples**:

Example from `index_new_file()`:
```rust
// CRITICAL: Read file content using absolute path (filesystem operation)
let content = fs::read_to_string(path)
    .with_context(|| format!("Failed to read file: {}", path.display()))?;

// CRITICAL: Normalize path for database query (database stores relative paths)
let relpath = normalize_to_relpath(path, &self.repo_root)
    .with_context(|| format!("Failed to normalize path: {}", path.display()))?;

let relpath_str = relpath.to_str()
    .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in path: {}", relpath.display()))?;

// Query with relative path
let file_row = client
    .query_opt(
        "SELECT id FROM maproom.files WHERE relpath = $1 ORDER BY id DESC LIMIT 1",
        &[&relpath_str],
    )
    .await?;
```

This implementation fixes the "File not found in database" error by ensuring database queries use the same path format (relative) that's stored in the database.
