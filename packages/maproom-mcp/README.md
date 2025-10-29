# @crewchief/maproom-mcp

Maproom MCP server with local LLM embeddings - **zero configuration required**.

Add one line to your `.mcp.json` and get semantic code search powered by local AI. No API keys, no cloud services, no complex setup.

## Features

✨ **Zero Configuration** - Works out of the box with Docker
🔒 **100% Local** - No API keys, no cloud dependencies, complete privacy
🚀 **Fast Hybrid Search** - Vector similarity + full-text search with PostgreSQL
🤖 **Local LLM** - Ollama with nomic-embed-text (768-dimensional embeddings)
🔄 **Auto-Embeddings** - Embeddings generated automatically during indexing
📦 **Fully Containerized** - Everything runs in Docker, isolated and clean
🌳 **Multi-Language** - Tree-sitter parsing for TypeScript, JavaScript, Rust, and more

## Quick Start

Add this to your `.mcp.json` configuration file:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

**That's it!** No other configuration needed.

### Where to find `.mcp.json`:

- **Claude Desktop (macOS)**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Claude Desktop (Windows)**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Cursor**: `.cursor/mcp.json` in your project root

## System Requirements

- **Docker Desktop 4.x+** ([Install Docker](https://docs.docker.com/get-docker/))
- **4-8 GB RAM** available for Docker
- **5 GB disk space** (images + model + database)
- **Supported OS**: macOS, Linux, Windows with WSL2

Verify Docker is running:
```bash
docker --version
docker compose version
```

## What to Expect

### First Run (2-5 minutes)
The first time you use Maproom, it will:
1. Download Docker images (~1.5 GB compressed)
2. Download the nomic-embed-text model (~275 MB)
3. Initialize PostgreSQL database with pgvector
4. Start all three services (postgres, ollama, maproom-mcp)
5. **Auto-generate embeddings** during first scan for instant semantic search

Progress indicators will show each step. This happens once.

### Subsequent Runs (10-20 seconds)
After the first run, startup is fast:
- Images and model are cached
- Services start from Docker cache
- Database persists between sessions

## Environment Variables (Optional)

All configuration is optional! Customize behavior only if needed:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "EMBEDDING_PROVIDER": "google",
        "LOG_LEVEL": "debug"
      }
    }
  }
}
```

Available variables:
- `EMBEDDING_PROVIDER` - Choose provider: `ollama` (default), `openai`, or `google`
- `LOG_LEVEL` - Logging verbosity: `error`, `warn`, `info`, `debug` (default: `info`)
- `EMBEDDING_MODEL` - Override embedding model (provider-specific)
- `EMBEDDING_DIMENSION` - Vector dimensions (provider-specific default)
- `EMBEDDING_BATCH_SIZE` - Batch size for embedding generation (default: `50`)

**Advanced: Custom Database**

By default, Maproom connects to `maproom-postgres:5432`. To use a custom database:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "DATABASE_URL": "postgresql://user:pass@custom-host:5432/mydb"
      }
    }
  }
}
```

## Data Persistence

All data is stored in Docker volumes:
- `maproom-data` - PostgreSQL database (indexed code + embeddings)
- `ollama-models` - Downloaded Ollama models (~275 MB)
- `maproom-logs` - MCP server logs
- `maproom-init-sql` - Database initialization script

Your indexed code and embeddings persist between sessions. To completely reset:

```bash
docker volume rm maproom-data ollama-models maproom-logs maproom-init-sql
```

## Container Management

The Maproom stack uses Docker containers with automatic restart policies. Understanding how to manage these containers will help you troubleshoot effectively.

### Service Status

Check if services are running:
```bash
docker compose -f ~/.maproom-mcp/docker-compose.yml ps
```

### Viewing Logs

View logs without disrupting the MCP connection:
```bash
# All services
docker compose -f ~/.maproom-mcp/docker-compose.yml logs -f

# Specific service
docker compose -f ~/.maproom-mcp/docker-compose.yml logs -f postgres
docker compose -f ~/.maproom-mcp/docker-compose.yml logs -f ollama
```

### Stopping Services

