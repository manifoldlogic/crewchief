# Ticket: CFGVER-2002: Implement config file update from package template

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement logic to copy new config files from the npm package template directory to the cache directory after successful backup. This replaces outdated configs with current versions while preserving user customizations.

## Background
After successfully creating a backup of existing configs, we need to copy new config files from the npm package template directory (`packages/maproom-mcp/config/`) to the cache directory (`~/.maproom-mcp/`). This step must preserve the user's `.env` file (which contains custom environment variables) while updating all managed config files.

This is step 3 in the safe update process (after backup and stopping containers).

Reference: `architecture.md` lines 114-169 for the complete update process with config copying as step 3.

## Acceptance Criteria
- [ ] Function `copyNewConfigs(packageVersion)` copies template files from package to cache directory
- [ ] Files copied: docker-compose.yml, init.sql, Dockerfile.mcp-server
- [ ] User .env file is preserved (NOT overwritten if it exists)
- [ ] Version file is updated with new package version and file hashes
- [ ] All copied files have permissions set to 0o600
- [ ] Function returns success status with updated version
- [ ] Function throws descriptive error if any copy operation fails

## Technical Requirements
- Template directory location: `packages/maproom-mcp/config/`
- Destination directory: `~/.maproom-mcp/`
- Files to copy from template:
  * docker-compose.yml (must update version header comment)
  * init.sql
  * Dockerfile.mcp-server
- Preserve existing: .env (user's environment variables)
- Use `fs.promises.copyFile()` for async file copying
- Use `fs.lstatSync()` to verify source files are regular files (not symlinks)
- After copying all files, compute hashes and update version file
- Set file permissions to 0o600 immediately after copying

## Implementation Notes
**Module Location:**
- Modify: `packages/maproom-mcp/src/config-manager.ts`
- Add function: `copyNewConfigs(packageVersion)`

**Update Flow:**
1. Validate template directory exists and contains required files
2. For each template file:
   - Verify source file is regular file (not symlink)
   - Copy to cache directory
   - Set permissions to 0o600
3. Check if .env exists in cache directory:
   - If exists: skip copying (preserve user file)
   - If not exists: do nothing (.env is optional)
4. Compute hashes for all newly copied files
5. Create updated version file with:
   - New package version
   - Current timestamp
   - File hashes
6. Return success status with version

**Version Header Update:**
The docker-compose.yml template should include a version header comment:
```yaml
# Maproom MCP Configuration - Version 1.2.3
# Generated: 2024-10-30T15:30:00.000Z
```

**Error Handling:**
- If template directory missing → throw error with clear message
- If required template file missing → throw error listing missing files
- If copy fails → throw error, rollback will restore backup
- If permission denied → throw error with permissions message

**Security Considerations:**
- Don't follow symlinks when copying (verify with lstatSync)
- Validate destination paths stay within cache directory
- Set restrictive permissions (0o600) immediately after creation
- Reference: `security-review.md` lines 188-236 for update security requirements

## Dependencies
- CFGVER-1001 (needs version file functions: `computeFileHash()`, `writeVersionFile()`)
- CFGVER-2001 (backup must complete successfully before update)

## Risk Assessment
- **Risk**: Overwriting user .env file - loses custom environment variables
  - **Mitigation**: Check if .env exists before copying, skip if present
  - **Severity**: High (data loss of user customizations)

- **Risk**: Partial update - some files copied, then failure
  - **Mitigation**: Rollback mechanism restores backup on any error
  - **Severity**: Medium (handled by rollback)

- **Risk**: Permission errors - cannot write to cache directory
  - **Mitigation**: Handle gracefully, provide clear error message, rollback
  - **Severity**: Medium (no data loss, clear recovery path)

- **Risk**: Symlink attacks - template contains symlinks
  - **Mitigation**: Verify source files with lstatSync before copying
  - **Severity**: High (could expose system files)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `copyNewConfigs()` function)
- **Read**: Template files in `packages/maproom-mcp/config/` (docker-compose.yml, init.sql, Dockerfile.mcp-server)
- **Write**: Config files in `~/.maproom-mcp/` (docker-compose.yml, init.sql, Dockerfile.mcp-server, .maproom-version)
- **Preserve**: `~/.maproom-mcp/.env` (user file, do not overwrite)
