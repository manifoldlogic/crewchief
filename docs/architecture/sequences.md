# Maproom Sequence Diagrams

Detailed request/response flows for key operations.

## 1. Search Request Flow

```mermaid
sequenceDiagram
    participant CC as Claude Code
    participant MCP as MCP Server
    participant DC as Daemon Client
    participant DM as Rust Daemon
    participant OL as Ollama
    participant DB as SQLite

    CC->>MCP: tools/call search<br/>{repo, query, mode: "hybrid"}
    MCP->>DC: search(params)

    alt Daemon not running
        DC->>DM: spawn crewchief-maproom serve
        DM-->>DC: ready (JSON-RPC)
    end

    DC->>DM: JSON-RPC search request
    DM->>OL: POST /api/embed<br/>{model: "nomic-embed-text", input: [query]}
    OL-->>DM: {embeddings: [[768 floats]]}

    par Parallel Search
        DM->>DB: FTS5 query (keyword match)
        DB-->>DM: FTS results with BM25 scores
    and
        DM->>DB: sqlite-vec query (cosine similarity)
        DB-->>DM: Vector results with distances
    end

    DM->>DM: RRF fusion (k=60)
    DM->>DM: Semantic ranking (kind multipliers)
    DM-->>DC: SearchResult {hits, total, timing}
    DC-->>MCP: SearchResult
    MCP-->>CC: MCP response with results
```

## 2. Indexing Flow (Scan)

```mermaid
sequenceDiagram
    participant U as User/Tool
    participant MCP as MCP Server
    participant DM as Rust Daemon
    participant TS as tree-sitter
    participant OL as Ollama
    participant DB as SQLite

    U->>MCP: tools/call scan<br/>{repo, root, paths}
    MCP->>DM: JSON-RPC scan request

    loop For each file
        DM->>TS: Parse file
        TS-->>DM: AST
        DM->>DM: Extract chunks<br/>(functions, classes, imports)

        loop For each chunk
            DM->>DM: Calculate blob SHA
            DM->>DB: Check code_embeddings<br/>WHERE blob_sha = ?

            alt Cache hit
                DB-->>DM: Existing embedding
            else Cache miss
                Note over DM,OL: Batch embeddings (50 per request)
            end
        end
    end

    DM->>OL: POST /api/embed<br/>{input: [batch of texts]}
    OL-->>DM: {embeddings: [[768 floats], ...]}

    DM->>DB: BEGIN TRANSACTION
    DM->>DB: INSERT INTO chunks
    DM->>DB: INSERT INTO code_embeddings
    DM->>DB: INSERT INTO vec_code_768
    DM->>DB: INSERT INTO chunk_edges
    DM->>DB: COMMIT

    DM-->>MCP: ScanResult {files, chunks, embeddings}
    MCP-->>U: Scan complete
```

## 3. Daemon Lifecycle

```mermaid
sequenceDiagram
    participant DC as Daemon Client
    participant DL as Daemon Lifecycle
    participant DM as Rust Daemon
    participant DB as SQLite

    Note over DC: First request arrives
    DC->>DL: ensureRunning()

    alt Daemon not started
        DL->>DM: spawn crewchief-maproom serve
        DM->>DB: Connect to SQLite<br/>(~/.maproom/maproom.db)
        DB-->>DM: Connection pool ready
        DM-->>DL: JSON-RPC ready line
        DL->>DM: ping()
        DM-->>DL: pong
        DL-->>DC: Daemon ready
    end

    DC->>DM: JSON-RPC request
    DM-->>DC: JSON-RPC response

    Note over DC,DM: Connection kept alive

    alt Daemon crashes
        DM--xDC: Process exit
        DC->>DL: restart()
        DL->>DL: Exponential backoff<br/>(100ms, 200ms, 400ms...)

        alt Retry < max attempts
            DL->>DM: spawn (retry)
            DM-->>DL: ready
        else Max retries exceeded
            DL->>DL: Circuit breaker OPEN
            DL-->>DC: DaemonUnhealthyError
        end
    end
```

## 4. Provider Auto-Detection

```mermaid
sequenceDiagram
    participant DM as Rust Daemon
    participant OL as Ollama
    participant CFG as Config

    Note over DM: Startup
    DM->>CFG: Read MAPROOM_EMBEDDING_PROVIDER

    alt Explicit provider set
        CFG-->>DM: "ollama"
        DM->>OL: GET http://localhost:11434/api/tags

        alt Ollama responds
            OL-->>DM: 200 OK
            DM->>DM: Use Ollama provider
        else Connection refused
            DM-->>DM: Error: Ollama not running
        end

    else No provider configured
        DM->>OL: GET http://localhost:11434/api/tags<br/>(2s timeout)

        alt Ollama detected
            OL-->>DM: 200 OK
            DM->>DM: Auto-select Ollama
            Note over DM: nomic-embed-text (768-dim)
        else Timeout/error
            DM-->>DM: Error with guidance:<br/>"Start Ollama: ollama serve"
        end
    end

    DM->>OL: Check model availability<br/>GET /api/tags

    alt Model available
        OL-->>DM: {models: ["nomic-embed-text", ...]}
    else Model missing
        DM-->>DM: Warning: Run "ollama pull nomic-embed-text"
    end
```

## 5. Context Assembly Flow

```mermaid
sequenceDiagram
    participant U as User/Tool
    participant MCP as MCP Server
    participant DM as Rust Daemon
    participant DB as SQLite

    U->>MCP: tools/call context<br/>{chunk_id, budget_tokens: 6000}
    MCP->>DM: JSON-RPC context request

    DM->>DB: SELECT * FROM chunks<br/>WHERE id = ?
    DB-->>DM: Target chunk

    par Relationship Traversal
        DM->>DB: SELECT callers FROM chunk_edges<br/>(recursive CTE, max depth 3)
        DB-->>DM: Caller chunks
    and
        DM->>DB: SELECT callees FROM chunk_edges
        DB-->>DM: Callee chunks
    and
        DM->>DB: SELECT imports FROM chunk_edges
        DB-->>DM: Import chunks
    and
        DM->>DB: SELECT tests WHERE<br/>edge_type = 'tests'
        DB-->>DM: Test chunks
    end

    DM->>DM: Rank by importance<br/>(kind multiplier × recency × distance)

    loop Fill token budget
        DM->>DM: Add highest-ranked chunk
        DM->>DM: Subtract tokens from budget

        alt Budget exhausted
            Note over DM: Stop adding chunks
        end
    end

    DM-->>MCP: ContextBundle<br/>{target, related[], relationships}
    MCP-->>U: Context response
```

## Timing Characteristics

| Operation | Typical Duration | Notes |
|-----------|------------------|-------|
| Daemon startup | 200-500ms | First request only |
| Ollama embed (single) | 50-100ms | Local GPU accelerated |
| Ollama embed (batch 50) | 500-800ms | Parallel processing |
| FTS5 search | 5-20ms | BM25 ranking |
| Vector search | 10-30ms | sqlite-vec cosine |
| Hybrid fusion | < 5ms | In-memory RRF |
| Context assembly | 20-50ms | Graph traversal |
| Full search (warm) | < 50ms | End-to-end |
