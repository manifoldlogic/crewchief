# Project Review: SQLFIX - SQLite Backend Fixes

**Review Date:** 2025-11-25
**Project Status:** Proceed with Caution
**Overall Risk:** Medium
**Tickets Created:** No - Pre-ticket review

## Executive Summary

SQLFIX is a well-scoped, focused project that addresses real, verified compile-time errors in the SQLite backend. The 4 errors documented in the analysis are accurate and the proposed fixes are appropriate. The project demonstrates good MVP discipline by explicitly deferring vector search and VSCode integration.

However, the project has a significant gap: **the analysis underestimates the actual scope of work**. While the documents focus on compile fixes, the existing SQLite code has runtime issues (FTS5 query syntax, schema gaps, untested code paths) that will surface during Phase 2. Additionally, the planning documents reference a "recently fixed" VSCode extension and factory.rs changes that are actually part of uncommitted work in the current session - these need to be committed first or the project scope needs adjustment.

**Key Recommendation**: Combine tickets 1001 and 1002 into a single ticket (compile fixes are interdependent), and expand Phase 2 to explicitly address the runtime issues visible in the code.

## Critical Issues (Blockers)

### Issue 1: Pre-requisite Work Not Committed

**Severity:** Critical
**Category:** Execution
**Description:** The analysis and architecture documents reference fixes that were made earlier in this conversation session (feature-gating in `db/mod.rs`, `factory.rs` updates, VSCode `extension.ts` restoration) but these changes are not committed to version control. The project plan assumes these exist.

**Impact:** Tickets will fail verification if the baseline state doesn't match planning assumptions. The project may appear to require more or less work depending on what's committed.

**Required Action:**
1. Commit the current uncommitted fixes before starting SQLFIX tickets
2. OR update SQLFIX scope to include these as ticket 1000 (pre-requisites)

**Documents Affected:** analysis.md (current state description), architecture.md (factory.rs section)

### Issue 2: Cargo.toml Missing Chrono Feature is NOT the Root Fix

**Severity:** Critical
**Category:** Technical
**Description:** The analysis states `rusqlite` needs the `chrono` feature added. However, examining `Cargo.toml` line 89 shows:
```toml
rusqlite = { version = "0.29.0", features = ["bundled"], optional = true }
```
The `chrono` feature is indeed missing. But the fix specified (`bundled-full` or adding `chrono`) may introduce other issues. The `chrono` feature in rusqlite 0.29 requires enabling the correct sub-feature.

**Impact:** Ticket 1001 acceptance criteria may pass Cargo.toml changes but fail actual compilation.

**Required Action:**
1. Verify exact feature flag needed: `chrono` or `bundled-sqlcipher-vendored` or similar
2. Test locally before writing acceptance criteria
3. Update architecture.md with verified feature combination

**Documents Affected:** architecture.md section 3.1

---

## High-Risk Areas (Warnings)

### Risk 1: Schema Module Export vs. Module Declaration

**Risk Level:** High
**Category:** Technical
**Description:** The error `E0432: unresolved import crate::db::sqlite::schema` is documented, but the fix "add `pub mod schema;`" is incomplete. The sqlite `mod.rs` already imports from `schema::init_schema` (line 10), which means the module IS declared but not exported. The issue may be:
- Missing `pub` keyword on the module declaration
- Module file not in correct location
- Conditional compilation issue

**Probability:** Medium
**Impact:** High - This is the first error and blocks all other work
**Mitigation:** Verify the actual module declaration in sqlite/mod.rs and check for `mod schema;` vs `pub mod schema;`

### Risk 2: FTS5 Query Syntax Will Break at Runtime

**Risk Level:** High
**Category:** Technical
**Description:** Examining `sqlite/mod.rs` lines 454-459, the FTS5 query construction is:
```rust
let fts_query = query
    .split_whitespace()
    .map(|t| format!("\"{}\"*", t.replace("\"", "")))
    .collect::<Vec<_>>()
    .join(" ");
```
This generates queries like `"term1"* "term2"*` which may not be valid FTS5 syntax (the `*` is for prefix matching, but the quoting semantics differ from tsvector). This will pass compilation but fail runtime tests.

**Probability:** High
**Impact:** Medium - FTS search is a core feature
**Mitigation:** Add explicit FTS5 syntax testing to ticket 1005. Reference SQLite FTS5 documentation for correct query syntax.

