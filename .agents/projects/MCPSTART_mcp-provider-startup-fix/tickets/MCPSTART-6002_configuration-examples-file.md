# Ticket: MCPSTART-6002: Create configuration examples file

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Create `docker-compose.env.example` with all provider configurations as reference, providing clear examples of how to configure Ollama, Google Vertex AI, and OpenAI embedding providers.

## Background
From MCPSTART_ARCHITECTURE.md lines 421-450 - users need clear examples of how to configure each embedding provider. A comprehensive `.env.example` file serves as both documentation and a quick-start template for users setting up maproom-mcp with different providers.

## Acceptance Criteria
- [x] Create `packages/maproom-mcp/config/docker-compose.env.example` file
- [x] Include Ollama configuration section (marked as default, zero-config)
- [x] Include Google Vertex AI configuration section with all required variables
- [x] Include OpenAI configuration section with all required variables
- [x] Include database configuration section
- [x] Include logging configuration section
- [x] Add clear comments explaining each variable and when to use it
- [x] Document in README how to use .env files with instructions

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

## Implementation Notes

**Files Created:**
1. `/workspace/packages/maproom-mcp/config/docker-compose.env.example`
   - Complete template with all provider configurations
   - Well-commented sections for Ollama (default), Google Vertex AI, OpenAI
   - Database configuration section with default connection string
   - Logging configuration with MAPROOM_MCP_DEBUG and RUST_LOG variables
   - Clear section headers using comment dividers
   - Emphasis on Ollama being zero-config default

**Documentation Added:**
2. `/workspace/packages/maproom-mcp/README.md`
   - Added "Using Environment Files" section after "Environment Variables (Optional)" section (line 115)
   - Three-step instructions: copy example file, edit for provider, automatic loading
   - Example showing Google Vertex AI configuration
   - Note about alternative direct environment variable usage

**Formatting:**
- Used comment dividers (=== and ---) for visual separation of sections
- All configuration lines are commented out (using #) as they are examples
- Clear indication that Ollama is default with "zero-config" emphasis
- Placeholder values for sensitive data (your-gcp-project-id, sk-..., etc.)
- Grouped related variables logically (provider selection, provider configs, database, logging)

**Verification Steps:**
1. Check that `/workspace/packages/maproom-mcp/config/docker-compose.env.example` exists
2. Verify all required sections are present (Ollama, Google, OpenAI, Database, Logging)
3. Confirm README includes "Using Environment Files" section with cp command showing ~/.maproom-mcp path
4. Verify Ollama is marked as default with "zero-config" language
5. Confirm all variables match the template from MCPSTART_ARCHITECTURE.md lines 425-450
