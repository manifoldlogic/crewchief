# Architecture: Devcontainer Go and MCP Language Server

## Approach

Use the official devcontainer Go feature for language installation and post-create script for MCP language server tooling.

## Design Decisions

### Decision 1: Use Devcontainer Feature for Go

**Choice**: `ghcr.io/devcontainers/features/go:1`

**Rationale**:
- Consistent with existing patterns (Node, Rust already use features)
- Automatic PATH configuration
- Version management built-in
- Official Microsoft-maintained feature

**Alternative Rejected**: Dockerfile `apt-get install golang-go`
- Outdated version in Ubuntu repos
- Manual PATH configuration
- Inconsistent with existing feature-based approach

### Decision 2: MCP Language Server in post-create.sh

**Choice**: Install via `go install` in post-create.sh

**Rationale**:
- Go must be available first (feature runs before script)
- `go install` is the canonical way to install Go tools
- Already have pattern for tool installation in post-create.sh (Claude Code, Husky, CrewChief CLI)

### Decision 3: Go Version

**Choice**: `latest` (default for feature)

**Rationale**:
- MCP language server uses modern Go features
- No specific version requirement identified
- Matches pattern of other languages (Rust uses latest)

## Changes Required

### File: `.devcontainer/devcontainer.json`

Add Go feature to existing features block:

```json
"features": {
    // ... existing features ...
    "ghcr.io/devcontainers/features/go:1": {
        "version": "latest"
    }
}
```

### File: `.devcontainer/scripts/post-create.sh`

Add MCP language server installation after Go verification:

```bash
# Install Go tools
if command -v go &> /dev/null; then
    print_step "Installing MCP Language Server..."
    go install github.com/isaacphi/mcp-language-server@latest || print_error "Failed to install MCP Language Server"
    print_success "MCP Language Server installed"
else
    print_error "Go not found, skipping MCP Language Server installation"
fi
```

### File: `.devcontainer/CLAUDE.md`

Update languages list to include Go.

## Implementation Order

1. Add Go feature to devcontainer.json
2. Add mcp-language-server installation to post-create.sh
3. Update CLAUDE.md documentation
4. Test container rebuild

## Verification Commands

```bash
# Verify Go installation
go version

# Verify MCP Language Server
which mcp-language-server
mcp-language-server --help

# Verify no regressions
node --version
cargo --version
claude --version
```

## Rollback Plan

If issues occur:
1. Remove Go feature from devcontainer.json
2. Remove go install section from post-create.sh
3. Rebuild container

All changes are additive and contained in configuration files.
