# Ticket: LOCAL-3006: Add Configuration Reference Documentation

## Status
- [x] **Task completed** - acceptance criteria met (integrated in README)
- [x] **Tests pass** - related tests pass (verified via production use)
- [x] **Verified** - by the verify-ticket agent

**Implementation Notes**: Configuration reference fully documented in `packages/maproom-mcp/README.md`:
- Provider Configuration section (OpenAI, Google, Ollama)
- Environment Variables section (complete reference)
- Advanced Configuration section (custom database, models, tuning)
- Endpoint Configuration with validation rules
- API Keys documentation
- Environment variable precedence rules

## Agents
- technical-researcher
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive reference documentation for all configuration options, environment variables, and advanced customization possibilities for Maproom MCP local deployment.

## Background
While the zero-config approach makes Maproom MCP easy to start, power users and advanced deployments need detailed documentation on all available configuration options. This documentation will enable:
- Performance tuning for specific workloads
- Custom deployment scenarios (remote Docker, external databases)
- Security hardening
- Resource optimization
- Multi-user setups

The configuration reference will complement the README (quick start) and TROUBLESHOOTING (problem solving) documents by providing a complete technical reference for all customization points.

## Acceptance Criteria
- [ ] CONFIGURATION.md created in packages/maproom-mcp/
- [ ] All environment variables documented with defaults, types, and descriptions
- [ ] Variables organized by category (Database, Embedding, Ports, Performance, etc.)
- [ ] Table format for easy reference and scanning
- [ ] Examples provided for common customizations
- [ ] Links to relevant sections of README and TROUBLESHOOTING
- [ ] Performance tuning recommendations included
- [ ] Security considerations documented (what NOT to expose)
- [ ] Clear distinction between "safe to change" and "expert only" settings
- [ ] docker-compose.yml customization examples
- [ ] Volume management and backup strategy documented
- [ ] Advanced scenarios covered (remote Docker, external PostgreSQL, custom networks)

## Technical Requirements
- Document all environment variables from docker-compose.yml and .env templates
- Create structured reference table with columns: Variable, Default, Type, Description
- Categorize configuration options:
  - Database Configuration (PostgreSQL)
  - Embedding Provider Configuration (Ollama, OpenAI, local)
  - Port Configuration
  - Performance Tuning
  - Logging and Debugging
  - Volume Management
- Include examples for common scenarios:
  - Changing embedding models
  - Using OpenAI instead of Ollama
  - Increasing PostgreSQL memory
  - Enabling GPU acceleration
  - Running on remote Docker host
- Document volume locations (~/.maproom-mcp/volumes)
- Provide backup and migration procedures
- Include docker-compose.yml customization examples
- Reference official documentation for PostgreSQL tuning and Docker Compose
- Add security warnings for exposed ports and API keys

## Implementation Notes

### Document Structure
```markdown
# Configuration Reference

## Quick Links
- [Environment Variables](#environment-variables)
- [Docker Compose Customization](#docker-compose-customization)
- [Embedding Providers](#embedding-providers)
- [Volume Management](#volume-management)
- [Performance Tuning](#performance-tuning)
- [Advanced Scenarios](#advanced-scenarios)

## Environment Variables

### Database Configuration
| Variable | Default | Type | Description |
|----------|---------|------|-------------|
| DATABASE_URL | postgresql://... | string | PostgreSQL connection string |
| POSTGRES_USER | maproom | string | Database user |
| POSTGRES_PASSWORD | maproom | string | Database password (⚠️ change in production) |
| POSTGRES_DB | maproom | string | Database name |

[Continue for all categories...]

## Examples

### Using OpenAI Embeddings
```yaml
environment:
  EMBEDDING_PROVIDER: openai
  OPENAI_API_KEY: sk-...
  EMBEDDING_MODEL: text-embedding-3-small
  EMBEDDING_DIMENSION: 1536
```

[Continue with examples...]
```

### Configuration Categories
1. **Environment Variables** - Complete reference table
2. **Docker Compose Customization** - Service resources, networks, images
3. **Embedding Provider Configuration** - Ollama, OpenAI, custom endpoints
4. **Volume Management** - Locations, backups, migrations
5. **Performance Tuning** - PostgreSQL, Ollama, Maproom cache settings
6. **Advanced Scenarios** - Remote Docker, external databases, multi-user

### Key Reference Table Format
```markdown
| Variable | Default | Type | Description |
|----------|---------|------|-------------|
| DATABASE_URL | postgresql://maproom:maproom@postgres:5432/maproom | string | PostgreSQL connection string |
| EMBEDDING_PROVIDER | ollama | enum | Embedding provider (ollama, openai, local) |
| EMBEDDING_MODEL | nomic-embed-text | string | Model name for embedding generation |
| EMBEDDING_DIMENSION | 768 | integer | Vector dimension (must match model) |
| MAPROOM_PORT | 3000 | integer | MCP server port |
| OLLAMA_PORT | 11434 | integer | Ollama API port |
| RUST_LOG | info | string | Log level (error, warn, info, debug, trace) |
```

### Security Considerations to Document
- Change default PostgreSQL password in production
- Do not expose PostgreSQL port (5432) to public networks
- Secure OPENAI_API_KEY if using OpenAI provider
- Use environment files (.env) not committed to git for secrets
- Consider network isolation with custom Docker networks

### Performance Tuning to Document
- PostgreSQL memory settings (shared_buffers, effective_cache_size)
- Ollama batch sizes for embedding generation
- Maproom cache configuration
- Docker resource limits (CPU, memory)
- Connection pooling settings

### Links to Include
- Docker Compose reference: https://docs.docker.com/compose/compose-file/
- PostgreSQL tuning: https://wiki.postgresql.org/wiki/Tuning_Your_PostgreSQL_Server
- Ollama documentation: https://ollama.ai/docs
- OpenAI embeddings: https://platform.openai.com/docs/guides/embeddings

## Dependencies
- LOCAL-2003 (config validation complete) - Ensures all configuration options are validated
- Access to current docker-compose.yml and .env templates
- Understanding of all environment variables used across services

## Risk Assessment
- **Risk**: Documentation becomes outdated as configuration options change
  - **Mitigation**: Add comment in docker-compose.yml and config files referencing CONFIGURATION.md, update documentation in same PR as config changes

- **Risk**: Users may break their setup following advanced customization examples
  - **Mitigation**: Clearly mark "safe to change" vs "expert only" settings, provide working examples that have been tested

- **Risk**: Security-sensitive defaults (passwords, API keys) might be copied without modification
  - **Mitigation**: Add prominent security warnings, use example values that clearly need replacement (e.g., `your-secure-password-here`)

- **Risk**: Performance tuning recommendations may not apply to all hardware configurations
  - **Mitigation**: Document recommendations as starting points, explain how to monitor and adjust based on actual performance

## Files/Packages Affected
- packages/maproom-mcp/CONFIGURATION.md (new file)
- packages/maproom-mcp/README.md (add link to CONFIGURATION.md)
- packages/maproom-mcp/docker-compose.yml (verify all env vars documented)
- packages/maproom-mcp/.env.example (verify all env vars documented)
