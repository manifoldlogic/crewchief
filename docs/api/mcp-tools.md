# MCP Tools Reference

Maproom exposes these tools via the Model Context Protocol (MCP) for semantic code search.

## Quick Reference

| Tool | Purpose | Required Params |
|------|---------|-----------------|
| `status` | Check index health | (none) |
| `search` | Find code semantically | `repo`, `query` |
| `open` | Retrieve file content | `relpath`, `worktree` |
| `context` | Get related code | `chunk_id` |
| `scan` | Index entire repository | (none, auto-detects) |
| `upsert` | Update specific files | `paths`, `commit`, `repo`, `worktree`, `root` |
| `explain` | Symbol documentation | `chunk_id` |

## Workflow

```
1. status    → Check what's indexed
2. search    → Find relevant code
3. open      → View specific file/range
4. context   → Get related chunks
```

---

## status

Check the maproom index status. **Always use this first** to understand what's searchable.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `repo` | string | No | Filter to specific repository |

### Response

```json
{
  "repos": ["crewchief"],
  "totalRepos": 1,
  "totalFiles": 245,
  "totalChunks": 1842,
  "backendType": "sqlite",
  "sqlitePath": "~/.maproom/maproom.db"
}
```

### Example

```json
{"method": "tools/call", "params": {"name": "status", "arguments": {}}}
```

---

## search

Semantic code search combining keyword matching and vector similarity.

### Parameters

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `repo` | string | Yes | - | Repository name (e.g., "crewchief") |
| `query` | string | Yes | - | Search query (1-3 words work best) |
| `worktree` | string | No | auto | Limit to specific worktree/branch |
| `k` | integer | No | 10 | Number of results (max useful: 20) |
| `mode` | enum | No | "hybrid" | `fts`, `vector`, or `hybrid` |
| `filter` | enum | No | "all" | `all`, `code`, `docs`, `config` |
| `filters` | object | No | - | Advanced filters (see below) |
| `debug` | boolean | No | false | Show score breakdowns |
| `deduplicate` | boolean | No | true | Deduplicate across worktrees |

### Search Modes

| Mode | Best For | Speed |
|------|----------|-------|
| `fts` | Exact keywords, identifiers | Fastest |
| `vector` | Conceptual queries, similar code | Fast |
| `hybrid` | General use (combines both) | Default |

### Advanced Filters

```json
{
  "filters": {
    "file_type": "ts,tsx",
    "recency_threshold": "7 days"
  }
}
```

| Filter | Description | Example |
|--------|-------------|---------|
| `file_type` | Comma-separated extensions | `"ts,tsx,js"` |
| `recency_threshold` | Modified within timeframe | `"7 days"`, `"1 month"` |

### Query Best Practices

**Good queries:**
- `"authentication"` - Single concept
- `"error handler"` - Code pattern
- `"WebSocket disconnect"` - Specific feature
- `"processCheckout"` - Function name

**Avoid:**
- `"How do I handle errors?"` - Full sentences
- `"src/cart/checkout.ts"` - File paths (use Glob)
- `"TODO"` - Exact strings (use Grep)

### Response

```json
{
  "hits": [
    {
      "file_path": "packages/cli/src/auth/login.ts",
      "chunk_id": "550e8400-e29b-41d4-a716-446655440000",
      "symbol": "handleLogin",
      "kind": "function",
      "start_line": 42,
      "end_line": 78,
      "content": "async function handleLogin(credentials)...",
      "score": 0.847
    }
  ],
  "total": 15,
  "query_embedding_time_ms": 45,
  "search_time_ms": 12
}
```

### Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {
      "repo": "crewchief",
      "query": "authentication",
      "mode": "hybrid",
      "k": 10
    }
  }
}
```

---

## open

Retrieve specific code from indexed files. Use after `search` to view full context.

### Parameters

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `relpath` | string | Yes | - | Relative file path (from search results) |
| `worktree` | string | Yes | - | Worktree name (from search results) |
| `range` | object | No | - | Line range `{start, end}` |
| `context` | integer | No | 0 | Extra context lines around range |

### Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "open",
    "arguments": {
      "relpath": "packages/cli/src/auth/login.ts",
      "worktree": "main",
      "range": {"start": 42, "end": 78},
      "context": 5
    }
  }
}
```

