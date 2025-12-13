# Initiative: Maproom Semantic Search Improvements

Created: 2025-12-09

## Vision Statement

Enhance maproom's semantic code search capabilities to deliver production-grade developer experience through improved query understanding, intelligent result filtering, relationship-aware search, and comprehensive test coverage that validates real-world usage patterns.

## Conceptual Frame

Maproom has proven its value through real-world architectural exploration, delivering 50-60% time savings with sub-100ms search latency. However, usage feedback revealed critical gaps:

**The Current Reality:**
- Generic RPC_ERROR messages obscure actual failure causes
- Results mix planning docs, archived tickets, and current code without distinction
- No visibility into how queries are interpreted or why certain results rank highly
- No confidence indicators to help users assess result quality
- Limited ability to filter by result type (code vs docs vs tests)

**The Core Problem:**
Semantic search succeeded at speed and basic relevance, but lacks the transparency and control needed for production developer tools. Developers need to understand *why* they got these results and have tools to refine searches when initial results are too broad.

**The Opportunity:**
Industry research (2025) shows that query understanding with feedback mechanisms, progressive result ranking based on confidence, and intelligent filtering are table stakes for modern code search. Companies like Cursor report 12.5% accuracy improvements through semantic search with proper user feedback integration.

## Domain Coherence

**Core Domain Concepts:**

- **Query Understanding**: Transforming natural language queries into structured search operations with diagnostic feedback
- **Result Confidence**: Scoring mechanisms that indicate how well a result matches query intent
- **Type-Aware Filtering**: Distinguishing code implementations from tests, docs, and archived content
- **Relationship Clustering**: Grouping semantically related chunks to provide contextual understanding
- **Progressive Ranking**: Presenting high-confidence results first with clear score thresholds
- **Error Transparency**: Meaningful error messages that guide users to successful searches
- **Search Observability**: Understanding what succeeded, what failed, and why

## Directional Clarity

**Desired End State:**

"When this initiative succeeds, developers using maproom will receive transparent, actionable search results with clear confidence indicators, intelligent filtering of irrelevant content, and helpful guidance when searches fail or produce unexpected results."

**Success Signals:**

- [ ] RPC_ERROR messages replaced with specific, actionable error diagnostics
- [ ] Result type filtering allows developers to focus on code vs docs vs tests
- [ ] Query understanding feedback shows how searches were interpreted
- [ ] Confidence scores help developers assess result quality
- [ ] Relationship-aware search clusters related code chunks for better context
- [ ] Comprehensive test suite validates semantic understanding capabilities
- [ ] Search latency remains under 100ms p95 after enhancements
- [ ] User feedback shows measurable improvement in search success rate

## Scope Boundaries

**In Scope:**

- Query understanding and diagnostic feedback mechanisms
- Result type filtering with smart defaults (code-first, exclude archived)
- Confidence scoring and progressive result presentation
- Relationship-aware search to cluster related chunks
- Error handling improvements with actionable messages
- Comprehensive test suites for semantic understanding validation
- Performance benchmarking to ensure sub-100ms latency maintained
- Search history and pattern learning foundations

**Out of Scope:**

- Multi-repository federated search (separate initiative)
- Natural language query expansion (Phase 2)
- Machine learning-based result re-ranking (Phase 2)
- User preference persistence (separate feature)
- Search analytics dashboard (separate tooling)
- Integration with external code search services

**Critical Constraints:**

- Must maintain sub-100ms p95 search latency
- Must not break existing MCP tool interfaces
- Must be backward compatible with existing clients
- Cannot require database schema migrations for production systems
- Must work within existing daemon architecture

## Derived Projects

### Phase 1: Foundation (Immediate Value)

1. **SRCHTRNSP**: Search Transparency and Error Diagnostics
   - Replace generic RPC_ERROR with specific error types
   - Add query understanding feedback to search responses
   - Implement score breakdown in debug mode

2. **SRCHFLTR**: Result Type Filtering and Smart Defaults
   - Add result type filtering (code/docs/tests/archived)
   - Implement smart defaults (code-first, exclude archived)
   - Add file type filtering (by extension)

### Phase 2: Intelligence (Enhanced Ranking)

3. **SRCHCONF**: Confidence Scoring and Progressive Results
   - Implement confidence scoring for search results
   - Add progressive result cutoff (high-confidence first)
   - Expose confidence metadata in responses

4. **SRCHREL**: Relationship-Aware Search
   - Cluster semantically related chunks
   - Add "related results" section to responses
   - Implement cross-reference scoring

### Phase 3: Validation (Comprehensive Testing)

5. **SRCHTST**: Comprehensive Search Test Suites
   - Semantic understanding tests (concept vs keyword)
   - Architectural discovery tests
   - Cross-cutting concern tests
   - "Grep-impossible" task validation
   - Performance regression tests

## Status

- [x] Research complete
- [x] Analysis complete
- [x] Decomposition complete
- [ ] Projects created

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Performance degradation from additional filtering/scoring | High - breaks core value prop | Benchmark each feature; reject if >10ms overhead |
| Breaking changes to MCP tool interface | High - breaks existing clients | Additive-only changes; version new features |
| Over-engineering query understanding | Medium - complexity without value | Start with simple diagnostics; iterate based on usage |
| Test suite maintenance burden | Medium - tests become stale | Focus on high-value scenarios; automate validation |
| Database schema changes required | High - production migration complexity | Design around existing schema; use JSON columns for new metadata |

## Dependencies

- Existing search infrastructure (FTS + vector + hybrid)
- Current daemon architecture and RPC protocol
- Semantic ranking system (SEMRANK)
- Deduplication system (SRCHDUP)

## Timeline Estimate

- **Phase 1 (Foundation)**: 2-3 weeks - immediate user-facing improvements
- **Phase 2 (Intelligence)**: 3-4 weeks - enhanced ranking and relationships
- **Phase 3 (Validation)**: 2-3 weeks - comprehensive test coverage

**Total**: 7-10 weeks for complete initiative

## Success Metrics

**Quantitative:**
- Search latency p95 < 100ms (maintained)
- RPC_ERROR occurrences reduced by 90%
- Result filtering reduces irrelevant results by 50%+
- Test coverage for semantic scenarios > 80%

**Qualitative:**
- User feedback indicates clearer understanding of search results
- Developers can successfully refine searches when needed
- Error messages guide users to successful searches
- Confidence in search results measurably increases
