# Ticket: CTXCLI-1001: Add Context Params Types

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add `ContextParams` and `ExpandConfig` types to the daemon types module for JSON-RPC context method parameter handling.

## Background
This is Phase 1 (Foundation) of the CTXCLI project. The Rust context assembler (completed in SQLIMPL Phase 4) needs to be exposed via JSON-RPC daemon. Before implementing the handler (CTXCLI-1002), we need the parameter types for deserializing incoming requests.

The types must support:
- Required `chunk_id` field (as String for JSON compatibility)
- Optional `budget_tokens` with default 6000
- Optional `expand` config with all expand options including React-specific fields

Reference: [planning/architecture.md](../planning/architecture.md) - Section 2: Daemon Context Method

## Acceptance Criteria
- [ ] `ContextParams` struct exists in `crates/maproom/src/daemon/types.rs`
- [ ] `ExpandConfig` struct exists with all expand options
- [ ] `ContextParams` deserializes from JSON correctly with minimal input `{"chunk_id": "12345"}`
- [ ] Default values applied when fields missing (budget_tokens=6000, max_depth=2)
- [ ] All 10 expand options supported (callers, callees, tests, docs, config, max_depth, routes, hooks, jsx_parents, jsx_children)
- [ ] Types match Rust `ExpandOptions` exactly (all 10 fields)
- [ ] Unit tests pass for deserialization scenarios

## Technical Requirements
- Use `#[serde(default)]` for optional fields with default functions
- `chunk_id` must be `String` type (not i64) for JSON compatibility
- `ExpandConfig` must implement `Default` trait
- Add default value functions: `default_budget() -> usize { 6000 }` and `default_max_depth() -> i32 { 2 }`

## Implementation Notes

### Type Definitions
```rust
#[derive(Debug, Deserialize)]
pub struct ContextParams {
    pub chunk_id: String,  // String for JSON compatibility
    #[serde(default = "default_budget")]
    pub budget_tokens: usize,
    #[serde(default)]
    pub expand: ExpandConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct ExpandConfig {
    #[serde(default)]
    pub callers: bool,
    #[serde(default)]
    pub callees: bool,
    #[serde(default)]
    pub tests: bool,
    #[serde(default)]
    pub docs: bool,
    #[serde(default)]
    pub config: bool,
    #[serde(default = "default_max_depth")]
    pub max_depth: i32,
    // React-specific
    #[serde(default)]
    pub routes: bool,
    #[serde(default)]
    pub hooks: bool,
    #[serde(default)]
    pub jsx_parents: bool,
    #[serde(default)]
    pub jsx_children: bool,
}

fn default_budget() -> usize { 6000 }
fn default_max_depth() -> i32 { 2 }
```

### Unit Tests Required
See [planning/quality-strategy.md](../planning/quality-strategy.md) for test examples:
- `test_context_params_deserialization_minimal()` - minimal JSON with only chunk_id
- `test_context_params_deserialization_full()` - full JSON with all fields
- `test_expand_config_defaults()` - verify Default implementation

## Dependencies
- None (first ticket in Phase 1)

## Risk Assessment
- **Risk**: Type mismatch with context module's `ExpandOptions`
  - **Mitigation**: Verify field names match `crates/maproom/src/context/types.rs ExpandOptions`

## Files/Packages Affected
- `crates/maproom/src/daemon/types.rs` (modify - add types)
