# Ticket: UNIWATCH-0001: Export Indexer Module Components

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (visibility changes only, no new tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Export `setup_head_watcher()`, `DebouncedHandler`, and `BranchSwitchEvent` from the indexer module so they can be used in `main.rs` for the unified watch command implementation.

## Background
The watch command enhancement requires three components that already exist in `indexer/mod.rs` but are currently private and marked with `#[allow(dead_code)]`. These were prepared during earlier work but never exported publicly. This is a prerequisite (Phase 0) for all subsequent implementation work.

**Plan Reference:** Phase 0 - Module Exports (Prerequisite)

## Acceptance Criteria
- [x] `setup_head_watcher()` is exported as public function from `crates/maproom/src/indexer/mod.rs`
- [x] `DebouncedHandler` struct and its `new()`, `should_handle()` methods are exported as public
- [x] `BranchSwitchEvent` struct is exported as public
- [x] All `#[allow(dead_code)]` annotations removed from these three items
- [x] `cargo check -p crewchief-maproom` passes with no dead_code warnings for these items

## Technical Requirements
- Add `pub` visibility modifier to:
  - `fn setup_head_watcher()` at ~line 668
  - `struct DebouncedHandler` at ~line 33
  - `struct BranchSwitchEvent` at ~line 112
- Add re-exports in `indexer/mod.rs`: `pub use self::{DebouncedHandler, BranchSwitchEvent, setup_head_watcher};`
- Remove `#[allow(dead_code)]` annotations from all three components
- Ensure `DebouncedHandler::new()` and `DebouncedHandler::should_handle()` methods have `pub` visibility

## Implementation Notes
These components already exist and are tested. This ticket is strictly about visibility changes - no functional changes needed.

**Existing locations:**
- `setup_head_watcher()` - indexer/mod.rs:668
- `DebouncedHandler` - indexer/mod.rs:33
- `BranchSwitchEvent` - indexer/mod.rs:112

After export, these can be imported in main.rs as:
```rust
use crewchief_maproom::indexer::{setup_head_watcher, DebouncedHandler, BranchSwitchEvent};
```

## Dependencies
- None (this is the prerequisite for all other UNIWATCH tickets)

## Risk Assessment
- **Risk**: Minimal - visibility changes only
  - **Mitigation**: Run `cargo check` to ensure no compilation errors

## Files/Packages Affected
- `crates/maproom/src/indexer/mod.rs` (~10 lines modified)
