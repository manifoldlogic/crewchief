# Ticket: LOCAL-2504: Fix CLI Wrapper Critical Bugs (Volume Creation & Service Name)

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Fix two critical bugs discovered during LOCAL-2502 verification that prevent the CLI wrapper from functioning. The bugs are: (1) external Docker volume `maproom-init-sql` is referenced but never created, and (2) service name mismatch between CLI code and docker-compose.yml configuration.

## Background
During verification of LOCAL-2502 (CLI wrapper implementation), two blocking bugs were identified:

1. **External Volume Bug**: The docker-compose.yml references an external volume `maproom-init-sql` (line 102-103) that is never created. When `docker compose up -d` runs, it fails because Docker cannot find this external volume. The volume was intended to mount the `init.sql` schema file but is incorrectly configured as external.

2. **Service Name Mismatch**: The CLI health check logic searches for a service named `maproom` (line 266 in cli.js), but the docker-compose.yml defines the service as `maproom-mcp` (line 62). This causes health checks to fail because the CLI cannot find the service it's looking for.

These bugs block the entire Phase 2.5-3 workflow:
- LOCAL-3001 (test npx startup flow) - cannot test without working CLI
- LOCAL-3008 (npm publish) - cannot publish broken package
- All Phase 4 tickets - depend on working Docker stack

The implementation is complete and correct except for these two configuration mismatches. This ticket focuses solely on fixing these bugs without changing any other functionality.

## Acceptance Criteria
- [x] Docker volume `maproom-init-sql` issue resolved (either created or removed from external dependencies)
- [x] Service name in CLI health check matches docker-compose.yml service definition
- [x] CLI successfully runs `docker compose up -d` without volume errors
- [x] CLI health check successfully finds and monitors the maproom service
- [x] All three services (postgres, ollama, maproom-mcp) start and become healthy
- [x] Stdio proxy connects to the correct container
- [x] Manual test: `node /workspace/packages/maproom-mcp/bin/cli.js` completes successfully
- [x] No regression in other CLI functionality (Docker checks, error handling, shutdown)

## Technical Requirements

### Bug 1: Fix External Volume Configuration

**Current (broken):**
```yaml
# docker-compose.yml line 12
volumes:
  - maproom-data:/var/lib/postgresql/data
  - maproom-init-sql:/docker-entrypoint-initdb.d:ro

# docker-compose.yml line 102-103
volumes:
  maproom-init-sql:
    external: true
```

**Problem**: Volume is marked as `external: true`, meaning Docker expects it to already exist outside the compose stack. It was never created.

**Solution Options**:

**Option A (Recommended)**: Remove external volume, mount init.sql directly from config directory
```yaml
# docker-compose.yml line 12
volumes:
  - maproom-data:/var/lib/postgresql/data
  - ./init.sql:/docker-entrypoint-initdb.d/init.sql:ro

# docker-compose.yml line 102-103 - REMOVE maproom-init-sql entirely
volumes:
  maproom-data:
    driver: local
  ollama-models:
    driver: local
  maproom-logs:
    driver: local
  # maproom-init-sql removed
```

**Option B**: Create the volume as a managed volume (not external)
```yaml
# docker-compose.yml line 102-103
volumes:
  maproom-data:
    driver: local
  ollama-models:
    driver: local
  maproom-logs:
    driver: local
  maproom-init-sql:
    driver: local  # Changed from external: true
```

**Recommendation**: Use Option A. It's simpler and the CLI already copies `init.sql` to `~/.maproom-mcp/` (line 105-116 of cli.js), so mounting it directly makes more sense than creating a separate volume.

### Bug 2: Fix Service Name Mismatch

**Current (broken):**
```javascript
// cli.js line 266
const requiredServices = ['postgres', 'ollama', 'maproom'];
```

```yaml
# docker-compose.yml line 62
maproom-mcp:
  build:
    context: .
```

**Problem**: CLI searches for service `maproom` but docker-compose defines `maproom-mcp`.

**Solution**:
```javascript
// cli.js line 266 - UPDATE
const requiredServices = ['postgres', 'ollama', 'maproom-mcp'];
```

