# Maproom Architecture

**Version**: 1.0
**Last Updated**: 2025-01-31
**Status**: Living Document

## Executive Summary

Maproom is a **hybrid semantic code search engine** that combines full-text search (FTS), vector similarity search, graph-based importance signals, and temporal signals to provide highly relevant code search results. The system indexes codebases using tree-sitter parsing, generates semantic embeddings via multiple providers (Ollama, Google Vertex AI, OpenAI), and stores everything in PostgreSQL with pgvector for efficient vector similarity queries.

**Core Value Proposition**: Find code by _what it does_ (semantic search) rather than _what it's called_ (keyword search).

**Performance Target**: < 50ms end-to-end search latency for k=10 results.

---

## System Overview

Maproom consists of six major subsystems:

1. **Indexing Pipeline** - Code parsing and chunk extraction
2. **Embedding Layer** - Multi-provider semantic embedding generation
3. **Storage Layer** - PostgreSQL database with pgvector
4. **Search Pipeline** - Hybrid retrieval with parallel execution
5. **Context Assembly** - Intelligent code context bundling
6. **MCP Integration** - Model Context Protocol server wrapper

```
┌─────────────────────────────────────────────────────────────────────┐
│                          MCP Server (TypeScript)                    │
│                        JSON-RPC stdio transport                      │
└────────────────────────┬────────────────────────────────────────────┘
                         │
                         │ spawns & manages
                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Rust Core (crewchief-maproom)                    │
│                                                                       │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐            │
│  │   Indexing   │   │    Search    │   │   Context    │            │
│  │   Pipeline   │   │   Pipeline   │   │   Assembly   │            │
│  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘            │
│         │                  │                  │                     │
│         │                  │                  │                     │
│         └──────────────────┼──────────────────┘                     │
│                            │                                        │
│                            ▼                                        │
│                   ┌─────────────────┐                               │
│                   │  Embedding Layer │                               │
│                   │  (Multi-Provider) │                               │
│                   └─────────────────┘                               │
│                            │                                        │
└────────────────────────────┼────────────────────────────────────────┘
                             │
                             ▼
                   ┌─────────────────┐
                   │   PostgreSQL    │
                   │   + pgvector    │
                   │   (Storage)     │
                   └─────────────────┘
```

---

## 1. Indexing Pipeline

**Purpose**: Transform source code into searchable chunks with structural metadata.

### Architecture

```
File Discovery → Tree-sitter Parsing → Chunk Extraction → Embedding → Database Storage
```

### Components

#### 1.1 File Discovery & Filtering
- **Location**: `crates/maproom/src/indexer/mod.rs`
- Walks repository filesystem respecting .gitignore
- Filters by supported file extensions (.ts, .js, .rs, .md, .json, .yaml, .toml)
- Configurable exclusion patterns (e.g., `node_modules/**`)

#### 1.2 Language Detection & Parsing
- **Location**: `crates/maproom/src/indexer/parser.rs`
- **Technology**: Tree-sitter grammars for precise AST-based parsing
- **Supported Languages**:
  - TypeScript/JavaScript (tree-sitter-typescript)
  - Rust (tree-sitter-rust)
  - Markdown (tree-sitter-markdown)
  - JSON (tree-sitter-json)
  - Python (tree-sitter-python)

#### 1.3 Chunk Extraction
- **Granularity**: Functions, classes, methods, markdown sections, JSON keys
- **Metadata Captured**:
  - Symbol name (e.g., `authenticate`, `UserService`)
  - Kind (e.g., `function`, `class`, `heading_2`, `json_key`)
  - Line ranges (start_line, end_line)
  - Language and file path
  - Preview text (first ~200 chars)

#### 1.4 Full-Text Search Indexing
- **Technology**: PostgreSQL `tsvector` with `ts_rank_cd` scoring
- **Tokenization**: Whitespace and punctuation splitting
- **Query Format**: Prefix matching with `term:*` expansion
- **Boosting**: Markdown headings get 1.2x-2.0x boost based on level

#### 1.5 Relationship Extraction
- **Graph Edges**: Stored in `chunk_edges` table
- **Edge Types**:
  - `import`: Module imports (TypeScript, Python, Rust)
  - `calls`: Function calls
  - `defines`: Export definitions
  - `tests`: Test → implementation links
  - `routes`: React Router route definitions

#### 1.6 Git Metadata Extraction
- **Recency Score**: Based on `git log` last modification time
- **Churn Score**: Based on frequency of changes over time
- **Formula**: Exponential decay from last commit timestamp

### Performance Characteristics
- **Throughput**: ~150-200 files/min (depending on file size and embedding provider)
- **Concurrency**: Configurable (default: 4 workers, max: 16)
- **Parallel Mode**: Batch processing for large codebases
- **Incremental**: Only re-index changed files (via `upsert`)

---

## 2. Embedding Layer

**Purpose**: Generate semantic vector embeddings for code chunks.

### Multi-Provider Architecture

Maproom supports **three embedding providers** with automatic detection and failover:

