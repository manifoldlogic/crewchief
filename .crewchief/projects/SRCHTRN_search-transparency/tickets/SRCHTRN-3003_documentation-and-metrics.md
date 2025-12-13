# SRCHTRN-3003: Documentation and Metrics Validation

## Title
Update documentation and validate success criteria

## Status
- [ ] **Implementation Complete**
- [ ] **Tests Passing**
- [ ] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: general
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Update CLAUDE.md with type sync documentation, document error types and suggestions, collect before/after performance metrics, validate all success criteria, and create runbook for adding new error types.

## Background
Project implementation complete - Phase 3 finalizes with comprehensive documentation and success criteria validation. This ensures future maintainers can extend the system and that the project achieved its goals.

## Acceptance Criteria
- [ ] CLAUDE.md updated with type sync patterns and manual audit checklist
- [ ] Error types and suggestions documented in daemon-client README
- [ ] Performance metrics collected (before/after comparison)
- [ ] 90% reduction in generic RPC_ERROR messages validated (log analysis)
- [ ] Query understanding visible on 100% of successful searches validated
- [ ] Performance maintained: p95 <100ms validated
- [ ] All 4 acceptance tests passing (embedding offline, repo not found, empty query, successful search)
- [ ] Runbook created for adding new error types

## Technical Requirements

### 1. Update CLAUDE.md

**File**: `packages/daemon-client/CLAUDE.md`

Add section on type synchronization:

```markdown
## Type Synchronization with Rust

**Source of Truth**: Rust types in `crates/maproom/src/search/errors.rs` and `crates/maproom/src/search/results.rs`

**Sync Pattern**: TypeScript types in `src/types.ts` mirror Rust structs with sync comments.

### Manual Sync Checklist

When Rust types change:
- [ ] Update corresponding TypeScript interfaces
- [ ] Verify sync comments still link correctly
- [ ] Run type sync validation tests: `pnpm test types.test.ts`
- [ ] Check integration tests pass

### Type Sync Validation

Run validation tests:
```bash
cd packages/daemon-client
pnpm test types.test.ts
```

Tests verify:
- Enum values match exactly (ErrorType, PipelineStage)
- Structure fields match (SearchErrorDetails, QueryUnderstanding)
- Serialization roundtrip works

### Adding New Error Types

1. **Rust** (`crates/maproom/src/search/errors.rs`):
   - Add variant to `ErrorType` enum
   - Add conversion case in `from_pipeline_error()`
   - Add 1-2 actionable suggestions

2. **TypeScript** (`packages/daemon-client/src/types.ts`):
   - Add variant to `ErrorType` union type
   - Update sync comment if needed

3. **Validation** (`packages/daemon-client/src/types.test.ts`):
   - Add new variant to validation test array
   - Verify test passes

4. **Integration Test** (`crates/maproom/tests/daemon_error_serialization.rs`):
   - Add test case for new error type
   - Verify serialization works end-to-end
```

### 2. Document Error Types

**File**: `packages/daemon-client/README.md`

Add section:

```markdown
## Error Handling

### Structured Error Details

When RPC errors occur, structured error details are available via `RpcError.getDetails()`:

```typescript
import { RpcError } from '@crewchief/daemon-client'

try {
  await client.search({ query: 'test', repo: 'crewchief' })
} catch (error) {
  if (error instanceof RpcError) {
    const details = error.getDetails()
    if (details) {
      console.log(`Error type: ${details.error_type}`)
      console.log(`Stage: ${details.stage}`)
      console.log(`Suggestions:`, details.suggestions)
    }
  }
}
```

### Error Types

| Error Type | Description | Common Causes |
|------------|-------------|---------------|
| `embedding_provider` | Embedding service failure | API key invalid, service offline, network issues |
| `database` | Database operation failure | Repository not indexed, connection timeout, corrupted DB |
| `validation` | Invalid query parameters | Empty query, query too long |
| `timeout` | Search execution timeout | Complex query, large repository |
| `not_found` | Resource not found | Repository doesn't exist, no meaningful content |
| `unknown` | Unexpected error | Internal errors, unclassified failures |

### Suggestions

Each error includes 1-2 actionable suggestions:
- Provider-specific fixes (e.g., "Start Ollama service: ollama serve")
- Fallback modes (e.g., "Try FTS mode: --mode fts")
- Configuration checks (e.g., "Check OPENAI_API_KEY")
```

### 3. Performance Metrics Collection

**File**: `.crewchief/projects/SRCHTRN_search-transparency/planning/performance-results.md`

```markdown
# Performance Results - SRCHTRN Project

## Before Implementation (Phase 1 Baseline)
**Date**: [from SRCHTRN-1000]
- p50: XX.Xms
- p95: XX.Xms
- p99: XXX.Xms

## After Implementation (Phase 2 Complete)
**Date**: YYYY-MM-DD
- p50: XX.Xms
- p95: XX.Xms
- p99: XXX.Xms

## Analysis
- **Overhead**: X.Xms (within 10ms budget: ✓/✗)
- **p95 Target**: <100ms (achieved: ✓/✗)
- **Metadata Assembly Time**: X.Xms

## Conclusion
[Performance impact assessment]
```

### 4. Success Criteria Validation

**File**: `.crewchief/projects/SRCHTRN_search-transparency/planning/success-validation.md`

