# Analysis: Docker-in-Docker Workspace Path Detection

## Problem Definition

### Current Behavior

When a user runs the standard setup command inside a devcontainer:

```bash
npx @crewchief/maproom-mcp setup --provider=openai
```

The setup process:
1. ✅ Detects Docker daemon is running
2. ✅ Validates Docker Compose is available
3. ✅ Copies docker-compose.yml to `~/.maproom-mcp/`
4. ✅ Starts containers with `docker compose up -d`
5. ❌ **Container cannot access workspace files**

### Root Cause

**Docker-in-Docker Architecture Issue:**

```
Host Machine (/Users/user/project)
    ↓ bind mount
Docker Desktop
    └─ devcontainer
        ├─ /workspace → /host_mnt/Users/user/project (bind mount from host)
        └─ docker.sock → /var/run/docker.sock (host Docker daemon)

When devcontainer spawns sibling containers via docker.sock:
    └─ maproom-mcp container
        ├─ Runs on host Docker daemon (not inside devcontainer)
        └─ Volume mount: ${WORKSPACE_HOST_PATH:-/workspace}:/workspace:ro
            ├─ WORKSPACE_HOST_PATH not set → defaults to "/workspace"
            └─ "/workspace" doesn't exist on host → mount fails
```

**The disconnect:**
- Inside devcontainer: `/workspace` exists and maps to `/host_mnt/Users/user/project`
- On Docker host: `/workspace` doesn't exist
- Sibling containers need host path: `/host_mnt/Users/user/project`

### Evidence from Session

**Database has correct paths:**
```sql
SELECT abs_path FROM maproom.worktrees WHERE name = 'main';
-- Result: /workspace
```

**But container can't access files:**
```bash
docker exec maproom-mcp ls /workspace/packages/maproom-mcp/src/index.ts
# Error: No such file or directory
```

**Volume mount inspection:**
```bash
docker inspect maproom-mcp --format '{{range .Mounts}}{{.Source}} -> {{.Destination}}{{end}}'
# Only shows: /var/lib/docker/volumes/maproom-mcp_maproom-logs/_data -> /app/logs
# Missing: /host_mnt/Users/.../crewchief -> /workspace
```

**When manually set:**
```bash
export WORKSPACE_HOST_PATH=/host_mnt/Users/danielbushman/git/manifoldlogic/crewchief
docker compose up -d maproom-mcp
docker exec maproom-mcp ls /workspace/packages/maproom-mcp/src/index.ts
# Success! File accessible
```

## Industry Solutions

### LSP Servers (Language Server Protocol)

**How they solve it:**
1. **Path mapping configuration**: Allow users to configure workspace root
2. **Stdio transport**: Run server inside the container, communicate via stdin/stdout
3. **Dynamic detection**: Detect execution environment and adjust paths

**Example: rust-analyzer in devcontainer:**
```json
{
  "rust-analyzer.server.path": "/usr/local/bin/rust-analyzer",
  "rust-analyzer.cargo.target": "x86_64-unknown-linux-gnu"
}
```
- Runs inside devcontainer
- No path mapping needed
- Direct filesystem access

### DevContainer Extensions (VS Code)

**Workspace mounting strategy:**
```json
{
  "workspaceFolder": "/workspace",
  "mounts": [
    "source=${localWorkspaceFolder},target=/workspace,type=bind"
  ]
}
```

**For sibling containers:**
- Extensions detect `${localWorkspaceFolder}` via environment variables
- Pass host path to sibling containers
- Use `WORKSPACE_FOLDER` or custom env vars

### Sourcegraph Code Intelligence

**Multi-environment support:**
```yaml
# docker-compose.yml
services:
  sourcegraph:
    volumes:
      - ${SRC_WORKSPACE_PATH:-/src}:/data/repositories:ro
```

**Detection logic:**
```bash
# Detect if running in Docker
if [ -f /.dockerenv ]; then
  # Inside Docker - find host path
  SRC_WORKSPACE_PATH=$(docker inspect $(hostname) | jq -r '.[0].Mounts[] | select(.Destination=="/workspace") | .Source')
fi
```

