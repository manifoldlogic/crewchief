# Config Version Management - Quality Strategy

## Testing Philosophy

This feature is **critical path** - if config management breaks, users can't connect to the MCP server. Our testing strategy balances pragmatism with confidence:

1. **Core Logic Must Be Bulletproof** - Version comparison, file integrity, update detection
2. **Happy Path Must Always Work** - First run and version updates are mission-critical
3. **Failure Modes Must Be Safe** - Rollback and error handling prevent data loss
4. **Edge Cases Covered Pragmatically** - Test the important ones, document the rest

## Test Coverage Goals

### What We MUST Test (High Priority)

These tests prevent user-facing failures and must pass before shipping:

1. **Version Detection Logic**
   - Detects when config needs update
   - Handles missing version file (first run)
   - Handles version file with older version
   - Handles version file with same version (skip update)

2. **File Integrity Checking**
   - Detects missing files
   - Detects corrupted files (hash mismatch)
   - Handles missing hash in version file

3. **Update Process**
   - Copies all required files
   - Creates version file with correct data
   - Preserves user .env file
   - Updates docker-compose.yml with current version

4. **Backup and Rollback**
   - Creates backup before update
   - Rollback restores working config
   - Cleanup removes old backups

5. **Docker Integration**
   - Stops running containers before update
   - Cleans up old volumes
   - Handles docker not running

### What We SHOULD Test (Medium Priority)

These add confidence but aren't strictly necessary for MVP:

1. **Error Recovery**
   - Update fails mid-process, rollback succeeds
   - Backup directory full (disk space)
   - Permission denied on cache directory

2. **Edge Cases**
   - Corrupted version file (invalid JSON)
   - Partial backups (some files missing)
   - Concurrent updates (two terminals running npx simultaneously)

### What We CAN SKIP (Low Priority)

These are theoretical or extremely rare - document but don't test:

1. **Network Failures** - npm handles package download, not our concern
2. **OS-Specific Edge Cases** - Windows vs macOS path differences (document known issues)
3. **Performance Benchmarks** - Config updates are infrequent, sub-second is good enough

## Test Structure

### Unit Tests (packages/maproom-mcp/tests/config-manager.test.js)

Fast, isolated tests for core logic:

```javascript
describe('Version Detection', () => {
  test('detects missing version file as needing update', () => {
    // Setup: No version file exists
    // Assert: needsUpdate() returns true with reason 'first_run'
  });

  test('detects version mismatch as needing update', () => {
    // Setup: Version file has 1.2.2, package is 1.2.3
    // Assert: needsUpdate() returns true with reason 'version_mismatch'
  });

  test('skips update when versions match', () => {
    // Setup: Version file has 1.2.3, package is 1.2.3
    // Assert: needsUpdate() returns false
  });
});

describe('File Integrity', () => {
  test('detects missing file', () => {
    // Setup: Version file lists docker-compose.yml, but file is missing
    // Assert: verifyIntegrity() returns invalid with reason 'missing'
  });

  test('detects corrupted file via hash mismatch', () => {
    // Setup: Modify docker-compose.yml, hash no longer matches
    // Assert: verifyIntegrity() returns invalid with reason 'hash_mismatch'
  });

  test('passes when all files valid', () => {
    // Setup: All files exist and hashes match
    // Assert: verifyIntegrity() returns valid
  });
});

describe('Backup Strategy', () => {
  test('creates backup with all config files', () => {
    // Setup: Create config files in cache dir
    // Act: Run backupConfigs()
    // Assert: All files copied to backup directory
  });

  test('keeps only last 5 backups', () => {
    // Setup: Create 10 backups
    // Act: Run cleanupOldBackups()
    // Assert: Only 5 most recent backups remain
  });
});
```

**Target:** 80% code coverage of config-manager module

### Integration Tests (packages/maproom-mcp/tests/integration/update-flow.test.js)

Test the complete update flow with real files:

```javascript
describe('Config Update Flow', () => {
  let testCacheDir;

  beforeEach(() => {
    // Setup: Create temporary cache directory
    testCacheDir = path.join(os.tmpdir(), `maproom-test-${Date.now()}`);
    fs.mkdirSync(testCacheDir, { recursive: true });
  });

  afterEach(() => {
    // Cleanup: Remove temporary cache directory
    fs.rmSync(testCacheDir, { recursive: true, force: true });
  });

  test('first run creates all config files', async () => {
    // Act: Run update process with empty cache dir
    await updateConfigs({ cacheDir: testCacheDir });

    // Assert:
    // - docker-compose.yml exists
    // - init.sql exists
    // - .maproom-version exists with correct version
    // - All file hashes in version file match actual files
  });

  test('version update replaces config files', async () => {
    // Setup:
    // 1. Create old config with version 1.2.2
    // 2. Modify docker-compose.yml to simulate old version

    // Act: Run update process with version 1.2.3
    await updateConfigs({ cacheDir: testCacheDir });

    // Assert:
    // - docker-compose.yml has new version in header
    // - .maproom-version has new version
    // - Backup exists with old config
  });

  test('preserves user .env file during update', async () => {
    // Setup:
    // 1. Create config with version 1.2.2
    // 2. Create user .env file with custom values

    // Act: Run update process
    await updateConfigs({ cacheDir: testCacheDir });

    // Assert:
    // - .env file still exists
    // - .env file contents unchanged
  });
});
```

