# Config Version Management - Simplified Implementation Plan

## Overview

Implement simple version tracking to prevent config drift. Focus on solving the immediate problem quickly, iterate if needed.

## Implementation Strategy

### Core Principle
**KISS (Keep It Simple, Stupid)**: Detect version changes, copy fresh configs, preserve user `.env` file.

### What We're Building

```typescript
// Version file: ~/.maproom-mcp/.version
1.2.0

// On CLI startup:
1. Read cached version from .version file
2. Compare to package.json version
3. If different: copy all configs from package, preserve .env
4. Write new version to .version file
```

## Tickets (4 total, ~1-2 days)

### CFGVER-001: Version File Schema (2 hours)
**Goal**: Track package version in cache directory

**Implementation**:
- Location: `~/.maproom-mcp/.version`
- Content: Single line with version string (e.g., "1.2.0")
- Functions: `readVersion()`, `writeVersion(version)`

**No need for**:
- JSON schema
- File hashes
- Timestamps
- Metadata

---

### CFGVER-002: Version Comparison (2 hours)
**Goal**: Detect when cached configs are stale

**Implementation**:
```typescript
function needsConfigUpdate(): boolean {
  const currentVersion = require('../package.json').version;
  const versionFile = path.join(CACHE_DIR, '.version');

  if (!fs.existsSync(versionFile)) return true; // First run

  const cachedVersion = fs.readFileSync(versionFile, 'utf-8').trim();
  return currentVersion !== cachedVersion; // Version mismatch
}
```

**Returns**: Simple boolean (true = needs update)

---

### CFGVER-003: Config Update with .env Preservation (3-4 hours)
**Goal**: Copy fresh configs, preserve user customizations

**Implementation**:
```typescript
function updateConfigs() {
  const CACHE_DIR = path.join(os.homedir(), '.maproom-mcp');
  const PACKAGE_CONFIGS = path.join(__dirname, '../config');

  // Backup user .env if exists
  const userEnv = path.join(CACHE_DIR, '.env');
  let envBackup = null;
  if (fs.existsSync(userEnv)) {
    envBackup = fs.readFileSync(userEnv, 'utf-8');
  }

  // Copy all configs from package
  fs.rmSync(CACHE_DIR, { recursive: true, force: true });
  fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });
  fs.cpSync(PACKAGE_CONFIGS, CACHE_DIR, { recursive: true });

  // Restore user .env if existed
  if (envBackup) {
    fs.writeFileSync(userEnv, envBackup, { mode: 0o600 });
  }

  // Write new version
  const currentVersion = require('../package.json').version;
  fs.writeFileSync(path.join(CACHE_DIR, '.version'), currentVersion);
}
```

**Key points**:
- Delete everything EXCEPT user `.env` contents (in memory)
- Copy fresh from package
- Restore user `.env` if it existed
- No backup directory (if fails, user re-runs npx)

---

### CFGVER-004: CLI Integration (2-3 hours)
**Goal**: Check version on every CLI startup

**Implementation** (`bin/cli.cjs`):
```javascript
async function main() {
  // Check for config updates
  if (needsConfigUpdate()) {
    console.log('📦 Updating Maproom MCP configs...');
    try {
      updateConfigs();
      console.log('✅ Configs updated successfully');
    } catch (error) {
      console.error('❌ Config update failed:', error.message);
      console.error('💡 Try deleting ~/.maproom-mcp/ and re-running');
      process.exit(1);
    }
  }

  // Continue with normal CLI startup...
}
```

**User experience**:
- Seamless: Update happens automatically
- Fast: Only on version change (not every run)
- Clear: Progress messages
- Safe fallback: Manual fix instructions if fails

---

## Timeline

| Ticket | Hours | Cumulative |
|--------|-------|------------|
| CFGVER-001 | 2 | 2 |
| CFGVER-002 | 2 | 4 |
| CFGVER-003 | 3-4 | 7-8 |
| CFGVER-004 | 2-3 | 9-11 |
| **Total** | **9-11 hours** | **1-2 days** |

## Testing Strategy

**Minimal testing approach**:
- Manual testing: First run, version update, .env preservation
- No unit tests initially (add if bugs appear)
- Real-world validation: Ship and monitor for issues

**Test scenarios**:
1. First run (no .version file) → creates configs
2. Version change (1.1.12 → 1.2.0) → updates configs
3. User has custom .env → preserved after update
4. Update fails → clear error message with recovery steps

## Risk Assessment

**Low Risk Approach**:
- Worst case: Update fails, user deletes `~/.maproom-mcp/` and re-runs
- No data loss: User `.env` preserved in memory during update
- No breaking changes: New configs are known-good from package

**What if things go wrong?**
- Update fails mid-process → User re-runs `npx` command
- .env lost → User recreates (rare, only if write fails)
- Containers broken → Restart with `docker compose up -d`

**Recovery is simple**: Delete cache directory, re-run npx.

## Success Metrics

- Zero config drift incidents after shipping
- Users don't notice the update (seamless)
- No support tickets about stale configs

## Future Enhancements

**If we need more safety later**, implement from comprehensive plan:
- Backup before update (rollback on failure)
- File integrity verification (detect corruption)
- Docker container cleanup (stop before update)
- Comprehensive test coverage (80%+)

**Archive location**: `.agents/archive/projects/CFGVER_config-version-management-comprehensive/`

## Decision: Ship Simple, Iterate

**Philosophy**: Solve the immediate problem (config drift) with minimal code. Add complexity only if real issues emerge.

**Bet**: The simple approach will work fine for 95% of cases. The 5% edge cases can be handled with clear error messages and manual recovery.
