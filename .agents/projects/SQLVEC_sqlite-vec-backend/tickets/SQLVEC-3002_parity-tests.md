# Ticket: Shared Integration Tests & Parity Check

**ID:** SQLVEC-3002
**Phase:** 3
**Status:** Pending
**Assigned To:** Quality Engineer

## Summary
Create a shared integration test suite that runs against both backends to verify functional parity.

## Background
We need to trust that SQLite behaves like Postgres for the core use cases.

## Acceptance Criteria
- [ ] `tests/store_compat.rs` created.
- [ ] Tests cover: indexing a file, searching by text, searching by vector, deleting.
- [ ] CI pipeline runs these tests for both backends.

## Technical Requirements
- **Test Matrix**: Use a macro or generic test function to run against `Box<dyn VectorStore>`.

## Implementation Notes
- Use `tempfile` for SQLite databases in tests.

## Dependencies
- SQLVEC-3001

## Risks
- Flaky tests due to timing/async differences.

