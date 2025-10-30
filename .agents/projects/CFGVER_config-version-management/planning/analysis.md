# Config Version Management - Analysis

## Problem Space

### The Configuration Drift Problem

The Maproom MCP CLI (`@crewchief/maproom-mcp`) uses a cached configuration directory at `~/.maproom-mcp/` that contains:
- `docker-compose.yml` - Service orchestration configuration
- `init.sql` - Database initialization
- `Dockerfile.mcp-server` - Container build instructions
- TypeScript source files for local builds

When users run `npx -y @crewchief/maproom-mcp@latest`, the CLI copies these files from the npm package to the cache directory. However, **once copied, these files persist across package updates** unless explicitly detected as outdated.

### Real-World Impact (October 30, 2024)

A configuration drift incident occurred when:
1. The docker-compose.yml was updated to use published Docker images (`image: manifoldlogic/crewchief_maproom-mcp:latest`)
2. The old cached config still used local builds (`build: context: ../../..`)
3. Users with cached configs from before the change experienced connection failures
4. The CLI's update detection logic **only checked for one specific pattern** and missed this architectural change

**Symptoms:**
- MCP server failed to connect after Claude Code restart
- Docker containers wouldn't start (invalid build context path)
- No automatic recovery mechanism
- Required manual deletion of `~/.maproom-mcp/` to fix

### Current State: Partial Detection (Insufficient)

The existing update logic in `packages/maproom-mcp/bin/cli.cjs` (lines 209-223):

```javascript
// Check if existing docker-compose.yml needs updating
let needsUpdate = !fs.existsSync(COMPOSE_FILE);

if (!needsUpdate && fs.existsSync(COMPOSE_FILE)) {
  const existingContent = fs.readFileSync(COMPOSE_FILE, 'utf-8');
  // Check if file has old hardcoded EMBEDDING_PROVIDER (MCP-008 fix)
  const hasHardcodedProvider = existingContent.includes('EMBEDDING_PROVIDER: ollama');
  const hasEnvironmentVariable = existingContent.includes('${EMBEDDING_PROVIDER');

  if (hasHardcodedProvider && !hasEnvironmentVariable) {
    console.error('⚡ Detected outdated docker-compose.yml (pre-MCP-008)');
    console.error('   Updating to support EMBEDDING_PROVIDER configuration...');
    needsUpdate = true;
  }
}
```

**Problems:**
- Only detects ONE specific outdated pattern
- Doesn't detect architectural changes (build → image)
- Doesn't detect new environment variables
- Doesn't detect service dependency changes
- Requires adding new pattern checks for each breaking change
- Pattern-matching is fragile and error-prone

### The User Experience Gap

**Expected behavior:**
1. User runs `npx -y @crewchief/maproom-mcp@latest`
2. CLI detects package update
3. Config automatically updates
4. Containers start with new configuration
5. Connection succeeds ✅

**Actual behavior:**
1. User runs `npx -y @crewchief/maproom-mcp@latest`
2. CLI uses cached config (outdated)
3. Containers fail to start
4. Connection fails ❌
5. User must manually troubleshoot or delete cache

## Industry Solutions

### How Other Tools Handle Config Management

#### 1. **Docker Desktop** - Version-Tagged Configs
Docker Desktop uses versioned configuration files with explicit version markers:
```yaml
# docker-compose.yml
version: '3.8'
services:
  ...
```
- Explicit version field
- Tools can validate compatibility
- Clear error messages on version mismatch

#### 2. **npm** - Lockfiles with Content Hashing
npm uses `package-lock.json` with integrity hashes:
```json
{
  "lockfileVersion": 2,
  "packages": {
    "node_modules/foo": {
      "integrity": "sha512-abc123..."
    }
  }
}
```
- Content hashing detects ANY change
- Automatic regeneration when mismatched
- No manual pattern maintenance

#### 3. **Homebrew** - Formula Versions with Checksums
Homebrew tracks formula versions and verifies checksums:
```ruby
class Foo < Formula
  url "https://example.com/foo-1.2.3.tar.gz"
  sha256 "abc123..."
end
```
- Explicit version in formula
- Checksum verification
- Auto-update on formula change

#### 4. **Kubernetes** - ConfigMap Versioning
Kubernetes uses ConfigMaps with version labels:
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config-v2
  labels:
    version: "2.0.0"
```
- Version in metadata
- Immutable configs (new version = new ConfigMap)
- Rolling updates based on version

### Common Patterns

All successful config management systems share:
1. **Explicit versioning** - Not implicit pattern detection
2. **Automatic detection** - Tools compare versions on every run
3. **Clear user communication** - Show what changed and why
4. **Safe updates** - Backup or rollback mechanisms
5. **Idempotent operations** - Running multiple times is safe

## Root Cause Analysis

### Why Pattern-Based Detection Fails

The current approach (`includes('EMBEDDING_PROVIDER: ollama')`) fails because:

1. **Maintenance Burden** - Each breaking change requires new pattern
2. **False Negatives** - Miss changes that don't match patterns
3. **False Positives** - Match unrelated content
4. **No Future-Proofing** - Can't detect changes we haven't anticipated
5. **Coupling** - Config structure tightly coupled to detection logic

### The Fundamental Problem

**The CLI doesn't know what version of the config it has.**

Without version tracking:
- Can't detect updates automatically
- Can't show meaningful error messages
- Can't decide whether to update or not
- Must rely on brittle pattern matching

## Research Findings

### npm Package Update Behavior

When users run `npx -y @crewchief/maproom-mcp@latest`:
1. npm checks for latest version on registry
2. Downloads package if newer version exists
3. Runs `bin/cli.cjs` from **new package code**
4. But cached configs in `~/.maproom-mcp/` are **not automatically updated**

**Key insight:** The CLI code is always latest, but configs can be stale.

### Docker Compose File Stability

Docker Compose configs are **not** stable across versions:
- Breaking changes in v2 vs v3 syntax
- New features added (profiles, depends_on conditions)
- Image references change (local build → registry)
- Environment variable syntax evolves

**Implication:** Config updates MUST be automated and reliable.

### User Expectations

Users expect "zero-configuration" behavior:
- `npx -y @crewchief/maproom-mcp@latest` should "just work"
- No manual cache management
- No Docker knowledge required
- Automatic recovery from stale state

## Success Criteria

A successful config management solution must:

1. **Detect ALL config changes** - Not just specific patterns
2. **Update automatically** - No user intervention required
3. **Preserve user safety** - Backup before replacing
4. **Communicate clearly** - Show what's happening and why
5. **Be maintainable** - No pattern list to maintain
6. **Be idempotent** - Safe to run multiple times
7. **Handle edge cases** - First run, corrupted files, permission errors

## Proposed Solution: Version-Based Management

Use explicit version tracking instead of pattern detection:

1. **Version Marker in Config** - Add version comment to docker-compose.yml
2. **Package Version Tracking** - Store package version in `.maproom-version` file
3. **Automatic Comparison** - Compare on every CLI startup
4. **Safe Updates** - Backup old config, replace with new, cleanup containers
5. **Clear Messaging** - Show version changes to user

This aligns with industry best practices and solves all identified problems.