| Provider | Dimension | Table | Use Case |
|----------|-----------|--------|----------|
| **Ollama** (mxbai-embed-large) | 1024 | `vec_code_1024` | Zero-config local deployment (default) |
| **Ollama** (nomic-embed-text) | 768 | `vec_code_768` | Legacy support |
| **Google Vertex AI** | 768 | `vec_code_768` | Production, enterprise |
| **OpenAI** | 1536 | `vec_code` | High-quality embeddings |

**Key Design Decision**: Maproom uses **dimension-specific vector tables** (`vec_code_768`, `vec_code_1024`, `vec_code`) to support multiple embedding dimensions seamlessly.

### Provider Selection Logic

**Location**: `packages/maproom-mcp/src/utils/provider-detection.ts`, `crates/maproom/src/embedding/providers.rs`

```
1. Check MAPROOM_EMBEDDING_PROVIDER environment variable
2. If not set, auto-detect:
   - Google: Check GOOGLE_APPLICATION_CREDENTIALS
   - OpenAI: Check OPENAI_API_KEY
   - Ollama: Check http://ollama:11434 connectivity
3. Fall back to Ollama (default zero-config)
4. Cache selection for session performance
```

### Embedding Generation

#### Code Embeddings
- **Purpose**: Semantic understanding of code functionality
- **Input**: Source code text (function bodies, class definitions)
- **Normalization**: Remove comments, normalize whitespace
- **Model**: `mxbai-embed-large` (Ollama default), `nomic-embed-text` (Ollama legacy), `textembedding-gecko@003` (Google), `text-embedding-3-small` (OpenAI)

#### Text Embeddings
- **Purpose**: Natural language search in documentation
- **Input**: Markdown content, code comments, docstrings
- **Model**: Same models as code embeddings

### Caching Strategy

- **Location**: `crates/maproom/src/embedding/mod.rs`
- **Algorithm**: LRU (Least Recently Used) with TTL
- **Size**: Configurable (default: 1000 entries)
- **TTL**: 1 hour (configurable)
- **Cache Key**: Hash of (text content + provider + model)
- **Hit Rate Target**: > 80% for repeated queries

### Performance Characteristics

- **Latency**:
  - Ollama (local): ~50-100ms per chunk
  - Google Vertex AI: ~100-200ms per chunk (network)
  - OpenAI: ~150-250ms per chunk (network)
- **Batch Processing**: Up to 100 chunks per API call (provider-dependent)
- **Cost Tracking**: Per-provider usage metrics
- **Retry Logic**: Exponential backoff for transient failures

---

## 3. Storage Layer

**Purpose**: Persistent storage for code chunks, embeddings, and relationships.

### Database Architecture

Maproom uses **PostgreSQL 16** with the **pgvector** extension for vector similarity search.

#### Dual PostgreSQL Setup

The system supports two database instances for different use cases:

1. **Devcontainer Database** (`postgres:5432`)
   - Purpose: Local development, CLI testing, integration tests
   - Connection: `postgresql://postgres:postgres@postgres:5432/crewchief`
   - Data: Ephemeral, can be reset

2. **MCP Production Database** (`maproom-postgres:5432`)
   - Purpose: Production-like MCP service, stable semantic search
   - Connection: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
   - Data: Persistent, production data

**Rationale**: Isolation between development experiments and stable MCP service.

### Schema Overview

**Namespace**: All tables in `maproom` schema.

#### Core Tables

**`repos`** - Repository metadata
```sql
CREATE TABLE maproom.repos (
  id SERIAL PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  remote_url TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

**`worktrees`** - Git worktree tracking
```sql
CREATE TABLE maproom.worktrees (
  id SERIAL PRIMARY KEY,
  repo_id INTEGER REFERENCES maproom.repos(id),
  name TEXT NOT NULL,
  abs_path TEXT NOT NULL,
  branch TEXT,
  commit_hash TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  UNIQUE(repo_id, name)
);
```

**`files`** - Indexed files
```sql
CREATE TABLE maproom.files (
  id SERIAL PRIMARY KEY,
  worktree_id INTEGER REFERENCES maproom.worktrees(id),
  relpath TEXT NOT NULL,
  language TEXT,
  last_modified TIMESTAMPTZ,
  UNIQUE(worktree_id, relpath)
);
```

**`chunks`** - Searchable code chunks (PRIMARY TABLE)
```sql
CREATE TABLE maproom.chunks (
  id SERIAL PRIMARY KEY,
  file_id INTEGER REFERENCES maproom.files(id),
  symbol_name TEXT,
  kind TEXT NOT NULL, -- 'function', 'class', 'heading_2', etc.
  start_line INTEGER NOT NULL,
  end_line INTEGER NOT NULL,
  preview TEXT,

  -- Full-text search
  ts_doc tsvector,

  -- Vector embeddings (multi-provider support)
  code_embedding vector(1536),           -- OpenAI 1536-dim
  text_embedding vector(1536),           -- OpenAI 1536-dim
  code_embedding_ollama vector(768),     -- Ollama/Google shared 768-dim
  text_embedding_ollama vector(768),     -- Ollama/Google shared 768-dim

  -- Temporal signals
  recency_score FLOAT DEFAULT 0.0,
  churn_score FLOAT DEFAULT 0.0,

  -- Metadata (JSONB for flexibility)
  metadata JSONB DEFAULT '{}',

  -- Timestamps
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW()
);
```

**`chunk_edges`** - Code relationships (graph)
```sql
CREATE TABLE maproom.chunk_edges (
  id SERIAL PRIMARY KEY,
  from_chunk_id INTEGER REFERENCES maproom.chunks(id),
  to_chunk_id INTEGER REFERENCES maproom.chunks(id),
  edge_type TEXT NOT NULL, -- 'import', 'calls', 'tests', 'defines'
  metadata JSONB DEFAULT '{}'
);
```

#### Indexes

**Full-Text Search Indexes**:
```sql
CREATE INDEX idx_chunks_ts_doc ON maproom.chunks USING GIN(ts_doc);
```

**Vector Similarity Indexes** (ivfflat algorithm):
```sql
-- OpenAI 1536-dim indexes
CREATE INDEX idx_chunks_code_embedding_ivfflat
ON maproom.chunks USING ivfflat (code_embedding vector_cosine_ops)
WITH (lists = 100);

