# SRCHTRN Ticket Index

**Project**: Search Transparency
**Total Tickets**: 13 (6 Phase 1, 4 Phase 2, 3 Phase 3)
**Status**: Ready for execution
**Timeline**: 6-9 days

---

## Phase 1: Error Diagnostics Foundation (3-4 days)

**Objective**: Replace 90% of generic RPC_ERROR messages with actionable diagnostics.

### Tickets

1. **SRCHTRN-1000: Performance Baseline Measurement** (2-4 hours)
   - Measure current search latency (p50, p95, p99)
   - Record query processing time breakdown
   - Document baseline for Phase 2 comparison
   - **Dependencies**: None (first task)
   - **Agent**: general

2. **SRCHTRN-1001: Rust Error Taxonomy** (6-8 hours)
   - Create `SearchErrorDetails`, `ErrorType`, `PipelineStage` enums
   - Implement `from_pipeline_error()` conversion logic
   - Add hardcoded suggestions for 6 error types
   - Audit PipelineError for context availability
   - **Dependencies**: SRCHTRN-1000 (parallel work acceptable)
   - **Agent**: rust-engineer

3. **SRCHTRN-1002: JSON-RPC Error Serialization** (3-4 hours)
   - Modify daemon RPC handler to serialize error details
   - Serialize error details in JSON-RPC `data` field
   - Preserve backward-compatible error message
   - **Dependencies**: SRCHTRN-1001
   - **Agent**: rust-engineer

4. **SRCHTRN-1003: TypeScript Error Types** (2-3 hours)
   - Create `packages/daemon-client/src/types.ts` with mirrored error types
   - Add sync comments linking to Rust source of truth
   - Export types for use in daemon-client and maproom-mcp
   - **Dependencies**: SRCHTRN-1001
   - **Agent**: typescript-engineer

5. **SRCHTRN-1004: TypeScript Error Deserialization** (3-4 hours)
   - Extend `RpcError` class with `details` field
   - Parse `data` field from JSON-RPC error responses
   - Implement `getUserMessage()` helper
   - **Dependencies**: SRCHTRN-1003
   - **Agent**: typescript-engineer

6. **SRCHTRN-1005: MCP Error Formatting** (4-5 hours)
   - Update `formatSearchError()` in maproom-mcp
   - Format structured error response
   - Manual E2E testing (embedding offline, repo not found)
   - **Dependencies**: SRCHTRN-1004
   - **Agent**: typescript-engineer

**Phase 1 Total**: 20-28 hours (~3-4 days)

---

## Phase 2: Query Understanding Metadata (2-3 days)

**Objective**: Add query understanding feedback to successful searches.

### Tickets

7. **SRCHTRN-2001: Query Understanding Structures** (3-4 hours)
   - Extend `crates/maproom/src/search/results.rs` with `QueryUnderstanding`
   - Define `QueryFilters` and `TimingBreakdown` sub-structures
   - Add optional `understanding` field to `SearchMetadata`
   - **Dependencies**: Phase 1 complete
   - **Agent**: rust-engineer

8. **SRCHTRN-2002: Metadata Assembly in Pipeline** (6-8 hours)
   - Modify `crates/maproom/src/search/pipeline.rs` to assemble metadata
   - Collect timing breakdown from existing timing data
   - Populate filters from `SearchOptions`
   - Performance regression testing (<10ms overhead)
   - **Dependencies**: SRCHTRN-1000 (baseline), SRCHTRN-2001
   - **Agent**: rust-engineer

9. **SRCHTRN-2003: TypeScript Query Understanding Types** (2-3 hours)
   - Add `QueryUnderstanding` interface to daemon-client types
   - Mirror Rust structures with sync comments
   - Update search response type to include optional `understanding`
   - **Dependencies**: SRCHTRN-2001
   - **Agent**: typescript-engineer

10. **SRCHTRN-2004: MCP Query Understanding Display** (3-4 hours)
    - Update maproom-mcp search tool to expose metadata
    - Include understanding in successful search responses
    - Format timing breakdown for readability
    - Manual E2E testing
    - **Dependencies**: SRCHTRN-2002, SRCHTRN-2003
    - **Agent**: typescript-engineer

**Phase 2 Total**: 14-19 hours (~2-3 days)

---

## Phase 3: Refinement and Polish (1-2 days)

**Objective**: Enhance suggestions, validate success criteria, finalize documentation.

### Tickets

11. **SRCHTRN-3001: Enhanced Error Suggestions** (4-5 hours)
    - Improve suggestion logic based on error context
    - Add provider-specific suggestions (OpenAI vs Ollama)
    - Add database-specific suggestions
    - **Dependencies**: Phase 1 + Phase 2 complete
    - **Agent**: rust-engineer

12. **SRCHTRN-3002: Client Validation Improvements** (2-3 hours)
    - Enhance Zod error messages (not new validation rules)
    - Improve error message clarity for existing pre-RPC validation
    - **Dependencies**: Phase 1 + Phase 2 complete
    - **Agent**: typescript-engineer

13. **SRCHTRN-3003: Documentation and Metrics** (4-6 hours)
    - Update CLAUDE.md with type sync documentation
    - Document error types and suggestions
    - Collect before/after performance metrics
    - Validate success criteria (90% RPC_ERROR reduction, etc.)
    - Create runbook for adding new error types
    - **Dependencies**: All tickets complete
    - **Agent**: general

