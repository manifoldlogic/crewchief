# MCPINIT Ticket Index

**Project**: MCP Extension Initialization (MCPINIT)
**Target Version**: 0.2.0
**Total Tickets**: 2
**Total Estimated Time**: 4-6 hours

## Overview

This project enables one-click MCP server setup from within the VSCode Maproom extension by writing `.vscode/mcp.json` configuration. The extension delegates all Docker orchestration to the existing CLI (`@crewchief/maproom-mcp@2.2.1`), following the MCP best practice pattern: **register the server, don't manage it**.

**Key Simplification**: Originally planned 5 tickets (700+ lines). After discovering the CLI already handles ALL infrastructure, reduced to 2 tickets (150 lines) - an 80% reduction in scope.

## Project Status

- **Phase 1 - Foundation**: 2 tickets (Ready for implementation)
- **Planning Status**: ✅ Complete
- **Review Status**: ✅ Reviewed and updated
- **Ready for Execution**: ✅ Yes

## Phase 1: Foundation (Tickets 1-2)

**Goal**: Implement core MCP registration functionality

### MCPINIT-1001: MCP Configuration Writer ⏳
- **Status**: Not Started
- **Complexity**: Low
- **Estimated Time**: 2-3 hours
- **Dependencies**: None
- **Agent**: vscode-extension-specialist
- **Files Created**:
  - `src/config/mcp-writer.ts` (~80 lines)
  - `src/constants.ts` (~10 lines)
  - `src/config/mcp-writer.test.ts` (~150 lines)

**Summary**: Create utility class to write `.vscode/mcp.json` with Maproom MCP server configuration. Supports all three providers (OpenAI, Google, Ollama), merges with existing MCP servers, uses environment variable syntax for credentials, validates workspace paths.

**Key Deliverables**:
- `MCPConfigWriter` class with `registerMCPServer()` method
- Version constant: `MAPROOM_MCP_VERSION = '2.2.1'`
- Unit tests for config generation and merging
- Security: Path validation, no plaintext credentials

