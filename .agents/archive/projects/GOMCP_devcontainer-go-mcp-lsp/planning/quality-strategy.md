# Quality Strategy: Devcontainer Go and MCP Language Server

## Testing Approach

This is a devcontainer configuration change with no unit tests applicable. Quality is verified through manual validation after container rebuild.

## Verification Checklist

### Pre-commit Verification

1. **Syntax Validation**
   - `devcontainer.json` is valid JSON
   - `post-create.sh` has no syntax errors

2. **Local Validation** (if possible)
   - JSON linting passes
   - Shell script syntax check: `bash -n post-create.sh`

### Post-rebuild Verification

Run these commands after container rebuild:

```bash
# 1. Go Installation
go version
# Expected: go version go1.x.x linux/amd64

# 2. Go PATH
which go
# Expected: /usr/local/go/bin/go

# 3. MCP Language Server Installation
which mcp-language-server
# Expected: /home/vscode/go/bin/mcp-language-server

# 4. MCP Language Server Execution
mcp-language-server --help 2>&1 | head -5
# Expected: Usage information (not "command not found")

# 5. Regression: Node.js
node --version
# Expected: v20.x.x

# 6. Regression: Rust
cargo --version
# Expected: cargo 1.x.x

# 7. Regression: Claude Code
claude --version 2>/dev/null || echo "claude available"
# Expected: Version or availability confirmation
```

### Integration Points

| Component | Verification | Expected Result |
|-----------|-------------|-----------------|
| Go compiler | `go version` | Version string |
| Go PATH | `echo $PATH \| grep -o '/usr/local/go'` | Path present |
| MCP LSP binary | `which mcp-language-server` | Path to binary |
| MCP LSP execution | `mcp-language-server --help` | Usage output |
| Existing Node | `node --version` | v20.x |
| Existing Rust | `cargo --version` | 1.x |

## Risk Mitigation

### Build Time Impact

**Risk**: Adding Go feature increases container build time.

**Mitigation**:
- Go feature uses binary download, not compilation
- Typical impact: +30-60 seconds to initial build
- Acceptable for the functionality gained

### PATH Conflicts

**Risk**: Go PATH not properly configured.

**Mitigation**:
- Using official devcontainer feature handles PATH automatically
- Feature adds to both `.bashrc` and `.zshrc`

### Installation Failures

**Risk**: `go install` fails (network, version incompatibility).

**Mitigation**:
- Using `|| print_error` pattern (non-fatal)
- Container still usable even if MCP LSP fails to install
- Can manually retry: `go install github.com/isaacphi/mcp-language-server@latest`

## Acceptance Criteria

- [ ] Container rebuilds successfully
- [ ] `go version` returns valid Go version
- [ ] `mcp-language-server` is accessible in PATH
- [ ] Existing Node.js, Rust, Claude Code still function
- [ ] No errors in post-create.sh output
