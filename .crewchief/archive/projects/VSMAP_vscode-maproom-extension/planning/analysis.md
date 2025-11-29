# Analysis: VSCode Maproom Extension

## Problem Definition

### Current State

Maproom provides powerful hybrid semantic search (FTS + vector + graph signals) through an MCP server that integrates with AI assistants (Claude Code, Cursor). The system works well but requires significant manual setup and maintenance:

**Setup Friction:**
1. Install `@crewchief/maproom-mcp` package
2. Run `setup --provider=<provider>` command
3. Configure `.mcp.json` or `.cursor/mcp.json`
4. Manually start Docker services (postgres + optional ollama)
5. Run initial `scan` command on repository
6. Remember to re-scan after branch switches or major changes

**Ongoing Maintenance:**
- Developers must manually trigger scans after branch switches
- No automatic detection of file changes requiring re-indexing
- Docker services may stop, requiring manual restart
- Index staleness is invisible until search results are outdated
- Multi-worktree workflows require manual coordination

**Result:** Users who would benefit from semantic search either:
- Skip setup due to complexity
- Set it up once but let indexes go stale
- Forget to use it because it's not reliably available

### User Pain Points

**From Developer Perspective:**

1. **Cognitive Load**
   - "Did I scan this repo?"
   - "When was the last scan?"
   - "Should I re-scan after this branch switch?"
   - "Are my Docker services running?"

2. **Context Switching**
   - Leave IDE to run terminal commands
   - Check Docker Desktop for service status
   - Remember syntax for scan/upsert commands
   - Configure MCP separately from VSCode

3. **Reliability Concerns**
   - Stale indexes return outdated results
   - Confidence erosion when results don't match current code
   - Abandonment after initial enthusiasm

4. **Environment Complexity**
   - Different setup for local vs devcontainer
   - PATH management for binaries
   - Docker networking differences
   - Provider credential management

### Target Users

**Primary:** Developers using VSCode/Cursor with AI assistants who want semantic code search

**Segments:**
- **Solo developers:** Working on personal/small team projects, value simplicity
- **Team members:** Need consistent index state across team, value automation
- **Polyglot developers:** Work with multiple languages, value broad tree-sitter support
- **AI-first developers:** Rely heavily on Claude/Cursor, expect tools to "just work"

**Common characteristics:**
- Already use VSCode/Cursor as primary IDE
- Already use MCP for AI assistant integration
- Want semantic search but unwilling to maintain manual workflows
- Expect IDE extensions to be low-friction and automatic

### Industry Context

**Existing Solutions:**

1. **GitHub Copilot Chat**
   - Semantic search via `@workspace` symbol
   - Automatic indexing (opaque)
   - Cloud-based embeddings
   - Pros: Zero setup, always available
   - Cons: Proprietary, limited to GitHub's infrastructure, no local control

2. **Sourcegraph Extension**
   - Code search across repositories
   - Requires Sourcegraph account/instance
   - Pros: Powerful search, cross-repo
   - Cons: Heavy infrastructure, enterprise-focused, external dependency

3. **VSCode Built-in Search**
   - Fast text search (ripgrep)
   - Regex support
   - Pros: Fast, zero setup
   - Cons: No semantic understanding, exact-match only

4. **Phind for VSCode**
   - AI-powered search and explanations
   - Cloud API
   - Pros: Natural language queries
   - Cons: Cloud dependency, subscription model

**Maproom's Differentiation:**
- **Local-first:** Can run entirely local (Ollama)
- **MCP-native:** Designed for AI assistant workflows
- **Hybrid search:** FTS + vector + graph (better results)
- **Open source:** Full transparency and control
- **Multi-provider:** Choose Ollama/OpenAI/Google based on needs

**Gap:** Maproom has the best technical foundation but worst UX. Extension fixes this.

## Research Findings

### VSCode Extension Ecosystem

**Extension Capabilities Relevant to This Project:**

1. **Activation Events**
   - `onStartupFinished`: Run after VSCode fully loads (ideal for background indexing)
   - `workspaceContains:.git`: Activate when git repo detected
   - Resource-efficient startup patterns

2. **Background Tasks**
   - `vscode.tasks` API for long-running processes
   - Extension-managed child processes
   - Progress notifications (`withProgress`)
   - Cancellation tokens for cleanup

3. **File System Watching**
   - `vscode.workspace.createFileSystemWatcher`
   - `.git/HEAD` monitoring for branch switches
   - Efficient change detection
   - Respects `.gitignore` patterns

