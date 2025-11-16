# Execution Plan: VSCode Maproom Extension

## Overview

This plan outlines the phased development of the Maproom VSCode extension, focusing on delivering automatic indexing capabilities with a pragmatic MVP approach. The plan integrates insights from analysis, architecture, quality strategy, security review, and agent suggestions.

## Project Scope Reminder

**In Scope (MVP):**
- ✅ Automatic repository scanning on workspace open
- ✅ File change watching with debounced updates
- ✅ Branch switch detection and incremental re-indexing
- ✅ Docker lifecycle management (auto-start with manual override)
- ✅ Provider configuration wizard (Ollama/OpenAI/Google)
- ✅ Status bar integration
- ✅ Development installation documentation

**Out of Scope (Post-MVP):**
- ❌ Search UI in VSCode (use MCP instead)
- ❌ Multi-workspace support
- ❌ Custom embedding models
- ❌ Marketplace publishing (VSIX only for now)

## Prerequisites

### 0. Agent Creation Phase
**Duration:** 1-2 days
**Deliverable:** Specialized agent definitions

**Tasks:**
1. Create VSCode Extension Specialist agent
   - Define expertise and responsibilities
   - Provide VSCode API documentation
   - Add example patterns from architecture
   - Test agent with simple extension task

2. Create Process Management Specialist agent
   - Define child process expertise
   - Document stream handling patterns
   - Add cross-platform considerations
   - Test agent with binary spawning task

3. Create Configuration & Secrets Specialist agent
   - Define SecretStorage expertise
   - Document configuration patterns
   - Add validation strategies
   - Test agent with credential storage task

**Acceptance Criteria:**
- [ ] All three agent definitions complete
- [ ] Agents tested with sample tasks
- [ ] Documentation comprehensive
- [ ] Ready for ticket assignment

**Assigned Agent:** Technical Researcher (for research), Manual (for agent file creation)

---

## Phase 1: Foundation (Week 1)

**Goal:** Extension scaffold, Docker integration, basic binary spawning

### Milestone 1.1: Extension Scaffold
**Duration:** 1-2 days
**Assigned Agent:** VSCode Extension Specialist

**Tasks:**
1. Initialize extension project in `packages/vscode-maproom/`
   - `package.json` with extension metadata
   - TypeScript configuration (tsconfig.json)
   - Build configuration (esbuild or webpack)
   - VSCode extension manifest

2. Implement extension entry points
   - `activate()` function with lifecycle logging
   - `deactivate()` function with cleanup
   - Basic error boundary
   - Output channel for logging

3. Register activation events
   - `onStartupFinished` for automatic activation
   - `workspaceContains:.git` for git repos only
   - Test activation performance (<500ms)

4. Setup development workflow
   - `npm run compile` for TypeScript build
   - `npm run watch` for continuous compilation
   - F5 debug configuration
   - `.vscodeignore` for packaging

**Deliverables:**
- `packages/vscode-maproom/src/extension.ts`
- `packages/vscode-maproom/package.json`
- `packages/vscode-maproom/tsconfig.json`
- Development launch configuration

**Acceptance Criteria:**
- [ ] Extension activates in Extension Development Host
- [ ] Activation time <500ms
- [ ] Logging to Output channel works
- [ ] Clean deactivation with no errors

**Testing:** Unit tests for activation lifecycle

---

### Milestone 1.2: Docker Manager
**Duration:** 2-3 days
**Assigned Agent:** Docker Engineer

**Tasks:**
1. Implement DockerManager class
   - Check Docker daemon status (`docker info`)
   - Start services (`docker compose up -d`)
   - Stop services (`docker compose down`)
   - Health check polling (postgres, ollama)

2. Service orchestration logic
   - Determine required services based on provider
   - Remove unused services when switching providers
   - Handle service dependencies (postgres before mcp)
   - Wait for healthy with timeout (120s)

3. Error handling
   - Docker not installed detection
   - Daemon not running detection
   - Service startup failures
   - Health check timeout errors

4. Configuration
   - Use bundled docker-compose.yml from `packages/maproom-mcp/config/`
   - Support override via settings (dockerComposePath)
   - Localhost-only binding (security)

