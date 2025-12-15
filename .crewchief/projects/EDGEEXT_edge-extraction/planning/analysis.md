# Analysis: edge extraction

## Problem Definition

The `chunk_edges` table exists but remains empty (0 rows) despite 140,338 chunks being indexed. The EdgeUpdater module (`crates/maproom/src/incremental/edge_updater.rs`) is a placeholder that deletes edges but never computes new ones. This blocks the SRCHREL_relationship-aware-search project, which depends on populated edge data for quality-weighted scoring.

**Specific Gap:** During indexing, chunks are extracted and stored via `parser::extract_chunks()`, but no code populates the `chunk_edges` table with relationship information (calls, imports, test_of, etc.).

**Business Impact:** Without edge extraction:
- Graph-based search ranking degrades to simple text matching
- Related code discovery fails (no callers/callees shown)
- Context assembly provides minimal value (empty relationships)
- Quality-weighted scoring cannot differentiate important vs peripheral code

## Context

### Existing Infrastructure

**Database Schema (Ready):**
```sql
CREATE TABLE chunk_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    src_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    dst_chunk_id INTEGER NOT NULL REFERENCES chunks(id) ON DELETE CASCADE,
    type TEXT NOT NULL,  -- 'imports', 'exports', 'calls', 'called_by', 'test_of', 'route_of'
    UNIQUE(src_chunk_id, dst_chunk_id, type)
);
```

**Edge Types Defined:**
- `imports` - Symbol imports another symbol (module dependency)
- `exports` - Symbol exports another symbol (API surface)
- `calls` - Function calls another function (execution flow)
- `called_by` - Inverse of calls (who depends on me)
- `test_of` - Test targets a specific function/class (quality signal)
- `route_of` - Route handler for a specific path (entry point)

**Database Layer (Complete):**
- `SqliteStore::insert_chunk_edge()` exists at `crates/maproom/src/db/sqlite/mod.rs:684-691`
- Graph traversal queries use recursive CTEs (tested)
- Edge deletion logic is functional

**Tree-Sitter Parsers (Available):**
- TypeScript/JavaScript/TSX via `tree-sitter-typescript`, `tree-sitter-javascript`
- Python via `tree-sitter-python`
- Rust via `tree-sitter-rust`
- Go via `tree-sitter-go`

**Partial Implementation:**
- Python import extraction exists (lines 123-176 in `indexer/mod.rs`)
- Extracts import statements into chunk metadata
- Creates `imports` edges by resolving symbol names
- Pattern: Parse → Match symbols → Create edges

### Why This Work Is Needed

**SRCHREL Blocker:** The relationship-aware search project (SRCHREL_relationship-aware-search) requires populated edges to implement quality-weighted scoring. Without edges:
- Cannot compute in-degree (number of callers)
- Cannot distinguish core implementations from utility functions
- Cannot boost architecturally significant code in search results

**Graph Features Unusable:** Multiple features depend on edges:
- Context assembly (`find_callers`, `find_callees` in `graph.rs`)
- Related code discovery
- Dependency analysis
- Test-to-implementation linking

**Performance:** Current graph queries are fast (recursive CTEs, indexed) but return empty results. The infrastructure is ready; only extraction is missing.

## Existing Solutions

### Industry Patterns

**LSP Servers:** Extract call graphs using AST traversal
- Tree-sitter provides CST (concrete syntax tree) nodes
- Call expressions link to function identifiers
- Resolution requires symbol table or heuristics

**Static Analysis Tools:** Build dependency graphs incrementally
- Store edges during initial parse
- Update edges on file modification
- Use symbol name matching for cross-file links

**Code Intelligence Platforms (Sourcegraph, GitHub):**
- Extract definitions and references
- Match by fully qualified names
- Prioritize same-file links (high confidence)

### Codebase: Python Import Extraction (Working)

**Location:** `crates/maproom/src/indexer/mod.rs:123-176`

**How it works:**
1. `parser::extract_chunks()` extracts Python imports into metadata
2. `process_python_imports()` parses metadata after chunk insertion
3. For each import, resolves symbol name via `find_chunk_by_symbol()`
4. Creates `imports` edge if target found

**Key Pattern:**
```rust
// 1. Extract during parse
let chunks = parser::extract_chunks(&content, "py");

// 2. Process after chunks inserted
for import in imports_metadata {
    let dst_chunk_id = store.find_chunk_by_symbol(repo_id, worktree_id, symbol_name)?;
    store.insert_chunk_edge(src_chunk_id, dst_chunk_id, "imports")?;
}
```

**Limitations:**
- Python-only (no TypeScript, Rust, Go)
- Only handles imports (no calls, test_of, etc.)
- Post-processing step (not during chunk extraction)

## Current State

### EdgeUpdater Placeholder

**Location:** `crates/maproom/src/incremental/edge_updater.rs`

**Current State:**
- Deletes stale edges correctly (lines 115-133)
- Stub for edge computation (returns empty Vec at line 242)
- Helper functions (`is_test_chunk`, `is_route_chunk`) partially implemented
- Marked with `#![allow(dead_code)]` (line 9)

**Missing:**
- Tree-sitter traversal for call expressions
- Symbol resolution logic
- Integration with parser
- Actual edge computation in `compute_edges()` function

### Database State

**Current:**
- `chunk_edges` table: 0 rows
- `chunks` table: 140,338 rows
- Database queries work (tested in graph.rs tests)

**Expected After Implementation:**
- 10,000-50,000 edges for typical repository
- Calls edges for TypeScript/JavaScript (Phase 1)
- Imports and test_of edges (Phase 2)

## Research Findings

### Tree-Sitter Call Expression Patterns

