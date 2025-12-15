# Architecture: edge extraction

## Overview

Extract code relationships (function calls, imports, test links) during indexing and populate the `chunk_edges` table. The solution integrates edge extraction into the existing scan/upsert pipeline, using tree-sitter to find call expressions and heuristic symbol resolution to link chunks.

**Key Principle:** Extract edges at the same time chunks are created (single-pass indexing) to minimize performance overhead and maintain consistency.

## Design Decisions

### Decision 1: Extract During Indexing (Not Post-Processing)

**Context:** Could extract edges as a separate pass after chunks exist, or integrate into scan pipeline.

**Decision:** Extract edges immediately after chunks are inserted during `scan_worktree()` and `upsert_files()`.

**Rationale:**
- Python imports already use this pattern (proven at `indexer/mod.rs:437-448`)
- File content already in memory (no duplicate I/O)
- Chunk IDs available immediately after insertion
- Simpler error handling (fail scan if extraction fails)
- Matches incremental update pattern (EdgeUpdater)

### Decision 2: Same-File Resolution Only (MVP)

**Context:** Symbol resolution can be same-file (high confidence) or cross-file (requires import tracking).

**Decision:** Phase 1 resolves calls within the same file only. Cross-file resolution deferred to Phase 2.

**Rationale:**
- 70-80% of calls are same-file in typical codebases
- No database queries needed (in-memory symbol table)
- Simpler implementation (no import graph tracking)
- MVP can unblock SRCHREL with partial edges
- Performance: <1ms per call vs ~10ms for database lookup

### Decision 3: TypeScript/JavaScript First

**Context:** Need to choose which languages to support in MVP.

**Decision:** Phase 1 implements TypeScript, JavaScript, TSX, JSX only. Python/Rust/Go deferred.

**Rationale:**
- 60-70% of codebases are TS/JS (highest ROI)
- Single tree-sitter grammar family (code reuse)
- Python already has imports (calls are lower priority)
- Can extend pattern to other languages incrementally

### Decision 4: Calls Edges Only (Phase 1)

**Context:** Six edge types defined (calls, imports, exports, called_by, test_of, route_of).

**Decision:** Phase 1 implements `calls` edges only.

**Rationale:**
- SRCHREL needs call graph for quality scoring (highest value)
- Simpler than imports (no module resolution)
- Language-agnostic pattern (all languages have calls)
- Unblocks relationship-aware search immediately
- Imports and test_of in Phase 2

### Decision 5: Batch Edge Insertion

**Context:** Could insert edges one-by-one or batch per file.

**Decision:** Batch insert all edges for a file in a single transaction.

**Rationale:**
- Minimizes database round-trips (1 transaction vs N)
- Leverages `INSERT OR IGNORE` for deduplication
- Matches chunk insertion pattern
- Performance: ~5ms for 30 edges vs ~150ms (30 × 5ms)

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Parser** | tree-sitter | Already used for chunk extraction, mature, multi-language support |
| **TypeScript Grammar** | `tree-sitter-typescript` | Official grammar, handles TS/TSX/JS/JSX |
| **Symbol Resolution** | HashMap (same-file), `find_chunk_by_symbol()` (cross-file) | Existing pattern from Python imports, proven |
| **Database API** | `SqliteStore::insert_chunk_edge()` | Existing method at `db/sqlite/mod.rs:684-691`, tested |
| **Error Handling** | Log warnings, continue scan | Partial edges better than no edges, matches Python imports behavior |

## Component Design

### Edge Extractor Module

**Location:** `crates/maproom/src/indexer/edges/`

```
edges/
├── mod.rs           # Public API: extract_edges(source, language, chunks)
├── typescript.rs    # TypeScript/JavaScript call extraction
└── common.rs        # Shared utilities (find_enclosing_chunk, etc.)
```

**Shared Types:**
The edge extractor reuses existing `Edge` and `EdgeType` structs from `crates/maproom/src/incremental/edge_updater.rs` (lines 184-215). These types will be made public and moved to a shared location accessible by both the incremental updater and the edge extractor.

**Public API:**
```rust
use crate::incremental::edge_updater::{Edge, EdgeType};

/// Extract edges from source code
pub fn extract_edges(
    source: &str,
    language: &str,
    chunks: &[ChunkWithId],  // Chunks with database IDs
) -> Result<Vec<Edge>>;

/// Chunk with database ID (after insertion)
pub struct ChunkWithId {
    pub id: i64,
    pub symbol_name: Option<String>,
    pub kind: String,
    pub start_line: i32,
    pub end_line: i32,
    pub file_id: i64,  // For Phase 2 cross-file resolution
}

// Edge and EdgeType are imported from edge_updater.rs
// Edge has: src_chunk_id, dst_chunk_id, edge_type
// EdgeType enum: Imports, Exports, Calls, CalledBy, TestOf, RouteOf
```

### TypeScript Extractor

**Algorithm:**
1. Parse file with tree-sitter
2. Build symbol table: `HashMap<symbol_name, chunk_id>`
3. Walk tree for `call_expression` nodes
4. Extract function identifier from call
5. Resolve in symbol table
6. Create edge if found

**Key Functions:**
- `extract_calls()` - Main entry point
- `find_call_expressions()` - Recursive tree walk
- `extract_function_identifier()` - Get callee name from AST
- `find_enclosing_chunk()` - Find chunk containing call

### Integration Points

**scan_worktree() Modification:**
```rust
// After chunk insertion loop (line ~435)
let chunks_with_ids = load_chunks_for_file(store, file_id).await?;
let edges = edges::extract_edges(&content, language, &chunks_with_ids)?;
if !edges.is_empty() {
    insert_edges(store, &edges).await?;
}
```

