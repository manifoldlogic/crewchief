# Ticket: RSTFIX-2002: Fix Unused Variables in Context Module

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
Fix unused variable warnings across the context module including cache.rs, graph.rs, relationships.rs, and all strategy files (python.rs, react.rs, rust.rs, typescript.rs). Approximately 10-12 warnings involving variables like `worktree_id`, `EdgeType`, and context-related data.

## Background
The context module provides code context assembly for search results. Several unused variables remain from iterative development of language-specific strategies and the context graph implementation. This is Phase 2 of the RSTFIX cleanup project, focusing on medium-risk unused variable warnings.

Reference: Phase 2 in planning/plan.md - "Unused Variables (Medium Risk)"

## Acceptance Criteria
- [ ] No unused variable warnings in `src/context/cache.rs`
- [ ] No unused variable warnings in `src/context/graph.rs`
- [ ] No unused variable warnings in `src/context/relationships.rs`
- [ ] No unused variable warnings in `src/context/strategies/*.rs` (python.rs, react.rs, rust.rs, typescript.rs)
- [ ] No unused variable warnings in `src/context/detectors/*.rs` (hooks.rs, jsx.rs)
- [ ] All 906 tests pass after changes
- [ ] Context assembly functionality remains intact

## Technical Requirements
- For truly unused variables: prefix with `_` (e.g., `_worktree_id`)
- For unused enum variants or type imports: remove if truly unused, or allow if intentionally reserved
- Pay attention to `EdgeType` usage in relationships.rs - may be intentionally scoped for future use
- Preserve all existing behavior - this is cleanup only, no functional changes
- Run context-related tests specifically to verify no regression

## Implementation Notes

**Files and likely issues:**
- `context/cache.rs` - Unused variables in LRU cache implementation
- `context/graph.rs` - Unused context graph parameters
- `context/relationships.rs` - Unused `EdgeType` import/variant
- `context/strategies/python.rs` - Unused detection parameters
- `context/strategies/react.rs` - Unused JSX-related variables
- `context/strategies/rust.rs` - Unused module analysis variables
- `context/strategies/typescript.rs` - Unused type analysis variables
- `context/detectors/hooks.rs` - Unused hook detection variables
- `context/detectors/jsx.rs` - Unused JSX component variables

**Decision tree:**
1. Parameter unused? → Prefix with `_`
2. Local variable unused? → Remove declaration or prefix with `_` if still semantically useful
3. Type/enum import unused? → Remove import
4. May be intentional for future? → Add `#[allow(dead_code)]` with comment explaining why

**Testing strategy:**
After fixes, specifically run tests related to:
- Context assembly (`cargo test context`)
- Language-specific strategies (`cargo test strategies`)
- Full test suite to catch any regressions

## Dependencies
- RSTFIX-1001: Auto-fix imports must complete first
- Can run in parallel with RSTFIX-2001 (search module) after imports are fixed

## Risk Assessment
- **Risk**: Medium - context assembly is a key feature, must not break functionality
  - **Mitigation**: Run context-related tests specifically after changes; verify context assembly still works correctly
- **Risk**: Some variables may be placeholders for future language strategies
  - **Mitigation**: Use `_` prefix to preserve intent; document any obviously incomplete code with comments

## Files/Packages Affected
- `crates/maproom/src/context/cache.rs`
- `crates/maproom/src/context/graph.rs`
- `crates/maproom/src/context/relationships.rs`
- `crates/maproom/src/context/strategies/python.rs`
- `crates/maproom/src/context/strategies/react.rs`
- `crates/maproom/src/context/strategies/rust.rs`
- `crates/maproom/src/context/strategies/typescript.rs`
- `crates/maproom/src/context/detectors/hooks.rs`
- `crates/maproom/src/context/detectors/jsx.rs`
