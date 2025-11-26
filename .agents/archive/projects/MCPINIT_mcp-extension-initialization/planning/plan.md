# Execution Plan: MCP Extension Initialization

## Project Goal

Enable one-click MCP server setup from within the VSCode Maproom extension by invoking the existing CLI with proper UI integration, eliminating the need for manual `npx @crewchief/maproom-mcp setup` commands.

## Success Criteria

**Must Have (MVP)**:
1. ✅ Setup wizard accessible via command palette
2. ✅ Progress notification shows setup status
3. ✅ MCP server registered in `.vscode/mcp.json`
4. ✅ Status bar indicates service health
5. ✅ Error messages guide users to resolution
6. ✅ No zombie processes or orphaned containers

**Won't Have (Out of Scope)**:
- Automatic Docker installation
- Remote development scenarios (SSH, WSL)
- Alternative database providers

## Timeline and Milestones

### Phase 1: Implementation (Tickets 1-2)

**Duration**: 4-6 hours

**Deliverables**:
- MCP configuration writer (MCPINIT-1001)
- Setup wizard integration (MCPINIT-1002)
- Unit and integration tests
- Manual testing with all 3 providers

**Success Metric**: End-to-end setup flow works from command palette, `.vscode/mcp.json` written correctly

---

### Release Phase

**Duration**: ~2-3 hours

**Activities**:
- Manual testing checklist (completed during MCPINIT-1002)
- Security review (verify path validation, credential handling)
- Version bump (0.1.1 → 0.2.0)
- Build VSIX (<5MB size check)
- Publish to marketplace

**Success Metric**: Extension published with setup feature

## Ticket Breakdown

### MCPINIT-1001: MCP Configuration Writer

**Description**: Create utility to write `.vscode/mcp.json` with Maproom MCP server configuration

**Agent**: `vscode-extension-specialist`

**Files Created**:
- `src/config/mcp-writer.ts` (80 lines)

**Files Modified**:
- None (new component)

