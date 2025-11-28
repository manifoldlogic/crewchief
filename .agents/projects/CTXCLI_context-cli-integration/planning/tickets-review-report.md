# CTXCLI Tickets Review Report

## Executive Summary

**Total Tickets Reviewed:** 12
**Overall Assessment:** Ready with minor corrections
**Critical Issues:** 1
**Warnings:** 5
**Recommendations:** 6

The CTXCLI project tickets are well-structured and follow the architectural plan. One critical issue requires correction (missing `routes` field in schema), and several warnings need attention before execution. The project is fundamentally sound with clear dependencies and achievable scope.

---

## Critical Issues

### CRIT-1: Missing `routes` Field in ExpandOptions Schema (CTXCLI-1001, CTXCLI-3001)

**Affected Tickets:** CTXCLI-1001, CTXCLI-3001

**Problem:** The Rust `ExpandOptions` struct in `crates/maproom/src/context/types.rs` includes a `routes` field (line 102: `pub routes: bool`) which is NOT included in the ticket specifications for `ExpandConfig` (daemon types) or the MCP schema updates.

**Impact:** Schema mismatch will cause deserialization failures when MCP clients send `routes: true` or when the Rust assembler returns route-related context. This breaks the stated acceptance criteria of "Types match Rust `ExpandOptions` exactly."

**Current Rust ExpandOptions Fields (10 total):**
1. `callers`
2. `callees`
3. `tests`
4. `docs`
5. `config`
6. `max_depth`
7. `routes` ← **MISSING from tickets**
8. `hooks`
9. `jsx_parents`
10. `jsx_children`

**Required Action:**
1. Update CTXCLI-1001 to add `routes: bool` to `ExpandConfig` struct
2. Update CTXCLI-3001 to add `routes: z.boolean().default(false)` to Zod schema
3. Update both tickets' acceptance criteria from "all 9 fields" to "all 10 fields"

**Priority:** Must fix before execution

---

## Warnings

### WARN-1: Context Assembler `get_chunk_metadata` Not Implemented (CTXCLI-1002)

**Affected Ticket:** CTXCLI-1002

**Concern:** The `BasicContextAssembler::get_chunk_metadata()` method in `assembler.rs:138-141` currently returns `anyhow::bail!("get_chunk_metadata not yet implemented for SQLite")`. The ticket assumes this method works.

**Impact:** CTXCLI-1002 cannot succeed until this method is implemented. The ticket should either:
1. Include implementing `get_chunk_metadata` as a prerequisite task
2. Reference an existing implementation ticket for this method
3. Explicitly note this dependency

**Potential Fix:** The TODO comment references "IDXABS-4001" but there's no such ticket in the current project. Consider:
- Adding `get_chunk_metadata` implementation to CTXCLI-1002 tasks
- Or creating a prerequisite ticket for this implementation

**Suggested Remediation:** Add to CTXCLI-1002 implementation notes:
```
**Prerequisite**: Implement `get_chunk_metadata()` method in `BasicContextAssembler`
using SqliteStore queries before the daemon handler can work.
```

### WARN-2: Existing MCP Context Tool Uses PostgreSQL (CTXCLI-3002)

**Affected Ticket:** CTXCLI-3002

**Concern:** The current `context.ts` implementation imports `Client` from `pg` and queries `maproom.chunks`, `maproom.files`, `maproom.worktrees`, and `maproom.relationships` tables. The ticket correctly identifies PostgreSQL removal, but doesn't fully account for:

1. The `relationships` table that may not exist in SQLite
2. The `metadata` column structure differences
3. The `ts_doc` FTS column used for full-text search

**Impact:** May need additional work beyond just "removing pg client usage."

**Suggested Remediation:** Add to CTXCLI-3002 implementation notes:
```
**Note**: The current implementation queries PostgreSQL-specific tables
(maproom.relationships) and columns (metadata JSONB, ts_doc tsvector).
These are replaced entirely by daemon calls - no SQL migration needed,
just remove all pg-related code.
```

### WARN-3: ContextBundle Field Mismatch Between Rust and MCP (CTXCLI-3002)

**Affected Ticket:** CTXCLI-3002

**Concern:** The mapping layer in CTXCLI-3002 extracts `worktree` and `repo` from `rustBundle.items[0]?.worktree`, but the Rust `ContextItem` struct does NOT contain `worktree` or `repo` fields (see `types.rs:45-59`):

```rust
pub struct ContextItem {
    pub relpath: String,
    pub range: LineRange,
    pub role: String,
    pub reason: String,
    pub content: String,
    pub tokens: usize,
    // No worktree or repo fields!
}
```

**Impact:** The mapping layer code will fail with undefined values:
```typescript
metadata: {
  worktree: rustBundle.items[0]?.worktree ?? 'unknown', // Will always be 'unknown'
  repo: rustBundle.items[0]?.repo ?? 'unknown',         // Will always be 'unknown'
}
```

