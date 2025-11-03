# Ticket: PROVFIX-6001: Update Documentation After Provider Fixes

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose (technical writing)
- verify-ticket
- commit-ticket

## Summary
Update README and code comments to document the provider configuration fixes, remove mentions of workarounds, document environment variable precedence rules, and add troubleshooting guidance. This ensures users understand how provider configuration works correctly after all fixes are applied.

## Background
After all fixes are complete and integration tested, documentation needs to reflect:
1. How environment variables actually work (no workarounds)
2. Clear precedence rules for endpoint configuration
3. Provider-specific behavior and defaults
4. Troubleshooting common misconfigurations

From `.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md` Phase 6:

Documentation should explain the clean environment variable contract:
- For cloud providers (OpenAI, Cohere): Rust handles endpoint, rarely need override
- For local providers (Ollama, Local): Rust provides defaults or accepts custom
- Clear precedence: explicit override > env var (if valid) > provider default

This is Phase 6, Ticket 1 of the PROVFIX implementation plan - the final documentation phase.

## Acceptance Criteria
- [x] README.md updated to remove workaround mentions (no workaround mentions found or needed)
- [x] README.md documents environment variable precedence clearly (added Environment Variables section)
- [x] README.md includes troubleshooting section for common issues (enhanced with 3 new troubleshooting entries)
- [x] Code comments added to config.rs explaining validation logic (detailed comments added at lines 166-185)
- [x] Code comments reference this project for future maintainers (PROVFIX referenced in comments)
- [x] CHANGELOG.md updated with bug fix details (comprehensive [Unreleased] section added)
- [x] Documentation is accurate and helpful for users (all docs reflect actual tested behavior)

## Technical Requirements

### File 1: `/workspace/packages/maproom-mcp/README.md`

**1. Remove workaround mentions** (if any exist):
   - Search for references to endpoint workarounds
   - Remove or update outdated setup instructions

**2. Add Environment Variables section:**
```markdown
## Environment Variables

### Provider Configuration

- `EMBEDDING_PROVIDER`: (Required) One of: `openai`, `cohere`, `google`, `ollama`, `local`
- `EMBEDDING_MODEL`: (Required) Model name for the provider
- `EMBEDDING_DIMENSION`: (Required) Vector dimension for embeddings
- `EMBEDDING_API_ENDPOINT`: (Optional) Custom endpoint override

### Endpoint Configuration

**Cloud Providers (OpenAI, Cohere):**
- Use official endpoints by default (https://api.openai.com/v1/embeddings, etc.)
- `EMBEDDING_API_ENDPOINT` only used if domain matches provider
- Example: Setting `EMBEDDING_API_ENDPOINT=http://localhost:11434` for OpenAI is ignored

**Ollama:**
- Defaults to `http://localhost:11434/api/embed`
- Set `EMBEDDING_API_ENDPOINT` for custom Ollama server location

**Google Vertex AI:**
- Endpoint constructed from `GOOGLE_VERTEX_REGION` (e.g., `us-west1`)
- `EMBEDDING_API_ENDPOINT` is ignored

**Local Provider:**
- Requires `EMBEDDING_API_ENDPOINT` to be set explicitly

### Environment Variable Precedence

1. Explicit configuration in code (if applicable)
2. `EMBEDDING_API_ENDPOINT` environment variable (validated by provider)
3. Provider-specific default endpoint

### API Keys

- `OPENAI_API_KEY`: For OpenAI provider
- `COHERE_API_KEY`: For Cohere provider
- `GOOGLE_APPLICATION_CREDENTIALS`: For Google Vertex AI
```

**3. Add Troubleshooting section:**
```markdown
## Troubleshooting

### "Connection refused" errors to localhost:11434

**Problem:** OpenAI or Cohere provider attempting to connect to local Ollama endpoint.

**Solution:** This was a bug in earlier versions. Update to latest version where provider-aware endpoint validation prevents this issue.

### Custom endpoint not used

**Problem:** Set `EMBEDDING_API_ENDPOINT` but provider uses default.

