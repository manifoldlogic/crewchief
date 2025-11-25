# Ticket: Implement SqliteStore Schema & Migrations

**ID:** SQLVEC-2002
**Phase:** 2
**Status:** Pending
**Assigned To:** Database Specialist

## Summary
Implement the schema creation logic for SQLite, mirroring the Postgres schema.

## Background
We need `files`, `chunks`, `repositories`, `worktrees` tables, plus the virtual tables for search.

## Acceptance Criteria
- [ ] `schema.rs` defines SQL statements for table creation.
- [ ] `vec_chunks` created using `vec0(embedding float[1536])`.
- [ ] `fts_chunks` created using `fts5(content, tokenizer='trigram')`.
- [ ] Standard relational tables created (foreign keys enabled).
- [ ] `initialize()` runs migrations idempotently.

## Technical Requirements
- **Idempotency**: Use `CREATE TABLE IF NOT EXISTS`.
- **FTS**: Use `trigram` tokenizer if available, or standard if not. Note: `trigram` might require extra build flags for SQLite itself. Stick to standard tokenizer first if complex.

## Implementation Notes
- Verify `sqlite-vec` table creation syntax matches the version we vendored.

## Dependencies
- SQLVEC-2001

## Risks
- Schema divergence from Postgres over time.
