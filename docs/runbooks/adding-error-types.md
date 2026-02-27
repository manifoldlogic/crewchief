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

**Questions to answer:**
- What user action triggers this error?
- What stage of the search pipeline does it occur in?
- What context information is available from the error?
- What are 1-2 actionable steps users can take to resolve it?

### 2. Update Rust Error Taxonomy
**File**: `crates/maproom/src/search/errors.rs`

#### 2.1 Add ErrorType Variant (if new type)
```rust
// Line ~60-75
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    EmbeddingProvider,
    Database,
    Validation,
    Timeout,
    NotFound,
    Unknown,
    // Add new variant here (in snake_case for JSON serialization)
    RateLimitExceeded,
}
```

#### 2.2 Add Conversion Case
```rust
// In SearchErrorDetails::from_pipeline_error() (line ~114)
match error {
    PipelineError::QueryProcessing(query_error) => {
        // Add handling for new error scenario
        match query_error {
            QueryProcessorError::RateLimit { retry_after_ms } => Self {
                error_type: ErrorType::RateLimitExceeded,
                stage: PipelineStage::QueryProcessing,
                context: HashMap::from([
                    ("retry_after_ms".to_string(), retry_after_ms.to_string()),
                ]),
                suggestions: vec![
                    format!("Wait {} seconds before retrying", retry_after_ms / 1000),
                    "Check API quota limits".to_string(),
                ],
            },
            // ... existing cases
        }
    }
    // ... other pipeline stages
}
```

#### 2.3 Context Whitelist
Only include whitelisted context keys (line ~104-113):
- `provider_error` - Embedding provider error details
- `provider` - Provider name (OpenAI, Google, Ollama)
- `error` - Generic error message
- `message` - Human-readable message
- `length` - Query length for validation errors
- `max_length` - Maximum allowed query length
- `repo_name` - Repository name for not found errors
- `worktree_id` - Worktree identifier
- `timeout_ms` - Timeout duration

**If adding new context keys**, update the whitelist test (line ~917-927).

#### 2.4 Add Unit Test
```rust
// Add to tests section (line ~669+)
#[test]
fn test_rate_limit_error() {
    let error = PipelineError::QueryProcessing(
        QueryProcessorError::RateLimit { retry_after_ms: 60000 }
    );

    let details = SearchErrorDetails::from_pipeline_error(&error);

    assert_eq!(details.error_type, ErrorType::RateLimitExceeded);
    assert_eq!(details.stage, PipelineStage::QueryProcessing);
    assert_eq!(details.context.get("retry_after_ms"), Some(&"60000".to_string()));
    assert!(details.suggestions.len() >= 2);
    assert!(details.suggestions.iter().any(|s| s.contains("60 seconds")));
    assert!(details.suggestions.iter().any(|s| s.contains("quota")));
}
```

### 3. Update TypeScript Types
**File**: `packages/daemon-client/src/types.ts`

#### 3.1 Add ErrorType Variant
```typescript
// Line ~70-76
/**
 * High-level error type categories for search errors.
 *
 * Sync with: crates/maproom/src/search/errors.rs::ErrorType
 */
export type ErrorType =
  | 'embedding_provider'
  | 'database'
  | 'validation'
  | 'timeout'
  | 'not_found'
  | 'unknown'
  | 'rate_limit_exceeded'  // Add new variant (snake_case)
```

#### 3.2 Verify Sync Comment
Ensure the sync comment points to the correct Rust file:
```typescript
// Sync with: crates/maproom/src/search/errors.rs::ErrorType
```

### 4. Update Type Sync Validation Test
**File**: `packages/daemon-client/src/types.test.ts`

#### 4.1 Add to Validation Array
```typescript
// Line ~14-35
it('should match Rust ErrorType enum values', () => {
  const rustErrorTypes = [
    'embedding_provider',
    'database',
    'validation',
    'timeout',
    'not_found',
    'unknown',
    'rate_limit_exceeded',  // Add new variant
  ]

  const tsErrorTypes: ErrorType[] = [
    'embedding_provider',
    'database',
    'validation',
    'timeout',
    'not_found',
    'unknown',
    'rate_limit_exceeded',  // Add new variant
  ]

  expect(rustErrorTypes).toEqual(tsErrorTypes)
})
```

#### 4.2 Add Specific Test Case
```typescript
it('should handle rate limit error details', () => {
  const details: SearchErrorDetails = {
    error_type: 'rate_limit_exceeded',
    stage: 'query_processing',
    context: {
      retry_after_ms: '60000',
    },
    suggestions: [
      'Wait 60 seconds before retrying',
      'Check API quota limits',
    ],
  }

  expect(details.error_type).toBe('rate_limit_exceeded')
  expect(details.context.retry_after_ms).toBe('60000')
  expect(details.suggestions).toHaveLength(2)
})
```

### 5. Integration Testing

