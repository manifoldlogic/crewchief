# Ticket: CLIMAP-1001: Update CLI README with correct environment variables and setup documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- technical-writer
- verify-ticket
- commit-ticket

## Summary

Fix critical documentation errors in packages/cli/README.md by replacing deprecated PG_DATABASE_URL references with MAPROOM_DATABASE_URL, adding embedding provider setup instructions, and creating comprehensive troubleshooting sections.

## Background

The current CLI README contains outdated documentation that actively harms user experience:

- Uses deprecated `PG_DATABASE_URL` in 4 locations instead of the current `MAPROOM_DATABASE_URL`
- Users following CLI docs set wrong environment variables and experience connection failures
- Missing documentation for embedding providers (OpenAI, Google Vertex AI, Ollama)
- No troubleshooting guide for common setup errors
- Maproom now uses a 4-tier fallback system that is not documented

This is the first ticket in Phase 1 of CLIMAP (CLI-Maproom Alignment). It addresses the most user-facing issue: documentation that leads users to configure the tool incorrectly.

**References:**
- Analysis: `.agents/projects/CLIMAP_cli-maproom-alignment/planning/analysis.md`
- Architecture: `.agents/projects/CLIMAP_cli-maproom-alignment/planning/architecture.md`
- Security Review: `.agents/projects/CLIMAP_cli-maproom-alignment/planning/security-review.md`

## Acceptance Criteria

- [x] All 4 references to `PG_DATABASE_URL` replaced with `MAPROOM_DATABASE_URL`
- [x] Database Setup section added with connection string format and Docker setup instructions
- [x] Embedding Provider Setup section added documenting all three providers (OpenAI, Google Vertex AI, Ollama)
- [x] Troubleshooting section added covering database connection, embedding provider, and binary errors
- [x] All code examples use correct environment variable names
- [x] Documentation includes explanation of 4-tier fallback hierarchy
- [x] Formatting is consistent with existing README sections
- [x] Examples link to or reference maproom-mcp Docker setup for database configuration

## Technical Requirements

### File to Modify
- `/workspace/packages/cli/README.md`

### Environment Variables to Document

**Primary (Current Standard):**
- `MAPROOM_DATABASE_URL` - PostgreSQL connection string (replaces deprecated PG_DATABASE_URL)
- `MAPROOM_EMBEDDING_PROVIDER` - One of: `ollama`, `openai`, `google`
- `MAPROOM_EMBEDDING_MODEL` - Provider-specific model name

**Provider-Specific:**
- OpenAI: `OPENAI_API_KEY`
- Google Vertex AI: `GOOGLE_PROJECT_ID`, `GOOGLE_APPLICATION_CREDENTIALS`
- Ollama: No additional credentials needed (local service)

**Fallback Hierarchy (for documentation):**
1. `MAPROOM_DATABASE_URL` (highest priority, current standard)
2. Component-specific variables (e.g., `MAPROOM_MCP_DATABASE_URL`)
3. `PG_DATABASE_URL` (deprecated, legacy support)
4. `DATABASE_URL` (lowest priority, generic fallback)

### Content Sections to Add

1. **Database Setup** (after Quick Start)
   - Connection string format
   - Docker setup using maproom-mcp
   - Fallback hierarchy explanation
   - Example configuration

2. **Embedding Provider Setup** (after Database Setup)
   - OpenAI setup with API key
   - Google Vertex AI setup with service account
   - Ollama setup (local installation)
   - Provider-specific model examples

3. **Troubleshooting** (near end of README)
   - Database connection errors
   - Embedding provider errors
   - Binary compatibility issues
   - Environment variable debugging

### Formatting Requirements
- Use consistent markdown formatting with existing sections
- Include code blocks with bash syntax highlighting
- Use clear headings and subheadings
- Include practical, copy-paste-ready examples
- Add links to external resources where helpful (Docker docs, provider docs)

## Implementation Notes

### Search and Replace Strategy
1. Use grep to locate all instances of "PG_DATABASE_URL" in README
2. Replace each with MAPROOM_DATABASE_URL
3. Update surrounding context if it references "legacy" or "deprecated"
4. Ensure all code examples are updated

### New Section Placement
- Insert "Database Setup" after "Quick Start" section
- Insert "Embedding Provider Setup" after "Database Setup"
- Insert "Troubleshooting" before final sections (Contributing, License, etc.)

### Content Guidelines
- Be practical - users should be able to copy/paste examples
- Include error messages users might see
- Link to maproom-mcp Docker compose for database setup
- Explain "why" for key decisions (e.g., why the fallback hierarchy exists)
- Keep tone consistent with existing README (technical but approachable)

### Cross-Reference
- Refer to `packages/maproom-mcp/config/docker-compose.yml` for Docker setup
- Ensure examples match actual maproom-mcp configuration
- Verify embedding provider variables match what the Rust code expects

## Dependencies

**Prerequisites:** None (first ticket in CLIMAP project)

**Blocked by:** None

**External Dependencies:**
- Requires verification that Docker setup instructions match maproom-mcp reality
- Examples should be tested for accuracy

## Risk Assessment

**Risk**: Documentation examples may not match actual behavior
- **Mitigation**: Cross-reference with maproom-mcp Docker setup and existing code examples. Technical writer should verify examples against actual configuration files.

**Risk**: Embedding provider setup may be incomplete or incorrect
- **Mitigation**: Reference existing maproom documentation and ensure provider names match what Rust code expects (ollama, openai, google).

**Risk**: Troubleshooting section may not cover the most common user errors
- **Mitigation**: Base troubleshooting on known issues from analysis document and common setup failures.

**Risk**: Breaking existing user workflows who have bookmarked specific README sections
- **Mitigation**: Add new sections rather than removing content; use clear headers for easy navigation.

## Files/Packages Affected

- `/workspace/packages/cli/README.md` - Primary file to modify

**Reference Files (read-only):**
- `/workspace/packages/maproom-mcp/config/docker-compose.yml` - Docker setup examples
- `/workspace/packages/maproom-mcp/README.md` - MCP-specific documentation
- `/workspace/.agents/projects/CLIMAP_cli-maproom-alignment/planning/analysis.md` - Problem analysis
- `/workspace/.agents/projects/CLIMAP_cli-maproom-alignment/planning/architecture.md` - Architecture decisions
- `/workspace/.agents/projects/CLIMAP_cli-maproom-alignment/planning/security-review.md` - Credential handling guidance

## Estimated Effort

3-4 hours:
- 30 min: Search and replace PG_DATABASE_URL references
- 1 hour: Write Database Setup section
- 1 hour: Write Embedding Provider Setup section
- 1 hour: Write Troubleshooting section
- 30 min: Review and verify all examples against actual code