**Suggested Remediation:** Either:
1. Add `worktree` and `repo` fields to Rust `ContextItem` struct
2. Or pass these values through the daemon's `ContextBundle` response at the bundle level
3. Or derive them from the request context (chunk_id lookup)

### WARN-4: Test Fixtures Need SQLite Compatibility (CTXCLI-4001)

**Affected Ticket:** CTXCLI-4001

**Concern:** The SQL fixture example uses PostgreSQL-style syntax. SQLite uses different syntax for some operations. Example from ticket:

```sql
INSERT INTO chunk_edges (from_chunk_id, to_chunk_id, edge_type)
VALUES (2, 1, 'calls');
```

This looks correct, but need to verify:
1. Table names match SQLite schema (`chunk_edges` vs `relationships`)
2. Column names are correct for SQLite
3. Auto-increment ID handling

**Impact:** Test fixtures may fail to load in SQLite.

**Suggested Remediation:** Verify fixture SQL against actual SQLite schema in `crates/maproom/src/db/sqlite/schema.rs`.

### WARN-5: Daemon-Client Missing context() Method Type Export (CTXCLI-3002)

**Affected Ticket:** CTXCLI-3002

**Concern:** The ticket shows adding `context()` method to `DaemonClient` class but doesn't mention updating `index.ts` exports. The current `packages/daemon-client/src/index.ts` exports:

```typescript
export { DaemonClient } from './client.js'
export type { SearchParams, SearchResult } from './client.js'
```

If `ContextParams` and `RustContextBundle` types are added to `client.ts`, they need to be exported from `index.ts` as well.

**Suggested Remediation:** Add to CTXCLI-3002 acceptance criteria:
```
- [ ] `ContextParams` and `RustContextBundle` types exported from index.ts
```

---

## Recommendations

### REC-1: Consider max_depth Default Value Consistency

**Area:** CTXCLI-1001, CTXCLI-2001, CTXCLI-3001

The Rust `ExpandOptions::default()` sets `max_depth: 1` (line 119), while tickets specify default of `2`. Either is valid, but should be consistent across:
- Daemon types (`default_max_depth() -> 2`)
- CLI args (`default_value_t = 2`)
- MCP schema (`.default(2)`)

**Suggestion:** Align with Rust default of 1, or update Rust to match ticket default of 2.

### REC-2: Add getDaemonClient Import to CTXCLI-3003

**Area:** CTXCLI-3003

The ticket implementation notes show importing from `'./daemon'`:
```typescript
import { getDaemonClient } from './daemon'
```

But this file may not exist. The current codebase doesn't show a `daemon.ts` file in `packages/maproom-mcp/src/`. The daemon client singleton may need to be created.

**Suggestion:** Clarify whether `getDaemonClient()` already exists or needs to be created as part of CTXCLI-3003.

### REC-3: Add Performance Test to CTXCLI-4001

**Area:** CTXCLI-4001

The quality strategy specifies:
- Context assembly (cold) < 100ms
- Context assembly (cached) < 10ms

Consider adding acceptance criteria:
```
- [ ] Cache persistence test shows second call < 50% of first call time
```

### REC-4: Add chunk_id Type Clarification

**Area:** CTXCLI-1001, CTXCLI-2001

The CLI uses `chunk_id: i64` while daemon uses `chunk_id: String`. This is intentional for JSON compatibility, but could cause confusion.

**Suggestion:** Add explicit note in both tickets about the type difference and the parse step.

### REC-5: Document Error Code Mapping

**Area:** CTXCLI-3002

The ticket mentions error handling pattern from `search.ts` but the MCP error format uses different codes:
- Rust daemon: `-32000` (chunk not found), `-32602` (invalid params)
- MCP format: `'CHUNK_NOT_FOUND'`, `'RPC_ERROR'` strings

**Suggestion:** Add explicit mapping table in implementation notes.

### REC-6: Test Parallel Execution Order

**Area:** Phase 4 (CTXCLI-4001, CTXCLI-4002, CTXCLI-4003)

The ticket index shows these can run in parallel, but:
- CTXCLI-4002 depends on CTXCLI-2003 (CLI implementation)
- CTXCLI-4003 depends on CTXCLI-3003 (MCP integration)
- CTXCLI-4001 depends on CTXCLI-1002 (daemon implementation)

All depend on Phase 1-3 completion. True parallel execution within Phase 4 is correct.

---

## Ticket Actions Required

### Tickets to Rework

| Ticket | Required Changes |
|--------|------------------|
| CTXCLI-1001 | Add `routes: bool` field to `ExpandConfig`; update "9 fields" to "10 fields" |
| CTXCLI-1002 | Add note about `get_chunk_metadata()` prerequisite implementation |
| CTXCLI-3001 | Add `routes: z.boolean().default(false)` to schema; update field count |
| CTXCLI-3002 | Fix metadata mapping (worktree/repo extraction); add type exports to acceptance criteria |

