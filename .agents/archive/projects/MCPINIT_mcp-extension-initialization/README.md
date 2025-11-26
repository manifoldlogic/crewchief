# Project: MCP Extension Initialization

**Status**: ✅ Complete (Archived 2025-11-26)
**Slug**: `MCPINIT`
**Target Version**: v0.2.0

## Overview

Enable one-click MCP server setup from within the VSCode Maproom extension by invoking the existing CLI with proper UI integration. This eliminates the manual `npx @crewchief/maproom-mcp setup` command and provides a seamless onboarding experience.

### Problem

Users currently must:
1. Manually run `npx @crewchief/maproom-mcp setup --provider=<provider>`
2. Manually create `.vscode/mcp.json` configuration
3. Navigate cryptic error messages when services aren't running

This creates friction and confusion during onboarding.

### Solution

**Invoke the existing CLI, don't replicate it.** Wrap the proven setup command in VS Code-native UI patterns:

- Interactive setup wizard (Command Palette)
- Real-time progress notifications
- Automatic MCP server registration
- Status bar health monitoring
- Helpful error recovery actions

## Key Insights

From `planning/analysis.md`:

> **The path forward is clear: invoke the existing CLI, don't replicate it.** The "complexity" was likely from trying to manage Docker directly rather than trusting the proven setup command.

From `planning/architecture.md`:

> This architecture achieves simplicity by **delegating complexity to proven components**. The CLI already handles Docker orchestration perfectly - we just wrap it in VS Code-native UI patterns.

## Scope

### In Scope (MVP)

✅ MCP configuration writer (writes `.vscode/mcp.json`)
✅ Setup wizard integration (uses existing UI)
✅ First-activation prompt
✅ Unit tests for config generation

**What Changed**: Discovered CLI already handles Docker orchestration. Extension just writes config file.

### Out of Scope

❌ Docker orchestration (CLI handles it)
❌ Process management (VS Code MCP client handles it)
❌ Status monitoring (not needed)
❌ Container lifecycle management
❌ Custom Docker Compose configurations
❌ Remote development scenarios

## Architecture

### Core Components (Simplified)

1. **MCP Configuration Writer** (`src/config/mcp-writer.ts`) - NEW (80 lines)
2. **Setup Wizard** (`src/ui/setupWizard.ts`) - Enhanced (+50 lines)
3. **Extension Activation** (`src/extension.ts`) - Modified (+20 lines)

**Total New Code**: ~150 lines

### Data Flow (Simplified)

```
User activates extension
  ↓
No .vscode/mcp.json found
  ↓
Prompt: "Run Setup?"
  ↓
Existing Setup Wizard
  ├─ Select Provider
  └─ Enter Credentials
  ↓
NEW: MCPConfigWriter
  └─ Write .vscode/mcp.json
  ↓
Show: "Restart VS Code"
  ↓
VS Code MCP Client invokes CLI
  ↓
CLI handles Docker orchestration
```

**Key Principle**: Extension writes config, VS Code + CLI handle the rest.

## Implementation Plan

### 2 Tickets (4-6 hours estimated)

| Ticket | Description | Agent | Complexity | Time |
|--------|-------------|-------|------------|------|
| **MCPINIT-1001** | MCP Configuration Writer | vscode-extension-specialist | Low | 2-3h |
| **MCPINIT-1002** | Setup Wizard Integration | vscode-extension-specialist | Low | 2-3h |

**Total: ~150 lines of new code (78% reduction from original plan)**

### Execution Sequence

**Sequential Order** (1002 depends on 1001):

```
MCPINIT-1001: MCP Configuration Writer (~80 lines)
       ↓
MCPINIT-1002: Setup Wizard Integration (~70 lines)
       ↓
Testing & Verification
```

### Ticket Workflow

```
vscode-extension-specialist (implements)
  ↓
unit-test-runner (executes tests)
  ↓
verify-ticket (checks acceptance criteria)
  ↓
commit-ticket (creates commit)
```

## Quality Strategy

**Philosophy**: Build confidence, not coverage.

### High-Risk Areas (Must Test)
1. CLI process management (zombies, leaks)
2. MCP configuration writing (JSON corruption, overwrites)
3. Environment detection (devcontainer vs local)
4. Setup cancellation (partial state)

