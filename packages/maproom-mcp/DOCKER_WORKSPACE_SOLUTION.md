# Docker Workspace Access Solution for Maproom MCP

## Problem Statement

The Maproom MCP server runs in a Docker container and needs to:
1. Index code files from the workspace
2. Access the PostgreSQL database
3. Work seamlessly in devcontainer environments
4. Be easy to set up with zero manual configuration

**Current Issue:** Docker-in-Docker (DinD) environments create sibling containers that can't easily share filesystem mounts.

## Industry Analysis

### How Others Solve This

1. **LSP Servers (Language Server Protocol)**
   - **Path Mapping**: Use configurable mappings between host paths and container paths
   - **Stdio Transport**: Run the server inside the container and communicate via stdin/stdout
   - **Volume Sharing**: Mount the workspace into the language server container

2. **DevContainer Best Practices**
   - **Shared Network**: All services on the same Docker network (e.g., `crewchief-network`)
   - **Bind Mounts**: The devcontainer already has the workspace mounted from the host
   - **Volume Reuse**: Sibling containers can share the same bind mount source

3. **Code Indexing Tools (Sourcegraph, Kythe, etc.)**
   - **Agent Pattern**: Run indexer as a sidecar container
   - **Volume Sharing**: Use external volumes accessible to both devcontainer and indexer
   - **Database-First**: Store paths relative to a known mount point

## Our Elegant Solution: **External Volume with Devcontainer Integration**

### Architecture

```
Host Machine
  └─ /Users/.../crewchief  (actual source code)
      ↓ bind mount
  Docker Desktop
    └─ devcontainer (crewchief_devcontainer-devcontainer-1)
        ├─ /workspace → /host_mnt/Users/.../crewchief
        └─ shares network: crewchief-network

    └─ maproom-mcp containers (started via npx)
        ├─ postgres (on crewchief-network)
        ├─ ollama (on crewchief-network)
        └─ maproom-mcp (on crewchief-network)
            └─ /workspace → (same bind mount as devcontainer)
```

### Solution Components

#### 1. **Shared Docker Network** ✅ (Already Implemented)

Both the devcontainer and maproom containers join the same external network:

```yaml
# In ~/.maproom-mcp/docker-compose.yml
networks:
  crewchief-network:
    external: true
    name: crewchief_devcontainer_crewchief-network
```

**Benefits:**
- Database accessible via `postgres:5432` from maproom-mcp
- No port forwarding needed
- Container-to-container communication

#### 2. **Workspace Bind Mount Strategy**

**Option A: Direct Bind Mount** (Recommended for devcontainers)

```yaml
# In ~/.maproom-mcp/docker-compose.yml
services:
  maproom-mcp:
    volumes:
      - /workspace:/workspace:ro  # Inside devcontainer, this works!
      - maproom-logs:/app/logs
```

**How it works:**
- The `/workspace` path exists inside the devcontainer
- When the devcontainer spawns sibling containers, they can mount `/workspace`
- Because the devcontainer has Docker-in-Docker enabled, sibling containers can access the same bind mounts

**Option B: External Volume** (For non-devcontainer setups)

```yaml
volumes:
  workspace-code:
    external: true
    name: ${WORKSPACE_VOLUME:-workspace-code}

services:
  maproom-mcp:
    volumes:
      - workspace-code:/workspace:ro
```

#### 3. **Auto-Detection Script in bin/cli.js**

The npx wrapper should:
1. Detect if running in a devcontainer (check `$IN_DEVCONTAINER` env var)
2. Auto-configure the docker-compose.yml based on environment
3. Use the appropriate network and volume strategy

```javascript
// In bin/cli.js (conceptual)
function setupDockerCompose() {
  const isDevContainer = process.env.IN_DEVCONTAINER === 'true';
  const workspaceFolder = process.env.WORKSPACE_FOLDER || '/workspace';

  if (isDevContainer) {
    // Use the existing workspace mount
    return {
      volumes: [`${workspaceFolder}:/workspace:ro`],
      networks: { 'crewchief-network': { external: true } }
    };
  } else {
    // Standard Docker Desktop setup
    return {
      volumes: [`${process.cwd()}:/workspace:ro`],
      networks: { 'maproom-network': {} }
    };
  }
}
```

#### 4. **Template docker-compose.yml with Environment Variables**

```yaml
services:
  maproom-mcp:
    volumes:
      - maproom-logs:/app/logs
      - ${WORKSPACE_PATH:-/workspace}:/workspace:ro
    networks:
      - ${DOCKER_NETWORK:-maproom-network}
    environment:
      DATABASE_URL: postgresql://maproom:maproom@${POSTGRES_HOST:-postgres}:5432/maproom

networks:
  maproom-network:
    external: ${NETWORK_EXTERNAL:-false}
    name: ${NETWORK_NAME:-maproom-network}
```

**Usage:**
```bash
# In devcontainer
WORKSPACE_PATH=/workspace DOCKER_NETWORK=crewchief-network NETWORK_EXTERNAL=true npx @crewchief/maproom-mcp

# On host
WORKSPACE_PATH=$(pwd) npx @crewchief/maproom-mcp
```

## Implementation Plan

### Phase 1: Devcontainer Integration (Immediate)

1. Update `~/.maproom-mcp/docker-compose.yml`:
   - Add external network reference to `crewchief-network`
   - Mount `/workspace:/workspace:ro` (works inside devcontainer)
   - Keep postgres on port 5433 to avoid conflicts

2. Test:
   ```bash
   cd ~/.maproom-mcp
   docker compose down
   docker compose up -d
   ```

3. Verify workspace is accessible:
   ```bash
   docker exec maproom-mcp ls -la /workspace
   ```

### Phase 2: Auto-Configuration (Polish)

1. Enhance `bin/cli.js` to:
   - Detect environment (devcontainer vs. host)
   - Write appropriate docker-compose.yml from template
   - Set correct environment variables

2. Create templates:
   - `config/docker-compose.devcontainer.yml`
   - `config/docker-compose.host.yml`

### Phase 3: Published Package (Distribution)

1. Update `package.json` files to include:
   - `bin/` directory with Rust binaries
   - `config/` templates

2. Add postinstall script that:
   - Detects environment
   - Copies appropriate configuration
   - Sets up directories

## Why This is Elegant

✅ **Zero Manual Configuration**
   - Auto-detects devcontainer vs. host
   - Uses environment variables for flexibility

✅ **Industry Standard Patterns**
   - Follows LSP server architecture
   - Uses Docker Compose best practices
   - Leverages existing devcontainer setup

✅ **Works Everywhere**
   - Devcontainers (VS Code, Cursor, GitHub Codespaces)
   - Local Docker Desktop
   - CI/CD environments

✅ **No Workarounds**
   - Direct filesystem access (not copying files)
   - Native Docker networking
   - Standard bind mounts

✅ **Maintainable**
   - Single docker-compose file with variables
   - Clear documentation
   - Easy to debug

## Next Steps

1. Implement Phase 1 immediately
2. Test scan functionality
3. Document in README
4. Publish updated package with configuration templates

---

**Date:** 2025-10-27
**Author:** Analysis based on industry research and devcontainer inspection
**Status:** Ready for implementation
