# Ticket: BINPKG-3002: Update package.json release scripts to use new release.js

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
Update `packages/maproom-mcp/package.json` release scripts (release:patch, release:minor, release:major) to call the new `release.js` script instead of `bump-version.js` + manual publish.

## Background
Current scripts call `bump-version.js` then `pnpm publish` directly. New approach: `release.js` bumps version, commits, tags, pushes → GitHub Actions builds binaries and publishes. This ticket updates the package.json scripts to use the new integrated workflow. By routing all releases through the release.js script, we ensure consistent version bumping, git tagging, and triggering of the CI/CD pipeline that builds platform binaries and publishes to npm.

## Acceptance Criteria
- [ ] `release:patch` script updated to: `"node ../../scripts/release.js patch"`
- [ ] `release:minor` script updated to: `"node ../../scripts/release.js minor"`
- [ ] `release:major` script updated to: `"node ../../scripts/release.js major"`
- [ ] Scripts no longer call `pnpm publish` directly (CI handles publishing)
- [ ] Scripts no longer call `bump-version.js` (deprecated)
- [ ] Manual test: Run `pnpm release:patch --dry-run` to verify script works
- [ ] Document that `pnpm publish` still works for manual emergency releases (with validation from BINPKG-2002)

## Technical Requirements
- File: `packages/maproom-mcp/package.json`
- Update scripts section:
  ```json
  {
    "scripts": {
      "release:patch": "node ../../scripts/release.js patch",
      "release:minor": "node ../../scripts/release.js minor",
      "release:major": "node ../../scripts/release.js major"
    }
  }
  ```
- Previous scripts (for reference):
  ```json
  {
    "scripts": {
      "release:patch": "node scripts/bump-version.js patch && pnpm publish --access public --no-git-checks",
      "release:minor": "node scripts/bump-version.js minor && pnpm publish --access public --no-git-checks",
      "release:major": "node scripts/bump-version.js major && pnpm publish --access public --no-git-checks"
    }
  }
  ```
- Remove --no-git-checks flag (CI handles publishing with proper checks)
- Remove --access public flag (CI handles publishing with access configuration)
- Script path is relative to package.json (../../ for repo root)
- Keep `pnpm publish` available for emergency manual releases
- Validation (BINPKG-2002) still runs on manual publish via prepublishOnly

## Implementation Notes
- **Workflow Change**: Release scripts now trigger CI/CD pipeline instead of publishing directly
- **Script Path Resolution**: Path is relative to package.json location - `../../` navigates from `packages/maproom-mcp/` to repo root
- **CI/CD Integration**: The release.js script creates a git tag which triggers GitHub Actions workflow
- **Emergency Publish**: Developers can still run `pnpm publish` directly for urgent fixes (validation still runs)
- **Dry Run Testing**: Use `--dry-run` flag to test release script without making changes
- **Deprecation**: The old `scripts/bump-version.js` file is no longer used by these commands

## Dependencies
- **BINPKG-3001** - release.js must exist at `scripts/release.js`
- **BINPKG-2002** - prepublishOnly hook provides safety net for manual publishes
- GitHub Actions workflow must be configured to trigger on version tags

## Risk Assessment
- **Risk**: Developers confused by workflow change (used to old bump + publish pattern)
  - **Mitigation**: Update documentation (BINPKG-4001) with clear examples and workflow diagrams
- **Risk**: Emergency manual publish needed but developer doesn't know how
  - **Mitigation**: `pnpm publish` still works with validation from BINPKG-2002, document in README
- **Risk**: Script path resolution breaks if package structure changes
  - **Mitigation**: Test script execution from package directory, document path requirements
- **Risk**: --dry-run flag might not work with release.js
  - **Mitigation**: Ensure release.js supports --dry-run flag (verify in BINPKG-3001)

## Files/Packages Affected
- MODIFY: `packages/maproom-mcp/package.json`
  - Update release:patch script
  - Update release:minor script
  - Update release:major script
