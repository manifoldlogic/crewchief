# Project Review: SQLITE - Full SQLite Implementation

**Review Date:** 2025-11-26
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

The SQLITE project has well-structured planning documents with clear goals, detailed architecture, and comprehensive quality strategy. The decision to build SQLite-native (not behind a trait abstraction) is pragmatic and appropriate for MVP velocity.

However, there are several significant issues that need attention before ticket creation:

1. **Reinvention concern**: The architecture proposes new modules (`hybrid.rs`, `embeddings.rs`, `graph.rs`) that partially duplicate existing PostgreSQL logic without considering reuse of database-agnostic utilities like `normalize_for_exact_match()` and RRF scoring.

2. **Schema conflict**: The architecture proposes removing `worktree_ids JSON` column and adding a junction table, but the existing implementation already has this column in production schema (from SQLFIX). Migration path not addressed.

3. **sqlite-vec complexity understated**: The extension is already bundled and loaded via `sqlite3_auto_extension`, but the architecture doesn't acknowledge this existing implementation or its limitations (hardcoded 1536-dim in schema).

4. **Missing embedding dimension support**: Current SQLite schema has hardcoded `float[1536]` in vec_chunks, but the project claims to support 768-dim (Ollama). No migration or dual-table strategy specified.

The project is feasible but needs refinement in a few areas before proceeding.

## Critical Issues (Blockers)

### Issue 1: Schema Migration Strategy Missing
**Severity:** Critical
**Category:** Architecture
**Description:** The architecture proposes significant schema changes (junction table replacing JSON column, new embedding tables) but the existing SQLite implementation already has a production schema from SQLFIX. There's no migration strategy - only `init_schema()` which creates tables if not exists.

**Impact:** Users with existing SQLite databases will lose data or face corruption when schema changes are applied.

**Required Action:**
1. Define a migration versioning system (schema_migrations table proposed but not implemented)
2. Create explicit migration scripts for each schema change
3. Add backwards-compatible rollback support
4. Test upgrade path from current SQLFIX schema

**Documents Affected:** architecture.md, plan.md

### Issue 2: Hardcoded 1536-dim Vector Tables
**Severity:** Critical
**Category:** Architecture
**Description:** Current schema.rs line 96-100 creates `vec_chunks USING vec0(code_embedding float[1536])`. Architecture proposes `vec_code_768` as a separate table but doesn't address:
- How to route queries to the correct table
- How to handle mixed-dimension deployments
- Migration from current schema

**Impact:** 768-dim embeddings (Ollama) won't work without schema changes, breaking a key stated goal.

**Required Action:**
1. Clarify dimension handling strategy in architecture.md
2. Either: (a) create separate tables per dimension, or (b) use a single table with dimension column
3. Add explicit migration ticket for dimension support

**Documents Affected:** architecture.md, schema.rs

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds

| Existing Solution | Proposed Duplication | Recommendation |
|------------------|---------------------|----------------|
| `normalize_for_exact_match()` in `search/fts.rs` | New exact match logic in `hybrid.rs` | Import and reuse the function |
| `ColumnSet` and `select_columns_for_dimension()` in `db/columns.rs` | Architecture mentions dimension handling without referencing | Use existing abstraction |
| `RankedResult`, `RankedResults` structs in `search/executor_types.rs` | New `FtsResult`, `VectorResult`, `SearchHit` structs | Consider if compatible |

**Wasted Effort:** ~2-4 hours if logic is reimplemented

### Missed Reuse Opportunities

| Available Component | Could Solve | Integration Method | Effort |
|--------------------|-------------|-------------------|--------|
| `normalize_for_exact_match()` | Exact match detection in semantic ranking | Direct import (same crate) | Low |
| `select_columns_for_dimension()` | Dimension-aware column selection | Direct import | Low |
| RRF constants and formula | Hybrid search fusion | Port pure logic (no DB deps) | Low |

### Pattern Violations

