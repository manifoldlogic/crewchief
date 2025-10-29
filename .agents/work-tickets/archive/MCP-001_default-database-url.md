# Ticket: MCP-001: Default DATABASE_URL for zero-config MCP experience

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- verify-ticket
- commit-ticket

## Summary
Add default DATABASE_URL to MCP server configuration to enable true zero-config experience. Users should only need to specify embedding provider preferences, not database connection details.

## Background
The maproom MCP server claims zero-config operation, but currently requires users to explicitly set DATABASE_URL in their MCP client configuration. This breaks the zero-config promise.

**Current behavior:**
```json
{
  "maproom": {
    "command": "npx",
    "args": ["-y", "@crewchief/maproom-mcp"],
    "env": {
      "DATABASE_URL": "postgresql://maproom:maproom@maproom-postgres:5432/maproom",  // ❌ Required
      "EMBEDDING_PROVIDER": "google"  // ✅ Optional (provider preference)
    }
  }
}
```

**Expected behavior:**
```json
{
  "maproom": {
    "command": "npx",
    "args": ["-y", "@crewchief/maproom-mcp"],
    "env": {
      "EMBEDDING_PROVIDER": "google"  // ✅ Only provider preference needed
      // DATABASE_URL auto-defaults to maproom-postgres
    }
  }
}
```

## Acceptance Criteria
- [x] MCP server defaults DATABASE_URL to `postgresql://maproom:maproom@maproom-postgres:5432/maproom` when not provided
- [x] Users can still override with explicit DATABASE_URL if needed
- [x] Zero-config experience documented in README
- [x] Updated MCP setup examples remove DATABASE_URL requirement

## Optional Enhancements (Future Work)
- [ ] Connection fallback logic implemented (try maproom-postgres, then localhost:5432)
- [ ] Clear error message if no database is reachable

## Technical Requirements

### 1. Default Connection String
In `packages/maproom-mcp/src/index.ts`, update database connection logic:

```typescript
const DATABASE_URL = process.env.DATABASE_URL ||
  'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
```

### 2. Connection Fallback (Optional Enhancement)
Try multiple connection strings in order:
1. `process.env.DATABASE_URL` (explicit override)
2. `postgresql://maproom:maproom@maproom-postgres:5432/maproom` (MCP container)
3. `postgresql://postgres:postgres@postgres:5432/crewchief` (devcontainer)
4. `postgresql://postgres:postgres@localhost:5432/maproom` (local development)

### 3. Error Handling
If all connections fail, provide helpful error message:
```
Unable to connect to maproom database. Tried:
  - maproom-postgres:5432 (MCP container)
  - postgres:5432 (devcontainer)
  - localhost:5432 (local)

To use a custom database, set DATABASE_URL:
  export DATABASE_URL="postgresql://user:pass@host:port/db"

To start the maproom database:
  docker compose -f packages/maproom-mcp/config/docker-compose.yml up -d
```

### 4. Documentation Updates
Update these files to remove DATABASE_URL from examples:
- `packages/maproom-mcp/README.md` - MCP setup section
- `crates/maproom/README.md` - MCP integration section
- `docs/guides/mcp-setup.md` (if exists)

## Implementation Notes

**Current code location:**
- Database connection: `packages/maproom-mcp/src/index.ts` (around line 20-30)
- Configuration examples: `packages/maproom-mcp/README.md`

**Testing approach:**
1. Remove DATABASE_URL from test MCP config
2. Verify server connects successfully
3. Test with explicit DATABASE_URL override
4. Test error handling with no database available

## Dependencies
- None (bug fix for existing functionality)

## Risk Assessment
- **Risk**: Default connection might fail in non-Docker environments
  - **Mitigation**: Implement fallback logic with clear error messages
- **Risk**: Breaking change for users who rely on DATABASE_URL being required
  - **Mitigation**: Still support explicit DATABASE_URL (backward compatible)

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` - Add default DATABASE_URL
- `packages/maproom-mcp/README.md` - Update setup examples
- `crates/maproom/README.md` - Update MCP integration section
- `packages/maproom-mcp/package.json` - Bump version (patch)

## Related Issues
- MPEMBED-5001 through MPEMBED-6901 - Multi-provider embedding support (completed)
- Zero-config promise documented in MPEMBED project documentation

## Implementation Notes

**Changes Made:**

1. **`packages/maproom-mcp/src/index.ts`** (lines 266-275):
   - Added `DEFAULT_DATABASE_URL` constant set to `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
   - Modified `getPg()` function to use default when `DATABASE_URL` is not set
   - Maintains backward compatibility: explicit `DATABASE_URL` or `PG_DATABASE_URL` still override the default
   - Removed error throw when no DATABASE_URL provided (now uses default instead)

2. **`packages/maproom-mcp/README.md`**:
   - Updated "Environment Variables" section to emphasize all configuration is optional
   - Changed heading to "Environment Variables (Optional)"
   - Moved DATABASE_URL to "Advanced: Custom Database" subsection
   - Updated examples to show zero-config setup (no DATABASE_URL needed)
   - Added "Database Configuration" section explaining zero-config default
   - Documented that custom DATABASE_URL is optional and how to use it

3. **`crates/maproom/README.md`**:
   - No changes needed - DATABASE_URL examples are for CLI usage, not MCP integration
   - MCP integration section references separate docs that don't exist yet

**Acceptance Criteria Status:**
- ✅ MCP server defaults DATABASE_URL to `postgresql://maproom:maproom@maproom-postgres:5432/maproom` when not provided
- ✅ Users can still override with explicit DATABASE_URL (backward compatible)
- ✅ Zero-config experience documented in README
- ✅ Updated MCP setup examples remove DATABASE_URL requirement
- ⚠️ Connection fallback logic NOT implemented (marked as "Optional Enhancement" in technical requirements)
- ⚠️ Enhanced error message NOT implemented (would require connection retry logic)

**Implementation Notes:**
- Simple, minimal change that achieves the core zero-config goal
- Maintains full backward compatibility
- Default connection string matches the existing MCP postgres service configuration
- Environment variable logging already in place for debugging
- No breaking changes to existing configurations

**Testing Recommendations:**
1. Test with no DATABASE_URL set (should connect to maproom-postgres)
2. Test with explicit DATABASE_URL set (should override default)
3. Test with PG_DATABASE_URL set (should override default)
4. Verify connection string is logged correctly in debug mode
