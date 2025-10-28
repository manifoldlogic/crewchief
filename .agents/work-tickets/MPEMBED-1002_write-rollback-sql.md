# Ticket: MPEMBED-1002: Write rollback SQL migration for 768-dim columns

## Status
- [x] **Task completed** - acceptance criteria met, critical bug fixed
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Bug Fix Notes
Fixed critical idempotency bug where DO block queried columns without checking existence first.

**Bug**: Lines 20-34 queried `code_embedding_ollama` and `text_embedding_ollama` columns directly, causing ERROR when columns don't exist.

**Fix**: Added column existence check using information_schema.columns before querying data:
- First check if columns exist in information_schema
- Only query for data if columns exist
- Provide appropriate notices for all states (columns exist with data, exist without data, don't exist)

**Testing**: Complete round-trip tested successfully:
1. Rollback on clean database (no columns) - SUCCESS with notice "Ollama/Google columns do not exist"
2. Forward migration - Creates 2 columns + 2 indexes
3. Rollback with data - Shows WARNING about data loss
4. Rollback again (idempotency) - SUCCESS with notice "Ollama/Google columns do not exist"

**Result**: Rollback is now fully idempotent and safe to run multiple times or in any database state.

## Agents
- migration-safety-specialist
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create rollback script that safely removes 768-dim columns and indexes if migration needs to be reversed.

## Background
Every production migration needs a tested rollback procedure. The rollback must handle partial migration state (columns exist but indexes don't) and must not affect existing OpenAI embeddings. It must document when rollback is safe (before Ollama embeddings generated) and warn about data loss if Ollama embeddings already exist.

This implements Phase 1: Database Migration rollback safety from the MPEMBED multi-provider embeddings plan.

## Acceptance Criteria
- [x] Rollback SQL drops `idx_chunks_code_vec_ollama` index
- [x] Rollback SQL drops `idx_chunks_text_vec_ollama` index
- [x] Rollback SQL drops `code_embedding_ollama` column
- [x] Rollback SQL drops `text_embedding_ollama` column
- [x] Uses `IF EXISTS` for idempotency
- [x] Uses `DROP INDEX CONCURRENTLY` (no blocking)
- [x] Includes warnings about when rollback is safe
- [x] Existing OpenAI columns remain intact after rollback
- [x] DO block checks column existence before querying (idempotency fix)

## Technical Requirements
- File location: `crates/maproom/migrations/0015_add_ollama_columns_rollback.sql`
- Check for data before dropping (warn if non-NULL values exist)
- Document data loss implications (any Ollama embeddings will be lost)
- Must be tested on fixture before production use
- Use PL/pgSQL for warning logic

## Implementation Notes
The rollback SQL should follow this structure:

```sql
-- migration 0015_add_ollama_columns_rollback.sql
-- ROLLBACK SAFETY: Only run if:
--   1. No Ollama embeddings generated yet (all columns are NULL), OR
--   2. You're okay losing Ollama embeddings and re-embedding later

BEGIN;

-- Check for data before dropping (warning only, doesn't prevent)
DO $$
DECLARE
  ollama_count INTEGER;
BEGIN
  SELECT COUNT(*) INTO ollama_count
  FROM maproom.chunks
  WHERE code_embedding_ollama IS NOT NULL OR text_embedding_ollama IS NOT NULL;

  IF ollama_count > 0 THEN
    RAISE WARNING 'Found % chunks with Ollama embeddings. These will be LOST if you proceed!', ollama_count;
  ELSE
    RAISE NOTICE 'No Ollama embeddings found. Safe to rollback.';
  END IF;
END $$;

COMMIT;

-- Drop indexes OUTSIDE transaction (CONCURRENTLY requires this)
-- Estimated duration: < 1 second
DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_code_vec_ollama;
DROP INDEX CONCURRENTLY IF EXISTS idx_chunks_text_vec_ollama;

BEGIN;

-- Drop columns (will fail if dependencies exist, which is good)
ALTER TABLE maproom.chunks
  DROP COLUMN IF EXISTS code_embedding_ollama,
  DROP COLUMN IF EXISTS text_embedding_ollama;

COMMIT;
```

Key considerations:
- Warning mechanism alerts operators about data loss
- IF EXISTS prevents errors on partial rollback
- CONCURRENTLY prevents blocking
- Column drop will fail if foreign key dependencies exist (safety feature)

## Dependencies
- MPEMBED-1001 (rollback corresponds to forward migration)

## Risk Assessment
- **Risk**: Accidental rollback after Ollama embeddings generated leads to data loss
  - **Mitigation**: Warning message shows count of non-NULL values, requires explicit confirmation
- **Risk**: Partial rollback leaves database in inconsistent state
  - **Mitigation**: IF EXISTS makes rollback idempotent, can be re-run safely

## Files/Packages Affected
- crates/maproom/migrations/0015_add_ollama_columns_rollback.sql (create)
