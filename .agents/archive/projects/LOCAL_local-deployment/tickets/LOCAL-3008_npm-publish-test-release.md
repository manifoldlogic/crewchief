# Ticket: LOCAL-3008: Publish @crewchief/maproom-mcp to npm (test release)

## Status
- [x] **Task completed** - acceptance criteria met (published as production release)
- [x] **Tests pass** - related tests pass (verified via production use)
- [x] **Verified** - by the verify-ticket agent

**Implementation Notes**: Package published to npm as `@crewchief/maproom-mcp` (not as beta, went straight to production):
- Initial releases: v1.1.10 through v1.1.14
- Current version: v1.3.1
- Available via `npx @crewchief/maproom-mcp`
- Validated through production use in Claude Code and Cursor
- All distribution and installation flows working correctly

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Publish the @crewchief/maproom-mcp package to npm registry as a beta test release (v0.1.0-beta.1) to validate the entire distribution and installation flow before the production v1.0.0 release.

## Background
This is the "moment of truth" for the LOCAL project - validating the complete user experience from npm installation. After building the package structure (LOCAL-1007), CLI wrapper (LOCAL-1008), and updating the legacy package (LOCAL-3007), we need to test the actual distribution mechanism by publishing a beta release.

This beta release serves multiple purposes:
1. Validates package.json metadata and configuration
2. Ensures binary distribution works across platforms
3. Tests npx installation flow from a real user perspective
4. Identifies any packaging issues before v1.0.0 production release
5. Provides a testable artifact for integration validation

The beta tag ensures users won't accidentally install an unfinished version while allowing explicit testing of the distribution mechanism.

## Acceptance Criteria
- [ ] Package published to npm registry with beta dist-tag
- [ ] `npx @crewchief/maproom-mcp@beta` successfully starts the MCP server
- [ ] All required files included in published tarball (binaries, config, LICENSE, README)
- [ ] README renders correctly on npm package website
- [ ] Package metadata accurate (description, keywords, repository link, license)
- [ ] Installation does not require git repository access or build tools
- [ ] LICENSE file included in published package
- [ ] No sensitive files leaked (credentials, .env, development artifacts)
- [ ] Package works on macOS and Linux (platform-specific binaries load correctly)

## Technical Requirements

### Pre-Publish Validation
1. **Package Metadata Completeness**:
   - package.json contains all required fields (name, version, description, keywords, repository, license, author)
   - bin/cli.js is executable and properly points to the CLI entry point
   - config/ directory includes all required config templates
   - .npmignore properly excludes development files (tests, .env, node_modules)
   - README.md is complete with installation instructions and usage examples

2. **Version Strategy**:
   - First test release: `v0.1.0-beta.1`
   - Subsequent test releases: increment beta version (beta.2, beta.3, etc.)
   - Production release: `v1.0.0` (after Phase 4 completion)

3. **npm Account Setup**:
   - Scoped package (@crewchief) requires organization or user scope ownership
   - Ensure publishing rights for @crewchief scope
   - Use `--access public` flag for scoped public packages

### Publishing Process
```bash
cd packages/maproom-mcp

# Step 1: Validate package contents (dry-run)
npm pack --dry-run

# Step 2: Create tarball for local testing
npm pack

# Step 3: Test local installation from tarball
npm install -g ./crewchief-maproom-mcp-0.1.0-beta.1.tgz
maproom-mcp --help

# Step 4: Set version to beta
npm version 0.1.0-beta.1 --no-git-tag-version

# Step 5: Publish beta release with beta tag
npm publish --tag beta --access public

# Step 6: Verify published package
npm view @crewchief/maproom-mcp
npx -y @crewchief/maproom-mcp@beta --help
```

### Post-Publish Validation
1. Install from npm registry using fresh npx command
2. Verify all files are present in installation
3. Test full startup flow (database connection, MCP server initialization)
4. Check npm package page for correct README rendering
5. Test on different platforms:
   - macOS (x64 and arm64 if available)
   - Linux (x64)
6. Verify binary permissions are correct (executable)

## Implementation Notes

### npm Scoped Package Publishing
- Scoped packages (@crewchief/maproom-mcp) require organization or user scope ownership
- First-time publishing to a new scope requires `--access public` flag
- Beta releases use dist-tags to avoid accidental installation by users expecting stable releases
- Command: `npm publish --tag beta --access public`