**Target:** All critical paths covered (first run, update, rollback)

### Manual Testing Checklist

Before release, manually verify:

- [ ] First run on clean system (no ~/.maproom-mcp/)
- [ ] Update from previous version (1.2.2 → 1.2.3)
- [ ] Update with running containers (stops cleanly)
- [ ] Update with user .env file (preserves)
- [ ] Rollback works after failed update
- [ ] Error messages are clear and actionable
- [ ] Works on macOS
- [ ] Works on Linux (devcontainer)

## Test Data

### Mock Version Files

**Version file for 1.2.2:**
```json
{
  "package_version": "1.2.2",
  "config_version": "1.2.2",
  "last_updated": "2024-10-29T10:00:00Z",
  "files": {
    "docker-compose.yml": {
      "hash": "sha256:abc123...",
      "size": 2048,
      "last_modified": "2024-10-29T10:00:00Z"
    }
  }
}
```

**Corrupted version file (missing hash):**
```json
{
  "package_version": "1.2.3",
  "files": {
    "docker-compose.yml": {
      "size": 2048
    }
  }
}
```

### Test Fixtures

Create fixtures for common scenarios:

```
tests/fixtures/
├── configs/
│   ├── v1.2.2/
│   │   ├── docker-compose.yml
│   │   ├── init.sql
│   │   └── .maproom-version
│   └── v1.2.3/
│       ├── docker-compose.yml
│       ├── init.sql
│       └── .maproom-version
└── user-files/
    └── .env.example
```

## Continuous Integration

### GitHub Actions Workflow

Add test job to existing workflow:

```yaml
# .github/workflows/test-config-manager.yml
name: Config Manager Tests

on:
  pull_request:
    paths:
      - 'packages/maproom-mcp/bin/cli.cjs'
      - 'packages/maproom-mcp/src/config-manager.js'
      - 'packages/maproom-mcp/tests/**'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: cd packages/maproom-mcp && npm install

      - name: Run unit tests
        run: cd packages/maproom-mcp && npm test

      - name: Run integration tests
        run: cd packages/maproom-mcp && npm run test:integration
```

## Testing Tools

### Test Framework

Use **Vitest** (already in project):
- Fast
- Built-in mocking
- TypeScript support
- Watch mode for development

### Assertion Library

Use **Vitest's built-in assertions**:
```javascript
import { describe, test, expect } from 'vitest';
```

### File System Mocking

Use **memfs** for unit tests (isolated, fast):
```javascript
import { vol } from 'memfs';

beforeEach(() => {
  vol.reset();
});

test('creates version file', () => {
  vol.fromJSON({
    '/tmp/cache/docker-compose.yml': 'version: "3.8"',
  });
  // Test uses vol instead of real fs
});
```

Use **real fs** for integration tests (confidence in actual behavior).

## Acceptance Criteria

### For MVP Release

All of these must be verified before shipping:

1. ✅ **First run creates config correctly**
   - All files present
   - Version file accurate
   - Containers can start

2. ✅ **Updates detect version mismatch**
   - Old config triggers update
   - New config is copied
   - Backup is created

3. ✅ **Rollback works on failure**
   - Simulated failure triggers rollback
   - Original config restored
   - Containers can still start

4. ✅ **User .env preserved**
   - Create user .env
   - Run update
   - .env unchanged

5. ✅ **Error messages are clear**
   - Docker not running: Clear message with command
   - Permission denied: Clear message with fix
   - Corrupted backup: Clear message with recovery steps

### Test Metrics

- **Unit tests:** 80%+ coverage
- **Integration tests:** All critical paths covered
- **Manual testing:** All checklist items verified
- **CI passing:** All tests green on main branch

## Known Limitations

Document but don't test these edge cases:

1. **Concurrent Updates** - Two terminals running npx simultaneously
   - **Risk:** Low (users rarely do this)
   - **Mitigation:** Document that it's not supported
   - **Future:** Add lock file if needed

2. **Disk Full During Update** - No space for backup
   - **Risk:** Low (config files are small)
   - **Mitigation:** Error message mentions disk space
   - **Future:** Check available space before update

3. **Corrupted Backup** - Backup itself is corrupted
   - **Risk:** Very low (backups are fresh copies)
   - **Mitigation:** Create backup before stopping containers
   - **Future:** Verify backup integrity after creation

## Testing Schedule

### During Development

- Run unit tests on every code change (watch mode)
- Run integration tests before committing
- Manual smoke test after significant changes

### Before PR

- All tests passing
- Code coverage meets target
- Manual testing checklist complete

### Before Release

- Full manual testing on macOS and Linux
- Test with actual npm package (not just local)
- Verify error messages and user experience

## Success Criteria

The testing strategy succeeds if:

1. **Zero user-reported config drift issues** after release
2. **Clear error messages** lead to quick user resolution
3. **Rollback mechanism** prevents data loss in failures
4. **CI catches regressions** before they reach main

This strategy provides confidence without over-testing. Focus on the happy path and critical failure modes, document the rest.
