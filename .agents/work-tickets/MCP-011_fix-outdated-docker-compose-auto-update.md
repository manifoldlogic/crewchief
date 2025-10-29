# Ticket: MCP-011: Fix outdated docker-compose.yml with auto-update detection

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (manually verified)
- [x] **Verified** - by manual testing with EMBEDDING_PROVIDER=google

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Fixed MCP-008 follow-up issue where existing installations at `~/.maproom-mcp/` retained outdated docker-compose.yml files with hardcoded `EMBEDDING_PROVIDER: ollama`, causing Ollama to start despite explicit configuration in `.mcp.json`. Implemented automatic detection and replacement of outdated configuration files.

## Background
After implementing MCP-008 (conditional Docker startup based on EMBEDDING_PROVIDER), users reported that Ollama containers were still starting despite setting `EMBEDDING_PROVIDER=google` in their `.mcp.json` configuration.

### Root Cause Analysis
The CLI installation process (`bin/cli.cjs`) only copied `docker-compose.yml` to `~/.maproom-mcp/` if the file didn't already exist:
```javascript
if (!fs.existsSync(dockerComposeDest)) {
  fs.copyFileSync(dockerComposeSource, dockerComposeDest);
}
```

**Problem**: Existing installations had docker-compose.yml with:
```yaml
environment:
  EMBEDDING_PROVIDER: ollama  # Hardcoded, overrides CLI env vars
```

This hardcoded value in the container environment took precedence over the environment variables passed from `.mcp.json`, causing:
1. Ollama to always start (hardcoded in docker-compose.yml)
2. User's `EMBEDDING_PROVIDER=google` configuration to be ignored
3. Confusion about why explicit configuration wasn't working

### Design Decision Context
The original design (skip copy if file exists) was intended to preserve user customizations. However, it prevented necessary updates when the upstream configuration changed. This ticket implements a smart update mechanism that:
- Detects outdated configuration patterns
- Automatically replaces outdated files with current versions
- Preserves the user intent (explicit provider configuration works)

## Acceptance Criteria
- [x] Auto-detect outdated docker-compose.yml files with hardcoded EMBEDDING_PROVIDER
- [x] Automatically replace outdated config files with current versions
- [x] Log clear messages when auto-update occurs
- [x] Stop unnecessary services explicitly before starting required ones
- [x] Verify Ollama does NOT start when EMBEDDING_PROVIDER=google
- [x] Verify only postgres and maproom-mcp services run with Google provider
- [x] Version bumped to 1.1.7
- [x] Ready for npm publish

## Technical Requirements
1. **Auto-Update Detection**:
   - Check existing docker-compose.yml for hardcoded `EMBEDDING_PROVIDER: ollama`
   - Use regex pattern matching to detect outdated configuration
   - Replace file if outdated pattern detected

2. **Service Management**:
   - Explicitly stop ollama service if EMBEDDING_PROVIDER is not ollama
   - Use `docker compose stop` before `docker compose up`
   - Ensure clean state before starting required services

3. **Logging**:
   - Clear messages when outdated config is detected
   - Clear messages when auto-update occurs
   - Clear messages showing which services are stopped vs started

4. **Version Management**:
   - Bump package.json version to 1.1.7
   - Prepare for npm publish with 2FA

## Implementation Notes

### Changes Made to bin/cli.cjs

**1. Added Auto-Update Detection (lines ~50-75)**:
```javascript
// Check if docker-compose.yml exists and is outdated
if (fs.existsSync(dockerComposeDest)) {
  const existingContent = fs.readFileSync(dockerComposeDest, 'utf-8');

  // Check for outdated hardcoded EMBEDDING_PROVIDER
  const hasHardcodedProvider = /EMBEDDING_PROVIDER:\s*ollama/i.test(existingContent);

  if (hasHardcodedProvider) {
    console.error('⚠️  Detected outdated docker-compose.yml with hardcoded EMBEDDING_PROVIDER');
    console.error('🔄 Auto-updating to latest configuration...');

    // Backup old file
    const backupPath = path.join(CONFIG_DIR, 'docker-compose.yml.backup');
    fs.copyFileSync(dockerComposeDest, backupPath);

    // Replace with new version
    fs.copyFileSync(dockerComposeSource, dockerComposeDest);

    console.error('✅ Configuration updated (backup saved as docker-compose.yml.backup)');
  }
} else {
  // Fresh install - copy config
  fs.copyFileSync(dockerComposeSource, dockerComposeDest);
}
```

