# Devcontainer Networking Guide

This document explains Docker networking issues when running Maproom MCP in a devcontainer environment and how they're resolved.

## Problem

When running the Maproom MCP postgres container in a Docker-in-Docker (DinD) devcontainer environment, port bindings like `127.0.0.1:5433:5432` don't expose ports to the devcontainer itself. This causes connection failures when the CLI tries to connect to `127.0.0.1:5433`.

### Error Symptoms

```
❌ Database initialization failed: connect ECONNREFUSED 127.0.0.1:5433
```

### Root Cause

In a Docker Desktop + devcontainer setup:
1. The postgres container runs with port binding `127.0.0.1:5433:5432`
2. This binding is on the **Docker VM's** localhost interface, not the devcontainer's
3. The CLI runs **inside** the devcontainer (which is itself a container)
4. Therefore, `127.0.0.1:5433` in the devcontainer doesn't reach the postgres container

## Solution

### Automatic Detection (Implemented)

The CLI now automatically detects whether it's running in a Docker environment and adjusts the connection string:

- **In devcontainer**: Uses `maproom-postgres:5432` (container hostname)
- **On host machine**: Uses `127.0.0.1:5433` (localhost port mapping)

This is handled by the `getDatabaseConnectionString()` function in `bin/cli.cjs`.

### Manual Override

You can override the database connection by setting environment variables:

```bash
export MAPROOM_DB_HOST=maproom-postgres
export MAPROOM_DB_PORT=5432
```

Or for localhost:

```bash
export MAPROOM_DB_HOST=127.0.0.1
export MAPROOM_DB_PORT=5433
```

### Devcontainer Network Connection

The postgres container needs to be on the same Docker network as the devcontainer for hostname resolution to work.

#### Automatic Connection (Helper Script)

Run the provided script to connect postgres to the devcontainer network:

```bash
./config/devcontainer-network-fix.sh
```

This script:
1. Detects if running in a devcontainer
2. Connects the `maproom-postgres` container to `crewchief_devcontainer_crewchief-network`
3. Enables hostname-based access from within the devcontainer

#### Manual Connection

If you need to manually connect postgres to the devcontainer network:

```bash
# Find your devcontainer network name
docker network ls | grep devcontainer

# Connect postgres to that network
docker network connect crewchief_devcontainer_crewchief-network maproom-postgres
```

## Port Binding: 0.0.0.0 vs 127.0.0.1

The docker-compose.yml uses `0.0.0.0:5433:5432` for maximum compatibility:

- `0.0.0.0:5433:5432` - Binds to all network interfaces (works in more scenarios)
- `127.0.0.1:5433:5432` - Binds to localhost only (more secure but fails in DinD)

For production deployments outside devcontainers, you may want to change back to `127.0.0.1` for security.

## Testing Connection

### Test Hostname Resolution

```bash
# Check if maproom-postgres hostname resolves
getent hosts maproom-postgres
# or
ping -c 1 maproom-postgres
```

### Test TCP Connection

```bash
# Try to connect to postgres via hostname
timeout 2 bash -c 'cat < /dev/null > /dev/tcp/maproom-postgres/5432' && echo "✓ Connected"

# Or via localhost
timeout 2 bash -c 'cat < /dev/null > /dev/tcp/127.0.0.1/5433' && echo "✓ Connected"
```

### Test PostgreSQL Connection

```bash
# Install pg client if not available
npm install -g pg

# Test connection via hostname
psql postgresql://maproom:maproom@maproom-postgres:5432/maproom -c 'SELECT 1;'

# Or via localhost
psql postgresql://maproom:maproom@127.0.0.1:5433/maproom -c 'SELECT 1;'
```

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ Docker Desktop VM                                           │
│                                                             │
│  ┌─────────────────────────────┐                           │
│  │ Devcontainer                │                           │
│  │ (crewchief_devcontainer_    │                           │
│  │  crewchief-network)         │                           │
│  │                             │                           │
│  │  ┌─────────────────────┐    │                           │
│  │  │ Maproom CLI         │────┼─────┐                     │
│  │  │ (bin/cli.cjs)       │    │     │                     │
│  │  └─────────────────────┘    │     │                     │
│  │                             │     │ Container hostname  │
│  └─────────────────────────────┘     │ maproom-postgres    │
│                                      │ :5432               │
│  ┌─────────────────────────────┐     │                     │
│  │ maproom-postgres            │◄────┘                     │
│  │ (maproom-mcp_maproom-       │                           │
│  │  network)                   │                           │
│  │                             │                           │
│  │ Ports: 0.0.0.0:5433->5432  │                           │
│  └─────────────────────────────┘                           │
│         │                                                   │
│         │ Port mapping (doesn't work for DinD)            │
│         ▼                                                   │
│    127.0.0.1:5433                                          │
│    (Docker VM localhost,                                   │
│     not accessible from                                    │
│     inside devcontainer)                                   │
└─────────────────────────────────────────────────────────────┘
```

## Summary

The Maproom MCP CLI now automatically handles both devcontainer and host environments by:

1. **Auto-detecting** the environment via hostname resolution
2. **Using container hostname** (`maproom-postgres:5432`) when in devcontainer
3. **Using localhost port mapping** (`127.0.0.1:5433`) when on host
4. **Allowing manual override** via `MAPROOM_DB_HOST` environment variable

The helper script `devcontainer-network-fix.sh` ensures the postgres container is on the correct network when running in a devcontainer.
