# Project Summary

**Maproom** is a code-aware indexing + retrieval layer for multi-agent work. It ingests repos into **PostgreSQL + pgvector**, exposes an **MCP server** for agents, and ships as a CrewChief-style subcommand:

```
crewchief maproom scan|upsert|watch|search|context|open
```

Core promises:

- **AST-first chunks** (symbols, not blobs)
- **Hybrid retrieval** (FTS/BM25 + vectors + light graph)
- **Budget-aware context bundles** for LLMs
- **Incremental, per-worktree indexing** for agent sandboxes
- **Postgres-first**: easy local dev, durable in CI

Non-goals for v1: global refactors, heavy CFG or whole-program analysis, multi-tenant auth.

---

# Architecture

## Components

- **`crewchief-maproom` (Rust, binary)**  
    Fast indexer + CLI. Walks worktrees, parses with tree-sitter, emits chunks/edges/summaries/embeddings, upserts into Postgres.
- **`maproom-mcp` (TypeScript/Node, service)**  
    MCP server exposing tools: `search`, `context`, `open`, `upsert`, `explain`. Orchestrates hybrid retrieval and assembles context bundles.
- **PostgreSQL 16+ with pgvector**  
    Stores repos, worktrees, files, chunks, edges; provides FTS, trigram, vector search.

## Language choices (why)

- **Rust** for the indexer: parallel, memory-safe, great tree-sitter support.
- **TypeScript** for MCP glue: fast iteration, JSON-RPC ergonomics, integrates smoothly with your agent stack.
- **Postgres** for everything persistent: one box for vectors + metadata + text search = no ops zoo.

---

# Repo & Package Layout

```
crewchief/
├─ crates/
│  ├─ maproom/
│     ├─ Cargo.toml
│     ├─ migrations/               # SQL migrations (schema maproom)
│     ├─ scripts/
│        ├─ dev_db.sh              # local PG init with extensions
│        └─ analyze.sql            # ANALYZE + ivfflat tuning
│     ├─ examples/                 # sample repos & fixtures for tests
│     ├─ README.md
│     └─ src/
│        ├─ main.rs                # Entry for crewchief-maproom binary
│        ├─ indexer/               # Indexer logic
│        └─ tsquery/               # Tree-sitter queries and helpers
├─ packages/
│  └─ maproom-mcp/
│     ├─ package.json
│     └─ src/                      # MCP server implementation
├─ .agents/
│  └─ knowledge/
│     └─ maproom/
│        └─ specification.md       # This file
```

---

# Database Schema (schema: `maproom`)

Extensions (run once):

```sql
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS unaccent;
```

Core tables:

```sql
CREATE SCHEMA IF NOT EXISTS maproom;

CREATE TABLE maproom.repos (
  id BIGSERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  root_path TEXT NOT NULL
);

CREATE TABLE maproom.worktrees (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  abs_path TEXT NOT NULL,
  UNIQUE (repo_id, name)
);

CREATE TABLE maproom.commits (
  id BIGSERIAL PRIMARY KEY,
  repo_id BIGINT NOT NULL REFERENCES maproom.repos(id) ON DELETE CASCADE,
  sha TEXT NOT NULL,
  committed_at TIMESTAMPTZ,
  UNIQUE (repo_id, sha)
);

CREATE TABLE maproom.files (
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

CREATE TYPE maproom.symbol_kind AS ENUM ('func','class','component','hook','module','var','type','other');

CREATE TABLE maproom.chunks (
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

CREATE TYPE maproom.edge_type AS ENUM ('imports','exports','calls','called_by','test_of','route_of');

CREATE TABLE maproom.chunk_edges (
  src_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  dst_chunk_id BIGINT NOT NULL REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  type maproom.edge_type NOT NULL,
  PRIMARY KEY (src_chunk_id, dst_chunk_id, type)
);

-- Optional signals
CREATE TABLE maproom.file_owners (
  file_id BIGINT REFERENCES maproom.files(id) ON DELETE CASCADE,
  owner TEXT NOT NULL,
  PRIMARY KEY(file_id, owner)
);

CREATE TABLE maproom.test_links (
  test_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  target_chunk_id BIGINT REFERENCES maproom.chunks(id) ON DELETE CASCADE,
  PRIMARY KEY(test_chunk_id, target_chunk_id)
);

-- Indexes
CREATE INDEX idx_chunks_tsv            ON maproom.chunks USING GIN (ts_doc);
CREATE INDEX idx_files_relpath_trgm    ON maproom.files  USING GIN (relpath gin_trgm_ops);
CREATE INDEX idx_chunks_code_vec       ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops) WITH (lists = 200);
CREATE INDEX idx_chunks_text_vec       ON maproom.chunks USING ivfflat (text_embedding vector_cosine_ops) WITH (lists = 200);
```