**Solution:** Ensure the endpoint domain matches your provider:
- OpenAI: Must contain "openai.com"
- Cohere: Must contain "cohere"
- Ollama/Local: Any endpoint accepted
- Google: Ignores `EMBEDDING_API_ENDPOINT`

### Database "column updated_at does not exist" errors

**Problem:** Missing column in database schema.

**Solution:** Run database migrations. The maproom binary automatically applies migrations on startup.
```

### File 2: `/workspace/crates/maproom/src/embedding/config.rs`

Add comments explaining the validation logic:

```rust
// Provider-aware endpoint loading
// This validation prevents cross-provider endpoint pollution (e.g., OpenAI using Ollama endpoint)
// See: PROVFIX project for context on why this validation is critical
match config.provider {
    Provider::OpenAI | Provider::Cohere => {
        // Cloud providers only accept endpoints matching their domain
        // This prevents Docker Compose defaults from leaking across providers
        if let Ok(endpoint) = env::var("EMBEDDING_API_ENDPOINT") {
            // Validate domain matches provider
            ...
        }
    }
    ...
}
```

### File 3: CHANGELOG.md (if exists)

Add entry documenting the bug fix:

```markdown
## [Version] - [Date]

### Fixed
- **Provider endpoint resolution bug**: Fixed critical bug where `EMBEDDING_API_ENDPOINT` environment variable was used for all providers, causing cloud providers (OpenAI, Cohere) to inherit Ollama's default endpoint. Now validates endpoint domain matches configured provider.
- **Database schema**: Added missing `updated_at` column to `chunks` table that prevented embedding updates from persisting.
- **CLI cleanup**: Removed workaround code that explicitly set endpoints for cloud providers.
- **Docker defaults**: Removed default `EMBEDDING_API_ENDPOINT` from Docker Compose that caused endpoint pollution.

### Improved
- Clear environment variable precedence rules for all providers
- Provider-specific endpoint validation
- Comprehensive unit tests for endpoint resolution
```

## Implementation Notes

### Recommended Approach
See `.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md` Phase 6 for documentation scope.

Key principles:
- **Be clear and concise** - avoid jargon
- **Explain WHY, not just HOW** - prevents misconfiguration
- **Include troubleshooting** for the exact bugs that were fixed
- **Reference this project** for future maintainers

### Documentation Guidelines
1. **User-focused**: Write for developers using maproom-mcp, not just maintainers
2. **Example-driven**: Show concrete examples of correct and incorrect usage
3. **Troubleshooting-first**: Lead with common problems users encountered
4. **Accurate**: Documentation must match actual behavior after all fixes

### What to Document
- **Environment variable behavior** (not workarounds)
- **Provider-specific rules** (domain validation)
- **Precedence order** (explicit > env var > default)
- **Common mistakes** (wrong endpoint for provider)

### What NOT to Document
- Internal implementation details (unless critical for understanding)
- Workarounds that are no longer needed
- Historical bugs (mention fixes, not how things used to fail)

## Dependencies
- **Requires:** PROVFIX-5001 (integration testing must pass before documenting)
- **Requires:** All implementation tickets complete (1001, 1002, 2001, 3001, 4001)

## Risk Assessment
- **Risk**: Documentation doesn't match actual behavior
  - **Mitigation**: Write documentation after integration tests prove behavior (PROVFIX-5001)

- **Risk**: Over-documentation confuses users
  - **Mitigation**: Focus on common use cases, provide examples, use clear headings

- **Risk**: Technical debt not documented for future maintainers
  - **Mitigation**: Add code comments referencing PROVFIX project and explaining validation logic

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/README.md` (primary updates - env vars, troubleshooting)
- `/workspace/crates/maproom/src/embedding/config.rs` (code comments explaining validation)
- `/workspace/CHANGELOG.md` (if exists - bug fix entry)

## Planning References
- Plan: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/plan.md`
  - Phase 6: Documentation
- Architecture: `/workspace/.agents/projects/PROVFIX_maproom-provider-fixes/planning/architecture.md`
  - Section: "Environment Variable Contract"

## Success Criteria
**Before:** Documentation outdated or mentions workarounds
**After:** Documentation accurate, clear, helpful, matches actual behavior

Users can configure providers correctly without confusion or bugs.
