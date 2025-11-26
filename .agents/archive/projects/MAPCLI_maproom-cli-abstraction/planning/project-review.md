# Project Review: MAPCLI - Maproom CLI Abstraction

**Review Date:** 2025-11-26
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

MAPCLI is a well-scoped project that correctly builds on the completed VECSTORE foundation. The planning documents demonstrate good understanding of the problem space and propose a sensible architectural approach using the existing `get_store()` factory pattern.

However, the review identifies several critical gaps that need addressing before ticket creation:

1. **Indexer Coupling Underestimated**: The indexer module (`scan_worktree`, `upsert_files`, `watch_worktree`) is tightly coupled to `tokio_postgres::Client` and will require significant work to abstract - more than the "hybrid approach" described.

2. **Status Module Not Addressed**: `status.rs` creates its own PostgreSQL connection directly via `tokio_postgres::connect()`, completely bypassing any abstraction. This is not mentioned in the analysis.

3. **Missing VectorStore Methods**: Several operations needed by CLI commands aren't in the trait (status queries, chunk counting by worktree).

The project is fundamentally sound but needs scope refinement before tickets are created.

## Critical Issues (Blockers)

### Issue 1: Status Module Direct PostgreSQL Coupling

**Severity:** Critical
**Category:** Architecture/Integration

**Description:** The `status.rs` module (lines 28-34) creates its own PostgreSQL connection:
```rust
let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls)
    .await
```

This module is completely independent of any abstraction layer and uses PostgreSQL-specific queries with JSONB operators (`@>`) that won't work with SQLite.

**Impact:** The `status` command will fail completely with SQLite. The plan mentions "Adapt to work with trait" but doesn't acknowledge this module creates its own connection outside the factory pattern.

**Required Action:**
1. Add status-related methods to VectorStore trait (or use existing list_repos/list_worktrees)
2. Refactor `status.rs` to accept a store parameter or use factory
3. Rewrite queries to be backend-agnostic

**Documents Affected:** analysis.md (missing), architecture.md (incomplete), plan.md (MAPCLI-1004 scope)

---

### Issue 2: Indexer Functions Require PostgreSQL Client

**Severity:** Critical
**Category:** Architecture

**Description:** The indexer functions have explicit `tokio_postgres::Client` parameters:
- `scan_worktree(client: &Client, ...)` (line 459)
- `upsert_files(client: &Client, ...)` (line 717)
- `watch_worktree(_client: &Client, ...)` (line 1076)
- `scan_worktree_parallel(pool: &PgPool, ...)` (line 259)

The plan's "hybrid approach" (architecture.md lines 127-142) suggests:
```rust
match store.backend_type() {
    BackendType::PostgreSQL => indexer::scan_worktree(&pg_client, ...),
    BackendType::SQLite => indexer::scan_worktree_sqlite(&sqlite_conn, ...),
}
```

But `indexer::scan_worktree_sqlite` doesn't exist and would need to be created - this is significant new work not accounted for in the tickets.

**Impact:** Cannot call indexer functions with SQLite backend. The scan/upsert/watch commands won't work without either:
1. Creating SQLite-specific indexer functions
2. Refactoring indexer to accept VectorStore trait

**Required Action:**
1. Decide on approach: duplicate indexer functions or refactor to trait
2. If duplicating: create MAPCLI-1001a for SQLite indexer functions
3. If refactoring: significantly expand MAPCLI-1001 scope or create new ticket
4. Update architecture.md with concrete plan, not placeholder

**Documents Affected:** analysis.md, architecture.md, plan.md

---

### Issue 3: Missing BackendType and backend_type() Method

**Severity:** High (Blocking MAPCLI-1001)
**Category:** Requirements Gap

**Description:** The architecture and plan reference `store.backend_type()` and `BackendType` enum, but these don't exist yet. The VectorStore trait (db/mod.rs) doesn't have a `backend_type()` method.

**Impact:** The proposed conditional logic won't compile until this is added.

**Required Action:**
1. Add `BackendType` enum to db/mod.rs or factory.rs
2. Add `fn backend_type(&self) -> BackendType` to VectorStore trait
3. Implement in both PostgresStore and SqliteStore
4. Clarify in MAPCLI-1001 that this is the first task

**Documents Affected:** plan.md (MAPCLI-1001 should explicitly list this)

## High-Risk Areas (Warnings)

### Risk 1: Daemon execute_search Uses Direct Queries

**Risk Level:** High
**Category:** Integration

