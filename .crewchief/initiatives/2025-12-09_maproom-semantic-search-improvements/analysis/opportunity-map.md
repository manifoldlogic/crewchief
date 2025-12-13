# Opportunity Map: Maproom Semantic Search Improvements

## Problem Spaces

### Problem 1: Opaque Error Messages

**Description:** Search failures surface as generic "RPC_ERROR" messages without specific diagnostic information

**Impact:**
- Developers cannot distinguish between daemon crashes, invalid queries, or database errors
- No actionable guidance for fixing searches
- Lost productivity troubleshooting blind failures
- Erodes trust in search reliability

**Current State:**
- RpcError class exists with code checking methods (isParseError, isInternalError, etc.)
- Error details exist in daemon but lost in client serialization
- No query validation feedback before execution

**Evidence from Usage:**
- 2 search failures during architectural exploration with only "RPC_ERROR" visible
- No indication whether issue was query syntax, daemon state, or data corruption

### Problem 2: Mixed Result Quality

**Description:** Results blend code implementations, documentation, tests, and archived content without distinction

**Impact:**
- Developers must manually filter irrelevant results
- Planning docs and archived tickets pollute code searches
- No way to focus on current, production code
- Time wasted reviewing outdated or tangential content

**Current State:**
- Semantic ranking (SEMRANK) prioritizes code over docs/tests (0.3-2.5x multipliers)
- No filtering by result type or recency
- No exclude patterns for archived content
- File path appears in results but requires manual inspection

**Evidence from Usage:**
- Searches returned planning docs alongside relevant code
- Archived ticket content appeared in results
- No quick way to say "code only" or "exclude .crewchief/"

### Problem 3: No Query Understanding Feedback

**Description:** Users don't see how their queries are interpreted or why results were selected

**Impact:**
- Cannot refine searches effectively
- No visibility into FTS vs vector vs hybrid mode selection
- No understanding of why certain results rank highly
- Difficult to debug unexpected results

**Current State:**
- Debug mode exists with score_breakdown field
- Only available when explicitly enabled
- Breakdown shows kind_multiplier and exact_match but not query processing
- No normalization feedback (camelCase → snake_case, etc.)

**Evidence from Usage:**
- Searches succeeded but unclear why certain results ranked first
- No indication of how natural language queries mapped to search terms

### Problem 4: No Confidence Indicators

**Description:** All results presented equally regardless of match quality

**Impact:**
- Users cannot assess result reliability
- Weak matches mixed with strong matches
- No clear threshold for "good enough" results
- Cognitive load determining which results matter

**Current State:**
- Scores exist (final_score from SEMRANK)
- Scores not normalized or contextualized
- No confidence bands (high/medium/low)
- No progressive cutoff for weak results

**Evidence from Usage:**
- Results delivered but no indication which were high-confidence
- All results equally weighted in presentation

### Problem 5: Missing Relationship Context

**Description:** Related code chunks scattered across results without clustering

**Impact:**
- Lose architectural context (e.g., interface + implementations)
- Miss cross-cutting concerns (multiple files implementing pattern)
- Cannot see "what else is related to this result"
- Fragmented understanding of code relationships

**Current State:**
- Deduplication (SRCHDUP) prevents duplicate results across worktrees
- No semantic clustering of related chunks
- No "related results" feature
- Import/call relationships exist but not exposed in search

**Evidence from Usage:**
- Searches found relevant chunks but not related context
- No indication of "also see these related implementations"

## Goals

### Goal 1: Transparent Search Operations

**Outcome:** Every search result includes query understanding feedback and clear error diagnostics

**Measurement:**
- 90% reduction in "RPC_ERROR" generic errors
- All errors include actionable next steps
- Query feedback shows normalization and mode selection

### Goal 2: Intelligent Result Filtering

**Outcome:** Developers can focus searches on relevant content types with smart defaults

**Measurement:**
- Result type filtering available (code/docs/tests/archived)
- 50%+ reduction in irrelevant results via smart defaults
- File type filtering by extension works reliably

### Goal 3: Confidence-Based Ranking

**Outcome:** High-confidence results surface first with clear quality indicators

**Measurement:**
- Confidence scores visible for all results
- Progressive cutoff excludes low-confidence results
- Users report better understanding of result quality

### Goal 4: Relationship-Aware Discovery

**Outcome:** Search results cluster related chunks and expose architectural relationships

**Measurement:**
- Related results appear for high-confidence matches
- Cross-cutting concerns discoverable via relationship clustering
- Architectural patterns visible through grouped results

### Goal 5: Comprehensive Semantic Validation

**Outcome:** Test suites validate semantic understanding, not just keyword matching

**Measurement:**
- Test coverage for concept-based searches (>80%)
- Architectural discovery scenarios validated
- "Grep-impossible" tasks pass reliably
- Performance maintained (<100ms p95)

## Constraints

### Performance Constraints

- **Latency Budget:** Must maintain <100ms p95 search latency
  - Current baseline: 40-60ms per query (per user feedback)
  - Each enhancement: <10ms overhead budget
  - Total enhancement budget: ~30-40ms

- **Throughput:** Must handle concurrent searches without degradation
  - Current: Daemon-based architecture supports parallel requests
  - Constraint: Additional filtering/scoring must not block

### Compatibility Constraints

- **MCP Interface:** Cannot break existing tool signatures
  - Current: `search(query, repo, mode, k, debug)` interface
  - Requirement: Additive changes only (new optional parameters)

- **Client Support:** Must work with existing maproom-mcp and vscode-maproom clients
  - Constraint: New features must degrade gracefully for old clients
  - Requirement: Backward compatibility for all existing queries

### Data Constraints

