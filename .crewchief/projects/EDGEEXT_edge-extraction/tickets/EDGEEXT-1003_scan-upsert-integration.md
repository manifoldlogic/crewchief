# Ticket: EDGEEXT-1003 - Scan/Upsert Integration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary

Integrate edge extraction into the scan_worktree() and upsert_files() pipeline. This wires the edge extractor module (EDGEEXT-1001) and TypeScript extractor (EDGEEXT-1002) into the actual indexing flow so edges are populated during file scanning.

## Background

The edge extractor module and TypeScript call extraction are complete, but they won't run without integration into the indexing pipeline. This ticket implements the actual delivery mechanism that calls extract_edges() during scan operations and inserts the results into the chunk_edges table.

The Python imports extraction (lines 437-448 in indexer/mod.rs) provides a proven pattern:
1. Extract chunks and insert them
2. Collect chunk IDs after insertion
3. Call extraction logic with chunk data
4. Batch insert edges

This ticket follows the same pattern but uses the new modular edge extractor for TypeScript/JavaScript files.

## Acceptance Criteria

- [x] Modify scan_worktree() to call extract_edges() after chunk insertion
- [x] Collect chunk IDs during chunk insertion loop (modify insertion loop)
- [x] Call edges::extract_edges() with source, language, and chunks_with_ids
- [x] Implement batch insert_edges() helper function
- [x] Same integration for upsert_files() function
- [x] Update EdgeUpdater.update_edges() to call extract_edges()
- [x] Error handling: Log warnings for extraction failures, don't fail scan
- [x] Edges appear in chunk_edges table after scanning TypeScript/JavaScript files
- [x] Incremental updates work: modifying file triggers edge recomputation

## Technical Requirements

### Integration Point 1: scan_worktree()

**Location:** `crates/maproom/src/indexer/mod.rs`, after line ~435 (after chunk insertion loop)

**Current Code (lines 402-435, simplified):**
```rust
// Insert chunks for this file
for chunk in chunks {
    store.insert_chunk(...).await?;
}

// Process Python imports (lines 437-448)
if language == "py" {
    process_python_imports(...).await?;
}
```

**New Code:**
```rust
use crate::indexer::edges::{self, ChunkWithId};

// Modified chunk insertion loop - collect IDs
let mut chunks_with_ids = Vec::new();
for chunk in chunks {
    let chunk_id = store.insert_chunk(...).await?;
    chunks_with_ids.push(ChunkWithId {
        id: chunk_id,
        symbol_name: chunk.symbol_name.clone(),
        kind: chunk.kind.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        file_id: file_id,
    });
}

// Process Python imports (existing)
if language == "py" {
    process_python_imports(...).await?;
}

// Extract edges for TypeScript/JavaScript (NEW)
if matches!(language, "typescript" | "tsx" | "javascript" | "jsx") {
    match edges::extract_edges(&content, language, &chunks_with_ids) {
        Ok(edges_to_insert) if !edges_to_insert.is_empty() => {
            if let Err(e) = insert_edges(&store, &edges_to_insert).await {
                warn!("Failed to insert edges for {}: {}", relpath, e);
                // Continue scan despite edge insertion failure
            } else {
                debug!("Inserted {} edges for {}", edges_to_insert.len(), relpath);
            }
        }
        Ok(_) => {
            // No edges extracted (empty file or no calls)
            trace!("No edges extracted for {}", relpath);
        }
        Err(e) => {
            warn!("Edge extraction failed for {}: {}", relpath, e);
            // Continue scan despite extraction failure
        }
    }
}
```

**Helper Function (add at module level):**
```rust
use crate::incremental::edge_updater::Edge;

/// Batch insert edges into the database
async fn insert_edges(store: &SqliteStore, edges: &[Edge]) -> Result<()> {
    for edge in edges {
        store.insert_chunk_edge(
            edge.src_chunk_id,
            edge.dst_chunk_id,
            edge.edge_type.as_str(),
        ).await?;
    }
    Ok(())
}
```

### Integration Point 2: upsert_files()

**Location:** `crates/maproom/src/indexer/mod.rs`, after line ~625 (after chunk insertion loop)

**Same Pattern:**
```rust
// After chunk insertion loop
let mut chunks_with_ids = Vec::new();
for chunk in chunks {
    let chunk_id = store.insert_chunk(...).await?;
    chunks_with_ids.push(ChunkWithId {
        id: chunk_id,
        symbol_name: chunk.symbol_name.clone(),
        kind: chunk.kind.clone(),
        start_line: chunk.start_line,
        end_line: chunk.end_line,
        file_id: file_id,
    });
}

// Extract edges (same as scan_worktree)
if matches!(language.unwrap_or(""), "typescript" | "tsx" | "javascript" | "jsx") {
    if let Ok(edges_to_insert) = edges::extract_edges(&content, language.unwrap(), &chunks_with_ids) {
        if !edges_to_insert.is_empty() {
            let _ = insert_edges(&store, &edges_to_insert).await;
        }
    }
}
```

### Integration Point 3: EdgeUpdater.update_edges()

**Location:** `crates/maproom/src/incremental/edge_updater.rs`, line ~240 (in update_edges stub)

**Current Code:**
```rust
pub async fn update_edges(&self, file_id: i64) -> Result<()> {
    // TODO: Implement edge recomputation
    // Current: Only deletes edges, doesn't recompute
    self.delete_edges_for_file(file_id).await?;
    Ok(())
}
```

