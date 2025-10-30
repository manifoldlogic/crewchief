# Ticket: CFGVER-3002: Implement safe Docker volume cleanup with label filtering

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- code-reviewer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement safe Docker volume cleanup that removes only Maproom-specific volumes using label filtering, ensuring user's other Docker volumes are never touched. This prevents accumulation of old volumes while maintaining absolute safety.

## Background
After stopping containers, old Docker volumes may accumulate over multiple config updates. These volumes consume disk space and clutter the Docker environment. However, volume cleanup is extremely dangerous if done incorrectly - we must never delete user's other Docker volumes.

This ticket implements label-based volume filtering to ensure only Maproom volumes are removed. The Docker compose file must be updated to add labels to volumes, and cleanup code must use strict filtering.

Reference: `architecture.md` lines 224-234 and `security-review.md` lines 168-187 for volume cleanup approach and safety requirements.

## Acceptance Criteria
- [ ] Function `cleanupOldVolumes()` removes only Maproom-labeled volumes
- [ ] Uses label filter: `label=com.crewchief.maproom=true`
- [ ] Does not affect user's other Docker volumes under any circumstances
- [ ] Handles Docker not available gracefully with log message
- [ ] Returns count of volumes removed for debugging
- [ ] Docker compose template includes label on volumes
- [ ] Best effort operation (doesn't fail update if cleanup fails)

## Technical Requirements
- **Module Location:** `packages/maproom-mcp/src/config-manager.ts`
- **Function Name:** `cleanupOldVolumes()`
- **Docker Command:** `docker volume prune -f --filter label=com.crewchief.maproom=true`
- **Implementation:**
  - Use `execFile` with array arguments: `['volume', 'prune', '-f', '--filter', 'label=com.crewchief.maproom=true']`
  - Parse output to count removed volumes
  - Log volumes removed for debugging
  - Handle errors gracefully (don't fail update)
  - Update docker-compose.yml template to add labels

**Docker Compose Template Changes:**
```yaml
volumes:
  maproom-pgdata:
    labels:
      - com.crewchief.maproom=true
```

## Implementation Notes
**Security-Critical Implementation (from `security-review.md` lines 168-187):**

```javascript
const { execFile } = require('child_process');
const { promisify } = require('util');
const execFileAsync = promisify(execFile);

async function cleanupOldVolumes() {
  try {
    const { stdout } = await execFileAsync('docker', [
      'volume',
      'prune',
      '-f',
      '--filter', 'label=com.crewchief.maproom=true'
    ], {
      cwd: CACHE_DIR,
      timeout: 15000 // 15 second timeout
    });

    // Parse output to count removed volumes
    const match = stdout.match(/Total reclaimed space: (.+)/);
    const removed = match ? match[1] : '0B';

    logger.info(`Cleaned up Maproom volumes: ${removed} reclaimed`);
    return { success: true, reclaimed: removed };
  } catch (error) {
    if (error.code === 'ENOENT') {
      logger.warn('Docker not available, skipping volume cleanup');
      return { success: true, skipped: true };
    }

    // Best effort - log warning but don't fail update
    logger.warn('Volume cleanup failed (non-critical):', error.message);
    return { success: false, error: error.message };
  }
}
```

**CRITICAL SAFETY RULES:**
1. **NEVER use `docker system prune`** - Too destructive, removes images, networks, build cache
2. **NEVER use `docker volume prune` without label filter** - Would delete ALL unused volumes
3. **NEVER use `docker volume rm` without explicit volume name** - Could delete wrong volume
4. **ALWAYS use label filter** - `--filter label=com.crewchief.maproom=true`
5. **ALWAYS use `-f` flag** - Prevent interactive prompts in automated scripts

**Error Handling:**
- Docker not found → Skip with log message
- Permission denied → Log warning, continue
- No volumes found → Log "No volumes to cleanup"
- Timeout → Log warning, continue

**Docker Compose Template Update:**
File: `packages/maproom-mcp/config/docker-compose.yml`

Add labels to volume definitions:
```yaml
services:
  maproom-postgres:
    volumes:
      - maproom-pgdata:/var/lib/postgresql/data

volumes:
  maproom-pgdata:
    labels:
      - com.crewchief.maproom=true
```

## Dependencies
- CFGVER-3001 (containers must be stopped before volume cleanup)

## Risk Assessment
- **Risk**: Deleting user's Docker volumes
  - **Mitigation**: Strict label filtering, never use volume name patterns
  - **Severity**: Critical (irreversible data loss)
  - **Reference**: `security-review.md` lines 168-187

- **Risk**: Using `docker system prune` accidentally
  - **Mitigation**: Code review must verify ONLY `volume prune` with label filter
  - **Severity**: Critical (destroys images, networks, build cache)

- **Risk**: Permission errors preventing cleanup
  - **Mitigation**: Best effort operation, log warning, continue update
  - **Severity**: Low (volumes accumulate but no data loss)

- **Risk**: Forgetting to add labels to new volumes
  - **Mitigation**: Template includes labels, code review verifies all volumes labeled
  - **Severity**: Medium (volumes won't be cleaned up)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `cleanupOldVolumes()` function)
- **Modify**: `packages/maproom-mcp/config/docker-compose.yml` (add labels to volumes)
- **Execute**: `docker` command (external dependency)

**Environment Detection Note**:
Volume cleanup commands work identically in both devcontainer and standalone environments. The label filtering ensures only Maproom MCP volumes are affected regardless of where Docker is running.
