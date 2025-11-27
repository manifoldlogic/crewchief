# Performance Tuning Guide

Optimize Maproom for your workload and hardware.

## Performance Overview

| Operation | Typical Duration | Bottleneck |
|-----------|------------------|------------|
| Search (warm) | < 50ms | SQLite query |
| Search (cold) | 200-300ms | Ollama embedding |
| Scan (1000 files) | 30-60s | Embedding generation |
| Upsert (10 files) | 2-5s | Embedding generation |
| Context assembly | 20-50ms | Graph traversal |

## Embedding Generation

Embedding generation is typically the slowest part of indexing.

### Ollama Tuning

**Batch size configuration:**
```bash
# Default: 50 texts per sub-batch, 8 concurrent batches
MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=50
MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=8
```

**For CPU-only systems:**
```bash
# Reduce concurrency to avoid OOM
MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=2
MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=25
```

**For GPU systems:**
```bash
# Increase throughput
MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=100
MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16
```

### GPU Acceleration

Ollama automatically uses available GPUs. Verify with:
```bash
# Check GPU usage
nvidia-smi

# Ollama GPU allocation
ollama ps
```

**CUDA memory tuning:**
```bash
# If OOM errors occur
CUDA_VISIBLE_DEVICES=0 ollama serve
```

### Embedding Cache (Blob SHA)

Maproom caches embeddings by content hash (blob SHA):
- Same code across branches shares embeddings
- 70-90% cost savings on incremental updates
- Cache stored in `code_embeddings` table

**Check cache effectiveness:**
```sql
-- In sqlite3 ~/.maproom/maproom.db
SELECT
  COUNT(*) as total_chunks,
  COUNT(DISTINCT blob_sha) as unique_content,
  ROUND(100.0 * COUNT(DISTINCT blob_sha) / COUNT(*), 1) as dedup_percent
FROM chunks WHERE blob_sha IS NOT NULL;
```

## Search Optimization

### Choose the Right Mode

| Mode | Speed | Quality | Best For |
|------|-------|---------|----------|
| `fts` | Fastest | Keywords | Exact identifiers, function names |
| `vector` | Fast | Semantic | Conceptual queries, similar code |
| `hybrid` | Default | Best | General use |

### Query Optimization

**Effective queries:**
```
"authentication"          # Single concept
"error handler"          # Code pattern
"WebSocket disconnect"   # Specific feature
```

**Ineffective queries:**
```
"How do I authenticate users?"  # Too verbose
"src/auth/login.ts"             # File path (use Glob)
"function handleLogin"          # Too specific
```

### Result Limits

```json
{
  "k": 10,  // Default, usually sufficient
  "k": 5,   // Faster, for quick lookups
  "k": 20   // Max useful, more isn't better
}
```

### Filtering

Narrow searches for faster results:
```json
{
  "filter": "code",              // Skip docs/config
  "filters": {
    "file_type": "ts,tsx",       // Specific extensions
    "recency_threshold": "7 days" // Recent files only
  }
}
```

## Database Tuning

### SQLite Settings (Applied Automatically)

```sql
PRAGMA journal_mode = WAL;      -- Concurrent reads
PRAGMA synchronous = NORMAL;    -- Balanced durability
PRAGMA busy_timeout = 5000;     -- 5s lock wait
PRAGMA foreign_keys = ON;
```

### Connection Pool

Default: 10 connections. Adjust for workload:
```rust
// In SqliteStore initialization
r2d2::Pool::builder()
    .max_size(10)  // Increase for high concurrency
    .build(manager)
```

### WAL Maintenance

```bash
# Check WAL size
ls -la ~/.maproom/maproom.db*

# Manual checkpoint (usually automatic)
sqlite3 ~/.maproom/maproom.db "PRAGMA wal_checkpoint(TRUNCATE)"
```

## Daemon Performance

### Why Use Daemon

| Metric | Process Spawn | Daemon |
|--------|---------------|--------|
| First request | 225ms | 225ms |
| Subsequent | 160-400ms | 20-50ms |
| Memory | ~50MB/request | Shared |

The daemon provides **20-50x speedup** through:
- Connection pooling (no SQLite reconnect)
- Warm OS page cache
- Binary stays loaded

### Request Timeouts

Default: 30 seconds. Adjust in daemon client config:
```typescript
const client = new DaemonClient({
  requestTimeout: 60000,  // 60s for large operations
});
```

## Indexing Performance

### Incremental vs Full Scan

| Scenario | Approach | Speed |
|----------|----------|-------|
| Initial setup | `scan` | Baseline |
| Few files changed | `upsert` | 5-10x faster |
| Major refactor | `scan` | Re-index all |
| Branch switch | Automatic | Cached embeddings |

### Parallel Indexing

```json
{
  "concurrency": 8,    // File processing workers
  "parallel": true     // Enable batch processing
}
```

**Tuning for hardware:**
- **2 CPU cores:** `concurrency: 2`
- **8 CPU cores:** `concurrency: 8` (default: 4)
- **16+ cores:** `concurrency: 16` (diminishing returns)

### Exclude Patterns

Skip unnecessary files:
```json
{
  "exclude": [
    "node_modules/**",
    "dist/**",
    "*.test.ts",
    "*.spec.ts",
    "**/__tests__/**"
  ]
}
```

## Memory Management

### Daemon Memory

Typical: 200-500MB depending on:
- Connection pool size
- SQLite cache
- In-flight requests

**Monitor:**
```bash
watch -n 1 'ps -o pid,rss,comm -p $(pgrep -f crewchief-maproom)'
```

### Ollama Memory

Model loaded: ~2-4GB for nomic-embed-text

**Reduce memory pressure:**
```bash
# Unload unused models
ollama rm unused-model
```

## Benchmarking

### Measure Search Latency

```bash
# Single search
time curl -X POST ... '{"method":"search",...}'

# Multiple searches
for i in {1..10}; do
  time curl -X POST ... '{"method":"search",...}'
done
```

### Check Debug Timing

Enable debug in search:
```json
{
  "debug": true
}
```

Response includes:
- `query_embedding_time_ms` - Ollama call
- `search_time_ms` - Database query
- `fusion_time_ms` - RRF combination

## Recommended Configurations

### Development (Local)

```bash
# Defaults work well
ollama serve
crewchief-maproom scan /path/to/repo
```

### CI/CD (Fast, Limited Resources)

```bash
MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=2
MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=25
```

### Large Codebase (GPU Available)

```bash
MAPROOM_EMBEDDING_PARALLEL_MAX_CONCURRENCY=16
MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=100
```

## Performance Checklist

- [ ] Ollama running with GPU acceleration
- [ ] Using `hybrid` mode for general searches
- [ ] Queries are 1-3 words
- [ ] Excluding unnecessary files (node_modules, etc.)
- [ ] Using incremental updates (`upsert`) when possible
- [ ] Daemon mode enabled (not process-per-request)
- [ ] Search `k` parameter is reasonable (≤ 20)