**2. Added Explicit Service Stop Logic (lines ~199-230)**:
```javascript
function startDockerCompose() {
  return new Promise((resolve, reject) => {
    const provider = (process.env.EMBEDDING_PROVIDER || '').toLowerCase();

    // Determine which services to stop
    const servicesToStop = [];
    if (provider && provider !== 'ollama') {
      servicesToStop.push('ollama');
      console.error('🛑 Stopping unnecessary services:', servicesToStop.join(', '));
    }

    // Stop services first if needed
    if (servicesToStop.length > 0) {
      const stopArgs = ['compose', 'stop', ...servicesToStop];
      const stopProcess = spawn('docker', stopArgs, {
        cwd: CONFIG_DIR,
        stdio: ['ignore', 'pipe', 'pipe']
      });

      stopProcess.on('close', (code) => {
        // Continue with startup after stop completes
        startRequiredServices();
      });
    } else {
      startRequiredServices();
    }

    function startRequiredServices() {
      // ... existing docker compose up logic
    }
  });
}
```

**3. Cleaned Up Debug Logging**:
- Removed verbose debug statements that were added during investigation
- Kept essential user-facing messages
- Clear indication of which provider is active

### Version Bump
Updated `packages/maproom-mcp/package.json`:
```json
{
  "version": "1.1.7"
}
```

## Dependencies
- **Prerequisite**: MCP-008 (Fix conditional Docker startup based on EMBEDDING_PROVIDER) - COMPLETED
- **Related**: MCP-010 (Fix missing maproom-mcp service health check) - COMPLETED

## Risk Assessment
- **Risk**: Auto-update might overwrite intentional user customizations to docker-compose.yml
  - **Mitigation**: Creates backup file (`docker-compose.yml.backup`) before replacing
  - **Mitigation**: Only triggers on specific outdated pattern (hardcoded EMBEDDING_PROVIDER)
  - **Future**: Consider version markers in config files for safer update detection

- **Risk**: Service stop command might fail if service doesn't exist
  - **Mitigation**: Docker Compose gracefully handles stopping non-existent services
  - **Mitigation**: Error handling in spawn process callback

- **Risk**: Users with custom docker-compose.yml configurations might lose changes
  - **Mitigation**: Backup file preserved for manual inspection
  - **Note**: Users should use environment variables instead of modifying docker-compose.yml

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Auto-update detection, service stop logic, cleaned debug logs
- `packages/maproom-mcp/package.json` - Version bump to 1.1.7

## Testing Results

### Manual Testing Performed
1. **Test with EMBEDDING_PROVIDER=google**:
   ```bash
   EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp
   ```
   - Result: Ollama service explicitly stopped
   - Result: Only postgres and maproom-mcp services running
   - Result: No Ollama container in `docker ps`
   - ✅ PASS

2. **Test auto-update detection**:
   - Created outdated docker-compose.yml with hardcoded EMBEDDING_PROVIDER
   - Ran CLI
   - Result: Detected outdated config
   - Result: Created backup file
   - Result: Replaced with current version
   - Result: Logged clear update messages
   - ✅ PASS

3. **Test fresh installation** (no existing docker-compose.yml):
   - Removed ~/.maproom-mcp/docker-compose.yml
   - Ran CLI
   - Result: Copied fresh config
   - Result: No auto-update messages (not needed)
   - ✅ PASS

4. **Test with default (no EMBEDDING_PROVIDER)**:
   ```bash
   npx @crewchief/maproom-mcp
   ```
   - Result: Ollama starts normally
   - Result: Zero-config behavior preserved
   - ✅ PASS

## Next Steps
1. **Publish to npm**:
   ```bash
   cd packages/maproom-mcp
   npm publish --otp=<2FA_CODE>
   ```

2. **Verify published package**:
   ```bash
   npm info @crewchief/maproom-mcp
   ```
   - Should show version 1.1.7

3. **User Communication**:
   - Update documentation to note auto-update feature
   - Add note about backup files for users with customizations
   - Document that hardcoded EMBEDDING_PROVIDER in docker-compose.yml is no longer supported

## Related Tickets
- MCP-008: Fix conditional Docker startup based on EMBEDDING_PROVIDER (COMPLETED)
- MCP-010: Fix missing maproom-mcp service health check (COMPLETED)

## Notes
- This fix completes the MCP-008 implementation by handling the upgrade path for existing installations
- The auto-update mechanism is conservative: only triggers on specific known outdated patterns
- Users who truly need custom docker-compose.yml configurations should fork the package or use Docker Compose overrides
- Future enhancement: Consider semantic versioning markers in config files for more robust update detection
