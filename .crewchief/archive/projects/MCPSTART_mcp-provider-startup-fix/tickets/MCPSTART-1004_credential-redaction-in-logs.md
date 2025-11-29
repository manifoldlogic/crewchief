# Ticket: MCPSTART-1004: Implement credential redaction in diagnostic logs

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- manual-verification
- verify-ticket
- commit-ticket

## Summary
Add redaction of sensitive values in diagnostic logs to prevent credential exposure while maintaining debugging utility.

## Background
Diagnostic logs from MCPSTART-1001 may contain sensitive information like API keys, database passwords, and credential file paths. This ticket implements redaction so that sensitive env vars show as "(redacted)" instead of their actual values, maintaining security while preserving diagnostic value.

This addresses security concerns from **MCPSTART_SECURITY_REVIEW.md Section 1** and completes Phase 1 diagnostic infrastructure.

This is part of the MCPSTART project (MCP Provider Startup Fix), Phase 1 - Diagnostic Infrastructure (v1.1.8).

## Acceptance Criteria
- [ ] Sensitive environment variables are redacted in all diagnostic output
- [ ] Redacted vars include: GOOGLE_APPLICATION_CREDENTIALS, OPENAI_API_KEY, DATABASE_URL, POSTGRES_PASSWORD
- [ ] Presence is still indicated (shows "(redacted)" not "(not set)")
- [ ] Redaction function is centralized and reusable
- [ ] Non-sensitive vars like EMBEDDING_PROVIDER still show full values
- [ ] Redaction applies to any key containing "KEY", "SECRET", "PASSWORD", "TOKEN"

## Technical Requirements
- Create `redactSensitive(data)` function
- List of sensitive keys to redact:
  - GOOGLE_APPLICATION_CREDENTIALS
  - OPENAI_API_KEY
  - DATABASE_URL
  - POSTGRES_PASSWORD
  - Any key containing "KEY", "SECRET", "PASSWORD", "TOKEN" (case-insensitive)
- Apply redaction in diagnosticLog() before JSON.stringify()
- Replace sensitive values with "(redacted)" string
- Preserve object structure for debugging
- Function must handle nested objects (recursive redaction)

## Implementation Notes
From MCPSTART_SECURITY_REVIEW.md lines 42-68, implement the following pattern:

```javascript
const SENSITIVE_ENV_VARS = [
  'GOOGLE_APPLICATION_CREDENTIALS',
  'OPENAI_API_KEY',
  'DATABASE_URL',
  'POSTGRES_PASSWORD'
];

const SENSITIVE_PATTERNS = ['KEY', 'SECRET', 'PASSWORD', 'TOKEN'];

function redactSensitive(data) {
  if (!data || typeof data !== 'object') return data;

  const redacted = { ...data };

  Object.keys(redacted).forEach(key => {
    const upperKey = key.toUpperCase();

    // Check explicit list
    const isExplicitlySensitive = SENSITIVE_ENV_VARS.some(
      sensitive => upperKey.includes(sensitive)
    );

    // Check patterns
    const matchesPattern = SENSITIVE_PATTERNS.some(
      pattern => upperKey.includes(pattern)
    );

    if (isExplicitlySensitive || matchesPattern) {
      redacted[key] = '(redacted)';
    } else if (typeof redacted[key] === 'object') {
      // Recursively redact nested objects
      redacted[key] = redactSensitive(redacted[key]);
    }
  });

  return redacted;
}
```

**Update locations:**
- Add `redactSensitive()` function to `packages/maproom-mcp/bin/cli.cjs`
- Update `diagnosticLog()` function to call `redactSensitive(data)` before `JSON.stringify()`
- Apply to all diagnostic output from MCPSTART-1001, 1002, and 1003

**Testing approach:**
- Manual verification: Run startup with sensitive env vars set
- Verify logs show "(redacted)" instead of actual values
- Verify non-sensitive vars still show full values
- Check that debugging utility is preserved (can still see structure)

## Dependencies
- **Prerequisite**: MCPSTART-1001 (environment diagnostic logging)
- **Prerequisite**: MCPSTART-1002 (docker command logging)
- **Prerequisite**: MCPSTART-1003 (container state logging)
- **Updates**: All three tickets' logging output to use redaction

## Risk Assessment
- **Risk**: Low - only changes log output, no functional changes
  - **Mitigation**: Test that debugging is still effective with redacted values
- **Risk**: Over-redaction could hide useful debugging info
  - **Mitigation**: Use pattern matching carefully, preserve object structure
- **Risk**: Under-redaction could still expose credentials
  - **Mitigation**: Include broad pattern matching (KEY, SECRET, PASSWORD, TOKEN)

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add redaction function, update diagnosticLog()

## Implementation Notes

**Completed Changes:**

1. **Added `redactSensitive()` function** (lines 20-63):
   - Takes an object and returns a redacted copy
   - Redacts specific sensitive env vars: GOOGLE_APPLICATION_CREDENTIALS, OPENAI_API_KEY, DATABASE_URL, POSTGRES_PASSWORD
   - Redacts any key containing: "KEY", "SECRET", "PASSWORD", "TOKEN" (case-insensitive)
   - Replaces sensitive values with "(redacted)" string
   - Handles nested objects recursively
   - Preserves object structure for debugging

2. **Updated `diagnosticLog()` function** (lines 69-77):
   - Calls `redactSensitive(data)` before `JSON.stringify()`
   - Applies redaction to all diagnostic output automatically

3. **Updated startup diagnostic log** (lines 80-89):
   - Changed from showing "(set)" indicators to actual values
   - This allows redaction function to properly redact sensitive values
   - Non-sensitive vars like EMBEDDING_PROVIDER, GOOGLE_PROJECT_ID, OLLAMA_HOST still show full values

**Testing Results:**
- Verified redaction with test script showing:
  - Sensitive values: "(redacted)"
  - Non-sensitive values: full value displayed
  - Nested objects: properly handled
  - Object structure: preserved

**All Acceptance Criteria Met:**
- ✅ Sensitive environment variables are redacted in all diagnostic output
- ✅ Redacted vars include: GOOGLE_APPLICATION_CREDENTIALS, OPENAI_API_KEY, DATABASE_URL, POSTGRES_PASSWORD
- ✅ Presence is still indicated (shows "(redacted)" not "(not set)")
- ✅ Redaction function is centralized and reusable
- ✅ Non-sensitive vars like EMBEDDING_PROVIDER still show full values
- ✅ Redaction applies to any key containing "KEY", "SECRET", "PASSWORD", "TOKEN"

**Verification Instructions:**
1. Run the CLI with sensitive env vars set:
   ```bash
   OPENAI_API_KEY=sk-test123 DATABASE_URL=postgresql://user:pass@host:5432/db npx @crewchief/maproom-mcp
   ```
2. Check diagnostic logs show "(redacted)" instead of actual values
3. Verify non-sensitive vars like EMBEDDING_PROVIDER still show full values
4. Confirm debugging utility is preserved (can still see object structure)
