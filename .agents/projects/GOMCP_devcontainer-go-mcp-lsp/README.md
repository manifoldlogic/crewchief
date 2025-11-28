# GOMCP: Devcontainer Go and MCP Language Server

## Status: Planning Complete

## Problem

The devcontainer lacks Go language support, preventing installation of Go-based tools like the MCP Language Server (`github.com/isaacphi/mcp-language-server`).

## Solution

1. Add Go devcontainer feature for language runtime
2. Install MCP Language Server via `go install` during initialization
3. Update documentation

## Scope

- 3 tickets, 3 phases
- Files modified: `devcontainer.json`, `post-create.sh`, `CLAUDE.md`
- No new dependencies beyond standard Go tooling

## Agents

- **docker-engineer**: All implementation work (devcontainer configuration)

## Planning Documents

- [Analysis](planning/analysis.md) - Problem breakdown and integration points
- [Architecture](planning/architecture.md) - Implementation approach and decisions
- [Quality Strategy](planning/quality-strategy.md) - Verification checklist
- [Security Review](planning/security-review.md) - Security assessment (Low risk)
- [Plan](planning/plan.md) - 3 phases, 3 tickets

## Tickets

See `tickets/` for implementation tickets (created via `/create-project-tickets`).

## Quick Reference

```bash
# After implementation, verify:
go version                    # Go installed
which mcp-language-server     # MCP LSP installed
mcp-language-server --help    # MCP LSP runs
```
