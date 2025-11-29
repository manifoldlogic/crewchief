# Project Review: CTXCLI (Context CLI Integration)

**Review Date:** 2025-11-28
**Project Status:** Proceed with Caution
**Overall Risk:** Medium

## Executive Summary

This project addresses a legitimate integration gap: the Rust context assembler (completed in SQLIMPL Phase 4) is fully implemented but not exposed to the MCP server. The proposed solution correctly follows the established `search` integration pattern (daemon JSON-RPC method + MCP tool update), which is the right architectural approach.

The planning documents are well-structured and demonstrate good understanding of the existing codebase. However, there are several issues that need attention before ticket creation: a dependency ordering concern with DaemonState initialization, potential schema synchronization gaps, and some missing detail around test infrastructure.

**Recommendation:** Revise the minor issues noted below, then proceed with ticket creation. The project is architecturally sound and follows established patterns.

## Critical Issues (Blockers)

### Issue 1: DaemonState Initialization Order

**Severity:** High
**Category:** Architecture
**Description:** The plan proposes adding `BasicContextAssembler` to `DaemonState` in CTXCLI-1003, but CTXCLI-1002 (daemon context handler) already depends on having the assembler available. The current `DaemonState` struct:

```rust
struct DaemonState {
    store: Arc<SqliteStore>,
    embedding_service: EmbeddingService,
}
```

The handler code in the architecture doc shows:
```rust
let assembler = BasicContextAssembler::new(
    state.store.clone(),
    CacheConfig::default(),
);
```

This creates a *new* assembler on every request, defeating the caching purpose.

**Impact:** Context cache won't persist across requests, negating one of the key benefits of the daemon approach.

**Required Action:**
1. Merge CTXCLI-1002 and CTXCLI-1003 into a single ticket, OR
2. Re-order so CTXCLI-1003 comes before CTXCLI-1002 (add assembler to state first, then add handler)
3. Update architecture.md to show assembler in DaemonState from the start

**Documents Affected:** plan.md, architecture.md

---

## Reinvention & Duplication Analysis

### Unnecessary Rebuilds
**None identified.** The project correctly leverages:
- Existing `BasicContextAssembler` from SQLIMPL Phase 4
- Existing `DaemonClient` TypeScript library
- Existing daemon pattern from search integration
- Existing `getDaemonClient()` singleton pattern

### Boundary Violations
**None identified.** The integration approach is correct:
- MCP tool will use daemon client (JSON-RPC) - proper boundary
- Daemon will call Rust context module - internal to same binary
- No direct PostgreSQL access in TypeScript context tool (removed)

### Missed Reuse Opportunities

**Potential Issue: Search tool's daemon integration pattern**

The `packages/maproom-mcp/src/tools/search.ts` shows the full pattern for daemon integration. This should be explicitly referenced as a template for CTXCLI-3002.

**Recommendation:** Add to CTXCLI-3002 description:
> "Follow the pattern established in `search.ts` for daemon client integration, including error handling for `DaemonStartError`, `DaemonTimeoutError`, and `RpcError`."

### Pattern Violations
**None identified.** The project correctly follows:
- Clap argument parsing pattern for CLI commands
- JSON-RPC daemon pattern with types in `types.rs`
- MCP tool validation with Zod schemas
- Singleton daemon client pattern

## High-Risk Areas (Warnings)

### Risk 1: Schema Synchronization

**Risk Level:** High
**Category:** Technical
**Description:** The Rust `ExpandOptions` has React-specific fields (`routes`, `hooks`, `jsx_parents`, `jsx_children`) but the current MCP `ExpandOptionsSchema` (in `context_schema.ts`) only has base fields. The plan mentions updating this in CTXCLI-3001, but there's no mention of keeping these schemas in sync long-term.

**Probability:** High
**Impact:** Medium

**Mitigation:**
- Add acceptance criterion to CTXCLI-3001: "Schema matches Rust ExpandOptions exactly"
- Consider generating TypeScript types from Rust (deferred, not for this project)
- Document the schema mapping in architecture.md

### Risk 2: ContextBundle Schema Mismatch