Tuning:

```sql
ANALYZE maproom.chunks; ANALYZE maproom.files;
SET ivfflat.probes = 10;  -- raise for larger repos if recall dips
```

---

# Indexing Pipeline (Rust / `crewchief-maproom`)

## Supported languages (v1)

- TypeScript/TSX, JavaScript/JSX.  
    Future: Python, Rust (drop-in new tree-sitter grammars).

## Steps

1. **Discover**  
    Walk worktree, respect `.gitignore`. Glob filters allow `--paths` or language subsets.

2. **Fingerprint**  
    `content_hash` (e.g., blake3) to detect unchanged files per commit/worktree.

3. **Parse (tree-sitter)**  
    Extract symbols (name, kind, span, signature), docstrings/JSDoc, imports/exports, shallow call edges.  
    Emit **symbol chunks**; also emit **region chunks** for very large files.

4. **Summarize**  
    Generate a terse 3–5 sentence English summary per chunk (cache by `(model_id, content_hash)`).

5. **Embed**

    - `code_embedding`: signature + docstring + (truncated) body.

    - `text_embedding`: the English summary.  
        Embedding dim fixed at 1536 for v1.

6. **ts_doc build**  
    Tokenize symbol name, split identifiers (camel/snake/kebab), include signature, docstring, preview → `to_tsvector('simple', unaccent(...))`.

7. **Persist**  
    Upsert files, chunks, edges; compute `recency_score` (exp decay on commit age) + `churn_score` (from `git log` count).

8. **Incremental**  
    `upsert` only changed files; update edges for affected neighbors.

## CLI (indexer)

```
crewchief maproom scan \
  --repo crewchief \
  --worktree crewchief-radar \
  --path .crewchief/worktrees/crewchief-radar \
  --commit $(git rev-parse HEAD)

crewchief maproom upsert \
  --paths src/auth/useAuth.ts src/app/router.tsx \
  --commit $(git rev-parse HEAD)

crewchief maproom watch --worktree radar --throttle 2s
crewchief maproom db migrate
```

Flags:

- `--languages ts,tsx,js,jsx`

- `--exclude "dist/**,**/*.snap"`

- `--embedding-model text-embedding-3-large`

- `--db postgres://maproom_writer:***@localhost:5432/maproom`

Return codes/JSON summaries for scripting under CrewChief.

---

# MCP Server (`maproom-mcp`)

Runs as a separate process (Node 20+), JSON-RPC over stdio.

## Tools

### `search`

**Args**

```json
{
  "query": "where is useAuth used?",
  "k": 10,
  "scope": { "repo": "crewchief", "worktree": "radar", "paths": ["src/"], "languages": ["ts","tsx"] },
  "mode": "auto"   // auto|code|text
}
```

**Behavior**

- Build `tsquery` from text.

- Get a query embedding (code or text based on heuristic).

- Run hybrid SQL (FTS + vectors + recency/churn bonuses).

- Return compact hits.

**Returns**

