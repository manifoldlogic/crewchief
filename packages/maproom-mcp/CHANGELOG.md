# Changelog

All notable changes to the Maproom MCP Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.1.0] - 2025-11-18

### Added

#### Worktree-Scoped Search with Auto-Detection (WTSRCH Project)

The `search` MCP tool now automatically scopes results to your current git branch, eliminating result duplication and making search results more relevant to your active work.

**Features:**
- **Auto-detection**: When `worktree` parameter is omitted, automatically detects current git branch and searches only that branch
- **Graceful fallback**: If current branch not indexed, falls back to `main` with helpful hint message
- **Explicit override**: Pass `worktree: "branch-name"` to search specific branch, or `worktree: null` to search all
- **Performance**: >99% cache hit rate for branch detection (60s TTL), <10ms search latency with warm cache
- **Result metadata**: Includes `auto_detected`, `mode`, and `hint` fields to inform users about resolution behavior

**Example:**
```typescript
// In feature-auth branch, automatically searches only feature-auth worktree
const results = await mcp__maproom__search({
  repo: "my-repo",
  query: "authentication flow"
})
// Returns: { hits: [...], worktree: "feature-auth", auto_detected: true, mode: "auto" }
```

**Why this matters:** Before v2.1.0, searches returned duplicated results across all branches, requiring manual worktree filtering. Now search "just works" for your current context.

**Technical implementation:**
- Phase 1 (WTSRCH-1001): Git branch detection with LRU caching
- Phase 2 (WTSRCH-2001): Four-tier worktree resolution (explicit > auto > fallback > all)
- Phase 3 (WTSRCH-3001): Search tool integration with metadata
- Phase 4 (WTSRCH-4001): Comprehensive testing (38 tests, all passing)

### Fixed
- **Critical**: Fixed provider endpoint resolution bug where cloud providers (OpenAI, Cohere) would inherit Ollama's default endpoint (PROVFIX-1001)
  - Added provider-aware endpoint validation that validates endpoint domains match configured provider
  - OpenAI now ignores `EMBEDDING_API_ENDPOINT=http://localhost:11434` from Docker Compose defaults
  - Prevents "Connection refused" errors when using cloud providers
  - Fixed cross-provider endpoint pollution from environment variables
- **Database schema**: Added missing `updated_at` column to `chunks` table (PROVFIX-2001)
  - Prevents "column updated_at does not exist" errors during embedding updates
  - Added auto-update trigger for timestamp tracking
  - Migration applies automatically on container startup
- **CLI cleanup**: Removed workaround code that explicitly set endpoints for cloud providers (PROVFIX-3001)
  - Simplified codebase by removing 3 instances of endpoint-setting workarounds
  - Rust now handles all endpoint resolution (single source of truth)
- **Docker defaults**: Removed default `EMBEDDING_API_ENDPOINT` from docker-compose.yml (PROVFIX-4001)
  - Prevents Docker Compose defaults from polluting environment for all providers
  - Provider-specific defaults now handled cleanly by Rust code
- **Open tool path resolution**: Fixed database pollution bug where multiple worktrees with same name caused incorrect path selection (OPNFIX-1001, OPNFIX-1002, OPNFIX-1003)
  - Added multi-candidate fallback mechanism that tries all matching worktrees in order (most recent first)
  - Open tool now validates file existence for each candidate path and returns first valid result
  - Enhanced error messages provide actionable guidance including suggestion to run `maproom db cleanup-stale`
  - Fixed issue where stale database entries from deleted worktrees caused "file not found" errors

### Added

#### Real-time Progress Indicators for Scan Command

The `scan` command now provides real-time feedback during indexing operations, making it easier to track progress on large repositories without wondering if the process has stalled.

**Features:**
- **File progress tracking**: See processed file count and percentage in real-time (e.g., "Processing: 450/1200 files (37%)")
- **Completion timing**: Prominently displays total scan duration
- **Smart output modes**: TTY terminals show in-place updates; non-TTY (CI/logs) shows periodic progress lines
- **Throttled updates**: Progress updates every 200-500ms to avoid console flooding

**Example output:**
```text
🔍 Scanning worktree: main @ abc12345
   Repository: my-repo
   Path: /path/to/repo

Processing: 45/100 files (45%)
✅ Completed in 8.3s
```

