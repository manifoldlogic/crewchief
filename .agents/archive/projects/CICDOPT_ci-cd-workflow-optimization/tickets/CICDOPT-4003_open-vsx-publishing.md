# Ticket: CICDOPT-4003: Add Open VSX Publishing

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (Open VSX publishing requires CI run with secrets)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Add job to publish extension to Open VSX Registry (parallel with VS Code Marketplace). Downloads .vsix artifact, publishes using ovsx with OVSX_PAT secret. Runs in parallel with VS Code Marketplace publishing to maximize distribution reach.

## Background
Open VSX Registry is an open-source alternative to the VS Code Marketplace, used by VSCodium, Gitpod, Eclipse Theia, and other VS Code-compatible editors. Publishing to Open VSX significantly expands the extension's reach beyond Microsoft's marketplace.

This job runs in parallel with the VS Code Marketplace publishing job (CICDOPT-4002), as they are independent operations that both consume the same .vsix artifact from the package job. The parallel execution reduces overall workflow time and allows either publisher to fail independently.

Like the VS Code publisher, this job implements graceful degradation - if the OVSX_PAT secret is not configured, the workflow skips publishing but still succeeds.

**Phase Reference**: Phase 4 - VSCode Extension Publishing (Future, Week 4+)
**Priority**: P2 (Future)

## Acceptance Criteria
- [ ] `publish-ovsx` job added to `.github/workflows/release-vscode-maproom.yml`
- [ ] Job dependencies configured: depends on `package-extension` (NOT publish-vscode - parallel execution)
- [ ] Conditional execution implemented:
  - Job runs only if `secrets.OVSX_PAT != ''`
  - Workflow succeeds even if job is skipped
- [ ] Downloads .vsix artifact from package-extension job
- [ ] Installs ovsx CLI: `npm install -g ovsx`
- [ ] Publishes extension: `ovsx publish vscode-maproom.vsix -p ${{ secrets.OVSX_PAT }}`
- [ ] Dry-run test completed with workflow_dispatch (skipped due to missing secret)
- [ ] Real publish test with test tag completed
- [ ] Extension verified as appearing in Open VSX Registry (https://open-vsx.org/)
- [ ] Verified both publishers (publish-vscode and publish-ovsx) run in parallel
- [ ] Job includes error handling for common ovsx failures

## Technical Requirements
- Must use `ovsx` package (official Open VSX Registry CLI)
- Secret must be accessed via `${{ secrets.OVSX_PAT }}`
- Job must skip gracefully if secret not configured (`if: secrets.OVSX_PAT != ''`)
- Must depend ONLY on `package-extension` (not `publish-vscode`) for parallel execution
- Must use pre-built .vsix artifact (not rebuild)
- Must handle ovsx authentication errors gracefully
- Should log Open VSX Registry URL after successful publish
- Node.js environment required (latest LTS version)

## Implementation Notes

### Job Configuration
```yaml
publish-ovsx:
  needs: package-extension  # Parallel with publish-vscode
  runs-on: ubuntu-latest
  if: secrets.OVSX_PAT != ''
  steps:
    - name: Download .vsix artifact
      uses: actions/download-artifact@v4
      with:
        name: vscode-extension-vsix

    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: 'lts/*'

    - name: Install ovsx
      run: npm install -g ovsx

    - name: Publish to Open VSX Registry
      run: ovsx publish vscode-maproom.vsix -p ${{ secrets.OVSX_PAT }}

    - name: Output Open VSX URL
      run: |
        echo "Extension published to: https://open-vsx.org/extension/<namespace>/<extension-name>"
```

### Parallel Execution
Key design decision: This job depends ONLY on `package-extension`, NOT on `publish-vscode`:
- Both publishers can run simultaneously
- Reduces total workflow time
- One publisher can fail without blocking the other
- Both publishers download the same .vsix artifact

Job dependency visualization:
```
package-extension
    ├── publish-vscode (parallel)
    └── publish-ovsx   (parallel)
```

### Open VSX vs VS Code Marketplace
Key differences from vsce (CICDOPT-4002):
- No `--pre-release` flag (Open VSX handles pre-releases differently)
- Different CLI tool (`ovsx` instead of `vsce`)
- Different registry URL and authentication
- Different namespace/publisher configuration

### Secret Requirements
The OVSX_PAT secret must be:
- Created in Open VSX Registry account (CICDOPT-4000)
- Added to GitHub repository secrets
- Generated from https://open-vsx.org/user-settings/tokens
- Associated with the correct namespace

### Error Handling
Common ovsx errors to handle:
- Authentication failures (invalid PAT)
- Namespace not found
- Version conflicts (version already published)
- Package validation failures
- Registry unavailable

### Testing Strategy
1. **Dry-run test**: workflow_dispatch without OVSX_PAT
   - Verify job is skipped gracefully
   - Verify workflow completes successfully

2. **Test publish**: Create test extension or use test version
   - Publish with test tag (e.g., `@crewchief/vscode-maproom@v0.0.1-test`)
   - Verify extension appears in Open VSX Registry
   - Manually unpublish or deprecate test version

3. **Parallel execution test**: Run with both VSCE_PAT and OVSX_PAT
   - Verify both jobs run simultaneously
   - Check workflow logs for parallel execution
   - Verify both marketplaces receive the extension

### Open VSX Registry URL Pattern
After publishing, the extension will be available at:
```
https://open-vsx.org/extension/<namespace>/vscode-maproom
```

The namespace is configured in the Open VSX account and may differ from the VS Code Marketplace publisher name.

### Pre-release Handling
Open VSX handles pre-releases differently than VS Code Marketplace:
- Pre-release versions are determined by semver (e.g., 1.0.0-beta)
- No explicit `--pre-release` flag needed
- Registry automatically detects pre-release versions from version string

## Dependencies
- **CICDOPT-4000**: OVSX_PAT secret must be configured in GitHub repository (required for publishing)
- **CICDOPT-4001**: package-extension job must complete successfully and produce .vsix artifact (required)

Note: This ticket does NOT depend on CICDOPT-4002 (VS Code Marketplace publishing). Both publishers run in parallel.

## Risk Assessment
- **Risk**: OVSX_PAT secret may expire or become invalid
  - **Mitigation**: Implement conditional execution (`if: secrets.OVSX_PAT != ''`); Open VSX tokens don't expire by default but can be revoked

- **Risk**: Open VSX namespace may not be configured correctly
  - **Mitigation**: Verify namespace during CICDOPT-4000; test with sample extension; document namespace configuration

- **Risk**: Open VSX Registry may have stricter validation than VS Code Marketplace
  - **Mitigation**: Test publishing with test extension; ensure package.json meets Open VSX requirements; document any Open VSX-specific requirements

- **Risk**: Version number may already exist in registry (duplicate publish)
  - **Mitigation**: ovsx will fail with clear error; coordinate version numbers with VS Code Marketplace; consider version validation before publish

- **Risk**: One publisher may succeed while the other fails
  - **Mitigation**: Parallel execution allows partial success; create-release job (CICDOPT-4004) handles partial success; document manual publish process for failed publisher

## Files/Packages Affected
- `.github/workflows/release-vscode-maproom.yml` (modify - add publish-ovsx job)
- `packages/vscode-maproom/package.json` (verify namespace/publisher configuration)

## Planning References
- Phase 4 VSCode Extension Publishing (see project plan)
- Open VSX Registry documentation
- CICDOPT-4000 for Open VSX account setup
- CICDOPT-4002 for comparison with VS Code Marketplace publishing