**New Code:**
```rust
use crate::indexer::edges::{self, ChunkWithId};

pub async fn update_edges(&self, file_id: i64) -> Result<()> {
    // 1. Delete old edges (existing logic)
    self.delete_edges_for_file(file_id).await?;

    // 2. Recompute edges (NEW)
    // Get file metadata
    let file = self.store.run(|conn| {
        conn.query_row(
            "SELECT relpath, language FROM files WHERE id = ?",
            params![file_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        )
    }).await?;

    let (relpath, language) = file;
    let language = match language {
        Some(lang) if matches!(lang.as_str(), "typescript" | "tsx" | "javascript" | "jsx") => lang,
        _ => {
            // No edge extraction for this language
            return Ok(());
        }
    };

    // Read file content
    let content = std::fs::read_to_string(&relpath)
        .with_context(|| format!("Failed to read file: {}", relpath))?;

    // Load chunks for this file
    let chunks_with_ids: Vec<ChunkWithId> = self.store.run(move |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, symbol_name, kind, start_line, end_line FROM chunks WHERE file_id = ?"
        )?;
        let chunks = stmt.query_map(params![file_id], |row| {
            Ok(ChunkWithId {
                id: row.get(0)?,
                symbol_name: row.get(1)?,
                kind: row.get(2)?,
                start_line: row.get(3)?,
                end_line: row.get(4)?,
                file_id: file_id,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(chunks)
    }).await?;

    // Extract edges
    let edges_to_insert = edges::extract_edges(&content, &language, &chunks_with_ids)?;

    // Insert edges
    for edge in edges_to_insert {
        self.store.insert_chunk_edge(
            edge.src_chunk_id,
            edge.dst_chunk_id,
            edge.edge_type.as_str(),
        ).await?;
    }

    Ok(())
}
```

## Implementation Notes

**Chunk ID Collection Strategy:**
We chose to collect chunk IDs during the insertion loop (Option B from architecture.md) rather than querying the database afterward. This is more efficient because:
- Chunks are already in memory during insertion
- No additional database round-trip needed
- Follows the same pattern as chunk insertion itself

**Error Handling Philosophy:**
- Edge extraction failures should NOT fail the scan
- Log warnings for edge extraction errors
- Continue processing files even if some edge extraction fails
- This matches the pattern from Python imports extraction
- Rationale: Partial edges are better than no scan at all

**Batch Insertion:**
Insert all edges for a file in sequence (not a single transaction per file, but all edges before moving to next file). This balances:
- Transaction overhead (not one per edge)
- Memory usage (not all edges in memory)
- Error recovery (can recover if one file fails)

**Language Filtering:**
Only extract edges for TypeScript/JavaScript files. Other languages return empty Vec from extract_edges(), but we can optimize by skipping the call entirely:
```rust
if matches!(language, "typescript" | "tsx" | "javascript" | "jsx") {
    // extract edges
}
```

**Integration with Existing Python Imports:**
The Python imports code remains unchanged. Both systems coexist:
- Python: Uses process_python_imports()
- TypeScript/JavaScript: Uses edges::extract_edges()
- Future: Refactor Python to use edges::extract_edges() with python.rs extractor

## Dependencies

**Prerequisites:**
- EDGEEXT-1001 (edge extractor module must exist)
- EDGEEXT-1002 (TypeScript extractor must be implemented)

**Blocks:**
- EDGEEXT-1004 (testing requires integration to work)

## Risk Assessment

**Risk:** insert_chunk() doesn't return chunk ID
**Mitigation:** Verify API signature, likely returns chunk ID or can be modified

**Risk:** Chunk insertion loop structure differs from expectations
**Mitigation:** Review actual code in indexer/mod.rs before implementation

**Risk:** EdgeUpdater file loading is too slow
**Mitigation:** Acceptable for incremental updates (only modified files), can optimize later

**Risk:** Edge insertion fails and breaks scan
**Mitigation:** Wrap in error handling, log warnings, continue scan

## Files/Packages Affected

**Modified Files:**
- `crates/maproom/src/indexer/mod.rs` (scan_worktree, upsert_files, insert_edges helper)
- `crates/maproom/src/incremental/edge_updater.rs` (update_edges implementation)

**No New Files** (uses modules from EDGEEXT-1001 and EDGEEXT-1002)

## Testing Notes

Testing is primarily handled in EDGEEXT-1004, but this ticket should verify:
- Basic integration: edges appear in database after scan
- Error handling: scan doesn't fail when edge extraction errors
- Incremental: modifying file triggers edge recomputation

Integration test (to be formalized in EDGEEXT-1004):
```rust
#[tokio::test]
async fn test_scan_creates_edges() {
    let store = setup_test_db().await;

    // Scan a TypeScript file
    scan_worktree(&store, "test", "main", test_repo_path, "HEAD").await?;

    // Verify edges exist
    let edge_count = store.run(|conn| {
        conn.query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| row.get(0))
    }).await?;

    assert!(edge_count > 0, "Expected edges to be created");
}
```

## Planning References

- Architecture: `.crewchief/projects/EDGEEXT_edge-extraction/planning/architecture.md` (lines 141-174)
- Plan: `.crewchief/projects/EDGEEXT_edge-extraction/planning/plan.md` (Phase 1, lines 16-17)
- Quality Strategy: `.crewchief/projects/EDGEEXT_edge-extraction/planning/quality-strategy.md` (lines 66-110)
