-- Test Database Schema Initialization
--
-- This script creates the complete Maproom schema for integration tests.
-- Combines the base schema (0001_init.sql) with migrations 0018-0020.
--
-- All statements use IF NOT EXISTS for idempotency (safe to run multiple times).
--
-- Related: MCPSIMP-4003

-- ============================================================================
-- Extensions
-- ============================================================================

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- ============================================================================
-- Schema
-- ============================================================================

CREATE SCHEMA IF NOT EXISTS maproom;

-- ============================================================================
-- TABLE: repos
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.repos (
  id BIGSERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  root_path TEXT NOT NULL
);

-- ============================================================================
-- TABLE: worktrees
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.worktrees (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  abs_path TEXT NOT NULL,
  UNIQUE (repo_id, name)
);

-- ============================================================================
-- TABLE: commits
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.commits (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  sha TEXT NOT NULL,
  committed_at TIMESTAMPTZ,
  UNIQUE (repo_id, sha)
);

-- ============================================================================
-- TABLE: files
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.files (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  worktree_id BIGINT REFERENCES maproom.worktrees(id) ON DELETE SET NULL,
  commit_id BIGINT NOT NULL REFERENCES maproom.commits(id) ON DELETE CASCADE,
  relpath TEXT NOT NULL,
  language TEXT,
  content_hash TEXT NOT NULL,
  size_bytes INT,
  last_modified TIMESTAMPTZ,
  UNIQUE (commit_id, relpath, content_hash)
);

-- ============================================================================
-- TYPE: symbol_kind
-- ============================================================================

DO $$ BEGIN
  CREATE TYPE maproom.symbol_kind AS ENUM ('func','class','component','hook','module','var','type','other');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ============================================================================
-- TABLE: chunks (base + migration 0018 blob_sha + migration 0020 worktree_ids)
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.chunks (
  id BIGSERIAL PRIMARY KEY,
  file_id BIGINT NOT NULL REFERENCES maproom.files(id) ON DELETE CASCADE,
  symbol_name TEXT,
  kind maproom.symbol_kind,
  signature TEXT,
  docstring TEXT,
  start_line INT NOT NULL,
  end_line INT NOT NULL,
  preview TEXT,
  ts_doc TSVECTOR,
  code_embedding VECTOR(1536),
  text_embedding VECTOR(1536),
  recency_score REAL DEFAULT 1.0,
  churn_score REAL DEFAULT 0.0,
  -- Migration 0018: blob_sha column
  blob_sha TEXT NOT NULL DEFAULT '',
  -- Migration 0020: worktree_ids column
  worktree_ids JSONB NOT NULL DEFAULT '[]',
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(file_id, start_line, end_line)
);

-- ============================================================================
-- TYPE: edge_type
-- ============================================================================

DO $$ BEGIN
  CREATE TYPE maproom.edge_type AS ENUM ('imports','exports','calls','called_by','test_of','route_of');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

-- ============================================================================
-- TABLE: chunk_edges
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.chunk_edges (
  src_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  dst_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  type maproom.edge_type NOT NULL,
  PRIMARY KEY (src_chunk_id, dst_chunk_id, type)
);

-- ============================================================================
-- TABLE: file_owners
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.file_owners (
  file_id BIGINT REFERENCES maproom.files(id) ON DELETE CASCADE,
  owner TEXT NOT NULL,
  PRIMARY KEY(file_id, owner)
);

-- ============================================================================
-- TABLE: test_links
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.test_links (
  test_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  target_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  PRIMARY KEY(test_chunk_id, target_chunk_id)
);

-- ============================================================================
-- TABLE: code_embeddings (migration 0019)
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding VECTOR(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);

-- ============================================================================
-- TABLE: worktree_index_state (migration 0020)
-- ============================================================================

CREATE TABLE IF NOT EXISTS maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  last_tree_sha TEXT NOT NULL,
  last_indexed TIMESTAMP DEFAULT NOW(),
  chunks_processed INT DEFAULT 0,
  embeddings_generated INT DEFAULT 0
);

-- ============================================================================
-- INDEXES
-- ============================================================================

-- Full-text search index
CREATE INDEX IF NOT EXISTS idx_chunks_tsv ON maproom.chunks USING GIN (ts_doc);

-- Trigram index for file path search
CREATE INDEX IF NOT EXISTS idx_files_relpath_trgm ON maproom.files USING GIN (relpath gin_trgm_ops);

-- Vector indexes for embedding similarity search
CREATE INDEX IF NOT EXISTS idx_chunks_code_vec ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops) WITH (lists = 200);
CREATE INDEX IF NOT EXISTS idx_chunks_text_vec ON maproom.chunks USING ivfflat (text_embedding vector_cosine_ops) WITH (lists = 200);

-- Migration 0018: blob_sha index
CREATE INDEX IF NOT EXISTS idx_chunks_blob_sha ON maproom.chunks(blob_sha);

-- Migration 0019: embeddings vector index (HNSW for fast approximate nearest neighbor)
CREATE INDEX IF NOT EXISTS idx_embeddings_vector ON maproom.code_embeddings USING hnsw (embedding vector_cosine_ops);

-- Migration 0020: worktree_ids GIN index
CREATE INDEX IF NOT EXISTS idx_chunks_worktree_ids ON maproom.chunks USING gin(worktree_ids);

-- Migration 0020: worktree_index_state tree_sha index
CREATE INDEX IF NOT EXISTS idx_worktree_index_state_tree_sha ON maproom.worktree_index_state(last_tree_sha);

-- ============================================================================
-- FUNCTION: compute_git_blob_sha (migration 0018)
-- ============================================================================

CREATE OR REPLACE FUNCTION maproom.compute_git_blob_sha(content TEXT)
RETURNS TEXT AS $$
  SELECT encode(
    digest(
      convert_to('blob ' || length(content), 'UTF8') || '\x00'::bytea || convert_to(content, 'UTF8'),
      'sha256'::text
    ),
    'hex'
  );
$$ LANGUAGE SQL IMMUTABLE;

-- ============================================================================
-- Schema initialization complete
-- ============================================================================