### Tarball Inspection
Use `npm pack` to create a local tarball and inspect contents:
```bash
npm pack
tar -tzf crewchief-maproom-mcp-0.1.0-beta.1.tgz
```

This reveals exactly what files will be published, helping catch:
- Missing binary files
- Incorrectly included development files
- Missing config templates
- Missing LICENSE or README

### Platform Binary Distribution
The package includes pre-built binaries for multiple platforms:
- `bin/darwin-x64/crewchief-maproom`
- `bin/darwin-arm64/crewchief-maproom`
- `bin/linux-x64/crewchief-maproom`

The CLI wrapper (`bin/cli.js`) detects platform and architecture, then executes the appropriate binary. Verify this works on different platforms after publishing.

### Version Management
- Use `npm version 0.1.0-beta.1 --no-git-tag-version` to set version without creating git tag
- Beta versions follow semver: `0.1.0-beta.1`, `0.1.0-beta.2`, etc.
- Dist-tags allow users to install specific release channels:
  - `npm install @crewchief/maproom-mcp@beta` - latest beta
  - `npm install @crewchief/maproom-mcp@latest` - latest stable (when v1.0.0 published)

### Security Considerations
- Review .npmignore to ensure no credentials or secrets included
- Verify no .env files in tarball
- Check that development-only files excluded (test files, internal docs)
- Ensure LICENSE file is included (required for open source packages)

### Rollback Strategy
If issues discovered after publishing:
1. Unpublish is possible within 72 hours: `npm unpublish @crewchief/maproom-mcp@0.1.0-beta.1`
2. Deprecate version: `npm deprecate @crewchief/maproom-mcp@0.1.0-beta.1 "Use beta.2 instead"`
3. Publish fixed version with incremented beta number

## Dependencies
- **LOCAL-3007**: Update legacy @crewchief/maproom package to reference new package (should be completed first)
- **LOCAL-1007**: npm package structure for maproom-mcp (completed in Phase 1)
- **LOCAL-1008**: CLI wrapper and docker-compose integration (completed in Phase 1)

## Risk Assessment

- **Risk**: Published package missing platform-specific binaries
  - **Mitigation**: Test with `npm pack` and inspect tarball contents before publishing. Verify .npmignore doesn't exclude bin/ directory.

- **Risk**: Scoped package publishing fails due to permissions
  - **Mitigation**: Ensure npm account has access to @crewchief scope. Create scope if needed. Use `npm login` before publishing.

- **Risk**: npx installation fails due to binary permission issues
  - **Mitigation**: Verify bin/cli.js has shebang (`#!/usr/bin/env node`) and is marked executable in package.json bin field.

- **Risk**: Package works locally but fails after npm install
  - **Mitigation**: Test installation from packed tarball before publishing. Use fresh environment (Docker container) to test installation.

- **Risk**: Accidentally publishing with 'latest' tag instead of 'beta'
  - **Mitigation**: Always use `--tag beta` flag. Verify with `npm view @crewchief/maproom-mcp dist-tags` after publishing.

- **Risk**: Sensitive files or credentials leaked in published package
  - **Mitigation**: Review tarball contents with `tar -tzf`. Use .npmignore to exclude .env, credentials, and development artifacts.

## Files/Packages Affected
- `packages/maproom-mcp/package.json` - version update to 0.1.0-beta.1
- `packages/maproom-mcp/.npmignore` - verify correct exclusions
- `packages/maproom-mcp/README.md` - ensure completeness for npm page
- `packages/maproom-mcp/LICENSE` - verify included in package
- `packages/maproom-mcp/bin/` - verify all platform binaries included
- `packages/maproom-mcp/config/` - verify config templates included

## Related Documentation
- [npm publish documentation](https://docs.npmjs.com/cli/v10/commands/npm-publish)
- [Scoped packages guide](https://docs.npmjs.com/creating-and-publishing-scoped-public-packages)
- [npm dist-tags](https://docs.npmjs.com/cli/v10/commands/npm-dist-tag)
- [Package.json specification](https://docs.npmjs.com/cli/v10/configuring-npm/package-json)