**Description:** `daemon/mod.rs` execute_search() (lines 108-298) uses direct SQL queries for repo/worktree resolution:
```rust
let repo_row = client.query_one(
    "SELECT id FROM maproom.repos WHERE name = $1",
    &[&params.repo],
)
```

And for fetching chunk details:
```rust
let chunk_row = client.query_opt(
    r#"SELECT c.start_line, c.end_line, ... FROM maproom.chunks c ..."#,
)
```

**Probability:** High - this will definitely break with SQLite
**Impact:** Medium - daemon search completely broken for SQLite

**Mitigation:**
- Use `store.get_repo_by_name()` instead of raw queries
- Create helper method or use existing trait methods for chunk details
- MAPCLI-1002 scope needs expansion

### Risk 2: Search Modes in Daemon

**Risk Level:** Medium
**Category:** Technical

**Description:** Daemon supports FTS, vector, and hybrid search modes. The VectorStore trait has these methods, but the daemon also uses `FTSExecutor` and `VectorExecutor` directly (lines 152-249). These executors may have PostgreSQL-specific implementations.

**Probability:** Medium
**Impact:** Medium - some search modes may not work

**Mitigation:** Verify executors work with trait methods or refactor daemon to only use trait search methods.

### Risk 3: Auto-Generate Embeddings Uses Raw Queries

**Risk Level:** Medium
**Category:** Technical

**Description:** `auto_generate_embeddings()` in main.rs (lines 386-395) uses direct client queries:
```rust
let client = crewchief_maproom::db::connect().await?;
let count_row = client.query_one(
    "SELECT COUNT(*) FROM maproom.chunks WHERE code_embedding IS NULL...",
)
```

**Probability:** High - will break with SQLite
**Impact:** Low - embedding generation is optional post-scan step

**Mitigation:**
- Add `count_chunks_missing_embeddings()` to VectorStore trait, OR
- Document that embedding generation is PostgreSQL-only for now

## Gaps & Ambiguities

### Requirements Gaps

1. **Status Query Abstraction**: No VectorStore method for status queries. Need:
   - `get_worktree_chunk_count(worktree_id: i64) -> i64`
   - Or refactor status to use existing `list_repos()` + `list_worktrees()` + iterate

2. **Embedding Count Query**: No trait method for counting chunks without embeddings

3. **Default SQLite Path**: Plan says `~/.maproom/maproom.db` but factory.rs defaults to `sqlite://maproom.db` (relative path)

### Technical Gaps

1. **SQLite Indexer Implementation**: Completely missing from codebase. Need to either:
   - Create `indexer_sqlite.rs` with equivalent functions
   - Refactor indexer to accept trait (significant work)

2. **Chunk Detail Fetching**: No trait method to get chunk details (start_line, end_line, symbol_name, kind, file_path) needed for search result formatting

### Process Gaps

1. **Testing Strategy Unclear for Indexer**: How to test scan command if indexer doesn't support SQLite?
2. **Order of Operations**: Should backend_type be added in MAPCLI-1001 or as prerequisite?

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **Indexer Refactoring**: If full indexer abstraction is pursued, this becomes a much larger project
2. **Status Module Rewrite**: Complete rewrite needed, not just "adapt"
3. **Embedding Pipeline**: May need its own abstraction

### Feasibility Challenges

1. **Single-Ticket Indexer Solution**: MAPCLI-1001 can't realistically add BackendType, refactor main.rs, AND create SQLite indexer functions
2. **SQLite Performance**: Sequential-only indexing may be very slow for large repos

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate

The project correctly identifies out-of-scope items (parallel scan, migration abstraction). However, some claimed "in scope" items (scan, upsert commands) actually depend on the out-of-scope indexer refactoring.

**Recommendation:** Be more honest about what can be achieved. Perhaps MVP is: search/status/cleanup work with SQLite, scan/upsert deferred.

### Pragmatism Score
**Rating:** Strong

- Uses existing `get_store()` factory pattern ✓
- Disables parallel mode for SQLite ✓
- Accepts SQLite auto-migration behavior ✓
- Doesn't over-abstract the indexer ✓

### Agent Compatibility
**Rating:** Adequate

- 5 tickets is reasonable
- rust-indexer-engineer appropriate for most work
- However, ticket scopes may be 8+ hours given hidden complexity

### Codebase Integration
**Rating:** Weak