#### 5.1 Add Rust Integration Test
**File**: `crates/maproom/tests/daemon_error_serialization.rs` (if it exists)

```rust
#[tokio::test]
async fn test_rate_limit_error_serialization() {
    let error = PipelineError::QueryProcessing(
        QueryProcessorError::RateLimit { retry_after_ms: 60000 }
    );

    let details = SearchErrorDetails::from_pipeline_error(&error);
    let json = serde_json::to_string(&details).unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("rate_limit_exceeded"));
    assert!(json.contains("query_processing"));
    assert!(json.contains("60000"));
    assert!(json.contains("60 seconds"));
}
```

#### 5.2 Manual Testing with Daemon
```bash
# Start daemon
cargo run --bin maproom -- serve

# Send RPC request that triggers the error
echo '{"jsonrpc":"2.0","method":"search","params":{"query":"test","repo":"nonexistent"},"id":1}' | \
  cargo run --bin maproom -- serve

# Verify response includes error_type, stage, context, suggestions
```

### 6. Update Documentation

#### 6.1 Update README Error Table
**File**: `packages/daemon-client/README.md`

```markdown
### Error Types

| Error Type | Description | Common Causes |
|------------|-------------|---------------|
| `embedding_provider` | Embedding service failure | API key invalid, service offline, network issues |
| `database` | Database operation failure | Repository not indexed, connection timeout, corrupted DB |
| `validation` | Invalid query parameters | Empty query, query too long |
| `timeout` | Search execution timeout | Complex query, large repository |
| `not_found` | Resource not found | Repository doesn't exist, no meaningful content |
| `unknown` | Unexpected error | Internal errors, unclassified failures |
| `rate_limit_exceeded` | API rate limit hit | Too many requests, quota exceeded |
```

#### 6.2 Add Example Usage
```markdown
### Rate Limit Handling Example

```typescript
import { RpcError } from '@crewchief/daemon-client'

try {
  await client.search({ query: 'test', repo: 'crewchief' })
} catch (error) {
  if (error instanceof RpcError) {
    const details = error.getDetails()
    if (details?.error_type === 'rate_limit_exceeded') {
      const retryAfterMs = parseInt(details.context.retry_after_ms || '0')
      console.log(`Rate limited. Retry after ${retryAfterMs / 1000} seconds`)
      // Implement exponential backoff or wait logic
    }
  }
}
```
```

## Example: Adding "RateLimitExceeded" Error

### Step-by-Step Implementation

#### 1. Rust Enum (errors.rs)
```rust
pub enum ErrorType {
    EmbeddingProvider,
    Database,
    Validation,
    Timeout,
    NotFound,
    Unknown,
    RateLimitExceeded,  // New variant
}
```

#### 2. Conversion Logic (errors.rs)
```rust
// In from_pipeline_error or from_embedding_error
EmbeddingError::RateLimit { retry_after_ms } => Self {
    error_type: ErrorType::RateLimitExceeded,
    stage: PipelineStage::QueryProcessing,
    context: HashMap::from([
        ("retry_after_ms".to_string(), retry_after_ms.to_string()),
        ("provider".to_string(), "openai".to_string()),  // If known
    ]),
    suggestions: vec![
        format!("Wait {} seconds before retrying", retry_after_ms / 1000),
        "Check API quota limits".to_string(),
    ],
}
```

#### 3. TypeScript Type (types.ts)
```typescript
export type ErrorType =
  | 'embedding_provider'
  | 'database'
  | 'validation'
  | 'timeout'
  | 'not_found'
  | 'unknown'
  | 'rate_limit_exceeded'  // New variant
```

#### 4. Validation Test (types.test.ts)
```typescript
const rustErrorTypes = [
  'embedding_provider',
  'database',
  'validation',
  'timeout',
  'not_found',
  'unknown',
  'rate_limit_exceeded',  // Add here
]
```

#### 5. Unit Test (errors.rs)
```rust
#[test]
fn test_rate_limit_error() {
    let error = PipelineError::QueryProcessing(
        QueryProcessorError::Embedding(
            EmbeddingError::RateLimit { retry_after_ms: 60000 }
        )
    );

    let details = SearchErrorDetails::from_pipeline_error(&error);

    assert_eq!(details.error_type, ErrorType::RateLimitExceeded);
    assert_eq!(details.stage, PipelineStage::QueryProcessing);
    assert_eq!(details.context.get("retry_after_ms"), Some(&"60000".to_string()));
    assert!(details.suggestions.len() >= 2);
}
```

#### 6. Verify Tests Pass
```bash
# Rust unit tests
cargo test -p maproom test_rate_limit_error

# TypeScript type sync tests
cd packages/daemon-client
pnpm test types.test.ts
```

## Validation Checklist

