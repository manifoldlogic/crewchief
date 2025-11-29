# Ticket: CFGVER-4003: Add MAPROOM_CACHE_DIR environment variable for testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Add MAPROOM_CACHE_DIR environment variable support to enable isolated testing with custom cache directories. This allows integration tests to run without affecting the user's actual ~/.maproom-mcp/ installation.

## Background
Testing requires using a custom cache directory instead of ~/.maproom-mcp/ to avoid polluting the user's real installation. Without environment variable support, tests either:
1. Affect the user's actual configuration (dangerous)
2. Require mocking the entire filesystem (brittle)
3. Can't test real file operations (incomplete coverage)

MAPROOM_CACHE_DIR solves this by allowing tests to specify a temporary directory while preserving all real file operations.

**Important**: This is a testing-only feature, not intended for production use.

## Acceptance Criteria
- [ ] Reads MAPROOM_CACHE_DIR environment variable at module load time
- [ ] Falls back to ~/.maproom-mcp/ if MAPROOM_CACHE_DIR not set
- [ ] All file operations use configured cache directory consistently
- [ ] Validates cache directory is writable on first use
- [ ] Documents environment variable in code comments as testing-only
- [ ] Integration tests set MAPROOM_CACHE_DIR to temporary directory

## Technical Requirements
- Check `process.env.MAPROOM_CACHE_DIR` when defining CACHE_DIR constant
- Use `path.resolve()` to handle relative paths in environment variable
- Validate cache directory exists and is writable before use
- Create cache directory with 0o700 permissions if missing
- Don't expose environment variable in user-facing documentation (internal testing only)

## Implementation Notes

**Module-Level Configuration:**
```javascript
// config-manager.js
const os = require('os');
const path = require('path');
const fs = require('fs');

// Use environment variable for testing, default to user's home directory
const CACHE_DIR = process.env.MAPROOM_CACHE_DIR ||
  path.join(os.homedir(), '.maproom-mcp');

// Validate and create cache directory
function ensureCacheDir() {
  // Create directory if missing
  if (!fs.existsSync(CACHE_DIR)) {
    fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });
  }

  // Test write permission
  const testFile = path.join(CACHE_DIR, '.write-test');
  try {
    fs.writeFileSync(testFile, 'test', { mode: 0o600 });
    fs.unlinkSync(testFile);
  } catch (error) {
    throw new Error(`Cache directory not writable: ${CACHE_DIR} - ${error.message}`);
  }
}

// Call on module load to fail fast
ensureCacheDir();
```

**Integration Test Usage:**
```javascript
// In integration tests
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'maproom-test-'));
process.env.MAPROOM_CACHE_DIR = tmpDir;

// Now require config-manager - it will use tmpDir
const { needsConfigUpdate, updateConfigs } = require('../src/config-manager');

// ... test code ...

// Cleanup
fs.rmSync(tmpDir, { recursive: true });
delete process.env.MAPROOM_CACHE_DIR;
```

**Security Considerations:**
- Validate path doesn't escape to system directories
- Ensure permissions are restrictive (0o700 for dir, 0o600 for files)
- Document that this is for testing only (not production)
- Don't document in user-facing README (keep internal)

**Edge Cases:**
- Relative paths: Resolve with path.resolve()
- Non-existent directory: Create with recursive: true
- Permission denied: Throw clear error message
- Disk space low: Let system error bubble up (can't prevent)

## Dependencies
- **CFGVER-1001**: CACHE_DIR constant must exist (will be modified to support env var)

## Risk Assessment
- **Risk**: Production users setting wrong directory via environment variable
  - **Mitigation**: Document as testing-only in code comments, don't mention in README
  - **Impact**: Low - environment variables are developer-focused

- **Risk**: Permission issues in custom directory
  - **Mitigation**: Validate writability early with ensureCacheDir()
  - **Impact**: Clear error message guides debugging

- **Risk**: Tests accidentally using production cache directory
  - **Mitigation**: Tests must explicitly set MAPROOM_CACHE_DIR or fail
  - **Impact**: Integration tests will enforce this pattern

- **Risk**: Path traversal via environment variable
  - **Mitigation**: Use path.resolve() to normalize, validate before use
  - **Impact**: Low - only affects test environment

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (CACHE_DIR constant)
- **Modify**: Integration test files to set MAPROOM_CACHE_DIR
- **No user-facing documentation** (testing-only feature)
