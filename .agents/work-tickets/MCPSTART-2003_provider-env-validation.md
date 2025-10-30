# Ticket: MCPSTART-2003: Add validation for required env vars per provider

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Validate that required environment variables are present for non-default providers (Google, OpenAI) before starting Docker Compose.

## Background
If user sets EMBEDDING_PROVIDER=google but doesn't provide GOOGLE_PROJECT_ID, startup will fail silently or with unclear errors. This ticket adds explicit validation with helpful error messages.

Completes Phase 2 - Environment Propagation Fix from MCPSTART_ARCHITECTURE.md.

## Acceptance Criteria
- [x] Validate Google provider requirements: GOOGLE_PROJECT_ID must be set
- [x] Validate OpenAI provider requirements: OPENAI_API_KEY must be set
- [x] Validation runs before starting Docker Compose
- [x] Clear error messages explain what's missing
- [x] Suggest how to fix (check .mcp.json or environment)
- [x] Ollama provider has no required vars (zero-config)

## Technical Requirements
- Function validateProviderConfig(provider)
- Check based on provider value:
  - 'google': require GOOGLE_PROJECT_ID (warn about GOOGLE_APPLICATION_CREDENTIALS)
  - 'openai': require OPENAI_API_KEY
  - 'ollama' or unset: no validation needed
- Exit with clear error if validation fails
- Use diagnosticLog to show what's being validated

## Implementation Notes
```javascript
function validateProviderConfig(provider) {
  diagnosticLog(`Validating provider configuration for: ${provider}`);

  if (provider === 'google') {
    if (!process.env.GOOGLE_PROJECT_ID) {
      console.error('❌ ERROR: EMBEDDING_PROVIDER=google requires GOOGLE_PROJECT_ID');
      console.error('   Check your .mcp.json configuration or set environment variable:');
      console.error('   export GOOGLE_PROJECT_ID=your-project-id');
      process.exit(1);
    }
    diagnosticLog('✓ GOOGLE_PROJECT_ID found');

    if (!process.env.GOOGLE_APPLICATION_CREDENTIALS) {
      console.error('⚠️  WARNING: GOOGLE_APPLICATION_CREDENTIALS not set');
      console.error('   Google Vertex AI may not work without credentials');
    } else {
      diagnosticLog('✓ GOOGLE_APPLICATION_CREDENTIALS found');
    }
  } else if (provider === 'openai') {
    if (!process.env.OPENAI_API_KEY) {
      console.error('❌ ERROR: EMBEDDING_PROVIDER=openai requires OPENAI_API_KEY');
      console.error('   Check your .mcp.json configuration or set environment variable:');
      console.error('   export OPENAI_API_KEY=your-api-key');
      process.exit(1);
    }
    diagnosticLog('✓ OPENAI_API_KEY found');
  } else if (provider === 'ollama' || !provider) {
    diagnosticLog('Using ollama provider (zero-config)');
  } else {
    console.error(`⚠️  WARNING: Unknown provider: ${provider}`);
    console.error('   Supported: ollama, google, openai');
  }
}
```

Call this function in cli.cjs after loading config and determining EMBEDDING_PROVIDER value:
```javascript
// After propagating environment variables
const embeddingProvider = process.env.EMBEDDING_PROVIDER || 'ollama';
validateProviderConfig(embeddingProvider);
```

## Dependencies
- MCPSTART-2001 (env propagation must exist first)
- MCPSTART-2002 (docker-compose verification should run before provider validation)

## Risk Assessment
- **Risk**: Low - fail-fast validation prevents cryptic runtime errors
  - **Mitigation**: Clear error messages with actionable suggestions
- **Risk**: May block legitimate use cases with alternative credential methods
  - **Mitigation**: Warnings instead of errors where possible (e.g., GOOGLE_APPLICATION_CREDENTIALS)

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs`

## Implementation Complete

### Changes Made

1. **Added `validateProviderConfig(provider)` function** (lines 832-870):
   - Validates Google provider: requires GOOGLE_PROJECT_ID (error if missing), warns about GOOGLE_APPLICATION_CREDENTIALS (warning if missing)
   - Validates OpenAI provider: requires OPENAI_API_KEY (error if missing)
   - Ollama provider: zero-config, no validation needed
   - Unknown providers: shows warning message with supported providers list
   - Uses `diagnosticLog()` to show validation steps
   - Error messages use ❌ prefix for required vars
   - Warning messages use ⚠️ prefix for optional vars
   - Calls `process.exit(1)` on validation failure

2. **Integrated validation into main() workflow** (lines 894-896):
   - Determines EMBEDDING_PROVIDER value (with 'ollama' default)
   - Calls `validateProviderConfig()` after `verifyDockerComposeConfig()`
   - Runs before `startDockerCompose()` to fail fast
   - Prevents silent failures with clear error messages

### Verification Steps

To test the implementation:

```bash
# Test 1: Ollama (zero-config) - should start without errors
npx @crewchief/maproom-mcp

# Test 2: Google without credentials - should fail with clear error
EMBEDDING_PROVIDER=google npx @crewchief/maproom-mcp
# Expected: "❌ ERROR: EMBEDDING_PROVIDER=google requires GOOGLE_PROJECT_ID"

# Test 3: Google with PROJECT_ID but no credentials - should show warning
EMBEDDING_PROVIDER=google GOOGLE_PROJECT_ID=test-project npx @crewchief/maproom-mcp
# Expected: "⚠️ WARNING: GOOGLE_APPLICATION_CREDENTIALS not set"

# Test 4: OpenAI without API key - should fail with clear error
EMBEDDING_PROVIDER=openai npx @crewchief/maproom-mcp
# Expected: "❌ ERROR: EMBEDDING_PROVIDER=openai requires OPENAI_API_KEY"

# Test 5: Unknown provider - should show warning
EMBEDDING_PROVIDER=unknown npx @crewchief/maproom-mcp
# Expected: "⚠️ WARNING: Unknown provider: unknown"
```

All acceptance criteria met:
- ✅ Validates Google provider requirements (GOOGLE_PROJECT_ID required)
- ✅ Validates OpenAI provider requirements (OPENAI_API_KEY required)
- ✅ Validation runs before Docker Compose starts
- ✅ Clear error messages explain what's missing
- ✅ Suggests how to fix (check .mcp.json or set environment variable)
- ✅ Ollama provider has zero-config (no required vars)
