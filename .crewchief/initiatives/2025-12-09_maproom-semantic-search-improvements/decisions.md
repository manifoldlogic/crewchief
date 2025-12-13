# Decisions: Maproom Semantic Search Improvements

Running log of key decisions made during this initiative.

---

## Decisions

### [2025-12-09] Three-Phase Approach: Foundation, Intelligence, Validation

**Context:** Initiative spans multiple improvement areas with varying complexity and value. Need to sequence work for maximum early value while managing risk.

**Decision:** Organize into three sequential phases:
1. Phase 1 (Foundation): Error diagnostics + result filtering
2. Phase 2 (Intelligence): Confidence scoring + relationship clustering
3. Phase 3 (Validation): Comprehensive test suites

**Rationale:**
- Phase 1 addresses immediate user pain points (RPC_ERROR, mixed results)
- Foundation work enables later enhancements
- Test suite validates all improvements, preventing regression
- Iterative delivery reduces scope risk

**Alternatives Considered:**
- **Big Bang Approach**: Implement all features simultaneously
  - Rejected: Too risky, long time to value
- **Feature Flags**: Implement everything but gate behind flags
  - Rejected: Adds complexity, delayed testing
- **Two Phases Only**: Combine intelligence + validation
  - Rejected: Test suite should validate completed work, not be built concurrently

---

### [2025-12-09] Additive-Only API Changes

**Context:** maproom-mcp and vscode-maproom depend on current search interface. Breaking changes would require coordinated releases.

**Decision:** All enhancements must be additive. New parameters must be optional with sensible defaults. New response fields must be optional.

**Rationale:**
- Maintains backward compatibility with existing clients
- Allows independent deployment of daemon vs clients
- New features degrade gracefully for old clients
- Industry best practice for API evolution

**Alternatives Considered:**
- **Versioned API**: Introduce /v2/search endpoint
  - Rejected: Unnecessary complexity, duplicate code paths
- **Breaking Changes with Migration**: Force upgrade all clients
  - Rejected: High coordination cost, deployment risk

---

### [2025-12-09] Client-Side Query Validation

**Context:** Need to provide better error messages without modifying Rust daemon for every validation rule.

**Decision:** Implement query validation in TypeScript (maproom-mcp) before RPC call. Preserve daemon errors when they occur but prevent common mistakes early.

**Rationale:**
- Faster iteration (TypeScript vs Rust + build + deploy)
- Better error messages (can reference MCP docs, etc.)
- Reduces unnecessary daemon calls
- Daemon still validates for robustness

**Alternatives Considered:**
- **Daemon-Only Validation**: All validation in Rust
  - Rejected: Slower iteration, less flexible error messages
- **No Validation**: Let daemon errors surface directly
  - Rejected: Doesn't solve RPC_ERROR problem

---

### [2025-12-09] Path-Based Result Type Filtering

**Context:** Need to distinguish code from docs from tests without schema changes or expensive classification.

**Decision:** Infer result type from file path patterns and chunk kind:
- Code: `kind IN (func, class, struct, etc.)` AND `path NOT LIKE %.md`
- Docs: `kind IN (heading, markdown_section, code_block)` OR `path LIKE %.md`
- Tests: `path LIKE %test%` OR `path LIKE %spec%` OR `kind = test`
- Archived: `path LIKE .crewchief/archive/%`

**Rationale:**
- No schema changes required
- Works with existing indexed data
- Fast filtering (path string operations)
- Covers 95%+ of real use cases

**Alternatives Considered:**
- **Add `result_type` Column**: Classify during indexing
  - Rejected: Requires schema migration, re-indexing
- **ML-Based Classification**: Train model to classify chunks
  - Rejected: Over-engineering, maintenance burden
- **User Manual Tagging**: Let users tag result types
  - Rejected: Friction, poor UX

---

### [2025-12-09] Smart Defaults: Code-First, Exclude Archived

**Context:** User feedback shows planning docs and archived content pollute results. Most searches target current production code.