```json
{
  "hits": [
    {
      "score": 0.87,
      "chunk_id": 12345,
      "relpath": "src/auth/useAuth.ts",
      "kind": "hook",
      "symbol_name": "useAuth",
      "preview": "export function useAuth() { ... }",
      "start_line": 6,
      "end_line": 160
    }
  ]
}
```

Hybrid scoring (template):

```sql
WITH lex AS (
  SELECT c.id, ts_rank_cd(c.ts_doc, to_tsquery('simple', $4)) AS lex_rank
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id
  WHERE f.repo_id = $1 AND (f.worktree_id = $2 OR $2 IS NULL)
),
sem AS (
  SELECT c.id,
         1.0 - (c.code_embedding <=> $3) AS sem_code,
         1.0 - (c.text_embedding <=> $3) AS sem_text
  FROM maproom.chunks c
  JOIN maproom.files f ON f.id = c.file_id
  WHERE f.repo_id = $1 AND (f.worktree_id = $2 OR $2 IS NULL)
)
SELECT c.id, f.relpath, c.symbol_name, c.kind, c.preview, c.start_line, c.end_line,
       (0.55*COALESCE(l.lex_rank,0) +
        0.30*CASE WHEN $6='code' THEN COALESCE(s.sem_code,0) ELSE COALESCE(s.sem_text,0) END +
        0.10*CASE WHEN $6='code' THEN COALESCE(s.sem_text,0) ELSE COALESCE(s.sem_code,0) END +
        0.03*c.recency_score + 0.02*(1.0/(1.0+c.churn_score))
       ) AS score
FROM maproom.chunks c
JOIN maproom.files f ON f.id = c.file_id
LEFT JOIN lex l ON l.id = c.id
LEFT JOIN sem s ON s.id = c.id
ORDER BY score DESC
LIMIT $5;
```

Params: `$1 repo_id, $2 worktree_id|null, $3 query_embedding, $4 ts_query, $5 k, $6 mode`.

---

### `context`

**Args**

```json
{
  "chunk_id": 12345,
  "budget_tokens": 6000,
  "expand": { "callers": true, "callees": true, "tests": true, "docs": true, "config": true }
}
```

**Assembly policy (default)**

1. Primary chunk (include signature/docstring; full body if ≤ ~300 LOC).

2. 1 nearest test (`test_links` or filename heuristic).

3. Up to 1 caller + 1 callee (prefer same dir/package).

4. If React, include nearest route + co-located style/hook.

5. Add config snippets if query smells like config (jest/tsconfig/eslint/vite/router).

6. Stop at token budget; order: overview → primary → neighbors → tests → config.

**Returns**

```json
{
  "bundle": [
    {"relpath":"src/auth/useAuth.ts","range":{"start":6,"end":160},"role":"primary","reason":"symbol"},
    {"relpath":"src/auth/__tests__/useAuth.test.ts","range":{"start":1,"end":90},"role":"test","reason":"linked test"},
    {"relpath":"src/app/router.tsx","range":{"start":20,"end":120},"role":"neighbor","reason":"callee route"}
  ],
  "token_estimate": 2980
}
```

### `open`

Args: `{ "relpath": "src/foo.ts", "range": {"start": 20, "end": 120}, "worktree": "radar", "commit": "abc123" }`  
Behavior: if `commit` is checked out, read from FS; else `git show <sha>:<relpath>`.

### `upsert`

Args: `{ "paths": ["src/x.ts","src/y.ts"], "commit": "abc123" }`  
Behavior: spawns `crewchief maproom upsert ...`; returns `{ "updated_files": N, "updated_chunks": M }`.

### `explain` (optional)

Returns a cached symbol card (generated at index time, invalidated on change).

---

# CrewChief Integration

- **Plugin model**: ship the binary as `crewchief-maproom`. CrewChief dispatches `crewchief maproom ...` Git-style.
- **Per-agent worktrees**: include `--worktree` on every call; Maproom namespaces data by `{repo_id, worktree_id, commit}`.
- **Radar/Deck view** (optional later): surface Maproom stats (files indexed, chunks, hot modules, last update) in your ops deck pane.