When you need to stop the services completely:
```bash
docker compose -f ~/.maproom-mcp/docker-compose.yml down
```

**Note**: Stopping services will break the MCP connection. You'll need to reload your MCP configuration in Claude/Cursor after restarting.

### Restarting Services (Advanced)

If you must restart a container, understand the consequences:

```bash
# Restart a specific container (BREAKS MCP CONNECTION)
docker restart maproom-postgres
docker restart maproom-ollama

# Restart all services (BREAKS MCP CONNECTION)
docker compose -f ~/.maproom-mcp/docker-compose.yml restart
```

**After restart**: You MUST reload your MCP configuration in Claude Desktop or Cursor (see "Connection lost after container restart" in Troubleshooting).

### Health Checks

All services have health checks that run automatically:
- **PostgreSQL**: Checked every 10 seconds (ready when accepting connections)
- **Ollama**: Checked every 30 seconds (ready when model is loaded)
- **Automatic Recovery**: Services with `restart: unless-stopped` policy will automatically restart if they fail

To verify health status:
```bash
docker inspect maproom-postgres --format='{{.State.Health.Status}}'
docker inspect maproom-ollama --format='{{.State.Health.Status}}'
```

## Security

### Automated Vulnerability Scanning

The Maproom MCP package includes automated security checks to prevent publishing packages with known vulnerabilities:

**Pre-Publish Audit**: Before every `npm publish` or `pnpm publish`, the package automatically runs:
```bash
pnpm audit --audit-level=high --prod
```

This checks **production dependencies only** for high or critical severity vulnerabilities. If any are found, the publish process is blocked automatically.

**Why production only?** Development dependencies (like test frameworks) don't ship to end users, so their vulnerabilities don't affect package consumers.

### Manual Security Checks

Run security audits manually during development:

```bash
# Check for moderate+ vulnerabilities (more comprehensive)
pnpm security-check

# Or run audit directly with custom levels
pnpm audit --audit-level=moderate
pnpm audit --audit-level=low
```

### Fixing Vulnerabilities

When vulnerabilities are detected:

1. **Automatic fixes** (try this first):
   ```bash
   pnpm audit fix
   ```
   This updates dependencies to patched versions automatically.