**Deliverables:**
- `packages/vscode-maproom/src/docker/manager.ts`
- `packages/vscode-maproom/src/docker/health.ts`
- `packages/vscode-maproom/src/docker/types.ts`

**Acceptance Criteria:**
- [ ] Services start successfully on localhost
- [ ] Health checks detect when services are ready
- [ ] Graceful errors when Docker unavailable
- [ ] Unused services removed on provider switch

**Testing:**
- Integration tests with real Docker
- Unit tests for health check logic
- Error handling tests (mock Docker failures)

---

### Milestone 1.3: Rust Binary Spawner
**Duration:** 2-3 days
**Assigned Agent:** Process Management Specialist

**Tasks:**
1. Implement RustBinarySpawner class
   - Platform detection (darwin-arm64, linux-x64, etc.)
   - Binary path resolution from bundled binaries
   - Binary integrity verification (checksums)
   - Spawn with validated arguments

2. Progress parsing
   - Parse stdout for "Scanning: X/Y files"
   - Emit progress events for UI updates
   - Handle errors from stderr
   - Capture exit codes

3. Process lifecycle
   - Graceful termination on cancel
   - Timeout enforcement (10 min for scan)
   - Cleanup on extension deactivation
   - Retry logic for transient failures

4. Security
   - Use `spawn()` not `exec()` (no shell)
   - Validate all arguments before passing
   - Environment variables for config (not CLI args)
   - Path validation within workspace

**Deliverables:**
- `packages/vscode-maproom/src/utils/binary.ts`
- `packages/vscode-maproom/src/utils/platform.ts`
- Platform-specific binary checksums

**Acceptance Criteria:**
- [ ] Correct binary selected for each platform
- [ ] Binary checksum validated before spawn
- [ ] Progress reported during scan
- [ ] Cancellation works within 5 seconds
- [ ] No shell injection vulnerabilities

**Testing:**
- Unit tests for platform detection
- Integration tests with real binary
- Security tests (path validation, no shell)
- Performance tests (spawn overhead <100ms)

---

### Milestone 1.4: Status Bar Integration
**Duration:** 1 day
**Assigned Agent:** VSCode Extension Specialist

**Tasks:**
1. Implement StatusBarManager class
   - Create status bar item (right side)
   - Update text and icon based on state
   - Click handler for details
   - Tooltip with metadata

2. State management
   - NOT_CONFIGURED → "⚙️ Maproom: Setup"
   - INDEXING → "⟳ Indexing... (X/Y)"
   - HEALTHY → "✓ Indexed (2m ago)"
   - ERROR → "✗ Error"

3. Real-time updates
   - Listen to indexing progress events
   - Update every second during scan
   - Debounce updates (max 1/second)
   - Hide when not relevant (no workspace)

**Deliverables:**
- `packages/vscode-maproom/src/ui/statusBar.ts`
- Status icons and text formatting

**Acceptance Criteria:**
- [ ] Status bar visible on activation
- [ ] Updates reflect indexing progress
- [ ] Click opens detailed status panel
- [ ] States render correctly

**Testing:**
- Unit tests for state transitions
- E2E tests for UI updates
- Manual testing for visual appearance

---

### Phase 1 Checkpoint

**Deliverables:**
- Working extension scaffold
- Docker services start/stop automatically
- Rust binary spawns successfully
- Status bar shows basic status

**Validation:**
- [ ] Extension installs via VSIX
- [ ] Docker services healthy
- [ ] Binary spawns and exits cleanly
- [ ] Status bar updates correctly

**Duration:** ~5-7 days total

---

## Phase 2: Indexing (Week 2)

**Goal:** File watching, branch watching, automatic indexing

### Milestone 2.1: Initial Scan Implementation
**Duration:** 2 days
**Assigned Agents:** Process Management Specialist, TypeScript Developer

**Tasks:**
1. Implement IndexingManager class
   - Orchestrate scan workflow
   - Manage Rust binary lifecycle
   - Track index status
   - Provide progress callbacks

2. Git repository detection
   - Find `.git` directory
   - Extract repo name from remote
   - Get current branch from `.git/HEAD`
   - Get commit SHA from `git rev-parse HEAD`

