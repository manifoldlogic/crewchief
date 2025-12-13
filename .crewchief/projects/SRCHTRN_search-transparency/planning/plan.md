# Plan: Search Transparency

## Overview

This document outlines the phased execution plan for replacing generic RPC_ERROR messages with structured error diagnostics and adding query understanding feedback to maproom's search pipeline.

**Strategy**: Bottom-up implementation starting with Rust error taxonomy, then TypeScript deserialization, then MCP formatting.

**Timeline**: 3 phases, progressive enhancement approach.

## Phases

### Phase 1: Error Diagnostics Foundation

**Objective**: Establish structured error types in Rust and propagate to TypeScript clients. Replace 90% of generic RPC_ERROR messages with actionable diagnostics.

**Deliverables**:
- Rust error taxonomy (`SearchErrorDetails`, `ErrorType`, `PipelineStage`)
- Error conversion logic (`from_pipeline_error()`)
- Hardcoded suggestions for 6 error types
- JSON-RPC error serialization with `data` field
- TypeScript error types (`packages/daemon-client/src/types.ts`)
- TypeScript error deserialization (`RpcError.details`, `getUserMessage()`)
- MCP tool error formatting

**Agent Assignments**:
- **rust-engineer**: Create `crates/maproom/src/search/errors.rs`, implement error taxonomy and conversion logic
- **rust-engineer**: Modify `crates/maproom/src/daemon/server.rs` search handler to serialize error details
- **typescript-engineer**: Create `packages/daemon-client/src/types.ts` with mirrored error types
- **typescript-engineer**: Extend `packages/daemon-client/src/rpc.ts` with error deserialization
- **typescript-engineer**: Update `packages/maproom-mcp/src/tools/search.ts` error formatting
- **unit-test-runner**: Validate error conversion logic
- **verify-ticket**: Confirm error scenarios work end-to-end

**Acceptance Criteria**:
- Performance baseline measured (p50, p95, p99 latency recorded for comparison)
- 6 error types defined (embedding_provider, database, validation, timeout, not_found, unknown)
- Each error type has 1-2 actionable suggestions (may be generic for limited-context errors)
- Error details serialize/deserialize across Rust ↔ TypeScript boundary
- Type sync validation test passes (enum values match between Rust and TypeScript)
- MCP tool displays structured errors with context and suggestions
- Manual test: Embedding provider offline shows actionable error
- Backward compatibility verified (existing MCP clients work with new error format)

**Dependencies**: None (daemon infrastructure verified, ready for extension)

**Estimated Effort**: 4-5 tickets, 2-3 days

---

### Phase 2: Query Understanding Metadata

**Objective**: Add query understanding feedback to successful search responses. Show users how queries are interpreted, what filters were applied, and timing breakdown.

**Deliverables**:
- `QueryUnderstanding` structure in Rust
- `QueryFilters` and `TimingBreakdown` sub-structures
- Metadata assembly in search pipeline
- TypeScript interfaces mirroring Rust types
- Optional `understanding` field in search responses
- MCP tool query understanding display

**Agent Assignments**:
- **rust-engineer**: Extend `crates/maproom/src/search/results.rs` with `QueryUnderstanding`
- **rust-engineer**: Modify `crates/maproom/src/search/pipeline.rs` to assemble metadata
- **typescript-engineer**: Add `QueryUnderstanding` interface to `packages/daemon-client/src/types.ts`
- **typescript-engineer**: Update search response handling in maproom-mcp
- **unit-test-runner**: Validate metadata assembly
- **verify-ticket**: Confirm query understanding visible in responses

**Acceptance Criteria**:
- Every successful search returns `metadata.understanding`
- Understanding includes: mode, tokens, expanded_terms, filters, fusion_strategy, timing
- Timing breakdown shows: query_processing_ms, search_execution_ms, score_fusion_ms, result_assembly_ms, total_ms
- Manual test: Search for "authenticate user" shows mode=auto, tokens, expanded terms
- Performance: <10ms overhead measured against Phase 1 baseline (blocks merge if exceeded)
- Performance regression test passes (p95 latency remains <100ms)

