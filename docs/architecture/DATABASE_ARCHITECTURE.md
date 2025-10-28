# Database Architecture: Dual PostgreSQL Setup

## Overview

CrewChief uses **two separate PostgreSQL instances** for different purposes. This architecture separates development/integration workflows from the production-like MCP service, providing isolation and flexibility.

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Docker Networks                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌────────────────────────┐         ┌──────────────────────────┐  │
│  │  Devcontainer Network  │         │   Maproom MCP Network    │  │
│  │  (crewchief-network)   │         │  (maproom-network)       │  │
│  │                        │         │                          │  │
│  │  ┌─────────────────┐   │         │  ┌────────────────────┐ │  │
│  │  │   PostgreSQL    │   │         │  │    PostgreSQL      │ │  │
│  │  │   (postgres)    │   │         │  │ (maproom-postgres) │ │  │
│  │  │                 │   │         │  │                    │ │  │
│  │  │ Host: postgres  │   │         │  │ Host: maproom-     │ │  │
│  │  │ Port: 5432      │   │         │  │       postgres     │ │  │
│  │  │ User: postgres  │   │         │  │ Port: 5432         │ │  │
│  │  │ DB: crewchief   │   │         │  │ User: maproom      │ │  │
│  │  │ Image: pg15     │   │         │  │ DB: maproom        │ │  │
│  │  └────────┬────────┘   │         │  │ Image: pg16        │ │  │
│  │           │            │         │  └─────────┬──────────┘ │  │
│  │           │            │         │            │            │  │
│  │  ┌────────▼─────────┐  │         │  ┌─────────▼─────────┐ │  │
│  │  │ CrewChief CLI    │  │         │  │   Maproom MCP     │ │  │
│  │  │                  │  │         │  │      Server       │ │  │
│  │  │ • cargo run      │  │         │  │                   │ │  │
│  │  │ • pnpm dev       │  │         │  │ • npx maproom-mcp │ │  │
│  │  │ • Integration    │  │         │  │ • MCP Tools       │ │  │
│  │  │   Tests          │  │         │  │ • Claude/Cursor   │ │  │
│  │  │ • Maproom Rust   │  │         │  │   Integration     │ │  │
│  │  │   Binary Dev     │  │         │  │                   │ │  │
│  │  └──────────────────┘  │         │  └───────────────────┘ │  │
│  └────────────────────────┘         └──────────────────────────┘  │
│                                                                     │
│  Data: 79,625 chunks                 Data: 23,218 chunks          │
│  Purpose: Development                Purpose: Production-like      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Why Two PostgreSQL Instances?

### Design Rationale

1. **Isolation of Concerns**
   - Development database can be reset/modified without affecting MCP service
   - MCP service database remains stable for AI assistant integrations
   - Different schemas can evolve independently during development

2. **Network Separation**
   - Prevents hostname conflicts on shared Docker networks
   - Each instance has a unique network alias
   - Reduces risk of accidental cross-contamination

3. **Different Use Cases**
   - Devcontainer: Fast iteration, testing, schema changes
   - MCP: Production-like stability, persistent data, external tool access

4. **Performance Optimization**
   - Each instance can be tuned for its specific workload
   - Devcontainer optimized for development speed
   - MCP optimized for vector search and embedding workloads

## PostgreSQL Instance Details

### 1. Devcontainer PostgreSQL

**Purpose**: Local development, CLI testing, integration tests

**When to Use**:
- Running `cargo run --bin crewchief-maproom`
- Developing new Maproom features
- Running integration tests with `cargo test`
- CLI development and debugging
- Schema migrations and testing

**Connection Details**:
```bash
Host:     postgres
Port:     5432
User:     postgres
Password: postgres
Database: crewchief
```

**Connection String**:
```bash
DATABASE_URL=postgresql://postgres:postgres@postgres:5432/crewchief
```

**Configuration**:
- **Image**: `pgvector/pgvector:pg15`
- **Network**: `crewchief-network`
- **Container Name**: `postgres` (generic name for dev convenience)
- **Volume**: `postgres-data` (ephemeral, can be recreated)
- **Init Script**: `.devcontainer/scripts/init-db.sql`

**Data Characteristics**:
- **Current Size**: ~79,625 chunks indexed
- **Lifetime**: Ephemeral - can be reset anytime
- **Schema**: Matches production but allows experimental changes
- **Rebuilding**: `docker compose down -v && docker compose up` (from `.devcontainer/`)

