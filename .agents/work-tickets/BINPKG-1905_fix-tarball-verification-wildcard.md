# Ticket: BINPKG-1905: Fix tarball verification to use specific filename instead of wildcard

## Status
- [x] **Task completed** - both cleanup and specific filename implemented
- [x] **Tests pass** - workflow change only, no code tests
- [ ] **Verified** - awaiting GitHub Actions workflow run
- [ ] **Committed**

## Agents
- github-actions-engineer
- verify-ticket
- commit-ticket

## Summary
Fix the tarball verification step in the GitHub Actions workflow to use the specific tarball filename instead of the `*.tgz` wildcard. Currently, verification fails when multiple .tgz files exist in the directory.

## Background
Discovered in workflow run #19053997770 after all three previous fixes (BINPKG-1902, BINPKG-1903, BINPKG-1904) were successfully implemented. All 4 platform builds succeeded, validation passed, and the tarball was created with all binaries included:

- darwin-arm64: 10.1MB ✓
- darwin-x64: 10.9MB ✓
- linux-arm64: 16.3MB ✓
- linux-x64: 17.7MB ✓

However, the "Verify tarball contains all binaries" step failed with:
```
Extracting tarball contents...
tar: crewchief-maproom-mcp-1.3.0.tgz: Not found in archive
tar: Exiting with failure status due to previous errors
```

**Root Cause**: When `*.tgz` wildcard expands to multiple files (e.g., from previous workflow runs that weren't cleaned up), tar interprets the command as trying to find files inside the archive rather than listing the archive contents.

## Acceptance Criteria
- [ ] Tarball verification uses specific filename from package.json version
- [ ] Verification step succeeds with tarball from successful npm pack
- [ ] All 4 binary paths are verified to exist in tarball
- [ ] Workflow completes successfully in dry-run mode
- [ ] Optional: Clean up old .tgz files before npm pack to prevent future issues

## Technical Requirements

### Current Problematic Code (lines 335-347)
```yaml
- name: Verify tarball contains all binaries
  working-directory: packages/maproom-mcp
  run: |
    echo "Extracting tarball contents..."
    tar -tzf *.tgz | grep bin/

    echo "Verifying all 4 binaries are present..."
    tar -tzf *.tgz | grep "package/bin/linux-x64/crewchief-maproom"
    tar -tzf *.tgz | grep "package/bin/linux-arm64/crewchief-maproom"
    tar -tzf *.tgz | grep "package/bin/darwin-x64/crewchief-maproom"
    tar -tzf *.tgz | grep "package/bin/darwin-arm64/crewchief-maproom"

    echo "✓ All binaries present in tarball"
```

### Solution 1: Use Specific Filename (Recommended)
```yaml
- name: Verify tarball contains all binaries
  working-directory: packages/maproom-mcp
  run: |
    # Get version from package.json
    VERSION=$(node -p "require('./package.json').version")
    TARBALL="crewchief-maproom-mcp-${VERSION}.tgz"

    echo "Extracting tarball contents from $TARBALL..."
    tar -tzf "$TARBALL" | grep bin/

    echo "Verifying all 4 binaries are present..."
    tar -tzf "$TARBALL" | grep "package/bin/linux-x64/crewchief-maproom"
    tar -tzf "$TARBALL" | grep "package/bin/linux-arm64/crewchief-maproom"
    tar -tzf "$TARBALL" | grep "package/bin/darwin-x64/crewchief-maproom"
    tar -tzf "$TARBALL" | grep "package/bin/darwin-arm64/crewchief-maproom"

    echo "✓ All binaries present in tarball"
```

### Solution 2: Clean Up Old Tarballs First
Add a cleanup step before npm pack:
```yaml
- name: Create npm tarball
  working-directory: packages/maproom-mcp
  run: |
    # Clean up old tarballs to avoid wildcard issues
    rm -f *.tgz
    npm pack
```

### Recommended Approach
Use **both solutions**:
1. Clean up old tarballs before npm pack (prevents accumulation)
2. Use specific filename in verification (more robust)

## Implementation Notes

The issue occurs because:
1. GitHub Actions runners may cache files between runs
2. If workflow fails and reruns, old .tgz files remain
3. `*.tgz` expands to: `crewchief-maproom-mcp-1.2.9.tgz crewchief-maproom-mcp-1.3.0.tgz`
4. Tar interprets this as: "list contents of first file and look for second file inside it"
5. Second file doesn't exist inside the archive, causing the error

This is a subtle shell globbing issue that only manifests when multiple matching files exist.

## Dependencies
- BINPKG-1902 (dead code fix) - COMPLETED
- BINPKG-1903 (vendored OpenSSL) - COMPLETED
- BINPKG-1904 (cross-arch validation) - COMPLETED

## Blocks
- BINPKG-1901 (canary release test)
- BINPKG-5002 (production release)

## Risk Assessment
- **Risk**: Very low - this is a straightforward script fix
- **Impact**: Unblocks the entire release pipeline

## Files to Modify
- `.github/workflows/build-and-publish-maproom-mcp.yml` (lines 330-347)

## Verification Commands
After implementing:
1. Trigger workflow with manual dispatch
2. Verify "Create npm tarball" step shows cleanup
3. Verify "Verify tarball contains all binaries" step uses specific filename
4. Verify all 4 binary paths are found in tarball
5. Verify workflow completes successfully

## Priority
**CRITICAL** - Last blocker for automated release pipeline. All builds succeed, all previous issues fixed, this is the final issue preventing success.
