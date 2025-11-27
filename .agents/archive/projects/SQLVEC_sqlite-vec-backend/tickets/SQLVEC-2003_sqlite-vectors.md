# Ticket: Implement SqliteStore Vector Operations

**ID:** SQLVEC-2003
**Phase:** 2
**Status:** Pending
**Assigned To:** Rust Engineer

## Summary
Implement vector insertion and nearest-neighbor search for SQLite.

## Background
This bridges the `VectorStore` vector methods to `sqlite-vec` SQL.

## Acceptance Criteria
- [ ] `upsert_chunks` writes embeddings to `vec_chunks` virtual table.
- [ ] `search` uses `vec_distance_cosine` (or equivalent `MATCH` syntax) for similarity search.
- [ ] Results are joined with metadata tables.

## Technical Requirements
- **Query**:
  ```sql
  SELECT rowid, distance
  FROM vec_chunks
  WHERE embedding MATCH ?
  ORDER BY distance
  LIMIT ?
  ```
  (Check specific `sqlite-vec` syntax, it changes often).

## Implementation Notes
- Ensure vectors are serialized correctly (JSON or binary blob) for the query parameter.

## Dependencies
- SQLVEC-2002

## Risks
- Syntax errors with the virtual table queries.