**Plan Reference**: [plan.md lines 82-148](../planning/plan.md#L82-L148)

---

### MCPINIT-1002: Setup Wizard Integration ⏳
- **Status**: Not Started (Blocked by MCPINIT-1001)
- **Complexity**: Low
- **Estimated Time**: 2-3 hours
- **Dependencies**: MCPINIT-1001 (CRITICAL)
- **Agent**: vscode-extension-specialist
- **Files Modified**:
  - `src/ui/setupWizard.ts` (+50 lines)
  - `src/extension.ts` (+20 lines)

**Summary**: Enhance existing setup wizard to call `MCPConfigWriter` after provider selection. Implement first-activation prompt that guides new users to run setup when `.vscode/mcp.json` is missing.

**Key Deliverables**:
- Wizard calls config writer after credential collection
- "Restart Now" prompt after successful configuration
- First-activation prompt (shows once per workspace)
- User-friendly error messages
- Manual testing with all 3 providers

**Plan Reference**: [plan.md lines 152-243](../planning/plan.md#L152-L243)

---

## Workflow Sequence

```
MCPINIT-1001: MCP Configuration Writer
       ↓ (required dependency)
MCPINIT-1002: Setup Wizard Integration
       ↓
unit-test-runner → verify-ticket → commit-ticket
```

**Execution Order**: Sequential (1002 depends on 1001)

**Parallel Opportunities**: None

## Ticket Workflow

Each ticket follows this workflow:

1. **Implementation**: `vscode-extension-specialist` completes the work
2. **Testing**: `unit-test-runner` executes test suite
3. **Verification**: `verify-ticket` checks acceptance criteria
4. **Commit**: `commit-ticket` creates Conventional Commit

If tests fail or verification fails, return to implementation agent for fixes.

## Project-Level Acceptance Criteria

Before marking project complete:

- [ ] All 2 tickets completed and committed
- [ ] Extension activates in <100ms (performance benchmark)
- [ ] Setup completes successfully with all 3 providers (manual test)
- [ ] `.vscode/mcp.json` written correctly (integration test)
- [ ] No zombie processes after operation (process check)
- [ ] VSIX size <5MB (build verification)
- [ ] All unit tests passing (CI)
- [ ] Documentation updated (README)

## Agent Assignments

### Primary Agent: vscode-extension-specialist
- Implements both tickets (~90% of work)
- Writes unit and integration tests
- Handles VSCode API integration

### Supporting Agents (via workflow)
- **unit-test-runner**: Test execution and reporting
- **verify-ticket**: Acceptance criteria verification
- **commit-ticket**: Git operations

**Why So Few Agents?** No subprocess management, no Docker orchestration, no status monitoring. Just file operations and UI integration.

## Key Decisions

### 1. Version Pinning Strategy
**Decision**: Use exact version `@crewchief/maproom-mcp@2.2.1` instead of `@latest`
**Rationale**: Guarantees schema compatibility between extension and MCP server
**Reference**: [version-strategy.md](../planning/version-strategy.md)

### 2. Architectural Pattern
**Decision**: Extension writes config, VS Code invokes CLI, CLI manages lifecycle
**Rationale**: Follows MCP best practices (like language server protocol)
**Reference**: [architecture.md](../planning/architecture.md)

### 3. Scope Reduction
**Decision**: Eliminate process management, status monitoring, Docker orchestration
**Rationale**: CLI already does this (1,972 lines). Don't duplicate.
**Impact**: Reduced from 5 tickets (700+ lines) to 2 tickets (150 lines)
**Reference**: [project-review.md](../planning/project-review.md)

## Risk Mitigation

### High-Risk Areas
1. **Path Traversal**: Write to wrong location
   - **Mitigation**: Path validation with `path.resolve()`
   - **Test**: Unit test for `../../etc/passwd` attack

2. **Config Overwrite**: Lose user's existing MCP servers
   - **Mitigation**: Always read + merge existing config
   - **Test**: Unit test verifies preservation

3. **Credential Exposure**: Plaintext API keys in config
   - **Mitigation**: Use `${env:VAR_NAME}` syntax
   - **Test**: Unit test verifies no actual secrets

### Medium-Risk Areas
4. **Version Skew**: Extension and CLI incompatible
   - **Mitigation**: Pin exact version in constant
   - **Test**: Integration test with pinned version

5. **Wizard UX**: Confusing error messages
   - **Mitigation**: User-friendly messages, clear next steps
   - **Test**: Manual UX testing

## Testing Strategy

### Unit Tests (Automated)
- Config generation for each provider
- Config merging preserves existing servers
- Path validation rejects traversal
- First-activation detection logic
- Wizard integration calls config writer

**Coverage Target**: 70% of new code

### Integration Tests (Automated)
- Full wizard flow writes `.vscode/mcp.json`
- Config file format is valid JSON
- Temp directory writes work correctly

### Manual Tests (Pre-Release)
- Setup with OpenAI provider → verify config → restart works
- Setup with Google provider → verify config → restart works
- Setup with Ollama provider → verify config → restart works
- First activation without config → prompt appears
- Delete config → prompt reappears
- "Remind Me Later" → prompt doesn't reappear immediately

**Full Checklist**: [quality-strategy.md](../planning/quality-strategy.md)

## Success Metrics

### MVP Success (Objective)
- [ ] Extension activates in <100ms
- [ ] Setup completes successfully (all 3 providers)
- [ ] No zombie processes after operation
- [ ] VSIX size <5MB
- [ ] All tests pass

### MVP Success (Subjective)
- [ ] First-time setup is intuitive
- [ ] Error messages are helpful
- [ ] Status feedback is clear

### Post-Release (1 month)
- Setup completion rate >80%
- GitHub issues related to setup <5
- Marketplace rating maintained >4.0 stars

## Documentation References

### Planning Documents
- [Analysis](../planning/analysis.md) - Problem definition, existing infrastructure discovery
- [Architecture](../planning/architecture.md) - System design, delegation pattern
- [Plan](../planning/plan.md) - Detailed ticket specifications
- [Quality Strategy](../planning/quality-strategy.md) - Testing approach, manual checklist
- [Security Review](../planning/security-review.md) - Security requirements, threat model
- [Version Strategy](../planning/version-strategy.md) - Version pinning, sync mechanisms
- [Agent Suggestions](../planning/agent-suggestions.md) - Required agents, workflow

### External References
- [VS Code MCP Documentation](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)
- [MCP Extension API Guide](https://code.visualstudio.com/api/extension-guides/ai/mcp)
- [Installing MCP Servers via Extensions](https://eclipsesource.com/blogs/2025/06/12/installing-mcp-servers-via-vscode-extensions/)

## Timeline

**Estimated Total Duration**: 4-6 hours (focused work)

**Breakdown**:
- MCPINIT-1001: 2-3 hours
- MCPINIT-1002: 2-3 hours
- Manual testing: Included in 1002
- Release activities: Separate (not part of ticket execution)

**Original Estimate**: 1-2 days (8-16 hours)
**Reduction**: 60% faster due to scope simplification

## Next Steps

1. **Execute Tickets**: Run `/work-on-project MCPINIT`
2. **Verify Quality**: Run `/review-tickets MCPINIT` (optional, for sanity check)
3. **Manual Testing**: Complete checklist from quality-strategy.md
4. **Release**: Follow release plan in [plan.md](../planning/plan.md)

## Change History

- **2025-01-23**: Planning phase completed
- **2025-01-23**: Project review identified 85% duplication, scope reduced
- **2025-01-23**: All planning documents updated to reflect simplified approach
- **2025-01-23**: Tickets created (2 tickets, down from 5)

---

**Work Reduction Summary**:
- Original: 5 tickets, 8 agents, 700+ lines of code, 1-2 days
- Revised: 2 tickets, 4 agents, 150 lines of code, 4-6 hours
- **Improvement**: 80% reduction in scope, 60% reduction in time

**Key Insight**: The path forward is clear - invoke the existing CLI, don't replicate it. This project exemplifies the "Reuse Over Rebuild" principle.