-- Ollama/Google 768-dim indexes
CREATE INDEX idx_chunks_code_embedding_ollama_ivfflat
ON maproom.chunks USING ivfflat (code_embedding_ollama vector_cosine_ops)
WITH (lists = 100);
```

**Performance Indexes**:
```sql
CREATE INDEX idx_files_worktree_id ON maproom.files(worktree_id);
CREATE INDEX idx_chunks_file_id ON maproom.chunks(file_id);
CREATE INDEX idx_chunk_edges_from ON maproom.chunk_edges(from_chunk_id);
CREATE INDEX idx_chunk_edges_to ON maproom.chunk_edges(to_chunk_id);
```

### Vector Search COALESCE Pattern

**Problem**: How to query multiple embedding columns with different dimensions?

**Solution**: Use PostgreSQL `COALESCE` to prefer 768-dim embeddings when available:

```sql
SELECT
  c.id,
  1 - (COALESCE(c.code_embedding_ollama, c.code_embedding) <=> $query_embedding) AS similarity
FROM maproom.chunks c
WHERE COALESCE(c.code_embedding_ollama, c.code_embedding) IS NOT NULL
ORDER BY similarity DESC
LIMIT 10;
```

**Preference Order**: `ollama` (768-dim) → `openai` (1536-dim)

**Rationale**: Faster vector operations on smaller dimensions, shared columns for Ollama/Google.

---

## 4. Search Pipeline

**Purpose**: Execute hybrid retrieval combining multiple signals and fuse results.

### Pipeline Architecture

```
Query String
    ↓
┌───────────────────┐
│ Query Processing  │  ~5ms
│ - Tokenization    │
│ - Embedding Gen   │
│ - Mode Detection  │
└─────────┬─────────┘
          │
          ▼
┌────────────────────────────────────────┐
│    Parallel Search Execution           │  ~30-40ms
│  ┌──────┐  ┌────────┐  ┌───────┐     │
│  │ FTS  │  │ Vector │  │ Graph │     │
│  │Search│  │ Search │  │Signals│     │
│  └──┬───┘  └───┬────┘  └───┬───┘     │
│     │          │           │          │
│     │   ┌──────────────┐   │          │
│     └───┤ Temporal     │───┘          │
│         │ Signals      │              │
│         └──────────────┘              │
└────────────────┬───────────────────────┘
                 │
                 ▼
        ┌────────────────┐
        │ Score Fusion   │  ~2-5ms
        │ (Weighted Avg) │
        └────────┬───────┘
                 │
                 ▼
        ┌────────────────┐
        │ Result Assembly│  ~5-10ms
        │ (Fetch Details)│
        └────────┬───────┘
                 │
                 ▼
         Final Ranked Results
