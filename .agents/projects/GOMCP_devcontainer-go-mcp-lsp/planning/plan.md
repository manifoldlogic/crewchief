# Plan: Devcontainer Go and MCP Language Server

## Objective

Add Go language support and MCP Language Server to the devcontainer initialization.

## Phases

### Phase 1: Add Go Support

Add the Go devcontainer feature to install Go compiler during container creation.

**Files Modified**:
- `.devcontainer/devcontainer.json` - Add Go feature

**Agent**: docker-engineer

**Tickets**:
- GOMCP-1001: Add Go devcontainer feature

### Phase 2: Install MCP Language Server

Add MCP Language Server installation to post-create script.

**Files Modified**:
- `.devcontainer/scripts/post-create.sh` - Add go install command

**Agent**: docker-engineer

**Tickets**:
- GOMCP-2001: Install MCP Language Server in post-create

### Phase 3: Documentation Update

Update devcontainer documentation to reflect new language support.

**Files Modified**:
- `.devcontainer/CLAUDE.md` - Update languages list

**Agent**: docker-engineer

**Tickets**:
- GOMCP-3001: Update devcontainer documentation

## Success Metrics

| Metric | Before | After |
|--------|--------|-------|
| Go available | No | Yes |
| MCP LSP available | No | Yes |
| Container builds | Yes | Yes |
| Existing tools work | Yes | Yes |

## Dependencies

- No external dependencies beyond GitHub network access during build
- All phases are sequential (Go must be available before MCP LSP)

## Risks

1. **Container build time increase**
   - Probability: Certain
   - Impact: Low (+30-60 seconds)
   - Mitigation: Acceptable tradeoff for functionality

2. **MCP Language Server installation failure**
   - Probability: Low
   - Impact: Low (non-fatal, container still usable)
   - Mitigation: Uses || print_error pattern, can retry manually

3. **Go feature version issues**
   - Probability: Very Low
   - Impact: Low
   - Mitigation: Using official Microsoft feature with "latest" version

## Verification

```bash
# After container rebuild
go version                    # Go installed
which mcp-language-server     # MCP LSP installed
node --version               # Node.js still works
cargo --version              # Rust still works
```

## Ticket Summary

| Ticket | Description | Agent |
|--------|-------------|-------|
| GOMCP-1001 | Add Go devcontainer feature | docker-engineer |
| GOMCP-2001 | Install MCP Language Server in post-create | docker-engineer |
| GOMCP-3001 | Update devcontainer documentation | docker-engineer |

Total: 3 tickets, 1 phase each
