-- Migration 0015: Add 768-dimensional embedding columns for Ollama and Google providers
-- Estimated duration: < 1 minute for 25K chunks
-- Safety: Non-blocking (CONCURRENTLY), idempotent (IF NOT EXISTS)
--
-- This migration adds support for multiple embedding providers:
-- - Ollama (nomic-embed-text): 768 dimensions
-- - Google Vertex AI (text-embedding-gecko): 768 dimensions
--
-- Existing OpenAI columns (1536 dimensions) remain unchanged.
-- The new columns will initially be NULL and populated by the embedding service.

BEGIN;

-- Add 768-dimensional columns for Ollama and Google providers
-- These columns will store embeddings from alternative providers to OpenAI
ALTER TABLE maproom.chunks
  ADD COLUMN IF NOT EXISTS code_embedding_ollama vector(768),
  ADD COLUMN IF NOT EXISTS text_embedding_ollama vector(768);

-- Add column comments for in-database documentation
COMMENT ON COLUMN maproom.chunks.code_embedding_ollama IS
  'Code embeddings from Ollama (nomic-embed-text) or Google Vertex AI (text-embedding-gecko) - 768 dimensions';

COMMENT ON COLUMN maproom.chunks.text_embedding_ollama IS
  'Text summary embeddings from Ollama or Google Vertex AI - 768 dimensions';

COMMIT;

-- Create IVFFlat indexes OUTSIDE transaction (CONCURRENTLY requires this)
-- Estimated duration: ~30-60 seconds for 25K chunks
--
-- IVFFlat parameters:
-- - lists = 200: Optimal for ~25K chunks (sqrt(25000) ≈ 158, rounded up for growth)
-- - vector_cosine_ops: Cosine similarity operator (same as OpenAI indexes)
--
-- CONCURRENTLY ensures non-blocking index creation:
-- - Reads and writes continue during index build
-- - Safe for production deployment
-- - Cannot run inside a transaction block

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_code_vec_ollama
  ON maproom.chunks
  USING ivfflat (code_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_chunks_text_vec_ollama
  ON maproom.chunks
  USING ivfflat (text_embedding_ollama vector_cosine_ops)
  WITH (lists = 200);
