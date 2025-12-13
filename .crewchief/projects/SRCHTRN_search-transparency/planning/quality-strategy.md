# Quality Strategy: Search Transparency

## Testing Philosophy

**Pragmatic Confidence Over Coverage**: Test for confidence that errors are actionable and query understanding is accurate, not for coverage numbers.

**Core Principles**:
1. Test critical paths (error serialization, type sync)
2. Test each error type at least once end-to-end
3. Validate performance doesn't regress
4. Skip testing internal implementation details
5. Focus on user-facing outcomes

**Non-Goals**:
- 100% code coverage
- Testing every possible error combination
- Mocking everything for pure unit tests
- Complex test fixtures

## Test Types

### Unit Tests

**Scope**:
- Error conversion logic (`from_pipeline_error()`)
- Suggestion generation for each error type
- Metadata assembly from ProcessedQuery
- TypeScript deserialization

**Tools**:
- Rust: `cargo test` with `#[test]`
- TypeScript: Jest with `@crewchief/*` packages

**Coverage Target**: Critical logic only (error conversion, metadata assembly)

**Examples**:

**Rust Unit Tests** (`crates/maproom/src/search/errors.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_provider_error_conversion() {
        let pipeline_error = PipelineError::QueryProcessing(
            QueryProcessorError::Embedding(EmbeddingError::Timeout)
        );

        let details = SearchErrorDetails::from_pipeline_error(&pipeline_error);

        assert_eq!(details.error_type, ErrorType::EmbeddingProvider);
        assert_eq!(details.stage, PipelineStage::QueryProcessing);
        assert!(details.suggestions.len() >= 2);
        assert!(details.suggestions.iter().any(|s| s.contains("FTS mode")));
    }

    #[test]
    fn test_database_error_conversion() {
        let pipeline_error = PipelineError::Database(
            "Connection timeout".to_string()
        );

        let details = SearchErrorDetails::from_pipeline_error(&pipeline_error);

        assert_eq!(details.error_type, ErrorType::Database);
        assert_eq!(details.stage, PipelineStage::SearchExecution);
        assert!(details.context.contains_key("message"));
        assert!(details.suggestions.len() >= 2);
    }

    #[test]
    fn test_all_error_types_have_suggestions() {
        let error_types = vec![
            PipelineError::QueryProcessing(QueryProcessorError::EmptyQuery),
            PipelineError::Database("test".to_string()),
            PipelineError::Assembly("test".to_string()),
        ];

        for error in error_types {
            let details = SearchErrorDetails::from_pipeline_error(&error);
            assert!(
                details.suggestions.len() >= 1,
                "Error type {:?} missing suggestions",
                details.error_type
            );
        }
    }
}
```

**TypeScript Unit Tests** (`packages/daemon-client/src/rpc.test.ts`):
```typescript
import { RpcError } from './rpc.js'
import type { SearchErrorDetails } from './types.js'

describe('RpcError', () => {
  it('should parse error details from data field', () => {
    const errorData: SearchErrorDetails = {
      error_type: 'embedding_provider',
      stage: 'query_processing',
      context: { provider_error: 'timeout' },
      suggestions: ['Check credentials', 'Try FTS mode'],
    }

    const error = new RpcError('Test error', -32000, errorData)

    expect(error.getDetails()).toEqual(errorData)
  })

  it('should format user message with context and suggestions', () => {
    const errorData: SearchErrorDetails = {
      error_type: 'database',
      stage: 'search_execution',
      context: { message: 'Connection failed' },
      suggestions: ['Check database connectivity'],
    }

    const error = new RpcError('Database error', -32000, errorData)
    const message = error.getUserMessage()

    expect(message).toContain('search_execution')
    expect(message).toContain('Connection failed')
    expect(message).toContain('Check database connectivity')
  })

  it('should handle missing error details gracefully', () => {
    const error = new RpcError('Generic error', -32000)

    expect(error.getDetails()).toBeUndefined()
    expect(error.getUserMessage()).toEqual('Generic error')
  })
})
```

### Integration Tests

**Scope**:
- End-to-end error serialization (Rust → JSON → TypeScript)
- Query understanding metadata in search responses
- Type consistency validation (Rust types serialize to expected TypeScript types)
- Performance regression detection

**Infrastructure**:
- **Rust Tests**: `crates/maproom/tests/search_transparency.rs` (new file)
- **TypeScript Tests**: `packages/maproom-mcp/tests/search-error-handling.test.ts`
- **Run Commands**: `cargo test -p crewchief-maproom` (Rust), `pnpm test` in maproom-mcp (TypeScript)
- **Test Data**: In-memory SQLite for Rust tests, mock daemon responses for TypeScript tests

**Approach**:
- Rust integration tests that call search pipeline and check JSON output
- TypeScript tests that deserialize real JSON from Rust
- Performance benchmarks before/after (using existing Prometheus metrics)

**Examples**:

**Rust Integration Test** (`crates/maproom/tests/search_transparency.rs`):
```rust
#[tokio::test]
async fn test_error_serialization_embedding_failure() {
    // Setup: Create search pipeline with mock embedding service that fails
    let pipeline = create_test_pipeline_with_failing_embeddings();

    // Execute: Trigger search that requires embeddings
    let result = pipeline.search("test query", SearchOptions {
        mode: SearchMode::Vector,
        // ...
    }).await;

    // Assert: Error is properly structured
    assert!(result.is_err());
    let error = result.unwrap_err();

    // Serialize to JSON (simulating RPC response)
    let error_details = SearchErrorDetails::from_pipeline_error(&error);
    let json = serde_json::to_value(&error_details).unwrap();

    // Verify JSON structure
    assert_eq!(json["error_type"], "embedding_provider");
    assert_eq!(json["stage"], "query_processing");
    assert!(json["suggestions"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn test_query_understanding_in_response() {
    let pipeline = create_test_pipeline();

    let result = pipeline.search("authenticate user", SearchOptions {
        repo_id: 1,
        // ...
    }).await.unwrap();

    // Verify metadata includes understanding
    assert!(result.metadata.understanding.is_some());
    let understanding = result.metadata.understanding.unwrap();

    assert_eq!(understanding.tokens, vec!["authenticate", "user"]);
    assert!(understanding.expanded_terms.contains(&"auth".to_string()));
    assert!(understanding.timing.total_ms > 0.0);
}
```

**TypeScript Integration Test** (`packages/maproom-mcp/tests/search-error-handling.test.ts`):
```typescript
import { handleSearchTool } from '../src/tools/search.js'

describe('Search error handling integration', () => {
  it('should handle embedding provider failure', async () => {
    // Setup: Mock daemon that returns error with details
    mockDaemonError({
      code: -32000,
      message: 'Embedding generation failed',
      data: {
        error_type: 'embedding_provider',
        stage: 'query_processing',
        context: { provider_error: 'timeout' },
        suggestions: ['Check credentials', 'Try FTS mode'],
      },
    })

    // Execute
    const result = await handleSearchTool({
      query: 'test',
      repo: 'crewchief',
      mode: 'vector',
    }, mockClient)

    // Assert: Error formatted correctly
    expect(result.isError).toBe(true)
    const errorText = JSON.parse(result.content[0].text)
    expect(errorText.error).toBe('embedding_provider')
    expect(errorText.suggestions).toHaveLength(2)
  })
})
```

### End-to-End Tests

**Scope**: Critical paths only - 4 key scenarios

**Approach**: Manual testing with real daemon, real database, real MCP client

**Critical Scenarios**:

1. **Embedding Provider Offline**
   - Setup: Stop Ollama service or invalidate API key
   - Action: Run search with vector mode
   - Expected: Error message identifies provider, suggests FTS mode

2. **Repository Not Found**
   - Setup: Search for non-existent repo
   - Action: Run search
   - Expected: Error message names repo, suggests `status` and `scan`

3. **Empty Query**
   - Setup: None
   - Action: Send empty query string
   - Expected: Zod validation catches before RPC, clear error message

4. **Successful Search with Understanding**
   - Setup: Indexed repo with embeddings
   - Action: Run search "authenticate user"
   - Expected: Results include metadata.understanding with tokens, mode, timing

**E2E Test Checklist**:
```bash
# 1. Embedding provider offline
export OPENAI_API_KEY=invalid
npx @crewchief/maproom-mcp
# → Search → Verify error shows "embedding_provider", suggests FTS mode

# 2. Repository not found
npx @crewchief/maproom-mcp
# → Search repo="nonexistent" → Verify error shows repo name, suggests status/scan

# 3. Empty query
npx @crewchief/maproom-mcp
# → Search query="" → Verify Zod validation error before RPC

# 4. Successful search
npx @crewchief/maproom-mcp
# → Search query="authenticate user" → Verify metadata.understanding exists
```

## Critical Paths

The following paths MUST be tested (automated or manual):

### 1. Error Conversion Path

**Flow**: PipelineError → SearchErrorDetails → JSON-RPC error → TypeScript RpcError → MCP formatted error

**Test Coverage**:
- Each ErrorType variant (6 total)
- Each PipelineStage variant (4 total)
- Suggestions present for each error
- Context extracted correctly

**Validation**: Integration test + manual E2E for at least 2 error types

### 2. Query Understanding Path

**Flow**: ProcessedQuery → QueryUnderstanding → SearchMetadata → JSON response → TypeScript → MCP display

**Test Coverage**:
- Metadata assembly from ProcessedQuery
- Timing data accuracy
- Filters populated correctly
- Optional field handling

**Validation**: Integration test + manual E2E

### 3. Type Synchronization Path

**Flow**: Rust struct → serde serialize → JSON → TypeScript deserialize → typed object

**Test Coverage**:
- SearchErrorDetails serialization roundtrip
- QueryUnderstanding serialization roundtrip
- Enum values match exactly
- Optional fields handled

**Validation Strategy**: Integration test + manual audit (automated codegen deferred to future)

**Type Sync Validation Test** (`packages/daemon-client/src/types.test.ts`):
```typescript
describe('Type synchronization with Rust', () => {
  // Sync with: crates/maproom/src/search/errors.rs::ErrorType
  it('should match Rust ErrorType enum values', () => {
    const rustErrorTypes = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown'
    ]

    // This will fail to compile if TypeScript ErrorType diverges
    const tsErrorTypes: ErrorType[] = [
      'embedding_provider',
      'database',
      'validation',
      'timeout',
      'not_found',
      'unknown'
    ]

    expect(rustErrorTypes).toEqual(tsErrorTypes)
  })

  // Sync with: crates/maproom/src/search/errors.rs::PipelineStage
  it('should match Rust PipelineStage enum values', () => {
    const rustStages = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly'
    ]

    const tsStages: PipelineStage[] = [
      'query_processing',
      'search_execution',
      'score_fusion',
      'result_assembly'
    ]

    expect(rustStages).toEqual(tsStages)
  })

  it('should deserialize SearchErrorDetails from Rust JSON', () => {
    const rustJson = {
      error_type: 'embedding_provider',
      stage: 'query_processing',
      context: { provider_error: 'timeout' },
      suggestions: ['Check credentials', 'Try FTS mode']
    }

    // TypeScript should parse without errors
    const details: SearchErrorDetails = rustJson
    expect(details.error_type).toBe('embedding_provider')
    expect(details.suggestions).toHaveLength(2)
  })
})
```

**Manual Audit Checklist** (Quality Gate - Phase 1):
- [ ] All enum variants in Rust have matching TypeScript types
- [ ] Sync comments present in TypeScript linking to Rust source
- [ ] Type sync validation tests pass
- [ ] Integration tests validate serialization works end-to-end

### 4. Performance Path

**Flow**: Baseline search → Add metadata → Measure overhead

**Test Coverage**:
- p50, p95, p99 latency before/after
- Metadata assembly time
- JSON serialization time
- Total overhead <10ms

**Validation**: Performance benchmarks + Prometheus metrics

**Baseline Measurement** (Phase 1, before any code changes):
- Run standard search workload (100 queries)
- Record p50, p95, p99 latency
- Document query processing time breakdown
- Save metrics for Phase 2 comparison

**Regression Testing** (Phase 2, after metadata added):
- Run same search workload
- Compare to baseline
- **BLOCK merge if**: p95 latency increases >10ms
- **INVESTIGATE if**: p99 latency increases >20ms
- **OPTIMIZE if**: Metadata assembly takes >10ms

## Test Data Strategy

**Principle**: Minimal, realistic test data

**Rust Tests**:
- Use in-memory SQLite for integration tests
- Small test corpus (5-10 files, ~50 chunks)
- Mock embedding service for error scenarios
- Real embedding service for happy path

**TypeScript Tests**:
- Mock daemon responses with realistic JSON
- Use example error responses from Rust tests
- No database needed (daemon-client is RPC-only)

**E2E Tests**:
- Use existing crewchief repo as test corpus
- Real SQLite database at ~/.maproom/maproom.db
- Real embedding provider (or intentionally offline)

**Test Data Repository**:
```
tests/fixtures/
├── error_responses/
│   ├── embedding_provider.json
│   ├── database.json
│   ├── validation.json
│   └── not_found.json
└── search_responses/
    ├── with_understanding.json
    └── without_understanding.json
```

## Quality Gates

### Before Ticket Verification

- [ ] Unit tests pass (`cargo test`, `pnpm test`)
- [ ] Integration tests pass (if added)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] No TypeScript errors (`pnpm build`)
- [ ] No linting errors (`pnpm lint`)
- [ ] Manual test of ticket acceptance criteria

