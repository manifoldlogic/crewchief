# Ticket: MCPSTART-2003: Add validation for required env vars per provider

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

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
- [ ] Validate Google provider requirements: GOOGLE_PROJECT_ID must be set
- [ ] Validate OpenAI provider requirements: OPENAI_API_KEY must be set
- [ ] Validation runs before starting Docker Compose
- [ ] Clear error messages explain what's missing
- [ ] Suggest how to fix (check .mcp.json or environment)
- [ ] Ollama provider has no required vars (zero-config)

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
