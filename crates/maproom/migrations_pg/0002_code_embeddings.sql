-- Content-addressed embedding pool (spec §5.4, Open Decision 4 DEFAULT = normalized).
-- One row per unique blob_sha: identical content embeds once and is shared across
-- worktrees/users, deduping both embedding compute ($) and vector storage.
--
-- The `embedding` column is a typeless `vector` (Open Decision 4 default (a)):
-- maproom supports mixed dims (768/1024/1536), validated in the app layer, so the
-- column is left unconstrained here. NOTE: pgvector cannot build an HNSW/IVFFlat
-- ANN index on a typeless `vector` column — the ANN index is a Phase-2 concern
-- (vector search) and will be added once the dim-typing decision is settled
-- (typeless + per-query cast, or per-dimension tables per §5.4 alt (b)).
CREATE TABLE code_embeddings (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    blob_sha      TEXT NOT NULL UNIQUE,
    embedding     vector,
    embedding_dim INTEGER NOT NULL,
    model_version TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