### GitHub Codespaces

**Automatic path detection:**
- Sets `GITHUB_WORKSPACE` environment variable
- All sibling containers use this variable
- Works transparently in cloud and local environments

```yaml
volumes:
  - ${GITHUB_WORKSPACE:-/workspace}:/workspace:ro
```

## Current Implementation

### docker-compose.yml (Line 167)

```yaml
volumes:
  - maproom-logs:/app/logs
  - ${WORKSPACE_HOST_PATH:-/workspace}:/workspace:ro
```

**Intended behavior:**
- If `WORKSPACE_HOST_PATH` set → use it
- Otherwise → default to `/workspace`

**Actual behavior:**
- `WORKSPACE_HOST_PATH` never set by setup command
- Always defaults to `/workspace`
- Works on host, fails in devcontainer

### bin/cli.cjs startDockerCompose() (Line 799-827)

```javascript
const env = {
  ...process.env,  // Spreads all environment variables
  MAPROOM_EMBEDDING_PROVIDER: process.env.MAPROOM_EMBEDDING_PROVIDER || 'ollama',
  MAPROOM_EMBEDDING_MODEL: process.env.MAPROOM_EMBEDDING_MODEL || 'nomic-embed-text',
  EMBEDDING_DIMENSION: process.env.MAPROOM_EMBEDDING_DIMENSION || '768'
};

const compose = spawn('docker', args, {
  cwd: CONFIG_DIR,
  env: env,  // Passes environment to docker compose
  stdio: ['ignore', 'pipe', 'pipe'],
  encoding: 'utf-8'
});
```

**What works:**
- Spreads `process.env` → includes any environment variables
- Sets provider-specific variables

**What's missing:**
- No code sets `WORKSPACE_HOST_PATH` before spreading
- No Docker-in-Docker detection
- No host path discovery

## Gap Analysis

### What Users Expect

```bash
# Inside devcontainer
npx @crewchief/maproom-mcp setup --provider=openai
# Should just work ✅
```

### What Actually Happens

```bash
# Inside devcontainer
npx @crewchief/maproom-mcp setup --provider=openai
# Containers start ✅
# But can't access workspace ❌
```

### Required User Workaround (Currently)

```bash
# Manual discovery
export WORKSPACE_HOST_PATH=$(docker inspect $(hostname) --format '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}')

# Then setup
npx @crewchief/maproom-mcp setup --provider=openai
# Now it works ✅
```

### What Should Happen

The setup command should:
1. **Detect environment**: Check if running inside Docker
2. **Discover host path**: Query Docker for actual mount source
3. **Set environment**: Export `WORKSPACE_HOST_PATH` automatically
4. **Proceed normally**: Continue with docker compose up

## Technical Requirements

### Detection Logic

**Check if inside Docker container:**
```javascript
function isInsideDocker() {
  // Method 1: Check for /.dockerenv file (most reliable)
  if (fs.existsSync('/.dockerenv')) return true;

  // Method 2: Check for /run/.containerenv (Podman)
  if (fs.existsSync('/run/.containerenv')) return true;

  // Method 3: Check cgroup (fallback)
  try {
    const cgroup = fs.readFileSync('/proc/1/cgroup', 'utf8');
    return cgroup.includes('docker') || cgroup.includes('containerd');
  } catch {
    return false;
  }
}
```

### Host Path Discovery

**Get actual host mount source:**
```javascript
function getWorkspaceHostPath() {
  try {
    const hostname = execSync('hostname', { encoding: 'utf8' }).trim();
    const cmd = `docker inspect ${hostname} --format '{{range .Mounts}}{{if eq .Destination "/workspace"}}{{.Source}}{{end}}{{end}}'`;
    const hostPath = execSync(cmd, { encoding: 'utf8' }).trim();

    if (hostPath) {
      return hostPath;
    }
  } catch (error) {
    // If inspection fails, we're probably not in a devcontainer
    return null;
  }

  return null;
}
```

### Fallback Strategy

