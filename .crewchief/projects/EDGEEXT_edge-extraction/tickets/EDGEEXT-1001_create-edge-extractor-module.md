# Ticket: EDGEEXT-1001 - Create Edge Extractor Module

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary

Create the foundational edge extractor module at `crates/maproom/src/indexer/edges/` with public API, data structures, and common utilities that will be used by language-specific extractors.

## Background

Edge extraction needs a dedicated module structure to isolate functionality and enable extensibility. The module will provide a common API that language-specific extractors (TypeScript, Python, Rust) will implement. This follows the established pattern from `crates/maproom/src/indexer/parser.rs` for language-specific code processing.

The `chunk_edges` table already exists in the database schema, and `SqliteStore::insert_chunk_edge()` is ready to use. This ticket focuses on creating the extraction layer that feeds into the existing database infrastructure.

## Acceptance Criteria

- [ ] Create `crates/maproom/src/indexer/edges/` directory
- [ ] Make existing `Edge` and `EdgeType` from `edge_updater.rs` public and accessible
- [ ] Implement `edges/mod.rs` with public API: `extract_edges(source, language, chunks) -> Result<Vec<Edge>>`
- [ ] Implement `edges/common.rs` with shared utilities (find_enclosing_chunk, etc.)
- [ ] Define `ChunkWithId` struct for chunks after database insertion (includes file_id field)
- [ ] Reuse shared `Edge` and `EdgeType` structs from edge_updater module
- [ ] Add unit tests for common utilities
- [ ] Module compiles without errors and integrates with existing indexer

## Technical Requirements

**Module Structure:**
```
crates/maproom/src/indexer/edges/
├── mod.rs           # Public API and dispatcher
├── common.rs        # Shared utilities
└── typescript.rs    # TypeScript/JavaScript extractor (stub for EDGEEXT-1002)
```

**Public API (edges/mod.rs):**
```rust
use anyhow::Result;
use crate::incremental::edge_updater::{Edge, EdgeType};

/// Chunk with database ID (after insertion)
#[derive(Debug, Clone)]
pub struct ChunkWithId {
    pub id: i64,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub file_id: i64,  // For Phase 2 cross-file resolution
}

/// Extract edges from source code
///
/// Reuses Edge and EdgeType from crate::incremental::edge_updater module.
/// These types are made public in edge_updater.rs for shared use.
pub fn extract_edges(
    source: &str,
    language: &str,
    chunks: &[ChunkWithId],
) -> Result<Vec<Edge>> {
    match language {
        "typescript" | "tsx" | "javascript" | "jsx" => {
            typescript::extract_calls(source, chunks)
        }
        // Python, Rust will be added in Phase 2/3
        _ => {
            // No edge extraction for unsupported languages
            Ok(Vec::new())
        }
    }
}
```

**Common Utilities (edges/common.rs):**
```rust
use super::ChunkWithId;

/// Find the chunk that contains a given line number
pub fn find_enclosing_chunk(chunks: &[ChunkWithId], line: i32) -> Option<&ChunkWithId> {
    chunks.iter().find(|chunk| {
        chunk.start_line <= line && line <= chunk.end_line
    })
}

/// Build a symbol table mapping symbol names to chunk IDs
pub fn build_symbol_table(chunks: &[ChunkWithId]) -> std::collections::HashMap<String, i64> {
    chunks
        .iter()
        .filter_map(|chunk| {
            chunk.symbol_name.as_ref().map(|name| (name.clone(), chunk.id))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_enclosing_chunk() {
        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("foo".to_string()),
                kind: "function".to_string(),
                start_line: 1,
                end_line: 5,
            },
            ChunkWithId {
                id: 2,
                symbol_name: Some("bar".to_string()),
                kind: "function".to_string(),
                start_line: 7,
                end_line: 12,
            },
        ];

        assert_eq!(find_enclosing_chunk(&chunks, 3).unwrap().id, 1);
        assert_eq!(find_enclosing_chunk(&chunks, 10).unwrap().id, 2);
        assert!(find_enclosing_chunk(&chunks, 6).is_none());
    }

    #[test]
    fn test_build_symbol_table() {
        let chunks = vec![
            ChunkWithId {
                id: 1,
                symbol_name: Some("foo".to_string()),
                kind: "function".to_string(),
                start_line: 1,
                end_line: 5,
            },
            ChunkWithId {
                id: 2,
                symbol_name: None,
                kind: "statement".to_string(),
                start_line: 7,
                end_line: 8,
            },
        ];

        let table = build_symbol_table(&chunks);
        assert_eq!(table.len(), 1);
        assert_eq!(table.get("foo"), Some(&1));
    }
}
```