**upsert_files() Modification:**
```rust
// After chunk insertion loop (line ~625)
let chunks_with_ids = load_chunks_for_file(store, file_id).await?;
let edges = edges::extract_edges(&content, language.unwrap(), &chunks_with_ids)?;
insert_edges(store, &edges).await?;
```

**EdgeUpdater Enhancement:**
```rust
pub async fn update_edges(&self, file_id: i64) -> Result<()> {
    // 1. Delete old edges (existing)
    self.delete_edges_for_file(file_id).await?;

    // 2. Recompute edges (NEW)
    let file = self.store.get_file_by_id(file_id).await?;
    let content = fs::read_to_string(&file.relpath)?;
    let chunks_with_ids = self.store.get_chunks_for_file(file_id).await?;
    let edges = edges::extract_edges(&content, &file.language, &chunks_with_ids)?;
    insert_edges(&self.store, &edges).await?;

    Ok(())
}
```

## Data Flow

**Scan Command Flow:**
```
File on disk
  ↓
Read content + detect language
  ↓
parser::extract_chunks(content, language) → Vec<SymbolChunk>
  ↓
Insert chunks → chunk IDs
  ↓
Load chunks with IDs from database → Vec<ChunkWithId>
  ↓
edges::extract_edges(content, language, chunks_with_ids) → Vec<Edge>
  ↓
insert_edges(store, edges) → batch INSERT
  ↓
Continue to next file
```

**Incremental Update Flow:**
```
File modification detected
  ↓
EdgeUpdater::update_edges(file_id)
  ↓
Delete edges WHERE src/dst IN (SELECT id FROM chunks WHERE file_id = ?)
  ↓
Re-parse file → new chunks
  ↓
extract_edges() → new edges
  ↓
insert_edges() → populate chunk_edges
```

## Performance Considerations

### Time Complexity

**Per File:**
- Tree-sitter parse: O(n) where n = file size (~5ms for 200 lines)
- Find call expressions: O(nodes) where nodes ≈ 3-5× LOC (~10ms)
- Build symbol table: O(chunks) (~1ms for 10 chunks)
- Resolve calls: O(calls × chunks) ≈ O(30 × 10) = O(300) lookups (~15ms)
- Batch insert: O(edges) (~5ms for 30 edges)
- **Total: ~36ms per file**

**Repository (1000 files):**
- Total edge extraction: ~36 seconds
- Baseline scan: ~120 seconds
- **Overhead: ~30% (acceptable)**

### Space Complexity

- Symbol table: O(chunks per file) ≈ 10 entries × 50 bytes = 500 bytes
- Edges buffer: O(calls per file) ≈ 30 edges × 24 bytes = 720 bytes
- **Per-file overhead: ~1.2 KB (negligible)**

### Database Impact

- Edge row: ~40 bytes (3 integers + type string + index overhead)
- 500K edges: ~20 MB
- Indexes: `chunk_edges(src_chunk_id)`, `chunk_edges(dst_chunk_id)` (already exist)
- Query performance: Recursive CTEs tested, <10ms for typical queries

### Optimization Strategies

1. **Batching:** Insert edges in single transaction per file
2. **Lazy Loading:** Only load chunks when edges are extracted
3. **Skip Empty:** Skip edge extraction if no call expressions found
4. **Parallel:** Leverage existing file parallelism in scan

## Maintainability

### Modularity

- Edge extraction isolated in `indexer/edges/` module
- Language-specific extractors (typescript.rs, python.rs)
- Common utilities shared (common.rs)
- Clear API boundary (`extract_edges()`)

### Extensibility

Adding new languages:
1. Create `edges/language_name.rs`
2. Implement `extract_calls(source, chunks)` function
3. Add case to `extract_edges()` dispatcher
4. Add tests

Adding new edge types:
1. Add variant to `EdgeType` enum
2. Implement extraction logic in language module
3. Update database queries if needed

### Testing

- Unit tests per language extractor
- Integration tests with synthetic repos
- Performance benchmarks
- Edge case handling (parse errors, ambiguous symbols)

### Logging

- Debug: Unresolved calls (may be cross-file)
- Info: Edge extraction stats per file (edges created)
- Warn: Parse failures, database errors
- Trace: Call expression details (verbose)

## Error Handling

### Parse Failures

Strategy: Log warning, return empty edge list (continue scan)

```rust
let tree = parser.parse(source, None).ok_or_else(|| {
    warn!("Failed to parse {} file for edge extraction", language);
})?;
// If parse fails, extract_edges returns Ok(Vec::new())
```

### Symbol Resolution Failures

Strategy: Skip edge (log at trace level to avoid noise)

```rust
if let Some(&callee_id) = symbol_table.get(&callee_name) {
    edges.push(edge);
} else {
    trace!("Unresolved call: {} (may be cross-file)", callee_name);
}
```

### Database Errors

Strategy: Log error, fail scan (data consistency priority)

```rust
insert_edges(store, &edges).await
    .context("Failed to insert edges for file")?;
```

## Security Considerations

**No New Attack Surface:**
- Read-only file operations
- Uses existing tree-sitter parsers (already trusted)
- Parameterized SQL queries (injection-safe)

**Performance DoS Mitigation:**
- Skip files >10,000 lines (existing limit)
- Tree-sitter parse timeout (30s, existing)
- Edge count limit: Warn if >1000 edges per file

## Future Extensions

### Phase 2: Cross-File Resolution

- Track import graph
- Resolve qualified names
- Create imports edges alongside calls

### Phase 3: Test Detection

- File path heuristics (`*.test.ts`, `__tests__/`)
- Create test_of edges
- Boost tested code in search

### Phase 4: Advanced Edges

- Route detection (Express, Next.js)
- Confidence scores (0.0-1.0)
- Called_by computed from calls