**Decision:** Default filter settings:
- `type: 'code'` (can override to 'all', 'docs', 'tests')
- `exclude_archived: true` (can override to `false`)

**Rationale:**
- Matches user intent 90%+ of the time
- Reduces noise without losing functionality
- Explicit override preserves access to all content
- Industry norm (e.g., GitHub search excludes forks by default)

**Alternatives Considered:**
- **No Defaults**: User must specify every filter
  - Rejected: Poor UX, verbose queries
- **`type: 'all'` Default**: Show everything, let user filter down
  - Rejected: Doesn't solve the mixed results problem

---

### [2025-12-09] Defer Search Observability to Phase 2

**Context:** Logging search queries and user interactions enables ML-based improvements but requires infrastructure (storage, privacy, analysis).

**Decision:** Defer search observability features (history, click tracking, query logs) to future phase. Focus Phase 1-3 on immediate user-facing improvements.

**Rationale:**
- Immediate value is in transparency and filtering, not analytics
- Observability is foundation for future ML, not end-user feature
- Privacy and storage concerns need careful design
- Can retrofit logging without changing user-facing features

**Alternatives Considered:**
- **Include in Phase 1**: Build observability from the start
  - Rejected: Distracts from user pain points, adds complexity
- **Lightweight Logging**: Just log queries without analysis
  - Rejected: Storage without analysis is waste; do properly or not at all

---

### [2025-12-09] Performance Budget: <10ms Per Enhancement

**Context:** Current search latency is 40-60ms (p95: ~40ms). Target is <100ms p95. Have ~30-40ms budget for enhancements.

**Decision:** Each enhancement must add <10ms latency (p95). Benchmark before merging. Reject features that exceed budget.

**Rationale:**
- 3-4 enhancements fit in budget (filtering, scoring, clustering, metadata)
- Maintains core value proposition (speed)
- Clear acceptance criteria for each feature
- Prevents death by a thousand cuts

**Alternatives Considered:**
- **No Hard Limit**: Allow features if "reasonably fast"
  - Rejected: Ambiguous, risk of performance regression
- **100ms Total Limit**: Track cumulative latency only
  - Rejected: No per-feature accountability, hard to debug regressions

---

### [2025-12-09] Confidence Scoring Deferred to Phase 2

**Context:** Confidence scoring requires threshold tuning and score normalization analysis. Not immediately actionable without baseline data.

**Decision:** Phase 1 focuses on transparency (error messages, query understanding, filtering). Confidence scoring in Phase 2 after collecting score distribution data.

**Rationale:**
- Need real query data to calibrate thresholds (high/medium/low)
- Filtering delivers more immediate value
- Confidence scoring builds on Phase 1 foundations
- Allows time for research and prototyping

**Alternatives Considered:**
- **Include in Phase 1**: Guess thresholds, iterate
  - Rejected: Risk of poor UX if thresholds wrong
- **Skip Entirely**: Only do filtering and diagnostics
  - Rejected: Confidence is valuable for ranking transparency

---

## Pending Decisions

**To Be Decided During Decomposition:**

1. **Relationship Clustering Scope**: 1-hop or 2-hop graph traversal?
2. **Test Suite Organization**: Integration tests vs unit tests vs benchmark tests?
3. **Filter Parameter Design**: Single `filter` object or multiple parameters (`type_filter`, `exclude_archived`, etc.)?
4. **Query Understanding Verbosity**: Full diagnostics or minimal feedback?

**To Be Decided During Implementation:**

1. **Confidence Threshold Values**: What scores constitute high/medium/low?
2. **Related Results Count**: How many related chunks to return per result?
3. **Error Message Format**: Structured JSON or human-readable strings?

---

## Decision Template

### [DATE] Decision Title

**Context:** [Why this decision was needed]

**Decision:** [What was decided]

**Rationale:** [Why this choice]

**Alternatives Considered:**
- [Option A]: [Why rejected]
- [Option B]: [Why rejected]
