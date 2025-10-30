-- init.sql - PostgreSQL initialization schema for Maproom LOCAL project
-- Auto-loaded on first container startup via docker-entrypoint-initdb.d
--
-- This schema supports hybrid search combining:
-- - Vector similarity search using pgvector (768-dimension nomic-embed-text embeddings)
-- - Full-text search using PostgreSQL tsvector/tsquery
-- - Graph relationships between code chunks
--
-- Database: PostgreSQL 16 with pgvector extension
-- Embedding Model: nomic-embed-text (768 dimensions)
-- Schema Namespace: maproom

-- Enable pgvector extension for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- Create maproom schema to avoid polluting public schema
CREATE SCHEMA IF NOT EXISTS maproom;

-- =============================================================================
-- TABLE: repositories
-- =============================================================================
-- Top-level container for a git repository
-- Each repository can have multiple worktrees
CREATE TABLE maproom.repositories (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE maproom.repositories IS 'Git repositories tracked by Maproom';
COMMENT ON COLUMN maproom.repositories.name IS 'Unique repository name identifier';

-- =============================================================================
-- TABLE: worktrees
-- =============================================================================
-- Git worktree within a repository
-- Provides isolation for different branches/features
CREATE TABLE maproom.worktrees (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL REFERENCES maproom.repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(repo_id, name)
);

COMMENT ON TABLE maproom.worktrees IS 'Git worktrees within repositories';
COMMENT ON COLUMN maproom.worktrees.repo_id IS 'Foreign key to parent repository';
COMMENT ON COLUMN maproom.worktrees.name IS 'Worktree name (e.g., branch name)';
COMMENT ON COLUMN maproom.worktrees.path IS 'Filesystem path to worktree';

-- =============================================================================
-- TABLE: files
-- =============================================================================
-- Individual files tracked within a worktree
-- Each file can contain multiple code chunks
CREATE TABLE maproom.files (
    id SERIAL PRIMARY KEY,
    worktree_id INTEGER NOT NULL REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
    relpath TEXT NOT NULL,
    file_type TEXT NOT NULL,
    size_bytes BIGINT,
    last_modified TIMESTAMPTZ,
    git_hash TEXT,
    UNIQUE(worktree_id, relpath)
);

COMMENT ON TABLE maproom.files IS 'Files tracked within worktrees';
COMMENT ON COLUMN maproom.files.worktree_id IS 'Foreign key to parent worktree';
COMMENT ON COLUMN maproom.files.relpath IS 'Relative path from worktree root';
COMMENT ON COLUMN maproom.files.file_type IS 'File extension (ts, rs, md, etc.)';
COMMENT ON COLUMN maproom.files.git_hash IS 'Git blob hash for change detection';

-- =============================================================================
-- TABLE: chunks
-- =============================================================================
-- Parsed code chunks with embeddings and full-text search tokens
-- Core table for hybrid search functionality
CREATE TABLE maproom.chunks (
    id BIGSERIAL PRIMARY KEY,
    file_id INTEGER NOT NULL REFERENCES maproom.files(id) ON DELETE CASCADE,
    symbol_name TEXT,
    kind TEXT NOT NULL,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    signature TEXT,
    docstring TEXT,
    preview TEXT NOT NULL,

    -- Vector embeddings (768 dimensions for nomic-embed-text model)
    -- IMPORTANT: These are 768-dim, not 1536-dim (OpenAI uses 1536)
    code_embedding vector(768),
    text_embedding vector(768),

    -- Full-text search tokens
    fts_tokens tsvector,

    -- Metadata timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE maproom.chunks IS 'Parsed code chunks with embeddings for hybrid search';
COMMENT ON COLUMN maproom.chunks.file_id IS 'Foreign key to parent file';
COMMENT ON COLUMN maproom.chunks.symbol_name IS 'Function/class/variable name';
COMMENT ON COLUMN maproom.chunks.kind IS 'Symbol type (function, class, interface, etc.)';
COMMENT ON COLUMN maproom.chunks.start_line IS 'Starting line number in file';
COMMENT ON COLUMN maproom.chunks.end_line IS 'Ending line number in file';
COMMENT ON COLUMN maproom.chunks.signature IS 'Function signature or type definition';
COMMENT ON COLUMN maproom.chunks.docstring IS 'Documentation comment';
COMMENT ON COLUMN maproom.chunks.preview IS 'Code preview (first few lines)';
COMMENT ON COLUMN maproom.chunks.code_embedding IS '768-dim vector embedding of code content (nomic-embed-text)';
COMMENT ON COLUMN maproom.chunks.text_embedding IS '768-dim vector embedding of documentation (nomic-embed-text)';
COMMENT ON COLUMN maproom.chunks.fts_tokens IS 'Full-text search tokens (tsvector)';

-- =============================================================================
-- INDEXES: Hybrid Search Performance
-- =============================================================================

-- Vector similarity indexes using ivfflat (approximate nearest neighbor)
-- lists=100: Initial configuration for small-to-medium datasets
-- Scale to sqrt(rows) as dataset grows (tune in Phase 4)
-- EXPLAIN ANALYZE target: <50ms p95 for k=10 vector search
CREATE INDEX idx_chunks_code_embedding ON maproom.chunks
    USING ivfflat (code_embedding vector_cosine_ops)
    WITH (lists = 100);

CREATE INDEX idx_chunks_text_embedding ON maproom.chunks
    USING ivfflat (text_embedding vector_cosine_ops)
    WITH (lists = 100);

COMMENT ON INDEX maproom.idx_chunks_code_embedding IS 'ivfflat index for code embedding similarity search (cosine distance)';
COMMENT ON INDEX maproom.idx_chunks_text_embedding IS 'ivfflat index for text embedding similarity search (cosine distance)';

-- Full-text search index using GIN (generalized inverted index)
-- Supports fast tsvector @@ tsquery operations
-- EXPLAIN ANALYZE target: <20ms p95 for simple text searches
CREATE INDEX idx_chunks_fts ON maproom.chunks USING GIN (fts_tokens);

COMMENT ON INDEX maproom.idx_chunks_fts IS 'GIN index for full-text search on tsvector tokens';

-- Standard B-tree indexes for common query patterns
CREATE INDEX idx_chunks_file_id ON maproom.chunks(file_id);
CREATE INDEX idx_chunks_kind ON maproom.chunks(kind);

COMMENT ON INDEX maproom.idx_chunks_file_id IS 'B-tree index for file-level chunk queries';
COMMENT ON INDEX maproom.idx_chunks_kind IS 'B-tree index for filtering by symbol kind';

-- =============================================================================
-- TABLE: chunk_edges
-- =============================================================================
-- Relationships between code chunks (imports, calls, dependencies)
-- Enables graph traversal queries for context assembly
CREATE TABLE maproom.chunk_edges (
    id BIGSERIAL PRIMARY KEY,
    from_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
    to_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
    edge_type TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(from_chunk_id, to_chunk_id, edge_type)
);

COMMENT ON TABLE maproom.chunk_edges IS 'Graph relationships between code chunks';
COMMENT ON COLUMN maproom.chunk_edges.from_chunk_id IS 'Source chunk (caller, importer, etc.)';
COMMENT ON COLUMN maproom.chunk_edges.to_chunk_id IS 'Target chunk (callee, imported symbol, etc.)';
COMMENT ON COLUMN maproom.chunk_edges.edge_type IS 'Relationship type (import, call, extends, etc.)';

-- Graph traversal indexes for efficient relationship queries
CREATE INDEX idx_edges_from ON maproom.chunk_edges(from_chunk_id);
CREATE INDEX idx_edges_to ON maproom.chunk_edges(to_chunk_id);
CREATE INDEX idx_edges_type ON maproom.chunk_edges(edge_type);

COMMENT ON INDEX maproom.idx_edges_from IS 'Index for outbound relationship queries';
COMMENT ON INDEX maproom.idx_edges_to IS 'Index for inbound relationship queries';
COMMENT ON INDEX maproom.idx_edges_type IS 'Index for filtering by relationship type';

-- =============================================================================
-- TABLE: stats
-- =============================================================================
-- Monitoring and observability metrics
-- Stores time-series data for performance tracking
CREATE TABLE maproom.stats (
    id SERIAL PRIMARY KEY,
    metric_name TEXT NOT NULL,
    metric_value NUMERIC NOT NULL,
    recorded_at TIMESTAMPTZ DEFAULT NOW()
);

COMMENT ON TABLE maproom.stats IS 'Time-series metrics for monitoring and observability';
COMMENT ON COLUMN maproom.stats.metric_name IS 'Metric identifier (e.g., chunks_indexed, search_latency_ms)';
COMMENT ON COLUMN maproom.stats.metric_value IS 'Numeric metric value';
COMMENT ON COLUMN maproom.stats.recorded_at IS 'Timestamp of metric recording';

-- Time-series query index (most recent first)
CREATE INDEX idx_stats_name_time ON maproom.stats(metric_name, recorded_at DESC);

COMMENT ON INDEX maproom.idx_stats_name_time IS 'Index for time-series metric queries';

-- =============================================================================
-- SCHEMA VALIDATION
-- =============================================================================
-- This schema creates successfully in PostgreSQL 16 with pgvector extension
-- Vector dimension: 768 (nomic-embed-text)
-- Cascade deletes enabled for data consistency
-- All tables properly indexed for hybrid search performance
-- =============================================================================
