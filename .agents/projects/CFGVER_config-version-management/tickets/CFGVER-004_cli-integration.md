# Ticket: CFGVER-004: CLI integration with version checking

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - manual testing complete

## Agents
- mcp-tools-engineer

## Summary
Integrate version checking into CLI startup flow. On every CLI run, check if configs need updating and update them automatically before proceeding.

## Background
The CLI entry point (`bin/cli.cjs`) needs to check for config updates before starting the MCP server. This ensures users always have current configs without manual intervention.

## Acceptance Criteria
- [ ] CLI checks for updates on every run
- [ ] Updates happen automatically when needed
- [ ] Clear progress messages during update
- [ ] Helpful error message if update fails
- [ ] Normal CLI flow continues after successful update
- [ ] Update only runs when version changes (not every time)

## Technical Requirements

**Module:** Modify `packages/maproom-mcp/bin/cli.cjs`

**Import config-manager:**
```javascript
const { needsConfigUpdate, updateConfigs } = require('../dist/config-manager.js');
```

**Add version check before main logic:**
```javascript
#!/usr/bin/env node

const { needsConfigUpdate, updateConfigs } = require('../dist/config-manager.js');

async function main() {
  try {
    // Check for config updates
    if (needsConfigUpdate()) {
      console.log('\n📦 Maproom MCP configs need updating...\n');

      try {
        updateConfigs();
        console.log('\n✅ Configs updated successfully!\n');
      } catch (updateError) {
        console.error('\n❌ Failed to update configs:', updateError.message);
        console.error('\n💡 Recovery: Delete ~/.maproom-mcp/ and re-run this command\n');
        process.exit(1);
      }
    }

    // Continue with existing CLI logic...
    // (existing code for starting MCP server)

  } catch (error) {
    console.error('Error:', error.message);
    process.exit(1);
  }
}

main();
```

## User Experience

**First run (no cached configs):**
```
$ npx -y @crewchief/maproom-mcp@latest

📦 Maproom MCP configs need updating...

  📋 Copied fresh configs from package...

✅ Configs updated successfully!

Starting Maproom MCP server...
```

**Version change (1.1.12 → 1.2.0):**
```
$ npx -y @crewchief/maproom-mcp@latest

📦 Maproom MCP configs need updating...

  💾 Preserving user .env file...
  📋 Copied fresh configs from package...
  ✅ Restored user .env file

✅ Configs updated successfully!

Starting Maproom MCP server...
```

**No update needed (same version):**
```
$ npx -y @crewchief/maproom-mcp@latest

Starting Maproom MCP server...
```

## Manual Testing

```bash
# Test 1: First run
rm -rf ~/.maproom-mcp
npx -y @crewchief/maproom-mcp@latest
# Expected: Update message, configs created, server starts

# Test 2: Second run (no update)
npx -y @crewchief/maproom-mcp@latest
# Expected: No update message, server starts immediately

# Test 3: Simulate version change
echo "0.0.1" > ~/.maproom-mcp/.version
npx -y @crewchief/maproom-mcp@latest
# Expected: Update message, configs refreshed, server starts

# Test 4: With custom .env
echo "MY_VAR=test" > ~/.maproom-mcp/.env
echo "0.0.1" > ~/.maproom-mcp/.version
npx -y @crewchief/maproom-mcp@latest
# Expected: Update message, .env preserved, server starts
```

## Build Requirements

**Before testing CLI:**
```bash
cd packages/maproom-mcp
pnpm build
# This compiles src/config-manager.ts → dist/config-manager.js
```

## Error Handling

If update fails:
```
❌ Failed to update configs: ENOSPC: no space left on device

💡 Recovery: Delete ~/.maproom-mcp/ and re-run this command
```

User can then:
1. Free up disk space
2. Delete `~/.maproom-mcp/`
3. Re-run `npx -y @crewchief/maproom-mcp@latest`

## Dependencies
- CFGVER-001, 002, 003 (all config-manager functions)

## Files Affected
- **Modify:** `packages/maproom-mcp/bin/cli.cjs`
- **Import:** `packages/maproom-mcp/dist/config-manager.js`

## Estimated Time
2-3 hours
