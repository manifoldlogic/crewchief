# Ticket: GOMCP-2001: Install MCP Language Server in Post-Create

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (configuration change, verified by container rebuild)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Add MCP Language Server installation to the post-create script using `go install`.

## Background
With Go available from GOMCP-1001, we can now install the MCP Language Server which provides LSP support for Model Context Protocol development. This follows the same pattern used for other tool installations (Claude Code, Husky, CrewChief CLI).

Reference: plan.md Phase 2 - Install MCP Language Server

## Acceptance Criteria
- [ ] Go installation check added before `go install` command
- [ ] `go install github.com/isaacphi/mcp-language-server@latest` command added
- [ ] Uses existing `print_step`, `print_success`, `print_error` functions
- [ ] Non-fatal error handling (uses `|| print_error` pattern)
- [ ] Shell script syntax is valid (`bash -n` passes)

## Technical Requirements
- Check `command -v go` before attempting installation
- Use `@latest` version tag
- Follow existing post-create.sh patterns for output and error handling
- Installation is non-fatal (container remains usable if it fails)

## Implementation Notes
Add to `.devcontainer/scripts/post-create.sh` after existing tool installations:

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

Binary will be installed to: `$HOME/go/bin/mcp-language-server`

## Dependencies
- GOMCP-1001: Add Go devcontainer feature (Go must be available)

## Risk Assessment
- **Risk**: Network issues could cause `go install` to fail
  - **Mitigation**: Non-fatal error handling, can retry manually with same command
- **Risk**: GOPATH/bin not in PATH
  - **Mitigation**: Devcontainer Go feature handles PATH setup automatically

## Files/Packages Affected
- `.devcontainer/scripts/post-create.sh`