3. Progress notifications
   - Show notification during scan
   - Update progress (0-100%)
   - Support cancellation
   - Handle errors gracefully

4. Integration with status bar
   - Update status during scan
   - Show completion time
   - Persist last scan timestamp

**Deliverables:**
- `packages/vscode-maproom/src/indexing/manager.ts`
- `packages/vscode-maproom/src/utils/git.ts`

**Acceptance Criteria:**
- [ ] Scan completes for test repository
- [ ] Progress notification shows percentage
- [ ] Cancellation stops binary cleanly
- [ ] Status bar reflects completion

**Testing:**
- Integration tests with test repositories
- Unit tests for git metadata extraction
- E2E tests for user flow

---

### Milestone 2.2: File Watcher
**Duration:** 2 days
**Assigned Agent:** TypeScript Developer

**Tasks:**
1. Implement FileWatcher class
   - Use `vscode.workspace.createFileSystemWatcher`
   - Watch `**/*` pattern (all files)
   - Exclude `.git/**` directory
   - Respect .gitignore patterns

2. Debouncing logic
   - Collect changes for 3 seconds
   - Reset timer on new changes
   - Batch upsert when timer expires
   - Queue multiple batches if needed

3. File change handling
   - onCreate → add to index
   - onChange → update index
   - onDelete → remove from index
   - Binary files → skip

4. Integration with IndexingManager
   - Call `upsert()` with changed files
   - Update status bar on completion
   - Handle errors without crashing watcher

**Deliverables:**
- `packages/vscode-maproom/src/indexing/fileWatcher.ts`
- `packages/vscode-maproom/src/indexing/debouncer.ts`

**Acceptance Criteria:**
- [ ] File save triggers update after 3s
- [ ] Multiple rapid saves batched correctly
- [ ] .git directory changes ignored
- [ ] Watcher survives errors

**Testing:**
- Unit tests for debouncing algorithm
- Integration tests with real file changes
- E2E tests for multi-file batching

---

### Milestone 2.3: Branch Watcher
**Duration:** 2 days
**Assigned Agent:** TypeScript Developer

**Tasks:**
1. Implement BranchWatcher class
   - Watch `.git/HEAD` file for changes
   - Parse current branch from HEAD content
   - Detect branch switches
   - Detect detached HEAD state

2. Incremental scan on branch switch
   - Trigger scan for new branch
   - Use worktree name = branch name
   - Incremental mode (only changed files)
   - Content-addressed deduplication (BLOBSHA)

3. Edge cases
   - Concurrent branch switches (queue)
   - .git/HEAD parse errors (corrupted)
   - Branch switch during active scan (cancel and restart)
   - First scan vs subsequent scans

4. User notifications
   - Show toast: "Branch changed to {branch}, re-indexing..."
   - Hide notification on completion
   - Error notification on failure

**Deliverables:**
- `packages/vscode-maproom/src/indexing/branchWatcher.ts`

**Acceptance Criteria:**
- [ ] Branch switch detected within 1 second
- [ ] Incremental scan triggered automatically
- [ ] Detached HEAD handled gracefully
- [ ] Concurrent switches queued properly

**Testing:**
- Unit tests for HEAD parsing
- Integration tests with git checkouts
- E2E tests for branch switching workflow

---

### Milestone 2.4: Auto-Start Workflow
**Duration:** 1 day
**Assigned Agent:** VSCode Extension Specialist

**Tasks:**
1. Implement auto-start logic in activate()
   - Check if configured (provider, credentials)
   - Start Docker services if autoStart=true
   - Wait for services healthy
   - Run initial scan if not scanned before

2. First-time detection
   - Check for existing index (query database)
   - Prompt user to scan if no index
   - Track last scan timestamp
   - Skip re-scan if recent (<1 hour)

3. Error recovery
   - Retry Docker start on transient failures
   - Show helpful errors with actions
   - Allow manual retry via command
   - Graceful degradation (skip scan, allow manual)

**Deliverables:**
- Enhanced `packages/vscode-maproom/src/extension.ts`
- Auto-start state machine