**Why this matters:** No more wondering if a long-running scan is stuck or making progress. You get immediate visual feedback and accurate completion times, improving the developer experience for daily use.

**Command improvements:**
- Scans current directory by default - no need to type `scan .`
- New `--verbose` flag available for future detailed diagnostics
- Minimal performance impact: <5% overhead through atomic counters and smart throttling

- Comprehensive unit tests for endpoint resolution covering all providers (PROVFIX-1002)
  - 8 tests including critical regression test `test_openai_ignores_ollama_endpoint`
  - Tests prove OpenAI/Cohere ignore wrong endpoints, Ollama accepts custom endpoints
  - All tests pass with `cargo test --lib config_endpoint_tests -- --test-threads=1`
- Integration test suite validating complete fix across all scenarios (PROVFIX-5001)
  - Database schema verification
  - Environment precedence validation
  - Provider-specific endpoint resolution
- Enhanced documentation for environment variables and troubleshooting (PROVFIX-6001)
  - Clear precedence rules for endpoint configuration
  - Provider-specific endpoint behavior documented
  - Troubleshooting section covers common misconfigurations
- **Open tool enhancements** (OPNFIX-2001, OPNFIX-2002):
  - Symlink validation ensures symlink targets remain within repository boundaries
  - Debug logging throughout path resolution process aids troubleshooting
  - Comprehensive test suite covering E2E workflows, security scenarios, and edge cases (OPNFIX-3001, 3002, 3003, 3004)
  - User documentation explaining multi-candidate fallback behavior and error messages (OPNFIX-4001)

### Security
- **Open tool symlink protection** (OPNFIX-2001):
  - Symlinks are now validated to prevent path traversal attacks
  - Symlink targets outside repository boundaries are rejected with clear error messages
  - Protects against malicious symlinks pointing to sensitive system files (e.g., `/etc/passwd`)

### Improved
- Clear environment variable precedence rules for all providers
- Provider-specific endpoint validation prevents configuration errors
- Code comments in config.rs explain validation logic for future maintainers
- README troubleshooting section addresses the exact bugs that were fixed

### Technical Details
Environment variable precedence (new behavior):
1. Explicit configuration in code (if applicable)
2. `EMBEDDING_API_ENDPOINT` environment variable (validated by provider)
3. Provider-specific default endpoint

Provider-specific validation rules:
- **OpenAI**: Only accepts endpoints containing "openai.com"
- **Cohere**: Only accepts endpoints containing "cohere"
- **Ollama/Local**: Accepts any endpoint (flexible for self-hosting)
- **Google**: Ignores `EMBEDDING_API_ENDPOINT` (uses region-based construction)

### Migration Notes
No breaking changes. Existing configurations will continue to work.

Benefits of upgrading:
- Cloud providers (OpenAI, Cohere) work reliably without workarounds
- Database updates persist correctly with new schema
- Cleaner environment variable handling
- Better error messages for misconfigurations

To upgrade:
```bash
npx @crewchief/maproom-mcp@latest setup --provider=<your-provider>
```

## [1.1.10] - 2025-10-29

### Fixed
- **Critical**: Fixed Docker Hub image distribution (v1.1.9 deployment failure)
  - docker-compose.yml now pulls pre-built images instead of building from source
  - Resolves "lstat /packages: no such file or directory" error
  - npm package now works correctly when installed globally or locally
- Build context error preventing users from starting services after npm install

### Added
- Automated Docker image publishing via GitHub Actions workflow
- Multi-platform support: AMD64 (x86_64) and ARM64 (Apple Silicon)
- Version pinning support via MAPROOM_VERSION environment variable
- docker-compose.override.yml for local development builds
- OCI-compliant image metadata labels (version, revision, created, etc.)
- Trivy security scanning in CI/CD pipeline

### Changed
- docker-compose.yml uses `image:` directive instead of `build:` for production
- Images now available at https://hub.docker.com/r/crewchief/maproom-mcp
- Faster startup: ~30 seconds (no build time)
- Development workflow: use docker-compose.override.yml for local builds

### Migration Notes
Upgrade from v1.1.9:
```bash
# Stop existing services
npx @crewchief/maproom-mcp stop

# Update package
npm install -g @crewchief/maproom-mcp@latest

# Start services (now pulls from Docker Hub)
npx @crewchief/maproom-mcp start
```

v1.1.9 is deprecated due to deployment failure. Skip directly to v1.1.10.

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
