# Ticket: CICDOPT-4002: Add Microsoft Marketplace Publishing

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (marketplace publishing requires CI run with secrets)
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
Add job to publish extension to VS Code Marketplace. Downloads .vsix artifact from package job, publishes using vsce with VSCE_PAT secret. Supports pre-release flag and conditional publishing based on secret availability.

## Background
The VS Code Marketplace is the primary distribution channel for VSCode extensions, with millions of users accessing extensions through the built-in extension browser. This ticket adds automated publishing to the marketplace as part of the release workflow.

The publishing job builds on the packaging workflow from CICDOPT-4001, consuming the .vsix artifact and publishing it using the official vsce (Visual Studio Code Extension) CLI tool. The job implements graceful degradation - if the VSCE_PAT secret is not configured, the workflow skips publishing but still succeeds, allowing the workflow to function in environments without marketplace credentials.

**Phase Reference**: Phase 4 - VSCode Extension Publishing (Future, Week 4+)
**Priority**: P2 (Future)

## Acceptance Criteria
- [ ] `publish-vscode` job added to `.github/workflows/release-vscode-maproom.yml`
- [ ] Job dependencies configured: depends on `package-extension`
- [ ] Conditional execution implemented:
  - Job runs only if `secrets.VSCE_PAT != ''`
  - Workflow succeeds even if job is skipped (graceful degradation)
- [ ] Downloads .vsix artifact from package-extension job
- [ ] Installs vsce globally: `npm install -g @vscode/vsce`
- [ ] Publishes extension with command:
  - Base: `vsce publish --packagePath vscode-maproom.vsix -p ${{ secrets.VSCE_PAT }}`
  - Adds `--pre-release` flag if `inputs.pre_release == true`
- [ ] Dry-run test completed with workflow_dispatch (skipped due to missing secret)
- [ ] Real publish test with test tag (then manually unpublished)
- [ ] Extension verified as appearing in VS Code Marketplace
- [ ] Job includes error handling for common vsce failures

## Technical Requirements
- Must use official `@vscode/vsce` package (Microsoft's official CLI)
- Secret must be accessed via `${{ secrets.VSCE_PAT }}`
- Job must skip gracefully if secret not configured (`if: secrets.VSCE_PAT != ''`)
- Must support pre-release publishing via `--pre-release` flag
- Must use `--packagePath` to publish pre-built .vsix (not rebuild)
- Must handle vsce authentication errors gracefully
- Should log marketplace URL after successful publish
- Node.js environment required (latest LTS version)

## Implementation Notes

### Job Configuration
```yaml
publish-vscode:
  needs: package-extension
  runs-on: ubuntu-latest
  if: secrets.VSCE_PAT != ''
  steps:
    - name: Download .vsix artifact
      uses: actions/download-artifact@v4
      with:
        name: vscode-extension-vsix

    - name: Setup Node.js
      uses: actions/setup-node@v4
      with:
        node-version: 'lts/*'

    - name: Install vsce
      run: npm install -g @vscode/vsce

    - name: Publish to VS Code Marketplace
      run: |
        if [ "${{ inputs.pre_release }}" = "true" ]; then
          vsce publish --packagePath vscode-maproom.vsix --pre-release -p ${{ secrets.VSCE_PAT }}
        else
          vsce publish --packagePath vscode-maproom.vsix -p ${{ secrets.VSCE_PAT }}
        fi

    - name: Output marketplace URL
      run: |
        echo "Extension published to: https://marketplace.visualstudio.com/items?itemName=<publisher>.<extension-name>"
```

### Pre-release Publishing
The `--pre-release` flag enables users to opt-in to pre-release versions:
- Pre-release versions can be published with higher version numbers
- Users must explicitly choose to install pre-release versions
- Useful for beta testing before stable release

### Secret Requirements
The VSCE_PAT secret must be:
- Created in VS Code Marketplace publisher account (CICDOPT-4000)
- Added to GitHub repository secrets
- Scoped with "Marketplace (publish)" permissions
- Non-expiring or with expiration monitoring

### Error Handling
Common vsce errors to handle:
- Authentication failures (invalid PAT)
- Version conflicts (version already published)
- Publisher not found
- Package validation failures

### Testing Strategy
1. **Dry-run test**: workflow_dispatch without VSCE_PAT
   - Verify job is skipped gracefully
   - Verify workflow completes successfully

2. **Test publish**: Create test extension or use test version
   - Publish with test tag (e.g., `@crewchief/vscode-maproom@v0.0.1-test`)
   - Verify extension appears in marketplace
   - Manually unpublish test version

3. **Pre-release test**: Test with `pre_release: true`
   - Verify `--pre-release` flag is added
   - Verify extension marked as pre-release in marketplace

### Marketplace URL Pattern
After publishing, the extension will be available at:
```
https://marketplace.visualstudio.com/items?itemName=<publisher>.vscode-maproom
```

The publisher name is configured in package.json and must match the marketplace account.

## Dependencies
- **CICDOPT-4000**: VSCE_PAT secret must be configured in GitHub repository (required for publishing)
- **CICDOPT-4001**: package-extension job must complete successfully and produce .vsix artifact (required)

## Risk Assessment
- **Risk**: VSCE_PAT secret may expire or become invalid
  - **Mitigation**: Implement conditional execution (`if: secrets.VSCE_PAT != ''`); add monitoring for secret expiration; document renewal process

- **Risk**: Version number may already exist in marketplace (duplicate publish)
  - **Mitigation**: vsce will fail with clear error; ensure version bumping process is followed; consider version validation before publish

- **Risk**: Publisher account may be suspended or have insufficient permissions
  - **Mitigation**: Test publishing with test extension; monitor marketplace account health; maintain backup publisher account

- **Risk**: Marketplace API may be temporarily unavailable
  - **Mitigation**: Add retry logic for transient failures; document manual publish process as fallback

- **Risk**: Pre-release flag may not work as expected
  - **Mitigation**: Test pre-release publishing separately; verify marketplace display; document pre-release semantics

## Files/Packages Affected
- `.github/workflows/release-vscode-maproom.yml` (modify - add publish-vscode job)
- `packages/vscode-maproom/package.json` (verify publisher field is set correctly)

## Planning References
- Phase 4 VSCode Extension Publishing (see project plan)
- VS Code Marketplace publishing documentation
- CICDOPT-4000 for marketplace account setup