**Phase 3 Total**: 10-14 hours (~1-2 days)

---

## Dependency Graph

```
SRCHTRN-1000 (baseline)
    ↓
SRCHTRN-1001 (Rust errors) ──────────────────┐
    ↓                                         ↓
SRCHTRN-1002 (JSON-RPC)    SRCHTRN-1003 (TS types)
                                ↓
                           SRCHTRN-1004 (TS deser)
                                ↓
                           SRCHTRN-1005 (MCP format)
                                ↓
                           [Phase 1 Complete]
                                ↓
SRCHTRN-1000 ──────┐       SRCHTRN-2001 (QU structs)
                   ↓            ↓
              SRCHTRN-2002 (metadata)    SRCHTRN-2003 (TS QU types)
                        ↘       ↙
                     SRCHTRN-2004 (MCP display)
                            ↓
                     [Phase 2 Complete]
                            ↓
                ┌───────────┴───────────┐
                ↓           ↓           ↓
          SRCHTRN-3001  SRCHTRN-3002  SRCHTRN-3003
          (suggestions) (validation)  (docs)
                ↘           ↓           ↙
                     [Project Complete]
```

---

## Execution Order

### Sequential Path (Critical)
1. SRCHTRN-1000 → Baseline measurement
2. SRCHTRN-1001 → Rust error taxonomy
3. SRCHTRN-1002 → JSON-RPC serialization
4. SRCHTRN-1003 → TypeScript error types
5. SRCHTRN-1004 → TypeScript deserialization
6. SRCHTRN-1005 → MCP error formatting
7. SRCHTRN-2001 → Query understanding structures
8. SRCHTRN-2002 → Metadata assembly (depends on baseline)
9. SRCHTRN-2003 → TypeScript query understanding types
10. SRCHTRN-2004 → MCP query understanding display
11. SRCHTRN-3001 → Enhanced error suggestions
12. SRCHTRN-3002 → Client validation improvements
13. SRCHTRN-3003 → Documentation and metrics validation

### Parallel Opportunities
- **Phase 1**: SRCHTRN-1003 and SRCHTRN-1002 can run in parallel after SRCHTRN-1001
- **Phase 2**: SRCHTRN-2003 can start when SRCHTRN-2001 completes (parallel with SRCHTRN-2002)
- **Phase 3**: SRCHTRN-3001 and SRCHTRN-3002 can run in parallel

---

## Quality Gates

### Phase 1 Complete
- [ ] All 6 error types tested end-to-end
- [ ] At least 2 manual E2E scenarios validated
- [ ] Error serialization integration test passes
- [ ] Type sync validation passes
- [ ] Backward compatibility verified (existing MCP clients work)

### Phase 2 Complete
- [ ] Query understanding integration test passes
- [ ] Manual E2E shows metadata in response
- [ ] Performance overhead <10ms vs baseline
- [ ] Timing data accuracy validated
- [ ] p95 latency <100ms

### Phase 3 Complete
- [ ] All acceptance tests pass (4 scenarios)
- [ ] Success criteria validated (90% RPC_ERROR reduction, etc.)
- [ ] Performance regression test passes
- [ ] Documentation complete
- [ ] Runbook tested

---

## Success Criteria

### Quantitative
- **90% reduction in generic RPC_ERROR messages** (measured via logs)
- **Query understanding visible on 100% of successful searches**
- **At least 2 refinement suggestions per failed query**
- **Performance maintained: p95 <100ms** (measured via Prometheus)

### Qualitative
- Error messages are actionable (user knows what to do)
- Query understanding is clear (user sees how query was interpreted)
- No debugging friction (developers can diagnose from error alone)

### Acceptance Tests
1. **Embedding provider offline** → Error identifies provider, suggests FTS mode
2. **Repository not found** → Error names repo, suggests status/scan
3. **Empty query** → Caught by Zod validation before RPC
4. **Successful search** → Metadata shows tokens, mode, timing

---

## Scoping Notes

### Tickets Potentially >8 Hours
- **SRCHTRN-1001** (6-8 hours): May exceed if significant error type refactoring needed
- **SRCHTRN-2002** (6-8 hours): May exceed if performance optimization required

**Monitoring**: Flag if tickets approach 8-hour limit. Consider splitting if needed.

### Risk Mitigations
- **Type sync**: Manual audit checklist + validation tests
- **Performance**: Baseline measurement + regression testing
- **Backward compatibility**: Integration tests + manual testing

---

## Agent Assignments

### By Phase
- **Phase 1**: rust-engineer (3 tickets), typescript-engineer (3 tickets), general (1 ticket)
- **Phase 2**: rust-engineer (2 tickets), typescript-engineer (2 tickets)
- **Phase 3**: rust-engineer (1 ticket), typescript-engineer (1 ticket), general (1 ticket)

### Total Agent Workload
- **rust-engineer**: 6 tickets (~30 hours)
- **typescript-engineer**: 6 tickets (~24 hours)
- **general**: 2 tickets (~10 hours)

**Note**: All tickets include verify-ticket and commit-ticket agents.

---

## Timeline Summary

**Optimistic**: 6 days (20 + 14 + 10 = 44 hours ÷ 8 hours/day = 5.5 days)
**Realistic**: 7-8 days (accounting for testing, debugging, reviews)
**Pessimistic**: 9 days (if performance optimization or error refactoring needed)

**Target**: 6-9 days total execution time