| Existing Pattern | Proposed Deviation | Impact |
|-----------------|-------------------|--------|
| VectorStore trait with 13 methods | "SQLite-only, no trait" | Acceptable - trait is already implemented |
| `spawn_blocking` for all DB ops | Architecture doesn't mention | Should preserve existing pattern |
| FTS5 external content with manual INSERT | Architecture proposes triggers | Inconsistent approach - pick one |

## High-Risk Areas (Warnings)

### Risk 1: sqlite-vec Extension Loading Race Condition
**Risk Level:** High
**Category:** Technical
**Description:** Current code uses `sqlite3_auto_extension(Some(std::mem::transmute(...)))` before pool creation. This is correct but fragile. If any future code creates a connection before this line, vector operations will fail silently.

**Probability:** Medium
**Impact:** High - vector search completely broken
**Mitigation:** Add runtime check that extension is loaded, fail fast with clear error message

### Risk 2: FTS5 External Content Table Synchronization
**Risk Level:** High
**Category:** Technical
**Description:** Architecture proposes triggers for FTS sync, but current implementation uses manual INSERT. The FTS5 external content table (`content='chunks'`) requires explicit maintenance. Mixed approaches could cause index desync.

**Probability:** Medium
**Impact:** High - search results become stale/missing
**Mitigation:** Pick one approach (triggers OR manual) and document clearly. Triggers are more robust.

### Risk 3: No code_embeddings Table in SQLite
**Risk Level:** High
**Category:** Architecture
**Description:** PostgreSQL uses `code_embeddings` table for deduplication (blob_sha → embedding). Architecture proposes this but current SQLite has `vec_chunks` keyed by chunk_id. This is fundamentally different - chunk_id doesn't deduplicate.

**Probability:** High (architectural gap)
**Impact:** High - 70-90% storage waste, slower queries
**Mitigation:** First ticket must create code_embeddings table and establish blob_sha JOIN pattern

### Risk 4: Graph Traversal Performance Unknown
**Risk Level:** Medium
**Category:** Technical
**Description:** Recursive CTEs in SQLite can be slow for deep graphs. PostgreSQL has optimized recursive query handling. No performance baseline or limits specified.

**Probability:** Medium
**Impact:** Medium - slow graph queries
**Mitigation:** Add max_depth limits (already in plan), add performance test for 100+ node graphs

## Gaps & Ambiguities

### Requirements Gaps

1. **FTS column weights**: Architecture mentions `FtsWeights` struct but plan has no ticket for implementing weighted BM25 search
2. **Graceful degradation**: What happens if sqlite-vec extension missing? FTS-only fallback?
3. **Database file location**: Architecture says `~/.maproom/maproom.db` but current code takes path as parameter

### Technical Gaps

1. **Blob-to-vector conversion format**: Architecture mentions "float32 as bytes" but sqlite-vec requires specific format (`vec_f32('[0.1, 0.2, ...]')`)
2. **FTS5 rank normalization**: Architecture mentions normalizing rank to 0-1 but doesn't specify formula (rank is negative, more negative = better)
3. **Connection pool and async**: Current code uses `spawn_blocking` pattern - architecture should explicitly preserve this

### Process Gaps

1. **No ticket for migration system**: Plan assumes migrations work but no ticket creates the system
2. **Testing sqlite-vec in CI**: Risk mitigation mentions this but no ticket addresses it

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **Phase 5 (Code Reorganization)** - extracting to crud.rs, error handling - could be done incrementally, not as separate phase
2. **Dual dimension support** - adding 768-dim is feature creep if 1536 works. Consider deferring

### Feasibility Challenges

1. **sqlite-vec MATCH semantics**: Different from pgvector's `<=>` operator. Need to verify L2 vs cosine distance
2. **Junction table JOIN performance**: Adding JOIN for every worktree filter query may impact performance

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate
- Core functionality is well-scoped
- Phase 5 (reorg) and Phase 6 (integration) could be trimmed
- Dual dimension support adds complexity without immediate value