```

### 4.1 Query Processing

**Location**: `crates/maproom/src/search/query_processor.rs`

**Steps**:
1. **Tokenization**: Split on whitespace, remove stop words
2. **FTS Query Generation**: Convert to PostgreSQL tsquery format (`term1:* & term2:*`)
3. **Embedding Generation**: Async call to embedding provider
4. **Query Expansion**: Add synonyms (e.g., "auth" → "authentication", "authorize")
5. **Mode Detection**: Heuristic classification (Code vs. Text vs. Auto)

**Mode Detection Heuristics**:
- **Code Mode**: Contains `::`, `->`, camelCase, snake_case patterns
- **Text Mode**: > 5 words, natural language structure
- **Auto Mode**: Ambiguous, use hybrid search

### 4.2 Parallel Search Execution

**Location**: `crates/maproom/src/search/executors.rs`

All searches execute **in parallel** using Tokio's `join!` macro for maximum throughput.

#### FTS Executor
- **Location**: `crates/maproom/src/search/fts.rs`
- **Algorithm**: PostgreSQL `ts_rank_cd` with prefix matching + **semantic ranking multipliers**
- **Boosting**: Markdown headings, JSON keys get relevance boost
- **Output**: Ranked results with FTS scores (0.0-1.0)

##### Semantic Entry Point Ranking (SEMRANK)

**Purpose**: Prioritize code implementations over tests and documentation in FTS results.

**Problem**: Traditional FTS ranks by keyword frequency, causing documentation (which mentions keywords repeatedly) to rank higher than actual function/class implementations.

**Solution**: Apply **kind multipliers** and **exact match multipliers** to FTS scores:

```
final_score = base_fts_score × kind_multiplier × exact_match_multiplier
```

**Kind Multipliers** (TypeScript implementation in `packages/maproom-mcp/src/tools/search.ts`):
```typescript
const kindMultipliers = {
  // Implementations (boost)
  func: 2.5, async_func: 2.5,
  class: 2.0, struct: 2.0, enum: 2.0, interface: 2.0,
  method: 1.5,
  component: 1.8, hook: 1.8,

  // Tests (demote)
  test: 0.6, test_function: 0.6,

  // Documentation (demote)
  heading_1: 0.6, heading_2: 0.5, heading_3: 0.3,
  markdown_section: 0.4, code_block: 0.4,
  comment: 0.3, doc_comment: 0.3,

  // Default
  default: 1.0
}
```

**Exact Match Multiplier**:
- 3.0× when `LOWER(symbol_name) = LOWER(normalize_query(query))`
- 1.0× otherwise

**Query Normalization**:
- camelCase → snake_case: `validateToken` → `validate_token`
- Spaces → underscores: `HTTP handler` → `http_handler`
- Case-insensitive matching: `Authenticate` → `authenticate`

**SQL Implementation** (applied in search tool):
```sql
SELECT
  *,
  (
    ts_rank_cd(ts_doc, query)
    *
    CASE kind
      WHEN 'func' THEN 2.5
      WHEN 'async_func' THEN 2.5
      WHEN 'class' THEN 2.0
      WHEN 'test' THEN 0.6
      WHEN 'heading_1' THEN 0.6
      WHEN 'heading_2' THEN 0.5
      WHEN 'comment' THEN 0.3
      ELSE 1.0
    END
    *
    CASE
      WHEN LOWER(symbol_name) = LOWER(normalize_for_exact_match($1))
      THEN 3.0
      ELSE 1.0
    END
  ) AS final_score
FROM maproom.chunks
WHERE ts_doc @@ query
ORDER BY final_score DESC
```

**Performance Impact**:
- **17% faster** on average (p95: 48ms → 40ms)
- Better ranking allows earlier result termination
- Implementations rank first, reducing wasted processing on docs

**Integration with RRF Fusion**: Semantic ranking applies to FTS scores **before** RRF fusion, ensuring improved implementation ranking carries through to hybrid search results.

**Documentation**: See `packages/maproom-mcp/docs/search-ranking.md` for complete details.

#### Vector Executor
- **Location**: `crates/maproom/src/search/vector.rs`
- **Algorithm**: pgvector cosine similarity (`<=>` operator)
- **Query**: Uses COALESCE pattern for multi-dimension support
- **Output**: Ranked results with similarity scores (0.0-1.0)

#### Graph Executor
- **Location**: `crates/maproom/src/search/graph.rs`
- **Algorithm**: PageRank-style importance from `chunk_edges`
- **Signal**: Chunks with many incoming edges rank higher
- **Output**: Importance scores (0.0-1.0)

#### Temporal Signals Executor
- **Location**: `crates/maproom/src/search/signals.rs`
- **Signals**:
  - **Recency**: Exponential decay from last modification
  - **Churn**: Penalty for frequently changed code (instability)
- **Formula**: `combined_score = recency_weight * recency + churn_weight * (1 / (1 + churn))`
- **Output**: Combined temporal score (0.0-1.0)

### 4.3 Score Fusion

**Location**: `crates/maproom/src/search/fusion/basic.rs`

**Algorithm**: Weighted linear combination

**Formula**:
```
final_score = w_fts × fts_score
            + w_vector × vector_score
            + w_graph × graph_score
            + w_recency × recency_score
            + w_churn × (1 / (1 + churn_score))
