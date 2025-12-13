# SRCHTRN-1002: JSON-RPC Error Serialization

## Title
Serialize SearchErrorDetails in JSON-RPC error responses

## Status
- [ ] **Implementation Complete**
- [ ] **Tests Passing**
- [ ] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: rust-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Modify the daemon RPC handler in `crates/maproom/src/daemon/mod.rs` to catch `PipelineError`, convert to `SearchErrorDetails`, and serialize in the JSON-RPC error `data` field while preserving backward-compatible error messages.

## Background
The daemon currently returns generic JSON-RPC errors with minimal information. With SRCHTRN-1001 providing structured error details, this ticket extends the RPC handler to serialize those details in the `data` field per JSON-RPC 2.0 spec.

**Extension Point Identified**: Lines 143-151 in `crates/maproom/src/daemon/mod.rs` where error responses are currently constructed with `e.to_string()` in the data field.

## Acceptance Criteria
- [ ] Search handler catches `PipelineError` from search execution
- [ ] `SearchErrorDetails::from_pipeline_error()` called on errors
- [ ] Error details serialized in JSON-RPC `error.data` field
- [ ] Human-readable error message preserved in `error.message` (backward compatibility)
- [ ] Error code remains `-32000` (application error)
- [ ] Integration test validates error serialization end-to-end
- [ ] Manual test: Trigger embedding error, verify structured error in response
- [ ] All tests passing

## Technical Requirements

### Current Error Handling (mod.rs lines 143-151)
```rust
Err(e) => {
    error!("Search failed: {}", e);
    JsonRpcResponse::error(
        id,
        -32000,
        "Search failed".to_string(),
        Some(serde_json::json!(e.to_string())),
    )
}
```

### Updated Error Handling
```rust
Err(e) => {
    error!("Search failed: {}", e);

    // Convert PipelineError to structured details
    let error_details = SearchErrorDetails::from_pipeline_error(&e);

    JsonRpcResponse::error(
        id,
        -32000,
        e.to_string(), // Preserve human-readable message
        Some(serde_json::to_value(error_details)?),
    )
}
```

### JSON-RPC Error Response Format
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32000,
    "message": "Query processing failed: Embedding generation failed: request timeout",
    "data": {
      "error_type": "embedding_provider",
      "stage": "query_processing",
      "context": {
        "provider_error": "request timeout"
      },
      "suggestions": [
        "Check your embedding provider credentials",
        "Verify network connectivity",
        "Try FTS mode while debugging: --mode fts"
      ]
    }
  },
  "id": 1
}
```

### Integration Test
Create test in `crates/maproom/tests/daemon_error_serialization.rs`:

```rust
#[tokio::test]
async fn test_search_error_serialization() {
    // Setup: Create test daemon with mock embedding service that fails
    let daemon = create_test_daemon_with_failing_embeddings().await;

    // Execute: Send search request that requires embeddings
    let request = json!({
        "jsonrpc": "2.0",
        "method": "search",
        "params": {
            "query": "test query",
            "repo": "test-repo",
            "mode": "vector"
        },
        "id": 1
    });

    let response = daemon.handle_request(request).await;

    // Assert: Error response has structured data
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32000);

    let error_data = &response["error"]["data"];
    assert_eq!(error_data["error_type"], "embedding_provider");
    assert_eq!(error_data["stage"], "query_processing");
    assert!(error_data["suggestions"].as_array().unwrap().len() >= 2);
}
```

## Implementation Notes
1. Import `SearchErrorDetails` from `crates/maproom/src/search/errors`
2. Locate the search handler in `crates/maproom/src/daemon/mod.rs` (around lines 143-151)
3. Replace `e.to_string()` in data field with serialized `SearchErrorDetails`
4. Handle serialization errors gracefully (fallback to simple string)
5. Preserve existing error message in `message` field for backward compatibility

### Error Handling for Serialization Failures
```rust
let error_data = match serde_json::to_value(&error_details) {
    Ok(value) => Some(value),
    Err(ser_err) => {
        warn!("Failed to serialize error details: {}", ser_err);
        Some(serde_json::json!(e.to_string())) // Fallback
    }
};

JsonRpcResponse::error(id, -32000, e.to_string(), error_data)
```

## Dependencies
- **SRCHTRN-1001**: Rust error taxonomy (must complete first - provides `SearchErrorDetails`)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Serialization may fail for complex error types
- Breaking existing error handling flow

**Mitigations**:
- Fallback to simple string if serialization fails
- Integration test validates end-to-end flow
- Manual testing with real daemon

## Files/Packages Affected
- **Modified**: `crates/maproom/src/daemon/mod.rs` (search error handler, ~10 lines)
- **New file**: `crates/maproom/tests/daemon_error_serialization.rs` (integration test)
- **Import**: `use crate::search::errors::SearchErrorDetails;`

## Estimated Effort
3-4 hours

**Breakdown**:
- Implementation: 1-2 hours
- Integration test: 1-2 hours
- Manual testing: 0.5-1 hour

## Planning References
- [plan.md](../planning/plan.md) - Phase 1 ticket breakdown
- [architecture.md](../planning/architecture.md) - JSON-RPC serialization design, extension point
- [quality-strategy.md](../planning/quality-strategy.md) - Integration testing approach