```markdown
# Success Criteria Validation - SRCHTRN Project

**Validation Date**: YYYY-MM-DD

## Quantitative Metrics

### 90% Reduction in Generic RPC_ERROR Messages
**Method**: Grep daemon logs for "RPC_ERROR" before/after
**Before**: X occurrences in 1000 requests
**After**: Y occurrences in 1000 requests
**Reduction**: Z% (target: 90%)
**Status**: ✓/✗

### Query Understanding on 100% of Successful Searches
**Method**: Sample 100 successful searches, check for metadata.understanding
**Sample Size**: 100 searches
**With Understanding**: X/100
**Status**: ✓/✗

### At Least 2 Suggestions Per Error
**Method**: Review error conversion logic and tests
**Error Types Checked**: 6 (all types)
**With 2+ Suggestions**: X/6
**Status**: ✓/✗

### Performance <100ms p95
**Method**: Prometheus metrics
**Measured p95**: XX.Xms
**Target**: <100ms
**Status**: ✓/✗

## Acceptance Tests

### 1. Embedding Provider Offline
**Test**: Stop Ollama, run vector search
**Expected**: Error shows "embedding_provider", suggests FTS mode
**Result**: ✓/✗
**Notes**: [observations]

### 2. Repository Not Found
**Test**: Search non-existent repo
**Expected**: Error shows repo name, suggests status/scan
**Result**: ✓/✗
**Notes**: [observations]

### 3. Empty Query
**Test**: Submit empty query
**Expected**: Zod validation error before RPC
**Result**: ✓/✗
**Notes**: [observations]

### 4. Successful Search with Understanding
**Test**: Search "authenticate user"
**Expected**: Metadata shows tokens, mode, timing
**Result**: ✓/✗
**Notes**: [observations]

## Overall Assessment
[Summary of validation results]
```

### 5. Runbook for Adding Error Types

**File**: `docs/runbooks/adding-error-types.md`

```markdown
# Runbook: Adding New Error Types

## Prerequisites
- Familiarity with Rust error handling
- Understanding of JSON-RPC serialization
- TypeScript type synchronization patterns

## Steps

### 1. Identify Error Scenario
- [ ] Document error scenario (what causes it)
- [ ] Determine if it maps to existing ErrorType or needs new one
- [ ] Identify available context for suggestions

### 2. Update Rust Error Taxonomy
**File**: `crates/maproom/src/search/errors.rs`

- [ ] Add variant to `ErrorType` enum (if new type)
- [ ] Add conversion case in `from_pipeline_error()`
- [ ] Extract context from error
- [ ] Generate 1-2 actionable suggestions
- [ ] Add unit test for new error type

### 3. Update TypeScript Types
**File**: `packages/daemon-client/src/types.ts`

- [ ] Add variant to `ErrorType` union type
- [ ] Verify sync comment links to Rust source
- [ ] Add to type sync validation test

### 4. Integration Testing
- [ ] Add integration test in `crates/maproom/tests/`
- [ ] Verify error serializes correctly
- [ ] Test with real daemon and MCP client

### 5. Documentation
- [ ] Update error type table in README
- [ ] Document common causes and suggestions
- [ ] Add example to documentation

## Example: Adding "RateLimitExceeded" Error

1. Rust enum:
```rust
pub enum ErrorType {
    // ... existing variants
    RateLimitExceeded,
}
```

2. Conversion logic:
```rust
EmbeddingError::RateLimit { retry_after } => Self {
    error_type: ErrorType::RateLimitExceeded,
    stage: PipelineStage::QueryProcessing,
    context: HashMap::from([
        ("retry_after".to_string(), retry_after.to_string()),
    ]),
    suggestions: vec![
        format!("Retry after {} seconds", retry_after),
        "Check API quota limits".to_string(),
    ],
}
```

3. TypeScript type:
```typescript
export type ErrorType =
  | 'embedding_provider'
  // ... existing types
  | 'rate_limit_exceeded'
```

4. Test:
```rust
#[test]
fn test_rate_limit_error() {
    let error = PipelineError::QueryProcessing(
        QueryProcessorError::Embedding(
            EmbeddingError::RateLimit { retry_after: 60 }
        )
    );

    let details = SearchErrorDetails::from_pipeline_error(&error);

    assert_eq!(details.error_type, ErrorType::RateLimitExceeded);
    assert!(details.suggestions.iter().any(|s| s.contains("60")));
}
```
```

## Implementation Notes
1. Review Phase 1 baseline and Phase 2 metrics
2. Run log analysis for RPC_ERROR reduction
3. Sample 100 searches for query understanding validation
4. Execute all 4 acceptance tests manually
5. Create comprehensive documentation

**Log Analysis Command**:
```bash
# Before implementation
grep "RPC_ERROR" daemon.log.before | wc -l

# After implementation
grep "RPC_ERROR" daemon.log.after | wc -l
```

## Dependencies
**All Phase 1, 2, 3 tickets complete**: Final validation and documentation task

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Metrics may not meet targets
- Documentation may be incomplete

**Mitigations**:
- Thorough testing throughout phases
- Success criteria tracked from Phase 1
- Comprehensive runbook for future maintainers

## Files/Packages Affected
- **Modified**: `packages/daemon-client/CLAUDE.md` (type sync documentation)
- **Modified**: `packages/daemon-client/README.md` (error handling guide)
- **New file**: `.crewchief/projects/SRCHTRN_search-transparency/planning/performance-results.md`
- **New file**: `.crewchief/projects/SRCHTRN_search-transparency/planning/success-validation.md`
- **New file**: `docs/runbooks/adding-error-types.md`

## Estimated Effort
4-6 hours

**Breakdown**:
- CLAUDE.md updates: 1-2 hours
- README documentation: 1 hour
- Metrics collection and analysis: 1-2 hours
- Acceptance test execution: 1-2 hours
- Runbook creation: 1 hour

## Planning References
- [plan.md](../planning/plan.md) - Phase 3 ticket breakdown, success criteria
- [quality-strategy.md](../planning/quality-strategy.md) - Manual testing checklist
- [project-review.md](../planning/project-review.md) - Success probability assessment