```

**Default Weights** (from `FusionWeights::default()`):
```rust
fts:     0.40  // Highest weight for keyword/exact matches
vector:  0.35  // Strong semantic similarity signal
graph:   0.10  // Moderate boost for important/central code
recency: 0.10  // Moderate boost for recently changed code
churn:   0.05  // Slight penalty for high-churn (unstable) code
```

**Weight Rationale**:
- FTS prioritized for exact keyword matches (developers know what they're looking for)
- Vector close second for semantic "similar functionality" search
- Graph/recency/churn provide tiebreaking and context-aware ranking

**Normalization**: All input scores already normalized to 0.0-1.0 by executors.

**Advanced Fusion** (Phase 3): Reciprocal Rank Fusion (RRF) available in `fusion/rrf.rs` for more sophisticated merging.

### 4.4 Result Assembly

**Location**: `crates/maproom/src/search/pipeline.rs:assemble_results()`

**Steps**:
1. Extract chunk IDs from fused results
2. Fetch chunk details from database (single batch query):
   - File path, symbol name, kind
   - Line ranges, preview text
3. Merge scores with chunk metadata
4. Return `ChunkSearchResult[]` with complete information

**Output Format**:
```typescript
{
  chunk_id: number,
  relpath: string,
  symbol_name: string,
  kind: string,
  start_line: number,
  end_line: number,
  score: number,
  source_scores: { FTS: 0.9, Vector: 0.8, Graph: 0.3, Signals: 0.5 }
}
```

### Performance Characteristics

**Target**: < 50ms end-to-end for k=10 results

**Typical Breakdown**:
- Query Processing: 5ms
- Parallel Search: 30-40ms (bottleneck: vector similarity)
- Fusion: 2-5ms
- Assembly: 5-10ms
- **Total**: 42-60ms (meets target most of the time)

**Optimization Opportunities**:
- Materialized views for graph importance (precomputed)
- Query result caching (LRU cache in `crates/maproom/src/search/cache.rs`)
- Vector index tuning (ivfflat lists parameter)

---

## 5. Context Assembly

**Purpose**: Intelligently gather related code chunks within a token budget.

### Architecture

```
Target Chunk
    ↓
┌──────────────────────┐
│ Relationship Traversal│
│ - Imports            │
│ - Exports            │
│ - Callers            │
│ - Callees            │
│ - Tests              │
│ - Documentation      │
└─────────┬────────────┘
          │
          ▼
┌──────────────────────┐
│ Importance Scoring   │
│ - Heuristics         │
│ - Proximity          │
│ - Edge type weights  │
└─────────┬────────────┘
          │
          ▼
┌──────────────────────┐
│ Priority Queue       │
│ - Budget manager     │
│ - Token counting     │
│ - Greedy selection   │
└─────────┬────────────┘
          │
          ▼
┌──────────────────────┐
│ Content Formatting   │
│ - Markdown output    │
│ - Syntax highlighting│
│ - Truncation         │
└─────────┬────────────┘
          │
          ▼
    ContextBundle
```

**Location**: `crates/maproom/src/context/`

### Components

#### 5.1 Relationship Discovery
- **Location**: `context/relationships.rs`
- **Graph Queries**: Traverse `chunk_edges` table
- **Edge Types**: Import, Export, Calls, Tests, Routes
- **Max Depth**: Configurable (default: 2 hops, max: 5)
- **Parallel Loading**: Fetch relationships concurrently

#### 5.2 Importance Scoring
- **Location**: `context/importance.rs`
- **Factors**:
  - Edge type weight (tests > callers > imports)
  - Graph distance (closer = higher importance)
  - File type (implementation > tests > docs)
  - Language-specific heuristics

#### 5.3 Budget Management
- **Location**: `context/budget.rs`
- **Algorithm**: Token-based greedy selection
- **Budget Allocation**:
  - Target chunk: Always included (priority 1)
  - Direct relationships: High priority
  - Transitive relationships: Lower priority
- **Token Counting**: Accurate estimation via `tiktoken` library

#### 5.4 Content Formatting
- **Location**: `context/formatter.rs`
- **Output Format**: Markdown with code blocks
- **Features**:
  - Syntax highlighting hints (language tags)
  - File headers with paths and line ranges
  - Truncation indicators for large files
  - Relationship explanations

### Expand Options

Configurable via `ExpandOptions` struct:

```rust
pub struct ExpandOptions {
    pub callers: bool,        // Include functions that call this
    pub callees: bool,        // Include functions called by this
    pub tests: bool,          // Include test files
    pub docs: bool,           // Include documentation
    pub config: bool,         // Include config files
    pub max_depth: usize,     // Traversal depth (default: 2)
}
```

### Example Output

```markdown
# Context Bundle

## Target Chunk
File: `src/auth/authenticate.ts:42-67`
```typescript
export async function authenticate(token: string): Promise<User> {
  const decoded = jwt.verify(token, process.env.JWT_SECRET);
  return await db.users.findById(decoded.userId);
}
```

## Callers (2)
### `src/routes/api.ts:123-130` calls `authenticate`
### `src/middleware/auth.ts:45-52` calls `authenticate`

## Tests (1)
### `src/auth/authenticate.test.ts:15-42` tests `authenticate`

Total tokens: 2,847 / 6,000 budget
```

---

## 6. MCP Integration

**Purpose**: Expose Maproom functionality via Model Context Protocol for AI assistants.

### Architecture

```
AI Assistant (Claude/Cursor)
         │
         │ JSON-RPC (stdio)
         ▼
┌────────────────────────┐
│  MCP Server (Node.js)  │
│  packages/maproom-mcp/ │
│  - JSON-RPC handler    │
│  - Tool schemas        │
│  - Provider detection  │
└───────────┬────────────┘
            │
            │ spawn & IPC
            ▼