**Acceptance Criteria**:
- [ ] Writes `.vscode/mcp.json` to workspace root
- [ ] Preserves existing MCP servers (merges, doesn't overwrite)
- [ ] Uses `${env:OPENAI_API_KEY}` syntax for environment variables
- [ ] Handles all three providers (openai, google, ollama)
- [ ] Creates `.vscode/` directory if missing
- [ ] Validates path is within workspace (no path traversal)
- [ ] Returns error if no workspace folder open

**Tests Required**:
- Unit: Config generation for each provider
- Unit: Merging with existing MCP servers
- Integration: Writes to temp directory
- Integration: Path validation

**Implementation Example**:
```typescript
// src/config/mcp-writer.ts
import { MAPROOM_MCP_VERSION } from '../constants'

export class MCPConfigWriter {
  async registerMCPServer(workspaceRoot: string, provider: string): Promise<void> {
    const configPath = path.join(workspaceRoot, '.vscode', 'mcp.json')

    // Read existing config or create new
    let config: MCPConfig = { mcpServers: {} }
    if (fs.existsSync(configPath)) {
      config = JSON.parse(fs.readFileSync(configPath, 'utf-8'))
    }

    // Add/update maproom server
    config.mcpServers.maproom = {
      command: 'npx',
      args: ['-y', `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`],
      env: this.buildEnvironment(provider)
    }

    // Write back
    await fs.promises.mkdir(path.dirname(configPath), { recursive: true })
    await fs.promises.writeFile(configPath, JSON.stringify(config, null, 2))
  }

  private buildEnvironment(provider: string): Record<string, string> {
    switch (provider) {
      case 'openai': return { OPENAI_API_KEY: '${env:OPENAI_API_KEY}' }
      case 'google': return { GOOGLE_APPLICATION_CREDENTIALS: '${env:GOOGLE_APPLICATION_CREDENTIALS}' }
      case 'ollama': return {}
    }
  }
}
```

**Estimated Complexity**: Low
**Estimated Time**: 2-3 hours
**Dependencies**: None

---

### MCPINIT-1002: Setup Wizard Integration

**Description**: Enhance existing setup wizard to write MCP config after provider selection

**Agent**: `vscode-extension-specialist`

**Files Modified**:
- `src/ui/setupWizard.ts` (+50 lines)
- `src/extension.ts` (+20 lines)

**Acceptance Criteria**:
- [ ] After provider selection in existing wizard, calls MCPConfigWriter
- [ ] Shows success message: "MCP server configured. Restart VS Code to activate."
- [ ] Command `maproom.setup` available in command palette
- [ ] On first activation, prompts: "Run Setup" or "Remind Me Later"
- [ ] Handles errors gracefully (e.g., no workspace open)
- [ ] Writes user-friendly error messages

**Tests Required**:
- Unit: Wizard flow calls config writer with correct provider
- Integration: Full wizard flow (mocked user input)
- Manual: Test with each provider in real VS Code

**Implementation Approach**:
```typescript
// src/ui/setupWizard.ts (enhance existing)
import { MCPConfigWriter } from '../config/mcp-writer'

export async function runSetupWizard(
  context: vscode.ExtensionContext
): Promise<EmbeddingProvider | undefined> {
  // ... existing provider selection code ...

  const provider = await showProviderPicker()
  if (!provider) return undefined

  // ... existing credential collection ...

  // NEW: Write MCP config
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) {
    vscode.window.showErrorMessage('No workspace folder open. Open a folder to configure Maproom.')
    return undefined
  }

  const writer = new MCPConfigWriter()
  await writer.registerMCPServer(workspaceRoot, provider)

  vscode.window.showInformationMessage(
    'Maproom MCP server configured! Restart VS Code to activate.',
    'Restart Now'
  ).then(action => {
    if (action === 'Restart Now') {
      vscode.commands.executeCommand('workbench.action.reloadWindow')
    }
  })

  return provider
}
```

```typescript
// src/extension.ts (add first-activation prompt)
export async function activate(context: vscode.ExtensionContext) {
  // Register commands
  registerSetupCommand(context)

  // Check if MCP config exists
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) return // No workspace, skip setup prompt

  const mcpConfigPath = path.join(workspaceRoot, '.vscode', 'mcp.json')
  const configExists = fs.existsSync(mcpConfigPath)

  if (!configExists) {
    // First time - prompt for setup
    const action = await vscode.window.showInformationMessage(
      'Maproom MCP server not configured. Run setup?',
      'Run Setup',
      'Remind Me Later'
    )

    if (action === 'Run Setup') {
      await vscode.commands.executeCommand('maproom.setup')
    }
  }
}
```

**Estimated Complexity**: Low
**Estimated Time**: 2-3 hours
**Dependencies**: MCPINIT-1001 (MCPConfigWriter must exist)

---

## Work Sequence

### Sequential Order

```
MCPINIT-1001: MCP Configuration Writer
       ↓ (required dependency)
MCPINIT-1002: Setup Wizard Integration
       ↓
unit-test-runner → verify-ticket → commit-ticket
```

**Total Duration**: 4-6 hours (vs 1-2 days originally planned)

**Parallel Opportunities**: None (1002 depends on 1001)

**Why This Is Simple**:
- No Docker orchestration (CLI handles it)
- No process management (CLI is self-contained)
- No health checking (CLI validates services)
- Just configuration: write JSON file, enhance UI

## Risk Mitigation

### Risk 1: CLI Output Format Changes

**Impact**: Progress parsing breaks, users see generic "Setting up..." message

**Mitigation**:
- Parse defensively with regex and fallbacks
- Test with multiple CLI versions
- Version check CLI before invoking

**Contingency**: Show generic progress if parsing fails, doesn't block setup

---

### Risk 2: Docker Not Installed

**Impact**: Setup fails with cryptic error

**Mitigation**:
- Check `docker` command availability before spawning CLI
- Show actionable error: "Docker not found. Install Docker Desktop: https://..."
- Include in manual testing checklist

**Contingency**: Clear error message guides user to install Docker

---

### Risk 3: Process Doesn't Terminate

**Impact**: Zombie processes consume resources

**Mitigation**:
- Implement cancellation handler with `process.kill()`
- Set timeout on process execution (10 minutes max)
- Test cancellation scenarios manually

**Contingency**: User can kill via task manager, extension restart cleans up

---

### Risk 4: Configuration Overwrite

**Impact**: User loses existing MCP server configurations

**Mitigation**:
- Always read existing config before writing
- Unit test: preserve other MCP servers
- Integration test: merge scenario

**Contingency**: User can restore from Git (workspace config is versioned)

---

### Risk 5: Activation Slowdown

**Impact**: Extension activation takes >100ms, poor user experience

**Mitigation**:
- Profile activation time with `Date.now()` measurement
- Defer all heavy operations (setup check, status monitoring)
- Start monitoring asynchronously after activation completes

**Contingency**: Optimize by removing blocking operations from activate()

## Testing Strategy

### Per Ticket

**Unit Tests**: Written alongside implementation
- Focus on logic and edge cases
- Mock external dependencies
- Fast execution (<1s total)

**Integration Tests**: After implementation
- Test component interactions
- Use temp directories
- Mock expensive operations (Docker)

**Manual Testing**: Before verification
- Test in real extension environment
- Verify UI/UX flows
- Check error scenarios

### Pre-Release

**Manual Testing Checklist** (from quality-strategy.md):
- [ ] First-time setup (each provider)
- [ ] Setup without Docker
- [ ] Setup cancellation
- [ ] Error recovery
- [ ] Configuration persistence
- [ ] Multi-workspace behavior

**Regression Testing**:
- [ ] Existing features still work (scan, watch, search)
- [ ] Database connectivity unchanged
- [ ] Extension activates successfully

**Security Review**:
- [ ] No credentials in logs
- [ ] SecretStorage used correctly
- [ ] No command injection vectors
- [ ] File operations stay within workspace

## Release Plan

### Version Bump

**Current**: 0.1.1
**Target**: 0.2.0 (minor version bump for new feature)

**Rationale**: Setup wizard is significant user-facing feature, warrants minor version

### Changelog

```markdown
## [0.2.0] - 2025-01-XX

### Added
- Setup wizard for one-click MCP server initialization
- Status bar health monitoring
- Automatic MCP server registration
- Error recovery commands

### Changed
- Extension activation now non-blocking
- Prompts for setup on first use if services not available

### Fixed
- N/A (no fixes in this release)
```

### Release Steps

1. **Complete All Tickets** (via `/work-on-project MCPINIT`)
   - Both tickets (MCPINIT-1001, MCPINIT-1002) implemented, tested, verified, committed

2. **Manual Testing** (1 hour)
   - Run through manual testing checklist
   - Test on clean machine if possible
   - Verify each provider (OpenAI, Ollama)

3. **Security Review** (30 minutes)
   - Review credential handling
   - Check for logged secrets
   - Verify SecretStorage usage

4. **Documentation Update** (30 minutes)
   - Update README with setup instructions
   - Add screenshots of setup wizard
   - Document configuration options

5. **Version Bump** (5 minutes)
   - Update `package.json` version to 0.2.0
   - Update CHANGELOG.md
   - Commit: `chore(release): bump version to 0.2.0`

6. **Build and Package** (5 minutes)
   - Run `pnpm build`
   - Run `vsce package --no-dependencies`
   - Verify VSIX size (<5MB)

7. **Publish** (5 minutes)
   - Trigger release workflow
   - Wait for marketplace update
   - Verify extension page shows new version

8. **Verify in Marketplace** (10 minutes)
   - Install from marketplace
   - Test setup flow
   - Verify no issues

**Total Release Time**: ~2.5 hours

### Rollback Plan

If critical issue discovered:

1. **Unpublish**: Remove from marketplace (via publisher portal)
2. **Fix**: Create hotfix branch
3. **Test**: Run full testing checklist
4. **Republish**: Version 0.2.1 with fix

Alternatively, revert to 0.1.1 if fix not immediate.

## Communication Plan

### During Development

**Internal**:
- Ticket progress tracked in todo list
- Commits follow Conventional Commit format
- Each ticket verified before commit

### Release Announcement

**Channels**:
- GitHub Release notes
- VS Code Marketplace description update
- README.md with prominent setup section

**Message**:
```
🎉 Maproom 0.2.0 brings one-click setup!

No more manual CLI commands - just run "Maproom: Setup" from the command palette and you're ready to go. The extension now handles Docker orchestration, MCP registration, and service monitoring automatically.

New in this release:
- Interactive setup wizard
- Real-time status monitoring
- Automatic MCP server registration
- Helpful error recovery

Get started in minutes instead of fiddling with configuration files!
```

## Maintenance Plan

### Post-Release Monitoring

**First Week**:
- Monitor GitHub issues for bug reports
- Check marketplace reviews
- Respond to questions within 24 hours

**First Month**:
- Gather usage feedback
- Identify common pain points
- Evaluate potential enhancements based on real usage

### Potential Future Enhancements

**Note**: These features are intentionally out of scope for MVP. Evaluate based on user feedback after release.

**Container Lifecycle Management**:
- Start/stop/restart individual services
- View container logs in Output channel
- Restart on config changes

**Advanced Configuration**:
- Custom ports and hostnames
- Multiple embedding providers
- Resource limits (memory, CPU)

**Remote Development**:
- Detect SSH/WSL/Remote-Containers
- Forward ports automatically
- Handle Docker-in-Docker scenarios

**Priority**: Ship simple version first, iterate based on real user needs.

## Success Metrics

### MVP Success

**Objective Metrics**:
- [ ] Extension activates in <100ms (measured)
- [ ] Setup completes successfully (all 3 providers tested)
- [ ] No zombie processes after cancellation (verified)
- [ ] VSIX size <5MB (measured)
- [ ] All tests pass (unit + integration)

**Subjective Metrics**:
- [ ] First-time setup experience is intuitive (manual testing)
- [ ] Error messages are helpful (manual testing)
- [ ] Status bar provides useful feedback (manual testing)

### Post-Release Success (1 month)

**Usage Metrics** (if we add telemetry):
- Setup completion rate >80%
- Average setup time <5 minutes
- Cancellation rate <10%

**Quality Metrics**:
- GitHub issues related to setup <5
- Marketplace rating maintained >4.0 stars
- No critical security issues reported

## Conclusion

This execution plan breaks the MCP initialization feature into **2 focused tickets** for MVP:

1. **MCPINIT-1001**: MCP Configuration Writer (~80 lines)
2. **MCPINIT-1002**: Setup Wizard Integration (~70 lines)

**Key Simplification**: After discovering the CLI already handles Docker orchestration (1,972 lines), we eliminated unnecessary duplication. The extension's only job is to write `.vscode/mcp.json` - VS Code's MCP client and the CLI handle everything else.

By following the ticket workflow (implement → test → verify → commit) and leveraging specialized agents, we ensure quality at every step.

The MVP focuses on the essential user experience: one-click setup via command palette with automatic MCP server registration. Advanced features (container management, custom configs, remote dev) are intentionally deferred for future consideration based on real user needs.

**Total Effort**: 2 tickets + release = estimated 4-6 hours of focused work (down from original 1-2 days)

**Risk Level**: Low (reusing proven CLI, minimal new code, comprehensive testing)

**User Value**: High (eliminates #1 onboarding friction point)

This project exemplifies the "Reuse Over Rebuild" principle: we invoke the battle-tested CLI and wrap it in VS Code-native UI patterns, achieving simplicity through delegation rather than duplication.
