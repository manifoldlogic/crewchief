# Ticket: Implement SqliteStore Core Operations

**ID:** SQLVEC-2003
**Phase:** 2
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Implement the `SqliteStore` struct implementing `VectorStore` using `rusqlite`.

## Background
This provides the actual implementation for the local backend.

## Acceptance Criteria
- [ ] `SqliteStore` implements `upsert_file`, `insert_chunk`, `search`.
- [ ] `insert_chunk` writes to `chunks`, `vec_chunks`, and `fts_chunks` (transactionally).
- [ ] `search` performs hybrid search (FTS + Vector) manually (query FTS, query Vector, merge results).

## Technical Requirements
- **Hybrid Search**:
  1. FTS Search: `SELECT rowid, rank FROM fts_chunks WHERE ...`
  2. Vector Search: `SELECT chunk_id, distance FROM vec_chunks WHERE ...`
  3. RRF (Reciprocal Rank Fusion) in Rust to combine scores.

## Implementation Notes
- Unlike Postgres where we did it all in SQL, SQLite hybrid search is often easier/faster to merge in application logic due to virtual table limitations.

## Dependencies
- SQLVEC-2002
- SQLVEC-1001

## Risks
- Search performance without the DB doing the join.

