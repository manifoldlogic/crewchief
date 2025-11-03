# Ticket: DBFALLBK-3001: Update Node.js CLI to Respect Explicit DATABASE_URL

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Modify the Node.js CLI scan and watch commands to respect existing DATABASE_URL environment variable instead of always overriding it, and add logging to indicate whether auto-detection or explicit config was used.

## Background
Currently, the Node.js CLI in packages/maproom-mcp/bin/cli.cjs always overrides the DATABASE_URL environment variable by calling getDatabaseConnectionString() unconditionally (line 1524 in scan command, similar in watch command). This means even when developers explicitly set DATABASE_URL (like in the devcontainer), the CLI ignores it and uses auto-detected values.

This creates problems:
- Devcontainer sets DATABASE_URL but CLI overrides it
- No way for users to explicitly specify database connection
- Inconsistent with standard practice of respecting explicit env vars

This ticket implements Phase 3 from planning/plan.md: making the CLI respect explicit DATABASE_URL and only use auto-detection as a fallback.

## Acceptance Criteria
- [ ] scan command respects existing DATABASE_URL when set
- [ ] watch command respects existing DATABASE_URL when set
- [ ] Logging clearly shows "Using explicit DATABASE_URL from environment" vs "Auto-detected database connection"
- [ ] When DATABASE_URL not set, auto-detection works as before (backward compatible)
- [ ] Debug output shows which connection method was used
- [ ] DATABASE_URL is sanitized in debug output (password replaced with ***)

## Technical Requirements
Update scan command (around line 1522-1526 in cli.cjs):

**Current code:**
```javascript
const env = {
  ...process.env,
  DATABASE_URL: getDatabaseConnectionString(),  // Always overrides!
  ...providerEnv
};
```

**New code:**
```javascript
const env = {
  ...process.env,
  ...providerEnv
};

// Only set DATABASE_URL if not already set
if (!env.DATABASE_URL) {
  env.DATABASE_URL = getDatabaseConnectionString();
  console.error('🔗 Auto-detected database connection');
} else {
  console.error('🔗 Using explicit DATABASE_URL from environment');
}
```

Apply the same logic to the watch command (around line 1672-1678).

Update scan command debug output (around line 1531-1536) to show the DATABASE_URL being used (sanitized):
```javascript
console.error('🔍 [DEBUG] Database connection:');
console.error(`   DATABASE_URL: ${sanitizeDatabaseUrl(env.DATABASE_URL)}`);
```

Add a sanitizeDatabaseUrl() helper function similar to the Rust sanitize_database_url() function that replaces password with ***:
```javascript
function sanitizeDatabaseUrl(url) {
  if (!url) return '(not set)';
  try {
    const parsed = new URL(url);
    if (parsed.password) {
      parsed.password = '***';
    }
    return parsed.toString();
  } catch (e) {
    return url.replace(/:([^:@]+)@/, ':***@'); // Fallback regex replacement
  }
}
```

## Implementation Notes
The key change is checking if env.DATABASE_URL already exists before calling getDatabaseConnectionString(). This makes auto-detection a true fallback instead of always overriding.

### Files to modify:

1. **Line ~1524: scan command env building**
   - Check if DATABASE_URL already exists in env
   - Only call getDatabaseConnectionString() if not set
   - Add logging to show which method was used

2. **Line ~1674: watch command env building**
   - Same logic as scan command
   - Ensure consistency between both commands

3. **Line ~1531-1536: scan command debug output**
   - Add DATABASE_URL to debug output
   - Use sanitizeDatabaseUrl() to hide password

4. **Add sanitizeDatabaseUrl() helper function**
   - Use URL API to parse and replace password
   - Fallback to regex if parsing fails
   - Return "(not set)" if URL is undefined/null

The getDatabaseConnectionString() function itself doesn't need changes - it already implements the fallback hierarchy. We're just changing when we call it.

### Behavior Matrix

| Scenario | DATABASE_URL set? | Result | Logging |
|----------|------------------|--------|---------|
| Devcontainer | Yes (from .env) | Uses explicit value | "Using explicit DATABASE_URL from environment" |
| Auto-detect | No | Calls getDatabaseConnectionString() | "Auto-detected database connection" |
| Override | Yes (but user wants auto) | Uses explicit value | "Using explicit DATABASE_URL from environment" |

## Dependencies
- DBFALLBK-2001 should be complete (Rust fallback) for consistency, but not strictly required

## Risk Assessment
- **Risk**: Breaking existing behavior for users who rely on auto-detection
  - **Mitigation**: Auto-detection still works when DATABASE_URL not set - this is backward compatible

- **Risk**: Users might not realize they can set DATABASE_URL explicitly
  - **Mitigation**: Clear logging shows which method was used, making the behavior transparent

- **Risk**: Edge cases where env.DATABASE_URL is empty string vs undefined
  - **Mitigation**: The check `if (!env.DATABASE_URL)` handles both undefined and empty string cases

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/bin/cli.cjs`
  - Update scan command env building (~line 1524)
  - Update watch command env building (~line 1674)
  - Update scan command debug output (~line 1531)
  - Add sanitizeDatabaseUrl() helper function (top of file or near getDatabaseConnectionString())
