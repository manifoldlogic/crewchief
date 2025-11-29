# Ticket: RSTFIX-3002: Remove Unused Struct Fields

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
Remove or address dead code warnings for the `Edge` struct and unused struct fields across the codebase. Approximately 4 warnings related to unused data structures.

## Background
The codebase contains unused structs and struct fields, primarily the `Edge` struct which appears to be from an older graph implementation that was replaced during the SQLIMPL migration. This is Phase 3 of the RSTFIX cleanup project.

Reference: Phase 3 in planning/plan.md - "Dead Code (Higher Risk)"

## Acceptance Criteria
- [ ] `Edge` struct either removed or marked with `#[allow(dead_code)]` with justification
- [ ] All unused struct fields either removed or marked with appropriate allow attribute
- [ ] No dead_code warnings for struct definitions
- [ ] No dead_code warnings for struct fields
- [ ] All 906 tests pass

## Technical Requirements
- Follow the dead code decision tree from architecture.md
- If a struct field is used in construction but never read, consider using `_` in destructuring patterns or removing
- If an entire struct is unused, check if it's part of a planned API or can be removed
- Check for serde derives - structs may be used for serialization even if fields seem unused in Rust code

## Implementation Notes
**Structs to investigate:**
- `Edge` struct - From old code graph system, likely replaceable
- Multiple unnamed fields in various structs

**Struct field considerations:**
1. Is the struct serialized/deserialized? → Fields may be needed for JSON/YAML
2. Is the struct part of a public API? → Fields may be needed for compatibility
3. Is the struct used in pattern matching? → May need `_` wildcard handling

**Edge struct context:**
The `Edge` struct was likely part of the code relationship graph that stored edges between chunks. After SQLIMPL migration, this may have been replaced by database-backed edges. Check:
- `chunk_edges` table in database schema
- Whether `Edge` struct is used in any queries

**Removal vs #[allow]:**
- Unused struct with no clear future? → Remove
- Struct that's part of evolving API? → `#[allow(dead_code)]` with comment

## Dependencies
- RSTFIX-1001: Auto-fix imports (must complete first)
- RSTFIX-3001: Remove dead functions (may reveal more struct usage info)

## Risk Assessment
- **Risk**: Higher - struct removal affects type system
  - **Mitigation**: Full test suite will catch type errors
- **Risk**: Struct may be used in serialization not caught by dead_code lint
  - **Mitigation**: Check for serde derives and JSON/YAML usage
- **Risk**: Struct may be part of planned API expansion
  - **Mitigation**: Use `#[allow(dead_code)]` with future-intent comment

## Files/Packages Affected
- Location of `Edge` struct to be determined during investigation
- Likely in `crates/maproom/src/context/` or `crates/maproom/src/incremental/`
