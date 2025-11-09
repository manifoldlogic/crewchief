# Maproom Test Fixtures

This directory contains test fixtures for the Maproom database migration and testing.

## mpembed_baseline_100.sql

**Purpose**: Fast-loading test fixture for multi-provider embedding migration (MPEMBED project)

**Contents**:
- 100 representative chunks sampled using stratified sampling
  - 50 TypeScript chunks (functions, classes, modules)
  - 30 Rust chunks (functions, modules, structs, impls, uses)
  - 20 Markdown chunks (sections, headings, code blocks)
- 86 unique files containing these chunks
- All related database records (repos, worktrees, commits, files)
- Preserves OpenAI embeddings (code_embedding, text_embedding) when present

**Load Time**: ~33ms (well under <5 second requirement)

**File Size**: ~192KB

## Usage

### Load fixture into a database:

```bash
psql $MAPROOM_DATABASE_URL < tests/fixtures/mpembed_baseline_100.sql
```

### Regenerate fixture:

```bash
cd crates/maproom
./scripts/create_fixture.sh
```

The script will:
1. Connect to maproom-postgres database (production-like instance)
2. Use stratified sampling to select diverse chunks
3. Export all related data with proper FK relationships
4. Save to `tests/fixtures/mpembed_baseline_100.sql`

### Verify fixture after loading:

```sql
-- Check chunk distribution by language
SELECT
  f.language,
  COUNT(*) as chunk_count
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
GROUP BY f.language
ORDER BY chunk_count DESC;

-- Check chunk distribution by kind
SELECT
  c.kind::text,
  COUNT(*) as count
FROM maproom.chunks c
GROUP BY c.kind
ORDER BY count DESC
LIMIT 10;

-- Check embedding status
SELECT
  COUNT(*) as total_chunks,
  COUNT(CASE WHEN code_embedding IS NOT NULL THEN 1 END) as with_code_emb,
  COUNT(CASE WHEN text_embedding IS NOT NULL THEN 1 END) as with_text_emb
FROM maproom.chunks;
```

## Notes

- The fixture is generated from production data, so embeddings may be NULL if not yet generated
- Stratified sampling ensures representative coverage of different file types and chunk kinds
- The fixture preserves all FK relationships and can be loaded into a clean database
- Sequence values are updated after load to avoid conflicts with future inserts
