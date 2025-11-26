# Project Review: SRCHDUP - Search Result Deduplication

**Review Date:** 2025-11-26
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

The SRCHDUP project is well-conceived with a clear problem definition and straightforward solution. The architecture correctly identifies the optimal insertion point (post-fusion deduplication) and uses appropriate existing patterns. However, there are several issues that need attention before ticket creation:

1. **MCP Integration Complexity Underestimated**: The plan assumes the MCP search tool can easily pass a `deduplicate` parameter, but the daemon client interface and Rust CLI command structure need updates in addition to the TypeScript schema.

2. **SQLite Backend Not Addressed**: The project doesn't mention SQLite support, but the search pipeline has a SQLite backend that also needs deduplication.

3. **Minor Ticket Over-Engineering**: 12 tickets for 3-5 days of work is slightly heavy; several tickets can be consolidated.

The project is fundamentally sound and should proceed after addressing the integration gaps. Success probability is high given the focused scope.

## Critical Issues (Blockers)

### Issue 1: Daemon Client SearchParams Missing Deduplication Parameter

**Severity:** Critical
**Category:** Integration
**Description:** The architecture shows adding `deduplicate` to MCP schema and passing it to the Rust daemon, but the daemon client (`packages/daemon-client/src/client.ts`) `SearchParams` interface does not include a `deduplicate` field. The daemon's JSON-RPC protocol must also be updated.

**Current SearchParams Interface:**
```typescript
export interface SearchParams {
  query: string
  repo: string
  worktree?: string
  limit?: number
  threshold?: number
  debug?: boolean
  // NO deduplicate field
}
```

**Impact:** MCP cannot pass deduplication preference to Rust daemon without updating daemon-client package.

**Required Action:**
1. Add `deduplicate?: boolean` to `SearchParams` in `daemon-client/src/client.ts`
2. Update Rust daemon's JSON-RPC handler to accept `deduplicate` param
3. Pass param through to SearchOptions in Rust

**Documents Affected:** architecture.md (Phase 3 tickets), plan.md (add daemon-client ticket)

### Issue 2: Rust CLI Command Interface Also Needs Update

**Severity:** Critical
**Category:** Architecture
**Description:** The Rust `crewchief-maproom search` command accepts parameters via CLI flags and JSON-RPC. Both need `--deduplicate`/`--no-deduplicate` flag added. The plan only mentions modifying `pipeline.rs` but the CLI command handler in `main.rs` needs updating too.

**Impact:** Cannot test deduplication via CLI without flag support.

**Required Action:**
1. Add `--deduplicate` flag to `SearchArgs` struct in `main.rs`
2. Pass flag to SearchOptions in CLI handler
3. Update JSON-RPC `search` method to accept param

**Documents Affected:** architecture.md (add CLI section), plan.md (add CLI ticket)

## High-Risk Areas (Warnings)

### Risk 1: SQLite Backend Deduplication Gap

**Risk Level:** High
**Category:** Integration
**Description:** The codebase has a SQLite backend (`crates/maproom/src/db/sqlite/`) with its own search implementation (`hybrid.rs`). The project plan only addresses PostgreSQL pipeline but SQLite users also experience duplicates.

**Probability:** High (SQLite is actively used)
**Impact:** Medium (SQLite users get inconsistent behavior)

**Mitigation:**
- Verify if SQLite search uses the same `FinalSearchResults` pipeline
- If separate, add SQLite deduplication ticket
- At minimum, document SQLite limitation

### Risk 2: Identity Key May Cause False Negatives

**Risk Level:** Medium
**Category:** Technical
**Description:** Using `(relpath, symbol_name, start_line)` as identity key may fail when:
- Same symbol in same file has slightly different line numbers across worktrees (code drift)
- Line number +/- 1 due to file header changes

**Probability:** Medium (common in active development)
**Impact:** Low (slightly reduced dedup effectiveness)

**Mitigation:**
- Consider using `(relpath, symbol_name)` for module-level chunks (kind = "module")
- Document the line-sensitivity as a known limitation
- Future: Add fuzzy line matching option

### Risk 3: Limit Interaction with Deduplication

**Risk Level:** Medium
**Category:** Technical
**Description:** If user requests `limit=10` but 50 raw results exist with 40 duplicates, should we:
- A) Deduplicate first, return 10 unique? (May return fewer if <10 unique exist)
- B) Take top 10, then deduplicate? (May return <10 results)

The plan doesn't specify this interaction.

**Probability:** High (common edge case)
**Impact:** Medium (unexpected result counts)

**Mitigation:**
- Clarify in architecture: deduplicate should happen BEFORE applying limit
- Request extra results from fusion to ensure limit can be satisfied post-dedup
- Document behavior in API

## Gaps & Ambiguities

### Requirements Gaps

1. **No specification for metadata enrichment**: The architecture suggests adding `duplicate_count` as a future enhancement but doesn't decide if we should track deduplicated count in current phase.

2. **Worktree name not available**: The analysis correctly notes worktree name isn't in `ChunkSearchResult`, but `PreferMain` strategy is listed as an option without noting it can't work yet.

