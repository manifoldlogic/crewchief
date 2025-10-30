# Ticket: CFGVER-003: Implement config update with .env preservation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - manual testing complete

## Agents
- database-engineer

## Summary
Copy fresh config files from package to cache directory, while preserving user's custom `.env` file if it exists.

## Background
When package version changes, we need fresh configs. However, users may have customized their `.env` file with environment variables - we must preserve those customizations.

**Strategy:** Read `.env` to memory → delete everything → copy fresh → restore `.env`

## Acceptance Criteria
- [ ] Function `updateConfigs()` copies all configs from package to cache
- [ ] User `.env` file preserved if it exists
- [ ] User `.env` NOT created if it didn't exist
- [ ] Cache directory created if missing
- [ ] Version file updated with current package version
- [ ] Clear console output during update

## Technical Requirements

**Module:** Add to `packages/maproom-mcp/src/config-manager.ts`

**Implementation:**
```typescript
export function updateConfigs(): void {
  const PACKAGE_CONFIGS = path.join(__dirname, '../config');
  const userEnvPath = path.join(CACHE_DIR, '.env');

  // Step 1: Backup user .env if exists
  let userEnvContent: string | null = null;
  if (fs.existsSync(userEnvPath)) {
    userEnvContent = fs.readFileSync(userEnvPath, 'utf-8');
    console.log('  💾 Preserving user .env file...');
  }

  // Step 2: Delete old cache directory
  if (fs.existsSync(CACHE_DIR)) {
    fs.rmSync(CACHE_DIR, { recursive: true, force: true });
  }

  // Step 3: Copy fresh configs from package
  fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });
  fs.cpSync(PACKAGE_CONFIGS, CACHE_DIR, { recursive: true });
  console.log('  📋 Copied fresh configs from package...');

  // Step 4: Restore user .env if it existed
  if (userEnvContent !== null) {
    fs.writeFileSync(userEnvPath, userEnvContent, { mode: 0o600 });
    console.log('  ✅ Restored user .env file');
  }

  // Step 5: Write current version
  const packageJsonPath = path.join(__dirname, '../package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
  writeVersion(packageJson.version);
}
```

## Manual Testing

```bash
# Test 1: Update without user .env
rm -rf ~/.maproom-mcp
node -e "const {updateConfigs} = require('./dist/config-manager.js'); updateConfigs()"
ls -la ~/.maproom-mcp/
# Expected: All config files copied, no .env file

# Test 2: Update with user .env (preserve)
echo "CUSTOM_VAR=my_value" > ~/.maproom-mcp/.env
node -e "const {updateConfigs} = require('./dist/config-manager.js'); updateConfigs()"
cat ~/.maproom-mcp/.env
# Expected: CUSTOM_VAR=my_value (preserved)

# Test 3: Verify version written
cat ~/.maproom-mcp/.version
# Expected: Current package version (e.g., 1.2.0)

# Test 4: Verify all configs copied
ls ~/.maproom-mcp/
# Expected: docker-compose.yml, init.sql, Dockerfile.mcp-server, .version, (.env if existed)
```

## Error Handling

If update fails:
- Throw descriptive error
- User can manually delete `~/.maproom-mcp/` and re-run
- No partial state (either all succeeds or nothing)

## Dependencies
- CFGVER-001 (requires `writeVersion()` function)
- CFGVER-002 (called after `needsConfigUpdate()` returns true)

## Files Affected
- **Modify:** `packages/maproom-mcp/src/config-manager.ts`
- **Read:** `packages/maproom-mcp/config/*` (source configs)
- **Read:** `~/.maproom-mcp/.env` (optional, preserve if exists)
- **Delete:** `~/.maproom-mcp/` (entire directory)
- **Write:** `~/.maproom-mcp/*` (all fresh configs)
- **Write:** `~/.maproom-mcp/.version`

## Estimated Time
3-4 hours
