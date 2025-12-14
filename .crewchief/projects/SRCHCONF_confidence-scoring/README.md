# Project: Confidence Scoring

**Slug:** SRCHCONF
**Status:** Planning
**Created:** 2025-12-14

## Summary

Add confidence scoring to maproom search results to help users assess result quality and reliability. Confidence signals expose transparency about search coverage, source agreement, exact matches, and score separation—enabling users to trust high-confidence results and investigate low-confidence ones.

**Key Value**: Transform search from "here are results with scores" to "here are results with confidence indicators showing why you should trust them."

## Problem Statement

Search results currently return relevance scores (FTS scores, cosine similarity, or fused scores from RRF), but users cannot determine:

1. **Result reliability** - Is this a strong match or just the best of bad options?
2. **Search coverage** - Did all search sources (FTS, vector, graph, signals) contribute?
3. **Signal agreement** - Do multiple scoring methods agree on this result?
4. **Query quality** - Was the query well-formed and likely to find good matches?

**Impact**: Users must manually verify results because they lack confidence indicators about result quality.

## Proposed Solution

Expose **confidence components** (not a single magic score) for each search result.

**MVP Scope (3 Core Signals)**:
1. **`source_count`** - Number of search executors that returned this chunk (1-4)
2. **`score_gap`** - Difference between this result's score and next result's score
3. **`is_exact_match`** - Whether query exactly matches symbol name

**Deferred to Phase 2** (validate MVP first):
- `relative_score` - Result score / top score
- `rank` - Position in result list
- Query-level confidence summary

**Design Principles**:
- **Transparency over magic** - Expose components, not opaque scores
- **In-memory computation** - Zero database queries, <5ms overhead
- **Backward compatible** - Optional field, opt-in via parameter (default: false)
- **MVP-focused** - Ship value, not ceremonies

**Initiative Alignment Note**:
This project is Phase 2 of the parent initiative (maproom-semantic-search-improvements). The initiative specified "0-100 confidence scale" and "confidence bands (HIGH/MEDIUM/LOW)", but this planning chooses component-based approach instead for better transparency and flexibility. Progressive filtering is deferred to post-MVP.

## Relevant Agents

**Planning & Review**:
- project-planner (this planning phase) ✅
- project-reviewer (review planning docs before tickets)

**Implementation** (Phase 1-3):
- rust-engineer (Rust structs and computation logic)
- typescript-engineer (TypeScript interfaces and MCP integration)
- documentation-writer (user-facing documentation)

**Quality Assurance**:
- unit-test-runner (execute Rust and TypeScript tests)
- verify-ticket (validate acceptance criteria)
- commit-ticket (create commits)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis, existing solutions, constraints, success criteria
- [architecture.md](planning/architecture.md) - Solution design, component structure, integration approach
- [plan.md](planning/plan.md) - 3-phase execution plan with agent assignments
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach focused on critical paths
- [security-review.md](planning/security-review.md) - Security assessment (verdict: ship without concerns)

## Key Decisions

### 1. Component-Based Confidence (Not Single Score)
Expose 3 core signals (MVP) instead of a single 0-1 confidence score. Rationale: transparency, no magic weights, users can interpret based on context. Deviates from initiative's "0-100 scale" for better transparency.

### 2. In-Memory Computation (No Database)
Compute confidence from existing in-memory data after score fusion. Rationale: zero performance impact from DB, stateless, <5ms overhead.

### 3. Optional Fields (Backward Compatibility)
Add `confidence: Option<ConfidenceSignals>` with `#[serde(skip_serializing_if = "Option::is_none")]`. Rationale: existing MCP consumers unaffected, opt-in via `include_confidence: false` default.

### 4. Make Exact Match Always Available
`exact_match_multiplier` must be computed unconditionally (not just debug mode) so `is_exact_match` signal works. Rationale: core confidence signal, cannot be debug-only.

