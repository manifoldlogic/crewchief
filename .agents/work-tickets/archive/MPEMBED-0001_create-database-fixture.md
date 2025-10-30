# Ticket: MPEMBED-0001: Create 100-chunk database test fixture

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Extract a representative 100-chunk fixture from production database for fast iteration during migration testing.

## Background
The production database has 23,632 chunks with existing OpenAI embeddings. Full migration testing is slow (~2-3 minutes per run), making TDD impractical during the multi-provider embedding migration.

This ticket implements the baseline fixture creation as outlined in Phase 0 (Pre-Implementation Setup) of the MPEMBED project plan. A small, fast-loading fixture with diverse chunk types is critical for rapid iteration during schema changes and provider implementation.

**Reference**: `crewchief_context/maproom/MPEMBED-multi-provider-embeddings/` - Phase 0, Day 0

## Acceptance Criteria
- [x] Fixture created from production database with 100 representative chunks
- [x] Fixture includes mix of file types: 50 TypeScript, 30 Rust, 20 Markdown
- [x] Fixture includes mix of chunk kinds: functions, classes, modules, structs, impls, markdown sections
- [x] Fixture preserves existing OpenAI embeddings (both code_embedding and text_embedding)
- [x] Fixture loads into empty database in <5 seconds (actual: ~33ms)
- [x] Script can regenerate fixture on demand (repeatable at crates/maproom/scripts/create_fixture.sh)

## Technical Requirements
- Use `pg_dump` with WHERE clause to extract specific chunks
- Include both schema and data for maproom.chunks, maproom.files tables
- Preserve FK relationships (chunk.file_id → files.id)
- Save as `.sql` file in `crates/maproom/tests/fixtures/mpembed_baseline_100.sql`
- Document selection criteria in script comments
- Use stratified sampling (by file type and chunk kind) to ensure representative sample

## Implementation Notes

Use pg_dump with stratified sampling to create a diverse fixture:

```bash
# Suggested approach:
pg_dump $DATABASE_URL \
  --table maproom.chunks \
  --table maproom.files \
  --data-only \
  --where "id IN (SELECT id FROM maproom.chunks ORDER BY random() LIMIT 100)" \
  > tests/fixtures/mpembed_baseline_100.sql
```

**Sampling Strategy**:
1. Query chunks grouped by file extension and chunk kind
2. Select proportional samples from each group
3. Verify embeddings exist for selected chunks
4. Export selected chunks + their parent files

**Verification**:
- Load fixture into fresh database
- Count chunks by type: `SELECT extension, count(*) FROM maproom.chunks c JOIN maproom.files f ON c.file_id = f.id GROUP BY extension`
- Verify embeddings non-null: `SELECT count(*) FROM maproom.chunks WHERE code_embedding IS NOT NULL`

## Dependencies
None (first ticket in MPEMBED project)

## Risk Assessment
- **Risk**: Random sampling might miss edge cases (very long chunks, special characters, null embeddings)
  - **Mitigation**: Use stratified sampling by file type and chunk kind; manually verify fixture includes edge cases

- **Risk**: Fixture becomes stale as production schema evolves
  - **Mitigation**: Document regeneration script so fixture can be updated as needed

## Files/Packages Affected
- crates/maproom/tests/fixtures/mpembed_baseline_100.sql (created - 192KB, 100 chunks)
- crates/maproom/scripts/create_fixture.sh (created - executable script)
- crates/maproom/tests/fixtures/README.md (created - documentation)

## Implementation Summary

Successfully created a fast-loading test fixture with the following characteristics:

**Performance**:
- Load time: ~33ms (well under <5 second requirement)
- File size: 192KB
- 100 chunks from 86 unique files

**Data Distribution**:
- 50 TypeScript chunks (functions, classes, modules)
- 30 Rust chunks (functions, modules, structs, impls, uses)
- 20 Markdown chunks (sections, headings, code blocks)

**Technical Approach**:
- Used PostgreSQL COPY format for reliable data export with proper escaping
- Implemented stratified sampling via SQL CTEs to ensure diverse representation
- Exported in dependency order: repos → worktrees → commits → files → chunks
- Preserved all FK relationships and embedding columns
- Script connects to maproom-postgres (production-like database)

**Files Created**:
1. `create_fixture.sh` - Repeatable script that:
   - Samples 100 chunks using stratified random sampling
   - Identifies all dependent records (files, commits, repos, worktrees)
   - Exports using PostgreSQL COPY format
   - Includes verification queries in output

2. `mpembed_baseline_100.sql` - SQL fixture that:
   - Loads data in correct dependency order
   - Disables triggers during load for performance
   - Updates sequences after load
   - Includes built-in verification queries

3. `README.md` - Documentation explaining:
   - Fixture contents and purpose
   - Usage instructions (load and regenerate)
   - Verification queries

The fixture is ready for use in migration testing and provider implementation.
