# Ticket: UNIWATCH-2002: Create BranchSwitchEvent NDJSON Struct

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

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
Define and implement the BranchSwitchEvent struct for NDJSON serialization, allowing the VSCode extension to detect and respond to branch switches.

## Background
The VSCode extension consumes NDJSON events from the watch command to update its status bar and UI. When a branch switch occurs, we need to emit a structured event that includes old/new branch names, old/new worktree IDs, and whether the worktree was newly created.

This enables the extension to:
- Update the branch indicator in the status bar
- Refresh file decorations based on new worktree context
- Notify the user of the branch change
- Track worktree creation for analytics/debugging

This ticket implements Phase 2 (Branch Switch Logic) from the UNIWATCH project plan, specifically the NDJSON event emission component.

## Acceptance Criteria
- [ ] `BranchSwitchEvent` struct defined with `Serialize` derive
- [ ] Fields included: `event_type` ("branch_switched"), `timestamp`, `repo`, `old_branch`, `new_branch`, `old_worktree_id`, `new_worktree_id`, `worktree_created`
- [ ] Serializes to valid JSON matching the spec
- [ ] Emitted to stdout via `println!()` in `handle_branch_switch()`
- [ ] Unit test `test_branch_switch_event_serialization()` passes

## Technical Requirements
- Location: `crates/maproom/src/indexer/mod.rs` (inline struct definition) OR new events module if preferred
- Use `serde::Serialize` derive macro
- Use `#[serde(rename = "type")]` for `event_type` field to match JSON convention
- Timestamp format: ISO 8601 (use chrono or std::time)
- Approximately 20 lines of new code
- Must match NDJSON format from architecture.md
- JSON must be single-line (no pretty printing) to maintain NDJSON format
- Must handle serialization errors gracefully (return Result)

## Implementation Notes

Implementation approach:

```rust
#[derive(Serialize)]
struct BranchSwitchEvent {
    #[serde(rename = "type")]
    event_type: &'static str,
    timestamp: String,
    repo: String,
    old_branch: String,
    new_branch: String,
    old_worktree_id: i32,
    new_worktree_id: i32,
    worktree_created: bool,
}

// Usage in handle_branch_switch() (after state update):
let event = BranchSwitchEvent {
    event_type: "branch_switched",
    timestamp: chrono::Utc::now().to_rfc3339(),
    repo: repo.to_string(),
    old_branch: /* capture before update */,
    new_branch: new_branch.clone(),
    old_worktree_id: /* capture before update */,
    new_worktree_id,
    worktree_created: created,
};
println!("{}", serde_json::to_string(&event)?);
```

**Expected JSON output:**
```json
{"type":"branch_switched","timestamp":"2025-01-16T10:30:00Z","repo":"crewchief","old_branch":"main","new_branch":"feature-auth","old_worktree_id":1,"new_worktree_id":42,"worktree_created":false}
```

**Integration with handle_branch_switch()**:
- Capture `old_branch` and `old_worktree_id` BEFORE updating state
- Create event struct after successful state update
- Emit to stdout before final return
- Handle serialization errors (log and continue, don't crash watcher)

**Key Considerations**:
- Use `to_string()` not `to_string_pretty()` to maintain NDJSON single-line format
- Timestamp should be UTC for consistency
- Event emission should happen after successful state update but before returning
- Consider whether to emit event on error paths (recommendation: only emit on success)

## Dependencies
- UNIWATCH-2001 (`handle_branch_switch` function needs to emit this event)
- Requires `serde` and `serde_json` crates (likely already in dependencies)
- May need to add `chrono` crate if not already present (or use `std::time::SystemTime`)

## Risk Assessment
- **Risk**: JSON serialization errors could crash the watcher
  - **Mitigation**: Use `Result` return type in `handle_branch_switch()`, log errors instead of `unwrap()`; consider using `serde_json::to_string()` with proper error handling

- **Risk**: Timestamp format inconsistencies
  - **Mitigation**: Use standard chrono format (`to_rfc3339()`) which guarantees ISO 8601 compliance

- **Risk**: NDJSON format violation (multi-line JSON)
  - **Mitigation**: Use `to_string()` not `to_string_pretty()`; add test to verify single-line output

- **Risk**: Missing dependency (chrono)
  - **Mitigation**: Check Cargo.toml first; if missing, add `chrono = { version = "0.4", features = ["serde"] }`

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (~20 new lines for struct definition)
- `crates/maproom/Cargo.toml` (may need to add `chrono` dependency)
- `crates/maproom/src/indexer/mod.rs` (integration into `handle_branch_switch()` function - modify existing code from UNIWATCH-2001)
- Test file (location TBD based on existing test structure)
