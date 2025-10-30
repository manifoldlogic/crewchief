# Ticket: MCPSTART-2002: Implement docker-compose.yml verification on startup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Verify docker-compose.yml uses environment variable syntax (not hardcoded values) before starting services.

## Background
MCP-008 and MCP-011 updated docker-compose.yml to use ${EMBEDDING_PROVIDER:-ollama} syntax, but users may have old hardcoded configs. This ticket adds verification that fails fast with clear error if the config file has hardcoded EMBEDDING_PROVIDER values that would override environment variables.

Implements **Phase 2.2** from MCPSTART_ARCHITECTURE.md - Docker Compose File Verification.

## Acceptance Criteria
- [x] Function verifyDockerComposeConfig() checks for hardcoded EMBEDDING_PROVIDER
- [x] Detects regex pattern: `EMBEDDING_PROVIDER:\s*['"]?ollama['"]?\s*$`
- [x] Checks for env var syntax: `\$\{EMBEDDING_PROVIDER[:\-]`
- [x] If hardcoded found WITHOUT env var syntax, exits with clear error
- [x] Error message shows config file location
- [x] Called after config file auto-update in setup phase

## Technical Requirements
- Read docker-compose.yml from CONFIG_DIR
- Check for patterns:
  - BAD: `EMBEDDING_PROVIDER: ollama` (hardcoded)
  - GOOD: `EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}` (env var)
- If hardcoded pattern found AND no env var syntax: fail with error
- Exit code 1 with clear message to user

## Implementation Notes
See MCPSTART_ARCHITECTURE.md lines 134-158 for detailed implementation guidance.

The verification function should:
1. Read docker-compose.yml file from CONFIG_DIR
2. Search for hardcoded EMBEDDING_PROVIDER patterns
3. Verify presence of environment variable syntax
4. Provide actionable error message if validation fails

Error message format:
```
❌ ERROR: docker-compose.yml contains hardcoded EMBEDDING_PROVIDER
   File: /path/to/docker-compose.yml

   Your config file has:
     EMBEDDING_PROVIDER: ollama

   It should be:
     EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}

   This was fixed in MCP-011. Please update your config file or run:
     npx @crewchief/maproom-mcp setup
```

## Dependencies
- MCPSTART-2001 (env propagation must exist first)

## Risk Assessment
- **Risk**: Low - fail-fast verification prevents silent failures
  - **Mitigation**: Clear error messages guide user to fix configuration

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs`

## Implementation Summary

### Changes Made

1. **Added `verifyDockerComposeConfig()` function** (lines 328-373 in cli.cjs):
   - Reads docker-compose.yml from CONFIG_DIR
   - Uses regex to detect hardcoded `EMBEDDING_PROVIDER: ollama` pattern
   - Uses regex to detect environment variable syntax `${EMBEDDING_PROVIDER:-...}`
   - Fails with exit code 1 if hardcoded found WITHOUT env var syntax
   - Provides clear, actionable error message showing file location and fix instructions
   - Logs diagnostic success message when verification passes

2. **Integrated verification into main() startup sequence** (line 852 in cli.cjs):
   - Called immediately after `setupConfigDirectory()` function
   - Positioned after auto-update but before `startDockerCompose()`
   - Ensures config is correct before attempting to start services

3. **Created comprehensive test suite** (packages/maproom-mcp/tests/docker-compose-verification.test.ts):
   - 16 tests covering all acceptance criteria
   - Tests hardcoded value detection (with/without quotes, whitespace)
   - Tests environment variable syntax detection (colon, dash, default value)
   - Tests mixed content scenarios (comments, multiple services)
   - Tests edge cases (empty content, other variables, multiline)
   - Tests regression scenarios (MCP-008, MCP-011 migrations)
   - All tests pass successfully

4. **Fixed vitest configuration** (packages/maproom-mcp/vitest.config.ts):
   - Added missing `minThreads: 1` parameter to resolve tinypool error
   - Ensures tests can run successfully

5. **Added vitest dev dependency** (packages/maproom-mcp/package.json):
   - Installed vitest@^1.6.1 to enable test execution

### Regex Patterns Used

- **Hardcoded detection**: `/EMBEDDING_PROVIDER:\s*['"]?ollama['"]?\s*$/m`
  - Matches: `EMBEDDING_PROVIDER: ollama`, `EMBEDDING_PROVIDER: 'ollama'`, `EMBEDDING_PROVIDER: "ollama"`
  - Multiline mode (`m` flag) to match across entire file

- **Environment variable detection**: `/\$\{EMBEDDING_PROVIDER[:\-]/`
  - Matches: `${EMBEDDING_PROVIDER:-...}`, `${EMBEDDING_PROVIDER:...}`, `${EMBEDDING_PROVIDER-...}`
  - Detects presence of env var syntax anywhere in file

### Error Message Format

Follows the exact format specified in ticket (lines 47-59):
```
❌ ERROR: docker-compose.yml contains hardcoded EMBEDDING_PROVIDER
   File: /home/vscode/.maproom-mcp/docker-compose.yml

   Your config file has:
     EMBEDDING_PROVIDER: ollama

   It should be:
     EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}

   This was fixed in MCP-011. Please update your config file or run:
     npx @crewchief/maproom-mcp setup
```

### Testing

All acceptance criteria verified:
- ✅ Function exists and checks for hardcoded EMBEDDING_PROVIDER
- ✅ Detects regex pattern correctly (tested with 16 test cases)
- ✅ Checks for env var syntax correctly
- ✅ Exits with code 1 and clear error when validation fails
- ✅ Error message shows config file location
- ✅ Called after setupConfigDirectory() in main() startup sequence

Test execution:
```bash
cd packages/maproom-mcp
./node_modules/.bin/vitest run tests/docker-compose-verification.test.ts
# Result: 16 tests passed
```

### Build/Run Commands

The implementation is in a CommonJS file (cli.cjs) and requires no build step. To test manually:

```bash
# Test with correct config (should succeed)
npx @crewchief/maproom-mcp

# To test failure scenario, temporarily modify config:
# 1. Edit ~/.maproom-mcp/docker-compose.yml
# 2. Change: EMBEDDING_PROVIDER: ${EMBEDDING_PROVIDER:-ollama}
# 3. To: EMBEDDING_PROVIDER: ollama
# 4. Run: npx @crewchief/maproom-mcp
# 5. Should see error and exit code 1
```

### Platform Notes

This implementation is platform-agnostic and works on:
- Linux (tested in devcontainer)
- macOS (no OS-specific code)
- Windows (uses Node.js fs/path modules, platform-independent)

No Docker build required for this change - it modifies the CLI wrapper script only.
