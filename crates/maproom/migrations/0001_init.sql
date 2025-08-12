-- Maproom schema init

CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;

CREATE SCHEMA IF NOT EXISTS maproom;

CREATE TABLE IF NOT EXISTS maproom.repos (
  id BIGSERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  root_path TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS maproom.worktrees (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  abs_path TEXT NOT NULL,
  UNIQUE (repo_id, name)
);

CREATE TABLE IF NOT EXISTS maproom.commits (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  sha TEXT NOT NULL,
  committed_at TIMESTAMPTZ,
  UNIQUE (repo_id, sha)
);

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

DO $$ BEGIN
  CREATE TYPE maproom.symbol_kind AS ENUM ('func','class','component','hook','module','var','type','other');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

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
  UNIQUE(file_id, start_line, end_line)
);

DO $$ BEGIN
  CREATE TYPE maproom.edge_type AS ENUM ('imports','exports','calls','called_by','test_of','route_of');
EXCEPTION WHEN duplicate_object THEN NULL; END $$;

CREATE TABLE IF NOT EXISTS maproom.chunk_edges (
  src_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  dst_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  type maproom.edge_type NOT NULL,
  PRIMARY KEY (src_chunk_id, dst_chunk_id, type)
);

CREATE TABLE IF NOT EXISTS maproom.file_owners (
  file_id BIGINT REFERENCES maproom.files(id) ON DELETE CASCADE,
  owner TEXT NOT NULL,
  PRIMARY KEY(file_id, owner)
);

CREATE TABLE IF NOT EXISTS maproom.test_links (
  test_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  target_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  PRIMARY KEY(test_chunk_id, target_chunk_id)
);

CREATE INDEX IF NOT EXISTS idx_chunks_tsv            ON maproom.chunks USING GIN (ts_doc);
CREATE INDEX IF NOT EXISTS idx_files_relpath_trgm    ON maproom.files  USING GIN (relpath gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_chunks_code_vec       ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops) WITH (lists = 200);
CREATE INDEX IF NOT EXISTS idx_chunks_text_vec       ON maproom.chunks USING ivfflat (text_embedding vector_cosine_ops) WITH (lists = 200);