┌────────────────────────┐
│  Rust Binary           │
│  crewchief-maproom     │
│  - Indexing            │
│  - Search              │
│  - Context assembly    │
└────────────────────────┘
```

**Location**: `packages/maproom-mcp/src/index.ts`

### MCP Tools Exposed

| Tool | Description | Primary Handler |
|------|-------------|-----------------|
| **status** | Index statistics | TypeScript (direct DB query) |
| **search** | Hybrid code search | TypeScript → Rust via spawn |
| **open** | Retrieve file contents | TypeScript (direct DB query) |
| **scan** | Index repository | TypeScript → Rust (spawn) |
| **upsert** | Update specific files | TypeScript → Rust (spawn) |
| **context** | Assemble code context | TypeScript → Rust (spawn) |
| **explain** | Generate symbol card | TypeScript → Rust (spawn) |

### Communication Protocol

**Transport**: stdio (standard input/output)
**Framing**: Newline-delimited JSON (with optional Content-Length headers)
**Spec**: MCP Protocol v2024-11-05

**Request Example**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "repo": "crewchief",
      "query": "authentication flow",
      "k": 10,
      "mode": "hybrid"
    }
  }
}
```

**Response Example**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{
      "type": "text",
      "text": "{\"hits\": [...], \"hint\": \"...\"}"
    }]
  }
}
```

### Zero-Configuration Design

**Philosophy**: Maproom should work out-of-the-box with minimal setup.

**Auto-Detection**:
- Repository name from `git remote`
- Worktree name from `git branch`
- Commit hash from `git rev-parse HEAD`
- Embedding provider from environment variables
- Database connection from default Docker Compose setup

**Fallback Defaults**:
- Database: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Provider: Ollama at `http://ollama:11434`
- Path: Current working directory

### Docker Compose Integration

**Location**: `~/.maproom-mcp/docker-compose.yml` (installed on first run)

**Services**:
1. **postgres** (pgvector/pgvector:pg16)
   - Port: `127.0.0.1:5433:5432`
   - Volume: `maproom-data`

2. **ollama** (ollama/ollama:latest)
   - Port: `127.0.0.1:11434:11434`
   - Auto-pulls `mxbai-embed-large` model
   - Volume: `ollama-models`

3. **maproom-mcp** (manifoldlogic/crewchief_maproom-mcp:latest)
   - Runs Rust binary in stdio mode
   - Environment: Provider config, database URL
   - Health check: PostgreSQL connectivity

---

## Data Flow Examples

### Example 1: Indexing a New Repository

```
User runs: mcp__maproom__scan({ repo: "myapp", path: "/workspace" })

1. MCP Server receives JSON-RPC request
2. Provider detection runs (finds Google Vertex AI)
3. Spawn crewchief-maproom with args:
   scan --repo myapp --path /workspace --provider google
4. Rust binary:
   a. Discovers files (904 files found)
   b. Parses with tree-sitter (35,181 chunks extracted)
   c. Inserts chunks into PostgreSQL (batch upsert)
   d. Generates embeddings (Google Vertex AI, 768-dim)
   e. Updates embedding columns in database
5. MCP Server parses stdout, returns statistics
6. User sees: "904 files, 35,181 chunks indexed"
```

### Example 2: Hybrid Search Query

```
User runs: mcp__maproom__search({
  repo: "myapp",
  query: "user authentication",
  mode: "hybrid",
  k: 10
})

1. Query Processing (5ms):
   - Tokenize: ["user", "authentication"]
   - FTS query: "user:* & authentication:*"
   - Generate embedding: [0.23, -0.45, ..., 0.12] (768-dim)

2. Parallel Execution (35ms):
   - FTS: ts_rank_cd query → 147 matches
   - Vector: cosine similarity → 89 matches
   - Graph: PageRank → 23 high-importance chunks
   - Signals: Recency + churn → temporal scores

3. Score Fusion (3ms):
   - Combine 259 unique chunks
   - Weighted average: 0.4×FTS + 0.35×Vector + 0.1×Graph + 0.15×Signals
   - Sort descending by final score
   - Take top 10

4. Result Assembly (7ms):
   - Fetch chunk details from DB (1 query, 10 rows)
   - Merge scores with metadata
   - Format as ChunkSearchResult[]

5. Return to user (Total: 50ms):
   [
     { chunk_id: 4523, relpath: "src/auth/login.ts",
       symbol_name: "authenticateUser", score: 0.87 },
     ...
   ]
```

### Example 3: Context Assembly

