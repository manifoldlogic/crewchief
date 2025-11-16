# Ticket: VSMAP-2002: Implement SecretStorage for API credentials

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Integrate VSCode SecretStorage API to securely store OpenAI/Google API keys. Pass credentials to Rust binary via environment variables.

## Background
This implements Phase 2 (Setup Wizard) of the VSMAP plan. After users select their embedding provider in VSMAP-2001, they need to provide API credentials if using OpenAI or Google. These credentials must be stored securely using VSCode's encrypted SecretStorage API and never logged or exposed. The credentials are passed to the Rust binary via environment variables when spawning processes.

Reference: VSMAP_PLAN.md Phase 2 "Setup Wizard - Credential Management"

## Acceptance Criteria
- [ ] Credentials stored in VSCode SecretStorage (encrypted at rest)
- [ ] API key input shown after provider selection for OpenAI/Google providers
- [ ] Credentials never logged to Output channel or debug console
- [ ] Environment variables set correctly when spawning Rust processes
- [ ] Credentials retrievable for binary spawn operations
- [ ] Input box masks password characters during entry

## Technical Requirements
- Use `context.secrets.store('maproom.api_key', key)` for storage
- Use `context.secrets.get('maproom.api_key')` for retrieval
- Set environment variables: `MAPROOM_OPENAI_API_KEY`, `MAPROOM_GOOGLE_APPLICATION_CREDENTIALS`
- Input box: `vscode.window.showInputBox({ password: true, prompt: "..." })`
- Provider-specific key names: `maproom.openai_key`, `maproom.google_key`
- No credential logging anywhere in codebase

## Implementation Notes
Create a secrets manager module that wraps VSCode SecretStorage API:

```typescript
class SecretsManager {
  constructor(private secrets: vscode.SecretStorage) {}

  async storeApiKey(provider: string, key: string): Promise<void>
  async getApiKey(provider: string): Promise<string | undefined>
  getEnvironmentVars(provider: string): Record<string, string>
}
```

The setup wizard (VSMAP-2001) should call this after provider selection:
1. If provider is "ollama", skip credential input (local model)
2. If provider is "openai" or "google", show password input box
3. Store credential in SecretStorage with provider-specific key

When spawning Rust processes (in BinarySpawner), retrieve credentials and set env vars:
```typescript
const env = {
  ...process.env,
  ...await secretsManager.getEnvironmentVars(provider)
};
```

Security considerations:
- Never log credentials (audit all console.log/output.appendLine calls)
- Use password-masked input boxes
- Clear credential variables after process spawn

## Dependencies
- VSMAP-2001 (provider selection UI) must be complete
- VSMAP-1003 (binary spawner) for integration

## Risk Assessment
- **Risk**: Credentials accidentally logged or exposed
  - **Mitigation**: Code review checklist item, search codebase for any credential logging
- **Risk**: Environment variables visible in process list
  - **Mitigation**: This is inherent to env var passing; document in security notes
- **Risk**: Users may not have API keys ready
  - **Mitigation**: Provide clear prompts with links to provider documentation

## Files/Packages Affected
- `src/config/secrets.ts` (new file, ~80 lines)
- `src/ui/setupWizard.ts` (integrate credential input)
- `src/process/spawner.ts` (integrate env vars in spawn)
- `src/test/secrets.test.ts` (new test file)