Also verify consistency in error messages:
```javascript
// cli.js line 342, 366 - UPDATE log messages
console.error('  docker compose logs maproom-mcp');  // Was: maproom
```

### Verification Steps

1. **Test volume creation**:
   ```bash
   cd ~/.maproom-mcp
   docker compose down -v  # Clean slate
   docker compose up -d    # Should succeed without volume errors
   ```

2. **Test service health checks**:
   ```bash
   docker compose ps --format json | jq '.Service'
   # Should show: postgres, ollama, maproom-mcp
   ```

3. **Test CLI execution**:
   ```bash
   node /workspace/packages/maproom-mcp/bin/cli.js
   # Should complete all health checks and establish stdio proxy
   ```

4. **Test stdio proxy**:
   ```bash
   # With CLI running, send MCP initialize request
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05"}}' | node /workspace/packages/maproom-mcp/bin/cli.js
   # Should receive valid MCP response
   ```

## Implementation Notes

### File Changes Required

1. **`/workspace/packages/maproom-mcp/config/docker-compose.yml`**:
   - Line 12: Change volume mount from `maproom-init-sql:/docker-entrypoint-initdb.d:ro` to `./init.sql:/docker-entrypoint-initdb.d/init.sql:ro`
   - Lines 102-103: Remove `maproom-init-sql` volume definition entirely

2. **`/workspace/packages/maproom-mcp/bin/cli.js`**:
   - Line 266: Change `'maproom'` to `'maproom-mcp'` in requiredServices array
   - Line 342: Change error log from `docker compose logs maproom` to `docker compose logs maproom-mcp`
   - Line 366: Change error log from `docker compose logs maproom` to `docker compose logs maproom-mcp`

### Why These Bugs Were Missed

1. **Volume bug**: The docker-compose.yml was likely copied from a template that assumed external volume creation. The CLI implementation correctly copies init.sql to the config directory but didn't update the compose file to use file mounting instead.

2. **Service name bug**: The service was originally designed as `maproom` but renamed to `maproom-mcp` for clarity. The CLI health check logic wasn't updated to match the rename.

### No Other Changes Required

- Container name `maproom-mcp` (line 66) is correctly used in stdio proxy (cli.js line 385)
- All other service references are correct
- Health check logic structure is sound, just needs correct service name
- Volume mounting strategy is correct, just needs to remove external volume

## Dependencies
- **Blocks**: LOCAL-3001 (test npx startup flow) - cannot test without working CLI
- **Blocks**: LOCAL-3008 (npm publish) - cannot publish broken package
- **Blocks**: All Phase 4 performance/testing tickets
- **Related**: LOCAL-2502 (parent ticket that introduced these bugs)
- **Priority**: HIGH - blocks critical Phase 3 deliverables

## Risk Assessment

### Risk: Breaking existing Docker stack if users already created external volume manually
- **Impact**: Low - this is new code, no existing users yet
- **Mitigation**: Phase 2.5 is not yet released, no production users
- **Mitigation**: Document cleanup steps in LOCAL-3005 (troubleshooting guide)

### Risk: init.sql file path resolution issues on different platforms
- **Impact**: Low - relative paths work in docker-compose context
- **Mitigation**: CLI already copies init.sql to `~/.maproom-mcp/` where compose runs
- **Mitigation**: Test on Linux and macOS (primary platforms)

### Risk: Missed service name references elsewhere in codebase
- **Impact**: Medium - other references might fail
- **Mitigation**: Grep for all occurrences of service name `maproom` in CLI
- **Mitigation**: Verify ticket acceptance criteria includes full test run

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/config/docker-compose.yml` - fix volume configuration
- `/workspace/packages/maproom-mcp/bin/cli.js` - fix service name references

## Success Metrics
After bug fixes:
1. `docker compose up -d` succeeds without volume errors
2. CLI health check finds all three services (postgres, ollama, maproom-mcp)
3. All services become healthy within 2 minutes
4. Stdio proxy establishes connection to maproom-mcp container
5. MCP JSON-RPC communication works end-to-end
6. Manual CLI test passes: `node bin/cli.js` → healthy services → Ctrl+C clean exit
7. Enables LOCAL-3001 testing to proceed