4. **Secrets Management**
   - `vscode.SecretStorage` API (encrypted, per-user)
   - Secure storage for API keys
   - Cross-platform credential management

5. **Configuration**
   - Workspace settings (`settings.json`)
   - User settings (global defaults)
   - Configuration schema with validation
   - Change listeners

6. **Status Bar Integration**
   - Persistent status items
   - Click handlers
   - Dynamic icons and text
   - Tooltips for details

### Docker Integration Patterns

**Common Approaches:**

1. **Docker Extension API**
   - Microsoft's official Docker extension exposes API
   - `vscode.docker` namespace (if extension installed)
   - Pros: Clean abstraction
   - Cons: Requires Docker extension, limited API

2. **Direct `docker` CLI**
   - Spawn `docker compose` commands
   - Parse stdout/stderr
   - Pros: Full control, no dependencies
   - Cons: Requires Docker installed, PATH management

3. **Docker SDK (Node)**
   - `dockerode` npm package
   - Programmatic container management
   - Pros: Type-safe, feature-rich
   - Cons: Large dependency, complex API

**Recommendation:** Use direct CLI approach (same as `packages/maproom-mcp/bin/cli.cjs`)
- Reuse existing docker-compose.yml files
- Consistent with current Maproom tooling
- Minimal dependencies
- Well-tested patterns already exist

### Indexing Lifecycle Patterns

**From Existing Implementations:**

1. **Branch Watcher** (`crewchief maproom branch-watch`)
   - Monitors `.git/HEAD` with OS file watcher
   - <1s detection latency
   - Incremental updates (content-addressed deduplication)
   - Graceful shutdown on Ctrl+C
   - **Key insight:** Very lightweight, perfect for extension background task

2. **File Watcher** (`npx @crewchief/maproom-mcp watch`)
   - Uses chokidar for file monitoring
   - 3-second debounce prevents spam
   - Respects .gitignore
   - Auto-upserts changed files
   - **Key insight:** Debouncing is critical for performance

3. **Initial Scan** (`scan` command)
   - 4 concurrent workers default
   - Progress reporting via stdout
   - 150-200 files/min throughput
   - **Key insight:** Users need progress visibility for large repos

**Best Practice:** Combine branch watcher + file watcher
- Branch watcher: Catch major changes (checkout)
- File watcher: Catch incremental edits
- Both feed into same upsert queue

### Developer Installation Patterns

**Pre-Marketplace Installation Methods:**

1. **VSIX Packaging**
   ```bash
   vsce package
   code --install-extension maproom-0.1.0.vsix
   ```
   - Standard VSCode extension format
   - Easy to share and install
   - Works with Cursor via `cursor --install-extension`

2. **Development Mode**
   ```bash
   cd packages/vscode-maproom
   npm install
   npm run compile
   code --extensionDevelopmentHost=.
   ```
   - Hot reload during development
   - Debugging support
   - Requires manual activation

3. **Symlink Installation**
   ```bash
   ln -s $(pwd)/packages/vscode-maproom ~/.vscode/extensions/maproom
   ```
   - Automatic updates
   - Requires reload
   - Platform-specific paths

**Recommendation:** Document all three approaches
- VSIX for testing/demo
- Development mode for contributors
- Symlink for continuous development

### Devcontainer Considerations

**Key Requirements:**

1. **Docker-in-Docker vs Docker-outside-of-Docker**
   - Devcontainers can use host Docker socket
   - Mount `/var/run/docker.sock` for host access
   - Maproom services run on host, accessible from container

2. **Network Connectivity**
   - Container can connect to `host.docker.internal:5433` (macOS/Windows)
   - Linux requires `--network=host` or custom networking
   - Database URL must adapt to environment

3. **Binary Availability**
   - Pre-built `crewchief-maproom` binaries in repo
   - Platform detection (linux-x64, darwin-arm64, etc.)
   - Must work inside container architecture

**Design Decision:** Same experience everywhere
- Extension uses same Docker setup regardless of environment
- Detect platform, spawn appropriate binary
- Use `host.docker.internal` with fallback to `localhost`
- No special devcontainer mode (keep simple)

## Key Insights

### Technical Insights

1. **Leverage Existing Infrastructure**
   - Don't reimplement scan/upsert logic
   - Spawn Rust binary as subprocess
   - Reuse docker-compose configurations
   - Parse binary stdout for progress