### Technical Gaps

1. **Search cache invalidation**: If deduplication is enabled by default, cached search results need consideration. The cache key in `cache.rs` should probably include the `deduplicate` flag.

2. **Metrics/telemetry**: Should we track deduplication statistics? How many duplicates removed per query?

### Process Gaps

1. **MCP test database setup**: E2E tests need duplicate data in test DB. Test fixture creation isn't specified.

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **`SelectionStrategy::PreferMain`**: Listed in architecture but cannot work (worktree_name not in results). Should be removed from MVP and noted as future work only.

2. **Performance benchmarking**: Phase 4 includes benchmarks but for 3-5 days of work, manual timing verification would suffice.

### Feasibility Assessment

**Positive:**
- Core algorithm is simple HashMap grouping
- No external dependencies
- Clear integration point in pipeline

**Concern:**
- End-to-end integration touches 4 layers: Rust core → Rust CLI → daemon-client → MCP TypeScript
- Each layer needs coordinated changes

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate

The project correctly focuses on score-based selection and defers worktree-priority. However, including `SelectionStrategy::PreferMain` in the code (even as a fallback to score) is unnecessary for MVP.

**Recommendation:** Remove `PreferMain` variant entirely from Phase 1. Add it when worktree_name is available.

### Pragmatism Score
**Rating:** Strong

The approach is pragmatic:
- Post-processing deduplication (no query changes)
- Uses existing result structure
- Configurable via flag

Minor concern: 12 tickets for a ~300 line feature is slightly ceremonial.

### Agent Compatibility
**Rating:** Strong

Tasks are well-sized and have clear boundaries. Agent assignments are appropriate.

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [x] Security concerns are addressed
- [ ] Dependencies on existing systems documented (missing daemon-client)

### Technical
- [x] Technology choices are appropriate
- [ ] Dependencies are identified and available (daemon-client gap)
- [ ] Integration points are well-defined (missing CLI, daemon-client)
- [x] Performance requirements are clear
- [x] Error handling is specified
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate
- [x] Task boundaries are clear
- [x] Verification criteria are explicit
- [x] Handoffs are defined
- [x] Rollback plan exists
- [ ] Integration with existing workflows considered (daemon-client)

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified
- [ ] Integration points with existing systems mapped (incomplete)
- [x] No reinvention of available functionality
- [x] Proper integration methods chosen
- [x] Component boundaries respected
- [x] Public interfaces used (not internals)
- [x] Appropriate coupling levels maintained

### Risk
- [x] Major risks are identified
- [x] Mitigation strategies exist
- [ ] Dependencies have fallbacks (daemon-client not considered)
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Update architecture.md** to include:
   - Daemon-client `SearchParams` interface update
   - Rust CLI `--deduplicate` flag addition
   - JSON-RPC protocol update
   - Cache key considerations

2. **Update plan.md** to:
   - Add ticket for daemon-client update (Phase 3)
   - Add ticket for CLI flag (Phase 2)
   - Consolidate unit test tickets (1002 + 1003 can merge)
   - Remove or clearly mark `PreferMain` as stub-only

3. **Clarify limit interaction**: Document that deduplication happens before limit is applied.

### Phase 1 Adjustments

- Remove `SelectionStrategy::PreferMain` implementation (keep type for future)
- Combine SRCHDUP-1002 and SRCHDUP-1003 into single ticket

### Phase 2 Adjustments

- Add CLI flag ticket (SRCHDUP-2004)

### Phase 3 Adjustments

- Split MCP work into:
  - SRCHDUP-3001: Update daemon-client SearchParams
  - SRCHDUP-3002: Update Rust daemon JSON-RPC handler
  - SRCHDUP-3003: Update MCP search schema
  - SRCHDUP-3004: E2E tests

### Documentation Updates

- **architecture.md**: Add daemon-client section, CLI section, cache section
- **plan.md**: Add daemon-client ticket, CLI ticket, consolidate unit tests
- **quality-strategy.md**: Add test fixture creation details

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

The core technical approach is sound, but the integration path is incomplete. The plan focuses on Rust pipeline + MCP TypeScript but misses the daemon-client layer in between.

**Primary concerns:**
1. Daemon-client interface update not planned
2. CLI flag not planned
3. SQLite backend may need separate work

### Recommended Path Forward

**REVISE THEN PROCEED:** Address the daemon-client integration gap and CLI flag before creating tickets. These are 30-minute documentation updates, not fundamental redesigns.

### Success Probability
Given current state: 75%
After recommended changes: 90%

### Final Notes

This is a well-designed project for a real user problem. The core algorithm is trivial (HashMap grouping), and the main work is integration across layers. The review findings are mostly about completeness of the integration plan rather than any fundamental issues.

The estimate of 3-5 days is reasonable. With the recommended changes, this project should execute smoothly.

**Strong points:**
- Problem is well-understood and validated
- Solution is simple and appropriate
- Testing strategy is pragmatic
- Security assessment is thorough

**Areas for growth:**
- Integration path needs more detail
- Consider all backends (SQLite)
- Ticket granularity could be coarser
