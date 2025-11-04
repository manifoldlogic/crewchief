# Ticket: BINPKG-1904: Fix binary validation to skip cross-architecture execution tests

## Status
- [x] **Task completed** - acceptance criteria met (validation conditional fixed)
- [x] **Tests pass** - no code tests, workflow change only
- [x] **Verified** - by the verify-ticket agent

## Agents
- github-actions-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Fix the binary validation script in the GitHub Actions workflow to skip execution tests for cross-architecture binaries (linux-arm64 on x64 runner). Currently, validation fails when trying to execute linux-arm64 binary on ubuntu-latest (x64) runner, blocking npm publishing even though all builds succeeded.

## Background
This issue was discovered during BINPKG-1903 fix verification when all 4 platform builds succeeded for the first time. The validation step in workflow run #19053603342 failed with:

```
Validating linux-arm64...
✓ Binary exists
✓ Binary size valid (16278568 bytes)
✓ Execute permission set
ERROR: Binary failed to execute --version
```

The validation script incorrectly assumes all Linux binaries can execute on the ubuntu-latest runner (which is x64 architecture). However, linux-arm64 is cross-compiled for ARM architecture and cannot execute on x64 hardware.

This is a CRITICAL blocker that prevents:
- BINPKG-1901 (canary release test)
- BINPKG-1007 (npm publish)
- BINPKG-5002 (production release)

The current validation logic in `.github/workflows/build-and-publish-maproom-mcp.yml` (lines 244-297):
```bash
if [[ "$platform" == linux-* ]]; then
  if ! "$BINARY_PATH" --version >/dev/null 2>&1; then
    echo "ERROR: Binary failed to execute --version"
    exit 1
  fi
  echo "✓ Binary executes successfully"
else
  echo "⊘ Skipping execution test (macOS binary on Linux runner)"
fi
```

This logic needs to be updated to only test binaries that match the runner architecture (linux-x64).

## Acceptance Criteria
- [x] Validation script updated to only attempt execution test for linux-x64 binary (line 285 changed)
- [x] linux-arm64 execution test is skipped with clear messaging (updated skip message)
- [x] darwin-x64 and darwin-arm64 execution tests remain skipped (no change to behavior)
- [x] All other validations (file exists, size check, permissions) still run for all 4 platforms (preserved)
- [ ] Workflow succeeds when all 4 binaries are present and valid (pending GitHub Actions run)
- [ ] Dry-run workflow run completes successfully with all validations passing (pending GitHub Actions run)

## Technical Requirements
- Modify `.github/workflows/build-and-publish-maproom-mcp.yml` validation step
- Change execution test conditional from `platform == linux-*` to `platform == linux-x64`
- Update skip message to clarify "cross-platform or cross-architecture binary"
- Preserve all existing validation checks (exists, size, permissions) for all platforms
- Maintain clear console output showing which validations run vs skip for each platform

## Implementation Notes
This is a simple one-line fix to the conditional logic in the validation script. The corrected logic should be:

```bash
# Test execution (--version should work for Linux x64 binaries on ubuntu runner)
# Skip execution test for cross-architecture binaries
if [[ "$platform" == "linux-x64" ]]; then
  if ! "$BINARY_PATH" --version >/dev/null 2>&1; then
    echo "ERROR: Binary failed to execute --version"
    exit 1
  fi
  echo "✓ Binary executes successfully"
else
  echo "⊘ Skipping execution test (cross-platform or cross-architecture binary)"
fi
```

**Key changes:**
1. Change condition from `linux-*` (matches both linux-x64 and linux-arm64) to `linux-x64` (matches only x64)
2. Update skip message to be more accurate about the reason
3. This ensures only native-architecture binaries are tested for execution

**Validation sequence per platform:**
- **linux-x64**: exists ✓, size ✓, permissions ✓, execution ✓
- **linux-arm64**: exists ✓, size ✓, permissions ✓, execution ⊘ (skip - cross-arch)
- **darwin-x64**: exists ✓, size ✓, permissions ✓, execution ⊘ (skip - cross-platform)
- **darwin-arm64**: exists ✓, size ✓, permissions ✓, execution ⊘ (skip - cross-platform)

## Dependencies
- BINPKG-1903 (OpenSSL cross-compilation fix) - COMPLETED
- All 4 platform build jobs must succeed before validation runs

## Risk Assessment
- **Risk**: Reduced test coverage by skipping execution tests for cross-arch binaries
  - **Mitigation**: The linux-arm64 binary is built with the same Rust toolchain and code as linux-x64, just targeting a different architecture. File existence, size, and permissions checks still verify the binary was created properly. Future enhancement could use QEMU for cross-arch execution testing if needed.

- **Risk**: False sense of security if linux-x64 binary is valid but others are broken
  - **Mitigation**: The build process itself will fail if cross-compilation has issues. Size checks verify binaries are non-empty and reasonable. The canary release test (BINPKG-1901) will provide real-world validation when npm package is installed on different platforms.

- **Risk**: Missing actual runtime issues in cross-compiled binaries
  - **Mitigation**: This is acceptable for the validation stage. Real validation happens when users install the npm package on their target platforms. GitHub Actions runners don't provide ARM64 Linux runners in free tier, so cross-arch testing would require additional infrastructure.

## Files/Packages Affected
- `.github/workflows/build-and-publish-maproom-mcp.yml` (lines 244-297, specifically the validation script section)
