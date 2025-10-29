# Ticket: MCPSTART-6002: Create configuration examples file

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create `docker-compose.env.example` with all provider configurations as reference, providing clear examples of how to configure Ollama, Google Vertex AI, and OpenAI embedding providers.

## Background
From MCPSTART_ARCHITECTURE.md lines 421-450 - users need clear examples of how to configure each embedding provider. A comprehensive `.env.example` file serves as both documentation and a quick-start template for users setting up maproom-mcp with different providers.

## Acceptance Criteria
- [ ] Create `packages/maproom-mcp/config/docker-compose.env.example` file
- [ ] Include Ollama configuration section (marked as default, zero-config)
- [ ] Include Google Vertex AI configuration section with all required variables
- [ ] Include OpenAI configuration section with all required variables
- [ ] Include database configuration section
- [ ] Include logging configuration section
- [ ] Add clear comments explaining each variable and when to use it
- [ ] Document in README how to use .env files with instructions

## Technical Requirements

Create the file with the complete template from MCPSTART_ARCHITECTURE.md lines 425-450:

```bash
# packages/maproom-mcp/config/docker-compose.env.example

# =============================================================================
# Maproom MCP Configuration Examples
# =============================================================================
# Copy this file to docker-compose.env and uncomment/edit as needed
# Default: Ollama (zero-config, no environment variables required)

# -----------------------------------------------------------------------------
# Embedding Provider Selection
# -----------------------------------------------------------------------------
# Options: ollama (default), google, openai
# EMBEDDING_PROVIDER=ollama

# -----------------------------------------------------------------------------
# Ollama Configuration (DEFAULT)
# -----------------------------------------------------------------------------
# No configuration required! Ollama runs in a container automatically.
# If you want to use a custom Ollama model:
# OLLAMA_MODEL=llama2

# -----------------------------------------------------------------------------
# Google Vertex AI Configuration
# -----------------------------------------------------------------------------
# Required for EMBEDDING_PROVIDER=google
# EMBEDDING_PROVIDER=google
# GOOGLE_CLOUD_PROJECT=your-gcp-project-id
# GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account-key.json

# -----------------------------------------------------------------------------
# OpenAI Configuration
# -----------------------------------------------------------------------------
# Required for EMBEDDING_PROVIDER=openai
# EMBEDDING_PROVIDER=openai
# OPENAI_API_KEY=sk-...

# -----------------------------------------------------------------------------
# Database Configuration
# -----------------------------------------------------------------------------
# These defaults work for the Docker Compose setup
# DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom

# -----------------------------------------------------------------------------
# Logging & Debugging
# -----------------------------------------------------------------------------
# Enable diagnostic logging (shows docker commands, container states, etc.)
# MAPROOM_MCP_DEBUG=true

# Rust logging level for maproom binary
# RUST_LOG=info
```

Also add to README.md a section explaining how to use env files:

```markdown
### Using Environment Files

You can configure maproom-mcp using an environment file:

1. Copy the example file:
   ```bash
   cp ~/.maproom-mcp/docker-compose.env.example ~/.maproom-mcp/docker-compose.env
   ```

2. Edit the file with your provider configuration:
   ```bash
   # For Google Vertex AI
   EMBEDDING_PROVIDER=google
   GOOGLE_CLOUD_PROJECT=my-project
   GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
   ```

3. The MCP server will automatically load this file on startup.

Alternatively, you can set environment variables directly when running npx commands.
```

## Implementation Notes

- The example file should be well-commented and self-documenting
- Group related variables together with clear section headers
- Use comments to indicate which variables are required vs. optional
- Show example values where helpful (but use placeholders like `your-project-id`)
- Mark the Ollama configuration as the default with "zero-config" emphasis
- Include the diagnostic logging variable since it's part of the troubleshooting workflow
- Position the README documentation in a logical place (after basic usage, in a "Configuration" section)

## Dependencies
- None - this is pure documentation

## Risk Assessment
- **Risk**: None - example file only, not loaded by default
  - **Mitigation**: N/A

## Files/Packages Affected
- `packages/maproom-mcp/config/docker-compose.env.example` (new file)
- `packages/maproom-mcp/README.md` (add documentation reference)