```
User runs: mcp__maproom__context({
  chunk_id: "4523",
  budget_tokens: 6000
})

1. Fetch target chunk (chunk_id=4523):
   - Function: authenticateUser in src/auth/login.ts
   - Always included (highest priority)

2. Relationship Discovery (parallel):
   - Imports: jwt library, db module
   - Callers: 3 functions call authenticateUser
   - Callees: authenticateUser calls verifyToken, getUserById
   - Tests: 2 test files reference authenticateUser

3. Importance Scoring:
   - Direct callers: High importance (0.9)
   - Tests: High importance (0.85)
   - Callees: Medium importance (0.7)
   - Imports: Low importance (0.4)

4. Priority Queue Selection:
   - Start with target chunk (347 tokens)
   - Add highest priority: test file (1,234 tokens)
   - Add callers: api.ts caller (456 tokens)
   - Add callees: verifyToken (523 tokens)
   - Budget remaining: 6000 - 2560 = 3440 tokens
   - Continue until budget exhausted

5. Format as Markdown:
   - Target chunk with code block
   - Relationship sections (Callers, Callees, Tests)
   - Token usage summary

6. Return: "Context bundle with 8 related chunks (5,847 tokens)"
```

---

## Performance Characteristics

### Indexing Performance

| Metric | Value | Notes |
|--------|-------|-------|
| **Throughput** | 150-200 files/min | Depends on file size, provider latency |
| **Concurrency** | 4-16 workers | Configurable via `--concurrency` |
| **Embedding Latency** | 50-250ms/chunk | Ollama fastest, OpenAI slowest |
| **Database Insert** | ~1ms/chunk | PostgreSQL batch insert |

### Search Performance

| Metric | Target | Actual | Notes |
|--------|--------|--------|-------|
| **End-to-End Latency** | < 50ms | 42-60ms | For k=10 results |
| **Query Processing** | < 5ms | ~5ms | Tokenization + embedding |
| **Parallel Search** | < 40ms | 30-40ms | Bottleneck: vector similarity |
| **Fusion** | < 5ms | 2-5ms | Weighted average computation |
| **Assembly** | < 10ms | 5-10ms | Database fetch + merge |

### Scalability

| Scale | Files | Chunks | Search Latency | Notes |
|-------|-------|--------|----------------|-------|
| **Small** | 100-1K | 5K-50K | ~30ms | Typical library |
| **Medium** | 1K-10K | 50K-500K | ~50ms | Large application |
| **Large** | 10K-100K | 500K-5M | ~100ms | Monorepo |
| **Enterprise** | 100K+ | 5M+ | ~200ms+ | Needs tuning |

**Optimization at Scale**:
- Increase ivfflat lists parameter (100 → 500 → 1000)
- Partition by repository/worktree
- Enable query result caching
- Use materialized views for graph importance

---

## Configuration & Tuning

### Fusion Weight Tuning

**Default Weights** (balance keyword + semantic):
```rust
FusionWeights {
  fts: 0.40,
  vector: 0.35,
  graph: 0.10,
  recency: 0.10,
  churn: 0.05
}
```

**Documentation-Heavy Codebase** (boost FTS for terminology):
```rust
FusionWeights {
  fts: 0.50,
  vector: 0.30,
  graph: 0.10,
  recency: 0.05,
  churn: 0.05
}
```

**Actively Developed Codebase** (boost recency):
```rust
FusionWeights {
  fts: 0.35,
  vector: 0.35,
  graph: 0.10,
  recency: 0.15,
  churn: 0.05
}
```

**Library/Framework Codebase** (boost graph importance):
```rust
FusionWeights {
  fts: 0.35,
  vector: 0.30,
  graph: 0.20,
  recency: 0.10,
  churn: 0.05
}
```

### Vector Index Tuning

**ivfflat Lists Parameter**:
- Small datasets (< 100K chunks): `lists = 100`
- Medium datasets (100K-1M chunks): `lists = 500`
- Large datasets (> 1M chunks): `lists = 1000`

**Formula**: `lists = sqrt(row_count)` is a good starting point.

**Trade-off**: Higher lists = faster search, slower index build, more memory.

### Context Assembly Tuning

**Budget Allocation**:
- Small context (< 3K tokens): Focus on direct relationships only
- Medium context (3K-10K tokens): Include 1-2 hop relationships
- Large context (10K-20K tokens): Include tests, docs, config

