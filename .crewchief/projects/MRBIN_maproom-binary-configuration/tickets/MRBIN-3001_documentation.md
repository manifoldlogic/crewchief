# Ticket: [MRBIN-3001]: Update Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update all documentation to describe the new maproomBinaryPath configuration option, resolution priority order, and provide examples of config-based binary path configuration for development workflows.

## Background
With the implementation complete, users need clear documentation on how to use the new config-based binary path feature. This is especially important for developers working with local builds who want to avoid manually setting environment variables.

Documentation must cover:
1. The configuration option itself
2. Resolution priority order (especially the global > packaged change)
3. Development workflow examples
4. Example configuration files

## Acceptance Criteria
- [x] README.md documents maproomBinaryPath config option
- [x] README.md shows example configuration with maproomBinaryPath
- [x] docs/development/local-development.md updated with config-based workflow
- [x] Example includes relative path for local builds
- [x] Resolution priority order documented clearly
- [x] Migration notes for priority order change included
- [x] All documentation examples are accurate and tested
- [x] Links to config documentation are correct

## Technical Requirements
- Document config field: `repository.maproomBinaryPath` (optional string)
- Document priority order: env > config > global > packaged
- Show example config with relative path for local development
- Show example config with absolute path for shared builds
- Explain behavior change (global now preferred over packaged)
- Provide migration checklist for users affected by priority change

## Implementation Notes

### README.md Updates
Add section to configuration documentation:

```markdown
### Binary Configuration

Specify a custom path to the maproom binary:

\`\`\`javascript
// crewchief.config.js
export default {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}
\`\`\`

**Resolution Priority:**
1. `CREWCHIEF_MAPROOM_BIN` environment variable
2. `config.repository.maproomBinaryPath`
3. Global install (`npm install -g @crewchief/cli`)
4. Packaged binary

**Use cases:**
- Local development with Rust builds
- Custom binary locations
- CI/CD environments with specific versions
```

### docs/development/local-development.md Updates
Add config-based development workflow:

```markdown
### Using Local Maproom Builds

When developing maproom, configure CrewChief to use your local build:

\`\`\`javascript
// crewchief.config.local.js
export default {
  repository: {
    maproomBinaryPath: './target/release/crewchief-maproom'
  }
}
\`\`\`

Build and test:
\`\`\`bash
cd crates/maproom
cargo build --release
cd ../..
crewchief maproom scan  # Uses your local build
\`\`\`

This approach is preferred over setting `CREWCHIEF_MAPROOM_BIN` because:
- Config persists across terminal sessions
- Can use `.local.js` to keep out of git
- Relative paths work from any location in repo
```

### Migration Guide
Include behavior change notes:

```markdown
### Priority Order Change (v0.x.0)

**Breaking change:** Binary resolution now prefers global installs over packaged binaries.

**Before:**
1. Environment variable
2. Packaged binary
3. Global install

**After:**
1. Environment variable
2. Config file
3. Global install
4. Packaged binary

**Who is affected:**
- Users with both global and packaged installs will now use global
- This prevents stale packaged binaries from being used

**Migration:**
- No action needed for most users
- To force packaged binary: uninstall global version
- To force specific binary: use `CREWCHIEF_MAPROOM_BIN` env var
```

## Dependencies
- MRBIN-2001 (maproom.ts implementation complete)
- MRBIN-2002 (worktrees.ts implementation complete)

## Risk Assessment
- **Risk**: Documentation examples don't work in practice
  - **Mitigation**: Test all examples manually before committing
- **Risk**: Missing important edge cases or use cases
  - **Mitigation**: Review with planning docs, cover all scenarios from architecture.md
- **Risk**: Migration notes unclear
  - **Mitigation**: Include concrete examples, provide checklist

## Files/Packages Affected
- README.md
- docs/development/local-development.md
- packages/cli/README.md (if exists)

## Verification Notes
Verify that:
1. All config examples are syntactically correct
2. Example paths work when tested manually
3. Priority order is documented consistently everywhere
4. Migration notes are clear and actionable
5. Links to config sections work
6. Documentation follows existing style and formatting
7. Code blocks have proper syntax highlighting
8. Examples use realistic paths
9. No conflicting information across docs
10. Documentation renders correctly in markdown viewers
