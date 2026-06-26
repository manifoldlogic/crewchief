-- Supporting indexes for the hot query paths (Phase-1 review follow-up).
-- Postgres does NOT auto-index foreign keys, and the PK/UNIQUE constraints in
-- 0001 only cover their leading columns, leaving several frequently-filtered
-- columns to sequential-scan. (The pgvector ANN index is a separate Phase-2
-- concern — see 0002 — and is intentionally not added here.)

-- Backward graph traversal: find_callers / incoming edges / graph-importance
-- GROUP BY dst_chunk_id all filter on dst_chunk_id, which the
-- (src_chunk_id, dst_chunk_id, type) PK cannot serve (it is src-leading).
CREATE INDEX idx_chunk_edges_dst ON chunk_edges (dst_chunk_id, type);

-- code_embeddings <-> chunks join, get_chunks_by_blob_sha, DISTINCT-blob_sha
-- counts, and fetch_chunks_needing_embeddings all filter/join on chunks.blob_sha.
CREATE INDEX idx_chunks_blob_sha ON chunks (blob_sha);

-- Per-worktree file lookups (get_worktree_file_count, language breakdown) use
-- worktree_id; get_file_id_by_relpath uses (worktree_id, relpath). One
-- worktree_id-leading composite serves both.
CREATE INDEX idx_files_worktree_relpath ON files (worktree_id, relpath);

-- find_file_relpath_by_content_hash (incremental rename detection) filters on
-- content_hash.
CREATE INDEX idx_files_content_hash ON files (content_hash);
