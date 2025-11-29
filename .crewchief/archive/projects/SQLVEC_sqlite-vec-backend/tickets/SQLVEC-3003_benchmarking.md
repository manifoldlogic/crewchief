# Ticket: Performance Tuning and Benchmarking

**ID:** SQLVEC-3003
**Phase:** 3
**Status:** Pending
**Assigned To:** Performance Engineer

## Summary
Benchmark the SQLite implementation and tune pragmas for performance.

## Background
SQLite defaults are slow. We need WAL mode, appropriate page size, etc.

## Acceptance Criteria
- [ ] Benchmarks run for indexing 10k files.
- [ ] Benchmarks run for searching (p95 latency).
- [ ] SQLite pragmas tuned: `journal_mode=WAL`, `synchronous=NORMAL`, `mmap_size`.

## Technical Requirements
- **Tools**: `criterion` or simple timing scripts.

## Implementation Notes
- Compare against Postgres baseline.

## Dependencies
- SQLVEC-3002

## Risks
- SQLite might be significantly slower for heavy write loads (indexing).