**Dependencies**: Phase 1 (type sync patterns established, performance baseline measured)

**Estimated Effort**: 3-4 tickets, 1-2 days

---

### Phase 3: Refinement and Polish

**Objective**: Improve suggestion quality, add client-side validation enhancements, and measure success metrics.

**Deliverables**:
- Enhanced suggestions based on error context
- Improved Zod validation error messages
- Client-side query validation before RPC
- Performance metrics before/after comparison
- Documentation updates (CLAUDE.md, README)
- Success criteria validation

**Agent Assignments**:
- **rust-engineer**: Enhance suggestion logic with more context-specific recommendations
- **typescript-engineer**: Improve Zod validation messages in maproom-mcp
- **typescript-engineer**: Add client-side query validation
- **doc-writer**: Update CLAUDE.md with type sync documentation
- **verify-ticket**: Validate all success criteria met

**Acceptance Criteria**:
- 90% reduction in generic RPC_ERROR messages (measured via logs)
- Query understanding visible on 100% of successful searches
- At least 2 refinement suggestions per error
- Performance maintained: p95 <100ms
- All acceptance tests pass (embedding offline, repo not found, empty query, successful search)

**Dependencies**: Phase 1 + Phase 2 complete

**Estimated Effort**: 2-3 tickets, 1 day

---

## Phase Details

### Phase 1 Tickets Breakdown

**SRCHTRN-1000: Performance Baseline Measurement**
- Measure current search latency (p50, p95, p99) using existing Prometheus metrics
- Record query processing time breakdown
- Document baseline for Phase 2 comparison
- No code changes - measurement only

**SRCHTRN-1001: Rust Error Taxonomy**
- **First step**: Audit existing PipelineError types to verify context availability
- Create `crates/maproom/src/search/errors.rs`
- Define `SearchErrorDetails`, `ErrorType`, `PipelineStage` enums
- Implement `from_pipeline_error()` with pattern matching
- Add hardcoded suggestions for each error type (1-2 suggestions acceptable for limited-context errors)
- Unit tests for error conversion
- Note: May require minor error type refactoring if context insufficient

**SRCHTRN-1002: JSON-RPC Error Serialization**
- Modify `crates/maproom/src/daemon/server.rs` search handler
- Catch `PipelineError` and convert to `SearchErrorDetails`
- Serialize error details in JSON-RPC `data` field
- Preserve backward-compatible error message
- Integration test for error serialization

**SRCHTRN-1003: TypeScript Error Types**
- Create `packages/daemon-client/src/types.ts`
- Define `SearchErrorDetails`, `ErrorType`, `PipelineStage` interfaces
- Add sync comments linking to Rust source of truth
- Export types for use in daemon-client and maproom-mcp

**SRCHTRN-1004: TypeScript Error Deserialization**
- Extend `RpcError` class with `details` field
- Parse `data` field from JSON-RPC error responses
- Implement `getUserMessage()` helper
- Unit tests for deserialization

**SRCHTRN-1005: MCP Error Formatting**
- Update `formatSearchError()` in maproom-mcp
- Check for RpcError with details
- Format structured error response
- Fallback to existing error handling
- Integration test for end-to-end error flow

### Phase 2 Tickets Breakdown

**SRCHTRN-2001: Query Understanding Structures**
- Extend `crates/maproom/src/search/results.rs`
- Define `QueryUnderstanding`, `QueryFilters`, `TimingBreakdown`
- Add optional `understanding` field to `SearchMetadata`
- Unit tests for structure creation

**SRCHTRN-2002: Metadata Assembly in Pipeline**
- Modify `crates/maproom/src/search/pipeline.rs`
- Assemble `QueryUnderstanding` from `ProcessedQuery`
- Collect timing breakdown from existing timing data
- Populate filters from `SearchOptions`
- Integration test for metadata assembly

