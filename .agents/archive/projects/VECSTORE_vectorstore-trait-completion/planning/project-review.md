# Project Review: VECSTORE

**Review Date:** 2025-11-26
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

VECSTORE is a well-defined, appropriately scoped project that addresses a genuine architectural gap. The analysis correctly identifies that the `VectorStore` trait is incomplete, with 47 files bypassing the trait to call PostgreSQL directly. The proposed solution—expanding the trait incrementally—follows established Rust patterns and builds on existing work.

**Strengths:**
- Excellent codebase audit identifying specific bypass locations
- Clear scope boundaries with explicit "out of scope" definitions
- Appropriate use of existing SQLite module implementations (vector.rs, hybrid.rs, fts.rs, graph.rs)
- Well-structured phases with logical dependencies

**Primary Concerns:**
1. **Scope Ambiguity in Analysis**: Success criteria #3 claims "CLI, daemon, and indexer use `Arc<dyn VectorStore>`" but this is explicitly out of scope in the same document (MAPCLI project). This needs clarification.
2. **Missing PostgreSQL Query Functions**: Some proposed trait methods don't have existing PostgreSQL implementations in `queries.rs` (e.g., `search_chunks_vector`, `get_chunk_by_id`). The plan assumes wrapping exists when it doesn't.
3. **Context Module Coupling**: The `context/` module contains 10+ functions that query PostgreSQL directly. These need to be wired through the trait, but they have complex dependencies.

**Recommendation:** Proceed after clarifying success criteria and verifying PostgreSQL function existence for each proposed method.

## Critical Issues (Blockers)

### Issue 1: Conflicting Success Criteria

**Severity:** Critical
**Category:** Requirements

**Description:** The analysis.md states success criteria #3 as "CLI, daemon, and indexer use `Arc<dyn VectorStore>`" but also lists CLI migration, daemon migration, and indexer migration as "Out of Scope" items belonging to MAPCLI project.

**Impact:** Agents will be confused about what constitutes project completion. The project could either be declared complete prematurely or work could drift into MAPCLI scope.

**Required Action:**
1. Remove success criteria #3 from analysis.md
2. Clarify that VECSTORE completes when the trait is expanded and both stores implement it
3. Consumer migration (CLI/daemon/indexer) is MAPCLI

**Documents Affected:** analysis.md (lines 210-214), plan.md (lines 179-186)

### Issue 2: Missing PostgreSQL Query Functions

**Severity:** Critical
**Category:** Architecture

**Description:** The architecture assumes PostgresStore will "wrap existing queries.rs functions" for new methods. However, several proposed methods don't have corresponding functions in queries.rs:

- `search_chunks_vector()` - **MISSING** (only `search_chunks_fts` exists)
- `search_chunks_hybrid()` - **MISSING**
- `get_chunk_by_id()` - **MISSING**
- `get_file_chunks()` - **MISSING**
- `get_chunk_context()` - **MISSING** (exists in `context/assembler.rs` but uses direct queries)
- `get_repo_by_name()` - **MISSING**
- `list_repos()` - **MISSING**

**Impact:** VECSTORE-1001, VECSTORE-1002, VECSTORE-1003 will require writing new PostgreSQL query functions, not just wiring existing ones. This significantly increases scope.

**Required Action:**
1. Audit each proposed method for existing PostgreSQL implementations
2. Add tickets for writing missing queries.rs functions
3. Update estimates to reflect actual work required

**Documents Affected:** architecture.md (PostgresStore Implementation Pattern section)

## High-Risk Areas (Warnings)

### Risk 1: Context Module Complexity

**Risk Level:** High
**Category:** Technical

**Description:** The `context/` module (`context/assembler.rs`, `context/relationships.rs`, `context/graph.rs`) contains sophisticated PostgreSQL queries with complex joins, CTEs, and relationship traversals. These use `Client` directly (10+ functions).

**Probability:** High
**Impact:** Medium

**Mitigation:**
- Consider whether context methods should be in VectorStore trait at all
- They may belong in a separate `ContextStore` trait or remain as higher-level functions that use VectorStore
- Document decision in architecture.md ADR section

### Risk 2: Parity Test False Confidence

**Risk Level:** Medium
**Category:** Testing

**Description:** Quality strategy proposes parity tests that assert `assert!((pg.score - sq.score).abs() < 0.1)`. However:
- PostgreSQL FTS uses `ts_rank_cd` with tsvector
- SQLite FTS5 uses BM25
- These ranking algorithms produce fundamentally different scores

**Probability:** High
**Impact:** Medium

**Mitigation:**
- Adjust parity tests to compare ranking order, not absolute scores
- Document known ranking differences
- Use relaxed tolerance or rank-based comparison

### Risk 3: SQLite sqlite-vec Dependency

**Risk Level:** Medium
**Category:** Technical

**Description:** Vector search in SQLite requires sqlite-vec extension which is statically linked. The trait methods assume graceful degradation, but test coverage for the "no sqlite-vec" path is unclear.

**Probability:** Low
**Impact:** High

**Mitigation:**
- Add explicit tests for graceful degradation path
- Document in CLAUDE.md that vector search requires sqlite-vec
- Consider `Option<Vec<SearchHit>>` return type for optional capabilities

## Gaps & Ambiguities

### Requirements Gaps

1. **Embedding Dimension Handling**
   - Current trait has `upsert_embeddings(..., dimension: usize)`
   - SQLite is hardcoded for 1536-dim
   - PostgreSQL supports 768 (Ollama) and 1536 (OpenAI)
   - **Gap:** New search methods need dimension-aware logic

2. **Transaction Semantics**
   - Analysis doesn't specify if `delete_worktree_data()` should be transactional
   - PostgreSQL cleanup uses explicit transactions
   - SQLite uses auto-commit by default
   - **Gap:** Clarify transaction requirements in architecture