**Acceptance Criteria:**
- [ ] Extension auto-starts on workspace open
- [ ] Services become healthy within 2 minutes
- [ ] Initial scan prompts user
- [ ] Errors show actionable messages

**Testing:**
- E2E tests for cold start
- Integration tests for warm start (services already running)
- Error handling tests

---

### Phase 2 Checkpoint

**Deliverables:**
- Automatic indexing on workspace open
- File changes trigger updates
- Branch switches trigger re-indexing
- Complete auto-start workflow

**Validation:**
- [ ] Open workspace → services start → scan completes
- [ ] Save file → update after 3s
- [ ] Switch branch → incremental re-index
- [ ] Status bar always accurate

**Duration:** ~6-7 days total

---

## Phase 3: Configuration & Setup (Week 3)

**Goal:** Provider configuration, credential management, setup wizard

### Milestone 3.1: Configuration Schema
**Duration:** 1 day
**Assigned Agent:** Configuration & Secrets Specialist

**Tasks:**
1. Define configuration schema in package.json
   - `maproom.autoStart` (boolean, default: true)
   - `maproom.provider` (enum: ollama|openai|google, default: ollama)
   - `maproom.databaseUrl` (string, default: auto-detect)
   - `maproom.watchDebounce` (number, default: 3000)
   - `maproom.scanConcurrency` (number, default: 4)
   - `maproom.dockerAutoManage` (boolean, default: true)
   - `maproom.showProgress` (boolean, default: true)

2. Implement ConfigurationManager class
   - Read workspace/user settings
   - Validate configuration values
   - Provide sensible defaults
   - Handle configuration changes

3. Validation logic
   - Concurrency: 1-16
   - Debounce: 500-10000ms
   - Provider: enum validation
   - Database URL: connection string format

**Deliverables:**
- Configuration schema in `package.json`
- `packages/vscode-maproom/src/config/manager.ts`
- `packages/vscode-maproom/src/config/validation.ts`

**Acceptance Criteria:**
- [ ] Settings appear in VSCode Settings UI
- [ ] Invalid values rejected with errors
- [ ] Defaults applied correctly
- [ ] Changes propagate immediately

**Testing:**
- Unit tests for validation logic
- Integration tests for settings updates

---

### Milestone 3.2: SecretStorage Integration
**Duration:** 1-2 days
**Assigned Agent:** Configuration & Secrets Specialist

**Tasks:**
1. Implement SecretsManager class
   - Store API keys: `maproom.{provider}.apiKey`
   - Retrieve from SecretStorage
   - Delete on provider switch
   - Environment variable fallback

2. Security measures
   - Never log API keys
   - Redact credentials in errors
   - Use environment variables for subprocess
   - Validate keys before storing

3. Provider-specific keys
   - OpenAI: `OPENAI_API_KEY`
   - Google: `GOOGLE_PROJECT_ID` (not secret, in settings)
   - Google: `GOOGLE_APPLICATION_CREDENTIALS` (path)
   - Ollama: No credentials needed

4. Migration logic
   - Detect old config format (if any)
   - Migrate to SecretStorage
   - Clear old plaintext config
   - One-time migration on activate

**Deliverables:**
- `packages/vscode-maproom/src/config/secrets.ts`
- Credential validation functions

**Acceptance Criteria:**
- [ ] API keys stored encrypted
- [ ] Keys never appear in logs
- [ ] Environment variable fallback works
- [ ] Migration completes successfully

**Testing:**
- Unit tests for credential storage
- Security tests (no logging)
- Integration tests with VSCode SecretStorage

---

### Milestone 3.3: Provider Configuration
**Duration:** 1-2 days
**Assigned Agent:** Configuration & Secrets Specialist

**Tasks:**
1. Implement provider selection logic
   - Ollama: No setup, auto-pull model
   - OpenAI: Require API key validation
   - Google: Require project ID and credentials

2. Credential validation
   - OpenAI: Test API call to `/v1/models`
   - Google: Validate project ID format
   - Ollama: Check service health
   - Show validation errors clearly

3. Provider switching
   - Prompt to confirm switch
   - Restart Docker with new services
   - Clear old provider credentials (optional)
   - Re-index with new provider

