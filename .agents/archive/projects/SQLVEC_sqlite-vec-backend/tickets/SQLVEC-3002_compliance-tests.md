# Ticket: Implement Store Compliance Test Suite

**ID:** SQLVEC-3002
**Phase:** 3
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Create a unified test suite that runs against both backends to ensure they behave identically.

## Background
We need confidence that switching to SQLite doesn't break search relevance or logic.

## Acceptance Criteria
- [ ] `tests/store_compliance.rs` created.
- [ ] Tests:
  - Insert file + chunk -> Retrieve it.
  - Vector search (insert 2 vectors, search for near one).
  - FTS search (insert text, search keyword).
  - Update/Delete handling.
- [ ] CI runs these tests for both backends.

## Technical Requirements
- Use macros or a generic test runner to instantiate the store.

## Implementation Notes
- This is critical for quality assurance.

## Dependencies
- SQLVEC-3001

## Risks
- Slight differences in floating point precision or FTS ranking algorithms.

