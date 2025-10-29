# Ticket: DKRHUB-3001: Update Package Version to v1.1.10

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Bump package.json version to 1.1.10 and update CHANGELOG.md with comprehensive release notes documenting the Docker Hub fix and new features.

## Background
This prepares the codebase for the v1.1.10 release that fixes the broken v1.1.9 deployment. The version bump and changelog update must be committed before creating the git tag that will trigger the GitHub Actions workflow.

Reference: DKRHUB_PLAN.md Phase 3, Task DKRHUB-3001 (lines 536-579)

## Acceptance Criteria
- [ ] `packages/maproom-mcp/package.json` version field updated to `"1.1.10"`
- [ ] CHANGELOG.md includes new v1.1.10 section with date `2025-10-29`
- [ ] Changelog documents Fixed, Added, and Changed sections
- [ ] All changes committed with clear commit message
- [ ] PR created for review (if using PR workflow) or changes pushed to main

## Technical Requirements
**File 1**: `packages/maproom-mcp/package.json`
```json
{
  "name": "@crewchief/maproom-mcp",
  "version": "1.1.10",
  // ... rest of package.json unchanged
}
```

**File 2**: `CHANGELOG.md` (add at top, after existing entries)
```markdown
## [1.1.10] - 2025-10-29

### Fixed
- **Critical**: Fixed Docker Hub image distribution (v1.1.9 deployment failure)
  - docker-compose.yml now pulls pre-built images instead of building from source
  - Resolves "lstat /packages: no such file or directory" error
  - npm package now works correctly when installed globally or locally
- Build context error preventing users from starting services after npm install

### Added
- Automated Docker image publishing via GitHub Actions workflow
- Multi-platform support: AMD64 (x86_64) and ARM64 (Apple Silicon)
- Version pinning support via MAPROOM_VERSION environment variable
- docker-compose.override.yml for local development builds
- OCI-compliant image metadata labels (version, revision, created, etc.)
- Trivy security scanning in CI/CD pipeline

### Changed
- docker-compose.yml uses `image:` directive instead of `build:` for production
- Images now available at https://hub.docker.com/r/crewchief/maproom-mcp
- Faster startup: ~30 seconds (no build time)
- Development workflow: use docker-compose.override.yml for local builds

### Migration Notes
Upgrade from v1.1.9:
```bash
# Stop existing services
npx @crewchief/maproom-mcp stop

# Update package
npm install -g @crewchief/maproom-mcp@latest

# Start services (now pulls from Docker Hub)
npx @crewchief/maproom-mcp start
```

v1.1.9 is deprecated due to deployment failure. Skip directly to v1.1.10.
```

**Commit Message**:
```
chore(release): bump version to 1.1.10

- Fix critical v1.1.9 deployment failure
- Add Docker Hub publishing workflow
- Update docker-compose.yml to use pre-built images
- Add multi-platform support (AMD64, ARM64)

Fixes: DKRHUB-Docker Hub Publishing project
```

## Implementation Notes
**Version Number Rationale**:
- v1.1.9: Broken (docker-compose build failure)
- v1.1.10: Patch release fixing deployment issue
- Patch version bump (not minor) because this is a bug fix, not new features

**Changelog Format**:
Following Keep a Changelog format:
- Sections: Fixed, Added, Changed, Deprecated, Removed, Security
- Date format: YYYY-MM-DD
- Version links at bottom of file (update if present)

**Breaking Changes**:
While this changes how images are built, it's not a breaking change for users:
- API/functionality unchanged
- Environment variables unchanged
- Data persistence unchanged
- For users: Transparent improvement (faster, more reliable)
- For developers: Override file preserves workflow

**Review Checklist**:
- [ ] Version number correct (1.1.10)
- [ ] Date correct (2025-10-29)
- [ ] All changes documented
- [ ] Grammar and spelling correct
- [ ] Links to Docker Hub included

## Dependencies
- DKRHUB-2001, DKRHUB-2002, DKRHUB-2003: All code changes must be complete
- DKRHUB-2902, DKRHUB-2903: Testing should be complete (verify changes work)

## Risk Assessment
- **Risk**: Incorrect version number
  - **Mitigation**: Double-check against plan, verify format (semantic versioning)
- **Risk**: Incomplete changelog
  - **Mitigation**: Review all tickets in project, ensure all changes documented
- **Risk**: Premature commit (before testing complete)
  - **Mitigation**: Verify all Phase 2 tests pass first

## Files/Packages Affected
- `packages/maproom-mcp/package.json` (version field only)
- `CHANGELOG.md` (add new section)
