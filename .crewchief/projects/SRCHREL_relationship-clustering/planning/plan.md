# Plan: Relationship-Aware Search

## Overview

This plan implements relationship-aware search clustering in 3 phased deliveries, building from core Rust infrastructure through TypeScript integration to comprehensive testing and documentation. Each phase delivers independently testable value while setting the foundation for subsequent work.

**Total Estimated Duration**: 3-4 weeks

## Phases

### Phase 1: Rust Core Infrastructure (Week 1)

**Objective:** Implement relationship expansion logic in Rust search pipeline with type definitions

**Deliverables:**
- `RelatedChunkResult` struct in `search/results.rs` with TYPE_SYNC comments
- `relationships.rs` module with `find_top_related_chunks()` function
- Edge weight computation (`compute_edge_weight()`)
- Module proximity boost (`extract_parent_dir()`, module weighting)
- Integration into search pipeline with `include_related` parameter
- Unit tests for relevance scoring, edge weighting, module proximity
- Performance benchmarks (baseline latency measurement)

**Agent Assignments:**
- rust-expert: Implement `RelatedChunkResult` type, relationship expansion module
- search-engineer: Integrate relationship expansion into search pipeline after confidence scoring
- database-engineer: Verify existing `chunk_edges` indexes are optimal for depth-2 traversal
- performance-engineer: Benchmark graph traversal latency, validate <20ms overhead budget
- test-engineer: Write unit tests for scoring logic, edge cases (empty results, no relationships)

**Acceptance Criteria:**
- `find_top_related_chunks()` returns top 5 related chunks with correct relevance scores
- Module proximity boost (1.2×) applied correctly
- Edge weights (0.5-1.1) applied based on target kind
- Performance: Single result traversal <8ms p95
- Tests pass for edge cases (cyclic graphs, empty results, depth limits)

### Phase 2: TypeScript Integration and API (Week 2)

**Objective:** Expose relationship expansion through daemon RPC and MCP tool

**Deliverables:**
- TypeScript `RelatedChunkResult` interface in `daemon-client/src/types.ts`
- Type synchronization validation tests
- `include_related` parameter in daemon client `SearchParams`
- MCP search tool schema updated with `include_related` parameter
- Response serialization/deserialization tests
- Backward compatibility tests (without parameter)
- Integration tests (E2E search with relationships)

**Agent Assignments:**
- typescript-expert: Add `RelatedChunkResult` type to daemon-client, update `SearchParams`
- mcp-engineer: Update search tool schema, add parameter documentation
- test-engineer: Type sync validation tests, E2E integration tests
- api-engineer: Verify JSON serialization, optional field handling

**Acceptance Criteria:**
- `RelatedChunkResult` TypeScript interface matches Rust struct (validated by tests)
- MCP search tool accepts `include_related=true` parameter
- Response includes `related` field for high-confidence results
- Backward compatibility: Searches without parameter work unchanged
- Type sync tests catch Rust ↔ TypeScript discrepancies

### Phase 3: Testing, Documentation, and Polish (Week 3)

**Objective:** Comprehensive testing, performance validation, user-facing documentation

**Deliverables:**
- Performance regression tests (<20ms overhead)
- Response size monitoring (payload <10KB)
- Confidence gating validation (only high-confidence results expanded)
- Edge case testing (no confidence, all low-confidence, etc.)
- User documentation (usage patterns, examples)
- Developer documentation (architecture, decision records)
- Error handling validation (graceful degradation)
- Final benchmarks and optimization (parallel traversal if needed)

**Agent Assignments:**
- test-engineer: Comprehensive test suite (confidence gating, edge cases, performance)
- performance-engineer: Benchmark suite, latency validation, parallel traversal optimization
- technical-writer: User documentation, MCP tool examples, architecture guide
- quality-engineer: Integration test coverage, error handling validation
- search-engineer: Final optimizations based on benchmark results