- **Schema Stability:** Cannot require production database migrations
  - Current: SQLite with fixed schema for chunks, edges, embeddings
  - Workaround: Use JSON columns for metadata extensions
  - Constraint: No new required columns

- **Index Performance:** New filtering must use existing indexes
  - Current: FTS index on ts_doc, vector index on embeddings
  - Constraint: No full table scans
  - Requirement: Filter pushdown to database layer

### Operational Constraints

- **Daemon Architecture:** Must work within existing lifecycle management
  - Current: Auto-start daemon, graceful shutdown, circuit breaker
  - Constraint: No new daemon processes or ports
  - Requirement: Features implement within current RPC methods

## Opportunities

### Opportunity 1: Query Diagnostics Layer

**Value:** Immediate improvement to developer experience with minimal implementation cost

**Feasibility:** High - requires TypeScript-side validation and error enrichment

**Approach:**
- Enhance RpcError serialization to preserve daemon error details
- Add query validation before RPC call (syntax, parameter checks)
- Return query understanding metadata (normalized form, mode selection rationale)

**Dependencies:** None - purely client-side enhancement

**Effort:** Small (1-2 weeks)

### Opportunity 2: Result Type Filtering

**Value:** Directly addresses #1 user complaint (mixed result quality)

**Feasibility:** High - file path-based filtering implementable without schema changes

**Approach:**
- Add `filter` parameter to search: `{ type: 'code' | 'docs' | 'tests' | 'all' }`
- Implement path-based filtering (exclude `.crewchief/archive/`, `*.md` for code-only)
- Add smart defaults (code-first, exclude archived)
- Support file extension filtering (`file_type: 'ts,tsx,js'`)

**Dependencies:** None - can implement purely in Rust daemon layer

**Effort:** Small-Medium (2-3 weeks)

### Opportunity 3: Confidence Scoring

**Value:** Helps users assess result quality and improves ranking transparency

**Feasibility:** Medium - requires score normalization and threshold tuning

**Approach:**
- Normalize final_score to 0-100 confidence percentage
- Define confidence bands (high: >80, medium: 50-80, low: <50)
- Add progressive cutoff (return only high+medium by default)
- Expose confidence in result metadata

**Dependencies:** Requires SEMRANK scores (already available)

**Effort:** Medium (2-3 weeks)

### Opportunity 4: Relationship Clustering

**Value:** Unlocks architectural discovery and cross-cutting concern identification

**Feasibility:** Medium-High - edges exist but requires clustering algorithm

**Approach:**
- Leverage existing import/call edges in database
- Cluster results by file proximity and edge relationships
- Add "related results" section to response
- Implement cross-reference scoring boost

**Dependencies:** Edge data exists; needs graph traversal implementation

**Effort:** Medium-Large (3-4 weeks)

### Opportunity 5: Semantic Test Suites

**Value:** Validates semantic understanding and prevents regressions

**Feasibility:** High - test infrastructure exists, needs scenario coverage

**Approach:**
- Create golden test sets for semantic queries
- Implement "grep-impossible" task validation
- Add architectural discovery test scenarios
- Build performance regression benchmarks
- Validate cross-cutting concern detection

**Dependencies:** Requires test data curation and baseline establishment

**Effort:** Medium (2-3 weeks)

### Opportunity 6: Search Observability

**Value:** Enables monitoring, debugging, and future ML-based improvements

**Feasibility:** Low-Medium - requires persistence and analysis infrastructure

**Approach:**
- Log search queries with results and user interactions
- Track query success/failure patterns
- Collect latency and result quality metrics
- Build foundation for search history feature

**Dependencies:** Logging infrastructure, privacy considerations

**Effort:** Medium-Large (3-4 weeks)

**Status:** Deferred to Phase 2 - focus on immediate user-facing improvements first

## Prioritization

### High Priority (Phase 1: Foundation)

1. **Query Diagnostics** - Immediate pain point relief
2. **Result Type Filtering** - Direct user request, high ROI

### Medium Priority (Phase 2: Intelligence)

3. **Confidence Scoring** - Enhances ranking transparency
4. **Relationship Clustering** - Unlocks architectural discovery

### Lower Priority (Phase 3: Validation)

5. **Semantic Test Suites** - Validates enhancements, prevents regression

### Deferred (Future Phases)

6. **Search Observability** - Foundation for ML/analytics, not immediate value

## Constraints vs Opportunities Matrix

| Opportunity | Performance Impact | Compatibility Risk | Data Complexity | Overall Feasibility |
|-------------|-------------------|-------------------|-----------------|---------------------|
| Query Diagnostics | Very Low (<5ms) | Very Low (client-side) | Very Low (no DB) | ⭐⭐⭐⭐⭐ Very High |
| Result Type Filtering | Low (<10ms) | Low (additive param) | Low (path filter) | ⭐⭐⭐⭐ High |
| Confidence Scoring | Low (<10ms) | Low (metadata only) | Medium (tuning) | ⭐⭐⭐⭐ High |
| Relationship Clustering | Medium (10-20ms) | Low (additive feature) | Medium (graph) | ⭐⭐⭐ Medium-High |
| Semantic Test Suites | N/A (tests) | N/A (tests) | Medium (curation) | ⭐⭐⭐⭐ High |
| Search Observability | Low-Medium | Low | High (storage) | ⭐⭐ Medium |

## Next Steps

1. **Validate assumptions** via prototype query diagnostics enhancement
2. **Gather user feedback** on filtering priorities (path vs type vs recency)
3. **Establish baselines** for confidence scoring thresholds
4. **Decompose into projects** with clear deliverables and dependencies
5. **Create project summaries** ready for `/create-project` execution
