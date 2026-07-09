# Debugging Guide

Tools and techniques for diagnosing Maproom issues.

## Enable Debug Logging

### Rust Daemon

```bash
# Info level (default)
RUST_LOG=info maproom serve

# Debug level (verbose)
RUST_LOG=debug maproom serve

# Trace level (very verbose)
RUST_LOG=trace maproom serve

# Module-specific logging
RUST_LOG=crewchief_maproom::search=debug maproom serve
```

### MCP Server

```bash
# Set log level
LOG_LEVEL=debug npx @crewchief/maproom-mcp

# Write to file
MAPROOM_MCP_LOG_FILE=/tmp/maproom.log npx @crewchief/maproom-mcp
```

## Search Debug Mode

Enable debug mode in search requests to see score breakdowns:

```json
{
  "name": "search",
  "arguments": {
    "repo": "crewchief",
    "query": "authentication",
    "debug": true
  }
}
```

**Debug output includes:**
- FTS score (BM25 rank)
- Vector score (cosine similarity)
- RRF fusion scores
- Kind multipliers applied
- Final combined score

## Check Index Status

### Via MCP Tool

```json
{"method": "tools/call", "params": {"name": "status"}}
```

### Direct Database Queries

**SQLite backend** (default — `~/.maproom/maproom.db`):
```bash
# Open database
sqlite3 ~/.maproom/maproom.db

# Count indexed items
SELECT 'repos' as type, COUNT(*) FROM repos
UNION SELECT 'worktrees', COUNT(*) FROM worktrees
UNION SELECT 'files', COUNT(*) FROM files
UNION SELECT 'chunks', COUNT(*) FROM chunks
UNION SELECT 'embeddings', COUNT(*) FROM code_embeddings;

# Recent files indexed
SELECT path, indexed_at
FROM files
ORDER BY indexed_at DESC
LIMIT 10;

# Check embedding coverage
SELECT
  COUNT(*) as total_chunks,
  COUNT(blob_sha) as with_blob_sha,
  (SELECT COUNT(*) FROM code_embeddings) as embeddings
FROM chunks;

# Verify sqlite-vec is working
SELECT vec_version();
```

**PostgreSQL backend** (shared deployment — substitute your connection string):
```bash
# Set connection URL (compose-internal or host port; see DATABASE_ARCHITECTURE.md)
export MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom

# Count indexed items
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT 'repos'       as type, COUNT(*) FROM repos
  UNION SELECT 'worktrees',     COUNT(*) FROM worktrees
  UNION SELECT 'files',         COUNT(*) FROM files
  UNION SELECT 'chunks',        COUNT(*) FROM chunks
  UNION SELECT 'embeddings',    COUNT(*) FROM code_embeddings;
"

# Recent files indexed
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT path, indexed_at FROM files ORDER BY indexed_at DESC LIMIT 10;
"

# Check embedding coverage
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT COUNT(*) as total_chunks,
         COUNT(blob_sha) as with_blob_sha,
         (SELECT COUNT(*) FROM code_embeddings) as embeddings
  FROM chunks;
"

# Active queries (pg_stat_activity equivalent of lsof)
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT pid, state, query_start, left(query, 80) as query_preview
  FROM pg_stat_activity
  WHERE datname = 'maproom'
    AND state != 'idle'
  ORDER BY query_start;
"
```

## Verify Ollama Health

```bash
# Check service is running
curl http://localhost:11434/api/tags

# Test embedding generation
curl http://localhost:11434/api/embed \
  -d '{"model":"mxbai-embed-large","input":["hello world"]}'

# Check available models
ollama list

# Monitor resource usage
ollama ps
```

## Database Inspection

### Check Integrity

**SQLite backend only:**
```bash
sqlite3 ~/.maproom/maproom.db "PRAGMA integrity_check"
```

**PostgreSQL backend:**
```bash
# Check for bloat and dead rows
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT tablename, n_live_tup, n_dead_tup,
         round(100.0 * n_dead_tup / NULLIF(n_live_tup + n_dead_tup, 0), 1) as dead_pct,
         last_vacuum, last_analyze
  FROM pg_stat_user_tables
  ORDER BY n_live_tup DESC;
"
```

### Analyze Performance

**SQLite backend only:**
```bash
# Query plan for search
sqlite3 ~/.maproom/maproom.db "EXPLAIN QUERY PLAN SELECT * FROM chunks WHERE file_id = 1"

# Index statistics
sqlite3 ~/.maproom/maproom.db "ANALYZE; SELECT * FROM sqlite_stat1"
```

**PostgreSQL backend:**
```bash
# EXPLAIN ANALYZE a query
psql "$MAPROOM_DATABASE_URL" -c "
  EXPLAIN (ANALYZE, BUFFERS)
  SELECT c.id FROM chunks c WHERE c.file_id = 1;
"

# Index usage statistics
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT indexname, idx_scan, idx_tup_read,
         pg_size_pretty(pg_relation_size(indexrelid)) as size
  FROM pg_stat_user_indexes
  ORDER BY pg_relation_size(indexrelid) DESC
  LIMIT 20;
"
```

