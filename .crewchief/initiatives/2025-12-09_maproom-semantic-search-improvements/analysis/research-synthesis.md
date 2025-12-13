# Research Synthesis: Maproom Semantic Search Improvements

## Key Findings

### From User Session (Architectural Exploration)

**What Worked Exceptionally Well:**

1. **Speed and Performance**
   - Search latency: 40-60ms per query
   - 50-60% time savings vs traditional code exploration (grep + manual file browsing)
   - Fast enough to support iterative exploration workflow

2. **Semantic Understanding**
   - Successfully found code by concept, not just keywords
   - "daemon architecture" query found relevant implementation files
   - Cross-cutting concern identification (error handling, RPC protocol)
   - Natural language queries worked without keyword engineering

3. **Architectural Discovery**
   - Quickly located architecture documentation
   - Identified system boundaries and component relationships
   - Discovered implementation patterns across the codebase

**Critical Limitations Encountered:**

1. **Error Transparency** (2 failures)
   - Generic RPC_ERROR messages without diagnostic details
   - No indication of root cause (daemon crash? invalid query? database corruption?)
   - No actionable guidance for fixing the issue
   - User had no recourse except retry

2. **Result Quality Mixed**
   - Planning docs and archived tickets appeared alongside current code
   - No quick way to filter "code only" or "exclude archived content"
   - Manual inspection required to distinguish relevant from tangential results
   - Time wasted reviewing outdated planning documents

3. **No Query Feedback**
   - Unclear how queries were interpreted
   - No visibility into why certain results ranked highly
   - Difficult to refine searches when results too broad
   - No indication of search mode used (FTS vs vector vs hybrid)

4. **No Confidence Indicators**
   - All results presented equally
   - No guidance on which results were high vs low quality matches
   - Cognitive load determining relevance

### From Codebase Research

**Existing Search Infrastructure:**

1. **Semantic Ranking System (SEMRANK)**
   - Kind-based multipliers: func (2.5x), class (2.0x), test (0.6x), docs (0.3-0.6x)
   - Exact match bonus: 3.0x for normalized symbol name matches
   - Query normalization: camelCase → snake_case, case-insensitive
   - Debug mode exists with score_breakdown field
   - Performance validated: 17% faster than baseline FTS

2. **Deduplication System (SRCHDUP)**
   - Prevents duplicate results across worktrees
   - Same file/symbol/line number deduplicated
   - Configurable via `deduplicate` parameter (default: true)

3. **Hybrid Search (RRF Fusion)**
   - Combines FTS + vector search via Reciprocal Rank Fusion
   - Supports three modes: fts, vector, hybrid
   - Vector search uses cosine similarity on embeddings
   - FTS search uses PostgreSQL ts_rank_cd (now SQLite FTS5)

4. **Search Quality Validation**
   - Parser quality validated (90%+ name completeness, 50-80% doc coverage)
   - Symbol extraction consistent across languages (Python, Rust, Go)
   - Production ready for text search, semantic search, type-aware search
   - Comprehensive test suite for parser output quality

**Error Handling Infrastructure:**

1. **RpcError Class** (daemon-client/src/errors.ts)
   - Specific error codes: -32700 (parse), -32600 (invalid request), -32601 (method not found), etc.
   - Helper methods: isParseError(), isInternalError(), isInvalidParams()
   - Contains rpcCode and optional data field
   - But: Serialization loses daemon error details, only generic "RPC_ERROR" surfaces

2. **Daemon Error Types**
   - DaemonStartError, DaemonCrashError, DaemonTimeoutError
   - DaemonUnhealthyError, SocketConnectionError, SocketTimeoutError
   - All inherit from DaemonError with specific error codes
   - But: Not exposed through search tool interface

**Performance Baselines:**

1. **Current Latency** (from maproom-mcp tests)
   - Average: 40-60ms (user feedback)
   - p95: 48ms → 40ms after SEMRANK (17% improvement)
   - Target: <100ms p95 maintained
   - Budget for enhancements: ~30-40ms overhead allowance

2. **Daemon Architecture Benefits**
   - 20-50x performance vs spawning new process per request
   - Cold start: 225ms vs daemon request: 5-10ms
   - Supports concurrent requests without degradation

