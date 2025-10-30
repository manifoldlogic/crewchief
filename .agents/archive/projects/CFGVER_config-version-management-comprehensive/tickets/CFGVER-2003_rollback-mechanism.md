# Ticket: CFGVER-2003: Implement config rollback on update failure

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
Implement rollback mechanism that restores config files from backup when any step of the update process fails. This is the critical safety net that prevents users from being stuck with broken configs.

## Background
If any step of the update process fails (backup succeeds but config copy fails, Docker restart fails, etc.), we must restore the backup to leave the system in a working state. This rollback mechanism is what makes the update process safe - no matter what goes wrong, users can recover.

The rollback must be reliable, provide clear feedback, and handle edge cases like missing or corrupted backups.

Reference: `architecture.md` lines 155-169 for rollback logic in the update error handler.

## Acceptance Criteria
- [ ] Function `rollbackConfigs(backupDir)` restores all files from backup directory to cache directory
- [ ] Handles missing backup directory gracefully (logs error, provides recovery steps)
- [ ] Verifies restored files match backup (optional hash check for integrity)
- [ ] Sets correct permissions on restored files (0o600)
- [ ] Returns success/failure status
- [ ] Provides clear error message if rollback fails
- [ ] Logs all restored file paths for debugging

## Technical Requirements
- Function takes backup directory path as parameter
- Verify backup directory exists before attempting restore
- Copy all files from backup directory to cache directory
- Use `fs.promises.copyFile()` for async copying
- Use `fs.lstatSync()` to verify backup files are regular files (not symlinks)
- Set permissions to 0o600 after copying each file
- Optional: Verify restored files by comparing hashes
- Log each file restoration for audit trail

## Implementation Notes
**Module Location:**
- Modify: `packages/maproom-mcp/src/config-manager.ts`
- Add function: `rollbackConfigs(backupDir)`

**Rollback Flow:**
1. Verify backup directory exists:
   - If missing: throw error with manual recovery instructions
2. List all files in backup directory
3. For each backup file:
   - Verify it's a regular file (not symlink)
   - Copy to cache directory (overwrite current file)
   - Set permissions to 0o600
   - Log: "Restored: filename"
4. Return success status

**Error Handling:**
- Missing backup directory:
  ```
  Error: Backup directory not found: /path/to/backup

  Manual recovery steps:
  1. Check for other backups: ls ~/.maproom-mcp/backups/
  2. If found, copy files manually:
     cp ~/.maproom-mcp/backups/TIMESTAMP/* ~/.maproom-mcp/
  3. If no backups, reinstall package:
     npm install -g @crewchief/maproom-mcp
  ```

- Permission denied:
  ```
  Error: Permission denied when restoring config files

  Try:
  chmod 700 ~/.maproom-mcp/
  chmod 600 ~/.maproom-mcp/*
  ```

- Corrupted backup:
  ```
  Error: Backup appears corrupted (missing required files)

  Available backups: ~/.maproom-mcp/backups/
  ```

**Hash Verification (Optional):**
If backup includes .maproom-version file, optionally verify restored files:
1. Read version file from backup
2. Compute hash of each restored file
3. Compare with backup version file
4. Log warning if mismatch (but don't fail - restoration is priority)

**Security Considerations:**
- Verify backup directory is within cache directory (prevent path traversal)
- Check file types before restoring (no symlinks)
- Set permissions immediately after restoring
- Reference: `security-review.md` lines 188-236 for rollback security

## Dependencies
- CFGVER-2001 (needs backup directory structure and file list)

## Risk Assessment
- **Risk**: Corrupted backup - backup exists but files are damaged
  - **Mitigation**: Provide manual recovery steps, list alternative backups
  - **Severity**: High (user may need to reinstall)

- **Risk**: Multiple failures - rollback itself fails after update failure
  - **Mitigation**: Provide detailed manual recovery commands, document escalation path
  - **Severity**: Critical (manual intervention required)

- **Risk**: Backup directory tampering - attacker modifies backup
  - **Mitigation**: Verify file types, check paths, optional hash verification
  - **Severity**: Medium (unlikely, requires local access)

- **Risk**: Permission errors - cannot write restored files
  - **Mitigation**: Provide chmod commands in error message
  - **Severity**: Medium (clear recovery path)

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add `rollbackConfigs()` function)
- **Read**: Backup files in `~/.maproom-mcp/backups/TIMESTAMP/`
- **Write**: Config files in `~/.maproom-mcp/` (restored from backup)
- **Log**: Restoration audit trail to console
