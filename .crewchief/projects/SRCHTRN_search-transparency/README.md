# Project: Search Transparency

**Slug:** SRCHTRN
**Status:** Planning
**Created:** 2025-12-13
**Initiative:** 2025-12-09_maproom-semantic-search-improvements
**Priority:** Highest (Phase 1 - Foundation)

## Summary

Replace generic "RPC_ERROR: Search failed" messages with structured, actionable error diagnostics and add query understanding feedback to maproom's semantic search pipeline.

**User Problem**: During a recent user session, 2 search failures occurred with only generic error messages - no indication of what went wrong, why it failed, or how to fix it. Users can't debug failures or understand how their queries are interpreted.

**Solution**:
1. **Structured Error Diagnostics**: Return detailed error information (error type, stage, context, suggestions)
2. **Query Understanding Feedback**: Show how queries are interpreted (tokens, mode, expanded terms, timing)
3. **Actionable Suggestions**: Provide 2-3 refinement suggestions per error

**Impact**: 90% reduction in generic errors, query understanding visible on every search, performance maintained <100ms p95.

## Problem Statement

Maproom's search returns generic "RPC_ERROR" messages when errors occur, providing no debugging information or guidance. Error context is lost during JSON-RPC serialization from Rust to TypeScript.

**Pain Points**:
- Opaque errors with no context about failure cause
- No visibility into query interpretation (mode, tokens, expanded terms)
- No actionable guidance for query refinement
- Error messages don't explain what stage failed

**Example Current Error**:
```
Search failed: RPC_ERROR
```

**Example Desired Error**:
```
Search failed at query_processing: Embedding generation failed: request timeout

Context:
  provider_error: request timeout

Suggestions:
  - Check your embedding provider credentials
  - Verify network connectivity
  - Try FTS mode while debugging: --mode fts
```

## Proposed Solution

### 1. Structured Error Taxonomy (Rust)

Create `SearchErrorDetails` with:
- `error_type`: enum (embedding_provider, database, validation, timeout, not_found, unknown)
- `stage`: enum (query_processing, search_execution, score_fusion, result_assembly)
- `context`: HashMap with error-specific details
- `suggestions`: Vec of actionable strings

Convert `PipelineError` → `SearchErrorDetails` → JSON-RPC error `data` field.

### 2. Query Understanding Metadata (Rust)

Create `QueryUnderstanding` with:
- `mode`: SearchMode (code, text, auto)
- `tokens`: Vec<String> (from tokenizer)
- `expanded_terms`: Vec<String> (from synonym expansion)
- `filters`: QueryFilters (repo, worktree, file types)
- `fusion_strategy`: String (e.g., "reciprocal_rank_fusion")
- `timing`: TimingBreakdown (query_processing_ms, search_execution_ms, etc.)

Add optional `understanding` field to `SearchMetadata` in successful responses.

### 3. TypeScript Deserialization (daemon-client)

- Mirror Rust types with sync comments
- Extend `RpcError` with `details?: SearchErrorDetails`
- Add `getUserMessage()` helper for formatted errors
- No new dependencies

### 4. MCP Error Formatting (maproom-mcp)

- Update `formatSearchError()` to use structured details
- Display query understanding in successful responses
- Fallback to existing error handling for backward compat

## Architecture Highlights

**Key Design Decisions**:
- Additive API changes only (backward compatible)
- Rust is single source of truth for types
- No over-engineering (6 error types, string suggestions)
- Performance first (<10ms overhead budget)

**Data Flow**:
```
Rust PipelineError
  → SearchErrorDetails
  → JSON-RPC error.data field
  → TypeScript RpcError.details
  → MCP formatted error
  → User sees actionable error
```

**Performance**:
- Error conversion: ~0.5ms (pattern matching)
- Metadata assembly: ~1ms (copying existing data)
- JSON serialization: ~2ms (serde)
- Total overhead: ~3.5ms << 10ms budget ✓

## Relevant Agents

- **project-planner** - Planning phase (current)
- **rust-engineer** - Error taxonomy, metadata assembly, daemon RPC handler
- **typescript-engineer** - TypeScript types, error deserialization, MCP formatting
- **unit-test-runner** - Execute tests
- **verify-ticket** - Verify acceptance criteria
- **commit-ticket** - Create commits

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis, research findings, constraints
- [architecture.md](planning/architecture.md) - Solution design, component responsibilities
- [plan.md](planning/plan.md) - Phased execution plan (3 phases, 6-9 days)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach (pragmatic confidence over coverage)
- [security-review.md](planning/security-review.md) - Security assessment (low risk, no sensitive data)

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

## Execution Plan (3 Phases)

### Phase 1: Error Diagnostics Foundation (3-4 days)
**Objective**: Replace 90% of generic RPC_ERROR messages with actionable diagnostics.

**Tickets**:
- SRCHTRN-1001: Rust error taxonomy
- SRCHTRN-1002: JSON-RPC error serialization
- SRCHTRN-1003: TypeScript error types
- SRCHTRN-1004: TypeScript error deserialization
- SRCHTRN-1005: MCP error formatting

**Dependencies**: None (foundation work)

### Phase 2: Query Understanding Metadata (2-3 days)
**Objective**: Add query understanding feedback to successful searches.

**Tickets**:
- SRCHTRN-2001: Query understanding structures
- SRCHTRN-2002: Metadata assembly in pipeline
- SRCHTRN-2003: TypeScript query understanding types
- SRCHTRN-2004: MCP query understanding display

**Dependencies**: Phase 1 (type sync patterns established)

### Phase 3: Refinement and Polish (1-2 days)
**Objective**: Enhance suggestions, validate success criteria.

**Tickets**:
- SRCHTRN-3001: Enhanced error suggestions
- SRCHTRN-3002: Client-side validation improvements
- SRCHTRN-3003: Documentation and metrics validation

**Dependencies**: Phase 1 + Phase 2 complete

**Total Timeline**: 6-9 days

## Technical Constraints

- **Performance Budget**: <10ms overhead per search
- **No Schema Changes**: Cannot modify database tables
- **Backward Compatibility**: Additive API changes only
- **Type Sync**: TypeScript ↔ Rust type alignment required
- **Client-Side Validation**: Zod schemas catch errors before RPC

## Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Type sync drift between Rust and TypeScript | Medium | High | Sync comments, integration tests, documentation |
| Performance regression | Low | Medium | Use existing data, measure with Prometheus, <10ms budget |
| Breaking existing clients | Low | High | Additive changes only, backward compat tests |
| Over-engineering | Medium | Low | MVP scope: 6 error types, pragmatic approach |

## Next Steps

**Immediate**:
1. Run `/review-project SRCHTRN` to validate planning
2. Address any planning gaps identified in review
3. Run `/create-project-tickets SRCHTRN` to generate execution tickets

**After Planning Validation**:
1. Begin Phase 1: Error Diagnostics Foundation
2. Execute tickets in order (dependencies noted in plan.md)
3. Manual testing after each phase
4. Metrics validation before deployment

## Agent Recommendation

**Assessment**: Custom specialized agents **not needed** for this project.

**Rationale**:
- Straightforward error handling and metadata assembly
- No complex specialized domains
- General rust-engineer and typescript-engineer skills are sufficient
- Clear type sync patterns make cross-language work manageable

**Conclusion**: Proceed with general agents. Standard agent skills cover this project well.
