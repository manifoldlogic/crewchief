# Ticket: IDXABS-6004: Complete All SQLite Stub Implementations

## Status
- [ ] **Task completed** - all TODOs resolved
- [ ] **Tests pass** - `cargo test -p crewchief-maproom`
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Complete all 52 TODO items across 21 files that were left as stubs during the PostgreSQL to SQLite migration. These functions return empty/placeholder values and need real implementations.

## Background
During the IDXABS migration, many functions were stubbed with TODO comments instead of being fully implemented. This affects:
- Search functionality (signals, graph, FTS, vector)
- Context assembly (assembler, cache, graph, detectors)
- Incremental indexing (already covered in IDXABS-6002)
- Language strategies (React, Python, Rust)

## TODO Inventory (52 items across 21 files)

### Incremental Module (13 TODOs) - Covered by IDXABS-6002
| File | Count | TODOs |
|------|-------|-------|
| `incremental/processor.rs` | 3 | SQLite file indexing, update, deletion |
| `incremental/detector.rs` | 4 | Hash retrieval, storage, move detection, batch query |
| `incremental/edge_updater.rs` | 4 | Edge updates, computation, test targets, insertion |
| `incremental/tree_sha_update.rs` | 2 | Worktree removal, incremental update |

### Search Module (7 TODOs)
| File | Count | TODOs |
|------|-------|-------|
| `search/pipeline.rs` | 1 | IDXABS-2003 placeholder |
| `search/signals.rs` | 2 | IDXABS-2003 placeholders (recency, churn scoring) |
| `search/graph.rs` | 2 | IDXABS-2003 placeholders (callers, callees) |
| `search/fts.rs` | 1 | IDXABS-2003 placeholder |
| `search/vector.rs` | 1 | IDXABS-2003 placeholder |

### Context Module (21 TODOs)
| File | Count | TODOs |
|------|-------|-------|
| `context/cache.rs` | 8 | IDXABS-4001 - cache operations |
| `context/graph.rs` | 3 | IDXABS-4001 - graph queries |
| `context/assembler.rs` | 3 | IDXABS-4001 - context assembly, truncation |
| `context/detectors/hooks.rs` | 3 | IDXABS-4001 - React hooks detection |
| `context/detectors/jsx.rs` | 3 | IDXABS-4001 - JSX detection |

### Context Strategies (6 TODOs)
| File | Count | TODOs |
|------|-------|-------|
| `context/strategies/react.rs` | 4 | Route, hook, JSX parent/child queries |
| `context/strategies/python.rs` | 1 | Parent class queries |
| `context/strategies/rust.rs` | 1 | Trait impl queries |

### Other Modules (5 TODOs)
| File | Count | TODOs |
|------|-------|-------|
| `main.rs` | 1 | watch_worktree removal note (covered by IDXABS-6003) |
| `migrate/markdown.rs` | 2 | FTS tsvector, blob SHA computation |
| `embedding/pipeline.rs` | 1 | Repo/worktree filtering |
| `db/sqlite/mod.rs` | 2 | Kind multipliers for scoring |

## Acceptance Criteria
- [ ] All 52 TODOs either implemented or documented as intentionally deferred
- [ ] No placeholder implementations that return empty results
- [ ] All search functionality works correctly
- [ ] All context assembly works correctly
- [ ] Tests validate each implemented function

## Implementation Priority

### High Priority (Core Functionality)
1. **Search module** (7 TODOs) - Required for search to work correctly
2. **Context assembler** (3 TODOs) - Required for context retrieval
3. **DB sqlite kind multipliers** (2 TODOs) - Affects search ranking

### Medium Priority (Enhanced Features)
4. **Context cache** (8 TODOs) - Performance optimization
5. **Context graph** (3 TODOs) - Relationship queries
6. **Language strategies** (6 TODOs) - Language-specific context

### Lower Priority (Edge Cases)
7. **Context detectors** (6 TODOs) - React/JSX specific
8. **Migrate markdown** (2 TODOs) - Markdown import
9. **Embedding pipeline** (1 TODO) - Filtering enhancement

## Technical Requirements

### Search Module Implementation

**search/signals.rs - Recency and Churn Scoring**
```rust
// Currently returns placeholder values
// Need to query file modification times and git history
```

**search/graph.rs - Caller/Callee Queries**
```rust
// Use SqliteStore.find_callers() and find_callees() methods
// These methods already exist in SqliteStore
```

### Context Module Implementation

**context/cache.rs - Cache Operations**
```rust
// Implement using SqliteStore chunk retrieval methods
// Methods like get_chunk_by_id(), get_file_chunks() exist
```

**context/graph.rs - Graph Queries**
```rust
// Use SqliteStore graph traversal methods
// find_callers, find_callees, find_imports exist
```

### SqliteStore Methods Available
The following methods already exist and should be used:
- `get_chunk_by_id()` - Get full chunk details
- `get_file_chunks()` - Get chunks for a file
- `find_callers()` - Graph traversal for callers
- `find_callees()` - Graph traversal for callees
- `find_imports()` - Import relationships
- `find_extensions()` - Type extensions
- `search_chunks_fts()` - Full-text search
- `search_chunks_vector()` - Vector similarity search
- `search_chunks_hybrid()` - Hybrid search

## Dependencies
- IDXABS-6001 (tests must compile)
- IDXABS-6002 (incremental module - 13 TODOs overlap)
- IDXABS-6003 (watch command)

## Risk Assessment
- **Risk**: Some TODOs may require new SqliteStore methods
  - **Mitigation**: Check existing methods first, add only if needed
- **Risk**: Performance impact from repeated database queries
  - **Mitigation**: Use batch operations where possible

## Files/Packages Affected
21 files across the maproom crate (see inventory above)

## Estimated Effort
High - 52 TODOs to resolve, significant implementation work.
Recommend splitting into sub-tickets by module if needed.
