# Ticket: CFGVER-001: Implement version file tracking

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - manual testing complete

## Agents
- database-engineer

## Summary
Create simple version tracking using a `.version` file in the cache directory. This file contains a single line with the package version string.

## Background
We need to track what version of configs the user has cached. The simplest approach: store the package version in a plain text file.

## Acceptance Criteria
- [ ] Function `readVersion()` reads `~/.maproom-mcp/.version` file
- [ ] Returns version string (e.g., "1.2.0") or null if file doesn't exist
- [ ] Function `writeVersion(version)` writes version to `.version` file
- [ ] File created with 0o600 permissions (user read/write only)
- [ ] Cache directory created with 0o700 if doesn't exist

## Technical Requirements

**Module:** Create `packages/maproom-mcp/src/config-manager.ts`

**Implementation:**
```typescript
import fs from 'fs';
import path from 'path';
import os from 'os';

const CACHE_DIR = path.join(os.homedir(), '.maproom-mcp');
const VERSION_FILE = path.join(CACHE_DIR, '.version');

export function readVersion(): string | null {
  if (!fs.existsSync(VERSION_FILE)) {
    return null;
  }
  return fs.readFileSync(VERSION_FILE, 'utf-8').trim();
}

export function writeVersion(version: string): void {
  // Ensure cache directory exists
  if (!fs.existsSync(CACHE_DIR)) {
    fs.mkdirSync(CACHE_DIR, { recursive: true, mode: 0o700 });
  }

  // Write version file
  fs.writeFileSync(VERSION_FILE, version, { mode: 0o600 });
}
```

## Manual Testing

```bash
# Test 1: First run (no .version file)
node -e "const {readVersion} = require('./dist/config-manager.js'); console.log(readVersion())"
# Expected: null

# Test 2: Write version
node -e "const {writeVersion} = require('./dist/config-manager.js'); writeVersion('1.2.0')"
# Expected: No error

# Test 3: Read version
node -e "const {readVersion} = require('./dist/config-manager.js'); console.log(readVersion())"
# Expected: 1.2.0

# Test 4: Verify file permissions
ls -l ~/.maproom-mcp/.version
# Expected: -rw------- (0o600)
```

## Dependencies
None (first ticket)

## Files Affected
- **Create:** `packages/maproom-mcp/src/config-manager.ts`
- **Write:** `~/.maproom-mcp/.version`

## Estimated Time
2 hours
