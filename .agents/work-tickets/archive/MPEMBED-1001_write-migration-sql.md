# Ticket: MPEMBED-1001: Write idempotent SQL migration to add 768-dim columns

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- migration-safety-specialist
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create production-safe SQL migration that adds `code_embedding_ollama` and `text_embedding_ollama` columns with IVFFlat indexes, preserving all existing data.

## Background
Current schema has `code_embedding vector(1536)` and `text_embedding vector(1536)` for OpenAI embeddings. We need to add 768-dimensional columns for Ollama and Google providers which both generate 768-dimensional embeddings. The migration must be idempotent (IF NOT EXISTS) for safe reruns and use CONCURRENTLY for index creation to avoid table locks during production deployment.

This implements Phase 1: Database Migration from the MPEMBED multi-provider embeddings plan.

## Acceptance Criteria
- [x] Migration SQL adds `code_embedding_ollama vector(768)` column
- [x] Migration SQL adds `text_embedding_ollama vector(768)` column
- [x] IVFFlat indexes created with `lists = 200` (optimal for ~25K chunks)
- [x] Migration uses `IF NOT EXISTS` for idempotency
- [x] Index creation uses `CREATE INDEX CONCURRENTLY` (no blocking)
- [x] Migration includes timing estimates in comments (< 1 minute expected)
- [x] Existing OpenAI columns and indexes untouched

## Technical Requirements
- File location: `crates/maproom/migrations/0015_add_ollama_columns.sql`
- Use vector_cosine_ops for similarity (same as existing indexes)
- Lists parameter: `sqrt(25000) ≈ 158` → use 200 for growth headroom
- Include safety checks: column existence verification
- Add detailed comments explaining dimension choice
- Column comments for documentation

## Implementation Notes
The migration SQL should follow this structure:

```sql
-- migration 0015_add_ollama_columns.sql
-- Estimated duration: < 1 minute for 25K chunks
-- Safety: Non-blocking (CONCURRENTLY), idempotent (IF NOT EXISTS)

BEGIN;

-- Add 768-dimensional columns for Ollama and Google providers
ALTER TABLE maproom.chunks
  ADD COLUMN IF NOT EXISTS code_embedding_ollama vector(768),
  ADD COLUMN IF NOT EXISTS text_embedding_ollama vector(768);

-- Add column comments for documentation
COMMENT ON COLUMN maproom.chunks.code_embedding_ollama IS
  'Code embeddings from Ollama (nomic-embed-text) or Google Vertex AI (text-embedding-gecko) - 768 dimensions';

COMMENT ON COLUMN maproom.chunks.text_embedding_ollama IS
  'Text summary embeddings from Ollama or Google Vertex AI - 768 dimensions';

COMMIT;

-- Create indexes OUTSIDE transaction (CONCURRENTLY requires this)
-- Estimated duration: ~30-60 seconds for 25K chunks
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_code_vec_ollama
  ON maproom.chunks
  USING ivfflat (code_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_text_vec_ollama
  ON maproom.chunks
  USING ivfflat (text_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);
```

Key considerations:
- ALTER TABLE operations in transaction for atomicity
- INDEX creation outside transaction (CONCURRENTLY requirement)
- IF NOT EXISTS prevents errors on reruns
- Column comments provide in-database documentation

## Dependencies
- MPEMBED-0001 (test on fixture first before production)

## Risk Assessment
- **Risk**: Index creation takes too long, blocking other operations
  - **Mitigation**: CONCURRENTLY allows concurrent reads, test on staging first
- **Risk**: Dimension mismatch if wrong vector size specified
  - **Mitigation**: Explicit vector(768) type enforces dimension validation

## Files/Packages Affected
- crates/maproom/migrations/0015_add_ollama_columns.sql (create)