**Risk Level:** Medium
**Category:** Technical
**Description:** The Rust `ContextBundle` (from `types.rs`) has a different structure than the MCP tool's response format. The plan doesn't detail the mapping:

**Rust ContextBundle:**
```rust
pub struct ContextBundle {
    pub items: Vec<ContextItem>,
    pub total_tokens: usize,
    pub truncated: bool,
}
```

**MCP ContextBundle (context.ts):**
```typescript
interface ContextBundle {
  items: ContextItem[]
  total_tokens: number
  budget_tokens: number      // Not in Rust
  budget_remaining: number   // Not in Rust
  truncated: boolean
  metadata: { ... }          // Not in Rust
  warnings?: string[]        // Not in Rust
}
```

**Probability:** Medium
**Impact:** Medium - MCP response format change could break clients

**Mitigation:**
1. Update Rust `ContextBundle` to include `budget_tokens` and `budget_remaining` (calculated fields)
2. OR add TypeScript mapping layer in CTXCLI-3002 to compute missing fields
3. Add explicit acceptance criterion about response format compatibility

### Risk 3: CLI Command Not Essential for MVP

**Risk Level:** Low
**Category:** Scope
**Description:** Phase 2 (CLI context command) is nice-to-have but not required for the primary goal of MCP integration. The CLI is primarily useful for debugging and testing, not for production use.

**Probability:** Low
**Impact:** Low

**Mitigation:** Consider making Phase 2 optional or parallel with Phase 3. The daemon method (Phase 1) + MCP integration (Phase 3) can be completed independently.

## Gaps & Ambiguities

### Requirements Gaps

1. **Missing: Error code mapping** - What error codes should the daemon return for context-specific errors? The architecture mentions -32000 (chunk not found), -32001 (file not found), -32002 (budget exceeded), but these aren't in the JSON-RPC standard. Need to document these clearly.

2. **Missing: Worktree parameter** - The Rust `BasicContextAssembler` needs chunk_id to find the worktree, but what if the chunk doesn't exist? The error handling path isn't fully specified.

### Technical Gaps

1. **Test database fixture** - Quality strategy mentions `tests/fixtures/context_test.sql` but doesn't specify who creates it or which ticket owns it. Should be explicitly assigned to CTXCLI-4001.

2. **Daemon client method signature** - The `DaemonClient` class needs a `.context()` method added, but this isn't mentioned in any ticket. Currently only has `.search()` and `.ping()`.

**Required Action:** Add to CTXCLI-3002 or create new ticket for adding `DaemonClient.context()` method to `packages/daemon-client/`.

### Process Gaps

1. **No performance validation ticket** - Quality strategy mentions "Context assembly < 100ms" but no ticket verifies this. Consider adding performance test to CTXCLI-4003.

## Scope & Feasibility Concerns

### Scope Creep Indicators

1. **Human-readable CLI output (CTXCLI-2003)** - This is polish, not MVP. Could be deferred or simplified. The `--json` output covers the essential use case.

2. **React-specific expand options** - Adding `hooks`, `jsx_parents`, `jsx_children` to MCP schema is scope expansion beyond basic functionality. Consider deferring to a follow-up ticket if time-constrained.

### Feasibility Challenges

**None significant.** The implementation is straightforward:
- Adding a method to existing daemon (proven pattern)
- Updating MCP tool to use daemon (proven pattern)
- All underlying components already exist

## Alignment Assessment

### MVP Discipline
**Rating:** Adequate

The project correctly focuses on the integration layer without rebuilding underlying components. However, Phase 2 (CLI command) and pretty-printing (CTXCLI-2003) could be deferred for faster delivery of the core MCP integration.

**Recommendation:** Consider making Phase 2 optional or executing Phase 1 → Phase 3 → Phase 4 → Phase 2.

### Pragmatism Score
**Rating:** Strong

- Leverages existing daemon pattern
- Reuses existing assembler, cache, strategies
- No new dependencies required
- Follows established code conventions

### Agent Compatibility
**Rating:** Strong

- Tickets are well-scoped (2-4 hour estimates)
- Clear file locations specified
- Acceptance criteria are measurable
- Dependencies are explicit

### Codebase Integration
**Rating:** Strong

