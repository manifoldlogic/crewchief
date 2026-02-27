# Changelog

All notable changes to the Maproom Semantic Search extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2025-01-25

### Added
- **Automatic Docker container startup** - Extension now starts PostgreSQL and MCP server automatically
- No manual `npx @crewchief/maproom-mcp setup` command needed
- Clear error messages when Docker not running with recovery instructions
- Progress notifications during Docker startup
- Troubleshooting documentation for Docker-related issues

### Changed
- Extension activation flow now includes Docker startup step
- Setup wizard automatically starts containers after provider selection
- Docker services start before PostgreSQL health checks

### Fixed
- "DATABASE_URL env var is required" error on fresh installations
- Extension failing when Docker containers not manually started
- Improved error handling with actionable user guidance

## [0.1.0] - 2025-11-16

### Added - Phase 1: Docker Service Management

- **Automated Docker service orchestration** via Docker Compose
  - PostgreSQL 16 with pgvector extension for vector storage
  - Ollama service for local embedding generation
  - Maproom MCP server for indexing and search
- **Health checking** with exponential backoff (1s → 16s delays)
  - 30-second timeout prevents indefinite hangs
  - PostgreSQL readiness verification via `pg_isready`
  - Detailed logging of health check attempts
- **Robust error handling** with specific error codes
  - Docker not found detection
  - Docker daemon not running detection
  - Health check timeout handling
  - User-friendly error messages with actionable guidance
- **Graceful shutdown** with SIGTERM → SIGKILL cascade
  - 5-second grace period for clean shutdown
  - Automatic resource cleanup
  - No process leaks

### Added - Phase 2: Setup Wizard & Configuration

- **Interactive setup wizard** for first-run configuration
  - QuickPick UI for embedding provider selection
  - Auto-detection of Ollama service (localhost:11434)
  - Recommends Ollama when detected locally
  - Re-runnable via Command Palette (`Maproom: Setup`)
- **Multi-provider support** for embeddings
  - Ollama (local, free, private)
  - OpenAI (requires API key)
  - Google Gemini (requires API key)
- **Secure credential storage** using VSCode SecretStorage API
  - Password-masked input for API keys
  - Encrypted storage via VSCode's built-in secrets manager
  - Platform-specific secure storage (Keychain, Secret Service, Credential Manager)
  - Delete credentials when changing providers
- **Workspace state persistence** for provider selection
  - Remembers chosen provider across sessions
  - Separate state per workspace

### Added - Phase 3: File Scanning & Watching

- **Initial workspace scanning** with progress tracking
  - Scans all files in workspace on activation
  - Progress notifications with file count updates
  - Cancellable long-running operations
  - Binary file detection and filtering (images, PDFs, compiled binaries)
- **Real-time file watching** for incremental updates
  - Monitors workspace for file changes (create, modify, delete)
  - Incremental re-indexing of changed files only
  - Efficient update mechanism (no full rescans)
  - Respects .gitignore patterns
- **NDJSON event stream parsing** for file scan output
  - Line-by-line parsing of scanner output
  - Rich metadata extraction (path, size, last modified, MIME type)
  - Event types: FileScanned, ScanProgress, ScanComplete, ScanError
  - Graceful handling of malformed JSON lines
  - Statistical logging (files found, total size, duration)
- **Scan statistics tracking**
  - Total files scanned counter
  - Total bytes processed counter
  - Scan duration measurement
  - Detailed logging to output channel

### Added - Phase 4: Polish & Testing

- **Process crash recovery** with circuit breaker pattern
  - Exponential backoff restart delays (1s → 32s)
  - Maximum 5 restart attempts before giving up
  - State machine: CLOSED → OPEN → HALF_OPEN → CLOSED
  - Automatic reset after successful operation
  - Manual reset capability via command
  - Prevents infinite restart loops
- **Status bar integration** with real-time updates
  - File count display ("Watching: 1,234 files")
  - Relative timestamps ("Updated: 2m ago")
  - State indicators (Starting, Indexing, Watching, Error)
  - Click to show output channel for details
  - Color-coded states (gray, blue, green, red)
  - Automatic timestamp refresh every minute
