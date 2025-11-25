# Ticket: Implement SqliteStore FTS Operations

**ID:** SQLVEC-2004
**Phase:** 2
**Status:** Pending
**Assigned To:** Database Specialist

## Summary
Implement Full Text Search logic using SQLite FTS5.

## Background
Postgres uses `tsvector`/`tsquery`. SQLite uses FTS5. The query syntax is different.

## Acceptance Criteria
- [ ] `upsert_chunks` writes content to `fts_chunks` table.
- [ ] `search` constructs valid FTS5 queries (handling boolean operators if supported).
- [ ] Hybrid search logic (if applicable) combines FTS scores with vector scores (likely application-side fusion or CTE).

## Technical Requirements
- **Query Construction**: Implement a builder that converts our generic `SearchQuery` object into FTS5 syntax.
- **Ranking**: Use `bm25()` function for scoring if available.

## Implementation Notes
- FTS5 is built-in to `rusqlite` bundled feature usually.

## Dependencies
- SQLVEC-2002

## Risks
- Search quality differences compared to Postgres.

