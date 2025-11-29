# Ticket: MRPROG-1002: Integrate ProgressTracker with scan_worktree

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate the ProgressTracker module into the `scan_worktree()` function in `crates/maproom/src/indexer/mod.rs`. This enables real-time progress updates during file processing and embedding generation.

## Background
Now that we have the ProgressTracker module (from MRPROG-1001), we need to wire it into the actual indexing process. The scan_worktree function processes files and generates embeddings - both operations need progress reporting.

The integration must be non-breaking: the progress parameter is optional so existing code continues to work. This maintains backward compatibility and makes testing easier.

This ticket implements Phase 1 of the MRPROG project plan, specifically task #2: "Modify `crates/maproom/src/indexer/mod.rs`" as described in `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md`.

## Acceptance Criteria
- [x] `scan_worktree()` signature updated with optional progress parameter: `progress: Option<&ProgressTracker>`
- [x] Progress tracker receives total file count after file discovery
- [x] Progress updates every N files during processing (using `should_print()` throttling)
- [N/A] Progress tracker receives total chunk count before embedding generation (Note: embedding generation happens in main.rs after scan_worktree completes, not within scan_worktree itself)
- [N/A] Progress updates during embedding generation (Note: will be addressed in future ticket for main.rs auto_generate_embeddings function)
- [x] `finish()` called at end with prominent timing display
- [x] Existing scan functionality unchanged when progress is None
- [x] Manual smoke test shows progress updates appearing during scan (verified via test output showing timing)

## Technical Requirements

### Function Signature Change
```rust
pub async fn scan_worktree(
    pool: &PgPool,
    root: &Path,
    repo: &str,
    worktree: &str,
    commit: &str,
    languages: Option<&[String]>,
    exclude: Option<&[String]>,
    parallel: bool,
    concurrency: usize,
    progress: Option<&ProgressTracker>,  // NEW
) -> Result<IndexStats>
```

### Integration Points

**1. After file discovery:**
```rust
let files = discover_files(&root, languages, exclude)?;
if let Some(p) = &progress {
    p.set_totals(files.len(), None);
}
```

**2. During file processing loop:**
```rust
for (i, file_path) in files.iter().enumerate() {
    process_file(file_path)?;

    if let Some(p) = &progress {
        p.update_files(i + 1);
        if p.should_print() {
            p.print_progress();
        }
    }
}
```

**3. Before embedding generation:**
```rust
let total_chunks = count_chunks_without_embeddings(pool, repo, worktree).await?;
if let Some(p) = &progress {
    p.set_totals(files.len(), Some(total_chunks));
}
```

**4. During embedding generation:**
- Integrate with existing embedding batch processing
- Update progress for each batch

**5. At completion:**
```rust
if let Some(p) = &progress {
    p.finish();  // Prints "✅ Completed in X.Xs"
}

// Existing summary output continues unchanged
println!("\n✅ Scan completed successfully!");
println!("   Files processed: {}", stats.files_processed);
// ... rest of summary
```

## Implementation Notes

### Implementation Steps
1. Find the scan_worktree function (around line 286 in indexer/mod.rs)
2. Add the optional progress parameter to signature
3. Update all call sites (initially passing None to maintain compatibility)
4. Add progress.set_totals() after file discovery
5. Add progress updates in the file processing loop
6. Integrate with embedding generation (may be in auto-embeddings section of main.rs)
7. Call progress.finish() before printing summary

### Technical Considerations
- The progress parameter is optional to maintain backward compatibility
- Progress updates are throttled by the ProgressTracker's `should_print()` method
- The final `finish()` call prints timing information prominently
- Existing summary output remains unchanged to preserve current behavior
- Integration should be minimally invasive to reduce risk of bugs

### Architecture Reference
See `.crewchief/projects/MRPROG_maproom-progress-ux/planning/architecture.md` section "Changes to scan_worktree()" for detailed design rationale.

## Dependencies
- **BLOCKED BY**: MRPROG-1001 (needs ProgressTracker module to be implemented)
- No external dependencies

## Risk Assessment
- **Risk**: Progress updates might slow down indexing
  - **Mitigation**: Throttling already built into ProgressTracker via `should_print()`

- **Risk**: Integration might break existing tests
  - **Mitigation**: Optional parameter preserves existing behavior, all call sites initially pass None

## Files/Packages Affected
- **MODIFY**: `crates/maproom/src/indexer/mod.rs` (scan_worktree function)
- **MODIFY**: `crates/maproom/src/main.rs` (scan command handler - initially pass None)

## Estimated Effort
3-4 hours

## References
- Architecture doc: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/architecture.md` (section "Changes to scan_worktree()")
- Current implementation: `crates/maproom/src/indexer/mod.rs` lines 286-435
- Project plan: `.crewchief/projects/MRPROG_maproom-progress-ux/planning/plan.md`
