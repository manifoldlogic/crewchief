# Ticket: DKRHUB-3005: Publish npm Package v1.1.10

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Build and publish the @crewchief/maproom-mcp@1.1.10 package to npm registry with the updated docker-compose.yml that pulls pre-built images from Docker Hub.

## Background
This is the final step that makes v1.1.10 available to users. The npm package now contains:
- Updated docker-compose.yml (pulls images, no build)
- All necessary configuration files
- CLI wrapper for easy service management

Users can install and run immediately: `npm install -g @crewchief/maproom-mcp@1.1.10 && npx @crewchief/maproom-mcp start`

Reference: DKRHUB_PLAN.md Phase 3, Task DKRHUB-3005 (lines 683-727)

## Acceptance Criteria
- [ ] Package built successfully: `pnpm build` completes without errors
- [ ] prepublishOnly script passes: `pnpm prepublishOnly` (runs security audit)
- [ ] Package contents verified: `npm pack --dry-run` shows correct files
- [ ] docker-compose.yml included in package (uses `image:` not `build:`)
- [ ] docker-compose.override.yml NOT included (development-only)
- [ ] Package published to npm: `pnpm publish --access public`
- [ ] Version 1.1.10 visible on npmjs.com: https://www.npmjs.com/package/@crewchief/maproom-mcp
- [ ] npm install succeeds: `npm install -g @crewchief/maproom-mcp@1.1.10`

## Technical Requirements
**Publishing Commands**:
```bash
# Navigate to package directory
cd packages/maproom-mcp

# 1. Clean and build
pnpm clean  # If clean script exists
pnpm build

# 2. Run prepublish checks (includes npm audit)
pnpm prepublishOnly

# 3. Verify package contents (dry run)
npm pack --dry-run

# Review output - should include:
# - bin/cli.cjs
# - config/docker-compose.yml (with image: directive)
# - config/init.sql
# - dist/ (compiled TypeScript)
# - src/ (TypeScript source)
# - package.json
# - README.md
# - LICENSE
#
# Should NOT include:
# - config/docker-compose.override.yml
# - config/Dockerfile.mcp-server (not needed, images on Docker Hub)
# - node_modules/
# - .env files

# 4. Publish to npm (requires npm login)
pnpm publish --access public

# Confirm publication:
# npm notice
# npm notice 📦  @crewchief/maproom-mcp@1.1.10
# npm notice === Tarball Details ===
# npm notice name:          @crewchief/maproom-mcp
# npm notice version:       1.1.10
# npm notice package size:  XX KB
# npm notice unpacked size: YY KB
# npm notice shasum:        [sha]
# npm notice integrity:     [integrity-hash]
# npm notice total files:   ZZ
# npm notice
# + @crewchief/maproom-mcp@1.1.10

# 5. Verify publication
npm view @crewchief/maproom-mcp version
# Should output: 1.1.10

npm view @crewchief/maproom-mcp
# Should show full package info

# 6. Test install
npm install -g @crewchief/maproom-mcp@1.1.10
which maproom-mcp
maproom-mcp --version  # Should show 1.1.10
```

**package.json Files Array** (verify):
```json
{
  "files": [
    "bin/cli.cjs",
    "config/docker-compose.yml",
    "config/init.sql",
    "dist/",
    "src/",
    "tsconfig.json",
    "README.md",
    "LICENSE"
  ]
}
```
Note: Dockerfile and override file should NOT be in files array.

## Implementation Notes
**prepublishOnly Script**:
The prepublishOnly script runs automatically before `npm publish` and typically includes:
- TypeScript compilation: `tsc`
- Security audit: `pnpm audit --audit-level=high --prod`
- Linting: `pnpm lint` (optional)

If audit finds vulnerabilities:
- Critical/High: Fix before publishing (update dependencies)
- Medium/Low: Document and plan fix for next release

**npm Authentication**:
```bash
# Login to npm (if not already)
npm login

# Verify authentication
npm whoami
# Should show your npm username
```

**Publishing Options**:
- `--access public`: Required for scoped packages (@crewchief/*)
- `--tag next`: For pre-release versions (not used here)
- `--dry-run`: Test publish without actually publishing

**Post-Publish Verification**:
1. Check npmjs.com page updates
2. Verify README renders correctly
3. Check package size (<100KB is good)
4. Test clean install on different machine

**Package Size Expectations**:
- Compressed (download): ~50KB
- Unpacked: ~200KB
- Main components:
  - bin/cli.cjs: ~5KB
  - config files: ~10KB
  - dist/ (compiled): ~50KB
  - src/ (source): ~100KB

**Rollback Plan**:
If issues discovered after publish:
```bash
# Deprecate broken version
npm deprecate @crewchief/maproom-mcp@1.1.10 "Deprecated: Use 1.1.11 instead"

# Publish hotfix
# (Bump to 1.1.11, fix issues, publish again)
```

## Dependencies
- DKRHUB-3004: Images must be on Docker Hub before publishing npm package
- DKRHUB-3001: package.json version must be 1.1.10
- DKRHUB-2001: docker-compose.yml must use `image:` directive

## Risk Assessment
- **Risk**: npm package includes Dockerfile or override (bloat)
  - **Mitigation**: Verify files array in package.json, test with `npm pack --dry-run`
- **Risk**: Security audit fails
  - **Mitigation**: Update vulnerable dependencies, retest before publishing
- **Risk**: Wrong version published
  - **Mitigation**: Verify package.json version matches tag (1.1.10)

## Files/Packages Affected
- Publishes: `@crewchief/maproom-mcp@1.1.10` to npm registry
- No code changes (this is a publishing step)
