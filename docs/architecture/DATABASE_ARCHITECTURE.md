# Database Architecture

## Overview

CrewChief supports **two database backends** for Maproom semantic search:

- **SQLite** (Default) - Zero configuration, perfect for individual developers
- **PostgreSQL** - For team sharing and high-concurrency production deployments

Both backends provide the same core functionality: semantic code search with vector embeddings. The choice depends on your use case and infrastructure requirements.

## Database Backend Options

### Quick Comparison

| Feature | SQLite (Default) | PostgreSQL |
|---------|------------------|------------|
| **Setup Required** | None | Docker or managed service |
| **Configuration** | Zero-config | Environment variables |
| **Best For** | Individual use, CI/CD | Teams, production |
| **Concurrent Access** | Single-writer | Multiple concurrent |
| **Vector Search** | sqlite-vec | pgvector |
| **File Location** | `~/.maproom/maproom.db` | Network service |
| **Embedding Dimensions** | 768 or 1536 | 768 or 1536 |

### When to Use SQLite (Recommended Default)

**Choose SQLite when:**
- You're an individual developer working on your own projects
- You want to get started immediately without Docker
- You're running in CI/CD pipelines
- You need a portable, self-contained database
- You're using the VSCode extension for personal use

**SQLite limitations:**
- Single-writer (no concurrent indexing from multiple processes)
- No parallel query execution
- Database locked during writes

### When to Use PostgreSQL

**Choose PostgreSQL when:**
- Multiple team members share a code index
- You need concurrent indexing across multiple worktrees
- You're deploying for production with high query volume
- You need advanced features (recursive CTEs, parallel queries)
- You require database replication or backup strategies

---

## SQLite Backend (Default)

SQLite is the **recommended default** for most users. It works immediately after install with zero configuration.

### How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│                     SQLite Architecture                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   User's Machine                                                │
│   ├── ~/.maproom/                                               │
│   │   └── maproom.db          ← Single-file database           │
│   │       ├── repos            (repository metadata)           │
│   │       ├── worktrees        (git worktrees)                 │
│   │       ├── chunks           (code chunks with embeddings)   │
│   │       └── chunk_edges      (relationships)                 │
│   │                                                             │
│   └── crewchief maproom scan  ← CLI creates DB automatically   │
│       crewchief maproom search                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Getting Started

```bash
# No setup required! Just run:
crewchief maproom scan

# Database created automatically at ~/.maproom/maproom.db
# Search immediately:
crewchief maproom search "authentication flow"
```

### SQLite Configuration

By default, SQLite is used when no `MAPROOM_DATABASE_URL` is set. You can also set it explicitly:

```bash
# Explicit SQLite URL (optional)
export MAPROOM_DATABASE_URL="sqlite://~/.maproom/maproom.db"

# Custom location
export MAPROOM_DATABASE_URL="sqlite:///path/to/custom/maproom.db"
```

### SQLite Schema

The SQLite schema mirrors PostgreSQL with these tables:

| Table | Purpose |
|-------|---------|
| `repos` | Repository metadata (name, path, remote URL) |
| `worktrees` | Git worktrees within repositories |
| `chunks` | Code chunks with content, embeddings, and metadata |
| `chunk_edges` | Relationships between chunks (imports, calls) |
| `symbols` | Extracted symbols (functions, classes, etc.) |

### Vector Search with sqlite-vec

