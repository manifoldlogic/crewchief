# Ticket: CFGVER-002: Implement version comparison logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - manual testing complete

## Agents
- database-engineer

## Summary
Detect when cached configs are stale by comparing the cached version (from `.version` file) to the current package version (from `package.json`).

## Background
On CLI startup, we need to know: "Do the cached configs need updating?" This happens when:
1. First run (no `.version` file)
2. Version mismatch (cached version ≠ package version)

## Acceptance Criteria
- [ ] Function `needsConfigUpdate()` returns boolean
- [ ] Returns `true` when `.version` file missing (first run)
- [ ] Returns `true` when cached version ≠ package version
- [ ] Returns `false` when cached version = package version
- [ ] Reads package version from `package.json` dynamically

## Technical Requirements

**Module:** Add to `packages/maproom-mcp/src/config-manager.ts`

**Implementation:**
```typescript
export function needsConfigUpdate(): boolean {
  // Read current package version
  const packageJsonPath = path.join(__dirname, '../package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
  const currentVersion = packageJson.version;

  // Read cached version
  const cachedVersion = readVersion();

  // First run - no version file
  if (!cachedVersion) {
    return true;
  }

  // Version mismatch
  return cachedVersion !== currentVersion;
}
```

## Manual Testing

```bash
# Test 1: First run (no .version file)
rm ~/.maproom-mcp/.version
node -e "const {needsConfigUpdate} = require('./dist/config-manager.js'); console.log(needsConfigUpdate())"
# Expected: true

# Test 2: Write matching version
node -e "const {writeVersion} = require('./dist/config-manager.js'); const pkg = require('./package.json'); writeVersion(pkg.version)"
node -e "const {needsConfigUpdate} = require('./dist/config-manager.js'); console.log(needsConfigUpdate())"
# Expected: false

# Test 3: Write mismatched version
node -e "const {writeVersion} = require('./dist/config-manager.js'); writeVersion('0.0.1')"
node -e "const {needsConfigUpdate} = require('./dist/config-manager.js'); console.log(needsConfigUpdate())"
# Expected: true
```

## Dependencies
- CFGVER-001 (requires `readVersion()` function)

## Files Affected
- **Modify:** `packages/maproom-mcp/src/config-manager.ts`
- **Read:** `packages/maproom-mcp/package.json`
- **Read:** `~/.maproom-mcp/.version`

## Estimated Time
2 hours