**TypeScript Stub (edges/typescript.rs):**
```rust
use anyhow::Result;
use super::{ChunkWithId, Edge};

/// Extract call edges from TypeScript/JavaScript source
/// Full implementation in EDGEEXT-1002
pub fn extract_calls(_source: &str, _chunks: &[ChunkWithId]) -> Result<Vec<Edge>> {
    // Stub: will be implemented in EDGEEXT-1002
    Ok(Vec::new())
}
```

**Module Integration:**
Add to `crates/maproom/src/indexer/mod.rs`:
```rust
pub mod edges;
```

## Implementation Notes

**Shared Types Strategy:**
1. Make `Edge` and `EdgeType` in `edge_updater.rs` public (remove `#[allow(dead_code)]`)
2. Import these types in `edges/mod.rs` via `use crate::incremental::edge_updater::{Edge, EdgeType}`
3. This ensures a single source of truth and prevents type divergence
4. EdgeUpdater and edge extractor share the same type definitions

**Design Principles:**
- Keep module API simple and composable
- Use existing patterns from `parser.rs` for language dispatching
- Make it easy to add new languages (just add new file + match arm)
- Keep common utilities testable and reusable
- Reuse existing types rather than duplicating

**Error Handling:**
- Return empty Vec for unsupported languages (don't fail the scan)
- Log warnings if edge extraction encounters issues
- Propagate errors only for critical failures (database, I/O)

**Performance Considerations:**
- Symbol table is built once per file (O(chunks))
- Find enclosing chunk is linear search (acceptable for <100 chunks per file)
- No database queries in this module (all in-memory)

## Dependencies

**Prerequisites:**
- Database schema with `chunk_edges` table (exists)
- `SqliteStore::insert_chunk_edge()` method (exists at `db/sqlite/mod.rs:684-691`)

**Blocks:**
- EDGEEXT-1002 (TypeScript call extraction - needs this module)
- EDGEEXT-1003 (Integration with scan/upsert - needs this module)

## Risk Assessment

**Risk:** Module API doesn't meet TypeScript extractor needs
**Mitigation:** API designed based on architecture.md specifications, will be validated in EDGEEXT-1002

**Risk:** ChunkWithId struct missing required fields
**Mitigation:** Struct mirrors database chunk schema, includes all fields needed for resolution

## Files/Packages Affected

**New Files:**
- `crates/maproom/src/indexer/edges/mod.rs`
- `crates/maproom/src/indexer/edges/common.rs`
- `crates/maproom/src/indexer/edges/typescript.rs`

**Modified Files:**
- `crates/maproom/src/indexer/mod.rs` (add `pub mod edges;`)
- `crates/maproom/src/incremental/edge_updater.rs` (make Edge and EdgeType public)

## Planning References

- Architecture: `.crewchief/projects/EDGEEXT_edge-extraction/planning/architecture.md` (lines 84-124)
- Plan: `.crewchief/projects/EDGEEXT_edge-extraction/planning/plan.md` (Phase 1, lines 10-33)
- Quality Strategy: `.crewchief/projects/EDGEEXT_edge-extraction/planning/quality-strategy.md` (lines 13-65)
