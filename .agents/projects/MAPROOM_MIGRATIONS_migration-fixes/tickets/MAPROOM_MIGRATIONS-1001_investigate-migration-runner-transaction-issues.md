# Ticket: MAPROOM_MIGRATIONS-1001: Investigate Migration Runner Transaction Issues

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Investigate and document root cause of "CREATE INDEX CONCURRENTLY cannot run inside a transaction block" errors in the migration runner. Multiple migrations (0004, 0008, 0010, 0012, 0015) use CONCURRENT indexes and fail even when using `simple_query()` approach.

## Background
During PROVFIX work, we discovered the migration runner cannot apply migrations cleanly:
1. Migration 0004 uses CREATE INDEX CONCURRENTLY but was in the batch_execute array
2. Migration 0008+ use `execute_with_concurrent_indexes()` which calls `simple_query()`
3. Even with simple_query, we get "cannot run inside a transaction block" errors
4. We manually applied migration 0016 as a workaround

This is blocking clean database setup and preventing automated migration application. The issue suggests fundamental problems with how we're managing database connections and transactions in the migration runner.

**Related Planning Documents:**
- Project location: `.agents/projects/MAPROOM_MIGRATIONS_migration-runner-fixes`
- This is a discovery ticket to inform implementation approach

## Acceptance Criteria
- [x] Document why `simple_query()` still triggers transaction errors despite being outside transactions
- [x] Identify if manual schema edits (init.sql) created inconsistencies between schema and migrations
- [x] Determine if migration tracking is needed (e.g., a migrations table to track applied migrations)
- [x] Analyze all CONCURRENT index migrations (0004, 0008, 0010, 0012, 0015) for common patterns
- [x] Document findings in a clear technical report (markdown file in project directory)
- [x] Create specific recommendations for implementation approach
- [x] Create follow-up implementation ticket(s) based on findings

## Technical Requirements
- Analyze `/workspace/crates/maproom/src/db/queries.rs` migration runner implementation
- Review all migration files in `/workspace/crates/maproom/migrations/`
- Understand tokio-postgres connection and transaction behavior
- Review `batch_execute()` vs `simple_query()` behavior
- Check database connection settings that might force transaction mode
- Compare schema state from manual init.sql vs migration-based schema
- Test migration runner behavior in isolated environment

## Implementation Notes

### Files to Investigate
- `/workspace/crates/maproom/src/db/queries.rs` - Migration runner implementation
- `/workspace/crates/maproom/migrations/` - All migration files, especially:
  - `0004_create_indexes.sql` - First CONCURRENT index migration
  - `0008_add_graph_indexes.sql` - Uses execute_with_concurrent_indexes()
  - `0010_add_symbol_metadata_index.sql`
  - `0012_create_gin_indexes.sql`
  - `0015_additional_gin_indexes.sql`

### Known Issues
- `batch_execute()` may leave connection in transaction mode
- `simple_query()` protocol may still wrap in transactions under certain conditions
- No migration tracking means no idempotency (can't safely re-run)
- Manual schema initialization may have diverged from migration history

### Questions to Answer
1. Why does PostgreSQL report transaction mode even with simple_query()?
2. Is the connection pooling or configuration forcing transaction mode?
3. Do we need explicit `BEGIN`/`COMMIT` control in the migration runner?
4. Should we use separate connections for CONCURRENT operations?
5. Is there a schema version mismatch between init.sql and migrations?
6. Do we need a `schema_migrations` table for tracking?

### Investigation Approach
1. Review tokio-postgres documentation on transaction behavior
2. Test simple_query() with CONCURRENT indexes in isolation
3. Compare database schema after init.sql vs after running all migrations
4. Check for implicit transaction wrapping in tokio-postgres
5. Look for connection settings that might affect transaction mode
6. Research PostgreSQL CONCURRENT index requirements

## Dependencies
- None (this is a discovery/investigation ticket)

## Risk Assessment
- **Risk**: Investigation may reveal need for major refactoring of migration system
  - **Mitigation**: Document findings clearly to inform decision-making; break implementation into phases if needed

- **Risk**: Current workaround (manual SQL) may have created schema drift
  - **Mitigation**: Include schema comparison in investigation; document any inconsistencies found

- **Risk**: May discover additional issues beyond transaction handling
  - **Mitigation**: Prioritize findings and create separate tickets for distinct issues

## Files/Packages Affected
- `/workspace/crates/maproom/src/db/queries.rs` (investigation only, no changes)
- `/workspace/crates/maproom/migrations/*.sql` (investigation only, no changes)
- Investigation output: New markdown document with findings (to be created in project directory)
- Follow-up: Implementation tickets to be created based on findings