### Risk 3: find_chunk_by_symbol Has Logic Bugs Beyond Move Semantics

**Risk Level:** Medium
**Category:** Technical
**Description:** The documented E0382 (use of moved value) at line 534 is real. However, examining lines 534-574, there are also logic issues:
- SQL queries use different parameter counts but all call the same `sql` variable
- Parameter binding (`params![repo_id, wid, path, symbol_name]`) doesn't match all branches
- The `relpath` variable is consumed but reused across branches

**Probability:** High
**Impact:** Medium - Will cause runtime panics
**Mitigation:** Ticket 1002 should be expanded to include logic review, not just fixing the move semantics error.

### Risk 4: CI Doesn't Test SQLite Feature

**Risk Level:** Medium
**Category:** Process
**Description:** The `.github/workflows/test.yml` workflow only tests with PostgreSQL. It builds `cargo build --release` which uses default features (postgres). Ticket 1006 is correct to address this, but until it's implemented, there's no CI safety net.

**Probability:** N/A (certainty)
**Impact:** Medium - Regression risk
**Mitigation:** Prioritize ticket 1006 to run immediately after compile fixes, not at the end of Phase 3.

---

## Gaps & Ambiguities

### Requirements Gaps

1. **ts_doc_text column missing from schema**: `ChunkRecord` has `ts_doc_text` field but `schema.rs` CREATE TABLE for chunks doesn't include it. This will cause runtime INSERT failures.
   - Impact: Blocks all chunk insertion
   - Suggested: Add to ticket 1003 scope

2. **busy_timeout PRAGMA location unclear**: Security review recommends adding `PRAGMA busy_timeout = 5000` but architecture says "add in ticket 1002". The PRAGMA should go in the connection init callback (line 43-51 of mod.rs), not in a separate migration.
   - Impact: Low - just needs clarification
   - Suggested: Clarify in ticket 1002 that this goes in `with_init` callback

3. **File permissions implementation location**: Security review says add `0600` permissions in `SqliteStore::connect()` but doesn't specify where relative to pool creation.
   - Impact: Medium - security concern
   - Suggested: Add as explicit task in ticket 1002 or create ticket 1002b for security hardening

### Technical Gaps

1. **sqlite-vec dimension validation**: Architecture mentions "may need dimension validation" for 1536-dim but doesn't specify whether sqlite-vec supports this dimension. The vendored code should be checked.
   - Impact: Medium - core functionality
   - Suggested: Add research task to Phase 0 or ticket 1001

2. **FTS5 content table relationship**: `schema.rs` lines 106-115 create an FTS5 table with `content='chunks'` but the chunks table uses different column names. FTS5 external content tables require matching column names or explicit mapping.
   - Impact: High - FTS won't work
   - Suggested: Fix in ticket 1003

### Process Gaps

1. **No rollback plan**: If SQLite feature causes regressions in main, there's no documented rollback procedure.
   - Impact: Low - feature flag provides natural rollback
   - Suggested: Add note to plan.md about feature flag as rollback mechanism

---

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **Quality strategy includes performance requirements**: The quality-strategy.md lists "Index 100 files in < 5 seconds" and "FTS search p95 < 100ms" as acceptance criteria. These are inappropriate for a "fix broken code" project - they belong in a future performance tuning project.
   - Suggested: Remove performance requirements from acceptance criteria, keep as aspirational goals only

2. **Integration tests scope too broad**: Quality strategy proposes `tests/store_parity.rs` with shared test logic across backends. This is valuable but should be a separate ticket, not bundled into 1005.
   - Suggested: Split ticket 1005 into 1005a (SQLite unit tests) and 1005b (parity test framework)

### Feasibility Challenges

1. **6 tickets may be too granular**: Tickets 1001 and 1002 are both ~30-minute tasks. The overhead of verification and commit for each exceeds the work itself.
   - Suggested: Merge 1001 + 1002 into single "Fix SQLite Compilation" ticket

2. **rust-indexer-engineer agent may not exist**: The agent assignment references "rust-indexer-engineer" but this isn't a standard agent name. Verify agent availability.
   - Suggested: Clarify which agent handles Rust database code

---

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds

None identified. The project correctly uses existing `VectorStore` trait and doesn't propose duplicating Postgres functionality.

### Missed Reuse Opportunities

1. **Existing test utilities in `/crates/maproom/tests/`**: There may be existing test helpers that could be reused for SQLite tests. Review before writing new test infrastructure.