**For non-devcontainer environments:**
```javascript
function resolveWorkspacePath() {
  // Check if already set (user override)
  if (process.env.WORKSPACE_HOST_PATH) {
    return process.env.WORKSPACE_HOST_PATH;
  }

  // Check if inside Docker
  if (isInsideDocker()) {
    const hostPath = getWorkspaceHostPath();
    if (hostPath) {
      return hostPath;
    }

    // Fallback to /workspace if detection fails
    console.warn('⚠️  Running in Docker but could not detect host path. Using /workspace');
    return '/workspace';
  }

  // Running on host - use current directory
  return process.cwd();
}
```

## Test Strategy

### Failing Test First

**Test file:** `tests/workspace-path-detection.test.js`

```javascript
describe('Docker-in-Docker workspace path detection', () => {
  it('should detect when running inside Docker', () => {
    // Stub: fs.existsSync('/.dockerenv') → true
    // Expect: isInsideDocker() → true
  });

  it('should discover host path from docker inspect', () => {
    // Stub: execSync('hostname') → 'container-id'
    // Stub: execSync('docker inspect...') → '/host_mnt/Users/user/project'
    // Expect: getWorkspaceHostPath() → '/host_mnt/Users/user/project'
  });

  it('should set WORKSPACE_HOST_PATH before docker compose', () => {
    // Run: setup command in simulated devcontainer
    // Verify: process.env.WORKSPACE_HOST_PATH set
    // Verify: passed to docker compose spawn
  });
});
```

### Validation Test

**Integration test:**
```javascript
it('should allow container to access workspace files after setup', async () => {
  // Setup: Run full setup command
  // Execute: docker exec maproom-mcp ls /workspace
  // Expect: Files are accessible
});
```

## Constraints and Considerations

### Security

- **Docker socket access**: Already required for Docker-in-Docker
- **No new permissions**: Uses existing docker inspect capability
- **Read-only mount**: Workspace mounted as `:ro` (already configured)

### Performance

- **One-time detection**: Runs only during setup, not on every request
- **Cached in environment**: Once set, no repeated detection
- **Fast operations**: `docker inspect` is instant (<100ms)

### Compatibility

**Must work in (MVP):**
- ✅ VS Code devcontainer (macOS/Linux hosts)
- ✅ Cursor devcontainer (macOS/Linux hosts)
- ✅ GitHub Codespaces
- ✅ Local Docker Desktop (host execution on macOS/Linux)

**Expected to work (not specifically tested in MVP):**
- ⏸ CI/CD environments (detection may fail in some environments - will gracefully fallback)
- ⏸ Windows/WSL2 devcontainers (WSL2 uses Linux paths, should work)
- ⏸ Podman (detection includes `/run/.containerenv` check, best-effort support)

**CI/CD Behavior:**
- Some CI environments have unusual hostname patterns that may break detection
- Expected behavior: Detection fails → gracefully falls back to `/workspace`
- This is acceptable - CI environments typically don't need devcontainer-style path resolution
- Manual `WORKSPACE_HOST_PATH` override available if needed

**Windows/WSL2:**
- Out of MVP scope for specific testing
- WSL2 uses Linux filesystem paths, detection logic should work
- If issues arise, users can manually set `WORKSPACE_HOST_PATH`

**Edge cases:**
- No docker socket access → graceful fallback to `/workspace`
- Multiple workspace mounts → use first match (first `/workspace` destination found)
- No workspace mount → use `process.cwd()`

## Success Metrics

1. **Zero manual configuration**: User runs setup command, it works
2. **Test coverage**: Failing test proves problem, passing test proves fix
3. **Backward compatibility**: Works on host without devcontainer
4. **Clear errors**: If detection fails, helpful error message
5. **Documentation**: README explains how it works

## Related Documentation

- `DOCKER_WORKSPACE_SOLUTION.md`: Industry analysis and architectural options
- `.devcontainer/docker-compose.yml`: Devcontainer configuration
- `packages/maproom-mcp/config/docker-compose.yml`: MCP service configuration
- `packages/maproom-mcp/bin/cli.cjs`: Setup command implementation
