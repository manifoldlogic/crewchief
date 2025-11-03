# Ticket: BINPKG-2001: Create local binary validation script

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create `scripts/validate-binaries.js` that verifies all 4 platform binaries exist before npm publish. This is the local safety net that prevents developers from accidentally publishing incomplete packages.

## Background
Version 1.3.0 was published without linux-x64 binaries because there was no validation. This script runs automatically via prepublishOnly hook (BINPKG-2002) to block any publish attempt without complete binaries. It's the last line of defense against incomplete releases.

This is part of Phase 2 of the BINPKG project, which focuses on local development safety mechanisms that complement the CI/CD automation built in Phase 1.

## Acceptance Criteria
- [ ] Script exists at `scripts/validate-binaries.js`
- [ ] Checks all 4 platforms exist: linux-x64, linux-arm64, darwin-x64, darwin-arm64
- [ ] For each platform, verifies binary file exists at `packages/maproom-mcp/bin/<platform>/crewchief-maproom`
- [ ] Checks binary size is reasonable (>1MB to ensure not corrupted)
- [ ] Warns if binary is very large (>100MB)
- [ ] Prints clear error messages indicating which platform is missing
- [ ] Exits with code 0 if all valid, code 1 if any missing
- [ ] Provides helpful guidance: "Run GitHub Actions workflow to build binaries"
- [ ] Can be run manually: `node scripts/validate-binaries.js`

## Technical Requirements

### File Location
- Create: `scripts/validate-binaries.js` (Node.js CommonJS)

### Platform Configuration
- Required platforms array: `['linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64']`
- Base path: `packages/maproom-mcp/bin`
- Binary name: `crewchief-maproom`

### Size Validation
- Minimum: 1,000,000 bytes (1MB) - fail if smaller (indicates corruption or placeholder)
- Maximum: 100,000,000 bytes (100MB) - warn if larger (indicates potential issue)

### Output Format
- Success: `✓ linux-x64: 12.3MB`
- Failure: `❌ Missing binary for linux-x64`
- Size warning: `⚠️ darwin-arm64: 105.2MB (larger than expected)`
- Guidance: `💡 Run: gh workflow run build-and-publish-maproom-mcp.yml`

### Exit Codes
- Exit 0: All binaries valid
- Exit 1: Any binary missing or too small

## Implementation Notes

### Core Logic
```javascript
const fs = require('fs');
const path = require('path');

const PLATFORMS = ['linux-x64', 'linux-arm64', 'darwin-x64', 'darwin-arm64'];
const BIN_BASE = path.join(__dirname, '..', 'packages', 'maproom-mcp', 'bin');
const BINARY_NAME = 'crewchief-maproom';

const MIN_SIZE = 1_000_000;  // 1MB
const MAX_SIZE = 100_000_000;  // 100MB

function formatSize(bytes) {
  return (bytes / 1_000_000).toFixed(1) + 'MB';
}
```

### Validation Steps
1. Iterate through each platform
2. Construct path: `${BIN_BASE}/${platform}/${BINARY_NAME}`
3. Check existence with `fs.existsSync()`
4. Check size with `fs.statSync().size`
5. Print progress for each platform (success/fail)
6. Exit early on first failure for fast feedback

### Error Messages
- Missing binary: Print platform name, suggest workflow run
- Too small binary: Indicate corruption, suggest rebuild
- Too large binary: Warn but don't fail (might be legitimate)

### Development Tips
- Use clear emoji indicators (✓ ❌ ⚠️ 💡)
- Print paths for debugging (helps troubleshoot path issues)
- Add comments explaining size thresholds
- Format sizes in MB for readability
- Group output logically (all validations, then summary)

## Dependencies
**None** - This is a standalone validation script

## Risk Assessment

- **Risk**: Path calculation errors for bin directory structure
  - **Likelihood**: Medium
  - **Impact**: High (false negatives/positives)
  - **Mitigation**: Test with actual bin structure, print calculated paths for debugging, use path.join() for cross-platform compatibility

- **Risk**: Size thresholds too strict (false failures) or too loose (miss real issues)
  - **Likelihood**: Low
  - **Impact**: Medium
  - **Mitigation**: Based on actual binary sizes (~12MB), reasonable range (1MB-100MB), warn rather than fail for large binaries

- **Risk**: Script doesn't work on all platforms (Windows vs Unix)
  - **Likelihood**: Low
  - **Impact**: Low (developers on all platforms need validation)
  - **Mitigation**: Use Node.js built-in path utilities, test on multiple platforms

## Files/Packages Affected

### Files to Create
- `/workspace/scripts/validate-binaries.js` - Binary validation script

### Files to Reference (Read Only)
- `/workspace/packages/maproom-mcp/bin/` - Binary directory structure (to verify paths)

### Packages Affected
- `packages/maproom-mcp` - Target package for validation (no code changes)

## Estimated Effort
**1-2 hours** - Straightforward validation script

Breakdown:
- 30 min: Review binary structure and existing scripts
- 30 min: Implement core validation logic
- 20 min: Add size checks and error messages
- 20 min: Test with actual binaries (present and missing)
- 20 min: Polish output formatting and documentation

## Priority
**High** - Critical safety mechanism for Phase 2. Required before BINPKG-2002 (prepublishOnly hook).

## Related Tickets

### Blocks (must be completed before these can start)
- BINPKG-2002: Add prepublishOnly npm hook to run this script

### Related
- BINPKG-1006: CI binary validation (similar logic, different context)
- BINPKG-2901: Validation tests (will test this script)

### Sequence
This is ticket 1 of Phase 2 in the BINPKG project:
1. **BINPKG-2001** (this ticket) - Local validation script
2. BINPKG-2002 - npm prepublishOnly hook
3. BINPKG-2003-2004 - Release playbook and documentation

## Reference Documentation

### Planning Documents
- **Project plan**: `.agents/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 2)
- **Architecture**: `.agents/projects/BINPKG_binary-packaging/planning/architecture.md`

### Related Scripts
- `scripts/build-and-package.sh` - Manual build script (reference for path structure)

### External References
- **Node.js fs module**: https://nodejs.org/api/fs.html
- **Node.js path module**: https://nodejs.org/api/path.html