## CLI Subcommand Integration

To integrate the maproom subcommand:

- Create `packages/cli/src/cli/maproom.ts` for the maproom command.
- Register a `Command('maproom')` that parses subcommands (scan, upsert, etc.) and spawns `crewchief-maproom <subcmd> <args>`.
- In `packages/cli/src/cli/index.ts`, import and call `registerMaproomCommand(program)`.
- Add build script `scripts/build-maproom.sh` for building the Rust binary.

---

# Configuration

**Env**

```
DATABASE_URL=postgres://maproom_writer:***@localhost:5432/maproom
EMBEDDINGS_MODEL=text-embedding-3-large
EMBEDDINGS_DIM=1536
INDEX_LANGUAGES=ts,tsx,js,jsx
IVFFLAT_LISTS=200
IVFFLAT_PROBES=10
```

**Per-repo policy (`maproom.yml`)**

```yaml
context:
  token_budget: 7000
  max_neighbors: 2
  prefer_same_dir: true
  react_routes_globs: ["src/app/router.tsx", "src/routes/**"]
index:
  include: ["src/**"]
  exclude: ["**/*.snap", "dist/**"]
```

---

# Performance Targets

- Index throughput (TS/TSX): ≥ 150 files/min on M-series laptop (cold).

- Search p95: < 50ms for k=10 (warm).

- Context assembly p95: < 120ms for default bundle.

DB sizing notes:

- Keep ≤ ~500k chunks per single-node instance before partitioning by repo.

- Increase `lists` and `probes` as the index grows; re-ANALYZE after bulk loads.

---

# Security & Guardrails

- Server filesystem access restricted to registered `worktrees.abs_path`.

- `git show` only for known `(repo_id, relpath)` in DB.

- Separate DB roles: `maproom_writer` (indexer) vs `maproom_reader` (MCP).

- No external net calls from MCP except embeddings if configured (can be local).

---

# Observability

- **Logs**: JSON with request_id, tool, latency, rows, bundle_token_estimate.

- **Metrics**: `mcp_search_latency_ms`, `mcp_context_latency_ms`, `chunks_total`, `edges_total`.

- **Telemetry**: record which slices agents actually request via `open` → tune bundle policy based on real usage.

---

# Testing

- **Unit (Rust)**:

  - Chunker on nasty TSX (HOCs, re-exports, deep generics).

  - Identifier splitting, docstring extraction.

- **SQL/Integration**:

  - Explain/Analyze hybrid query against fixture repo; enforce plan uses GIN + ivfflat.

- **Golden tests**:  
    Query → expected ordered bundle (file ranges + reasons).

- **E2E**:  
    Start local PG, run migrations, index sample repo, hit MCP `search/open`, assert stable results.

---

# Roadmap

- **V1**: TS/TSX/JS/JSX, hybrid search, bundles, `scan|upsert|watch`, MCP `search|context|open|upsert`.

- **V2**: Python + Rust grammars; better test linking; explicit React Router detection; owner/churn signals surfaced.

- **V3**: Cross-encoder reranker; learned bundle policies; symbol cards (`explain`) precomputed; minimal web UI to inspect bundles.

---

# “Fast start” Checklist

1. Postgres local:

```
createdb maproom
psql maproom -f crates/maproom/migrations/000_init.sql
```

2. First index:

```
crewchief maproom scan --repo crewchief --worktree radar --path /repos/crewchief-radar --commit $(git rev-parse HEAD)
psql maproom -f crates/maproom/scripts/analyze.sql
```

3. MCP online:

```
pnpm --filter maproom-mcp install && pnpm --filter maproom-mcp start
```

4. From your agent:

```
mcp maproom.search "find useAuth" --k 8 --scope.worktree radar
mcp maproom.context --chunk 123 --budget 6000 --expand callers,tests
```