**Max Depth**:
- `max_depth: 1` → Direct relationships only (callers, callees)
- `max_depth: 2` → Transitive relationships (caller's callers)
- `max_depth: 3+` → Deep traversal (rarely needed, expensive)

---

## Security & Deployment

### Security Considerations

1. **Database Credentials**:
   - Never commit credentials to version control
   - Use environment variables (`MAPROOM_DATABASE_URL`)
   - Rotate passwords regularly

2. **Embedding Provider Keys**:
   - Store in secure environment variables
   - Use service accounts (Google Vertex AI)
   - Enable API key restrictions (OpenAI)

3. **Network Exposure**:
   - Bind PostgreSQL to `127.0.0.1` (localhost only)
   - Use `maproom-network` Docker bridge for service isolation
   - Never expose MCP server to public internet

4. **Input Validation**:
   - Sanitize file paths to prevent directory traversal
   - Validate chunk IDs to prevent SQL injection
   - Limit query length to prevent DoS

### Deployment Patterns

#### Local Development (Zero-Config)
```bash
# Docker Compose handles everything
docker-compose -f ~/.maproom-mcp/docker-compose.yml up -d
```

#### Production Deployment
```yaml
# Kubernetes deployment with persistent volumes
# - PostgreSQL StatefulSet with pgvector
# - Ollama Deployment with GPU support
# - Maproom MCP Deployment with HPA
```

#### Cloud Deployment (Google Cloud)
```yaml
# Cloud SQL for PostgreSQL + pgvector
# Cloud Run for MCP server
# Vertex AI for embeddings (no Ollama needed)
```

---

## Monitoring & Observability

### Metrics

**Location**: `crates/maproom/src/metrics/mod.rs`

**Prometheus Metrics Exposed**:
- `maproom_queries_total{mode, success}` - Query count
- `maproom_query_latency_seconds{mode}` - Latency histogram
- `maproom_result_count{mode}` - Results per query
- `maproom_embedding_cache_hits` - Cache hit rate
- `maproom_fusion_time_seconds` - Fusion duration

### Logging

**Log Levels**:
- `ERROR`: Database connection failures, embedding errors
- `WARN`: Slow queries (> 100ms), cache misses
- `INFO`: Query execution, indexing progress
- `DEBUG`: Score breakdowns, graph traversal
- `TRACE`: Full SQL queries, embedding vectors

**Structured Logging**: Uses `tracing` crate with JSON output for production.

### Health Checks

**Database Health**:
```bash
pg_isready -h maproom-postgres -U maproom -d maproom
```

**Embedding Provider Health**:
```bash
# Ollama
curl http://ollama:11434/api/tags

# Google Vertex AI
gcloud auth application-default print-access-token
```

**MCP Server Health**:
```bash
# Check if MCP server responds to initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize"}' | node dist/index.js
```

---

## Future Enhancements

### Phase 3: Advanced Features (Planned)

1. **Reciprocal Rank Fusion (RRF)**
   - Already implemented in `fusion/rrf.rs`
   - More sophisticated than weighted average
   - Better handling of missing results

2. **Cross-Encoder Reranking**
   - Second-stage reranking with BERT-based model
   - Improves top-10 precision
   - Trade-off: Adds 50-100ms latency

3. **Query Caching**
   - LRU cache for frequent queries
   - Cache key: hash(query + options)
   - Invalidation: On upsert/scan

4. **Materialized Views**
   - Precomputed graph importance scores
   - Refresh on chunk_edges changes
   - Eliminates graph traversal latency

### Phase 4: Scalability

1. **Horizontal Scaling**
   - Read replicas for search queries
   - Separate write/read databases
   - Connection pooling (PgBouncer)

2. **Sharding**
   - Partition by repository
   - Route queries to correct shard
   - Federated search across shards

3. **Incremental Indexing**
   - Watch mode for file changes
   - Only reindex modified files
   - Background embedding generation

---

## References

### Key Documentation

- **Database Architecture**: `docs/architecture/DATABASE_ARCHITECTURE.md`
- **Weight Tuning**: `crates/maproom/docs/WEIGHT_TUNING.md`
- **Multi-Provider Embeddings**: `crates/maproom/docs/MPEMBED-4003-implementation-summary.md`
- **Performance Bottlenecks**: `crates/maproom/docs/PERFORMANCE_BOTTLENECKS.md`
- **MCP Server README**: `packages/maproom-mcp/README.md`

### Technology Stack

- **Language**: Rust (core), TypeScript (MCP wrapper)
- **Database**: PostgreSQL 16 + pgvector extension
- **Parsing**: tree-sitter (TypeScript, Rust, Python, Markdown, JSON)
- **Embeddings**: Ollama (mxbai-embed-large), Google Vertex AI (textembedding-gecko), OpenAI (text-embedding-3-small)
- **Vector Index**: ivfflat algorithm (pgvector)
- **Protocol**: Model Context Protocol (MCP) v2024-11-05

### External Dependencies

- **pgvector**: https://github.com/pgvector/pgvector
- **tree-sitter**: https://tree-sitter.github.io/tree-sitter/
- **MCP Specification**: https://modelcontextprotocol.io/
- **Ollama**: https://ollama.ai/
- **Google Vertex AI**: https://cloud.google.com/vertex-ai
- **OpenAI Embeddings**: https://platform.openai.com/docs/guides/embeddings

---

## Glossary

- **Chunk**: Atomic unit of code (function, class, markdown section) with metadata
- **FTS**: Full-Text Search using PostgreSQL tsvector
- **Hybrid Search**: Combination of FTS + vector similarity + graph + temporal signals
- **Fusion**: Combining scores from multiple search strategies
- **MCP**: Model Context Protocol for AI assistant integration
- **pgvector**: PostgreSQL extension for vector similarity search
- **ivfflat**: Inverted file flat index for approximate nearest neighbor search
- **Embedding**: Dense vector representation of code/text for semantic similarity
- **Context Bundle**: Collection of related code chunks assembled within a token budget
- **Worktree**: Git worktree representing a branch or commit snapshot

---

**End of Document**
