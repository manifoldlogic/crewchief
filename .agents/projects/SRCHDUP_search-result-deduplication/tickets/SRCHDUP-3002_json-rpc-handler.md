# Ticket: SRCHDUP-3002: Update Rust daemon JSON-RPC handler for deduplicate

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

Update the Rust daemon's JSON-RPC handler to accept the `deduplicate` parameter from incoming search requests and pass it through to the SearchOptions when executing searches.

## Background

The Rust daemon receives JSON-RPC calls from the daemon-client TypeScript package. The `search` method handler must be updated to deserialize the `deduplicate` field and include it when constructing SearchOptions.

**Reference:** plan.md Phase 3, architecture.md Section 7 "Rust Daemon JSON-RPC Handler"

## Acceptance Criteria

- [ ] `SearchRequest` struct has `deduplicate: Option<bool>` field with serde attribute
- [ ] Handler extracts `deduplicate` and passes to SearchOptions
- [ ] Missing `deduplicate` defaults to `true`
- [ ] JSON-RPC `search` method accepts `{"deduplicate": false}` in params
- [ ] `cargo build` succeeds
- [ ] Existing daemon tests pass

## Technical Requirements

### SearchRequest Update
```rust
// In crates/maproom/src/daemon/handlers.rs or similar

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub repo: String,
    pub worktree: Option<String>,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub debug: Option<bool>,
    #[serde(default = "default_deduplicate")]
    pub deduplicate: Option<bool>,
}

fn default_deduplicate() -> Option<bool> {
    Some(true)
}
```

### Handler Update
```rust
impl SearchRequest {
    pub fn to_search_options(&self, repo_id: i64, worktree_id: Option<i64>) -> SearchOptions {
        SearchOptions::new(repo_id, worktree_id, self.limit.unwrap_or(10))
            .with_deduplicate(self.deduplicate.unwrap_or(true))
    }
}
```

Or directly in the handler:
```rust
async fn handle_search(req: SearchRequest) -> Result<Vec<SearchResult>> {
    let options = SearchOptions::new(repo_id, worktree_id, req.limit.unwrap_or(10))
        .with_deduplicate(req.deduplicate.unwrap_or(true));

    let results = pipeline.search(&req.query, &options).await?;
    // ...
}
```

## Implementation Notes

1. **Find handler location** - Locate where search JSON-RPC method is handled
2. **Check serde version** - Ensure `#[serde(default)]` works as expected
3. **Match field name exactly** - Must match what daemon-client sends
4. **Verify deserialization** - Test with curl or manual JSON-RPC call

### Verification
```bash
# Test JSON-RPC call with deduplicate param
curl -X POST http://localhost:PORT/jsonrpc \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"search","params":{"query":"test","repo":"crewchief","deduplicate":false},"id":1}'
```

## Dependencies

- SRCHDUP-2002 (SearchOptions has deduplicate field)
- Can be done in parallel with SRCHDUP-3001

## Risk Assessment

- **Risk**: Field name mismatch between TypeScript and Rust
  - **Mitigation**: Use exact same field name `deduplicate` in both
- **Risk**: Daemon crate location differs from expected
  - **Mitigation**: Search codebase for JSON-RPC handler, `jsonrpsee`, or `search` method

## Files/Packages Affected

- `crates/maproom/src/daemon/` (handlers.rs or similar)
- May involve `crates/maproom/src/daemon/methods.rs` or `routes.rs` depending on structure