**SRCHTRN-2003: TypeScript Query Understanding Types**
- Add `QueryUnderstanding` interface to daemon-client types
- Mirror Rust structures with sync comments
- Update search response type to include optional `understanding`

**SRCHTRN-2004: MCP Query Understanding Display**
- Update maproom-mcp search tool to expose metadata
- Include understanding in successful search responses
- Format timing breakdown for readability
- Integration test for query understanding visibility

### Phase 3 Tickets Breakdown

**SRCHTRN-3001: Enhanced Error Suggestions**
- Improve suggestion logic based on error context
- Add provider-specific suggestions (OpenAI vs Ollama)
- Add database-specific suggestions
- Unit tests for suggestion generation

**SRCHTRN-3002: Client-Side Validation Enhancement**
- Enhance Zod error messages (not new validation rules - validation already exists)
- Improve error message clarity for existing pre-RPC validation
- Focus: message quality, not validation coverage
- Unit tests for improved messages

**SRCHTRN-3003: Documentation and Metrics**
- Update CLAUDE.md with type sync documentation
- Document error types and suggestions
- Collect before/after performance metrics
- Validate success criteria
- Create runbook for adding new error types

## Dependencies

### Internal Dependencies

**Cross-Phase Dependencies**:
- Phase 2 depends on Phase 1 (type sync patterns established)
- Phase 3 depends on Phase 1 + Phase 2 (all infrastructure in place)

**Within-Phase Dependencies**:
- Phase 1:
  - SRCHTRN-1003 depends on SRCHTRN-1001 (Rust types must exist first)
  - SRCHTRN-1004 depends on SRCHTRN-1003 (TypeScript types needed)
  - SRCHTRN-1005 depends on SRCHTRN-1004 (deserialization needed)
- Phase 2:
  - SRCHTRN-2003 depends on SRCHTRN-2001 (Rust types must exist first)
  - SRCHTRN-2004 depends on SRCHTRN-2002 + SRCHTRN-2003 (both Rust and TS ready)

### External Dependencies

- **Daemon Infrastructure**: Verified - all infrastructure exists and is ready for extension
  - `crates/maproom/src/daemon/mod.rs` - Request handling
  - `crates/maproom/src/daemon/types.rs` - Type definitions
  - Extension point identified (lines 143-151 in mod.rs)
- **SRCHFLTR Project**: Runs in parallel in Phase 1, no conflicts expected
- **Existing Search Pipeline**: Must not break backward compatibility
- **Prometheus Metrics**: Existing system used for performance validation
- **Zod Validation**: Existing validation enhanced, not replaced

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Type sync drift between Rust and TypeScript | Medium | High | Sync comments linking types, integration tests validating serialization, documentation in CLAUDE.md |
| Performance regression from metadata assembly | Low | Medium | Use existing in-memory data, measure before/after with Prometheus, <10ms budget |
| Breaking existing clients | Low | High | Additive changes only (optional fields), existing clients ignore unknown fields, backward compat tests |
| Over-engineering error framework | Medium | Low | MVP scope: 6 error types, pragmatic conversion functions, no abstraction layers |
| Suggestion quality insufficient | Low | Medium | Start with hardcoded strings, enhance in Phase 3 based on feedback |
| JSON-RPC spec violations | Low | High | Follow JSON-RPC 2.0 spec exactly, use `data` field for extensions, integration tests |

**Mitigation Actions**:

1. **Type Sync Drift**:
   - Action: Add sync comments to every type definition
   - Action: Integration test that serializes Rust → JSON → TypeScript
   - Action: CI check for type consistency (manual for MVP, can automate later)

2. **Performance Regression**:
   - Action: Measure baseline performance before Phase 2
   - Action: Measure after Phase 2 with Prometheus metrics
   - Action: Require p95 <100ms for release