4. Environment setup
   - Set `MAPROOM_EMBEDDING_PROVIDER` env var
   - Pass credentials to Rust binary via env
   - Configure docker-compose service selection

**Deliverables:**
- `packages/vscode-maproom/src/providers/config.ts`
- `packages/vscode-maproom/src/providers/validation.ts`

**Acceptance Criteria:**
- [ ] Provider selection persists
- [ ] Credentials validated before save
- [ ] Docker services match provider
- [ ] Switching providers works smoothly

**Testing:**
- Integration tests for each provider
- Validation tests (valid/invalid keys)
- Provider switching E2E tests

---

### Milestone 3.4: Setup Wizard
**Duration:** 2-3 days
**Assigned Agent:** VSCode Extension Specialist

**Tasks:**
1. Implement setup wizard UI
   - Welcome screen explaining Maproom
   - Provider selection (QuickPick)
   - Credential input (InputBox, password)
   - Validation and error display
   - Success screen with next steps

2. Wizard flow
   - Step 1: Welcome
   - Step 2: Choose provider
   - Step 3: Enter credentials (if needed)
   - Step 4: Validate credentials
   - Step 5: Start Docker services
   - Step 6: Wait for healthy
   - Step 7: Prompt for initial scan
   - Step 8: Success

3. Error handling
   - Invalid credentials → retry
   - Docker fails → show error + retry
   - User cancels → save partial progress
   - Validation timeout → retry option

4. Skip/retry options
   - Allow skipping initial scan
   - Retry button on failures
   - Cancel anytime
   - Resume from last step

**Deliverables:**
- `packages/vscode-maproom/src/ui/setupWizard.ts`
- Welcome and success messaging

**Acceptance Criteria:**
- [ ] Wizard completes for Ollama (no credentials)
- [ ] Wizard validates OpenAI API key
- [ ] Wizard validates Google credentials
- [ ] Errors show actionable messages
- [ ] Wizard can be cancelled

**Testing:**
- E2E tests for complete wizard flow
- Error handling tests (mock failures)
- UX testing (manual)

---

### Phase 3 Checkpoint

**Deliverables:**
- Complete configuration system
- Secure credential storage
- Provider configuration and validation
- Polished setup wizard

**Validation:**
- [ ] New user completes setup in <2 minutes
- [ ] All providers configure successfully
- [ ] Credentials stored securely
- [ ] Settings UI works correctly

**Duration:** ~5-7 days total

---

## Phase 4: Testing, Polish & Documentation (Week 4)

**Goal:** Comprehensive testing, documentation, development installation guide

### Milestone 4.1: Unit Tests
**Duration:** 2 days
**Assigned Agent:** Test Engineer

**Tasks:**
1. Unit test coverage for critical logic
   - Debouncing algorithm (fileWatcher)
   - Path validation (security)
   - Platform detection (binary)
   - Configuration validation
   - Branch parsing (branchWatcher)

2. Test infrastructure setup
   - Vitest configuration
   - Test utilities and mocks
   - Coverage reporting
   - CI integration

3. Achieve >70% coverage
   - Focus on complex logic
   - Skip trivial getters/setters
   - Mock VSCode APIs
   - Test error paths

**Deliverables:**
- Unit tests in `src/**/*.test.ts`
- Vitest configuration
- Coverage reports

**Acceptance Criteria:**
- [ ] All unit tests passing
- [ ] Coverage >70%
- [ ] Critical paths 100% coverage
- [ ] Fast execution (<30s total)

---

### Milestone 4.2: Integration Tests
**Duration:** 2-3 days
**Assigned Agent:** Test Engineer

**Tasks:**
1. Docker integration tests
   - Service startup
   - Health checks
   - Service removal
   - Error scenarios

2. Binary spawning tests
   - Scan execution
   - Upsert execution
   - Progress parsing
   - Cancellation

3. Database integration tests
   - Connection validation
   - Index queries
   - Schema verification

4. SecretStorage tests
   - Store and retrieve
   - Delete credentials
   - Environment fallback

**Deliverables:**
- Integration tests in `src/test/integration/*.test.ts`
- Test fixtures and helpers

