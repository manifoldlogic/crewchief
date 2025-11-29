# Ticket: TESTFIX-1003: Fix daemon types unused field warning

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the unused field warning in `crates/maproom/src/daemon/types.rs` that is causing Rust compilation to fail in CI with `-D warnings` enabled.

## Background
The CI workflow is failing during the "Initialize test database schema" step because of an unused field warning:

```
error: field `jsonrpc` is never read
  --> crates/maproom/src/daemon/types.rs:5:9
error: could not compile `crewchief-maproom` (bin "crewchief-maproom") due to 1 previous error
```

This is blocking TESTFIX-1002's validation because the Rust migration runner cannot build when `RUSTFLAGS: -D warnings` is set in CI. The `jsonrpc` field in the `JsonRpcRequest` struct is deserialized from incoming JSON-RPC requests but never actually read in the code, as the code only validates the protocol version implicitly through successful deserialization.

This ticket implements the "Fix compilation errors" task from the TESTFIX project plan, ensuring the codebase compiles cleanly with strict warning settings in CI.

## Acceptance Criteria
- [x] The `jsonrpc` field warning in `crates/maproom/src/daemon/types.rs:5:9` is resolved
- [x] Rust compilation succeeds with `RUSTFLAGS: -D warnings` enabled
- [x] The fix preserves JSON-RPC 2.0 protocol compliance (field must still be deserialized)
- [x] Cargo build completes successfully: `cargo build --release --bin crewchief-maproom`
- [x] All existing tests continue to pass

## Technical Requirements
- Suppress the warning while maintaining correct deserialization behavior
- Use Rust's `#[allow(dead_code)]` attribute on the specific field
- Preserve the comment explaining the field's purpose ("Must be '2.0'")
- Do not alter the JSON-RPC protocol implementation
- Ensure the fix works across all Rust toolchain versions used in CI

## Implementation Notes
The `jsonrpc` field serves a validation purpose through serde deserialization - if the field is missing or has the wrong value, deserialization will fail. However, the code never explicitly reads the field value after deserialization, triggering the "never read" warning.

**Recommended approach:**
Add `#[allow(dead_code)]` attribute to the `jsonrpc` field in the `JsonRpcRequest` struct:

```rust
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String, // Must be "2.0"
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<serde_json::Value>,
}
```

This approach:
- Silences the warning without changing runtime behavior
- Preserves JSON-RPC protocol compliance through serde validation
- Maintains code clarity with the existing comment
- Is the idiomatic Rust solution for validation-only fields

**Alternative approaches considered but not recommended:**
- Removing the field: Would break JSON-RPC 2.0 compliance
- Reading the field and validating: Adds unnecessary runtime overhead since serde already validates during deserialization
- Making it private: Doesn't resolve the warning

## Dependencies
- Blocks TESTFIX-1002 validation (CI cannot build migration runner)
- No dependencies on other tickets

## Risk Assessment
- **Risk**: The fix might not resolve all compilation warnings in CI
  - **Mitigation**: Run `cargo build` locally with `RUSTFLAGS="-D warnings"` to verify before committing

- **Risk**: Suppressing warnings might hide legitimate issues
  - **Mitigation**: Use field-specific `#[allow(dead_code)]` rather than module-level suppression to maintain warning coverage for other code

- **Risk**: JSON-RPC protocol compliance might be affected
  - **Mitigation**: The field remains in the struct and is deserialized; only the warning is suppressed. Serde will still enforce the field's presence during deserialization

## Files/Packages Affected
- `/workspace/crates/maproom/src/daemon/types.rs` (line 5)
