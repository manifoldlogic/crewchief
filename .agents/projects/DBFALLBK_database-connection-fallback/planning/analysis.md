# Analysis: Database Connection Fallback

## Problem Space

### Current State

CrewChief's Maproom component uses two separate PostgreSQL databases:

1. **Devcontainer PostgreSQL** (`postgres:5432/crewchief`)
   - Used for local development and testing
   - Set via `DATABASE_URL` in docker-compose.yml
   - Contains ephemeral development data (~79,625 chunks)

2. **Maproom MCP PostgreSQL** (`maproom-postgres:5432/maproom`)
   - Used for production-like MCP service
   - Persistent data for semantic search (~23,218 chunks)
   - Separate network for isolation

The Rust binary (`crewchief-maproom`) and Node.js CLI (`cli.cjs`) currently have **inconsistent database connection behavior**:

**Node.js CLI**:
- **Always overrides** `DATABASE_URL` environment variable
- Uses auto-detection logic in `getDatabaseConnectionString()`
- Priority: `MAPROOM_DB_HOST` → `maproom-postgres` hostname check → `localhost:5433`
- Ignores explicit `DATABASE_URL` even when set

**Rust Binary**:
- **Requires** `DATABASE_URL` environment variable
- No fallback logic - fails with error if not set
- Used directly by developers via `cargo run`
- Used indirectly via Node.js CLI (which sets `DATABASE_URL`)

### Pain Points

#### Pain Point 1: Devcontainer DATABASE_URL Ignored
When developers run the Node.js CLI inside the devcontainer:
- docker-compose.yml sets: `DATABASE_URL=postgresql://postgres:postgres@postgres:5432/crewchief`
- Developer expects to use development database
- **But CLI overrides it** with `maproom-postgres` connection
- Developer unknowingly uses MCP database instead of dev database
- This causes confusion when development changes don't appear in expected database

#### Pain Point 2: Rust Binary Requires Explicit DATABASE_URL
When developers run the Rust binary directly:
```bash
cargo run --bin crewchief-maproom -- scan --path /workspace
```
- Requires `DATABASE_URL` to be set
- No auto-detection of `maproom-postgres`
- Fails with error: "DATABASE_URL env var is required"
- Forces developers to remember to set env var manually

#### Pain Point 3: Inconsistent Behavior
The two components behave differently:
- CLI: Auto-detects maproom-postgres (but ignores explicit DATABASE_URL)
- Binary: Requires explicit DATABASE_URL (no auto-detection)
- Developers must understand which they're using to predict behavior

### User Expectations

**Typical User** (using MCP):
- Runs: `npx @crewchief/maproom-mcp scan /workspace`
- Expects: Auto-detection to work (use maproom-postgres)
- Current: ✅ Works via CLI

**Developer** (in devcontainer):
- Runs: `cargo run --bin crewchief-maproom -- scan --path /workspace`
- Expects: Use devcontainer DATABASE_URL
- Current: ✅ Works (reads DATABASE_URL from docker-compose.yml)

**Developer** (in devcontainer, using CLI):
- Runs: `node /workspace/packages/maproom-mcp/bin/cli.cjs scan /workspace`
- Expects: Use devcontainer DATABASE_URL
- Current: ❌ **Broken** - CLI overrides it with maproom-postgres

**Standalone User** (no DATABASE_URL set):
- Runs: `./crewchief-maproom scan --path /workspace`
- Expects: Auto-detection to work (use maproom-postgres if available)
- Current: ❌ **Broken** - binary fails without DATABASE_URL

## Industry Solutions

### Connection String Priority Patterns

Most database tools use a consistent priority hierarchy:

**1. Explicit Configuration First** (PostgreSQL, MySQL clients)
- Respect explicitly set environment variables
- Only fall back to auto-detection when not set
- Example: `psql` checks `PGHOST`, `PGPORT`, `PGDATABASE` before defaults

**2. Fallback to Sensible Defaults** (Docker, Kubernetes)
- Auto-detect container hostnames via DNS
- Fall back to localhost
- Example: Docker Compose service names resolve via internal DNS

**3. Environment Variable Precedence** (12-Factor App)
- Explicit env vars > config files > defaults
- Example: `DATABASE_URL` > `database.yml` > hardcoded defaults

### Common Connection Fallback Patterns

#### Pattern A: Tiered Fallback (Redis, MongoDB)
```
1. Explicit connection string (DATABASE_URL)
2. Individual components (HOST, PORT, DB, USER, PASS)
3. Service discovery (DNS, mDNS, Consul)
4. Localhost defaults
```

#### Pattern B: Smart Detection (Docker Compose)
```
1. Check if running in container (/.dockerenv exists)
2. Try container hostname resolution
3. Fall back to localhost
```

#### Pattern C: Configuration Hierarchy (Rails, Django)
```
1. Environment variables (highest)
2. Environment-specific config (development.yml, production.yml)
3. Default config (config/database.yml)
4. Framework defaults (sqlite for dev, error for prod)
```

### Best Practices from Research

1. **Always Respect Explicit Config**: If user sets `DATABASE_URL`, use it. Don't override.