2. **Manual updates** (if automatic fixes don't work):
   ```bash
   # Update specific vulnerable package
   pnpm update <package-name>

   # Or update all dependencies to latest compatible versions
   pnpm update
   ```

3. **Check for breaking changes**:
   ```bash
   # Review what changed
   git diff package.json

   # Test the package still works
   pnpm build
   pnpm test
   ```

### Emergency Override (Use with Extreme Caution)

If you must publish despite vulnerabilities (e.g., false positive, no fix available):

```bash
# Temporarily remove security check from prepublishOnly
# Edit package.json to remove "pnpm audit --audit-level=high --prod"
# Then publish
pnpm publish

# IMMEDIATELY restore the security check after publishing
git checkout package.json
```

**WARNING**: Only override security checks when:
- You've verified the vulnerability is a false positive
- The vulnerability is in a transitive dependency with no fix available
- You've documented the risk and mitigation strategy
- You've filed an issue to track fixing the vulnerability

Never publish with known high/critical vulnerabilities in direct dependencies.

### Supply Chain Security Best Practices

1. **Keep dependencies updated**: Run `pnpm update` regularly
2. **Review dependency changes**: Check `pnpm audit` output before updates
3. **Minimize dependencies**: Fewer dependencies = smaller attack surface
4. **Pin versions**: Use exact versions in package.json for production packages
5. **Monitor security advisories**: Subscribe to GitHub security alerts for this repository

## Security Considerations

### Credentials Management

⚠️ **Never commit credentials to version control**

Maproom uses environment variables and `.env` files for sensitive configuration. Follow these best practices:

**Use `.env` files** (git-ignored by default):
```bash
# .env file (automatically excluded from git)
DATABASE_URL=postgresql://maproom:secure-password@maproom-postgres:5432/maproom
OPENAI_API_KEY=sk-your-api-key-here
GOOGLE_API_KEY=your-google-api-key
```

**Security checklist**:
- ✅ **Rotate credentials regularly** - Change database passwords and API keys periodically
- ✅ **Use unique credentials per environment** - Development, staging, and production should have different credentials
- ✅ **Use strong passwords** - Generate random passwords with tools like `openssl rand -base64 32`
- ✅ **Consider secret management tools** - For production deployments, use HashiCorp Vault, AWS Secrets Manager, or similar
- ✅ **Revoke unused credentials** - Remove API keys and passwords when no longer needed

**Example: Rotating database password**:
```bash
# 1. Generate new password
NEW_PASSWORD=$(openssl rand -base64 24)

# 2. Update docker-compose.yml environment variable
# POSTGRES_PASSWORD=${NEW_PASSWORD}

# 3. Update DATABASE_URL in .env or MCP config
# DATABASE_URL=postgresql://maproom:${NEW_PASSWORD}@maproom-postgres:5432/maproom

# 4. Restart services with new password
docker compose -f ~/.maproom-mcp/docker-compose.yml down
docker compose -f ~/.maproom-mcp/docker-compose.yml up -d
```

### Network Security

🔒 **Services are bound to localhost (127.0.0.1) by default**

Maproom services are configured to only accept connections from the local machine, preventing unauthorized network access.

**Default configuration** (secure):
```yaml
services:
  postgres:
    ports:
      - "127.0.0.1:5432:5432"  # Only accessible from localhost
  ollama:
    ports:
      - "127.0.0.1:11434:11434"  # Only accessible from localhost
```

**Exposing services to the network** (⚠️ use with caution):

If you need to access Maproom services from other machines (e.g., remote development, team collaboration), understand the risks:

```yaml
services:
  postgres:
    ports:
      - "5432:5432"  # ⚠️ WARNING: Exposed to all network interfaces
```

**⚠️ Security implications**:
- Database will be accessible from other machines on your network
- Weak passwords can lead to unauthorized access
- Sensitive code and embeddings could be exposed

**Safer alternatives for remote access**:

1. **SSH Tunneling** (recommended):
   ```bash
   # On remote machine, create tunnel to local Maproom
   ssh -L 5432:localhost:5432 user@maproom-host
   ssh -L 11434:localhost:11434 user@maproom-host

   # Now connect to localhost:5432 as if Maproom was local
   ```

2. **VPN**: Use a VPN to securely access the network where Maproom is running

3. **Firewall Rules**: If you must expose services, use firewall rules to restrict access:
   ```bash
   # Linux (iptables) - only allow specific IP
   sudo iptables -A INPUT -p tcp --dport 5432 -s 192.168.1.100 -j ACCEPT
   sudo iptables -A INPUT -p tcp --dport 5432 -j DROP

   # macOS (pfctl) - restrict to local network
   # Add to /etc/pf.conf:
   # block in proto tcp from any to any port 5432
   # pass in proto tcp from 192.168.1.0/24 to any port 5432
   ```

**Container networking**:
- All Maproom services communicate via the `maproom-network` Docker network
- This network is isolated from other Docker networks by default
- Cross-container communication uses internal hostnames (`maproom-postgres`, `maproom-ollama`)

### Diagnostic Logging

🔍 **Sensitive values are redacted in diagnostic logs**

Maproom automatically redacts sensitive information from logs to prevent credential leakage.

**What gets redacted**:
- Database passwords in connection strings
- API keys (OpenAI, Google, etc.)
- OAuth tokens
- Authentication headers
- Secret environment variables

**Example redacted log output**:
```
[INFO] Connecting to database: postgresql://maproom:***REDACTED***@maproom-postgres:5432/maproom
[INFO] Embedding provider: openai (API key: ***REDACTED***)
[DEBUG] Environment: DATABASE_URL=postgresql://maproom:***REDACTED***@...
```

**Safe log sharing**:

When reporting issues, you can safely share diagnostic logs:

```bash
# Collect logs for bug reports
docker compose -f ~/.maproom-mcp/docker-compose.yml logs > maproom-debug.log

# Verify sensitive data is redacted before sharing
grep -i "password\|api_key\|token" maproom-debug.log
# Should show: ***REDACTED*** for all sensitive values
```

**⚠️ What is NOT redacted**:
- Repository names and file paths (may contain sensitive project names)
- Code content in search results (your actual source code)
- Database host and port numbers
- Embedding model names

**Before sharing logs publicly**:
1. Review for sensitive project names or file paths
2. Remove any internal hostnames or IP addresses
3. Verify all passwords/tokens show as `***REDACTED***`
4. Consider sharing only relevant excerpts instead of full logs

### Security Reporting

🛡️ **Found a security vulnerability?**

We take security seriously and appreciate responsible disclosure.

**Contact**: security@crewchief.dev

**What to include in your report**:
1. **Description** - Clear description of the vulnerability
2. **Impact** - What an attacker could do (data exposure, code execution, etc.)
3. **Reproduction steps** - Detailed steps to reproduce the issue
4. **Environment** - OS, Docker version, Maproom version
5. **Suggested fix** (optional) - If you have ideas for mitigation

**Example security report**:
```
Subject: [SECURITY] Credential leak in error messages

Description:
Database passwords appear in plaintext in error messages when
connection fails with invalid credentials.

Impact:
An attacker with access to error logs could extract database passwords.

Reproduction:
1. Configure invalid DATABASE_URL with password
2. Start Maproom MCP
3. Check logs - password appears in plaintext

Environment:
- macOS 13.5
- Docker Desktop 4.25.0
- Maproom MCP v1.0.0

Suggested Fix:
Redact credentials in error messages using regex replacement.
```

**What to expect**:
- **Acknowledgment**: Within 48 hours
- **Assessment**: Initial assessment within 1 week
- **Fix timeline**: Critical issues patched within 2 weeks, others in next release
- **Credit**: We'll credit you in release notes (unless you prefer anonymity)

**Disclosure policy**:
- Please allow 90 days for us to fix the issue before public disclosure
- We'll coordinate disclosure timing with you
- Critical vulnerabilities may be disclosed sooner if actively exploited

**Out of scope**:
- Vulnerabilities in third-party dependencies (report to upstream projects)
- Docker daemon vulnerabilities (report to Docker)
- Issues requiring physical access to the machine
- Social engineering attacks

## Troubleshooting

### Connection lost after container restart

**Symptom**: MCP tools stop working after restarting Docker containers

**Cause**: The MCP client (Claude Code/Cursor) uses stdio transport, which creates a direct process connection. When you restart the container with `docker restart` or `docker-compose restart`, the stdio connection breaks and cannot automatically reconnect.

**Solution**: Reload the MCP configuration to re-establish the connection:

**Claude Desktop**:
1. Open the settings menu (⌘+,)
2. Navigate to "Developer" tab
3. Click "Reload MCP Configuration"
4. Or restart Claude Desktop entirely

**Cursor**:
1. Open Command Palette (⌘+Shift+P or Ctrl+Shift+P)
2. Run "MCP: Reload Configuration"
3. Or restart Cursor entirely

**Alternative**: Instead of restarting containers, use Docker's log viewing to troubleshoot without breaking the connection:
```bash
# View logs without restarting
docker compose -f ~/.maproom-mcp/docker-compose.yml logs -f postgres
docker compose -f ~/.maproom-mcp/docker-compose.yml logs -f ollama
docker compose -f ~/.maproom-mcp/docker-compose.yml logs -f maproom-mcp
```

**Why this happens**: MCP's stdio transport creates a persistent process pipe between the client and server. Unlike HTTP-based protocols that can reconnect automatically, stdio connections are tied to the server process lifecycle. When the container restarts, the original process exits and a new one is created, requiring a new connection from the client.

### Docker is not running

**Error**: `Cannot connect to the Docker daemon`

**Solution**: Start Docker Desktop or the Docker service

```bash
# macOS
open -a Docker

# Linux (systemd)
sudo systemctl start docker
```

### Port already in use

**Error**: `port is already allocated`

**Solution**: Change the port or stop the conflicting service

```bash
# Use different port
MAPROOM_PORT=8080 npx @crewchief/maproom-mcp start

# Or find and stop the conflicting service
lsof -i :3000
```

### Services fail to start

**Check service logs**:

```bash
npx @crewchief/maproom-mcp logs
```

**Common issues**:
- Insufficient memory: Docker Desktop needs at least 4GB RAM
- Disk space: Embedding model requires ~500MB, database grows with indexed code
- Network issues: Ensure Docker can pull images from Docker Hub

### Reset everything

If services are in a bad state, reset and restart:

```bash
npx @crewchief/maproom-mcp stop
docker compose -f <path-to-config>/docker-compose.yml down -v
npx @crewchief/maproom-mcp start
```

## Architecture

The stack consists of three Docker services orchestrated automatically:

1. **PostgreSQL 16** (`pgvector/pgvector:pg16`)
   - Vector database with pgvector extension
   - Stores code chunks, embeddings, and relationships
   - Hybrid search combining full-text (tsvector) and vector similarity (ivfflat)
   - Container name: `maproom-postgres`
   - Network hostname: `maproom-postgres` (unique to avoid conflicts on shared networks)

2. **Ollama** (`ollama/ollama:latest`)
   - Local LLM inference server
   - Runs nomic-embed-text model for 768-dimensional embeddings
   - Completely offline, no API keys or cloud dependencies

3. **Maproom MCP Server** (TypeScript + Node.js)
   - MCP server implementation following Model Context Protocol
   - Communicates via stdio with Claude/Cursor
   - Provides tools: `search`, `open`, `context`, `upsert`, `status`
   - Calls Rust indexer binary for code parsing and indexing

### Database Architecture: Dual PostgreSQL Setup

**Important**: CrewChief uses **two separate PostgreSQL instances** for different purposes:

1. **Maproom MCP PostgreSQL** (this instance)
   - **Purpose**: Production-like MCP service, stable semantic search
   - **Hostname**: `maproom-postgres` (unique to avoid conflicts)
   - **Credentials**: `maproom:maproom`
   - **Database**: `maproom`
   - **When to use**: MCP tools, Claude/Cursor integration, `npx @crewchief/maproom-mcp`

2. **Devcontainer PostgreSQL** (separate instance)
   - **Purpose**: Local development, CLI testing, integration tests
   - **Hostname**: `postgres`
   - **Credentials**: `postgres:postgres`
   - **Database**: `crewchief`
   - **When to use**: `cargo run`, development, `cargo test`

**Why two instances?**

- **Isolation**: Development database can be reset without affecting MCP service
- **Network Safety**: Prevents hostname conflicts on shared Docker networks
- **Use-Case Optimization**: Each tuned for its specific workload
- **Data Separation**: Development data vs. production-like persistent data

For complete details on the dual-database architecture, see [Database Architecture Documentation](../../docs/architecture/DATABASE_ARCHITECTURE.md).

### Database Configuration

**Zero Configuration Default:**

The MCP server automatically connects to `maproom-postgres:5432` without requiring `DATABASE_URL` configuration. This provides true zero-config setup for most users.

**Default connection string:**
```
postgresql://maproom:maproom@maproom-postgres:5432/maproom
```

**Why `maproom-postgres` instead of `postgres`?**

On shared Docker networks (e.g., in devcontainer environments), the generic hostname `postgres` can resolve to multiple PostgreSQL instances, causing authentication failures. Using the unique hostname `maproom-postgres` ensures the MCP server always connects to the correct database instance.

**Custom Database (Optional):**

To use a different database, set `DATABASE_URL` in your MCP configuration:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "DATABASE_URL": "postgresql://user:pass@custom-host:5432/mydb"
      }
    }
  }
}
```

**Network Configuration**:
- The postgres service has a network alias `maproom-postgres` in the `maproom-network`
- This alias is consistent across both development and production docker-compose configurations
- Services on shared networks can coexist without hostname conflicts

## Documentation

For more information:
- [Full Documentation](https://github.com/your-org/crewchief/tree/main/packages/maproom-mcp)
- [MCP Protocol](https://modelcontextprotocol.io)
- [Ollama Models](https://ollama.com/library)
- [pgvector](https://github.com/pgvector/pgvector)

## License

MIT - see [LICENSE](./LICENSE) file for details