### 5. MVP Scope Reduction
Start with 3 signals, defer relative_score, rank, and query summary to Phase 2. Rationale: validate core signals first, iterate based on user feedback.

## Timeline

**Phase 1**: Core Confidence Infrastructure (Rust structs + computation logic)
**Phase 2**: TypeScript Type Sync and Integration (Rust-TS boundary + pipeline integration)
**Phase 3**: MCP Tool and Documentation (expose via MCP, document usage)

**Estimated**: 2-3 days total development time across 3 phases

## Success Criteria

**Functional (MVP - 3 Signals)**:
- ✅ Each search result includes 3 confidence signals when `include_confidence: true`
- ✅ Signals: source_count, score_gap, is_exact_match
- ✅ Exact match detection works without debug mode enabled
- ✅ Rust-TypeScript types synchronized with validation tests
- ✅ Backward compatibility maintained (default: false, existing consumers work)

**Quality**:
- ✅ Confidence signals are interpretable (self-explanatory field names)
- ✅ Documentation explains each component with examples
- ✅ Performance <5ms overhead validated by benchmarks in Phase 1

**User Experience**:
- ✅ High confidence results (3+ sources, exact match, large gap) → users trust immediately
- ✅ Low confidence results (1 source, no exact match, small gap) → users investigate
- ✅ Developers can improve queries based on confidence signals

**Deferred to Post-MVP**:
- Query-level confidence summary
- Additional signals (relative_score, rank)
- Categorical confidence bands
- Progressive filtering

## Risks & Mitigation

| Risk | Mitigation |
|------|-----------|
| Type sync breaks Rust-TypeScript | Automated validation tests, TYPE_SYNC comments, CI checks |
| Performance regression >5ms | O(1) per-result computation, benchmark tests |
| Backward compatibility broken | Optional fields with serde skip, integration tests |
| User confusion about signals | Clear documentation, self-explanatory names, examples |

## Next Steps

1. **Review Planning**: Run `/review-project SRCHCONF` to validate planning documents
2. **Address Feedback**: Update planning based on review findings if needed
3. **Generate Tickets**: Run `/create-project-tickets SRCHCONF` to create implementation tickets
4. **Execute**: Work through tickets phase by phase

## Agent Evaluation Recommendation

**Assessment**: This project would **benefit from specialized agents** in performance optimization and type synchronization, but general agents are sufficient for MVP.

**Areas where specialized agents could help** (optional, post-MVP):
- **Performance Engineer**: Optimize confidence computation if overhead exceeds 5ms target
- **Type Sync Specialist**: Automate Rust-TypeScript type synchronization and validation

**Decision**: Proceed with general agents for MVP. Re-evaluate after Phase 2 if performance or type sync issues emerge.

## Files Modified (Summary)

**Phase 1** (Rust Core):
- `crates/maproom/src/search/results.rs` - Add ConfidenceSignals struct (3 fields)
- `crates/maproom/src/search/confidence.rs` - NEW module for computation logic
- `crates/maproom/src/search/fts.rs` - Make exact_match_multiplier always available
- `crates/maproom/src/search/mod.rs` - Export confidence module
- `crates/maproom/benches/confidence_overhead.rs` - NEW benchmark

**Phase 2** (Integration):
- `packages/daemon-client/src/types.ts` - TypeScript interface (3 fields)
- `packages/daemon-client/src/types.test.ts` - Type validation tests
- `crates/maproom/src/search/executors.rs` - Integrate confidence computation

**Phase 3** (MCP Exposure):
- `packages/maproom-mcp/src/tools/search_schema.ts` - Add include_confidence parameter
- `packages/maproom-mcp/src/tools/search.ts` - Pass parameter to daemon
- `packages/daemon-client/src/client.ts` - Update SearchParams interface
- `packages/maproom-mcp/docs/confidence-scoring.md` - NEW documentation

**Total**: ~10 modified files, 3 new modules, ~400 lines of new code (estimated, reduced for MVP)