### From Industry Research (2025)

**Query Understanding and Feedback:**

1. **QEIEF Model** (January 2025 research)
   - Query expansion using intentional enhancement and feedback
   - Addresses semantic/structural differences between queries and code
   - Leverages developer-written descriptions for query expansion
   - Result: Improved semantic representation and precision

2. **Cursor's Semantic Search** (Production deployment)
   - 12.5% accuracy improvement with semantic search
   - Uses agent sessions as training data
   - Aligns embedding model with LLM-generated rankings
   - Tracks "what content would have been most helpful"

3. **Feedback Loop Integration** (Shopify example)
   - Logs search-click feedback
   - Re-ranks results based on post-click user behavior
   - Learning actual user paths improves precision over time

**Progressive and Confidence-Based Ranking:**

1. **Google's Confidence Advisories** (2025)
   - Shows content advisories when low confidence in result quality
   - Automatically detects rapidly-changing topics
   - AI Overviews now appear for 30% of queries (up from 19%)

2. **User Behavior Signals**
   - Click-through rate confirmed as Google ranking factor
   - User interaction with search results informs ranking
   - Progressive adaptation based on user behavior

**Best Practices:**

1. **Monitor and Track** - Log performance and user feedback
2. **Iterate Based on Usage** - Adjust models/datasets based on actual needs
3. **Provide Transparency** - Users should understand why results surfaced
4. **Enable Refinement** - Give users tools to narrow/broaden searches

## Open Questions

### Query Understanding

1. **Normalization Visibility**
   - Should users see how their query was normalized? (e.g., "validateToken" → "validate_token")
   - Would showing detected patterns help refine searches?
   - How verbose should query understanding feedback be?

2. **Mode Selection**
   - Should system auto-select FTS vs vector vs hybrid based on query characteristics?
   - Or should users have explicit control?
   - What heuristics determine best mode?

### Confidence Scoring

1. **Threshold Tuning**
   - What score thresholds define high/medium/low confidence?
   - Should thresholds vary by search mode (FTS vs vector)?
   - How to normalize scores across different ranking systems?

2. **Progressive Cutoff**
   - Should low-confidence results be excluded by default?
   - Or shown with clear warnings?
   - What if high-confidence results are sparse?

### Result Filtering

1. **Smart Defaults**
   - Should "code-first" be the default filter?
   - Should .crewchief/archive/ be auto-excluded?
   - How to handle edge cases (e.g., docs about archived code)?

2. **Filter Granularity**
   - File type filtering: by extension (*.ts) or by category (code/docs/tests)?
   - Path filtering: glob patterns or predefined exclusions?
   - Recency filtering: modified-since or git commit date?

### Relationship Clustering

1. **Graph Traversal Depth**
   - How many hops to traverse for "related results"?
   - Should clustering be recursive (related to related)?
   - Performance impact of deep graph traversal?

2. **Relationship Weighting**
   - How to weight import vs call vs proximity relationships?
   - Should same-file chunks always cluster?
   - How to prevent over-clustering (too many related results)?

### Test Coverage

1. **Golden Test Sets**
   - What queries constitute "semantic understanding"?
   - How to define expected results without overfitting?
   - How to keep test sets current as codebase evolves?

2. **Performance Regression**
   - What latency thresholds trigger test failures?
   - Should tests enforce <100ms p95 or allow degradation?
   - How to benchmark across different hardware?

## Assumptions

### Technical Assumptions

1. **Existing Infrastructure is Sufficient**
   - Assumption: Current FTS/vector/hybrid search provides good baseline results
   - Validation: User feedback confirms 50-60% time savings and semantic understanding works
   - Risk: Low - infrastructure proven in production

2. **No Schema Changes Required**
   - Assumption: Enhancements can use JSON columns or client-side processing
   - Validation: Filtering can use existing path/kind fields; scores already available
   - Risk: Medium - relationship clustering may need edge table optimization

3. **Performance Budget Available**
   - Assumption: ~30-40ms overhead budget for enhancements
   - Validation: Current baseline 40-60ms, target <100ms p95
   - Risk: Medium - need to benchmark each feature

