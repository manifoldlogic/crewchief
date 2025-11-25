# Ticket: Implement SQLite Schema and Migrations

**ID:** SQLVEC-2002
**Phase:** 2
**Status:** Pending
**Assigned To:** Database Specialist

## Summary
Create the SQLite equivalent of the Postgres schema, including FTS5 virtual tables and `vec0` vector tables.

## Background
Postgres uses `pgvector` and `tsvector`. SQLite uses `sqlite-vec` virtual tables and FTS5 virtual tables. The schemas are conceptually similar but syntactically different.

## Acceptance Criteria
- [ ] `migrations/sqlite/0001_init.sql` created.
- [ ] Table `files` created (standard SQL).
- [ ] Table `chunks` created.
- [ ] Virtual table `vec_chunks` using `vec0` created.
- [ ] Virtual table `fts_chunks` using `fts5` created.
- [ ] Triggers (if needed) to keep FTS/Vector tables in sync with `chunks`.

## Technical Requirements
- **Schema**:
  ```sql
  CREATE VIRTUAL TABLE vec_chunks USING vec0(
    chunk_id INTEGER PRIMARY KEY,
    embedding FLOAT[768]
  );
  ```

## Implementation Notes
- Keep relational data in standard tables (`chunks`) and only search data in virtual tables (`vec_chunks`, `fts_chunks`) linked by ID.

## Dependencies
- SQLVEC-2001

## Risks
- Trigger complexity. It might be easier to manage sync in application code (`SqliteStore`) rather than SQL triggers.

