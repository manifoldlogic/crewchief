# Ticket: CFGVER-4001: Integrate version checking into CLI startup

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- code-reviewer
- verify-ticket
- commit-ticket

## Summary
Integrate version checking into the CLI entry point so that every run of `npx -y @crewchief/maproom-mcp@latest` automatically checks for config updates before starting the MCP server. This is the user-facing integration that makes automatic updates work seamlessly.

## Background
The CLI entry point (`packages/maproom-mcp/bin/cli.cjs`) is where users interact with Maproom MCP. Without version checking at startup, users get stale configuration and broken functionality. By integrating needsConfigUpdate() and updateConfigs() at the entry point, we ensure configuration stays synchronized with the installed package version automatically.

This completes Phase 4's objective: "Integrate version management into CLI entry point" ensuring users always have correct configuration without manual intervention.

Reference: `architecture.md` lines 214-238 for CLI integration details.

## Acceptance Criteria
- [ ] CLI checks for updates before starting MCP server by calling needsConfigUpdate()
- [ ] If update needed, calls updateConfigs() synchronously (awaits completion)
- [ ] If update fails, shows error message and exits with non-zero code
- [ ] If no update needed, continues to normal MCP server startup
- [ ] Async operations are handled correctly with async/await pattern
- [ ] Process exit code is 1 on update failure, 0 on success

## Technical Requirements
- Modify: `packages/maproom-mcp/bin/cli.cjs`
- Import config-manager functions: `{ needsConfigUpdate, updateConfigs }`
- Add update check at beginning of main() before MCP server startup
- Use async/await pattern for proper error handling
- Set process.exit(1) on update failure
- Preserve existing CLI behavior when no update needed

## Implementation Notes

**CLI Integration Pattern:**
```javascript
#!/usr/bin/env node

const { needsConfigUpdate, updateConfigs } = require('../dist/config-manager.js');

async function main() {
  try {
    // Check for config updates
    const updateCheck = needsConfigUpdate();

    if (updateCheck.needsUpdate) {
      // Perform update synchronously before starting server
      await updateConfigs();
    }

    // Continue with normal CLI flow
    // ... existing MCP server startup code
  } catch (error) {
    console.error(`Fatal error: ${error.message}`);
    process.exit(1);
  }
}

main().catch(error => {
  console.error(`Unhandled error: ${error.message}`);
  process.exit(1);
});
```

**Error Handling Strategy:**
- Update failure: Show error, don't start MCP server (fail-safe approach)
- Rollback failure: Show recovery instructions with actionable commands
- Docker errors: Show actionable message with next steps
- Any error during update blocks server startup (better than broken config)

**Architecture Reference:**
- CLI integration: `architecture.md` lines 214-238
- Error handling: `architecture.md` lines 274-298

## Dependencies
- **CFGVER-1002**: needsConfigUpdate() function must exist
- **CFGVER-2002**: updateConfigs() function must exist
- All Phase 1, 2, and 3 tickets must be complete

## Risk Assessment
- **Risk**: Slow startup due to version checking
  - **Mitigation**: needsConfigUpdate() is fast (only reads version file and computes hashes)
  - **Impact**: Acceptable trade-off for automatic correctness

- **Risk**: Update failure blocks MCP server startup
  - **Mitigation**: This is intentional - better to fail than run with broken config
  - **Impact**: User gets clear error message with recovery steps

- **Risk**: Async handling errors in CLI entry point
  - **Mitigation**: Use async/await with try/catch, test thoroughly
  - **Impact**: Unhandled promise rejections could crash without error message

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/bin/cli.cjs` (main entry point)
- **Import from**: `packages/maproom-mcp/src/config-manager.ts`
- **No changes to**: MCP server startup logic (preserve existing behavior)