2. **Connection URL parsing in `connection.rs`**: The `SqliteStore::connect()` manually strips `sqlite://` prefix. Check if `connection.rs` already handles this.

### Pattern Consistency

1. **Error handling inconsistency**: PostgresStore uses `anyhow::Context` consistently. SqliteStore mixes bare `?` with `.context()`. Ticket 1002 should standardize.

2. **Async pattern**: SqliteStore correctly uses `spawn_blocking` for sync rusqlite calls - this matches the documented architecture.

---

## Alignment Assessment

### MVP Discipline
**Rating:** Strong
- Project explicitly defers vector search, VSCode integration, benchmarks
- Focuses on compile → runtime → test progression
- Clear "out of scope" section in plan

### Pragmatism Score
**Rating:** Adequate
- Reasonable scope for fixing broken code
- Minor overreach in quality strategy (performance requirements)
- Could simplify ticket structure

### Agent Compatibility
**Rating:** Adequate
- Tasks are appropriately sized (2-4 hours each)
- Clear verification criteria
- Agent names need verification

### Codebase Integration
**Rating:** Strong
- Uses existing `VectorStore` trait
- Follows existing module structure
- No new dependencies beyond what's already declared

### Separation of Concerns
**Rating:** Strong
- SQLite module properly isolated
- Feature flags maintain separation
- No boundary violations

---

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] Dependencies on existing systems documented (uncommitted changes)

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [ ] Integration points are well-defined (FTS5 syntax unclear)
- [ ] Performance requirements are clear (should be removed)
- [ ] Error handling is specified (needs standardization note)
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [x] Rollback plan exists (feature flag)
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified
- [x] Integration points with existing systems mapped
- [x] No reinvention of available functionality
- [x] Component boundaries respected

### Risk
- [x] Major risks are identified
- [ ] Mitigation strategies exist (some gaps)
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [ ] Failure modes are understood (runtime errors not fully mapped)

---

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Commit or incorporate uncommitted changes**: The session has fixes to `db/mod.rs`, `factory.rs`, and `extension.ts` that must be committed before SQLFIX starts, or incorporated as ticket 1000.

2. **Verify rusqlite chrono feature**: Test locally that adding `features = ["bundled", "chrono"]` resolves the DateTime error before finalizing ticket 1001 acceptance criteria.

3. **Examine schema.rs for missing columns**: The `ts_doc_text` column and potentially others are missing. Update ticket 1003 scope to include schema alignment.

### Phase 1 Adjustments

- Merge tickets 1001 and 1002 into single "Fix SQLite Compilation" ticket
- Include schema module export fix verification
- Include busy_timeout PRAGMA addition

### Phase 2 Adjustments

- Expand ticket 1003 to include:
  - Missing `ts_doc_text` column
  - FTS5 content table column mapping
  - Schema version tracking for SQLite

- Expand ticket 1004 to include:
  - FTS5 query syntax validation
  - `find_chunk_by_symbol` logic fix

### Phase 3 Adjustments

- Move ticket 1006 (CI) earlier - run after 1002 to catch regressions sooner
- Split ticket 1005 into unit tests vs parity tests
- Remove performance requirements from acceptance criteria

### Documentation Updates

- **analysis.md**: Add section on runtime issues discovered during review
- **architecture.md**: Update section 3.1 with verified rusqlite feature set
- **quality-strategy.md**: Remove performance acceptance criteria, keep as goals
- **plan.md**: Update ticket structure per recommendations

---

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. Uncommitted prerequisite changes create baseline confusion
2. Runtime issues (FTS5 syntax, schema gaps) not fully scoped in tickets
3. Ticket granularity too fine for small fixes

### Recommended Path Forward

**REVISE THEN PROCEED:** Address the critical issue (uncommitted changes) and update ticket scope to include runtime fixes discovered during review. The project is fundamentally sound but needs minor adjustments before execution.

### Success Probability
Given current state: 70%
After recommended changes: 90%

### Final Notes

This is a well-conceived fix project with clear scope. The main risk is that the analysis focused on compile-time errors while the SQLite implementation has additional runtime issues. The phased approach (compile → runtime → test) is correct, but Phase 2 tickets need expansion to cover the full scope of "make it actually work" beyond just "make it compile."

The explicit deferral of vector search and VSCode integration demonstrates good MVP discipline. The security review is thorough and actionable. Overall, this project is ready to proceed after minor scope adjustments.
