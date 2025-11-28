# Ticket: CTXCLI-1002: Implement Daemon Context Handler with State Support

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `context` method handler to the JSON-RPC daemon with `BasicContextAssembler` integrated into `DaemonState` to enable caching across requests.

## Background
This ticket combines the original CTXCLI-1002 (handler) and CTXCLI-1003 (state support) to ensure proper initialization order. The assembler must be in DaemonState *before* the handler is called to enable caching - creating a new assembler per request would defeat the caching purpose.

This completes Phase 1 (Foundation) by exposing the context assembler via the daemon's JSON-RPC interface.

Reference: [planning/architecture.md](../planning/architecture.md) - DaemonState with Context Support

## Acceptance Criteria
- [ ] `BasicContextAssembler` is added to `DaemonState` struct
- [ ] `DaemonState::new()` initializes assembler with `CacheConfig::default()`
- [ ] Assembler reuses database connection from `SqliteStore`
- [ ] `context` method case added to `handle_request()` match
- [ ] `execute_context()` function implemented using `state.context_assembler`
- [ ] Daemon responds to `context` method with valid `ContextBundle` JSON
- [ ] Returns `-32000` error for missing chunk
- [ ] Returns `-32602` error for invalid params
- [ ] Context cache persists across requests (verified by test - second call faster)
- [ ] No performance regression for search operations

## Technical Requirements
- Import `BasicContextAssembler` and `CacheConfig` from context module
- Import `ExpandOptions` from context types
- Convert `ContextParams` (from CTXCLI-1001) to `ExpandOptions` for assembler
- Parse `chunk_id` string to `i64`
- Use `state.context_assembler.assemble()` (not new instance)
- Serialize `ContextBundle` result to `serde_json::Value`

## Implementation Notes

### DaemonState Update
```rust
pub struct DaemonState {
    pub store: Arc<SqliteStore>,
    pub embedding_service: EmbeddingService,
    pub context_assembler: BasicContextAssembler,  // NEW
}

impl DaemonState {
    pub fn new(store: Arc<SqliteStore>, embedding_service: EmbeddingService) -> Self {
        Self {
            store: store.clone(),
            embedding_service,
            context_assembler: BasicContextAssembler::new(
                store,
                CacheConfig::default(),
            ),
        }
    }
}
```

### Handler Addition
```rust
async fn handle_request(request: JsonRpcRequest, state: Arc<DaemonState>) -> JsonRpcResponse {
    match request.method.as_str() {
        "ping" => ...,
        "search" => ...,
        "context" => {
            let params: ContextParams = match serde_json::from_value(...) { ... };
            match execute_context(state, params).await {
                Ok(bundle) => JsonRpcResponse::success(id, bundle),
                Err(e) => JsonRpcResponse::error(id, -32000, e.to_string(), None),
            }
        }
        _ => ...
    }
}
```

### execute_context Implementation
```rust
async fn execute_context(
    state: Arc<DaemonState>,
    params: ContextParams,
) -> Result<serde_json::Value> {
    let chunk_id = params.chunk_id.parse::<i64>()?;
    let options = ExpandOptions {
        callers: params.expand.callers,
        callees: params.expand.callees,
        tests: params.expand.tests,
        docs: params.expand.docs,
        config: params.expand.config,
        max_depth: params.expand.max_depth,
        routes: params.expand.routes,
        hooks: params.expand.hooks,
        jsx_parents: params.expand.jsx_parents,
        jsx_children: params.expand.jsx_children,
        ..Default::default()
    };

    let bundle = state.context_assembler
        .assemble(chunk_id, params.budget_tokens, options)
        .await?;

    Ok(serde_json::to_value(bundle)?)
}
```

### Error Codes
- `-32602`: Invalid params (missing chunk_id, parse error)
- `-32000`: Chunk not found
- `-32001`: File not found on disk
- `-32002`: Budget exceeded

## Dependencies
- CTXCLI-1001 (Context params types must exist)

## Implementation Notes - Prerequisites

**Important**: The `BasicContextAssembler::get_chunk_metadata()` method in `assembler.rs:138-141` currently returns a bail error ("not yet implemented"). However, the `DefaultStrategy` in `context/strategies/default.rs` has a working implementation. The daemon handler should use `DefaultStrategy.assemble()` or ensure the BasicContextAssembler delegates to a strategy that has `get_chunk_metadata` implemented.

**Approach Options:**
1. Use `DefaultStrategy` directly (recommended - it has the working implementation)
2. Or implement `BasicContextAssembler::get_chunk_metadata()` using SqliteStore queries

## Risk Assessment
- **Risk**: Breaking existing search functionality
  - **Mitigation**: Run search tests after changes, ensure Arc<SqliteStore> sharing works
- **Risk**: Assembler lifetime issues with DaemonState
  - **Mitigation**: Verify BasicContextAssembler can be stored in struct (no lifetime issues)

## Files/Packages Affected
- `crates/maproom/src/daemon/mod.rs` (modify - add context handler, update DaemonState)
