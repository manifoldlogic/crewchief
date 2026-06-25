-- maproom PostgreSQL schema — Phase 1 base.
-- Postgres dialect of the SQLite runtime schema (src/db/sqlite/migrations.rs),
-- applying the arch-doc pivots: many-to-many chunk<->worktree junction (NOT a
-- chunks.worktree_ids array) and a content-addressed embedding pool (0002).
-- Authored fresh per spec §5; the legacy crates/maproom/migrations/*.sql files
-- are a schema reference only and are NOT wired to runtime.

-- pgvector must exist before any `vector`-typed object (0002). R-MIG-3.
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE repos (
    id        BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    root_path TEXT NOT NULL
);

CREATE TABLE worktrees (
    id       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    repo_id  BIGINT NOT NULL REFERENCES repos (id) ON DELETE CASCADE,
    name     TEXT NOT NULL,
    abs_path TEXT NOT NULL,
    UNIQUE (repo_id, name)
);

CREATE TABLE commits (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    repo_id      BIGINT NOT NULL REFERENCES repos (id) ON DELETE CASCADE,
    sha          TEXT NOT NULL,
    committed_at TIMESTAMPTZ,
    UNIQUE (repo_id, sha)
);

CREATE TABLE files (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    repo_id       BIGINT NOT NULL REFERENCES repos (id) ON DELETE CASCADE,
    -- SET NULL (not CASCADE): deleting a worktree must NOT cascade-delete files
    -- (and their content-shared chunks) that other worktrees still reference via
    -- the chunk_worktrees junction. Orphan chunks are GC'd explicitly in
    -- delete_worktree_data; embeddings are kept (persistent pool, R-WT-4).
    worktree_id   BIGINT REFERENCES worktrees (id) ON DELETE SET NULL,
    commit_id     BIGINT NOT NULL REFERENCES commits (id) ON DELETE CASCADE,
    relpath       TEXT NOT NULL,
    language      TEXT,
    content_hash  TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL,
    last_modified TIMESTAMPTZ,
    UNIQUE (commit_id, relpath, content_hash)
);

-- kind is free-form TEXT (no enum) to preserve parity with arbitrary kind
-- strings the SQLite backend stores. ts_doc is populated from ts_doc_text via
-- to_tsvector('simple', ...) at insert time (replaces SQLite's manual FTS5 row).
CREATE TABLE chunks (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    file_id       BIGINT NOT NULL REFERENCES files (id) ON DELETE CASCADE,
    blob_sha      TEXT NOT NULL,
    symbol_name   TEXT,
    kind          TEXT,
    signature     TEXT,
    docstring     TEXT,
    start_line    INTEGER,
    end_line      INTEGER,
    preview       TEXT,
    ts_doc        TSVECTOR,
    recency_score REAL DEFAULT 1.0,
    churn_score   REAL DEFAULT 0.0,
    metadata      JSONB,
    UNIQUE (file_id, start_line, end_line)
);

-- Authoritative many-to-many chunk<->worktree mapping (NOT chunks.worktree_ids).
CREATE TABLE chunk_worktrees (
    chunk_id    BIGINT NOT NULL REFERENCES chunks (id) ON DELETE CASCADE,
    worktree_id BIGINT NOT NULL REFERENCES worktrees (id) ON DELETE CASCADE,
    PRIMARY KEY (chunk_id, worktree_id)
);

-- type is free-form TEXT. Canonical stored values: imports/exports/calls/test_of/extends.
CREATE TABLE chunk_edges (
    src_chunk_id BIGINT NOT NULL REFERENCES chunks (id) ON DELETE CASCADE,
    dst_chunk_id BIGINT NOT NULL REFERENCES chunks (id) ON DELETE CASCADE,
    type         TEXT NOT NULL,
    PRIMARY KEY (src_chunk_id, dst_chunk_id, type)
);

CREATE TABLE index_state (
    id                   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    worktree_id          BIGINT NOT NULL UNIQUE REFERENCES worktrees (id) ON DELETE CASCADE,
    tree_sha             TEXT NOT NULL,
    chunks_processed     INTEGER,
    embeddings_generated INTEGER,
    last_indexed         TIMESTAMPTZ NOT NULL
);

CREATE TABLE encoding_runs (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    started_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at       TIMESTAMPTZ,
    status            TEXT NOT NULL DEFAULT 'running',
    total_chunks      BIGINT NOT NULL,
    chunks_completed  BIGINT NOT NULL DEFAULT 0,
    chunks_per_second DOUBLE PRECISION,
    last_batch_at     TIMESTAMPTZ,
    provider          TEXT,
    dimension         INTEGER
);

-- FTS GIN index over the tsvector column.
CREATE INDEX idx_chunks_ts_doc ON chunks USING GIN (ts_doc);
-- worktree -> chunk lookups.
CREATE INDEX idx_chunk_worktrees_worktree_id ON chunk_worktrees (worktree_id);
-- index_state tree lookups.
CREATE INDEX idx_index_state_tree_sha ON index_state (tree_sha);