**Environment Setup** (`.devcontainer/devcontainer.json`):
```json
{
  "remoteEnv": {
    "DATABASE_URL": "postgresql://postgres:postgres@postgres:5432/crewchief"
  }
}
```

### 2. Maproom MCP PostgreSQL

**Purpose**: Standalone MCP service, production-like isolated instance

**When to Use**:
- Testing MCP tools with Claude/Cursor
- Running `npx @crewchief/maproom-mcp`
- Production deployments of Maproom MCP
- External AI assistant integrations
- Stable, persistent semantic search index

**Connection Details**:
```bash
Host:     maproom-postgres  # Unique hostname to avoid conflicts
Port:     5432
User:     maproom
Password: maproom
Database: maproom
```

**Connection String**:
```bash
DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom
```

**Configuration**:
- **Image**: `pgvector/pgvector:pg16` (newer version)
- **Network**: `maproom-network`
- **Container Name**: `maproom-postgres` (unique to prevent conflicts)
- **Network Alias**: `maproom-postgres` (ensures correct routing)
- **Volume**: `maproom-data` (persistent, production data)
- **Performance Tuning**: Custom `postgresql.conf` for vector workloads

**Data Characteristics**:
- **Current Size**: ~23,218 chunks indexed
- **Lifetime**: Persistent - production data
- **Schema**: Stable, production-ready
- **Optimization**: Tuned for hybrid search (FTS + vector)

**Docker Compose Locations**:
- **Development**: `packages/maproom-mcp/config/docker-compose.yml`
- **Standalone**: `config/docker-compose.yml`

**Performance Optimizations** (from `postgresql.conf`):
- `shared_buffers=512MB` - Large buffer cache for vector operations
- `effective_cache_size=3GB` - Planner optimization
- `maintenance_work_mem=256MB` - Index creation and VACUUM
- `random_page_cost=1.1` - SSD-optimized
- `max_parallel_workers=4` - Parallel query execution

## Connection String Reference

### Quick Reference Table

| Instance | Host | Port | User | Password | Database | URL |
|----------|------|------|------|----------|----------|-----|
| **Devcontainer** | `postgres` | 5432 | `postgres` | `postgres` | `crewchief` | `postgresql://postgres:postgres@postgres:5432/crewchief` |
| **Maproom MCP** | `maproom-postgres` | 5432 | `maproom` | `maproom` | `maproom` | `postgresql://maproom:maproom@maproom-postgres:5432/maproom` |

### Environment Variables by Context

**Inside Devcontainer**:
```bash
export DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief"
```

**For MCP Service**:
```bash
export DATABASE_URL="postgresql://maproom:maproom@maproom-postgres:5432/maproom"
```

**In Tests** (`packages/maproom-mcp/tests/`):
```bash
export TEST_DATABASE_URL="postgresql://postgres:postgres@postgres:5432/crewchief"
# Tests fall back to DATABASE_URL if TEST_DATABASE_URL not set
```

## When to Use Which Database

### Use Devcontainer PostgreSQL For:

1. **Development**
   - Writing new Maproom features in Rust
   - Testing CLI commands locally
   - Debugging indexing or search issues

2. **Testing**
   - Running `cargo test` integration tests
   - E2E tests with `packages/maproom-mcp/tests/`
   - Schema migration testing

3. **Experimentation**
   - Trying new indexes or query patterns
   - Testing schema changes
   - Benchmarking query performance

4. **Commands**
   ```bash
   # From within devcontainer
   cargo run --bin crewchief-maproom -- scan /workspace
   cargo run --bin crewchief-maproom -- search "authentication"
   cargo test --test integration_tests
   ```

### Use Maproom MCP PostgreSQL For:

1. **MCP Integration**
   - Claude Desktop semantic search
   - Cursor IDE integration
   - Any MCP client connection

2. **Production-like Testing**
   - Testing MCP tools with real AI assistants
   - Validating deployment configurations
   - Performance testing under load

3. **Persistent Indexing**
   - Long-term code index storage
   - Production semantic search service
   - External tool integrations

4. **Commands**
   ```bash
   # From anywhere with npx
   npx @crewchief/maproom-mcp

   # Or with docker-compose
   cd config
   docker compose up -d
   ```

## Data Migration Guide

### Exporting from Devcontainer to MCP