**TypeScript/JavaScript:**
- Node type: `call_expression`
- Children: `function` (identifier or member_expression), `arguments`
- Example: `foo()` → `call_expression(identifier("foo"), arguments)`
- Member calls: `obj.method()` → `call_expression(member_expression, arguments)`

**Python:**
- Node type: `call`
- Children: `function` (identifier or attribute), `argument_list`
- Similar structure to JS

**Rust:**
- Node type: `call_expression`
- Children: `function` (path or identifier), `arguments`
- Turbofish syntax: `foo::<T>()` requires special handling

### Symbol Resolution Heuristics

**Same-File Resolution (High Confidence):**
```rust
// 1. Extract function name from call_expression
let function_name = extract_function_identifier(call_node)?;

// 2. Find chunk with matching symbol_name in same file
let target = chunks.iter().find(|c| c.symbol_name == Some(function_name));

// 3. Create edge
if let Some(target_chunk) = target {
    edges.push(Edge { src: caller_chunk_id, dst: target_chunk.id, type: "calls" });
}
```

**Cross-File Resolution (Best Effort):**
```rust
// Use existing find_chunk_by_symbol() from Python imports
let target_chunk_id = store.find_chunk_by_symbol(repo_id, worktree_id, function_name, None)?;
```

### Performance Benchmarks (Estimated)

**Edge Extraction Cost:**
- Tree-sitter traversal: ~5ms per file (already done for chunk extraction)
- Symbol resolution (same-file): ~1ms per call (in-memory lookup)
- Symbol resolution (cross-file): ~10ms per call (database query)
- Edge insertion: ~0.5ms per edge (batched)

**Typical File:**
- 10 chunks, 30 call expressions
- Same-file: 24 calls (24ms)
- Cross-file: 6 calls (60ms)
- **Total: ~90ms per file**

**Optimization:** Batch database queries for cross-file resolution (10ms total instead of 60ms).

**Repository (1000 files):**
- Total edge extraction: ~35 seconds
- Baseline scan: ~120 seconds
- **Overhead: ~29% (within acceptable range)**

## Constraints

### Technical Constraints

1. **Symbol Resolution Complexity**
   - No type system or LSP (cannot resolve qualified names accurately)
   - Cross-file resolution is heuristic (name matching only)
   - Ambiguity: Multiple chunks with same symbol name

2. **Tree-Sitter Limitations**
   - Provides CST (not AST) - more verbose traversal
   - No semantic analysis (cannot determine if identifier is function call vs property access)
   - Language-specific node types require per-language code

3. **Performance Budget**
   - Indexing already scans all files
   - Adding edge extraction must stay within ~20-30% overhead
   - 140K chunks → potentially 500K+ edges (storage + query impact)

4. **Incremental Updates**
   - Edge extraction must integrate with incremental indexer
   - File modification triggers edge recomputation
   - EdgeUpdater already has deletion logic

### Business Constraints

1. **MVP Timeline**
   - SRCHREL is blocked (HIGH priority)
   - Need fastest path to unblock relationship-aware search
   - Can iterate on accuracy after initial implementation

2. **Accuracy vs Speed Trade-off**
   - Perfect resolution not required for MVP
   - Heuristic matching acceptable (70-80% accuracy)
   - Same-file edges more valuable than cross-file (lower risk)

3. **Language Coverage**
   - TypeScript/JavaScript most critical (codebase majority)
   - Python already has imports (extend to calls)
   - Rust/Go nice-to-have (can defer)

## Success Criteria

### Must Achieve (MVP)

1. **Functional Edge Extraction**
   - [ ] `chunk_edges` table populated during scan
   - [ ] `calls` edges extracted for TypeScript/JavaScript
   - [ ] Edges survive incremental updates (EdgeUpdater integration)
   - [ ] At least 10,000 edges in test repository (140K chunks)

2. **Accuracy Targets**
   - [ ] ≥70% precision (edges created are correct)
   - [ ] ≥60% recall (most actual calls captured)
   - [ ] Same-file calls: ≥85% accuracy (easier to resolve)

3. **Performance Budget**
   - [ ] Scan time increase <30% (baseline: ~2-3s for small repo)
   - [ ] Edge extraction: <100ms per file (typical file has 10-20 chunks)
   - [ ] No memory leaks or unbounded allocations

4. **Integration Complete**
   - [ ] Works with `scan` command (initial indexing)
   - [ ] Works with `upsert` command (file updates)
   - [ ] EdgeUpdater deletes + recomputes edges correctly

### Should Achieve (Iteration 2)

1. **Extended Coverage**
   - [ ] `imports` edges for TypeScript/JavaScript (parity with Python)
   - [ ] `test_of` edges using file path heuristics (`*.test.ts`, `__tests__/`)
   - [ ] Python `calls` extraction (extend existing Python support)

2. **Improved Accuracy**
   - [ ] Cross-file resolution for exports (module APIs)
   - [ ] Distinguish method calls from function calls (attach to class chunk)

3. **Observability**
   - [ ] Log edge extraction stats (edges created, resolution failures)
   - [ ] Metrics: edges/file, resolution success rate

### Nice to Have (Future)

1. **Advanced Features**
   - [ ] `route_of` edges for Express/Next.js routes
   - [ ] `called_by` computed from `calls` (inverse edges)
   - [ ] Rust trait/impl edges
   - [ ] Go interface satisfaction edges

2. **Quality Improvements**
   - [ ] Qualified name resolution (import path + symbol)
   - [ ] Confidence scores on edges (same-file: 1.0, cross-file: 0.7)
   - [ ] Duplicate edge detection (multiple call sites)