### Before Phase Completion

**Phase 1**:
- [ ] All 6 error types tested end-to-end
- [ ] At least 2 manual E2E scenarios validated
- [ ] Error serialization integration test passes
- [ ] Type sync validation passes

**Phase 2**:
- [ ] Query understanding integration test passes
- [ ] Manual E2E shows metadata in response
- [ ] Performance benchmark shows <10ms overhead
- [ ] Timing data accuracy validated

**Phase 3**:
- [ ] All acceptance tests pass (4 scenarios)
- [ ] Success criteria validated
- [ ] Performance regression test passes (p95 <100ms)
- [ ] Documentation tests (README examples work)

### Before Production Deployment

- [ ] Full test suite passes (Rust + TypeScript)
- [ ] All 4 E2E scenarios manually validated
- [ ] Performance metrics collected and reviewed
- [ ] Backward compatibility verified (existing MCP clients work)
- [ ] Type sync audit complete (Rust ↔ TypeScript)

## Backward Compatibility Testing

**Goal**: Ensure existing MCP clients continue working with new error format

**Clients to Test**:
- **maproom-mcp** (primary client) - MCP server using daemon-client
- **vscode-maproom** (uses maproom-mcp) - VSCode extension

**Test Strategy**: Manual testing with existing client versions

**Test Scenarios**:

1. **Old Error Handling Still Works**
   - Test: Trigger error with existing MCP client (before new error format)
   - Expected: Error message displayed (may be generic, but no crash)
   - Validates: Old clients ignore new `data` field

2. **New Fields Are Optional**
   - Test: Successful search with existing MCP client
   - Expected: Results displayed correctly
   - Validates: Old clients ignore `metadata.understanding` field

3. **No Breaking Changes**
   - Test: Run full MCP test suite with new daemon
   - Expected: All existing tests pass
   - Validates: API changes are additive only

**Quality Gate** (Phase 1):
- [ ] Existing MCP client works with new error format
- [ ] No crashes from new optional fields
- [ ] Error messages display correctly (even if generic)

## Non-Functional Testing

### Performance Testing

**Tools**: Existing Prometheus metrics

**Metrics**:
- Search latency (p50, p95, p99)
- Query processing time
- Metadata assembly time
- JSON serialization time

**Approach**:
1. Collect baseline metrics before Phase 2
2. Run same search workload after Phase 2
3. Compare results: <10ms overhead, p95 <100ms
4. Use existing metrics dashboard

**Regression Criteria**:
- p95 latency increases >10ms → BLOCK
- p99 latency increases >20ms → INVESTIGATE
- Metadata assembly >10ms → OPTIMIZE