3. **Breaking Changes**:
   - Action: Manual testing with existing MCP clients
   - Action: Verify existing error handling still works
   - Action: Document backward compatibility in PR

## Success Metrics

### Phase 1 Metrics

- [ ] 6 error types defined and tested
- [ ] Each error type has 2-3 actionable suggestions
- [ ] Error details serialize/deserialize correctly
- [ ] MCP tool displays structured errors
- [ ] Manual test: Embedding provider offline shows actionable error
- [ ] Manual test: Repository not found shows actionable error

### Phase 2 Metrics

- [ ] `QueryUnderstanding` structure implemented
- [ ] Every successful search returns metadata
- [ ] Timing breakdown accurate
- [ ] Performance overhead <10ms (measured)
- [ ] Manual test: Search shows query understanding

### Phase 3 Metrics

- [ ] 90% reduction in generic RPC_ERROR messages (measured in logs)
- [ ] Query understanding visible on 100% of successful searches
- [ ] At least 2 refinement suggestions per error
- [ ] Performance maintained: p95 <100ms
- [ ] All acceptance tests pass

### Overall Success Criteria

- [ ] **90% reduction in generic RPC_ERROR messages** - Measured by grepping logs for "RPC_ERROR" before/after
- [ ] **Query understanding visible on every search** - Verified by checking response structure
- [ ] **At least 2 refinement suggestions per failed query** - Validated in error conversion tests
- [ ] **Performance maintained <100ms p95** - Measured via Prometheus metrics
- [ ] **Backward compatibility preserved** - Tested with existing MCP clients
- [ ] **Type sync documented** - CLAUDE.md updated with sync patterns
- [ ] **All acceptance tests pass** - Embedding offline, repo not found, empty query, successful search

## Rollout Plan

### Phase 1 Rollout

1. Merge SRCHTRN-1001 to SRCHTRN-1003 (Rust + TypeScript types)
2. Test error serialization with unit tests
3. Merge SRCHTRN-1004 (TypeScript deserialization)
4. Merge SRCHTRN-1005 (MCP formatting)
5. Manual testing with various error scenarios
6. Deploy to production

### Phase 2 Rollout

1. Merge SRCHTRN-2001 (Rust query understanding structures)
2. Merge SRCHTRN-2002 (pipeline metadata assembly)
3. Performance testing (before/after comparison)
4. Merge SRCHTRN-2003 + SRCHTRN-2004 (TypeScript + MCP)
5. Manual testing with query understanding visibility
6. Deploy to production

### Phase 3 Rollout

1. Merge SRCHTRN-3001 (enhanced suggestions)
2. Merge SRCHTRN-3002 (client-side validation)
3. Collect metrics and validate success criteria
4. Merge SRCHTRN-3003 (documentation)
5. Final validation with all acceptance tests
6. Deploy to production

## Timeline Estimate

**Phase 1**: 3-4 days
- Day 1: Rust error taxonomy and JSON-RPC serialization (SRCHTRN-1001, SRCHTRN-1002)
- Day 2: TypeScript types and deserialization (SRCHTRN-1003, SRCHTRN-1004)
- Day 3: MCP formatting and integration testing (SRCHTRN-1005)
- Day 4: Manual testing and rollout

**Phase 2**: 2-3 days
- Day 1: Rust query understanding structures (SRCHTRN-2001, SRCHTRN-2002)
- Day 2: TypeScript and MCP integration (SRCHTRN-2003, SRCHTRN-2004)
- Day 3: Performance testing and rollout

**Phase 3**: 1-2 days
- Day 1: Enhanced suggestions and validation (SRCHTRN-3001, SRCHTRN-3002)
- Day 2: Documentation and metrics validation (SRCHTRN-3003)

**Total**: 6-9 days

**Note**: Timeline assumes parallel work with SRCHFLTR project in Phase 1 (foundation).