### Pragmatism Score
**Rating:** Strong
- SQLite-native approach avoids abstraction overhead
- No unnecessary "enterprise" features
- Testing strategy is confidence-focused

### Agent Compatibility
**Rating:** Strong
- Phases are clear with explicit deliverables
- Tickets are sized appropriately (2-8 hours)
- Single agent assignment (rust-indexer-engineer) is appropriate

### Codebase Integration
**Rating:** Weak
- Doesn't acknowledge existing implementation in sqlite/mod.rs
- Misses reuse opportunities for exact match, column selection
- Proposes new structs that may conflict with existing types

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] **Dependencies on existing systems documented** - Missing acknowledgment of existing sqlite/mod.rs

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [ ] **Integration points are well-defined** - vec_chunks vs code_embeddings gap
- [x] Performance requirements are clear
- [x] Error handling is specified
- [ ] **Existing tools/libraries identified for reuse** - normalize_for_exact_match not referenced
- [ ] **No unnecessary duplication of functionality** - Some duplication identified

### Integration & Reuse
- [ ] **Existing solutions evaluated before building new** - Partially
- [x] Current patterns and conventions followed
- [ ] **Reusable components identified** - Some missed
- [ ] **Integration points with existing systems mapped** - Gap in schema understanding
- [ ] **No reinvention of available functionality** - Some concerns

### Risk
- [x] Major risks are identified
- [ ] **Mitigation strategies exist** - Some gaps
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Add Schema Migration Ticket (Phase 0)**: Before any other work, create migration versioning system. Add `schema_migrations` table, version check on connect, migration runner.

2. **Clarify Embedding Table Design**: Decide between:
   - Option A: `code_embeddings` table (like PostgreSQL) with blob_sha PK
   - Option B: Keep `vec_chunks` but add blob_sha JOIN
   Document decision in architecture.md

3. **Reference Existing Utilities**: Update architecture.md to explicitly reference:
   - `src/search/fts.rs::normalize_for_exact_match()` for exact match detection
   - `src/db/columns.rs::select_columns_for_dimension()` for dimension handling (or explain why not applicable)

4. **FTS Sync Strategy**: Choose triggers OR manual sync, not both. Update architecture.md.

### Phase 1 Adjustments

- **Combine Phase 0 tickets**: Schema Migration and CRUD Updates can be one ticket (schema changes require CRUD changes)
- **Defer 768-dim support**: Implement 1536-dim first, add 768 as enhancement ticket
- **Add migration ticket first**: Before schema changes, migration system must exist

### Risk Mitigations

1. **Add sqlite-vec verification test**: First ticket should include test that verifies extension loaded correctly
2. **Add FTS rebuild command**: If FTS gets out of sync, users need way to rebuild
3. **Document WAL cleanup**: WAL files can grow large - document how to checkpoint

### Documentation Updates

- **architecture.md**: Add section on existing implementation, what's being preserved vs changed
- **plan.md**: Add Phase -1 or move migration system to start of Phase 0
- **quality-strategy.md**: Add test for extension loading failure

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. Schema migration strategy is completely missing - this must be addressed first
2. Embedding table design (vec_chunks vs code_embeddings) needs clarification
3. Some reusable utilities not referenced, leading to potential duplication

### Recommended Path Forward

**REVISE THEN PROCEED:** Address critical issues (schema migration, embedding table design) and high-risk items (extension verification) before creating tickets. The project is well-conceived but has execution gaps that could cause rework.

### Success Probability
Given current state: **70%**
After recommended changes: **90%**

### Final Notes

The planning is thorough and the SQLite-native approach is correct for MVP. The main gaps are:
1. Not fully acknowledging what already exists (sqlite/mod.rs has 645 lines of working code)
2. Migration strategy is critical for any schema changes and is missing
3. Some database-agnostic utilities in the crate could be reused

Once these are addressed, ticket creation should proceed smoothly. The rust-indexer-engineer agent is well-suited for this work.
