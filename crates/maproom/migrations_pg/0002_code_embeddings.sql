-- Content-addressed embedding pool (spec §5.4, Open Decision 4 DEFAULT = normalized).
-- One row per unique blob_sha: identical content embeds once and is shared across
-- worktrees/users, deduping both embedding compute ($) and vector storage.
--
-- The `embedding` column is a typeless `vector` (Open Decision 4 default (a)):
-- maproom supports mixed dims (768/1024/1536), validated in the app layer, so the
-- column is left unconstrained here. NOTE: pgvector cannot build an HNSW/IVFFlat
-- ANN index on a typeless `vector` column — the ANN index was a Phase-2 concern
-- (vector search) to be added once the dim-typing decision was settled.
-- RESOLVED in migration 0004: the dim-typing decision landed as one typed
-- `embedding_<dim>` column per supported dim (NOT per-dimension tables), and 0004
-- backfills this typeless column into them, builds a partial cosine HNSW index per
-- dim, then DROPS this column. This table therefore no longer has an `embedding`
-- column after 0004 — see migrations_pg/0004_vector_ann.sql.
CREATE TABLE code_embeddings (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blob_sha      TEXT NOT NULL UNIQUE,
    embedding     vector,
    embedding_dim INTEGER NOT NULL,
    model_version TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
