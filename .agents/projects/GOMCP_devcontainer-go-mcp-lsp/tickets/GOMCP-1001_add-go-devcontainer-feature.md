# Ticket: GOMCP-1001: Add Go Devcontainer Feature

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (configuration change, verified by container rebuild)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- verify-ticket
- commit-ticket

## Summary
Add the official Go devcontainer feature to install Go compiler during container creation.

## Background
The devcontainer needs Go language support to enable installation of Go-based tools like the MCP Language Server. This ticket adds the Go runtime using the official Microsoft devcontainer feature, following the same pattern used for Node.js and Rust.

Reference: plan.md Phase 1 - Add Go Support

## Acceptance Criteria
- [ ] Go feature added to `.devcontainer/devcontainer.json` features block
- [ ] Feature configured with `version: "latest"`
- [ ] JSON syntax is valid (no parsing errors)
- [ ] Pattern matches existing features (Node, Rust)

## Technical Requirements
- Use feature: `ghcr.io/devcontainers/features/go:1`
- Set version to "latest" for consistency with other language features
- Add after existing features in the features block

## Implementation Notes
Add to `.devcontainer/devcontainer.json` in the features section:

```json
"ghcr.io/devcontainers/features/go:1": {
    "version": "latest"
}
```

The Go feature automatically:
- Installs Go to `/usr/local/go`
- Adds Go to PATH in `.bashrc` and `.zshrc`
- Sets up GOPATH at `$HOME/go`

## Dependencies
- None - this is the first ticket in the project

## Risk Assessment
- **Risk**: Feature may increase container build time
  - **Mitigation**: Acceptable tradeoff (~30-60 seconds), uses binary download not compilation

## Files/Packages Affected
- `.devcontainer/devcontainer.json`