### Test Coverage
- Unit tests: 70% target for new code
- Integration tests: Critical paths
- Manual testing: Full UX checklist

See `planning/quality-strategy.md` for complete testing strategy.

## Security

**Critical Requirements**:
1. Credentials stored in VS Code SecretStorage (encrypted)
2. Environment variable syntax in `.vscode/mcp.json` (no plaintext keys)
3. No shell injection in process spawning
4. File operations stay within workspace
5. No credential logging

See `planning/security-review.md` for complete security analysis.

## Success Metrics

### MVP Success (Objective)
- [ ] Extension activates in <100ms
- [ ] Setup completes successfully (all 3 providers)
- [ ] No zombie processes after cancellation
- [ ] VSIX size <5MB
- [ ] All tests pass

### MVP Success (Subjective)
- [ ] First-time setup is intuitive
- [ ] Error messages are helpful
- [ ] Status bar provides useful feedback

### Post-Release (1 month)
- Setup completion rate >80%
- GitHub issues related to setup <5
- Marketplace rating maintained >4.0 stars

## Key Deliverables

### Planning Documents (Complete)
- ✅ `planning/analysis.md` - Discovery of existing infrastructure
- ✅ `planning/architecture.md` - Simplified design (no subprocess management)
- ✅ `planning/quality-strategy.md` - Focused testing approach
- ✅ `planning/security-review.md` - Simplified security model
- ✅ `planning/version-strategy.md` - Extension/MCP version alignment
- ✅ `planning/agent-suggestions.md` - 4 agents (reduced from 8)
- ✅ `planning/plan.md` - 2 tickets (reduced from 5)

### Implementation (Pending)
- ⏳ 2 tickets in `tickets/` directory (to be created)
- ⏳ ~150 lines of new code (78% reduction)
- ⏳ Unit tests for config generation
- ⏳ Integration tests for wizard flow

### Release (Pending)
- ⏳ Manual testing (3 providers)
- ⏳ Version bump (0.1.1 → 0.2.0)
- ⏳ Publish to VS Code Marketplace

**Work Reduction Summary**:
- Original: 5 tickets, 8 agents, 700+ lines of code, 1-2 days
- Revised: 2 tickets, 4 agents, 150 lines of code, 4-6 hours

## Next Steps

1. **Create Tickets**: Run `/create-project-tickets MCPINIT`
2. **Review Tickets**: Run `/review-tickets MCPINIT`
3. **Execute**: Run `/work-on-project MCPINIT`
4. **Release**: Follow release plan in `planning/plan.md`

## References

### Planning Documents
- [Analysis](planning/analysis.md) - Problem definition, research, approach
- [Architecture](planning/architecture.md) - System design, components, data flow
- [Quality Strategy](planning/quality-strategy.md) - Testing approach, risk mitigation
- [Security Review](planning/security-review.md) - Threat model, security requirements
- [Agent Suggestions](planning/agent-suggestions.md) - Required agents and workflow
- [Execution Plan](planning/plan.md) - Timeline, milestones, release plan

### Key Decisions

**Version Strategy**: Extension pins exact MCP version (`@crewchief/maproom-mcp@2.2.1`) to guarantee schema compatibility. See `planning/version-strategy.md` for sync mechanisms and release coordination.

### Related Components
- `packages/vscode-maproom/` - VSCode extension source
- `packages/maproom-mcp/bin/cli.cjs` - CLI being invoked
- `packages/maproom-mcp/config/docker-compose.yml` - Docker orchestration

### Research Sources
- [Use MCP servers in VS Code](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)
- [MCP developer guide | VS Code Extension API](https://code.visualstudio.com/api/extension-guides/ai/mcp)
- [Installing MCP Servers via VS Code Extensions](https://eclipsesource.com/blogs/2025/06/12/installing-mcp-servers-via-vscode-extensions/)

## Project Metadata

**Created**: 2025-01-23
**Sprint**: Q1 2025
**Priority**: High (eliminates #1 onboarding friction)
**Risk Level**: Low (reusing proven CLI)
**Estimated Effort**: 1-2 days focused work

**Status Transitions**:
- 2025-01-23: Planning phase started
- 2025-01-23: Planning phase completed
- TBD: Implementation started
- TBD: Testing completed
- TBD: Released to marketplace

---

*This project follows the CrewChief ticket-based workflow. See `.agents/README.md` for workflow details.*
