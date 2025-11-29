# Ticket: CFGVER-1002: Implement version comparison logic for detecting updates

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
Implement version comparison logic to determine when cached configs need updating. This replaces brittle pattern-matching with explicit semver comparison between cached version and current package version.

## Background
The current pattern-based detection (`includes('EMBEDDING_PROVIDER: ollama')`) only catches one specific outdated pattern and missed the architectural change from local builds to Docker images. This caused connection failures for users with cached configs.

The new approach compares package versions explicitly:
- First run (no version file) → update needed
- Version mismatch (1.2.2 vs 1.2.3) → update needed
- Same version (1.2.3 vs 1.2.3) → no update needed

Reference: `analysis.md` lines 46-57: "Only detects ONE specific outdated pattern. Doesn't detect architectural changes (build → image). Doesn't detect new environment variables. Requires adding new pattern checks for each breaking change."

Reference: `architecture.md` lines 72-113: Update detection logic with version comparison.

## Acceptance Criteria
- [ ] Function `needsConfigUpdate()` accepts no parameters (reads version file internally)
- [ ] Returns object with structure: `{ needsUpdate: boolean, reason: string, oldVersion?: string, newVersion?: string }`
- [ ] Handles missing version file → returns `{ needsUpdate: true, reason: 'first_run' }`
- [ ] Handles version mismatch → returns `{ needsUpdate: true, reason: 'version_mismatch', oldVersion: '1.2.2', newVersion: '1.2.3' }`
- [ ] Handles same version → returns `{ needsUpdate: false }`
- [ ] Handles corrupted version file → treats as missing (returns `{ needsUpdate: true, reason: 'first_run' }`)
- [ ] Validates version format with regex: `/^\d+\.\d+\.\d+(-[a-z0-9.]+)?(\+[a-z0-9.]+)?$/i`

## Technical Requirements
- Use simple string comparison for semver versions (e.g., "1.2.3" !== "1.2.2")
- Read cached version from version file using `readVersionFile()` from CFGVER-1001
- Read current package version from `packages/maproom-mcp/package.json`
- Handle edge cases:
  - Missing version file
  - Corrupted version file (invalid JSON)
  - Invalid version format (not semver)
  - Missing package.json (should never happen, but handle gracefully)
- Return structured result object (not boolean) for detailed error messaging

## Implementation Notes
**Function Location:**
- Add to module: `packages/maproom-mcp/src/config-manager.ts`
- Export function: `needsConfigUpdate()`

**TypeScript Interfaces:**
```typescript
export interface UpdateCheckResult {
  needsUpdate: boolean;
  reason?: 'first_run' | 'version_mismatch';
  oldVersion?: string;
  newVersion?: string;
}
```

**Implementation Pattern:**
```typescript
export function needsConfigUpdate(): UpdateCheckResult {
  // Read current package version
  const packageJsonPath = path.join(__dirname, '../package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
  const PACKAGE_VERSION = packageJson.version;

  // Validate package version format
  if (!isValidVersion(PACKAGE_VERSION)) {
    throw new Error('Invalid package version format');
  }

  // Read cached version
  const versionData = readVersionFile();

  // First run - no version file
  if (!versionData) {
    return { needsUpdate: true, reason: 'first_run' };
  }

  // Compare versions
  if (versionData.package_version !== PACKAGE_VERSION) {
    return {
      needsUpdate: true,
      reason: 'version_mismatch',
      oldVersion: versionData.package_version,
      newVersion: PACKAGE_VERSION
    };
  }

  return { needsUpdate: false };
}
```

**Version Validation (from `security-review.md` lines 275-278):**
```typescript
function isValidVersion(version: string): boolean {
  return /^\d+\.\d+\.\d+(-[a-z0-9.]+)?(\+[a-z0-9.]+)?$/i.test(version);
}
```

**Error Handling:**
- Corrupted version file → treat as first run
- Invalid version format → throw error (fail fast)
- Missing package.json → throw error (critical failure)

## Dependencies
- **CFGVER-1001** - Requires `readVersionFile()` function

## Risk Assessment
- **Risk**: Invalid version formats causing comparison failures
  - **Mitigation**: Validate version format with regex before comparison

- **Risk**: Corrupted version file causing false positives
  - **Mitigation**: Treat corrupted files as missing (safe default: trigger update)

- **Risk**: Edge cases in semver comparison (pre-release, build metadata)
  - **Mitigation**: Use exact string comparison (sufficient for package.json versions)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `needsConfigUpdate()` function)
- **Read**: `packages/maproom-mcp/package.json` (current version)
- **Read**: `~/.maproom-mcp/.maproom-version` (cached version)