- Doesn't acknowledge `status.rs` has own connection
- Underestimates indexer coupling
- Missing trait methods not identified

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [ ] **Plan is detailed enough to create tickets from** (gaps identified)
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] **Dependencies on existing systems documented** (missing status.rs, indexer coupling)

### Technical
- [x] Technology choices are appropriate
- [ ] **Dependencies are identified and available** (indexer functions for SQLite missing)
- [ ] **Integration points are well-defined** (status.rs missed)
- [x] Performance requirements are clear
- [x] Error handling is specified
- [ ] **Existing tools/libraries identified for reuse** (partial)
- [ ] **No unnecessary duplication of functionality** (may need duplicate indexer)

### Process
- [x] Agent assignments are appropriate
- [ ] **Task boundaries are clear** (scope of 1001/1004 unclear given gaps)
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [x] Rollback plan exists
- [ ] **Integration with existing workflows considered** (status.rs, embedding generation)

### Risk
- [x] Major risks are identified
- [ ] **Mitigation strategies exist** (indexer mitigation unclear)
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [ ] **Failure modes are understood** (what if indexer can't be abstracted?)

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Add status.rs to analysis.md**
   - Document that it creates its own PostgreSQL connection
   - Decide: refactor to use store or mark status as PostgreSQL-only for MVP

2. **Clarify Indexer Strategy**
   - Option A: Create duplicate SQLite indexer functions (more work, cleaner separation)
   - Option B: Defer scan/upsert/watch to SQLite until indexer refactored (smaller MVP)
   - Option C: Have indexer accept `&dyn VectorStore` (biggest refactor)
   - **Recommend Option B for MVP**

3. **Add BackendType as Prerequisite**
   - Create MAPCLI-1000 or expand factory.rs documentation
   - Make clear this is first step before any other work

### Phase 1 Adjustments

**If pursuing full scope:**
- Split MAPCLI-1001 into:
  - MAPCLI-1001a: Add BackendType enum and trait method
  - MAPCLI-1001b: Update main.rs db::connect() calls
  - MAPCLI-1001c: Create SQLite indexer functions (if Option A)

**If pursuing reduced MVP:**
- Remove scan/upsert/watch from scope
- Focus on: search, status, cleanup, daemon
- Add ticket for "SQLite indexer" as follow-up project

### Risk Mitigations

1. **Indexer Risk**: Accept that scan/upsert/watch may be PostgreSQL-only initially
2. **Status Risk**: Add status refactoring to MAPCLI-1004 explicitly
3. **Testing Risk**: Create integration tests that mock indexer for SQLite path

### Documentation Updates

- **analysis.md**:
  - Add section on status.rs connection pattern
  - Add section on indexer function signatures
  - Update "Commands Requiring Changes" table with accurate effort

- **architecture.md**:
  - Replace placeholder "hybrid approach" with concrete decision
  - Add status.rs to "Component Changes" section
  - Add chunk detail fetching to daemon changes

- **plan.md**:
  - Add MAPCLI-1000 for BackendType (or merge into 1001)
  - Expand MAPCLI-1004 to include status.rs refactor
  - Consider splitting MAPCLI-1001 if indexer abstraction included

## Review Conclusion

### Readiness Assessment

**Can this project succeed as currently defined?** No without changes

**Primary concerns:**
1. Indexer functions require PostgreSQL Client - no SQLite path exists
2. Status module bypasses all abstraction - will break completely
3. BackendType enum doesn't exist yet - blocking first ticket

### Recommended Path Forward

**REVISE THEN PROCEED:** Address critical issues before starting execution.

Specific revision needed:
1. Decide indexer strategy and update plan accordingly
2. Add status.rs to scope
3. Ensure BackendType is clearly first deliverable
4. Consider reduced MVP scope that defers indexer-dependent commands

### Success Probability

Given current state: **55%**
After recommended changes: **85%**

### Final Notes

The project's foundation is solid - using existing `get_store()` factory is the right approach. The VECSTORE work provides a good abstraction layer. The main issues are:

1. **Incomplete inventory** of PostgreSQL dependencies in the codebase
2. **Underestimated complexity** of the indexer coupling
3. **Missing prerequisite** (BackendType) that should be first ticket

These are all fixable with documentation updates and scope adjustment. The project should proceed after revisions, not be abandoned.

**Suggested MVP Scope Reduction:**
If timeline is tight, consider:
- ✅ daemon + search + status + cleanup work with SQLite
- ⏸️ scan/upsert/watch deferred to "MAPCLI Phase 2" when indexer abstracted

This delivers value (zero-config semantic search) without the indexer complexity.