### Response

Returns the file content with line numbers.

---

## context

Retrieve contextually relevant code around a chunk. Assembles related imports, callers, callees, and tests within a token budget.

### Parameters

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `chunk_id` | string | Yes | - | Chunk UUID (from search results) |
| `budget_tokens` | integer | No | 6000 | Max tokens (1000-20000) |
| `expand` | object | No | - | Control which relationships to include |

### Expand Options

```json
{
  "expand": {
    "callers": true,
    "callees": true,
    "tests": true,
    "docs": false,
    "config": false,
    "max_depth": 2
  }
}
```

| Option | Default | Description |
|--------|---------|-------------|
| `callers` | true | Chunks that call this function |
| `callees` | true | Chunks called by this function |
| `tests` | true | Test files for this code |
| `docs` | false | Documentation chunks |
| `config` | false | Related config files |
| `max_depth` | 2 | Relationship traversal depth (max 5) |

### Response

```json
{
  "target": {
    "chunk_id": "...",
    "symbol": "handleLogin",
    "content": "..."
  },
  "related": [
    {
      "chunk_id": "...",
      "symbol": "validateCredentials",
      "relationship": "callee",
      "content": "..."
    }
  ],
  "total_tokens": 4250
}
```

### Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "context",
    "arguments": {
      "chunk_id": "550e8400-e29b-41d4-a716-446655440000",
      "budget_tokens": 8000
    }
  }
}
```

---

## scan

Index an entire repository. Auto-detects repo name, branch, and path.

### Parameters

| Name | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `repo` | string | No | auto | Repository name |
| `worktree` | string | No | auto | Worktree/branch name |
| `path` | string | No | cwd | Path to scan |
| `commit` | string | No | HEAD | Git commit hash |
| `concurrency` | integer | No | 4 | Parallel workers (1-16) |
| `parallel` | boolean | No | false | Enable parallel batching |
| `languages` | array | No | all | Limit to specific languages |
| `exclude` | array | No | - | Glob patterns to exclude |

### Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "scan",
    "arguments": {
      "path": "/workspace",
      "languages": ["typescript", "rust"],
      "exclude": ["node_modules/**", "*.test.ts"]
    }
  }
}
```

### Response

```json
{
  "files_processed": 245,
  "chunks_created": 1842,
  "embeddings_generated": 1650,
  "embeddings_cached": 192,
  "duration_ms": 45000
}
```

---

## upsert

Update specific files in the index. Use for targeted updates after file changes.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `paths` | array | Yes | File paths to index |
| `commit` | string | Yes | Git commit hash (or "HEAD") |
| `repo` | string | Yes | Repository name |
| `worktree` | string | Yes | Worktree name |
| `root` | string | Yes | Repository root directory |

### Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "upsert",
    "arguments": {
      "paths": ["src/auth/login.ts", "src/auth/logout.ts"],
      "commit": "HEAD",
      "repo": "crewchief",
      "worktree": "main",
      "root": "/workspace"
    }
  }
}
```

---

## explain

Generate a symbol card with detailed documentation for a code chunk.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `chunk_id` | string/integer | Yes | Chunk ID from search results |

### Response

Returns markdown-formatted explanation including:
- Symbol metadata (kind, file, lines)
- Code preview
- Relationships (callers, callees)
- Usage examples

### Example

```json
{
  "method": "tools/call",
  "params": {
    "name": "explain",
    "arguments": {
      "chunk_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

---

## Error Responses

All tools return JSON-RPC errors on failure:

```json
{
  "error": {
    "code": -32000,
    "message": "Repository 'unknown' not found",
    "data": {
      "available_repos": ["crewchief"]
    }
  }
}
```

### Common Error Codes

| Code | Meaning |
|------|---------|
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32000 | Application error (see message) |
