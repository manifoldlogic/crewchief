# Ticket: CFGVER-5003: Documentation Updates

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- documentation-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Update all user and developer documentation for the new version management system. Users need clear documentation for automatic updates and troubleshooting. Developers need updated architecture docs and JSDoc comments for the config-manager module.

## Background
Users will encounter the new version management system when they run `npx -y @crewchief/maproom-mcp@latest`. They need to understand:
- What's happening during automatic updates
- How to troubleshoot common issues
- What changed in this release

Developers need updated architecture documentation and comprehensive JSDoc comments to understand and maintain the config-manager module.

## Acceptance Criteria
- [ ] README.md updated with "Configuration Management" section
- [ ] Troubleshooting guide created for common issues
- [ ] Architecture docs reference new config-manager module
- [ ] JSDoc comments added to all public functions in config-manager.js
- [ ] Change log entry prepared for release
- [ ] All documentation technically accurate (verified by code-reviewer)

## Technical Requirements

**User-Facing Documentation:**

1. **README.md Update** (`packages/maproom-mcp/README.md`)
   Add "Configuration Management" section:
   - Explain automatic updates on CLI startup
   - Describe version tracking system
   - Link to troubleshooting guide
   - Show example update output

2. **Troubleshooting Guide** (Create: `packages/maproom-mcp/docs/TROUBLESHOOTING.md`)
   Sections:
   - Config update failed
   - Docker permission denied
   - Rollback instructions
   - Manual config reset
   - Common error messages and solutions

3. **Change Log** (`packages/maproom-mcp/CHANGELOG.md`)
   Add release entry:
   ```markdown
   ## [1.2.3] - 2024-11-XX

   ### Added
   - Automatic configuration version management
   - Config update detection on CLI startup
   - Backup and rollback mechanisms
   - Docker container management during updates
   - Clear progress messages for update operations

   ### Fixed
   - Config drift causing MCP connection failures (#123)
   - Stale cached configs after package updates

   ### Security
   - Path traversal prevention in config files
   - Command injection prevention in Docker operations
   - File permission hardening (0o600 for configs)
   ```

**Developer-Facing Documentation:**

1. **Module JSDoc** (`packages/maproom-mcp/src/config-manager.ts`)
   Add JSDoc comments for:
   - Module overview (purpose, architecture)
   - All public functions (parameters, return types, examples)
   - Complex internal functions (algorithm explanation)
   - Security considerations (path validation, permission handling)

2. **Architecture Reference**
   Ensure architecture docs reference the config-manager module:
   - Version tracking design
   - Update flow diagram
   - Error handling strategy
   - Security model

## Implementation Notes

**README.md Structure:**
Add section after "Installation":
```markdown
## Configuration Management

Maproom MCP automatically manages its configuration files to prevent drift between package versions.

### How It Works

When you run `npx -y @crewchief/maproom-mcp@latest`, the CLI:
1. Checks if cached configs need updating
2. Creates a backup of current configs
3. Updates configs to match package version
4. Manages Docker containers during update
5. Rolls back on any failure

### Troubleshooting

If you encounter issues during config updates, see [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md).
```

**Troubleshooting Guide Template:**
```markdown
# Troubleshooting Maproom MCP

## Config Update Failed

**Error:** "Config update failed: [reason]"

**Cause:** Update process encountered an error and rolled back.

**Solution:**
1. Check the error message for specific reason
2. Verify Docker is running (if Docker-related)
3. Check disk space: `df -h ~/.maproom-mcp`
4. Try manual reset: `rm -rf ~/.maproom-mcp`

## Docker Permission Denied

**Error:** "Docker permission denied"

**Cause:** User account doesn't have Docker access.

**Solution:**
```bash
# macOS
sudo dseditgroup -o edit -a $(whoami) -t user docker

# Linux
sudo usermod -aG docker $(whoami)
```

## Manual Config Reset

If automatic updates fail repeatedly:
```bash
# Stop containers
docker compose -f ~/.maproom-mcp/docker-compose.yml down

# Remove cached configs
rm -rf ~/.maproom-mcp

# Re-run CLI (will recreate configs)
npx -y @crewchief/maproom-mcp@latest
```
```

**JSDoc Example:**
```javascript
/**
 * Detects if cached config needs updating.
 *
 * Compares the package version in .maproom-version file with current package.json version.
 * Returns true if version file is missing, versions don't match, or file integrity fails.
 *
 * @param {string} cacheDir - Path to cache directory (default: ~/.maproom-mcp)
 * @returns {Promise<{needsUpdate: boolean, reason: string}>} Update decision with reason
 *
 * @example
 * const { needsUpdate, reason } = await detectUpdateNeeded('/path/to/cache');
 * if (needsUpdate) {
 *   console.log(`Update needed: ${reason}`);
 * }
 */
async function detectUpdateNeeded(cacheDir) {
  // Implementation...
}
```

## Dependencies
- All implementation tickets complete (Phase 1-4)

## Risk Assessment
- **Risk**: Documentation drift (docs don't match implementation)
  - **Mitigation**: Code-reviewer verifies technical accuracy against code

- **Risk**: Missing edge cases in troubleshooting guide
  - **Mitigation**: Gather common issues from testing phase

- **Risk**: JSDoc comments become outdated
  - **Mitigation**: Include JSDoc validation in code review checklist

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/README.md` (add Configuration Management section)
- **Create**: `packages/maproom-mcp/docs/TROUBLESHOOTING.md` (troubleshooting guide)
- **Modify**: `packages/maproom-mcp/CHANGELOG.md` (add release entry)
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add JSDoc comments)