Before merging:
- [ ] Rust ErrorType enum updated
- [ ] Conversion logic added to from_pipeline_error()
- [ ] Suggestions are actionable (1-2 specific steps)
- [ ] Context uses whitelisted keys only
- [ ] Unit test added and passing
- [ ] TypeScript ErrorType union updated
- [ ] Type sync test updated
- [ ] Integration test added (if applicable)
- [ ] README.md error table updated
- [ ] Example usage documented
- [ ] Sync comments point to correct files

## Common Pitfalls

### 1. Serialization Case Mismatch
**Problem**: Rust uses PascalCase, JSON uses snake_case
**Solution**: Use `#[serde(rename_all = "snake_case")]` on enum

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]  // Critical!
pub enum ErrorType {
    RateLimitExceeded,  // Serializes to "rate_limit_exceeded"
}
```

### 2. Non-Whitelisted Context Keys
**Problem**: Adding arbitrary context keys can leak sensitive data
**Solution**: Only use whitelisted keys, add to whitelist test if needed

```rust
// ❌ BAD: Exposes internal paths
context.insert("file_path".to_string(), path.display().to_string());

// ✅ GOOD: Uses whitelisted key
context.insert("error".to_string(), "File not found".to_string());
```

### 3. Vague Suggestions
**Problem**: Suggestions like "Try again later" are not actionable
**Solution**: Provide specific commands or configuration checks

```rust
// ❌ BAD: Vague
suggestions: vec!["Fix the issue".to_string()]

// ✅ GOOD: Specific
suggestions: vec![
    "Check OPENAI_API_KEY environment variable".to_string(),
    "Verify API key at https://platform.openai.com/api-keys".to_string(),
]
```

### 4. Forgetting Type Sync Test
**Problem**: TypeScript types drift from Rust without validation
**Solution**: Always update types.test.ts validation arrays

```typescript
// Add to BOTH arrays in the test
const rustErrorTypes = [..., 'new_error_type']
const tsErrorTypes: ErrorType[] = [..., 'new_error_type']
```

## Testing Strategy

### Unit Testing
```bash
# Test specific error conversion
cargo test -p maproom test_rate_limit_error

# Test all error types have suggestions
cargo test -p maproom test_all_error_types_have_suggestions

# Test context whitelist enforcement
cargo test -p maproom test_context_whitelist_enforced
```

### Type Sync Testing
```bash
cd packages/daemon-client
pnpm test types.test.ts
```

### Integration Testing
```bash
# Start daemon in one terminal
cargo run --bin maproom -- serve

# Send RPC requests in another terminal
echo '{"jsonrpc":"2.0","method":"search","params":{"query":"test","repo":"nonexistent"},"id":1}' | \
  nc localhost 9876  # Or use daemon stdin
```

### Manual Testing Scenarios
1. **Trigger the error** - Create conditions that cause the new error type
2. **Verify error_type** - Check JSON response has correct error_type value
3. **Check suggestions** - Ensure suggestions are actionable and make sense
4. **Test context** - Verify context fields contain expected values
5. **TypeScript client** - Test with actual MCP or VSCode client

## Troubleshooting

### Error Not Appearing in TypeScript
**Check:**
1. Is the Rust type using `#[serde(rename_all = "snake_case")]`?
2. Did you rebuild the Rust binary?
3. Is the daemon using the new binary?

**Fix:**
```bash
cargo build --release --bin maproom
# Restart daemon to use new binary
```

### Type Sync Test Failing
**Check:**
1. Did you add the variant to BOTH arrays in types.test.ts?
2. Is the casing correct (snake_case)?
3. Are the arrays in the same order?

**Fix:**
```typescript
// Ensure both arrays have identical values in same order
const rustErrorTypes = ['embedding_provider', ..., 'new_type']
const tsErrorTypes: ErrorType[] = ['embedding_provider', ..., 'new_type']
```

### Suggestions Not Helpful
**Check:**
1. Are suggestions specific to the error scenario?
2. Do they include actual commands or configuration keys?
3. Would a new user understand what to do?

**Fix:**
```rust
// Be specific and actionable
suggestions: vec![
    "Set OPENAI_API_KEY environment variable: export OPENAI_API_KEY=sk-...".to_string(),
    "Or try FTS mode without embeddings: --mode fts".to_string(),
]
```

## Related Documentation

- [Error Taxonomy Design](../../../.crewchief/projects/SRCHTRN_search-transparency/planning/architecture.md)
- [Type Synchronization Guide](../../../packages/daemon-client/CLAUDE.md)
- [Search Pipeline Architecture](../../../crates/maproom/src/search/README.md)
- [Daemon Client Error Handling](../../../packages/daemon-client/README.md#error-handling)

## Maintenance Notes

**Frequency**: Add new error types as new error scenarios are discovered

**Owner**: Team member responsible for search transparency

**Monitoring**: Watch for "unknown" error types in logs - candidates for new error types

**Periodic Review**: Quarterly review of error types to ensure:
- Suggestions are still accurate
- Context fields are still relevant
- No new error scenarios need categorization