**Acceptance Criteria:**
- [ ] All integration tests passing
- [ ] Tests run on CI
- [ ] Real Docker/database used
- [ ] Cleanup after tests

---

### Milestone 4.3: E2E Tests
**Duration:** 2-3 days
**Assigned Agent:** Test Engineer

**Tasks:**
1. Setup workflow E2E test
   - Complete wizard for each provider
   - Verify services started
   - Verify initial scan

2. File watching E2E test
   - Save file → update triggered
   - Multiple files → batched
   - Verify status bar updates

3. Branch switching E2E test
   - Checkout branch → scan triggered
   - Verify incremental update
   - Verify status bar

4. Error recovery E2E test
   - Docker not running → error shown
   - Binary fails → error shown
   - Retry works

**Deliverables:**
- E2E tests in `src/test/e2e/*.test.ts`
- @vscode/test-electron configuration

**Acceptance Criteria:**
- [ ] All E2E tests passing
- [ ] Tests run in CI (Linux only)
- [ ] Cover critical workflows
- [ ] Realistic test scenarios

---

### Milestone 4.4: Development Installation Documentation
**Duration:** 1-2 days
**Assigned Agent:** Technical Researcher

**Tasks:**
1. Write installation guide for developers
   - Prerequisites (Node.js, Docker, pnpm)
   - Build instructions
   - VSIX packaging
   - Installation in VSCode/Cursor

2. Document three installation methods
   - **Method 1:** VSIX package (for testing)
   - **Method 2:** Symlink (for development)
   - **Method 3:** Debug mode (for contributors)

3. Platform-specific instructions
   - macOS setup
   - Linux setup
   - Windows setup (if supported)
   - Devcontainer setup

4. Troubleshooting guide
   - Docker not found
   - Binary not executable
   - Extension won't activate
   - Services unhealthy

**Deliverables:**
- `packages/vscode-maproom/DEVELOPMENT.md`
- `packages/vscode-maproom/INSTALLATION.md`
- `packages/vscode-maproom/TROUBLESHOOTING.md`

**Acceptance Criteria:**
- [ ] Instructions complete for all methods
- [ ] Tested on all platforms
- [ ] Screenshots/examples included
- [ ] Troubleshooting covers common issues

---

### Milestone 4.5: Code Quality & Polish
**Duration:** 1-2 days
**Assigned Agents:** All

**Tasks:**
1. Code review and refactoring
   - Remove dead code
   - Improve naming
   - Add missing comments
   - Extract reusable utilities

2. Performance optimization
   - Measure activation time
   - Optimize hot paths
   - Reduce memory usage
   - Profile extension

3. Error message improvements
   - Make errors actionable
   - Add "Learn More" links
   - Include troubleshooting hints
   - Test all error paths

4. Logging improvements
   - Structured logging
   - Appropriate log levels
   - No credential leaks
   - Helpful debug output

**Deliverables:**
- Refactored codebase
- Performance benchmarks
- Improved error messages
- Comprehensive logging

**Acceptance Criteria:**
- [ ] Activation time <500ms
- [ ] Memory usage <50MB idle
- [ ] All errors actionable
- [ ] No credentials in logs

---

### Milestone 4.6: Security Audit
**Duration:** 1 day
**Assigned Agent:** Configuration & Secrets Specialist

**Tasks:**
1. Implement security mitigations from security-review.md
   - Path validation comprehensive
   - Binary checksum verification
   - Credential logging prevention
   - No shell injection

2. Security testing
   - Attempt path traversal
   - Verify credentials encrypted
   - Check HTTPS enforcement
   - Validate input sanitization

3. Security documentation
   - Document data flows
   - Privacy policy (what leaves machine)
   - Security best practices
   - Threat model summary

**Deliverables:**
- Security tests passing
- `packages/vscode-maproom/SECURITY.md`
- Updated README with privacy info

**Acceptance Criteria:**
- [ ] All high-priority security gaps addressed
- [ ] Security tests passing
- [ ] Data flows documented
- [ ] No critical vulnerabilities

---

### Phase 4 Checkpoint

