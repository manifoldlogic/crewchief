# Database Architecture: maproom-postgres

## Overview

CrewChief uses a **single PostgreSQL instance** (`maproom-postgres`) for all Maproom semantic search operations. This database serves both development workflows and the production MCP service, with intelligent connection fallback for different environments.

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
DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom
```

## Connection Fallback System

Both Rust binary and Node.js CLI use intelligent fallback to automatically detect the database:

### Connection Priority (4-tier fallback)

1. **DATABASE_URL** environment variable (explicit configuration)
   ```bash
   export DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
   ```
   - Highest priority
   - Recommended for production
   - Respects user's explicit choice

2. **MAPROOM_DB_HOST** environment variable (component override)
   ```bash
   export MAPROOM_DB_HOST="custom-postgres"
   export MAPROOM_DB_PORT="5432"  # optional, defaults to 5432
   ```
   - Useful for custom database setups
   - Builds connection string from components
   - Allows flexible configuration

3. **maproom-postgres** hostname resolution (auto-detection)
   - Attempts to resolve `maproom-postgres` hostname
   - Works automatically in Docker environments
   - No configuration needed if container is running
   - Used by default in devcontainer

4. **localhost:5433** (development fallback)
   ```bash
   # Connects to postgresql://maproom:maproom@127.0.0.1:5433/maproom
   ```
   - Final fallback for local testing
   - Useful when running postgres on host machine
   - Non-standard port (5433) to avoid conflicts

### Connection Examples

**Devcontainer** (auto-detection works automatically):
```bash
# No DATABASE_URL needed - auto-detects maproom-postgres
cargo run --bin crewchief-maproom -- scan /workspace

# Or set explicitly
export DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
```

**MCP Service**:
```bash
# Auto-detects when maproom-postgres container is running
npx @crewchief/maproom-mcp

# Or set explicitly in docker-compose
environment:
  - DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom
```

**Custom Setup**:
```bash
# Using component override
export MAPROOM_DB_HOST="my-custom-postgres"
export MAPROOM_DB_PORT="5432"
cargo run --bin crewchief-maproom -- scan /workspace
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

## Troubleshooting

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
3. Set DATABASE_URL explicitly as a workaround:
   ```bash
   export DATABASE_URL="postgresql://maproom:maproom@127.0.0.1:5433/maproom"
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

3. Verify DATABASE_URL format:
   ```bash
   echo $DATABASE_URL
   # Should be: postgresql://maproom:maproom@maproom-postgres:5432/maproom
   ```

### Schema/Table Missing

**Symptom**: `relation "code_chunks" does not exist`

**Solutions**:
1. Run migrations:
   ```bash
   cargo run --bin crewchief-maproom -- db migrate
   ```

2. Check if tables exist:
   ```bash
   docker exec maproom-postgres psql -U maproom -d maproom -c "\dt"
   ```

3. Re-index if needed:
   ```bash
   cargo run --bin crewchief-maproom -- scan /workspace
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
cargo run --bin crewchief-maproom -- scan /workspace
```

### Monitor Size

```bash
docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT pg_size_pretty(pg_database_size('maproom'));"
```

## Best Practices

1. **Use Auto-detection**: In most cases, just ensure maproom-postgres is running - the connection fallback handles the rest

2. **Explicit for Production**: Set DATABASE_URL explicitly in production environments for reliability

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

| Aspect | Details |
|--------|---------|
| **Hostname** | `maproom-postgres` |
| **Network** | `maproom-network` |
| **Port** | 5432 (internal), 5433 (host) |
| **User** | `maproom:maproom` |
| **Database** | `maproom` |
| **Image** | `pgvector/pgvector:pg16` |
| **Data** | Persistent via Docker volumes |
| **Use Cases** | Development, MCP service, testing, production |
| **Connection** | Auto-detected (fallback to DATABASE_URL) |

---

**Need Help?** See the troubleshooting section above or check the [Maproom MCP README](../../packages/maproom-mcp/README.md).