- Correctly identifies existing daemon singleton pattern
- Uses existing DaemonClient library
- Follows established MCP tool patterns
- No reinvention of existing functionality

### Separation of Concerns
**Rating:** Strong

- MCP tool → daemon client → Rust daemon (proper layers)
- JSON-RPC boundary between TypeScript and Rust
- Context assembly encapsulated in Rust module
- Clear interface contracts

## Execution Readiness Checklist

### Documentation
- [x] Requirements are specific and measurable
- [x] Architecture decisions are clear and justified
- [x] Plan has concrete milestones and deliverables
- [x] Plan is detailed enough to create tickets from
- [x] Test strategy is defined and pragmatic
- [ ] Security concerns are addressed (no security-review.md)
- [x] Dependencies on existing systems documented

### Technical
- [x] Technology choices are appropriate
- [x] Dependencies are identified and available
- [x] Integration points are well-defined
- [ ] Performance requirements are clear (need validation ticket)
- [x] Error handling is specified
- [x] Existing tools/libraries identified for reuse
- [x] No unnecessary duplication of functionality

### Process
- [x] Agent assignments are appropriate (or determinable)
- [x] Task boundaries are clear (or can be derived)
- [x] Verification criteria are explicit (or definable)
- [x] Handoffs are defined
- [ ] Rollback plan exists (not documented)
- [x] Integration with existing workflows considered

### Integration & Reuse
- [x] Existing solutions evaluated before building new
- [x] Current patterns and conventions followed
- [x] Reusable components identified
- [x] Integration points with existing systems mapped
- [x] No reinvention of available functionality
- [x] Proper integration methods chosen:
  - [x] APIs for service communication
  - [x] JSON-RPC for daemon communication
- [x] Component boundaries respected
- [x] Public interfaces used (not internals)
- [x] Appropriate coupling levels maintained

### Risk
- [x] Major risks are identified
- [ ] Mitigation strategies exist (partial)
- [x] Dependencies have fallbacks
- [x] Critical path is protected
- [x] Failure modes are understood

## Recommendations

### Immediate Actions (Before Creating Tickets)

1. **Merge or re-order CTXCLI-1002 and CTXCLI-1003** - The daemon handler needs the assembler in state to enable caching. Either combine these tickets or reverse their order.

2. **Add DaemonClient.context() method to scope** - The TypeScript daemon client needs a `.context()` method. Either add to CTXCLI-3002 or create a new Phase 3 ticket.

3. **Clarify ContextBundle response mapping** - Document how Rust ContextBundle maps to MCP response format. Add `budget_tokens` and `budget_remaining` calculation.

### Phase 1 Adjustments

- Consider adding assembler to `DaemonState` in CTXCLI-1001 alongside params types, simplifying the dependency chain.

### Risk Mitigations

1. **Schema sync:** Add cross-reference comments in both Rust and TypeScript schema files pointing to each other.

2. **Performance:** Add performance assertion to CTXCLI-4003 E2E tests (context assembly < 200ms).

### Documentation Updates

- **architecture.md:** Show `BasicContextAssembler` in `DaemonState` struct diagram
- **quality-strategy.md:** Assign test fixture creation to specific ticket
- **plan.md:** Add note about `DaemonClient.context()` method

## Review Conclusion

### Readiness Assessment
**Can this project succeed as currently defined?** Yes with caveats

**Primary concerns:**
1. DaemonState initialization order will break caching if not fixed
2. Missing DaemonClient.context() method in scope
3. ContextBundle schema mapping not detailed

### Recommended Path Forward

**REVISE THEN PROCEED:** Address the three issues above before creating tickets. The project is architecturally sound and follows established patterns well.

### Success Probability
Given current state: **75%**
After recommended changes: **90%**

### Final Notes

This is a well-designed integration project that correctly leverages existing infrastructure. The planning documents show good understanding of the codebase and appropriate use of the daemon pattern. The issues identified are all addressable with minor plan updates - no fundamental redesign required.

The project fills a real gap (MCP context tool using legacy PostgreSQL while search uses daemon) and will result in a cleaner, more consistent architecture. Recommend proceeding after addressing the noted issues.
