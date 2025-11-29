# Analysis: Devcontainer Go and MCP Language Server

## Problem Statement

The devcontainer needs Go language support and the MCP Language Server (`github.com/isaacphi/mcp-language-server`) installed during initialization to enable Go language features for MCP server development and LSP support.

## Current State

### Existing Devcontainer Setup

The devcontainer currently includes:
- **Languages**: Node.js 20, Rust, Python (via base image)
- **Features**: Git, GitHub CLI, Docker-in-Docker, common-utils
- **Missing**: Go language support

### Initialization Flow

```
devcontainer.json
  ├── features (language runtimes installed here)
  │   ├── common-utils:2
  │   ├── git:1
  │   ├── node:1 (v20)
  │   ├── rust:1
  │   └── docker-in-docker:2
  └── postCreateCommand → scripts/post-create.sh
      ├── Claude Code installation
      ├── Husky installation
      ├── CrewChief CLI installation
      ├── pnpm dependencies
      ├── Maproom binary build
      └── Git/environment configuration
```

### MCP Language Server

The `mcp-language-server` is a Go-based tool that provides Language Server Protocol (LSP) support for MCP (Model Context Protocol). Installing it requires:
1. Go compiler installed
2. `go install github.com/isaacphi/mcp-language-server@latest`

## Research Findings

### Go Installation Options

1. **Devcontainer Features (Recommended)**
   - `ghcr.io/devcontainers/features/go:1`
   - Handles PATH setup automatically
   - Installs at `/usr/local/go`
   - Adds GOPATH to user's environment

2. **Dockerfile Installation**
   - Download from golang.org
   - Manual PATH configuration required
   - More control but more maintenance

3. **Post-create Script**
   - Using `apt-get install golang-go`
   - Older version, not recommended for modern Go tools

### MCP Language Server Installation

Must be done in `post-create.sh` after Go is available:
- Uses `go install` which requires `$GOPATH/bin` in PATH
- Default GOPATH: `$HOME/go`
- Binary installed to: `$HOME/go/bin/mcp-language-server`

## Integration Points

### devcontainer.json
Add Go feature to existing features list.

### post-create.sh
Add `go install` command after Go is confirmed available.

## Success Criteria

1. Go compiler available via `go version`
2. `mcp-language-server` binary accessible in PATH
3. No regressions to existing devcontainer functionality
4. Container rebuild succeeds cleanly
