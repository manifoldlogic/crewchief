# Ticket: VSMAP-4003: Create user documentation and README

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation only)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Write comprehensive README with installation instructions, troubleshooting guide, and usage examples. Document known limitations and system requirements.

## Background
This continues Phase 4 (Polish & Testing) of the VSMAP plan. Users need clear documentation to install, configure, and troubleshoot the extension. Good documentation reduces support burden and improves user experience. This ticket can be done in parallel with other Phase 4 work.

Reference: VSMAP_PLAN.md Phase 4 "Polish & Testing - Documentation"

## Acceptance Criteria
- [ ] README.md includes installation steps (VSIX install instructions)
- [ ] System requirements documented (Docker Desktop, VSCode 1.85+)
- [ ] Troubleshooting section covers common issues (Docker, binary errors)
- [ ] Usage examples show setup wizard, status bar, commands
- [ ] Known limitations clearly listed (Windows experimental, etc.)
- [ ] CHANGELOG.md created with version 0.1.0 initial release entry
- [ ] Screenshots included for setup wizard and status bar

## Technical Requirements
- Markdown format following standard README structure
- Screenshots stored in `docs/images/` directory
- Links to external resources (Docker Desktop, VSCode docs)
- Platform support matrix table (OS, architecture, status)
- Clear error message solutions in troubleshooting
- Example commands with expected output
- License section (match project license)

## Implementation Notes
README.md structure:

```markdown
# VSCode Maproom Extension

> Semantic code search powered by Maproom - index and search your codebase by meaning, not just text.

## Features
- 🔍 Semantic search using embeddings
- 🚀 Real-time indexing with file watching
- 🐳 Integrated Docker services (no manual setup)
- 🔒 Secure credential storage
- 📊 Progress tracking and status updates

## System Requirements
- VSCode 1.85.0 or higher
- Docker Desktop 24.0+ (running)
- 4GB RAM minimum (8GB recommended)
- 2GB free disk space

## Platform Support
| Platform | Architecture | Status |
|----------|--------------|--------|
| Linux | x64 | ✅ Supported |
| Linux | arm64 | ✅ Supported |
| macOS | arm64 (M1+) | ✅ Supported |
| macOS | x64 (Intel) | ✅ Supported |
| Windows | x64 | ⚠️ Experimental |

## Installation

### From VSIX
1. Download latest VSIX from releases
2. Open VSCode
3. Run: `code --install-extension maproom-vscode-0.1.0.vsix`
4. Reload VSCode

### From Source
```bash
git clone https://github.com/your-org/maproom-vscode
cd maproom-vscode
npm install
npm run compile
code --extensionDevelopmentPath=$(pwd)
```

## Getting Started

1. **Open a workspace** - Extension activates automatically
2. **Setup wizard** - Select embedding provider:
   - Ollama (recommended, free, local)
   - OpenAI (requires API key)
   - Google (requires API key)
3. **Initial scan** - Wait for indexing to complete
4. **Search** - Use command palette: `Maproom: Search`

## Usage

### Status Bar
- **Starting...** - Docker services initializing
- **Indexing: 1,234 files** - Scan in progress
- **Watching** - Active, ready for search
- **Error** - Click for details

### Commands
- `Maproom: Setup` - Re-run setup wizard
- `Maproom: Search` - Semantic code search
- `Maproom: Restart Watchers` - Restart background processes

### Settings
- `maproom.provider` - Embedding provider (ollama/openai/google)
- `maproom.logLevel` - Log verbosity (info/debug/error)

## Troubleshooting

### Docker not starting
**Error**: "Docker services failed to start"

**Solution**:
1. Ensure Docker Desktop is running
2. Check Docker has 4GB+ memory allocated
3. Verify: `docker ps` works in terminal
4. Restart VSCode

### Binary permission denied (Linux/macOS)
**Error**: "EACCES: permission denied"

**Solution**:
```bash
chmod +x ~/.vscode/extensions/maproom.vscode-maproom-*/bin/*/crewchief-maproom
```

### Ollama not detected
**Error**: "Could not connect to Ollama"

**Solution**:
1. Verify Ollama running: `ollama list`
2. Check port 11434: `curl http://localhost:11434`
3. Select different provider in setup

### Process keeps crashing
**Error**: "Maproom watcher crashed after 5 restart attempts"

**Solution**:
1. Check Output channel: View → Output → Maproom
2. Look for error details in logs
3. Report issue with log excerpt

## Known Limitations
- **Windows**: Experimental support, file watching may be slow
- **Large repos**: Initial scan >10k files may take 5-10 minutes
- **Memory**: Embedding requires 2-4GB RAM for large codebases
- **Network**: OpenAI/Google require internet connection

## Development
See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup.

## License
[Your License]

## Support
- 🐛 [Report a bug](https://github.com/your-org/maproom-vscode/issues)
- 💡 [Request a feature](https://github.com/your-org/maproom-vscode/issues)
- 📖 [Documentation](https://maproom.dev)
```

CHANGELOG.md structure:
```markdown
# Changelog

## [0.1.0] - 2025-11-16

### Added
- Initial release
- Docker service management
- Setup wizard with provider selection
- Secure credential storage
- Initial workspace scanning
- Real-time file watching
- Process crash recovery
- Status bar integration

### Known Issues
- Windows support experimental
- Large workspaces (>10k files) may have slow initial scan
```

Screenshots to capture:
1. Setup wizard (QuickPick with providers)
2. Progress notification (indexing)
3. Status bar (various states)
4. Error notification (with action buttons)

Store in `docs/images/`:
- `setup-wizard.png`
- `progress-notification.png`
- `status-bar.png`
- `error-notification.png`

## Dependencies
- None (can be done in parallel with other Phase 4 work)
- VSMAP-4002 (manual testing) will inform Known Limitations section

## Risk Assessment
- **Risk**: Documentation may become outdated quickly
  - **Mitigation**: Link to code examples, version documentation clearly
- **Risk**: Screenshots may not match final UI
  - **Mitigation**: Take screenshots last, after UI finalized
- **Risk**: Troubleshooting may miss common issues
  - **Mitigation**: Update based on manual testing feedback

## Files/Packages Affected
- `README.md` (new comprehensive documentation)
- `CHANGELOG.md` (new file, version history)
- `docs/TROUBLESHOOTING.md` (new detailed troubleshooting guide)
- `docs/images/` (new directory with screenshots)
- `CONTRIBUTING.md` (optional, development guidelines)