### Check WAL Status

**SQLite backend only:**
```bash
# WAL file size
ls -la ~/.maproom/maproom.db*

# Force checkpoint
sqlite3 ~/.maproom/maproom.db "PRAGMA wal_checkpoint(TRUNCATE)"
```

**PostgreSQL backend:**
```bash
# Check long-running transactions (common source of bloat)
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT pid, now() - pg_stat_activity.query_start AS duration,
         query, state
  FROM pg_stat_activity
  WHERE state != 'idle'
    AND query_start < now() - interval '5 minutes'
  ORDER BY duration DESC;
"
```

## Process Inspection

### Find Running Processes

```bash
# Find daemon processes
pgrep -f "maproom"

# Full process info
ps aux | grep maproom

# Check file handles (SQLite backend only)
lsof ~/.maproom/maproom.db

# PostgreSQL backend: check active connections instead
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT count(*) as connections, state
  FROM pg_stat_activity
  WHERE datname = 'maproom'
  GROUP BY state;
"
```

### Kill Stuck Processes

```bash
# Graceful kill
pkill -TERM -f "maproom serve"

# Force kill if needed
pkill -9 -f "maproom serve"
```

## Network Debugging

### Check Ollama Connectivity

```bash
# Port check
nc -zv localhost 11434

# Detailed HTTP test
curl -v http://localhost:11434/api/tags

# Check for firewall rules
sudo iptables -L -n | grep 11434
```

## Log Analysis

### Common Log Patterns

**Successful search:**
```
INFO search: query="auth" mode=hybrid results=10 duration_ms=45
```

**Embedding cache hit:**
```
DEBUG embedding: blob_sha=abc123 cache_hit=true
```

**Database connection:**
```
INFO sqlite: connected path=~/.maproom/maproom.db pool_size=10   # SQLite backend
INFO postgres: connected pool_size=10 migrations=applied          # PostgreSQL backend
```

**Warning signs:**
```
WARN sqlite: busy timeout exceeded                                 # SQLite backend only
ERROR embedding: ollama connection refused
WARN search: no embeddings found, falling back to FTS
```

### Filtering Logs

```bash
# Search errors only
RUST_LOG=error maproom serve 2>&1 | grep -i error

# Embedding related
RUST_LOG=debug maproom serve 2>&1 | grep embedding

# Search queries
RUST_LOG=info maproom serve 2>&1 | grep "search:"
```

## Performance Profiling

### Time Operations

```bash
# Time a scan
time maproom scan /path/to/repo

# Time a search (via daemon)
time curl -X POST ... '{"method":"search",...}'
```

### Memory Usage

```bash
# Monitor memory
watch -n 1 'ps -o pid,rss,comm -p $(pgrep -f maproom)'

# Check system memory
free -h
```

## Reset and Recovery

### Soft Reset (Keep Data)

**SQLite backend:**
```bash
# Kill all processes
pkill -f maproom

# Checkpoint WAL (SQLite backend only)
sqlite3 ~/.maproom/maproom.db "PRAGMA wal_checkpoint(TRUNCATE)"

# Restart clean
maproom serve
```

**PostgreSQL backend:**
```bash
# Kill maproom processes; the DB server keeps running
pkill -f maproom
# Terminate any stuck backend connections
psql "$MAPROOM_DATABASE_URL" -c "
  SELECT pg_terminate_backend(pid)
  FROM pg_stat_activity
  WHERE datname = 'maproom' AND state = 'idle in transaction'
    AND query_start < now() - interval '30 minutes';
"
maproom serve
```

### Hard Reset (Fresh Start)

**SQLite backend:**
```bash
# Stop everything
pkill -f maproom

# Remove database
rm ~/.maproom/maproom.db*

# Re-index
maproom scan /path/to/repo
```

**PostgreSQL backend:**
```bash
# Stop maproom processes
pkill -f maproom

# Drop and recreate the maproom DB (destructive — deletes all indexed data)
psql "postgresql://maproom:maproom@maproom-postgres:5432/postgres" \
  -c "DROP DATABASE IF EXISTS maproom; CREATE DATABASE maproom;"

# Re-index
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom \
  maproom scan /path/to/repo
```

### Selective Re-index

```bash
# Re-index specific files
maproom upsert \
  --paths "src/auth/*.ts" \
  --commit HEAD \
  --repo myproject \
  --worktree main \
  --root /path/to/repo
```

## Reporting Issues

When reporting bugs, include:

1. **Environment info:**
   ```bash
   maproom --version
   ollama --version
   sqlite3 --version
   uname -a
   ```

2. **Debug logs:**
   ```bash
   RUST_LOG=debug maproom serve 2>&1 | tee debug.log
   ```

3. **Database state:**
   ```sql
   SELECT 'chunks', COUNT(*) FROM chunks
   UNION SELECT 'embeddings', COUNT(*) FROM code_embeddings;
   ```

4. **Steps to reproduce**

5. **Expected vs actual behavior**
