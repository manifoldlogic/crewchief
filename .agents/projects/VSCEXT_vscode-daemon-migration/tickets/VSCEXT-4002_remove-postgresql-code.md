# Ticket: VSCEXT-4002: Remove PostgreSQL code and settings

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (N/A - deletion only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- verify-ticket
- commit-ticket

## Summary
Delete all PostgreSQL-specific code and remove PostgreSQL-related settings from package.json. SQLite is the only supported database after this change.

## Background
The extension originally supported PostgreSQL via Docker containers. With the architectural shift to SQLite-only, all PostgreSQL-specific code and configuration should be removed.

Reference: planning/plan.md - Phase 4, Ticket 4002
Reference: planning/architecture.md - Settings Changes

## Acceptance Criteria
- [ ] `src/services/postgres-checker.ts` deleted
- [ ] PostgreSQL settings removed from `package.json` contributes.configuration
- [ ] No PostgreSQL references in TypeScript code
- [ ] SQLite remains as only database option
- [ ] TypeScript compiles without errors

## Technical Requirements

**Files to Delete**:
- `src/services/postgres-checker.ts` - PostgreSQL health checker

**Settings to Remove from package.json** (contributes.configuration):
- `maproom.database.provider` (if exists)
- `maproom.database.host`
- `maproom.database.port`
- `maproom.database.user`
- `maproom.database.password`
- `maproom.database.name`

**Settings to Keep** (SQLite-related):
- `maproom.database.sqlitePath` - Path to SQLite database
- `maproom.embedding.provider` - Embedding provider selection
- `maproom.embedding.model` - Model name

**Verification Commands**:
```bash
# Find PostgreSQL references
grep -r "postgres" packages/vscode-maproom/src/ --include="*.ts" -i
grep -r "pg" packages/vscode-maproom/src/ --include="*.ts"
grep -r "database.host" packages/vscode-maproom/ --include="*.json"
grep -r "database.port" packages/vscode-maproom/ --include="*.json"

# Verify TypeScript compilation
cd packages/vscode-maproom && pnpm build
```

## Implementation Notes
1. First, audit package.json for PostgreSQL settings
2. Delete postgres-checker.ts
3. Remove PostgreSQL settings from package.json
4. Fix any imports that referenced deleted files
5. Update any code that checked for PostgreSQL provider
6. Verify compilation and grep for remaining references

```json
// Settings to REMOVE from package.json contributes.configuration
{
  "maproom.database.provider": "...",  // REMOVE
  "maproom.database.host": "...",       // REMOVE
  "maproom.database.port": "...",       // REMOVE
  "maproom.database.user": "...",       // REMOVE
  "maproom.database.password": "...",   // REMOVE
  "maproom.database.name": "..."        // REMOVE
}
```

## Dependencies
- VSCEXT-4001 (Docker removal should happen first)

## Risk Assessment
- **Risk**: Users lose PostgreSQL configuration
  - **Mitigation**: SQLite is a fresh local index; PostgreSQL data not migrated
- **Risk**: Hidden PostgreSQL dependencies
  - **Mitigation**: Comprehensive grep search before and after

## Files/Packages Affected
- `packages/vscode-maproom/src/services/postgres-checker.ts` - Delete
- `packages/vscode-maproom/package.json` - Remove PostgreSQL settings
- `packages/vscode-maproom/src/services/index.ts` - Remove postgres exports (if exists)
