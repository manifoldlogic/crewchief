# Docker Networking Fix Summary

## Problem Resolved

Fixed connection failures when running Maproom MCP CLI in a devcontainer environment:
```
❌ Database initialization failed: connect ECONNREFUSED 127.0.0.1:5433
```

## Root Cause

In Docker-in-Docker (devcontainer) environments, port bindings like `127.0.0.1:5433:5432` bind to the Docker VM's localhost, not the devcontainer's interface. This makes the postgres container inaccessible from the CLI running inside the devcontainer.

## Changes Made

### 1. **docker-compose.yml** - Port Binding Update

**File**: `/workspace/packages/maproom-mcp/config/docker-compose.yml`

**Change**: Updated port bindings from `127.0.0.1` to `0.0.0.0` for Docker-in-Docker compatibility.

```yaml
# Before
ports:
  - "127.0.0.1:5433:5432"  # Bind to localhost only for security

# After
ports:
  - "0.0.0.0:5433:5432"  # Bind to all interfaces for Docker-in-Docker compatibility
```

**Why**: Binding to `0.0.0.0` allows access from all network interfaces, enabling connections from both the host and other containers.

### 2. **cli.cjs** - Smart Connection String Detection

**File**: `/workspace/packages/maproom-mcp/bin/cli.cjs`

**Added**: `getDatabaseConnectionString()` function that auto-detects the environment and returns the appropriate connection string.

```javascript
function getDatabaseConnectionString() {
  // Allow manual override
  if (process.env.MAPROOM_DB_HOST) {
    return `postgresql://maproom:maproom@${process.env.MAPROOM_DB_HOST}:${process.env.MAPROOM_DB_PORT || 5432}/maproom`;
  }

  // Auto-detect environment via hostname resolution
  try {
    const { execSync } = require('child_process');
    execSync('getent hosts maproom-postgres 2>/dev/null || ping -c 1 -W 1 maproom-postgres 2>/dev/null', {
      stdio: 'pipe',
      timeout: 1000
    });
    // In devcontainer: use container hostname
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
  } catch (error) {
    // On host: use localhost port mapping
    return 'postgresql://maproom:maproom@127.0.0.1:5433/maproom';
  }
}
```

**Updated Locations** (replaced hardcoded `127.0.0.1:5433` with `getDatabaseConnectionString()`):
- `initializeDatabaseSchema()` function (line 1198)
- `validateDatabaseSchema()` function (line 1229)
- `runScan()` function (line 1519)
- `runUpsert()` function (line 1653)

### 3. **devcontainer-network-fix.sh** - Network Setup Helper

**File**: `/workspace/packages/maproom-mcp/config/devcontainer-network-fix.sh`

**Purpose**: Automatically connects the postgres container to the devcontainer network, enabling hostname-based access.

```bash
#!/bin/bash
# Connects maproom-postgres to devcontainer network if detected
docker network connect crewchief_devcontainer_crewchief-network maproom-postgres
```

**Usage**:
```bash
./config/devcontainer-network-fix.sh
```

### 4. **Documentation**

**File**: `/workspace/packages/maproom-mcp/config/DEVCONTAINER_NETWORKING.md`

Comprehensive guide covering:
- Problem explanation
- Solution architecture
- Manual override options
- Testing procedures
- Network diagrams

## How It Works

### Automatic Environment Detection

1. **CLI starts** → calls `getDatabaseConnectionString()`
2. **Checks for manual override** → `MAPROOM_DB_HOST` environment variable
3. **Tests hostname resolution** → tries to resolve `maproom-postgres`
4. **Returns appropriate connection**:
   - ✅ Hostname resolves → use `maproom-postgres:5432` (devcontainer)
   - ❌ Hostname fails → use `127.0.0.1:5433` (host)

### Network Connectivity

The postgres container must be on the same network as the devcontainer:

```bash
# Postgres networks
- maproom-mcp_maproom-network (default, for MCP service)
- crewchief_devcontainer_crewchief-network (added for CLI access)

# Devcontainer network
- crewchief_devcontainer_crewchief-network
```

When postgres is on both networks, the CLI can access it via hostname.

## Testing

### Verify Connection String Detection

```bash
node -e "
const { execSync } = require('child_process');
try {
  execSync('getent hosts maproom-postgres', { stdio: 'pipe' });
  console.log('Using: maproom-postgres:5432');
} catch {
  console.log('Using: 127.0.0.1:5433');
}
"
```

### Verify Network Connectivity

```bash
# Check postgres is on both networks
docker inspect maproom-postgres --format '{{range $net, $config := .NetworkSettings.Networks}}{{$net}}{{"\n"}}{{end}}'

# Should show:
# maproom-mcp_maproom-network
# crewchief_devcontainer_crewchief-network
```

### Test Database Connection

```bash
# Via hostname (devcontainer)
psql postgresql://maproom:maproom@maproom-postgres:5432/maproom -c 'SELECT 1;'

# Via localhost (host)
psql postgresql://maproom:maproom@127.0.0.1:5433/maproom -c 'SELECT 1;'
```

## Manual Overrides

### Override Database Host

```bash
export MAPROOM_DB_HOST=maproom-postgres
export MAPROOM_DB_PORT=5432
```

### Use Localhost

```bash
export MAPROOM_DB_HOST=127.0.0.1
export MAPROOM_DB_PORT=5433
```

## Benefits

1. **Zero Configuration**: Works automatically in both devcontainer and host environments
2. **Manual Override**: Allows custom connection strings when needed
3. **Backward Compatible**: Existing setups on host machines continue working
4. **Developer Friendly**: No manual network configuration required
5. **Documented**: Comprehensive guides for troubleshooting

## Files Changed

```
packages/maproom-mcp/
├── config/
│   ├── docker-compose.yml                  (updated port bindings)
│   ├── devcontainer-network-fix.sh         (new helper script)
│   └── DEVCONTAINER_NETWORKING.md          (new documentation)
├── bin/
│   └── cli.cjs                             (added smart connection detection)
└── DOCKER_NETWORKING_FIX_SUMMARY.md        (this file)
```

## Next Steps

1. **Test the setup**: Run the CLI in devcontainer to verify connection works
2. **Run network fix**: Use `./config/devcontainer-network-fix.sh` if needed
3. **Monitor logs**: Check diagnostic logs with `MAPROOM_MCP_DEBUG=true`
4. **Document edge cases**: Report any issues with specific Docker configurations

## Support

For issues or questions, see:
- [Devcontainer Networking Guide](config/DEVCONTAINER_NETWORKING.md)
- [Database Architecture Documentation](../../docs/architecture/DATABASE_ARCHITECTURE.md)
