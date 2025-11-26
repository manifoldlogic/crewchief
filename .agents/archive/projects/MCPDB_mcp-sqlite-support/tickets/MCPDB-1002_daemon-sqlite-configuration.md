# Ticket: MCPDB-1002: Daemon SQLite Configuration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (TypeScript implementation)
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update `daemon.ts` to use `resolveDatabaseConfig()` for database URL resolution and add SQLite-specific validation with helpful error messages when the SQLite file is missing.

## Background
The daemon client currently reads `MAPROOM_DATABASE_URL` directly from environment. With SQLite support, we need to:
1. Use the new `resolveDatabaseConfig()` for type-aware URL resolution
2. Validate SQLite file existence before spawning daemon
3. Provide helpful error messages guiding users to create an index

**Plan Reference:** Phase 2 - Daemon Integration (plan.md)

## Acceptance Criteria
- [x] Daemon uses `resolveDatabaseConfig()` instead of raw environment variable
- [x] SQLite file existence validated before daemon spawn (when type is `sqlite`)
- [x] Missing SQLite file produces helpful error with instructions
- [x] Error message includes path and suggests `crewchief-maproom scan` command
- [x] Daemon correctly receives SQLite URL in environment
- [x] PostgreSQL path unchanged (no regressions)

## Technical Requirements

### Import and Use resolveDatabaseConfig
```typescript
import { resolveDatabaseConfig } from './utils/resolve-database.js'
```

### SQLite File Validation
Before spawning daemon, if config type is `sqlite`:
```typescript
const config = resolveDatabaseConfig()

if (config.type === 'sqlite' && config.path) {
  if (!existsSync(config.path)) {
    throw new Error(
      `SQLite database not found: ${config.path}\n\n` +
      `To create an index:\n` +
      `  crewchief-maproom scan --path /your/repo\n\n` +
      `Or specify a different database:\n` +
      `  export MAPROOM_DATABASE_URL=sqlite:///path/to/your.db`
    )
  }
}
```

### Pass Resolved URL to Daemon
```typescript
daemonClient = new DaemonClient({
  binaryPath,
  env: {
    MAPROOM_DATABASE_URL: config.url,  // Use resolved URL, not raw env
    // ... other env vars
  }
})
```

## Implementation Notes

### File to Modify
`packages/maproom-mcp/src/daemon.ts`

### Current Code Pattern (around line 80-100)
```typescript
// Current (to be updated):
if (!process.env.MAPROOM_DATABASE_URL) {
  throw new Error('MAPROOM_DATABASE_URL environment variable is required')
}

// Updated:
const config = resolveDatabaseConfig()
// Validation and use config.url
```

### Error Message Requirements
Error messages should be:
1. Clear about what's wrong (file not found)
2. Include the actual path checked
3. Provide actionable next steps (scan command)
4. Offer alternatives (different database URL)

### Testing Approach
- Mock `existsSync` to test missing file scenario
- Verify error message contains expected content
- Verify PostgreSQL path doesn't do file existence check

## Dependencies
- **MCPDB-1001**: Must complete first (provides `resolveDatabaseConfig()`)

## Risk Assessment
- **Risk**: Breaking daemon startup for PostgreSQL users
  - **Mitigation**: Only perform file existence check for SQLite type; PostgreSQL path unchanged
- **Risk**: Error message not helpful enough
  - **Mitigation**: Include specific path and concrete commands in error message

## Files/Packages Affected
- `packages/maproom-mcp/src/daemon.ts` (modify)
- `packages/maproom-mcp/tests/unit/daemon.test.ts` (create/modify for error handling tests)