### Tickets to Defer

None - all tickets are appropriately scoped for the current project.

### Tickets to Skip

None - all tickets are necessary for project completion.

### Tickets to Split

None - tickets are appropriately sized (5 Small, 7 Medium).

### Tickets to Merge

None - current granularity is appropriate.

---

## Integration Assessment

### Overall Integration Health: **Good**

The integration points are well-defined:

1. **Rust Daemon ↔ MCP Server:** Clear JSON-RPC interface with typed params
2. **CLI ↔ Database:** Direct SQLiteStore usage following existing patterns
3. **Test Fixtures:** Shared fixture file owned by CTXCLI-4001

### Key Integration Points

| Point | Status | Notes |
|-------|--------|-------|
| DaemonState + BasicContextAssembler | ⚠️ Needs attention | `get_chunk_metadata()` not implemented |
| daemon types → context::types | ⚠️ Missing field | Add `routes` to ExpandConfig |
| MCP schema → Rust schema | ⚠️ Missing field | Add `routes` to Zod schema |
| ContextBundle response mapping | ⚠️ Needs fix | worktree/repo not in ContextItem |
| daemon-client.context() | ✓ Clear | Follows search() pattern |
| Test fixtures | ✓ Clear | SQLite-compatible SQL |

### Risks to Existing Functionality

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Search performance regression | Low | Acceptance criteria includes "No performance regression for search" |
| Breaking MCP clients | Low | All new fields are optional with defaults |
| PostgreSQL removal issues | Medium | Thorough testing in CTXCLI-4003 |

---

## Dependency Analysis

### Dependency Chain Validation: **Valid**

```
Phase 1: CTXCLI-1001 → CTXCLI-1002
Phase 2: CTXCLI-2001 → CTXCLI-2002 → CTXCLI-2003
Phase 3: CTXCLI-3001 → CTXCLI-3002 → CTXCLI-3003
                              ↑
                        CTXCLI-1002
Phase 4: CTXCLI-4001, CTXCLI-4002, CTXCLI-4003 (parallel after Phase 3)
         CTXCLI-4004 (after all above)
```

### Problematic Dependencies

None identified. The cross-phase dependency (CTXCLI-3002 → CTXCLI-1002) is correctly documented.

### Sequencing Recommendations

Current sequencing is optimal. No changes needed.

### Parallel Execution Opportunities

Within Phase 4, the three test tickets can run in parallel:
- CTXCLI-4001 (daemon tests)
- CTXCLI-4002 (CLI tests)
- CTXCLI-4003 (MCP tests)

Each tests independent components with no shared state.

---

## Recommendations for Execution

### Suggested Ticket Execution Order

1. **Fix critical issue first:** Update CTXCLI-1001 and CTXCLI-3001 to add `routes` field
2. Execute Phase 1: CTXCLI-1001 → CTXCLI-1002
3. Execute Phase 2: CTXCLI-2001 → CTXCLI-2002 → CTXCLI-2003
4. Execute Phase 3: CTXCLI-3001 → CTXCLI-3002 → CTXCLI-3003
5. Execute Phase 4 (parallel): CTXCLI-4001, CTXCLI-4002, CTXCLI-4003
6. Execute CTXCLI-4004 (documentation)

### Risk Mitigation Strategies

1. **Before CTXCLI-1002:** Verify `get_chunk_metadata()` implementation exists or add it
2. **Before CTXCLI-3002:** Verify ContextBundle metadata field requirements
3. **During Phase 4:** Run existing test suites to catch regressions

### Key Checkpoints During Execution

| After Ticket | Checkpoint |
|--------------|------------|
| CTXCLI-1002 | Daemon responds to `crewchief-maproom serve` with `context` method |
| CTXCLI-2003 | `crewchief-maproom context --chunk-id 1` shows human-readable output |
| CTXCLI-3003 | MCP tool calls work via daemon (no pg imports in context.ts) |
| CTXCLI-4004 | All tests pass, documentation complete |

### Success Criteria for Project Completion

- [ ] `crewchief-maproom context --chunk-id <id>` returns valid bundle
- [ ] MCP `context` tool uses daemon (no PostgreSQL)
- [ ] All expand options work (callers, callees, tests, hooks, jsx, routes, etc.)
- [ ] Context assembly < 100ms (cached)
- [ ] All tests pass in CI
- [ ] CLAUDE.md files updated

---

## Summary

The CTXCLI tickets are well-designed with one critical schema issue requiring immediate correction before execution. The warnings identified are important but don't block execution if implementers are aware of them. The project is ready to proceed after the `routes` field is added to CTXCLI-1001 and CTXCLI-3001.

**Review Completed:** 2025-11-28
**Reviewer:** tickets-review agent