**Deliverables:**
- Comprehensive test suite (unit, integration, E2E)
- Development installation documentation
- Polished codebase
- Security audit complete

**Validation:**
- [ ] All tests passing on CI
- [ ] Documentation complete and tested
- [ ] Extension ready for distribution
- [ ] Security review passed

**Duration:** ~7-10 days total

---

## Release Preparation

### Pre-Release Checklist

**Code Quality:**
- [ ] All tests passing (unit, integration, E2E)
- [ ] Test coverage >70%
- [ ] No TypeScript errors
- [ ] No linting errors
- [ ] No security vulnerabilities (`npm audit`)

**Functionality:**
- [ ] Extension activates in <500ms
- [ ] Setup wizard completes for all providers
- [ ] Automatic indexing works (scan, watch, branch)
- [ ] Docker services start/stop correctly
- [ ] Status bar accurate
- [ ] Error handling graceful

**Documentation:**
- [ ] DEVELOPMENT.md complete
- [ ] INSTALLATION.md complete
- [ ] TROUBLESHOOTING.md complete
- [ ] SECURITY.md complete
- [ ] README.md updated
- [ ] CHANGELOG.md created

**Security:**
- [ ] Credentials stored encrypted
- [ ] No credentials in logs
- [ ] Path validation comprehensive
- [ ] Binary checksums verified
- [ ] HTTPS enforced for cloud APIs

**Platform Testing:**
- [ ] Works on macOS (Intel)
- [ ] Works on macOS (Apple Silicon)
- [ ] Works on Linux (x64)
- [ ] Works on Windows (x64) - if supported
- [ ] Works in devcontainer

**Performance:**
- [ ] Activation time <500ms
- [ ] Scan throughput >100 files/min
- [ ] Memory usage <50MB idle
- [ ] CPU usage <5% idle

### VSIX Packaging

```bash
cd packages/vscode-maproom

# Install packaging tool
pnpm add -D @vscode/vsce

# Package extension
pnpm run package

# Output: maproom-0.1.0.vsix

# Generate checksum
sha256sum maproom-0.1.0.vsix > maproom-0.1.0.vsix.sha256

# Test installation
code --install-extension maproom-0.1.0.vsix
```

### Distribution

**Initial Release:**
1. Publish VSIX to GitHub Releases
2. Include installation instructions
3. Include checksum for verification
4. Tag release (e.g., `vscode-maproom-v0.1.0`)

**Future:**
- Publish to VSCode Marketplace
- Publish to Open VSX (for Cursor)
- Automated releases via GitHub Actions

---

## Timeline Summary

| Phase | Duration | Deliverables |
|-------|----------|-------------|
| **Phase 0: Agent Creation** | 1-2 days | 3 specialized agents |
| **Phase 1: Foundation** | 5-7 days | Extension scaffold, Docker, binary spawning, status bar |
| **Phase 2: Indexing** | 6-7 days | File/branch watching, auto-start, automatic indexing |
| **Phase 3: Configuration** | 5-7 days | Config schema, SecretStorage, setup wizard |
| **Phase 4: Testing & Polish** | 7-10 days | Tests, docs, security, polish |
| **Release Preparation** | 1-2 days | Packaging, testing, distribution |
| **TOTAL** | **25-35 days** | **Functional MVP extension** |

**Calendar Estimate:** 5-7 weeks (accounting for delays, reviews, iterations)

---

## Risk Management

### High-Risk Items

**Risk 1: Docker Integration Complexity**
- **Impact:** Extension won't work if Docker fails
- **Mitigation:** Extensive integration tests, graceful errors, manual override
- **Contingency:** Provide Docker troubleshooting guide, manual start commands

**Risk 2: Platform-Specific Binary Issues**
- **Impact:** Extension fails on some platforms
- **Mitigation:** Test on all platforms, bundle all binaries, checksum verification
- **Contingency:** Platform-specific documentation, binary download fallback

**Risk 3: VSCode Extension API Learning Curve**
- **Impact:** Delays in UI implementation
- **Mitigation:** Create VSCode Extension Specialist agent first, reference examples
- **Contingency:** Simplify UI, use simpler alternatives (commands vs WebView)

