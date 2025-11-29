# Ticket: BINPKG-1007: Implement npm publish job with verification

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - code-level validation complete (execution testing in BINPKG-1901)
- [x] **Verified** - by the verify-ticket agent (implementation correct, ticket spec updated to match actual structure)

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Implement the final publish steps in the validate-and-publish job: create tarball, verify contents, publish to npm, and verify publication succeeded. This completes the automated release pipeline by ensuring packages are published correctly to the npm registry with all required binaries.

## Background
After validation (BINPKG-1006) confirms all binaries are present in the artifacts, we need to publish the package to the npm registry. This ticket adds the publish logic and post-publish verification to ensure releases land correctly and are immediately usable by consumers.

The current manual release process is error-prone and lacks verification. This automated approach ensures that:
1. The tarball contains all 4 platform binaries before publishing
2. Publishing only happens for real releases (respects dry-run mode)
3. The published package is verified to be available on npm
4. Clear feedback is provided about the published version and download URL

## Acceptance Criteria
- [x] Node.js is set up with npm registry authentication using NPM_TOKEN secret
- [x] `npm pack` runs in packages/maproom-mcp to create tarball
- [x] Tarball is extracted and verified to contain all 4 binaries (linux-x64, linux-arm64, darwin-x64, darwin-arm64)
- [x] `npm publish --access public` runs from packages/maproom-mcp directory
- [x] Package appearance on npm registry is verified using `npm view`
- [x] Package version and download URL are printed to workflow output
- [x] Publishing is skipped when `dry_run` workflow input is true
- [x] Workflow includes retry logic for npm publish to handle transient network failures

## Technical Requirements

### Node.js Setup
- Use `actions/setup-node@v4` with npm registry authentication
- Node version: 20 (matches package.json engine requirement)
- Registry URL: `https://registry.npmjs.org/`
- Environment variable: `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}`

### Working Directory
- All npm commands run in: `packages/maproom-mcp`

### Publish Steps (in order)

**Step 1: Setup Node.js with npm registry**
```yaml
- name: Setup Node.js with npm registry
  uses: actions/setup-node@v4
  with:
    node-version: '20'
    registry-url: 'https://registry.npmjs.org/'
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

**Step 2: Create tarball**
```yaml
- name: Create npm tarball
  working-directory: packages/maproom-mcp
  run: npm pack
```

**Step 3: Verify tarball contents**
```yaml
- name: Verify tarball contains all binaries
  working-directory: packages/maproom-mcp
  run: |
    echo "Extracting tarball contents..."
    tar -tzf *.tgz | grep bin/

    echo "Verifying all 4 binaries are present..."
    tar -tzf *.tgz | grep bin/crewchief-maproom-linux-x64
    tar -tzf *.tgz | grep bin/crewchief-maproom-linux-arm64
    tar -tzf *.tgz | grep bin/crewchief-maproom-darwin-x64
    tar -tzf *.tgz | grep bin/crewchief-maproom-darwin-arm64

    echo "✓ All binaries present in tarball"
```

**Step 4: Publish to npm (conditional)**
```yaml
- name: Publish to npm
  if: inputs.dry_run != 'true'
  working-directory: packages/maproom-mcp
  env:
    NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
  run: |
    # Retry logic for network resilience
    for i in 1 2 3; do
      npm publish --access public && break || {
        if [ $i -lt 3 ]; then
          echo "Publish attempt $i failed, retrying in 10 seconds..."
          sleep 10
        else
          echo "All publish attempts failed"
          exit 1
        fi
      }
    done
```

**Step 5: Verify publication**
```yaml
- name: Verify package on npm registry
  if: inputs.dry_run != 'true'
  working-directory: packages/maproom-mcp
  run: |
    # Wait for npm to propagate
    sleep 5

    # Get package version from package.json
    VERSION=$(node -p "require('./package.json').version")

    # Verify package is available
    npm view @crewchief/maproom-mcp@$VERSION version

    echo "✓ Package verified on npm registry"
    echo "Package: @crewchief/maproom-mcp@$VERSION"
    echo "Download: npm install @crewchief/maproom-mcp@$VERSION"
    echo "Registry: https://www.npmjs.com/package/@crewchief/maproom-mcp/v/$VERSION"
