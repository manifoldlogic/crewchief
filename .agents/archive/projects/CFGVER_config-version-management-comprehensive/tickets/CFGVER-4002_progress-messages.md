# Ticket: CFGVER-4002: Implement user-friendly progress messages during updates

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
Add clear, informative progress messages during config updates so users understand what's happening and can trust the automatic update process. Messages should use consistent formatting with emoji indicators and provide actionable next steps on failure.

## Background
Users need visibility into what the CLI is doing during config updates. Silent updates create uncertainty and distrust. Clear messages build confidence that the system is working correctly. When updates fail, actionable error messages with recovery commands reduce support burden and empower users to fix issues themselves.

This aligns with Phase 4's user experience goal: making automatic updates transparent and trustworthy.

Reference: `architecture.md` lines 240-272 for message examples and formatting.

## Acceptance Criteria
- [ ] First run shows: "⚡ Initializing Maproom configuration..." with version and location
- [ ] Update shows: "⚡ Updating Maproom configuration..." with old and new versions
- [ ] Each update step shows progress: "Backed up to: ...", "Stopped containers", "Copied new configuration files", etc.
- [ ] Success shows: "✅ Configuration updated successfully"
- [ ] Failure shows: "❌ Update failed: [reason]" with recovery steps and copy-pasteable commands
- [ ] All progress messages use stdout (console.log), errors use stderr (console.error)
- [ ] Consistent emoji usage: ⚡ (in progress), ✅ (success), ❌ (error), ⚠️ (warning)

## Technical Requirements
- Add console.log() calls to updateConfigs() function for progress tracking
- Add console.error() calls for error messages
- Include context in messages: old version, new version, backup path
- Recovery commands must be copy-pasteable (exact syntax)
- Use consistent formatting across all messages
- Keep messages concise but informative (one line per step)

## Implementation Notes

**Message Examples (from architecture.md):**

**First Run:**
```
⚡ Initializing Maproom configuration...
   Version: 1.2.3
   Location: ~/.maproom-mcp/
✅ Configuration initialized
```

**Version Update:**
```
⚡ Updating Maproom configuration...
   From: 1.2.2
   To: 1.2.3
   Backed up to: ~/.maproom-mcp/backups/2024-10-30T15-30-00Z/
   Stopped containers
   Copied new configuration files
   Updated version tracking
   Cleaned up old resources
✅ Configuration updated successfully
```

**Update Failure:**
```
❌ Update failed: Cannot stop running containers
   Please stop containers manually:
   $ docker compose -f ~/.maproom-mcp/docker-compose.yml down

   Then run again:
   $ npx -y @crewchief/maproom-mcp@latest
```

**Rollback Success:**
```
⚠️ Update failed, rolling back...
   Restored from: ~/.maproom-mcp/backups/2024-10-30T15-30-00Z/
✅ Rollback successful - reverted to version 1.2.2
```

**Implementation Strategy:**
1. Add logging to updateConfigs() at each major step
2. Pass version info and backup path to logging calls
3. Format error messages with recovery commands
4. Test messages with real users for clarity

**Message Placement in updateConfigs():**
- Start: "⚡ Updating Maproom configuration..."
- After backup: "Backed up to: [path]"
- After container stop: "Stopped containers"
- After file copy: "Copied new configuration files"
- After version update: "Updated version tracking"
- After cleanup: "Cleaned up old resources"
- Success: "✅ Configuration updated successfully"
- Error: "❌ Update failed: [reason]" + recovery steps

**Architecture Reference:**
- Message examples: `architecture.md` lines 240-272
- User communication strategy: `architecture.md` lines 214-238

## Dependencies
- **CFGVER-4001**: CLI integration must be complete (messages show during updates)
- **CFGVER-2002**: updateConfigs() function must exist (where messages are added)

## Risk Assessment
- **Risk**: Verbose output overwhelming users
  - **Mitigation**: Keep messages concise (one line per step), use indentation for grouping
  - **Impact**: User feedback will guide verbosity adjustments

- **Risk**: Confusing error messages
  - **Mitigation**: Test with real users, include copy-pasteable recovery commands
  - **Impact**: Iterate based on support questions

- **Risk**: Inconsistent formatting across messages
  - **Mitigation**: Define message format constants, use templates
  - **Impact**: Code review will catch inconsistencies

## Files/Packages Affected
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (add console.log/error calls)
- **Modify**: `packages/maproom-mcp/src/config-manager.ts` (updateConfigs function)
- **No changes to**: CLI entry point (already integrated in CFGVER-4001)
