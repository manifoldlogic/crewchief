# Changelog

All notable changes to the Maproom MCP Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.9] - 2025-01-XX

### Fixed
- **CRITICAL**: Ollama no longer starts when using Google or OpenAI providers (#MCP-008, #MCP-011)
  - Fixed environment variable propagation from MCP client to Docker Compose
  - Added explicit env passing in all spawn() calls (MCPSTART-2001)
  - Implemented pre-flight container cleanup to remove stale containers (MCPSTART-3001)
  - Added explicit stop/remove for unnecessary services based on selected provider (MCPSTART-3002)
  - Added container state verification after startup to ensure only required services are running (MCPSTART-3003)

### Added
- Comprehensive diagnostic logging with MAPROOM_MCP_DEBUG=true (MCPSTART-1001, 1002, 1003)
  - Docker command execution logging
  - Container state verification logging
  - Credential redaction in logs (MCPSTART-1004)
  - docker-compose.yml config validation (MCPSTART-2002)
  - Provider env var validation with fail-fast behavior (MCPSTART-2003)
- Integration test suite for startup scenarios (MCPSTART-4001, 4002)
  - Test all three provider configurations (Ollama, Google, OpenAI)
  - Verify correct containers start/stop
  - Verify environment variable propagation

### Security
- Services now bound to localhost only (127.0.0.1) instead of 0.0.0.0 (MCPSTART-5001)
- Added npm audit check to prepublishOnly script (MCPSTART-5002)
- Credential redaction in diagnostic logs (MCPSTART-1004)
- Security documentation added to README (MCPSTART-5003)

### Changed
- Environment variables now explicitly passed to Docker Compose in all spawn() calls
- Config files at ~/.maproom-mcp/ auto-update if outdated (preserves user customizations)
- Improved error messages for missing provider credentials

### Upgrade Notes
- **Config files**: Files at ~/.maproom-mcp/ will auto-update on first run if the template has changed. Your customizations in docker-compose.env will be preserved.
- **No breaking changes**: Existing users can upgrade seamlessly.
- **Troubleshooting**: New MAPROOM_MCP_DEBUG=true mode available for diagnosing startup issues.
- **Security**: Services now bind to localhost only. If you were accessing from another machine, you'll need to adjust network configuration.

### Migration Guide
No migration steps required. Simply upgrade to the latest version:
```bash
npx @crewchief/maproom-mcp@latest
```

If you experience issues, see the Troubleshooting section in the README.

## [1.1.8] - 2024-XX-XX

Initial release of Maproom MCP Server.

### Added
- MCP server implementation for semantic code search
- Docker Compose based deployment with PostgreSQL and Ollama
- Support for multiple embedding providers (Ollama, Google Generative AI, OpenAI)
- Hybrid search combining FTS and vector similarity
- Tree-sitter based code parsing for TypeScript, Rust, and other languages
- MCP tools: status, search, open, context, explain, upsert
