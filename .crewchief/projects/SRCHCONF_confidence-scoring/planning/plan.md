# Plan: Confidence Scoring

## Overview

This project adds confidence scoring to maproom search results through a 3-phase incremental delivery approach. Each phase delivers independent value while building toward complete confidence transparency.

**Timeline**: 3 phases, estimated 2-3 days total development time
**Approach**: MVP-first, incremental delivery, backward compatible

## Phases

### Phase 1: Core Confidence Infrastructure

**Objective**: Implement foundational confidence types and computation logic in Rust

**Deliverables**:
- `ConfidenceSignals` struct in `crates/maproom/src/search/results.rs` (3 core fields: source_count, score_gap, is_exact_match)
- `confidence.rs` module with computation functions
- Make `exact_match_multiplier` always available (not debug-only)
- Unit tests for all confidence computation logic
- Serialization roundtrip tests
- Performance benchmarks (measure <5ms overhead before integration)

**Agent Assignments**:
- rust-engineer: Create confidence structs and computation module
- unit-test-runner: Execute Rust unit tests
- verify-ticket: Validate implementation against acceptance criteria
- commit-ticket: Create commit when verified

**Success Criteria**:
- ✅ `ConfidenceSignals` struct compiles with 3 core fields (source_count, score_gap, is_exact_match)
- ✅ `exact_match_multiplier` computed unconditionally in FTS ranking (not debug-only)
- ✅ `compute_result_confidence()` function works for all edge cases
- ✅ All unit tests pass (minimum 8 test cases for 3 fields)
- ✅ Serde serialization works (JSON roundtrip successful)
- ✅ Performance benchmark shows <5ms overhead for 20 results

**Files Modified**:
- `crates/maproom/src/search/results.rs` - Add ConfidenceSignals struct (3 fields)
- `crates/maproom/src/search/confidence.rs` - NEW module
- `crates/maproom/src/search/fts.rs` (or equivalent) - Make exact_match_multiplier always available
- `crates/maproom/src/search/mod.rs` - Export confidence module
- `crates/maproom/src/search/confidence_test.rs` - Unit tests
- `crates/maproom/benches/confidence_overhead.rs` - NEW benchmark

### Phase 2: TypeScript Type Sync and Integration

**Objective**: Synchronize types across Rust-TypeScript boundary and integrate with search pipeline

**Deliverables**:
- TypeScript `ConfidenceSignals` interface in `packages/daemon-client/src/types.ts` (3 fields matching Rust)
- Type sync validation tests (moved to Phase 1 for earlier validation)
- Integration with search executors
- `include_confidence` parameter support (default: false)

**Agent Assignments**:
- typescript-engineer: Create TypeScript interfaces with TYPE_SYNC comments
- rust-engineer: Integrate confidence computation into search pipeline
- unit-test-runner: Execute type sync tests
- verify-ticket: Validate type synchronization
- commit-ticket: Create commit when verified

**Success Criteria**:
- ✅ TypeScript interface matches Rust struct field-for-field (3 fields)
- ✅ TYPE_SYNC comments link correctly
- ✅ Type validation tests pass (roundtrip Rust → JSON → TypeScript) - run in Phase 1
- ✅ `include_confidence` parameter accepted with default=false
- ✅ Confidence computed when parameter is true
- ✅ Confidence omitted when parameter is false (backward compat)
- ✅ Integration tests pass with confidence enabled
- ✅ Exact match detection works without debug mode

**Files Modified**:
- `packages/daemon-client/src/types.ts` - Add interfaces
- `packages/daemon-client/src/types.test.ts` - Add validation tests
- `crates/maproom/src/search/executors.rs` - Integrate confidence computation
- `crates/maproom/src/search/results.rs` - Add optional confidence field to ChunkSearchResult

### Phase 3: MCP Tool and Documentation

**Objective**: Expose confidence via MCP tool and document usage patterns

**Deliverables**:
- `include_confidence` parameter in MCP search tool schema
- Updated MCP tool to pass parameter to daemon
- Documentation explaining confidence signals
- Examples showing high/low confidence scenarios
- End-to-end integration tests

**Agent Assignments**:
- typescript-engineer: Update MCP search tool and schema
- documentation-writer: Create confidence usage guide
- unit-test-runner: Execute integration tests
- verify-ticket: Validate end-to-end functionality
- commit-ticket: Create commit when verified

**Success Criteria**:
- ✅ MCP search tool accepts `include_confidence: true` (defaults to false)
- ✅ Search results include confidence when requested
- ✅ Backward compatibility maintained (existing calls work)
- ✅ Documentation explains 3 core confidence signals
- ✅ Examples show interpretation of signals (high/low confidence scenarios)
- ✅ Integration tests pass for both modes (with/without confidence)
- ✅ Performance overhead < 5ms confirmed (benchmark from Phase 1)
- ✅ Client display shows confidence in readable format

**Files Modified**:
- `packages/maproom-mcp/src/tools/search_schema.ts` - Add parameter
- `packages/maproom-mcp/src/tools/search.ts` - Pass parameter to daemon
- `packages/daemon-client/src/client.ts` - Add to SearchParams
- `packages/maproom-mcp/docs/confidence-scoring.md` - NEW documentation
- `packages/maproom-mcp/tests/integration/confidence.test.ts` - NEW integration tests

## Dependencies

### Phase 1 Prerequisites (COMPLETE)

This project is Phase 2 of the parent initiative. Phase 1 dependencies are complete:

1. **SRCHTRN (Search Transparency)** - ✅ COMPLETE (archived)
   - Delivered `QueryUnderstanding` struct and metadata pattern
   - Provides query interpretation transparency
   - Located in `crates/maproom/src/search/results.rs`

2. **SRCHFLTR (Result Filtering)** - ✅ COMPLETE (archived)
   - Delivered result filtering infrastructure
   - Provides cleaner result sets

**Status**: All external dependencies satisfied. Ready to proceed.

### Cross-Phase Dependencies

- Phase 2 depends on Phase 1: Cannot sync types until Rust structs exist
- Phase 3 depends on Phase 2: Cannot expose via MCP until integration complete

### Internal Dependencies

- **Exact match detection**: Need to make `exact_match_multiplier` always available (not debug-only)
- **Source scores**: Already available in `ChunkSearchResult`, confidence will use existing data

### Parallel Work Opportunities

- Documentation (Phase 3) can be drafted during Phase 1-2
- Integration tests (Phase 3) can be planned during Phase 2
- Benchmarking moved to Phase 1 for early validation

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Type sync breaks between Rust/TypeScript | Medium | High | Automated validation tests, TYPE_SYNC comments, CI checks |
| Performance regression (>5ms overhead) | Low | Medium | Benchmark tests, O(1) per-result computation, early profiling |
| Backward compatibility broken | Low | High | Optional fields with serde skip, integration tests for old API |
| Exact match detection unavailable (no debug data) | Medium | Low | Graceful degradation: is_exact_match=false, other signals still work |
| User confusion about signal interpretation | Medium | Medium | Clear documentation with examples, self-explanatory field names |
| Adoption too low (users don't enable) | Medium | Low | Make opt-in for MVP, gather feedback, enable by default in future |

### Risk Handling Strategies

**Type Sync Breaks**:
- Automated tests run on every commit
- Manual review checklist during code review
- TYPE_SYNC comments make relationships explicit

**Performance Regression**:
- Measure overhead in Phase 2 integration
- Profile with realistic workloads
- Optimize if >5ms detected (unlikely given O(1) per-result)

**Backward Compatibility**:
- Integration tests cover both modes (with/without confidence)
- Optional field pattern proven in SearchMetadata.understanding
- Beta test with existing MCP consumers before defaulting to enabled

## Success Metrics

### Phase 1 Success
- [ ] All Rust structs compile without warnings
- [ ] 100% unit test coverage for confidence computation
- [ ] Serialization roundtrip successful
- [ ] No clippy warnings in new code

### Phase 2 Success
- [ ] Type sync validation tests pass
- [ ] Integration tests pass with confidence enabled
- [ ] Integration tests pass with confidence disabled
- [ ] No type errors in TypeScript compilation

### Phase 3 Success
- [ ] MCP tool documentation complete
- [ ] End-to-end search with confidence works
- [ ] Performance overhead measured < 5ms
- [ ] Backward compatibility verified (existing clients work)
- [ ] At least 3 example scenarios documented

### Overall Project Success
- [ ] All 3 phases complete and merged
- [ ] Zero breaking changes to existing API
- [ ] Performance target met (<50ms p95 total latency)
- [ ] Documentation explains all confidence signals clearly
- [ ] CI pipeline includes confidence validation tests

## Rollout Plan

### Phase 1: Internal Testing
- Deploy to development environment
- Test with sample queries
- Validate confidence signals match expectations

### Phase 2: Opt-In Beta
- Release with `include_confidence=false` default
- Document opt-in process
- Gather feedback from early adopters
- Monitor performance metrics

### Phase 3: General Availability
- Announce feature in release notes
- Keep opt-in default for MVP
- (Future) Enable by default after validation period

## Rollback Strategy

**If Issues Detected**:
1. **Performance regression**: Disable confidence by default, investigate optimization
2. **Type sync errors**: Revert TypeScript changes, fix validation tests, re-deploy
3. **Backward compatibility break**: Revert MCP tool changes, restore previous behavior
4. **Critical bug**: Feature flag to disable confidence computation entirely

**No Schema Changes**: Rollback is simple - revert code, no database migration needed

## Future Enhancements (Post-MVP)

After validating core 3-signal MVP with users:

1. **Additional Signals**:
   - `relative_score` - Result score / top score (0.0-1.0)
   - `rank` - Position in result list (1-based)
   - Validate utility before adding

2. **Query-Level Summary** (deferred from MVP):
   - `SearchConfidenceSummary` with query-wide metrics
   - Active sources, coverage ratio, exact match ratio
   - Result saturation indicator

3. **Categorical Confidence Bands**:
   - HIGH/MEDIUM/LOW classification (if user feedback requests)
   - Derived from component signals with tunable thresholds
   - Simpler for users who don't need raw signals

4. **Progressive Filtering** (from initiative, out of MVP scope):
   - Automatic filtering of low-confidence results
   - User-configurable confidence threshold
   - Requires validation of confidence accuracy first

5. **Default Parameter Change**:
   - Flip `include_confidence` default to `true` after validation period
   - Requires monitoring adoption and feedback
   - Separate decision post-MVP

## Monitoring and Validation

### During Development
- Unit test coverage reports
- Type validation test results
- Integration test pass rate
- Performance benchmark trends

### Post-Deployment
- Percentage of searches with `include_confidence=true` (track via logging)
- Average confidence computation time (track via timing metrics)
- Distribution of source_count values (understand signal quality)
- Exact match ratio trends (validate exact match detection)

### Success Indicators
- >50% adoption after 2 weeks (Phase 2 opt-in)
- <5ms average overhead maintained
- Zero backward compatibility issues reported
- Positive feedback on signal interpretability
