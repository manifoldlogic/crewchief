# Ticket: CFGVER-1901: Create unit tests for version management core logic

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive unit tests for core version management logic (version file creation, version comparison, file integrity verification). Target 80%+ code coverage to ensure bulletproof reliability of the config update detection system.

## Background
Core version management logic is critical infrastructure that determines when configs are updated. Bugs in this logic could cause:
- False negatives (missing updates, leaving users with stale configs)
- False positives (unnecessary updates, disrupting workflows)
- Data loss (corrupted version files)
- Security issues (undetected file tampering)

Comprehensive unit tests provide confidence that these functions work correctly in all scenarios, including edge cases.

Reference: `quality-strategy.md` lines 12-85: Test structure, coverage goals, and scenarios for version detection and file integrity.

## Acceptance Criteria
- [ ] Test file created at `packages/maproom-mcp/tests/config-manager.test.ts`
- [ ] Uses Vitest testing framework (installed via CFGVER-0001)
- [ ] Uses `memfs` for file system mocking (isolated, fast tests)
- [ ] Code coverage >= 80% for `packages/maproom-mcp/src/config-manager.ts`
- [ ] All happy path scenarios tested
- [ ] All error/edge case scenarios tested
- [ ] Tests are isolated (no shared state between tests)
- [ ] Tests run in < 1 second total

## Technical Requirements
**Testing Framework:**
- Vitest (already in project at root level)
- memfs for file system mocking
- No real file system access during tests

**Test Structure:**
```javascript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Volume } from 'memfs';

describe('config-manager', () => {
  beforeEach(() => {
    // Reset mocked file system before each test
    vol.reset();
  });

  describe('createVersionFile()', () => {
    // Tests for version file creation
  });

  describe('needsConfigUpdate()', () => {
    // Tests for version comparison
  });

  describe('verifyFileIntegrity()', () => {
    // Tests for integrity checking
  });
});
```

## Implementation Notes

### Test Scenarios (from `quality-strategy.md` lines 27-68)

**1. Version File Creation:**
- [ ] Creates version file with valid JSON schema
- [ ] Includes package_version from package.json
- [ ] Includes config_version matching package_version
- [ ] Includes last_updated timestamp in ISO 8601 format
- [ ] Computes correct SHA-256 hashes for all config files
- [ ] Includes file size and last_modified for each file
- [ ] Creates file with 0o600 permissions
- [ ] Creates cache directory with 0o700 permissions if missing
- [ ] Handles permission errors gracefully

**2. Version Detection (`needsConfigUpdate()`):**
- [ ] Missing version file → returns `{ needsUpdate: true, reason: 'first_run' }`
- [ ] Version mismatch (1.2.2 vs 1.2.3) → returns `{ needsUpdate: true, reason: 'version_mismatch', oldVersion: '1.2.2', newVersion: '1.2.3' }`
- [ ] Same version (1.2.3 vs 1.2.3) → returns `{ needsUpdate: false }`
- [ ] Corrupted version file (invalid JSON) → returns `{ needsUpdate: true, reason: 'first_run' }`
- [ ] Invalid version format → throws error or treats as corrupted
- [ ] Missing package.json → throws error

**3. File Integrity (`verifyFileIntegrity()`):**
- [ ] All files valid → returns `{ valid: true, corruptedFiles: [] }`
- [ ] Missing file → returns `{ valid: false, corruptedFiles: [{ filename: 'docker-compose.yml', reason: 'missing' }] }`
- [ ] Modified file (hash mismatch) → returns `{ valid: false, corruptedFiles: [{ filename: 'init.sql', reason: 'hash_mismatch' }] }`
- [ ] Multiple corrupted files → includes all in corruptedFiles array
- [ ] Symlink instead of regular file → returns reason: 'not_regular_file'
- [ ] Unreadable file (permissions) → treats as corrupted

**4. Hash Computation:**
- [ ] Computes correct SHA-256 hash for known content
- [ ] Returns hash in format "sha256:abc123..."
- [ ] Handles empty files correctly
- [ ] Handles large files (use stream if needed)
- [ ] Consistent hash for same content

**5. Version File Reading:**
- [ ] Reads and parses valid version file
- [ ] Returns null for missing version file
- [ ] Returns null for corrupted JSON
- [ ] Validates required fields (package_version, files)
- [ ] Returns null for invalid schema

### Mock Setup Example:
```javascript
import { vol } from 'memfs';

beforeEach(() => {
  vol.reset();

  // Mock cache directory with version file
  vol.fromJSON({
    '/home/user/.maproom-mcp/.maproom-version': JSON.stringify({
      package_version: '1.2.3',
      config_version: '1.2.3',
      last_updated: '2024-10-30T15:30:00.000Z',
      files: {
        'docker-compose.yml': {
          hash: 'sha256:abc123...',
          size: 2048,
          last_modified: '2024-10-30T15:30:00.000Z'
        }
      }
    }),
    '/home/user/.maproom-mcp/docker-compose.yml': 'version: "3.8"\nservices:\n  ...'
  });

  // Mock require() for package.json
  vi.mock('../package.json', () => ({
    version: '1.2.3'
  }));
});
```

### Coverage Goals:
- **Statements**: >= 80%
- **Branches**: >= 75% (cover main error paths)
- **Functions**: 100% (all exported functions tested)
- **Lines**: >= 80%

**Run coverage:**
```bash
cd packages/maproom-mcp
vitest run --coverage
```

## Dependencies
- **CFGVER-0001** - Vitest testing infrastructure must be installed first (CRITICAL)
- **CFGVER-1001** - Tests version file creation and hashing
- **CFGVER-1002** - Tests version comparison logic
- **CFGVER-1003** - Tests file integrity verification

## Risk Assessment
- **Risk**: Incomplete test coverage leaving bugs undetected
  - **Mitigation**: Target 80%+ coverage, review coverage report

- **Risk**: Tests coupled to file system (slow, flaky)
  - **Mitigation**: Use memfs for isolated in-memory file system

- **Risk**: Tests sharing state causing intermittent failures
  - **Mitigation**: Reset file system in beforeEach() hook

## Files/Packages Affected
- **Create**: `packages/maproom-mcp/tests/config-manager.test.ts`
- **Test**: `packages/maproom-mcp/src/config-manager.ts`
- **Dependencies**: Add `memfs` to devDependencies if not already present