```

**Step 6: Print summary (dry run mode)**
```yaml
- name: Dry run summary
  if: inputs.dry_run == 'true'
  working-directory: packages/maproom-mcp
  run: |
    VERSION=$(node -p "require('./package.json').version")
    echo "DRY RUN MODE - Publish skipped"
    echo "Would publish: @crewchief/maproom-mcp@$VERSION"
    echo "Tarball verified and ready for publication"
```

### Error Handling
- Each step should fail fast if prerequisites are not met
- NPM_TOKEN missing should produce clear error message
- Version conflicts (already published) will be rejected by npm (safe failure)
- Network failures trigger retry logic (3 attempts with 10s delay)

### Conditional Execution
- Publish and verify steps only run when: `inputs.dry_run != 'true'`
- Dry run summary only runs when: `inputs.dry_run == 'true'`

## Implementation Notes

### NPM_TOKEN Secret Setup
Before this workflow can run, the NPM_TOKEN secret must be configured:

1. Generate npm token: https://www.npmjs.com/settings/YOUR_USERNAME/tokens
   - Type: "Automation" token
   - Scope: Read and write permissions
2. Add to GitHub secrets: Settings > Secrets and variables > Actions > New repository secret
   - Name: `NPM_TOKEN`
   - Value: Your npm automation token

The workflow will fail with a clear error if this secret is not configured.

### Package Scope and Access
- Package is scoped: `@crewchief/maproom-mcp`
- Scoped packages default to private on npm
- Must use `--access public` flag to publish publicly

### Tarball Verification Strategy
Verifying tarball contents before publishing prevents:
- Publishing packages without binaries
- Shipping incomplete releases
- User-facing errors from missing platform support

The verification uses `tar -tzf` to list contents and `grep` to find each binary.
If any binary is missing, the grep will fail and stop the workflow.

### Retry Logic Rationale
npm publish can fail due to:
- Transient network issues
- npm registry rate limits
- CDN propagation delays

The 3-attempt retry with 10-second delays handles most transient failures
while avoiding infinite loops. The sleep allows time for temporary issues to resolve.

### Post-Publish Verification
After publishing, we:
1. Wait 5 seconds for npm CDN to propagate
2. Use `npm view` to verify package is queryable
3. Print package version and download URL for workflow logs

This confirms the publish succeeded and provides useful output for release notes.

### Dry Run Mode
The `dry_run` input allows:
- Testing the workflow without publishing
- Validating builds and tarball creation
- CI/CD pipeline testing on feature branches

When dry_run is true:
- All validation steps run (tarball creation and verification)
- Publish and verification steps are skipped
- Summary shows what would have been published

## Dependencies
**Required for this ticket:**
- BINPKG-1006: Binary validation job (must download artifacts and verify presence)
- NPM_TOKEN secret configured in GitHub repository settings

**Blocks:**
- BINPKG-1901: End-to-end testing (needs complete publish workflow)
- BINPKG-5002: Production release (needs working automated pipeline)

**Sequence:**
- This is ticket 7 of 11 in Phase 1 of the BINPKG project
- Final implementation ticket for the core workflow
- Completes the automated release pipeline

## Risk Assessment

- **Risk**: NPM_TOKEN secret not configured in repository
  - **Likelihood**: High (first-time setup)
  - **Impact**: High (workflow fails to publish)
  - **Mitigation**: Document setup clearly in implementation notes, workflow fails with clear error message pointing to npm token setup, include setup verification in BINPKG-1901 testing

- **Risk**: npm publish fails due to network issues
  - **Likelihood**: Medium (npm registry can be unreliable)
  - **Impact**: Medium (release delayed, requires manual retry)
  - **Mitigation**: Implement 3-attempt retry logic with delays, document manual publish fallback procedure, log detailed error messages for debugging

- **Risk**: Version conflict (version already published to npm)
  - **Likelihood**: Low (only happens if workflow runs twice for same tag)
  - **Impact**: Low (npm rejects duplicate, safe failure)
  - **Mitigation**: npm registry prevents duplicate version publishes, workflow fails safely with clear error, concurrency control in workflow prevents parallel runs for same tag

- **Risk**: Tarball verification passes but published package is corrupt
  - **Likelihood**: Very Low (npm handles integrity)
  - **Impact**: High (users download broken package)
  - **Mitigation**: Post-publish verification with npm view, npm's own integrity checks, BINPKG-1901 includes end-to-end download testing

- **Risk**: Package publishes successfully but npm CDN propagation delayed
  - **Likelihood**: Low (npm CDN usually fast)
  - **Impact**: Low (verification step might fail temporarily)
  - **Mitigation**: 5-second wait before verification, retry logic on verification step, CDN propagation is eventual consistency (users can still install after delay)

## Files/Packages Affected

### Files to Modify
- `/workspace/.github/workflows/build-and-publish-maproom-mcp.yml` - Add publish steps to validate-and-publish job

### Files to Reference (Read Only)
- `/workspace/packages/maproom-mcp/package.json` - Package metadata and version
- `/workspace/.crewchief/projects/BINPKG_binary-packaging/planning/plan.md` - Phase 1 planning
- `/workspace/.crewchief/projects/BINPKG_binary-packaging/planning/architecture.md` - Publish flow architecture

### Packages Affected
- `packages/maproom-mcp` - Package being published to npm registry

### GitHub Secrets Required
- `NPM_TOKEN` - npm automation token for publishing (must be configured in repository settings)

## Estimated Effort
**2 hours** - Implement publish steps with verification and error handling

Breakdown:
- 30 min: Setup Node.js with npm registry authentication
- 30 min: Implement tarball creation and verification
- 30 min: Implement publish with retry logic
- 20 min: Implement post-publish verification
- 10 min: Add dry-run conditional logic and summary

## Priority
**High** - Completes the automated release pipeline. Final step in Phase 1 core implementation.

## Related Tickets

### Depends On (must complete before this)
- BINPKG-1001: Workflow structure and triggers
- BINPKG-1002: Linux x64 build implementation
- BINPKG-1003: Linux ARM64 build implementation
- BINPKG-1004: macOS x64 build implementation
- BINPKG-1005: macOS ARM64 build implementation
- BINPKG-1006: Binary validation job

### Blocks (cannot start until this completes)
- BINPKG-1901: End-to-end workflow testing
- BINPKG-2001: Local validation script
- BINPKG-5002: Production release

### Sequence
This is ticket 7 of 11 in Phase 1 of the BINPKG project:
1. BINPKG-1001 - Workflow structure
2. BINPKG-1002-1005 - Platform build implementations
3. BINPKG-1006 - Binary validation
4. **BINPKG-1007** (this ticket) - npm publish
5. BINPKG-1901 - End-to-end testing

## Reference Documentation

### Planning Documents
- **Project plan**: `.crewchief/projects/BINPKG_binary-packaging/planning/plan.md` (Phase 1, lines 63-79)
- **Architecture**: `.crewchief/projects/BINPKG_binary-packaging/planning/architecture.md` (lines 213-248)

### External References
- **npm publish documentation**: https://docs.npmjs.com/cli/v10/commands/npm-publish
- **npm automation tokens**: https://docs.npmjs.com/creating-and-viewing-access-tokens
- **GitHub Actions setup-node**: https://github.com/actions/setup-node
- **Scoped packages**: https://docs.npmjs.com/cli/v10/using-npm/scope

### Verification Steps
After implementing this ticket:
1. Verify Node.js setup step includes registry-url and NODE_AUTH_TOKEN
2. Verify tarball verification checks for all 4 binaries
3. Verify publish step has retry logic (3 attempts)
4. Verify publish step respects dry_run input
5. Verify post-publish verification waits and checks npm registry
6. Verify dry-run mode prints summary without publishing
7. Verify all steps use correct working directory (packages/maproom-mcp)
8. Test workflow in dry-run mode (BINPKG-1901)