**Risk 4: SecretStorage Security Issues**
- **Impact:** Credentials leaked
- **Mitigation:** Automated tests, code review, security audit
- **Contingency:** Environment variable fallback, clear security documentation

### Medium-Risk Items

**Risk 5: Test Coverage Insufficient**
- **Impact:** Bugs slip through
- **Mitigation:** Target 70% coverage, focus on critical paths
- **Contingency:** Manual testing checklist, beta testing period

**Risk 6: File Watching Edge Cases**
- **Impact:** Missed file changes
- **Mitigation:** Comprehensive E2E tests, debouncing tests
- **Contingency:** Manual rescan command, periodic full scans

**Risk 7: Performance Degradation**
- **Impact:** Slow activation or indexing
- **Mitigation:** Performance benchmarks, profiling
- **Contingency:** Lazy loading, background processing, concurrency limits

---

## Success Metrics

### MVP Success Criteria

**Functional:**
1. ✅ Extension installs and activates successfully
2. ✅ Setup wizard completes in <2 minutes
3. ✅ Automatic indexing works without user intervention
4. ✅ Status bar reflects accurate state
5. ✅ Works with all three providers (Ollama, OpenAI, Google)

**Technical:**
1. ✅ Activation time <500ms
2. ✅ Test coverage >70%
3. ✅ Memory usage <50MB idle
4. ✅ No critical security vulnerabilities
5. ✅ Works on macOS, Linux, Windows (if supported)

**User Experience:**
1. ✅ Installation documentation clear and complete
2. ✅ Error messages actionable
3. ✅ Setup wizard intuitive
4. ✅ No manual steps required after setup
5. ✅ Troubleshooting guide comprehensive

### Post-MVP Metrics (Future)

- Active installations (after marketplace publishing)
- Setup completion rate
- Error rate (crashes, failures)
- User feedback (GitHub issues, ratings)
- Performance metrics (activation time, scan speed)

---

## Next Steps After MVP

**Immediate (Phase 5):**
1. Beta testing with early adopters
2. Bug fixes based on feedback
3. Performance optimization
4. Documentation improvements

**Near-Term (Phases 6-7):**
1. Marketplace publishing
2. Multi-workspace support
3. Index statistics panel
4. Search UI (if needed)

**Long-Term (Future):**
1. Custom embedding models
2. Enterprise features (audit logging, policy enforcement)
3. Advanced configuration (exclude patterns, custom concurrency)
4. Performance analytics dashboard

---

## Agent Assignment Summary

| Agent | Primary Responsibilities | Estimated Tickets |
|-------|-------------------------|-------------------|
| **VSCode Extension Specialist** | Extension scaffold, status bar, setup wizard, packaging | 8-10 |
| **Process Management Specialist** | Binary spawning, progress parsing, process lifecycle | 6-8 |
| **Configuration & Secrets Specialist** | Config schema, SecretStorage, provider config, security | 6-8 |
| **Docker Engineer** | Docker orchestration, health checks, service management | 4-6 |
| **TypeScript Developer** | File watching, branch watching, utilities | 6-8 |
| **Test Engineer** | Unit, integration, E2E tests, CI configuration | 8-10 |
| **Technical Researcher** | Documentation, API research, troubleshooting guides | 4-6 |

**Total Tickets:** ~40-60

---

## Conclusion

This execution plan provides a clear roadmap from agent creation through MVP release. The phased approach ensures:

1. **Foundation First:** Core infrastructure (extension, Docker, binary) before features
2. **Incremental Value:** Each phase delivers working functionality
3. **Quality Built-In:** Testing and security integrated throughout
4. **Clear Ownership:** Agent assignments for each milestone
5. **Risk Management:** Identified risks with mitigations

**Key to Success:**
- Create specialized agents before starting (Phase 0)
- Follow phase sequence strictly (don't skip ahead)
- Test continuously (not just Phase 4)
- Document as you build (not at the end)
- Ship MVP, iterate based on feedback

**Target:** Functional MVP in 5-7 weeks, ready for development distribution.

**Next:** Begin Phase 0 (Agent Creation) to prepare for implementation.