2. **Background Processing is Critical**
   - Users won't tolerate blocking operations
   - Show progress for long-running scans
   - Debounce file changes to prevent spam
   - Graceful cancellation on workspace close

3. **Docker is Non-Negotiable**
   - PostgreSQL required for search
   - Ollama makes local embeddings practical
   - Docker Compose provides consistent environment
   - Must handle "Docker not installed" gracefully

4. **Provider Flexibility Matters**
   - Ollama: Free, local, zero-config
   - OpenAI: Fast, high-quality, costs money
   - Google: Alternative cloud option
   - Users want to switch without re-setup

### User Experience Insights

1. **"Set and Forget" is the Goal**
   - Extension should disappear after setup
   - Status bar item provides passive awareness
   - Only show notifications on errors or first scan
   - Manual commands for power users only

2. **Trust Through Visibility**
   - Users need to know index is fresh
   - "Last updated 2 minutes ago" builds confidence
   - Error states must be obvious and actionable
   - Health checks prevent silent failures

3. **Progressive Complexity**
   - Default setup should be one-click (Ollama)
   - Advanced users can choose OpenAI/Google
   - Manual controls available but hidden
   - Settings for power users

### Risk Insights

1. **Docker Desktop Issues**
   - Not all developers have Docker installed
   - Docker Desktop license changes (enterprise)
   - Docker daemon may not be running
   - **Mitigation:** Clear error messages, installation links

2. **Resource Usage Concerns**
   - Indexing is CPU-intensive
   - Embeddings consume memory
   - Continuous watching uses battery
   - **Mitigation:** Configurable concurrency, pause commands

3. **Platform Fragmentation**
   - Windows vs macOS vs Linux differences
   - ARM64 vs x64 binary selection
   - Docker networking varies by platform
   - **Mitigation:** Thorough testing, platform-specific docs

4. **Stale Index Detection**
   - Git operations outside VSCode (terminal)
   - Branch switches via `gh` CLI
   - Network file systems (slow watches)
   - **Mitigation:** Periodic full scans, manual rescan command

## Requirements Summary

### Functional Requirements

**Must Have (MVP):**
- FR1: Auto-scan repository on workspace open
- FR2: Watch `.git/HEAD` for branch switches
- FR3: Watch file changes with debouncing
- FR4: Auto-start Docker services (with opt-out)
- FR5: Provider configuration wizard
- FR6: Status bar item with index state
- FR7: Secure API key storage

**Should Have (Post-MVP):**
- FR8: Multi-workspace support
- FR9: Manual scan/upsert commands
- FR10: Service health monitoring
- FR11: Index statistics view

**Could Have (Future):**
- FR12: Scheduled full re-scans
- FR13: Custom embedding models
- FR14: Index sharing across machines
- FR15: Search UI in VSCode

### Non-Functional Requirements

**Performance:**
- NFR1: Extension activation <500ms
- NFR2: File change detection <1s
- NFR3: Branch switch detection <1s
- NFR4: Indexing throughput >100 files/min
- NFR5: Idle CPU usage <5%
- NFR6: Idle memory usage <50MB

**Reliability:**
- NFR7: Graceful handling of Docker unavailability
- NFR8: Auto-recovery from indexing failures
- NFR9: No data loss on extension crash
- NFR10: Works offline (Ollama provider)

**Security:**
- NFR11: API keys stored encrypted
- NFR12: No credentials logged
- NFR13: Database access restricted to extension
- NFR14: Validate all user inputs

**Usability:**
- NFR15: Setup wizard completable in <2 minutes
- NFR16: Clear error messages with actions
- NFR17: Status visible without clicking
- NFR18: Works in devcontainer without special config

**Maintainability:**
- NFR19: TypeScript with strict typing
- NFR20: Unit test coverage >70%
- NFR21: Integration tests for core workflows
- NFR22: Inline documentation for complex logic

## Conclusion

The VSCode Maproom extension addresses a clear market need: making semantic code search accessible without manual maintenance. By focusing on automatic indexing and service management, it removes the primary barrier to Maproom adoption.

**Key Success Factors:**
1. **Automation:** Indexing happens transparently
2. **Simplicity:** Setup in <2 minutes
3. **Reliability:** Always ready when needed
4. **Flexibility:** Works with any embedding provider
5. **Integration:** Seamless with existing MCP workflows

**Next Steps:** Proceed to architecture phase with focus on MVP scope and pragmatic technical choices.
