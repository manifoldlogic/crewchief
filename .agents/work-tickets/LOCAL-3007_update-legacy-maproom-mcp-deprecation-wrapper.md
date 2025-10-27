# Ticket: LOCAL-3007: Update legacy maproom-mcp with deprecation wrapper

## Status
- [x] **Task completed** - marked as future enhancement - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update the existing `maproom-mcp` npm package (the legacy manual-setup version) to forward to the new `@crewchief/maproom-mcp` containerized version with a clear deprecation notice. This ensures a smooth migration path for existing users without breaking changes.

## Background
The LOCAL project is creating a fully containerized version of Maproom MCP (`@crewchief/maproom-mcp`) that includes Docker, bundled PostgreSQL, and local LLM embeddings with zero configuration.

The legacy `maproom-mcp` package requires manual PostgreSQL setup and API keys. To provide a seamless migration path for existing users, we need to:
1. Update the legacy package with a deprecation wrapper
2. Forward all calls to the new containerized package
3. Communicate the benefits and migration steps clearly
4. Mark the package as deprecated on npm

This prevents breaking existing installations while guiding users to the improved version.

## Acceptance Criteria
- [ ] Legacy maproom-mcp package updated with deprecation wrapper script
- [ ] Clear deprecation message displayed when legacy package runs
- [ ] Wrapper correctly forwards all arguments to @crewchief/maproom-mcp
- [ ] Exit codes properly passed through from new package
- [ ] Published as new patch version of legacy package
- [ ] npm deprecate command executed successfully
- [ ] Migration guide added to legacy package README
- [ ] GitHub issue created to track 6-month migration timeline

## Technical Requirements
- Update bin file in legacy `maproom-mcp` package with Node.js wrapper script
- Use `child_process.spawn` to forward to new package via npx
- Display deprecation warning with clear benefits and migration instructions
- Pass through all CLI arguments (`process.argv.slice(2)`)
- Forward stdio streams (`stdio: 'inherit'`)
- Handle exit codes properly (`.on('exit', process.exit)`)
- Bump package version (patch increment)
- Publish to npm
- Execute npm deprecate command with clear message

## Implementation Notes

**Deprecation Wrapper Implementation** (from LOCAL_ARCHITECTURE.md lines 276-306):

```javascript
#!/usr/bin/env node
console.warn(`
⚠️  DEPRECATION NOTICE: maproom-mcp has moved to @crewchief/maproom-mcp

The new version includes:
  • 🐳 Fully containerized with Docker
  • 🚀 Local LLM embeddings (no API keys required)
  • 📦 Bundled PostgreSQL
  • 🔌 Zero-configuration setup

Please update your .mcp.json:
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}

This compatibility wrapper will forward to the new package.
The legacy manual-setup version will be removed in 6 months.
`);

// Forward to new package
const { spawn } = require('child_process');
spawn('npx', ['-y', '@crewchief/maproom-mcp', ...process.argv.slice(2)], {
  stdio: 'inherit'
}).on('exit', process.exit);
```

**npm Deprecation Command**:
```bash
npm deprecate maproom-mcp "Package moved to @crewchief/maproom-mcp. Please update your config."
```

**Migration Timeline** (from LOCAL_ARCHITECTURE.md lines 313-316):
- **Month 1-3**: Both packages work, warnings in legacy
- **Month 4-6**: Announce removal date
- **Month 6+**: Remove legacy or make permanent redirect

**Key Considerations**:
- The wrapper should use `npx -y` to auto-install the new package
- Warning message goes to stderr (`console.warn`) so it doesn't interfere with JSON output
- All arguments must be forwarded exactly as received
- Exit code must match the new package's exit code
- The deprecation is informative, not punitive - emphasize benefits

**Reference Documentation**:
- npm deprecate: https://docs.npmjs.com/cli/v10/commands/npm-deprecate
- Package migration best practices: https://docs.npmjs.com/policies/deprecations

## Dependencies
- **LOCAL-1008** - CLI wrapper for docker-compose must be complete and published
- The new `@crewchief/maproom-mcp` package must be available on npm
- Access to npm publish credentials for legacy `maproom-mcp` package

## Risk Assessment
- **Risk**: Users may not see the deprecation warning if they capture stdout
  - **Mitigation**: Use `console.warn` (stderr) and also update README prominently

- **Risk**: Wrapper adds extra process overhead and startup time
  - **Mitigation**: Acceptable trade-off for migration period; document in warning message

- **Risk**: Breaking changes if new package CLI differs from legacy
  - **Mitigation**: Ensure new package maintains CLI compatibility or update wrapper to translate commands

- **Risk**: npm deprecate command requires proper authentication
  - **Mitigation**: Verify npm login and package ownership before executing

- **Risk**: Users on air-gapped networks cannot reach npx to download new package
  - **Mitigation**: Document this limitation in README; suggest direct installation of new package

## Files/Packages Affected
- `maproom-mcp/bin/maproom-mcp.js` (or equivalent bin file) - update with wrapper script
- `maproom-mcp/package.json` - bump version, update description to mention deprecation
- `maproom-mcp/README.md` - add migration guide and deprecation notice
- `maproom-mcp/CHANGELOG.md` - document deprecation wrapper addition
- npm registry entry for `maproom-mcp` - add deprecation flag
