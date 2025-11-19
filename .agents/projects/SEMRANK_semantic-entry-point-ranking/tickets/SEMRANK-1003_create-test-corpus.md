# Ticket: SEMRANK-1003: Create Test Corpus Repository

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
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create sample repository with known structure across 3 languages (Rust, TypeScript, Python) to validate semantic ranking correctness.

## Background
Phase 1 of SEMRANK requires a controlled test corpus to validate that implementations rank above tests/docs. This test corpus must have a controlled structure with known chunk kinds and symbol names that can be verified after indexing. The corpus will be indexed in SEMRANK-1004 and used throughout Phase 3 testing to verify semantic ranking improvements.

Scope is intentionally constrained to prevent delays: 30-50 chunks total, with a strict 1-day time box. If creation exceeds this time, we will fall back to using a subset of the existing maproom codebase.

This ticket implements the test infrastructure portion of the Phase 1 plan.

## Acceptance Criteria
- [ ] Test repository created with 30-50 chunks across 3 languages (Rust, TypeScript, Python)
- [ ] Structure per language includes: 5 functions + 3 tests + 2 docs = 10 chunks minimum
- [ ] File path examples match specification: `src/auth/validate.rs`, `tests/auth_test.rs`, `docs/auth_guide.md`
- [ ] Representative samples are simple and self-contained (NOT full applications)
- [ ] Variety in chunk kinds: func, class, component, hook, module, heading_1, heading_2
- [ ] All files are valid and parseable by tree-sitter (no syntax errors)

## Technical Requirements
- **Scope Constraint**: 50 chunks maximum, 1 day time box
- **Fallback Strategy**: Use existing maproom codebase subset if creation exceeds 1 day
- **Languages**: Rust, TypeScript, Python (maproom already supports these well)
- **Complexity**: Simple functions to avoid dependency complexity
- **Example Structure**:
  ```
  test-repo/
  ├── rust/
  │   ├── src/auth/authenticate.rs (fn authenticate)
  │   ├── tests/auth_test.rs (test functions)
  │   └── docs/auth_guide.md (markdown headings)
  ├── typescript/
  │   ├── src/auth.ts (function authenticate)
  │   ├── __tests__/auth.test.ts (test functions)
  │   └── README.md (documentation)
  └── python/
      ├── src/auth.py (def authenticate)
      ├── tests/test_auth.py (test functions)
      └── docs/api.md (documentation)
  ```

## Implementation Notes
- Create in `/tmp/semrank-test-corpus` or similar temporary location
- Functions should be simple (10-20 lines each)
- Tests should reference the functions by name (for term frequency testing)
- Documentation should mention function names (for ranking comparison)
- Prioritize variety in chunk kinds over depth of functionality
- Ensure files are syntactically valid for tree-sitter parsing
- Use common patterns from each language (e.g., React hooks for TypeScript, pytest for Python)

## Dependencies
- None

## Risk Assessment
- **Risk**: Scope creep into full applications with complex dependencies
  - **Mitigation**: Strictly time-box to 1 day, use fallback plan
- **Risk**: Tree-sitter parsing failures on created files
  - **Mitigation**: Test that files are syntactically valid
- **Risk**: Insufficient variety in chunk kinds
  - **Mitigation**: Ensure all major kinds represented (func, class, component, hook, module, heading_*)

## Files/Packages Affected
- Test corpus directory structure (outside main codebase, temporary location)
- 30-50 source files across 3 languages (Rust, TypeScript, Python)
