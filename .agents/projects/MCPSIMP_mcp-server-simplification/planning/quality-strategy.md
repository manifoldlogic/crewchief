# Quality Strategy: MCP Server Simplification

## Testing Philosophy

This project is primarily a deletion and simplification effort. The testing strategy focuses on verifying that the remaining functionality works correctly, not on achieving coverage metrics.

## Critical Paths

### Path 1: MCP Server Startup
```
npx @crewchief/maproom-mcp
  → resolveDatabase() picks correct URL
  → import('../dist/index.js') runs
  → MCP server responds to JSON-RPC
```

**Tests Required**:
1. Explicit `MAPROOM_DATABASE_URL` is used when set
2. DevContainer detection uses container hostname
3. Default falls back to localhost:5433
4. MCP server starts and responds to `initialize` request

### Path 2: Database Connectivity
```
MCP server starts
  → First tool call triggers daemon spawn
  → Daemon connects to database
  → Query executes successfully
```

**Tests Required**:
1. `status` tool returns database connection info
2. Error message when database unavailable
3. Daemon spawns successfully with correct env vars

### Path 3: Tool Handlers (Existing)
The existing tool handlers should continue to work unchanged. Existing tests cover this.

## Test Strategy

### Unit Tests

**New Tests (~5)**:
```javascript
// test/unit/resolve-database.test.js
describe('resolveDatabase', () => {
  test('uses MAPROOM_DATABASE_URL when set')
  test('uses container hostname when IN_DEVCONTAINER=true')
  test('defaults to localhost:5433')
})
```

### Integration Tests

**Smoke Test**:
```bash
# Database available
npx @crewchief/maproom-mcp &
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | nc localhost <port>
# Expect: valid JSON-RPC response

# Database unavailable
MAPROOM_DATABASE_URL=postgresql://bad@host:5432/nope npx @crewchief/maproom-mcp
# Expect: Error message, exit code 1
```

**VSCode Extension Test**:
- Extension starts PostgreSQL container
- MCP server connects successfully
- Search tool returns results

### MCPConfigWriter Tests

**New tests for mcp-writer.ts**:
```typescript
describe('MCPConfigWriter.buildEnvironment', () => {
  test('always includes MAPROOM_DATABASE_URL')
  test('always includes MAPROOM_EMBEDDING_PROVIDER')
  test('includes OPENAI_API_KEY for openai provider')
  test('includes GOOGLE_APPLICATION_CREDENTIALS for google provider')
  test('no extra env vars for ollama provider')
})
```

### Manual Verification Checklist

**MCP Server**:
- [ ] `npx @crewchief/maproom-mcp` with database → Server starts
- [ ] `npx @crewchief/maproom-mcp` without database → Clear error
- [ ] DevContainer with `IN_DEVCONTAINER=true` → Uses container hostname
- [ ] MCP tools (search, open, status) function correctly

**VSCode Extension**:
- [ ] Extension starts only PostgreSQL container (not Ollama or MCP)
- [ ] Generated mcp.json includes `MAPROOM_DATABASE_URL`
- [ ] Generated mcp.json includes `MAPROOM_EMBEDDING_PROVIDER`
- [ ] MCP server connects successfully after extension setup

**DevContainer Testing**:
- [ ] Test in VS Code Dev Container
- [ ] Test in GitHub Codespaces (if available)
- [ ] Verify `IN_DEVCONTAINER` detection works or explicit URL override works

## Risk Mitigation

### Risk: Breaking Existing Users
**Mitigation**:
- Major version bump (3.0.0)
- Clear error message if database not found
- VSCode extension unchanged for those users

### Risk: DevContainer Detection Fails
**Mitigation**:
- `IN_DEVCONTAINER` is a well-established convention
- Explicit `MAPROOM_DATABASE_URL` always available as override

### Risk: Daemon Spawn Breaks
**Mitigation**:
- Daemon spawning code unchanged
- Existing tests cover this path

## What NOT to Test

### Don't Test Deleted Code
The Docker orchestration, config management, and Ollama handling are removed. No need to test what doesn't exist.

### Don't Test External Dependencies
- PostgreSQL works (database team's problem)
- npx works (npm's problem)
- Rust daemon works (existing tests cover it)

## MVP Testing Scope

| Component | Test Type | Coverage |
|-----------|-----------|----------|
| `resolveDatabase()` | Unit | 100% (3 paths) |
| CLI entry point | Integration | Smoke test |
| MCP server startup | Integration | Smoke test |
| Tool handlers | Existing | Already covered |
| `MCPConfigWriter.buildEnvironment()` | Unit | 100% (all providers) |
| Extension docker-compose | Manual | PostgreSQL-only verification |

## Success Criteria

1. **All existing MCP tool tests pass** - No regression
2. **New unit tests pass** - Database resolution works
3. **MCPConfigWriter tests pass** - Env vars correctly generated
4. **Manual smoke test passes** - Server starts with database
5. **Error handling works** - Clear message without database
6. **DevContainer works** - Auto-detects container network
7. **Extension starts PostgreSQL only** - No Ollama or MCP containers