2. **Fail Fast in Production**: Auto-detection is great for development, but production should require explicit config.

3. **Consistent Behavior**: All components should use same connection logic.

4. **Clear Error Messages**: When auto-detection fails, explain what was tried and why it failed.

5. **Logging**: Log which connection method was used for debugging.

## Current Codebase State

### Node.js CLI Implementation

File: `/workspace/packages/maproom-mcp/bin/cli.cjs`

**getDatabaseConnectionString() function** (lines 104-127):
```javascript
function getDatabaseConnectionString() {
  // Check if MAPROOM_DB_HOST environment variable is set (allows override)
  if (process.env.MAPROOM_DB_HOST) {
    return `postgresql://maproom:maproom@${process.env.MAPROOM_DB_HOST}:${process.env.MAPROOM_DB_PORT || 5432}/maproom`;
  }

  // Try to detect if we're in a Docker/devcontainer environment
  // by checking if we can resolve the maproom-postgres hostname
  try {
    const { execSync } = require('child_process');
    // Quick DNS check for maproom-postgres hostname
    execSync('getent hosts maproom-postgres 2>/dev/null || ping -c 1 -W 1 maproom-postgres 2>/dev/null', {
      stdio: 'pipe',
      timeout: 1000
    });
    // If we got here, maproom-postgres hostname resolves
    diagnosticLog('Using container hostname for database connection');
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
  } catch (error) {
    // Hostname doesn't resolve, use localhost
    diagnosticLog('Using localhost for database connection');
    return 'postgresql://maproom:maproom@127.0.0.1:5433/maproom';
  }
}
```

**Usage in scan command** (line 1524):
```javascript
const env = {
  ...process.env,
  DATABASE_URL: getDatabaseConnectionString(),  // Always overrides!
  ...providerEnv
};
```

**Problem**: Unconditionally sets `DATABASE_URL`, ignoring any existing value in `process.env.DATABASE_URL`.

### Rust Binary Implementation

File: `/workspace/crates/maproom/src/db/pool.rs`

**create_pool() function** (lines 118-120):
```rust
pub async fn create_pool() -> anyhow::Result<PgPool> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL env var is required (tip: use a .env file)")?;
    // ... rest of connection logic
}
```

**Problem**: No fallback logic - immediately fails if `DATABASE_URL` not set.

File: `/workspace/crates/maproom/src/db/queries.rs`

**connect() function** (lines 5-8):
```rust
pub async fn connect() -> anyhow::Result<Client> {
    let database_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL env var is required (tip: use a .env file)")?;
    // ... rest of connection logic
}
```

**Problem**: Same issue - no fallback.

### Unused Environment Variables

File: `/workspace/.devcontainer/docker-compose.yml`

**Lines 25-29**:
```yaml
environment:
  - CREWCHIEF_DB_HOST=postgres
  - CREWCHIEF_DB_PORT=5432
  - CREWCHIEF_DB_NAME=crewchief
  - CREWCHIEF_DB_USER=postgres
  - CREWCHIEF_DB_PASSWORD=postgres
```

**Analysis**: These variables are completely unused. Likely added for future use but never referenced in code. Should be removed or repurposed.

## Technical Research

### DNS Resolution in Containers

Docker Compose and similar container orchestrators provide automatic DNS resolution for service names:

- Service `maproom-postgres` resolves to container IP on the shared network
- Resolution happens via Docker's embedded DNS server (127.0.0.11)
- Works across containers on same network (`maproom-network`, `crewchief-network`)
- Fails outside containers (host machine can't resolve these names)

**Testing Method**: `getent hosts <hostname>` or `ping -c 1 -W 1 <hostname>`

### PostgreSQL Connection String Format

Standard libpq format:
```
postgresql://[user[:password]@][host][:port][/dbname][?param=value]
```

Components can also be set via individual env vars:
- `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, `PGPASSWORD`

But `DATABASE_URL` is the de facto standard for modern apps (Rails, Django, Node.js ORMs).

### Rust Environment Variable Handling

Rust's `std::env::var()` only checks environment at runtime, doesn't look for fallbacks.

Standard pattern for fallbacks:
```rust
let value = std::env::var("VAR_NAME")
    .unwrap_or_else(|_| "default_value".to_string());
```

Or with optional values:
```rust
let value = std::env::var("VAR_NAME").ok();  // Returns Option<String>
```

### Error Context Considerations

Rust's `anyhow::Context` adds helpful error messages:
```rust
.context("DATABASE_URL env var is required (tip: use a .env file)")?;
```

With fallback logic, we should:
1. Log which method was used (DEBUG level)
2. Only error if all fallback attempts fail
3. Explain what was tried in error message

## Conclusion

The core problem is **inconsistent connection logic** between Node.js CLI (which overrides DATABASE_URL) and Rust binary (which requires it). The solution needs to:

1. Make both components use identical fallback logic
2. Respect explicit `DATABASE_URL` when set (don't override)
3. Provide sensible defaults (maproom-postgres) when not set
4. Maintain backward compatibility with existing usage patterns
5. Fail with clear error messages when all fallbacks exhausted

This aligns with industry best practices: explicit config first, then smart defaults.