- **Command palette commands**
  - `Maproom: Show Output` - View detailed logs
  - `Maproom: Setup` - Re-run setup wizard
  - `Maproom: Restart Watchers` - Restart file watching processes
- **Comprehensive test suite** (71% code coverage, 270 tests)
  - Unit tests for all core modules
  - Integration tests for multi-process workflows
  - Mock implementations for VSCode APIs
  - Test coverage reporting via Vitest
  - Continuous integration ready
- **Platform detection and binary selection**
  - Automatic platform/architecture detection
  - Support for darwin-x64, darwin-arm64, linux-x64, linux-arm64, win32-x64
  - Binary path resolution per platform
  - File extension handling (.exe for Windows)
- **Comprehensive documentation**
  - User-facing README with installation guide
  - Troubleshooting guide for common issues
  - CHANGELOG tracking all features
  - API documentation in code comments
  - Manual testing checklist
  - Development guidelines

### Developer Experience

- **TypeScript with strict mode** for type safety
- **ESM modules** (import/export) throughout
- **Vitest test framework** with coverage reporting
- **Docker Compose configuration** for local development
- **VSCode extension development** patterns and best practices
- **Structured logging** to dedicated output channel
- **Error boundaries** preventing cascading failures

### Known Issues

- **Windows support is experimental**
  - File watching may be slower or miss events
  - Binary compatibility not fully tested
  - Feedback requested from Windows users
- **Large repositories (>10,000 files)**
  - Initial scan can take 5-10 minutes
  - Progress updates may appear slow
  - Subsequent updates are incremental and much faster
- **Memory usage**
  - Embedding generation requires 2-4GB RAM for large codebases
  - Docker services need 4GB+ allocated in Docker Desktop
  - May struggle on systems with 8GB total RAM
- **Ollama model download**
  - First-time setup downloads ~500MB embedding model
  - May take several minutes on slow connections
  - Download happens in background via Ollama
- **Search UI not yet implemented**
  - Extension indexes files successfully
  - Semantic search query interface coming in v0.2.0
  - Current version focuses on indexing foundation

### Technical Details

- **Extension activation**: `onStartupFinished` for minimal startup impact
- **Docker Compose file**: Located at `config/docker-compose.yml`
- **Binary location**: `bin/<platform>/maproom`
- **Database**: PostgreSQL 16 with pgvector on port 5433
- **Embedding service**: Ollama on port 11434
- **Test coverage**: 71% overall (96% config, 94% UI, 81% process, 100% utils)
- **Supported file types**: Text files only (binary detection via MIME type)

### Dependencies

- VSCode Engine: `^1.85.0`
- Node.js: `^20.0.0`
- Docker Desktop: `24.0+` (required)
- TypeScript: `^5.3.0`
- Vitest: `^1.0.0`

### Platform Requirements

| Platform | Architecture | Status |
|----------|--------------|--------|
| Linux | x64 | ✅ Supported |
| Linux | arm64 | ✅ Supported |
| macOS | arm64 (M1+) | ✅ Supported |
| macOS | x64 (Intel) | ✅ Supported |
| Windows | x64 | ⚠️ Experimental |

### Migration Notes

This is the initial release. No migration required.

### Upgrade Notes

This is the initial release. No upgrade path.

---

## [Unreleased]

### Planned for v0.2.0

- Semantic search query interface
- Search results panel
- Code preview in search results
- Keyboard shortcuts for quick search
- Search history and saved searches

### Planned for v0.3.0

- Workspace settings configuration
- Custom file exclusion patterns
- Configurable embedding models
- Performance optimizations for large repos
- Windows full support (exit experimental)

---

**Legend:**
- ✅ Supported - Fully tested and stable
- ⚠️ Experimental - Functional but not fully tested
- 🚧 In Progress - Under active development
- 📋 Planned - Scheduled for future release

[0.1.0]: https://github.com/crewchief/vscode-maproom/releases/tag/v0.1.0
[Unreleased]: https://github.com/crewchief/vscode-maproom/compare/v0.1.0...HEAD