SQLite uses the [sqlite-vec](https://github.com/asg017/sqlite-vec) extension for vector similarity search:

- Supports 768-dimensional (Ollama) and 1536-dimensional (OpenAI) embeddings
- Uses approximate nearest neighbor search
- Embedded in the Rust binary (no external dependencies)

### SQLite Limitations

1. **Single-Writer**: Only one process can write at a time. If you run `maproom scan` in two terminals simultaneously, one will wait.

2. **No Parallel Queries**: Complex queries run sequentially. PostgreSQL can parallelize across CPU cores.

3. **Database Locking**: During writes, reads may be briefly blocked.

4. **No Network Access**: SQLite is a local file. For team sharing, use PostgreSQL.

---

## PostgreSQL Backend (Team/Production)

PostgreSQL is recommended for team environments and production deployments where concurrent access is required.

```
┌──────────────────────────────────────────────────────────────────┐
│                      Docker Environment                           │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │             Maproom Network (maproom-network)               │ │
│  │                                                             │ │
│  │  ┌──────────────────────┐     ┌──────────────────────────┐ │ │
│  │  │    PostgreSQL        │     │   Devcontainer           │ │ │
│  │  │ (maproom-postgres)   │────▶│   - cargo run            │ │ │
│  │  │                      │     │   - pnpm dev             │ │ │
│  │  │ Host: maproom-       │     │   - Integration tests    │ │ │
│  │  │       postgres       │     └──────────────────────────┘ │ │
│  │  │ Port: 5432           │                                  │ │
│  │  │ User: maproom        │     ┌──────────────────────────┐ │ │
│  │  │ DB: maproom          │────▶│   Maproom MCP            │ │ │
│  │  │ Image: pg16          │     │   - MCP Server           │ │ │
│  │  │ + pgvector           │     │   - Claude/Cursor        │ │ │
│  │  └──────────────────────┘     │   - npx maproom-mcp      │ │ │
│  │                               └──────────────────────────┘ │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  Data: Persistent via Docker volumes                             │
│  Purpose: Development + Production                               │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

## Database Details

**Maproom PostgreSQL** (`maproom-postgres:5432/maproom`)
- **Purpose**: Semantic code search, MCP service, development, testing
- **Connection**: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- **Network**: Accessible from devcontainer and MCP containers via `maproom-network`
- **Data**: Persistent via Docker volumes (`maproom-data`)
- **Image**: `pgvector/pgvector:pg16`
- **Container**: Managed via `config/docker-compose.yml` in maproom-mcp package

**Connection Details**:
```bash
Host:     maproom-postgres
Port:     5432 (internal), 5433 (host)
User:     maproom
Password: maproom
Database: maproom
```

**Connection String**:
```bash
MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom
```

## Connection Fallback System

Both Rust binary and Node.js CLI use intelligent fallback to automatically detect the database:

### Connection Priority (5-tier fallback)

1. **`--database-url`** global CLI flag (highest precedence; overrides `MAPROOM_DATABASE_URL` for the process)
   ```bash
   maproom --database-url "postgresql://maproom:maproom@localhost:5432/maproom" status
   ```
2. **MAPROOM_DATABASE_URL** environment variable (explicit configuration)
   ```bash
   export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
   ```
   - Recommended for production
   - Respects user's explicit choice
   - A `postgres://`/`postgresql://` URL selects the PostgreSQL backend (requires a `--features postgres` build); any other value selects SQLite

3. **MAPROOM_DB_HOST** environment variable (component override)
   ```bash
   export MAPROOM_DB_HOST="custom-postgres"
   export MAPROOM_DB_PORT="5432"  # optional, defaults to 5432
   ```
   - Useful for custom database setups
   - Builds connection string from components
   - Allows flexible configuration

4. **maproom-postgres** hostname resolution (auto-detection)
   - Attempts to resolve `maproom-postgres` hostname
   - Works automatically in Docker environments
   - No configuration needed if container is running
   - Used by default in devcontainer

5. **localhost:5433** (development fallback)
   ```bash
   # Connects to postgresql://maproom:maproom@127.0.0.1:5433/maproom
   ```
   - Final fallback for local testing
   - Useful when running postgres on host machine
   - Non-standard port (5433) to avoid conflicts

### Connection Examples

**Devcontainer** (auto-detection works automatically):
```bash
# No MAPROOM_DATABASE_URL needed - auto-detects maproom-postgres
cargo run --bin maproom -- scan /workspace

# Or set explicitly
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
```

**MCP Service**:
```bash
# Auto-detects when maproom-postgres container is running
npx @crewchief/maproom-mcp

# Or set explicitly in docker-compose
environment:
  - MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom
```

**Custom Setup**:
```bash
# Using component override
export MAPROOM_DB_HOST="my-custom-postgres"
export MAPROOM_DB_PORT="5432"
cargo run --bin maproom -- scan /workspace
```

## Starting the Database

**Using Docker Compose**:
```bash
# From maproom-mcp config directory
cd /workspace/packages/maproom-mcp/config
docker compose up -d

# Or from standalone config
cd /workspace/config
docker compose up -d

# Verify it's running
docker ps | grep maproom-postgres
```

**Health Check**:
```bash
# Check container status
docker logs maproom-postgres

# Test connection
psql "postgresql://maproom:maproom@maproom-postgres:5432/maproom" -c "SELECT version();"
```

## Performance Optimizations

The database includes performance tuning for vector search workloads:

**postgresql.conf settings**:
- `shared_buffers=512MB` - Large buffer cache for vector operations
- `effective_cache_size=3GB` - Query planner optimization
- `maintenance_work_mem=256MB` - Index creation and VACUUM
- `random_page_cost=1.1` - SSD-optimized
- `max_parallel_workers=4` - Parallel query execution

## Schema

The maproom database schema includes core tables for code indexing and several recent additions for content-addressed storage and worktree tracking.

### Core Tables

**Core schema** (migrations 0000-0017):
- `repos` - Repository metadata
- `worktrees` - Git worktrees within repositories
- `files` - Indexed files
- `chunks` - Code chunks extracted from files
- `chunk_relationships` - Dependencies between chunks

### Blob SHA Column (Migration 0018)

The `chunks` table includes a `blob_sha` column for content-addressed storage:

```sql
ALTER TABLE maproom.chunks ADD COLUMN blob_sha TEXT NOT NULL;
CREATE INDEX idx_chunks_blob_sha ON maproom.chunks(blob_sha);
```

**Purpose**: Enable deduplication of embeddings based on content hash (git-compatible blob SHA).

**Status**: Column exists and populated (backfilled with SHA-256 hashes of chunk content).

**Future**: Multiple chunks with identical content will reference same embedding in code_embeddings table (implementation pending BLOBSHA-IMPL project).

**Function**: `compute_git_blob_sha(text)` - PostgreSQL function that computes git-compatible blob SHA
- Format: `SHA256("blob <size>\0<content>")`
- Matches Rust implementation in `crates/maproom/src/content_hash.rs`

### Code Embeddings Table (Migration 0019)

Deduplicated storage for code embeddings:

```sql
CREATE TABLE maproom.code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536) NOT NULL,
  model_version TEXT NOT NULL DEFAULT 'text-embedding-3-small',
  created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_embeddings_vector ON maproom.code_embeddings
  USING hnsw (embedding vector_cosine_ops);
```

**Purpose**: Store one embedding per unique blob_sha, reducing embedding costs by 70-90%.

**Status**: Table exists but not yet populated (embeddings generation pending BLOBSHA-IMPL project).

**Index**: HNSW (Hierarchical Navigable Small World) index for fast approximate nearest neighbor search using cosine similarity.

**Foreign Key**: Disabled during migration for existing data. Will be enabled after indexer populates embeddings.

### Worktree Tracking (Migration 0020)

BRANCHX schema for worktree-aware indexing:

```sql
-- worktree_ids JSONB column in chunks table
ALTER TABLE maproom.chunks
  ADD COLUMN worktree_ids JSONB DEFAULT '[]'::jsonb NOT NULL;

CREATE INDEX idx_chunks_worktree_ids ON maproom.chunks
  USING gin (worktree_ids);

-- Tracking table for worktree index state
CREATE TABLE maproom.worktree_index_state (
  worktree_id BIGINT PRIMARY KEY REFERENCES maproom.worktrees(id) ON DELETE CASCADE,
  last_tree_sha TEXT,
  last_indexed TIMESTAMP DEFAULT NOW(),
  chunks_processed BIGINT DEFAULT 0,
  embeddings_generated BIGINT DEFAULT 0
);
```

**Purpose**: Track which worktrees contain each chunk, enable incremental indexing.

**Status**: Schema complete, incremental update logic pending BRANCHX-IMPL project.

**JSONB Operators**: Use `?` (contains), `?|` (any of), `?&` (all of), `-` (remove) for querying worktree_ids.

**Example Queries**:
```sql
-- Find chunks in specific worktree
SELECT * FROM maproom.chunks WHERE worktree_ids ? '123';

-- Find chunks in any of multiple worktrees
SELECT * FROM maproom.chunks WHERE worktree_ids ?| ARRAY['123', '456'];

-- Get worktree index statistics
SELECT
  w.name,
  wis.last_tree_sha,
  wis.chunks_processed,
  wis.last_indexed
FROM maproom.worktree_index_state wis
JOIN maproom.worktrees w ON w.id = wis.worktree_id
ORDER BY wis.last_indexed DESC;
```

### Migration History

See `crates/maproom/migrations/` for all migration SQL files.

**Recent additions** (SCHMAFIX project, November 2025):
- Migration 0018: `add_blob_sha.sql` - Content-addressed storage foundation
- Migration 0019: `create_code_embeddings.sql` - Deduplicated embeddings table
- Migration 0020: `add_worktree_tracking.sql` - Worktree-aware chunk tracking

**To view applied migrations**:
```bash
psql $MAPROOM_DATABASE_URL -c "SELECT version, filename FROM maproom.schema_migrations ORDER BY version DESC LIMIT 10;"
```

## SQLite Troubleshooting

### Database Locked

**Symptom**: `database is locked` or `SQLITE_BUSY`

**Cause**: Another process is writing to the database.

**Solutions**:
1. Wait for the other process to finish:
   ```bash
   # Check for running maproom processes
   ps aux | grep maproom
   ```

2. If a process is stuck, kill it:
   ```bash
   # Find and kill stuck process
   pkill -f maproom
   ```

3. Ensure only one indexing process runs at a time.

### Corrupt Database

**Symptom**: `database disk image is malformed` or unexpected errors

**Solutions**:
1. Check database integrity:
   ```bash
   sqlite3 ~/.maproom/maproom.db "PRAGMA integrity_check;"
   ```

2. If corrupt, delete and re-index:
   ```bash
   rm ~/.maproom/maproom.db
   crewchief maproom scan
   ```

### Re-indexing

**When to re-index:**
- After upgrading CrewChief to a new major version
- If search results seem stale or incorrect
- After deleting and recreating the database

**How to re-index:**
```bash
# Remove existing database
rm -rf ~/.maproom/

# Scan fresh
crewchief maproom scan
```

### Disk Space

**Symptom**: `disk I/O error` or database operations fail

**Solutions**:
1. Check available disk space:
   ```bash
   df -h ~/.maproom/
   ```

2. Check database size:
   ```bash
   ls -lh ~/.maproom/maproom.db
   ```

3. If space is low, consider:
   - Removing unused indexed repositories
   - Moving the database to a larger disk
   - Using PostgreSQL for large codebases

---

## PostgreSQL Troubleshooting

### Connection Refused

**Symptom**: `connection refused` or `could not connect to server`

**Solutions**:
1. Verify maproom-postgres is running:
   ```bash
   docker ps | grep maproom-postgres
   ```

2. Start if needed:
   ```bash
   cd config && docker compose up -d
   ```

3. Check logs:
   ```bash
   docker logs maproom-postgres
   ```

4. Verify network connectivity:
   ```bash
   docker network inspect maproom-network
   ```

### Hostname Not Found

**Symptom**: `could not translate host name "maproom-postgres" to address`

**Solutions**:
1. Verify you're in the correct Docker network
2. Check that maproom-postgres container is running
3. Set MAPROOM_DATABASE_URL explicitly as a workaround:
   ```bash
   export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@127.0.0.1:5433/maproom"
   ```

### Authentication Failed

**Symptom**: `FATAL: password authentication failed for user`

**Solutions**:
1. Verify you're using correct credentials:
   - User: `maproom`
   - Password: `maproom`
   - Database: `maproom`

2. Check you're connecting to the right host:
   - Should be `maproom-postgres` not `postgres`

3. Verify MAPROOM_DATABASE_URL format:
   ```bash
   echo $MAPROOM_DATABASE_URL
   # Should be: postgresql://maproom:maproom@maproom-postgres:5432/maproom
   ```

### Schema/Table Missing

**Symptom**: `relation "code_chunks" does not exist`

**Solutions**:
1. Run migrations:
   ```bash
   cargo run --bin maproom -- db migrate
   ```

2. Check if tables exist:
   ```bash
   docker exec maproom-postgres psql -U maproom -d maproom -c "\dt"
   ```

3. Re-index if needed:
   ```bash
   cargo run --bin maproom -- scan /workspace
   ```

### Port Conflicts

**Symptom**: `port 5432 is already allocated` or `port 5433 is already allocated`

**Solution**: Modify docker-compose.yml to use different host port:
```yaml
ports:
  - "5434:5432"  # Changed from 5433:5432
```

Then connect via `localhost:5434` from host machine.

## Database Management

### Backup

**Full backup**:
```bash
docker exec maproom-postgres pg_dump \
  -U maproom \
  -d maproom \
  -F c \
  -f /tmp/maproom_backup.dump

docker cp maproom-postgres:/tmp/maproom_backup.dump ./backups/
```

**Schema only**:
```bash
docker exec maproom-postgres pg_dump \
  -U maproom \
  -d maproom \
  --schema-only \
  -f /tmp/schema.sql

docker cp maproom-postgres:/tmp/schema.sql ./
```

### Restore

```bash
# Copy backup to container
docker cp ./backup.dump maproom-postgres:/tmp/restore.dump

# Restore
docker exec maproom-postgres pg_restore \
  -U maproom \
  -d maproom \
  --clean --if-exists \
  /tmp/restore.dump
```

### Reset Database

⚠️ **CAUTION**: This deletes all indexed data!

```bash
cd /workspace/config
docker compose down -v  # Removes volumes
docker compose up -d

# Wait for initialization, then re-index
cargo run --bin maproom -- scan /workspace
```

### Monitor Size

```bash
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT pg_size_pretty(pg_database_size('maproom'));"
```

## Best Practices

1. **Use Auto-detection**: In most cases, just ensure maproom-postgres is running - the connection fallback handles the rest

2. **Explicit for Production**: Set MAPROOM_DATABASE_URL explicitly in production environments for reliability

3. **Regular Backups**: Back up the database before major changes or schema migrations

4. **Monitor Performance**: Check query performance and adjust postgresql.conf if needed

5. **Version Control Schema**: Use migrations for all schema changes

6. **Test Connections**: After any configuration change, verify connection with a simple query

## Additional Resources

- [Maproom MCP README](../../packages/maproom-mcp/README.md) - MCP setup and usage
- [PostgreSQL Configuration](../../packages/maproom-mcp/config/postgresql.conf) - Performance tuning
- [Docker Compose (MCP)](../../config/docker-compose.yml) - Database stack setup
- [Migrations](../../crates/maproom/migrations/) - Schema evolution

## Summary

### SQLite (Default)

| Aspect | Details |
|--------|---------|
| **File Location** | `~/.maproom/maproom.db` |
| **Setup** | None required |
| **Vector Extension** | sqlite-vec (embedded) |
| **Best For** | Individual use, CI/CD |
| **Connection** | Automatic when no MAPROOM_DATABASE_URL set |

### PostgreSQL (Team/Production)

| Aspect | Details |
|--------|---------|
| **Hostname** | `maproom-postgres` |
| **Network** | `maproom-network` |
| **Port** | 5432 (internal), 5433 (host) |
| **User** | `maproom:maproom` |
| **Database** | `maproom` |
| **Image** | `pgvector/pgvector:pg16` |
| **Best For** | Teams, production, concurrent access |
| **Connection** | `MAPROOM_DATABASE_URL` or auto-detected |

---

**Need Help?** See the troubleshooting sections above or check the [README Quick Start](../../README.md#quick-start-sqlite---recommended).