4. **Daemon Architecture Supports Extensions**
   - Assumption: Current RPC protocol can accommodate new parameters/responses
   - Validation: Existing additive changes (debug mode, deduplication) worked well
   - Risk: Low - protocol designed for extensibility

### User Assumptions

1. **Transparency Improves Trust**
   - Assumption: Showing query understanding and confidence scores helps users
   - Validation: Industry research (Cursor, Google) shows transparency works
   - Risk: Low-Medium - could add cognitive load if too verbose

2. **Filtering Reduces Noise**
   - Assumption: Result type filtering (code/docs/tests) is valuable
   - Validation: User feedback directly requested this
   - Risk: Very Low - clear user need

3. **Developers Want Control**
   - Assumption: Users prefer smart defaults with override options
   - Validation: Industry norm for developer tools
   - Risk: Low - standard UX pattern

### Product Assumptions

1. **Iterative Delivery is Acceptable**
   - Assumption: Phase 1 (transparency + filtering) delivers immediate value
   - Validation: User pain points prioritized in Phase 1
   - Risk: Low - phased approach reduces scope risk

2. **Test Suite Prevents Regression**
   - Assumption: Comprehensive tests validate semantic understanding
   - Validation: Existing search quality validation approach works
   - Risk: Medium - test maintenance burden

3. **Backward Compatibility is Mandatory**
   - Assumption: Existing MCP clients cannot be broken
   - Validation: maproom-mcp and vscode-maproom depend on current interface
   - Risk: Low - additive-only changes enforced

## Research Gaps

### Needs Further Investigation

1. **Confidence Score Calibration**
   - Need to analyze score distribution across real queries
   - Establish baselines for high/medium/low thresholds
   - Validate thresholds against user perception

2. **Relationship Clustering Performance**
   - Need to prototype graph traversal algorithms
   - Benchmark latency impact of clustering
   - Determine optimal depth and weighting

3. **Filter Effectiveness**
   - Need to quantify how much noise filtering reduces
   - Test smart defaults with real user workflows
   - Validate path-based type inference accuracy

### Deferred to Implementation

1. **Query Expansion** - Complex ML problem, defer to Phase 2
2. **User Behavior Tracking** - Privacy and infrastructure concerns, defer
3. **Multi-Repo Search** - Separate initiative, out of scope

## Actionable Insights

### High-Confidence Opportunities

1. **Error Diagnostics** - Clear path to implementation, high user impact
2. **Result Type Filtering** - Directly requested by users, technically straightforward
3. **Query Understanding Feedback** - Industry-validated approach, additive enhancement

### Medium-Confidence Opportunities

4. **Confidence Scoring** - Requires threshold tuning but approach is clear
5. **Semantic Test Suites** - Test patterns exist, needs curation effort

### Lower-Confidence Opportunities

6. **Relationship Clustering** - Technically feasible but performance risk, needs prototyping

## Recommendations

### Immediate Actions (Phase 1)

1. **Enhance Error Handling**
   - Preserve daemon error details through RPC serialization
   - Add query validation before RPC call
   - Return actionable error messages with suggested fixes

2. **Implement Result Filtering**
   - Add type filter parameter (code/docs/tests/all)
   - Implement smart defaults (code-first, exclude archived)
   - Support file extension filtering

3. **Add Query Understanding**
   - Return normalized query form in metadata
   - Explain mode selection rationale
   - Show detected patterns (symbol names, etc.)

### Near-Term Actions (Phase 2)

4. **Confidence Scoring** - After establishing thresholds via analysis
5. **Relationship Clustering** - After performance prototyping

### Long-Term Actions (Phase 3+)

6. **Test Suites** - Validate all enhancements, prevent regression
7. **Search Observability** - Foundation for future ML/analytics

## Success Criteria

**Must Have:**
- 90% reduction in generic RPC_ERROR occurrences
- Result type filtering working reliably
- Query understanding feedback in all responses
- Performance maintained (<100ms p95)

**Should Have:**
- Confidence scores for all results
- Progressive filtering of low-confidence results
- Comprehensive test coverage (>80% scenarios)

**Nice to Have:**
- Relationship clustering for top results
- Search history foundation
- Advanced filtering (recency, complex globs)
