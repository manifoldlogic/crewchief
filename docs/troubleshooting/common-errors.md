# Common Errors

Solutions for frequently encountered Maproom issues.

---

## Ollama Connection Errors

### Error: Connection refused to Ollama

**Symptoms:**
- `connection refused` at `localhost:11434`
- Embedding generation fails during scan/upsert
- Error message: "Failed to connect to Ollama"

**Causes:**
1. Ollama service not running
2. Ollama running on different port
3. Firewall blocking localhost connections

**Solutions:**

```bash
# 1. Start Ollama
ollama serve

# 2. Verify it's running
curl http://localhost:11434/api/tags

# 3. Check if model is available
ollama list

# Expected output should show nomic-embed-text
```

### Error: Model not found

**Symptoms:**
- `model 'nomic-embed-text' not found`
- Scan completes but no embeddings generated

**Solution:**
```bash
# Pull the embedding model
ollama pull nomic-embed-text

# Verify it's available
ollama list | grep nomic
```

### Error: Ollama timeout

**Symptoms:**
- Requests hang then timeout after 60s
- Works for small batches but fails on large scans

**Causes:**
1. Model loading on first request
2. GPU memory constraints
3. Large batch sizes

**Solutions:**
```bash
# Warm up the model first
curl http://localhost:11434/api/embed -d '{"model":"nomic-embed-text","input":["test"]}'

# Check GPU memory usage
nvidia-smi  # If using GPU

# Reduce batch size via environment
MAPROOM_EMBEDDING_PARALLEL_SUB_BATCH_SIZE=25 crewchief-maproom scan
```

---

## SQLite Errors

### Error: Database is locked

**Symptoms:**
- `SQLITE_BUSY: database is locked`
- Concurrent operations fail
- Usually during scan while searching

**Causes:**
- Multiple processes writing simultaneously
- Long-running transaction blocking others
- WAL checkpoint in progress

**Solutions:**
```bash
# 1. Check for other processes
lsof ~/.maproom/maproom.db

# 2. Wait and retry (usually resolves automatically)
# SQLite has 5000ms busy timeout configured

# 3. If stuck, check for zombie processes
pgrep -f crewchief-maproom
pkill -f "crewchief-maproom serve"  # Kill stuck daemon
```

### Error: Database file not found

**Symptoms:**
- `no such file or directory: ~/.maproom/maproom.db`
- Search returns no results

**Solutions:**
```bash
# 1. Check database location
ls -la ~/.maproom/

# 2. Create directory if missing
mkdir -p ~/.maproom

# 3. Run initial scan to create database
crewchief-maproom scan /path/to/repo
```

### Error: sqlite-vec extension not loaded

**Symptoms:**
- Vector search returns no results
- FTS search works but hybrid mode degrades
- Warning in logs about missing extension

**Cause:**
The sqlite-vec extension should be statically linked, but may fail on some platforms.

**Solutions:**
```bash
# 1. Check if extension is available
sqlite3 ~/.maproom/maproom.db "SELECT vec_version()"

# 2. Rebuild with correct features
cargo build --release --features sqlite

# 3. Fall back to FTS-only mode
# Search with mode: "fts" instead of "hybrid"
```

---

## Search Errors

### Error: Repository not found

**Symptoms:**
- `Repository 'xyz' not found`
- `available_repos: []` in response

**Causes:**
1. Never indexed the repository
2. Repository name mismatch
3. Database was reset

**Solutions:**
```bash
# 1. Check indexed repos via status tool
# Look at available_repos in response

# 2. Scan the repository
crewchief-maproom scan /path/to/repo

# 3. Verify repo name matches git remote
git remote -v
```

### Search returns no results

**Symptoms:**
- Empty results for queries that should match
- `total: 0` in response

**Causes:**
1. Repository not indexed
2. Query too specific
3. Wrong worktree scope
4. Embeddings not generated

**Solutions:**
```bash
# 1. Check index status first
# Use status tool to verify repo is indexed

# 2. Simplify query
# "authentication handler validate" → "authentication"

# 3. Try FTS mode for exact matches
# Set mode: "fts" instead of "hybrid"

# 4. Remove worktree filter
# Omit worktree parameter to search all branches

# 5. Verify embeddings exist
sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM code_embeddings"
```

### Search results are stale

**Symptoms:**
- Search finds old versions of code
- Recently modified files not appearing
- Deleted code still in results

**Solutions:**
```bash
# 1. Re-index changed files
crewchief-maproom upsert --paths "src/changed.ts" --commit HEAD

# 2. Full re-scan if many changes
crewchief-maproom scan /path/to/repo

# 3. Check file timestamps vs index
sqlite3 ~/.maproom/maproom.db "SELECT path, indexed_at FROM files ORDER BY indexed_at DESC LIMIT 10"
```

---

## Daemon Errors

### Error: Daemon crashed repeatedly

**Symptoms:**
- `DaemonUnhealthyError: Circuit breaker open`
- Repeated restart attempts in logs
- All MCP requests fail

**Causes:**
1. Database corruption
2. Out of memory
3. Configuration error

**Solutions:**
```bash
# 1. Check daemon logs
RUST_LOG=debug crewchief-maproom serve

# 2. Test database integrity
sqlite3 ~/.maproom/maproom.db "PRAGMA integrity_check"

# 3. Reset database if corrupted
rm ~/.maproom/maproom.db
crewchief-maproom scan /path/to/repo
```

### Error: Request timeout

**Symptoms:**
- `DaemonTimeoutError: Request timed out after 30s`
- Search hangs indefinitely

**Causes:**
1. Very large result sets
2. Database lock contention
3. Slow disk I/O

**Solutions:**
```bash
# 1. Reduce result count
# Set k: 5 instead of k: 20

# 2. Check disk I/O
iostat -x 1 5

# 3. Kill and restart daemon
pkill -f "crewchief-maproom serve"
```

---

## MCP Protocol Errors

### Error: Invalid JSON-RPC request

**Symptoms:**
- `Parse error` or `Invalid Request`
- Works in one client but not another

**Solutions:**
- Ensure request follows JSON-RPC 2.0 format
- Check `jsonrpc: "2.0"` field is present
- Verify `method` and `params` structure

### Error: Method not found

**Symptoms:**
- `-32601 Method not found`

**Solutions:**
- Check tool name spelling
- Use `tools/list` to see available tools
- Ensure MCP protocol version matches

---

## Quick Diagnostic Checklist

When something doesn't work:

1. **Check Ollama:** `curl http://localhost:11434/api/tags`
2. **Check database:** `ls -la ~/.maproom/maproom.db`
3. **Check index:** Use `status` MCP tool
4. **Enable debug:** `RUST_LOG=debug`
5. **Kill stuck processes:** `pkill -f crewchief-maproom`

See [Debugging Guide](debugging.md) for detailed diagnostic procedures.
