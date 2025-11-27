# Ticket: Performance Benchmarking

**ID:** SQLVEC-3003
**Phase:** 3
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Benchmark the indexing and search performance of SQLite vs Postgres.

## Background
We expect SQLite to be faster for single-user, but maybe slower for concurrent indexing. We need numbers.

## Acceptance Criteria
- [ ] Criterion benchmarks created for `upsert` and `search`.
- [ ] Report generated comparing throughput/latency.
- [ ] Optimization of SQLite settings (page size, cache size) if needed.

## Technical Requirements
- **Tool**: `criterion`.

## Implementation Notes
- Don't block release on this unless it's unusable.

## Dependencies
- SQLVEC-3001

## Risks
- SQLite being too slow for large repos.

