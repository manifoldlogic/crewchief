# Project: Edge Extraction

**Slug:** EDGEEXT
**Status:** Planning Complete
**Created:** 2025-12-14
**Priority:** HIGH (Blocks SRCHREL_relationship-aware-search)

## Summary

Implement edge extraction during indexing to populate the `chunk_edges` table with code relationships (function calls, imports, test links). Currently, the table exists but is empty (0 rows) despite 140,338 indexed chunks. This blocks SRCHREL's quality-weighted scoring feature, which requires edge data to compute in-degree (number of callers) and distinguish core implementations from peripheral utilities.

**Impact:** Without edge extraction, graph-based search ranking degrades to simple text matching, related code discovery fails, and context assembly provides minimal value.

## Problem Statement

The EdgeUpdater module (`crates/maproom/src/incremental/edge_updater.rs`) is a placeholder that deletes edges but never computes new ones. During indexing, chunks are extracted and stored, but no code populates `chunk_edges` with relationship information.

**Specific Gap:** No integration between tree-sitter parsing (which can identify call expressions) and the database layer (which has `insert_chunk_edge()` ready to use).

**Business Impact:**
- Graph features unusable (find_callers, find_callees return empty)
- SRCHREL project blocked (cannot implement quality scoring)
- Context assembly provides minimal value
- Search ranking cannot boost architecturally significant code

## Proposed Solution

**Extract edges during indexing (not as separate post-processing):**

1. **Phase 1 (MVP - 1 week):** TypeScript/JavaScript `calls` edges with same-file resolution
   - Extract call expressions via tree-sitter traversal
   - Resolve symbols within same file (high confidence, 85% accuracy)
   - Insert edges batch-style during scan/upsert
   - Integrate with EdgeUpdater for incremental updates

2. **Phase 2 (1 week):** Cross-file resolution + test edges
   - Add database lookups for cross-file calls (60% accuracy)
   - Detect test files via path heuristics (`*.test.ts`)
   - Create `test_of` edges linking tests to implementations

3. **Phase 3 (1 week):** Python + Rust support
   - Extend pattern to Python (call extraction)
   - Extend pattern to Rust (call extraction)
   - Document approach for adding new languages

**Key Decisions:**
- **Same-file first:** 70-80% of calls are same-file (high ROI, simple)
- **Calls only (MVP):** Most valuable for SRCHREL quality scoring
- **TypeScript/JavaScript first:** 60-70% of codebases
- **Batch insertion:** Minimize database round-trips (5ms vs 150ms)
- **Integration point:** After chunk insertion (matches Python imports pattern)

## Relevant Agents

### Planning Phase
- **project-planner:** Complete planning documents ✓

### Implementation Phase

**Phase 1: TypeScript/JavaScript Calls (Week 1)**
- **rust-expert:** Edge extractor module, tree-sitter traversal, symbol resolution
- **database-engineer:** Review batch insertion, ensure `insert_chunk_edge()` works
- **test-engineer:** Unit tests, integration tests with synthetic repos

**Phase 2: Cross-File + Test Edges (Week 2)**
- **rust-expert:** Cross-file resolution, test file detection
- **database-engineer:** Optimize `find_chunk_by_symbol()` for batching
- **test-engineer:** Accuracy validation, integration tests

**Phase 3: Python + Rust (Week 3)**
- **rust-expert:** Language-specific extractors
- **test-engineer:** Language-specific tests

### Verification Phase
- **verify-ticket:** Validate acceptance criteria (edge count, accuracy, performance)
- **commit-ticket:** Create commits for completed work

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem definition, research findings, constraints, success criteria
- [architecture.md](planning/architecture.md) - Solution design, component architecture, integration points, performance considerations
- [plan.md](planning/plan.md) - 3-phase execution plan with agent assignments and timelines
- [quality-strategy.md](planning/quality-strategy.md) - Testing philosophy, critical paths, accuracy validation
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk, server-side optimization)

## Dependencies

### External (BLOCKING THIS PROJECT)
- Database schema: `chunk_edges` table exists ✓
- Database API: `SqliteStore::insert_chunk_edge()` exists ✓
- Tree-sitter parsers: Installed and working ✓
- Graph queries: Recursive CTEs tested ✓

### Internal (BLOCKED BY THIS PROJECT)
- **SRCHREL_relationship-aware-search:** Requires populated edges for quality scoring
- **Context assembly:** Needs edges for caller/callee relationships
- **Related code discovery:** Depends on edge traversal

## Key Metrics

**Must Achieve (Phase 1 MVP):**
- [ ] `chunk_edges` table populated (≥10,000 edges for test repo)
- [ ] Same-file calls: ≥85% precision, ≥60% recall
- [ ] Performance overhead <30% (scan time)
- [ ] Incremental updates work (edges recomputed on file change)
- [ ] SRCHREL unblocked (can implement quality scoring)

