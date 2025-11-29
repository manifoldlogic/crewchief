# Ticket: RSTFIX-3001: Remove Dead Functions and Methods

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
Remove dead code functions and methods that are never called: `compute_edges`, `find_test_targets`, `insert_edges`, `is_route_chunk`, `is_test_chunk`, `as_str`, `create_context_item`, and `evict_lru_if_needed`.

## Background
During the SQLIMPL migration and feature development, several functions became dead code - either replaced by new implementations or never fully integrated. The compiler has verified these functions are never called. This is Phase 3 of the RSTFIX cleanup project.

Reference: Phase 3 in planning/plan.md - "Dead Code (Higher Risk)"

## Acceptance Criteria
- [ ] Function `compute_edges` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Function `find_test_targets` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Function `insert_edges` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Function `is_route_chunk` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Function `is_test_chunk` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Method `as_str` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Method `create_context_item` removed or marked with `#[allow(dead_code)]` with justification
- [ ] Method `evict_lru_if_needed` removed or marked with `#[allow(dead_code)]` with justification
- [ ] No dead_code warnings for removed functions
- [ ] All 906 tests pass

## Technical Requirements
- Follow the dead code decision tree from architecture.md:
  1. Is the code called anywhere? → Keep, investigate
  2. Is it intended for future use? → Add `#[allow(dead_code)]` with comment
  3. Neither? → Remove entirely
- Check git history for context on each function
- Look for TODO/FIXME comments that explain intent
- Prefer removal over `#[allow(dead_code)]` unless there's clear future intent

## Implementation Notes
**Dead code decision tree:**
```
Is the code called anywhere?
├── Yes → Keep, investigate why warning exists
└── No → Is it intended for future use?
    ├── Yes → Add #[allow(dead_code)] with comment explaining intent
    └── No → Remove entirely
```

**Functions to investigate:**
- `compute_edges` - Edge computation, may be from old graph system
- `find_test_targets` - Test detection, may be planned feature
- `insert_edges` - Edge insertion, may be from old graph system
- `is_route_chunk` - Route detection (React/Next.js), may be planned feature
- `is_test_chunk` - Test file detection, may be planned feature
- `as_str` - String conversion method
- `create_context_item` - Context assembly helper
- `evict_lru_if_needed` - Cache eviction helper

**Investigation steps for each:**
1. Search for references with ripgrep: `rg "function_name" crates/maproom/`
2. Check git log for when/why it was added
3. Look for TODO comments nearby
4. Make removal vs allow decision

**All removed code is recoverable from git history.**

## Dependencies
- RSTFIX-1001: Auto-fix imports (must complete first)
- RSTFIX-2001, 2002, 2003: Phase 2 should complete to ensure no new dead code surfaces

## Risk Assessment
- **Risk**: Higher - removing functions is permanent (though recoverable from git)
  - **Mitigation**: Check git history and TODO comments before removal
- **Risk**: Code may be called through macros or dynamic dispatch
  - **Mitigation**: Full test suite will catch any broken functionality
- **Risk**: Code may be intentional scaffolding for future features
  - **Mitigation**: Use `#[allow(dead_code)]` with explanation rather than removing

## Files/Packages Affected
- Files will be determined during investigation
- Likely candidates based on analysis.md:
  - `crates/maproom/src/context/*.rs`
  - `crates/maproom/src/incremental/*.rs`
  - `crates/maproom/src/search/*.rs`
