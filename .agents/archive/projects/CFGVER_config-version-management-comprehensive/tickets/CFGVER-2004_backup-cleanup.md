# Ticket: CFGVER-2004: Implement automatic cleanup of old backups

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement automatic cleanup logic that removes old backup directories to prevent disk space accumulation. Keep only the 5 most recent backups.

## Background
Over time, backup directories accumulate in `~/.maproom-mcp/backups/` and waste disk space. Each backup contains ~50KB of config files, so 100 backups = ~5MB. While not huge, this grows unnecessarily and clutters the backups directory.

We should automatically clean up old backups after successful updates, keeping only the most recent 5 backups as a safety buffer. This provides sufficient history for recovery without unlimited growth.

Reference: `architecture.md` lines 170-191 for backup strategy and cleanup timing.

## Acceptance Criteria
- [ ] Function `cleanupOldBackups()` lists all backup directories in `~/.maproom-mcp/backups/`
- [ ] Sorts backup directories by timestamp (newest first)
- [ ] Keeps the 5 most recent backups
- [ ] Deletes older backup directories recursively
- [ ] Handles errors gracefully (logs warning, doesn't fail update)
- [ ] Logs number of backups cleaned up
- [ ] Skips cleanup if 5 or fewer backups exist

## Technical Requirements
- Backup directory location: `~/.maproom-mcp/backups/`
- Keep count: 5 (configurable constant `MAX_BACKUPS = 5`)
- Use `fs.promises.readdir()` to list backup directories
- Parse ISO 8601 timestamps from directory names (format: `YYYY-MM-DDTHH-mm-ss.sssZ`)
- Sort by directory name (ISO 8601 sorts lexicographically)
- Use `fs.promises.rm(dir, { recursive: true, force: true })` to remove old backups
- Call after successful config update (don't call if update failed)
- Log deletion actions for debugging
- Don't fail update if cleanup fails (best effort)

## Implementation Notes
**Module Location:**
- Modify: `packages/maproom-mcp/src/config-manager.ts`
- Add function: `cleanupOldBackups()`
- Add constant: `MAX_BACKUPS = 5`

**Cleanup Flow:**
1. Read all entries in `~/.maproom-mcp/backups/`
2. Filter to directories only (ignore files)
3. Filter to valid timestamp format (YYYY-MM-DDTHH-mm-ss.sssZ pattern)
4. Sort by directory name (descending - newest first)
5. If count <= MAX_BACKUPS: skip cleanup
6. For each directory after the 5th:
   - Delete recursively
   - Log: "Deleted old backup: TIMESTAMP"
7. Log: "Cleaned up N old backups"

**Error Handling:**
- If backups directory doesn't exist: skip cleanup (no error)
- If permission denied on delete: log warning, continue
- If any deletion fails: log warning, continue with remaining
- Never throw error (cleanup is best effort)

**Logging Examples:**
```
Cleaned up 3 old backups
Deleted old backup: 2024-10-15T12-00-00.000Z
Deleted old backup: 2024-10-14T09-30-00.000Z
Deleted old backup: 2024-10-13T16-45-00.000Z
```

**Edge Cases:**
- No backups directory: skip cleanup
- Fewer than 6 backups: skip cleanup
- Permission errors: log warning, continue
- Malformed directory names: skip, don't crash

**Call Site:**
Call from `updateConfigs()` after successful update:
```javascript
try {
  await updateConfigs();
  await cleanupOldBackups(); // Best effort, don't fail on error
} catch (error) {
  // ... handle update error
}
```

**Security Considerations:**
- Validate backup directory paths before deletion (must be within `~/.maproom-mcp/backups/`)
- Ensure deletion stays within backups directory (no path traversal)
- Handle permission errors gracefully (don't expose system internals)
- Reference: `security-review.md` lines 188-236 for cleanup security

## Dependencies
- CFGVER-2001 (needs backup directory structure and timestamp format)

## Risk Assessment
- **Risk**: Deleting wrong directory - incorrect path validation
  - **Mitigation**: Validate all paths are within `~/.maproom-mcp/backups/` before deletion
  - **Severity**: Critical (data loss if wrong directory deleted)

- **Risk**: Permission errors - cannot delete backups
  - **Mitigation**: Log warning and continue (non-critical operation)
  - **Severity**: Low (backups accumulate but system still works)

- **Risk**: Regex injection in directory names
  - **Mitigation**: Use simple string matching for timestamp format, not regex with user input
  - **Severity**: Low (unlikely with ISO 8601 format)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `cleanupOldBackups()` function)
- **Read**: Backup directories in `~/.maproom-mcp/backups/`
- **Delete**: Old backup directories in `~/.maproom-mcp/backups/` (keep only 5 most recent)