**Scenario**: You've indexed a large codebase in devcontainer and want to move it to MCP database.

```bash
# 1. Export from devcontainer database
docker exec postgres pg_dump \
  -U postgres \
  -d crewchief \
  --clean --if-exists \
  -F c \
  -f /tmp/crewchief_backup.dump

# 2. Copy dump file to host
docker cp postgres:/tmp/crewchief_backup.dump ./backup.dump

# 3. Copy to maproom container
docker cp ./backup.dump maproom-postgres:/tmp/maproom_restore.dump

# 4. Restore to maproom database
docker exec maproom-postgres pg_restore \
  -U maproom \
  -d maproom \
  --clean --if-exists \
  /tmp/maproom_restore.dump
```

### Syncing Schema Changes

**Scenario**: You've made schema changes in devcontainer and need to apply to MCP.

```bash
# 1. Export schema only from devcontainer
docker exec postgres pg_dump \
  -U postgres \
  -d crewchief \
  --schema-only \
  -f /tmp/schema.sql

# 2. Copy and apply to MCP
docker cp postgres:/tmp/schema.sql ./schema.sql
docker cp ./schema.sql maproom-postgres:/tmp/schema.sql
docker exec maproom-postgres psql \
  -U maproom \
  -d maproom \
  -f /tmp/schema.sql
```

### Using SQL Dumps

**Export table data**:
```bash
# Export specific table (e.g., code_chunks)
docker exec postgres pg_dump \
  -U postgres \
  -d crewchief \
  -t code_chunks \
  -f /tmp/chunks.sql
```

**Import table data**:
```bash
docker cp postgres:/tmp/chunks.sql ./chunks.sql
docker cp ./chunks.sql maproom-postgres:/tmp/chunks.sql
docker exec maproom-postgres psql \
  -U maproom \
  -d maproom \
  -f /tmp/chunks.sql
```

### Resetting Databases

**Reset devcontainer database** (safe, for development):
```bash
cd /workspace/.devcontainer
docker compose down -v  # Removes volumes
docker compose up -d
# Wait for health check
docker exec postgres psql -U postgres -d crewchief -c "SELECT version();"
```

**Reset MCP database** (⚠️ CAUTION: Deletes production data):
```bash
cd /workspace/config
docker compose down -v  # Removes volumes including indexed data
docker compose up -d
# Re-index your codebase from scratch
```

## Troubleshooting

### Connection Refused Errors

**Symptom**: `connection refused` or `could not connect to server`

**Diagnosis**:
```bash
# Check if containers are running
docker ps | grep postgres

# Check container logs
docker logs postgres          # Devcontainer
docker logs maproom-postgres  # MCP

# Test connectivity from devcontainer
psql "postgresql://postgres:postgres@postgres:5432/crewchief" -c "SELECT 1;"
psql "postgresql://maproom:maproom@maproom-postgres:5432/maproom" -c "SELECT 1;"
```

**Solution**:
- Ensure correct `DATABASE_URL` for your context
- Verify Docker networks are properly configured
- Check firewall rules if accessing from host

### Authentication Failed Errors

**Symptom**: `FATAL: password authentication failed for user "postgres"`

**Common Causes**:
1. Using wrong credentials for the database instance
2. Hostname resolves to wrong PostgreSQL instance (network conflict)

**Diagnosis**:
```bash
# Check which IPs the hostname resolves to
docker exec devcontainer nslookup postgres
docker exec maproom-mcp nslookup maproom-postgres

# Should see only ONE IP for each hostname
```

**Solution**:
- Use `postgres` hostname for devcontainer database
- Use `maproom-postgres` hostname for MCP database
- Never use generic `localhost` from inside containers

### Schema Mismatch Errors

**Symptom**: `relation "code_chunks" does not exist` or column errors

**Diagnosis**:
```bash
# Check if tables exist in devcontainer
docker exec postgres psql -U postgres -d crewchief -c "\dt"

# Check if tables exist in MCP
docker exec maproom-postgres psql -U maproom -d maproom -c "\dt"

# Compare schemas
docker exec postgres pg_dump -U postgres -d crewchief --schema-only > dev_schema.sql
docker exec maproom-postgres pg_dump -U maproom -d maproom --schema-only > mcp_schema.sql
diff dev_schema.sql mcp_schema.sql
```

**Solution**:
- Run migrations: `crewchief-maproom db migrate`
- Or manually apply schema from devcontainer to MCP (see Migration Guide)

