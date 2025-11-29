# Config Version Management - Problem Analysis

## The Problem

**Date:** October 30, 2024

**Incident:** Users running `npx -y @crewchief/maproom-mcp@latest` experienced MCP connection failures after package update.

**Root Cause:** The CLI caches configs at `~/.maproom-mcp/`. When the npm package updated, users got latest code but stale configs.

**Specific Change:** docker-compose.yml changed from local Docker builds to published images. Pattern-based detection (`includes('EMBEDDING_PROVIDER: ollama')`) missed this architectural change.

## Current Detection Logic (Brittle)

From `packages/maproom-mcp/bin/cli.cjs` lines 209-223:

```javascript
let needsUpdate = !fs.existsSync(COMPOSE_FILE);
if (!needsUpdate && fs.existsSync(COMPOSE_FILE)) {
  const existingContent = fs.readFileSync(COMPOSE_FILE, 'utf-8');
  const hasHardcodedProvider = existingContent.includes('EMBEDDING_PROVIDER: ollama');
  const hasEnvironmentVariable = existingContent.includes('${EMBEDDING_PROVIDER');
  if (hasHardcodedProvider && !hasEnvironmentVariable) {
    needsUpdate = true;
  }
}
```

**Why This Fails:**
- Only detects ONE specific pattern (hardcoded ollama)
- Missed architectural change (build → image)
- Requires adding new patterns for each breaking change
- Tight coupling between config structure and detection logic

## User Impact

**Without version tracking:**
- Can't detect updates automatically
- Can't show meaningful error messages
- Can't decide whether to update or not
- Must rely on brittle pattern matching
- Users manually delete cache to fix

**User frustration:**
- "Why isn't my MCP server connecting?"
- "I just ran the latest version..."
- "Do I need to delete cache again?"

## Solution Requirements

**Must have:**
- Detect ALL config changes automatically
- Work on first run (no cached configs)
- Work on version changes (1.1.12 → 1.2.0)
- Preserve user customizations (.env file)

**Nice to have (later):**
- Backup before update
- Rollback on failure
- File integrity verification
- Comprehensive testing

## Simplified Solution

**Instead of pattern matching:**
```typescript
// Store version explicitly
writeFile('~/.maproom-mcp/.version', '1.2.0');

// Compare on next run
if (cachedVersion !== packageVersion) {
  updateConfigs(); // Copy fresh, preserve .env
}
```

**Benefits:**
- Detects ALL changes (not just specific patterns)
- No maintenance burden (no patterns to add)
- Industry standard (npm, Docker, Kubernetes use this)
- Future-proof (works for any config change)

## Risk: Is Simple Too Risky?

**What if update fails?**
- User re-runs `npx` command
- Clear error message with recovery steps
- Worst case: Delete `~/.maproom-mcp/`, re-run

**Data loss risk:**
- User `.env` preserved in memory during update
- Only lost if write fails (rare)
- User can recreate (environment variables)

**Production safety:**
- This is a CLI tool (not production server)
- Users run it manually on their machines
- Failed updates are recoverable

## Decision: Ship Simple

**Bet:** The simple approach handles 95% of cases. The 5% edge cases can fail gracefully with clear recovery instructions.

**If we need more later:** Comprehensive plan archived at `.crewchief/archive/projects/CFGVER_config-version-management-comprehensive/`