### Type Safety Testing

**Goal**: Ensure Rust and TypeScript types stay in sync

**Approach**:
1. Integration test serializes Rust → JSON
2. Compare JSON schema to TypeScript type expectations
3. Deserialize JSON → TypeScript objects
4. Validate required fields present
5. Validate enum values match

**Example Validation**:
```typescript
// Type sync validation test
it('should match Rust ErrorType enum', () => {
  const rustErrorTypes = ['embedding_provider', 'database', 'validation', 'timeout', 'not_found', 'unknown']
  const tsErrorTypes: ErrorType[] = ['embedding_provider', 'database', 'validation', 'timeout', 'not_found', 'unknown']

  // This will fail to compile if types diverge
  expect(rustErrorTypes).toEqual(tsErrorTypes)
})
```

## Testing Anti-Patterns to Avoid

1. **Testing Implementation Details**: Don't test private functions, test outcomes
2. **Excessive Mocking**: Mock only external dependencies (embedding APIs, databases)
3. **Coverage Chasing**: Don't write tests just to hit 100% coverage
4. **Complex Fixtures**: Keep test data minimal and realistic
5. **Flaky Tests**: If a test is flaky, fix or remove it - don't retry

## Manual Testing Checklist

Before declaring a phase complete, manually test:

**Phase 1: Error Diagnostics**
- [ ] Embedding provider offline → Shows actionable error with suggestions
- [ ] Repository not found → Shows repo name and suggests status/scan
- [ ] Empty query → Caught by Zod validation before RPC
- [ ] Database connection error → Shows database error with troubleshooting

**Phase 2: Query Understanding**
- [ ] Search "authenticate user" → Shows mode=auto, tokens, expanded terms
- [ ] Search "User::authenticate" → Shows mode=code
- [ ] Search "how to authenticate a user" → Shows mode=text
- [ ] Timing breakdown shows reasonable values (total matches sum of parts)

**Phase 3: Refinement**
- [ ] All Phase 1 + Phase 2 tests still pass
- [ ] Performance metrics show no regression
- [ ] Enhanced suggestions are helpful and specific
- [ ] Documentation examples work correctly