### Data Out of Sync

**Symptom**: Search results differ between instances, or MCP tools show stale data

**Diagnosis**:
```bash
# Count chunks in each database
docker exec postgres psql -U postgres -d crewchief \
  -c "SELECT COUNT(*) FROM code_chunks;"

docker exec maproom-postgres psql -U maproom -d maproom \
  -c "SELECT COUNT(*) FROM code_chunks;"
```

**Solution**:
- Re-index the repository: `crewchief-maproom scan /workspace`
- Or migrate data from devcontainer to MCP (see Migration Guide)

### Port Conflicts

**Symptom**: `port 5432 is already allocated`

**Cause**: Both instances trying to expose port 5432 to host

**Solution**:
- Modify one docker-compose to use different host port:
  ```yaml
  ports:
    - "5433:5432"  # Host:Container
  ```
- Access via `localhost:5433` from host machine

## Best Practices

### Development Workflow

1. **Daily Development**: Use devcontainer PostgreSQL
   - Faster iteration
   - No impact on MCP service
   - Easy to reset if needed

2. **Before Committing**: Test with MCP PostgreSQL
   - Verify MCP tools still work
   - Ensure no breaking changes
   - Run E2E tests with `TEST_DATABASE_URL`

3. **CI/CD**: Use dedicated test database
   - Set `TEST_DATABASE_URL` in CI environment
   - Isolated from both dev and MCP instances
   - Clean state for each test run

### Data Management

1. **Regular Backups**: Back up MCP database regularly
   ```bash
   docker exec maproom-postgres pg_dump \
     -U maproom -d maproom \
     -F c -f /tmp/backup_$(date +%Y%m%d).dump
   docker cp maproom-postgres:/tmp/backup_*.dump ./backups/
   ```

2. **Version Control**: Keep schema in sync
   - Use migrations for schema changes
   - Test migrations on devcontainer first
   - Apply to MCP database after validation

3. **Monitor Sizes**: Check database sizes periodically
   ```bash
   docker exec postgres psql -U postgres -d crewchief \
     -c "SELECT pg_size_pretty(pg_database_size('crewchief'));"
   docker exec maproom-postgres psql -U maproom -d maproom \
     -c "SELECT pg_size_pretty(pg_database_size('maproom'));"
   ```

### Configuration Management

1. **Environment Files**: Use separate `.env` files
   ```
   .devcontainer/.env      → DATABASE_URL for devcontainer
   config/.env             → DATABASE_URL for MCP
   packages/maproom-mcp/.env → DATABASE_URL for MCP package
   ```

2. **Documentation**: Update this doc when changing:
   - Connection strings
   - Docker compose configurations
   - Network names or aliases
   - Performance tuning parameters

3. **Testing**: Always test both instances after changes
   ```bash
   # Test devcontainer
   cargo test --bin crewchief-maproom

   # Test MCP
   cd packages/maproom-mcp && pnpm test
   ```

## Additional Resources

- [Maproom MCP README](../../packages/maproom-mcp/README.md) - MCP setup guide
- [PostgreSQL Configuration](../../packages/maproom-mcp/config/postgresql.conf) - Performance tuning
- [Docker Compose (Dev)](../../.devcontainer/docker-compose.yml) - Devcontainer setup
- [Docker Compose (MCP)](../../config/docker-compose.yml) - MCP stack setup
- [Migrations](../../crates/maproom/migrations/) - Schema evolution

## Summary

| Aspect | Devcontainer PostgreSQL | Maproom MCP PostgreSQL |
|--------|------------------------|------------------------|
| **Purpose** | Development, testing, experimentation | Production-like, MCP service, stable indexing |
| **Hostname** | `postgres` | `maproom-postgres` |
| **Network** | `crewchief-network` | `maproom-network` |
| **User** | `postgres:postgres` | `maproom:maproom` |
| **Database** | `crewchief` | `maproom` |
| **Image** | `pgvector/pgvector:pg15` | `pgvector/pgvector:pg16` |
| **Data Lifetime** | Ephemeral (can reset) | Persistent (production) |
| **Use Cases** | cargo run, tests, CLI dev | MCP tools, Claude/Cursor, npx |
| **Optimization** | Development speed | Vector search performance |
| **Current Size** | ~79,625 chunks | ~23,218 chunks |

---

**Need Help?** See troubleshooting section above or check individual README files in respective component directories.