3. **Debug Mode Behavior**
   - `search_chunks_*(..., debug: bool)` returns `SearchHit` with optional debug fields
   - What debug info should vector/hybrid search return?
   - **Gap:** Define debug output for new search methods

### Technical Gaps

1. **`ChunkFull` vs `ChunkRecord` Types**
   - Architecture proposes `ChunkFull` type for context retrieval
   - `ChunkRecord` already exists for insertion
   - **Gap:** Define relationship between these types, avoid duplication

2. **`ChunkSummary` Definition Missing**
   - Referenced in architecture but not defined
   - Different from `ChunkRecord` how?
   - **Gap:** Define type with specific fields

### Process Gaps

1. **PostgreSQL Test Database Setup**
   - Quality strategy mentions `MAPROOM_DATABASE_URL_TEST` env var
   - No documentation on how to set this up
   - **Gap:** Add test database setup instructions

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **Context Assembly Scope**
   - Plan proposes `get_chunk_context(chunk_id, surrounding)` as single method
   - Existing context assembly is 3 files, 400+ lines, with strategy pattern
   - Risk: Simplifying to one method may lose important functionality
   - **Recommendation:** Start with simpler methods, defer complex context assembly

2. **Cleanup Module Integration**
   - Cleanup module has 3 types, 5 functions, 200+ lines
   - Integrating into trait adds significant scope
   - **Recommendation:** Keep cleanup as VECSTORE-1006, but size appropriately

### Feasibility Challenges

1. **PostgreSQL Function Writing**
   - Missing ~7 query functions in queries.rs
   - Each requires PostgreSQL-specific SQL (joins, CTEs, tsvector)
   - Feasible but underestimated in current plan

2. **Cross-Backend Parity**
   - SQLite and PostgreSQL have different:
     - FTS ranking algorithms
     - Vector distance functions
     - Index structures
   - Achieving "equivalent results" requires careful tolerance definition

## Alignment Assessment

### MVP Discipline
**Rating:** Strong

- Clear scope boundaries
- Explicit "out of scope" list
- Phased approach with independent value per phase
- No unnecessary abstractions (single trait, not multiple)

### Pragmatism Score
**Rating:** Strong

- Uses existing sqlite/* module implementations
- Wraps rather than rewrites
- Reasonable test strategy (contract + parity, not 100% coverage)
- No new dependencies

### Agent Compatibility
**Rating:** Adequate

- Tickets are well-sized (2-8 hours each)
- Clear acceptance criteria
- Single agent type (rust-indexer-engineer) for consistency
- **Concern:** Some tickets may need splitting if PostgreSQL functions are missing

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] **Dependencies on existing systems documented** (missing: queries.rs function audit)

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [ ] **Integration points are well-defined** (partial: context module unclear)
- [x] Performance requirements are clear
- [x] Error handling is specified
- [x] Existing tools/libraries identified for reuse
- [ ] **No unnecessary duplication** (ChunkFull vs ChunkRecord needs clarification)

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [x] Rollback plan exists (trait expansion is additive)
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified (sqlite/* modules)
- [ ] **Integration points with existing systems mapped** (context module integration unclear)
- [x] No reinvention of available functionality
- [x] Proper integration methods chosen (trait wrapping)
- [x] Component boundaries respected
- [x] Public interfaces used
- [x] Appropriate coupling levels maintained

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Audit queries.rs for each proposed method**
   - List which methods exist vs need creation
   - Update estimates accordingly
   - Consider adding a "Phase 0" for missing PostgreSQL functions

2. **Clarify success criteria**
   - Remove "CLI/daemon/indexer use trait" from VECSTORE success criteria
   - Add to MAPCLI prerequisites instead

3. **Define ChunkFull and ChunkSummary types**
   - Add type definitions to architecture.md
   - Clarify relationship to ChunkRecord

### Phase 1 Adjustments

- **VECSTORE-1001** (Vector Search): Add sub-task for writing `search_chunks_vector` in queries.rs if missing
- **VECSTORE-1002** (Hybrid Search): Add sub-task for writing `search_chunks_hybrid` in queries.rs if missing

### Risk Mitigations

1. **Parity Testing:** Change score comparison from absolute to rank-based
2. **Context Complexity:** Consider deferring `get_chunk_context` to later phase or separate project
3. **Missing Functions:** Create ticket inventory spreadsheet mapping methods to existing implementations

### Documentation Updates

- **analysis.md**: Remove conflicting success criteria #3
- **architecture.md**: Add ADR for context module approach, define ChunkFull/ChunkSummary
- **plan.md**: Add "Phase 0" for PostgreSQL function prerequisites if needed

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. Success criteria conflict with out-of-scope definition
2. Missing PostgreSQL query functions underestimate scope
3. Context module integration complexity

### Recommended Path Forward

**REVISE THEN PROCEED:** Address the conflicting success criteria and audit queries.rs before creating tickets. The core approach is sound, but the gap between "wrapper" work and "new function" work needs clarification.

### Success Probability
Given current state: **70%**
After recommended changes: **90%**

### Final Notes

This is a well-conceived project with clear boundaries. The decomposition from the larger SQLITE-INTEGRATION proposal was done correctly—VECSTORE passes the Project Boundary Framework criteria for Interface Stability, Context Coherence, and Testable Completion.

The main risk is underestimating the PostgreSQL work. The SQLite side is well-prepared (existing vector.rs, hybrid.rs, fts.rs implementations), but the PostgreSQL side may require more query authoring than the plan suggests.

Once the success criteria are clarified and the queries.rs audit is complete, this project is ready for ticket creation.