**Should Achieve (Phase 2):**
- [ ] Cross-file calls: ≥60% accuracy
- [ ] `test_of` edges created for test files
- [ ] Performance optimized (batched queries)

**Nice to Have (Phase 3):**
- [ ] Python calls extracted
- [ ] Rust calls extracted
- [ ] Documentation for adding new languages

## Timeline

**Total Duration:** 2-3 weeks

- **Week 1:** Phase 1 - TypeScript/JavaScript calls (same-file)
  - Edge extractor module + TypeScript traversal
  - Integration with scan/upsert + EdgeUpdater
  - Unit + integration tests

- **Week 2:** Phase 2 - Cross-file resolution + test edges
  - Database lookup for cross-file calls
  - Test file detection + `test_of` edges
  - Performance optimization

- **Week 3:** Phase 3 - Python + Rust support (can overlap with SRCHREL)
  - Python/Rust extractors
  - Language-specific tests
  - Documentation

## Next Steps

1. **Review Planning:** Run `/review-project EDGEEXT` to validate planning documents
2. **Create Tickets:** Run `/create-project-tickets EDGEEXT` to generate implementation tickets
3. **Begin Implementation:** Start with Phase 1 (TypeScript/JavaScript calls)

## Architecture Highlights

**Component Structure:**
```
crates/maproom/src/indexer/edges/
├── mod.rs           # Public API: extract_edges(source, language, chunks)
├── typescript.rs    # TypeScript/JavaScript call extraction
└── common.rs        # Shared utilities
```

**Data Flow:**
```
File on disk
  ↓
Read content + detect language
  ↓
parser::extract_chunks(content, language) → Vec<SymbolChunk>
  ↓
Insert chunks → chunk IDs
  ↓
Load chunks with IDs from database
  ↓
edges::extract_edges(content, language, chunks_with_ids) → Vec<Edge>
  ↓
insert_edges(store, edges) → batch INSERT into chunk_edges
  ↓
Continue to next file
```

**Integration Points:**
- `scan_worktree()` - Add edge extraction after chunk insertion (line ~435)
- `upsert_files()` - Add edge extraction after chunk insertion (line ~625)
- `EdgeUpdater::update_edges()` - Enhance to recompute edges (line ~115)

**Performance Budget:**
- Tree-sitter parse: ~5ms per file
- Symbol resolution: ~15ms per file (same-file only)
- Batch insertion: ~5ms for 30 edges
- **Total: ~25-35ms per file (30% overhead)**

## Testing Strategy

**Critical Paths (MUST test):**
1. Edge extraction pipeline (scan → extract → insert → populate database)
2. Incremental updates (file change → edges recomputed)
3. Symbol resolution (same-file calls resolved correctly)
4. Error handling (parse failures, database errors)
5. Performance budget (<30% overhead)

**Test Types:**
- **Unit:** Call extraction, symbol resolution, edge struct creation
- **Integration:** End-to-end scan with synthetic repos
- **Performance:** Measure scan time overhead, verify no memory leaks

**Synthetic Test Repos:**
- `typescript_calls/` - Simple call chains
- `typescript_methods/` - Object methods and class methods
- `typescript_complex/` - Nested calls, callbacks

**Quality Gates:**
- All unit tests pass (`cargo test`)
- Integration tests pass (synthetic repos)
- Performance tests pass (<30% overhead)
- No linting errors (`cargo clippy`)
- Accuracy validation ≥70% on sample repository

## Security Assessment

**Risk Level:** LOW

- No new authentication/authorization
- No user-facing features (server-side only)
- Read-only file operations
- Parameterized SQL queries (injection-safe)
- Performance DoS mitigated (skip files >10,000 lines, warn if >1000 edges per file)

**Approved for MVP** with standard performance limits and error handling.

## Agent Recommendation

**Custom agents NOT recommended** for this project.

**Rationale:** Well-suited for general agents (rust-expert, database-engineer, test-engineer). Straightforward enhancement to existing indexing pipeline using established patterns from Python imports. No deep specialized domain expertise required.

**General agents are sufficient for:**
- Rust module creation and tree-sitter integration
- Database batch operations (existing pattern)
- Unit and integration testing (standard approach)
- Performance optimization (profiling tools)

## References

**Codebase Locations:**
- EdgeUpdater: `crates/maproom/src/incremental/edge_updater.rs`
- Database API: `crates/maproom/src/db/sqlite/mod.rs:684-691` (`insert_chunk_edge`)
- Graph traversal: `crates/maproom/src/db/sqlite/graph.rs`
- Python imports: `crates/maproom/src/indexer/mod.rs:123-176` (existing pattern)
- Parser: `crates/maproom/src/indexer/parser.rs`

**Related Projects:**
- [SRCHREL_relationship-aware-search](../SRCHREL_relationship-aware-search/) - Blocked by this project

**Documentation:**
- Database Architecture: `docs/architecture/DATABASE_ARCHITECTURE.md`
- Maproom Architecture: `docs/architecture/MAPROOM_ARCHITECTURE.md`