**Acceptance Criteria:**
- Performance tests validate <20ms overhead at p95
- Response size <10KB for typical queries (10 results, 3 with relationships)
- Confidence gating works correctly (threshold: source_count >= 2 OR is_exact_match)
- Documentation includes usage examples, decision rationale, architectural overview
- Error handling gracefully degrades (failures don't break search)
- All acceptance criteria from previous phases still passing

## Dependencies

### External Dependencies (Must Complete Before Starting)

1. **SRCHCONF (Confidence Scoring)** - COMPLETE ✓
   - Provides `ConfidenceSignals` with `source_count` and `is_exact_match` fields
   - Critical for gating relationship expansion to high-confidence results
   - Status: Archived, fully functional

2. **SRCHFLTR (Result Filtering)** - COMPLETE ✓
   - Provides cleaner result sets before clustering
   - Enables type-aware edge weighting (code vs test distinction)
   - Status: Archived, fully functional

### Internal Phase Dependencies

```
Phase 1 (Rust Core) → Phase 2 (TypeScript Integration) → Phase 3 (Testing & Docs)
        ↓
   Blocks Phase 2                Blocks Phase 3
```

**Phase 1 → Phase 2**:
- Rust types must exist before TypeScript mirroring
- `include_related` parameter must be in search pipeline before daemon client exposes it
- Relationship expansion logic must be testable before E2E integration tests

**Phase 2 → Phase 3**:
- Full API integration required before comprehensive testing
- Type synchronization must work before validating payloads
- MCP tool parameter must exist before documenting user-facing examples

### Cross-Project Integration Points

**With SRCHCONF**:
- Requires `confidence` field to be populated on `ChunkSearchResult`
- Fallback: If `include_confidence=false`, skip relationship expansion
- Recommendation: Users should enable both (`include_confidence=true` + `include_related=true`)

**With SRCHFLTR**:
- Benefits from filtered results (code-only) for cleaner relationships
- Type filtering enables better edge weight detection
- Not required: Relationship expansion works on unfiltered results too

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| **Performance Budget Exceeded** (>20ms overhead) | Medium | High | Phase 1: Benchmark early. Phase 3: Implement parallel traversal if needed. Cap at 2 results if 3 exceeds budget. |
| **Type Sync Errors** (Rust ↔ TypeScript mismatch) | Low | High | Phase 2: Validation tests catch discrepancies immediately. CI fails on type mismatch. |
| **Related Chunks Not Useful** (low quality) | Medium | Medium | Phase 1: Confidence gating prevents expansion of weak results. Edge weights deprioritize test relationships. Module proximity boosts same-directory chunks. |
| **Response Size Bloat** (>10KB payloads) | Low | Medium | Phase 3: Monitor payload size. Cap at 5 related chunks per result. Metadata-only responses (no content). |
| **Confidence Unavailable** (user didn't enable it) | Medium | Low | Phase 2: Document recommended usage (`include_confidence=true` + `include_related=true`). Skip expansion if confidence missing. |
| **Graph Traversal Fails** (database error) | Low | Low | Phase 1: Graceful degradation (log warning, `related=None`). Don't fail entire search. |

## Success Metrics

### Must Achieve (MVP Launch)

- [ ] **Relationship Discovery**: Related chunks appear for high-confidence results
  - Test: Search with `include_related=true`, verify results with `confidence.source_count >= 2` have `related` field

- [ ] **Performance Budget**: <20ms p95 overhead for relationship expansion
  - Test: Benchmark search latency with/without `include_related`, measure delta

- [ ] **Backward Compatibility**: Existing clients unaffected
  - Test: Search without `include_related` parameter, verify no `related` field, same latency

- [ ] **Confidence Gating**: Only high-confidence results expanded
  - Test: Mock 10 results (3 high-confidence, 7 low), verify only 3 have `related` field

- [ ] **Type Synchronization**: Rust ↔ TypeScript types match
  - Test: Type sync validation tests pass in CI

### Should Achieve (Enhanced Quality)

- [ ] **Module Proximity Boost**: Same-directory chunks rank higher
  - Test: Verify chunks in same directory as source have higher relevance scores

- [ ] **Edge Weight Accuracy**: Code→code > code→test
  - Test: Result with edges to both code and test, verify code ranks higher

- [ ] **Graceful Degradation**: Graph errors don't break search
  - Test: Simulate database error during traversal, verify search completes with `related=None`

### Nice to Have (Future Enhancement)

- [ ] **Parallel Traversal**: If sequential exceeds budget, parallelize
  - Condition: Only implement if Phase 1 benchmarks show >20ms overhead

- [ ] **Cross-Result Deduplication**: Related chunks don't duplicate across results
  - Defer to Phase 2 if complexity exceeds MVP scope

- [ ] **Configurable Depth**: Allow users to specify `max_depth` parameter
  - Defer to future iteration based on user feedback

## Timeline

### Week 1: Rust Core (Phase 1)

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Mon-Tue | Type definitions, relationships module | `RelatedChunkResult`, `find_top_related_chunks()` |
| Wed-Thu | Pipeline integration, relevance scoring | Edge weights, module proximity, search integration |
| Fri | Unit tests, initial benchmarks | Test suite, performance baseline |

**Checkpoint**: Can traverse graph and return weighted related chunks for a single result.

### Week 2: TypeScript Integration (Phase 2)

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Mon | TypeScript types, daemon client | `RelatedChunkResult` interface, `SearchParams.include_related` |
| Tue | MCP tool schema, parameter validation | Tool schema update, parameter docs |
| Wed-Thu | Type sync tests, E2E integration | Validation tests, integration tests |
| Fri | Backward compatibility, error cases | Compatibility tests, error handling |

**Checkpoint**: MCP tool returns related chunks in response, types validated.

### Week 3: Testing & Documentation (Phase 3)

| Day | Focus | Deliverable |
|-----|-------|-------------|
| Mon | Performance testing, optimization | Benchmark suite, parallel traversal (if needed) |
| Tue | Confidence gating tests, edge cases | Comprehensive test coverage |
| Wed | User documentation, examples | Usage patterns, MCP tool guide |
| Thu | Developer docs, architecture review | Architecture guide, decision records |
| Fri | Final validation, sign-off | All success metrics verified |

**Checkpoint**: All tests pass, documentation complete, ready for production deployment.

### Week 4 Buffer: Optimization and Refinement (If Needed)

- Address any performance issues discovered in Week 3
- Refine edge weight heuristics based on real-world testing
- Improve documentation clarity based on review feedback
- Handle edge cases discovered during comprehensive testing

## Rollout Strategy

### Stage 1: Internal Validation (Week 1-2)

- Feature flag: `include_related=false` by default
- Manual testing with `include_related=true`
- Performance benchmarks on production database copy
- Validate confidence gating with real search queries

### Stage 2: Opt-In Beta (Week 3)

- Documentation published
- MCP tool parameter available
- Monitor latency, response size, error rates
- Gather user feedback on related chunk quality

### Stage 3: General Availability (Week 4)

- `include_related` defaults to `false` (opt-in maintained)
- Considered for default-on in future based on feedback
- Monitoring dashboards track adoption, performance

## Post-Launch Monitoring

### Key Metrics to Track

1. **Adoption Rate**: % of searches with `include_related=true`
2. **Performance Impact**: p95 latency delta with relationships enabled
3. **Quality Signals**:
   - Average related chunk count per result
   - % of results with related chunks (should match confidence threshold ~30%)
   - Click-through rate on related chunks (if UX supports it)
4. **Error Rates**: Graph traversal failures, timeouts
5. **Response Size**: p95 payload size with relationships

### Success Indicators

- **Latency**: p95 overhead remains <20ms
- **Adoption**: >20% of searches use `include_related=true` within 1 month
- **Quality**: Users report related chunks are useful (qualitative feedback)
- **Stability**: <1% error rate for relationship expansion
- **Scalability**: Performance remains stable as database grows

## Future Enhancements (Post-MVP)

Based on MVP learnings, consider:

1. **ML-Based Clustering**: Augment graph edges with embedding similarity
2. **User Preferences**: Remember preferred relationship types (imports vs calls)
3. **Cross-Repository Relationships**: Traverse edges across repo boundaries
4. **Configurable Depth**: Allow `max_depth` parameter for power users
5. **Relationship Type Filtering**: Allow `edge_types=['import', 'call']` parameter
6. **Progressive Deduplication**: Remove related chunks that appear in main results
7. **Relationship Visualization**: Graph diagram of chunk relationships (UX enhancement)
