# Ticket: CICDOPT-4001: Create VSCode Extension Build Workflow

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (workflow validation requires CI run)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- github-actions-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create workflow for building and packaging the vscode-maproom extension. Calls reusable TypeScript build workflow, runs extension-specific validation, packages with vsce, and uploads .vsix artifact for use by publishing jobs.

## Background
The CrewChief project includes a VSCode extension (`packages/vscode-maproom/`) that needs automated build and packaging infrastructure. This ticket implements the foundation of the VSCode extension publishing pipeline by creating a workflow that builds, packages, and produces a .vsix artifact.

This workflow serves as the foundation for Phase 4 (VSCode Extension Publishing), producing the .vsix file that will be consumed by marketplace publishing jobs (VS Code Marketplace and Open VSX Registry). The workflow must integrate with the existing reusable TypeScript build workflow created in Phase 1 to maintain consistency across all TypeScript packages.

**Phase Reference**: Phase 4 - VSCode Extension Publishing (Future, Week 4+)
**Priority**: P2 (Future)

## Acceptance Criteria
- [ ] New workflow file created at `.github/workflows/release-vscode-maproom.yml`
- [ ] `build-extension` job calls `reusable-typescript-build.yml` with:
  - `workspace_filter: '@crewchief/vscode-maproom...'`
  - `artifact_name: 'vscode-extension-dist'`
- [ ] `package-extension` job implemented with dependency on `build-extension`:
  - Downloads TypeScript dist artifact from build-extension job
  - Installs vsce package manager: `pnpm add -g @vscode/vsce`
  - Packages extension: `vsce package --out vscode-maproom.vsix`
  - Runs smoke tests (verify package.json exists, verify dist/ directory exists)
  - Uploads .vsix artifact with 90-day retention
- [ ] Workflow triggers configured:
  - Push trigger for tags matching `@crewchief/vscode-maproom@v*.*.*`
  - workflow_dispatch with inputs: `version`, `pre_release`
- [ ] Dry-run test succeeds using workflow_dispatch
- [ ] .vsix artifact structure verified (contains extension.js, package.json, dist/)
- [ ] Workflow includes proper permissions configuration
- [ ] Documentation added to workflow file explaining job flow

## Technical Requirements
- Workflow must use GitHub Actions workflow syntax (YAML)
- Must leverage existing `reusable-typescript-build.yml` for consistency
- Must use `@vscode/vsce` package (official VS Code extension CLI)
- Artifact retention: 90 days (standard for release artifacts)
- Smoke tests should verify minimal package structure before upload
- workflow_dispatch inputs should match publishing job requirements
- Must download artifacts using `actions/download-artifact@v4`
- Must upload artifacts using `actions/upload-artifact@v4`

## Implementation Notes

### Workflow Structure
The workflow follows this job dependency chain:
```
build-extension (reusable) → package-extension → [publishing jobs]
```

### Build Job
Calls the reusable TypeScript build workflow with extension-specific configuration:
```yaml
build-extension:
  uses: ./.github/workflows/reusable-typescript-build.yml
  with:
    workspace_filter: '@crewchief/vscode-maproom...'
    artifact_name: 'vscode-extension-dist'
```

### Package Job
Key steps for the package-extension job:
1. Download dist artifact from build-extension
2. Set up Node.js environment
3. Install dependencies: `pnpm install --frozen-lockfile`
4. Install vsce globally: `pnpm add -g @vscode/vsce`
5. Run packaging: `vsce package --out vscode-maproom.vsix`
6. Smoke tests:
   ```bash
   # Verify package.json exists
   test -f package.json
   # Verify dist directory exists
   test -d dist
   # Verify .vsix was created
   test -f vscode-maproom.vsix
   ```
7. Upload .vsix artifact

### Workflow Triggers
```yaml
on:
  push:
    tags:
      - '@crewchief/vscode-maproom@v*.*.*'
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to publish (e.g., 1.0.0)'
        required: true
        type: string
      pre_release:
        description: 'Publish as pre-release'
        type: boolean
        default: false
```

### Permissions
```yaml
permissions:
  contents: read  # For checkout
```

Note: This workflow only builds and packages. Publishing jobs (CICDOPT-4002, CICDOPT-4003) will require `contents: write` for releases.

### Testing Strategy
1. Use workflow_dispatch with a test version (e.g., `0.0.0-test`)
2. Verify workflow completes successfully
3. Download .vsix artifact and inspect contents
4. Verify smoke tests passed in logs

## Dependencies
- **CICDOPT-4000**: Marketplace accounts and credentials setup required (for vsce package command, even without publishing)
- **CICDOPT-1001**: Reusable TypeScript build workflow must exist
- **CICDOPT-1002**: Turborepo cache configuration needed for efficient builds

## Risk Assessment
- **Risk**: vsce package command may require authentication even for packaging (without publishing)
  - **Mitigation**: Test packaging command locally without credentials first; if authentication required, document dependency on CICDOPT-4000 completion

- **Risk**: Extension package.json may not be properly configured for vsce
  - **Mitigation**: Run vsce validation before packaging; add smoke tests to verify package structure

- **Risk**: TypeScript dist may not contain all required files for extension
  - **Mitigation**: Verify dist/ contents in smoke tests; ensure build workflow includes all extension assets

- **Risk**: Artifact upload/download may fail with large extension bundles
  - **Mitigation**: Use v4 of artifact actions (improved compression); set appropriate retention period

## Files/Packages Affected
- `.github/workflows/release-vscode-maproom.yml` (new file)
- `packages/vscode-maproom/package.json` (may need vsce configuration verification)
- `packages/vscode-maproom/` (extension source, will be packaged)

## Planning References
- Phase 4 VSCode Extension Publishing (see project plan)
- Reusable workflow pattern established in Phase 1
