# Ticket: MCPINIT-1002: Integrate MCP Configuration Writer into Setup Wizard with First-Activation Prompt

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Enhance the existing setup wizard to automatically register the Maproom MCP server after provider selection. When setup completes, the extension will:
1. Write `.vscode/mcp.json` using `MCPConfigWriter` (from MCPINIT-1001)
2. Prompt user to restart VS Code to activate the MCP server
3. Provide clear success messaging with next steps

Also implement a first-activation flow that prompts new users to run setup when they first open a workspace without Maproom configuration.

This ticket completes the one-click setup experience, eliminating all manual configuration steps.

## Background

The VSCode Maproom extension already has a setup wizard (`src/ui/setupWizard.ts`, 285 lines) that:
- Guides users through provider selection (OpenAI, Google, Ollama)
- Collects credentials via `SecretStorage`
- Manages embedding provider configuration

However, after the wizard completes, users must still:
1. Manually create `.vscode/mcp.json` to register the MCP server
2. Restart VS Code for MCP client to discover the server
3. Figure out the correct MCP configuration syntax

**This ticket closes the gap**: Enhance the existing wizard to automatically write `.vscode/mcp.json` using the `MCPConfigWriter` (MCPINIT-1001), completing the end-to-end automation.

Additionally, implement a first-activation prompt so new users discover the setup wizard without having to search the command palette.

**Key Principle**: We're enhancing existing functionality, not rebuilding it. The wizard already handles the hard parts (UI, credential collection) - we just add MCP registration at the end.

## Acceptance Criteria

### Wizard Integration
- [ ] After provider selection, `runSetupWizard()` calls `MCPConfigWriter.registerMCPServer()`
- [ ] Success message shows: "MCP server configured! Restart VS Code to activate."
- [ ] Message includes "Restart Now" button that triggers `workbench.action.reloadWindow`
- [ ] Command `maproom.setup` available in command palette
- [ ] Wizard handles "no workspace open" gracefully with clear error message
- [ ] User-friendly error messages for all failure modes

### First-Activation Prompt
- [ ] On first activation (no `.vscode/mcp.json` exists), show prompt: "Maproom MCP server not configured. Run setup?"
- [ ] Prompt has two buttons: "Run Setup" and "Remind Me Later"
- [ ] "Run Setup" button invokes `maproom.setup` command
- [ ] "Remind Me Later" dismisses without action
- [ ] Prompt only shows once per workspace (not every activation)

### Testing
- [ ] Unit test: Wizard calls config writer with correct provider after selection
- [ ] Unit test: First-activation logic correctly detects missing config
- [ ] Integration test: Full wizard flow writes `.vscode/mcp.json`
- [ ] Manual test: Successfully completed setup with each provider (OpenAI, Google, Ollama)
- [ ] Manual test: Restart successfully activates MCP server

## Technical Requirements

### File Modifications

**File 1**: `packages/vscode-maproom/src/ui/setupWizard.ts` (+50 lines)

**Changes**:
1. Import `MCPConfigWriter` from `../config/mcp-writer`
2. After credential collection, add MCP registration step
3. Show success message with restart prompt

**Implementation**:
```typescript
import { MCPConfigWriter } from '../config/mcp-writer'

export async function runSetupWizard(
  context: vscode.ExtensionContext
): Promise<EmbeddingProvider | undefined> {
  // ... existing provider selection code ...

  const provider = await showProviderPicker()
  if (!provider) return undefined

  // ... existing credential collection via SecretStorage ...

  // NEW: Write MCP configuration
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) {
    vscode.window.showErrorMessage(
      'No workspace folder open. Open a folder or workspace to configure Maproom.'
    )
    return undefined
  }

  try {
    const writer = new MCPConfigWriter()
    await writer.registerMCPServer(workspaceRoot, provider)

    // Success! Prompt for restart
    const action = await vscode.window.showInformationMessage(
      'Maproom MCP server configured! Restart VS Code to activate the MCP server.',
      'Restart Now',
      'Later'
    )

    if (action === 'Restart Now') {
      await vscode.commands.executeCommand('workbench.action.reloadWindow')
    }
  } catch (error) {
    vscode.window.showErrorMessage(
      `Failed to configure MCP server: ${error instanceof Error ? error.message : String(error)}`
    )
    return undefined
  }

  return provider
}
```

**File 2**: `packages/vscode-maproom/src/extension.ts` (+20 lines)

**Changes**:
1. Add first-activation check in `activate()` function
2. Check if `.vscode/mcp.json` exists
3. Show prompt if missing, only once per workspace

**Implementation**:
```typescript
import * as path from 'path'
import * as fs from 'fs'

export async function activate(context: vscode.ExtensionContext) {
  // ... existing command registration ...

  // Check for MCP configuration on first activation
  await checkAndPromptForSetup(context)
}

async function checkAndPromptForSetup(context: vscode.ExtensionContext): Promise<void> {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) {
    return // No workspace, skip prompt
  }

  const mcpConfigPath = path.join(workspaceRoot, '.vscode', 'mcp.json')
  const configExists = fs.existsSync(mcpConfigPath)

  if (!configExists) {
    // Check if we've already prompted for this workspace
    const workspaceState = context.workspaceState
    const hasPrompted = workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

    if (!hasPrompted) {
      const action = await vscode.window.showInformationMessage(
        'Maproom MCP server not configured. Run setup to enable semantic code search?',
        'Run Setup',
        'Remind Me Later'
      )

      // Mark as prompted (even if they chose "Remind Me Later")
      await workspaceState.update('maproom.hasPromptedSetup', true)

      if (action === 'Run Setup') {
        await vscode.commands.executeCommand('maproom.setup')
      }
    }
  }
}
```

### Command Registration (existing, verify it's present)

```typescript
// Should already exist in extension.ts
vscode.commands.registerCommand('maproom.setup', async () => {
  await runSetupWizard(context)
})
```

### Error Handling Requirements

**Clear error messages for common failures**:

1. **No workspace open**:
   > "No workspace folder open. Open a folder or workspace to configure Maproom."

2. **Config write failed**:
   > "Failed to configure MCP server: [specific error]. Try running setup again or check Output panel for details."

3. **Provider selection cancelled**:
   > (No error message - silent cancellation is appropriate)

4. **Existing MCP config is malformed**:
   > "Existing .vscode/mcp.json has invalid JSON. Please fix or remove it before running setup."

## Implementation Notes

### Design Pattern: Enhancement, Not Replacement

The existing wizard is well-designed and functional. We're adding ONE new step at the end:

**Current flow**:
1. Show provider picker (existing)
2. Collect credentials (existing)
3. Return selected provider (existing)

**Enhanced flow**:
1. Show provider picker (existing)
2. Collect credentials (existing)
3. **Write MCP config** (NEW - this ticket)
4. **Prompt for restart** (NEW - this ticket)
5. Return selected provider (existing)

### First-Activation UX Considerations

**When to show prompt**:
- First time extension activates in a workspace
- After `.vscode/mcp.json` is deleted
- NOT every time VS Code starts (annoying)
- NOT when user explicitly dismisses setup

**Storage mechanism**: Use `workspaceState` (workspace-scoped, not global) so each workspace is independent.

**Reset mechanism**: If user wants to see prompt again, they can either:
1. Delete `.vscode/mcp.json` (will re-trigger prompt)
2. Run `maproom.setup` from command palette directly

### Testing Strategy

**Unit Tests**:
1. Wizard calls `MCPConfigWriter.registerMCPServer()` with correct provider
2. First-activation detects missing `.vscode/mcp.json`
3. Workspace state prevents duplicate prompts
4. Error handling for "no workspace" scenario

**Integration Tests**:
1. Full wizard flow writes `.vscode/mcp.json` with correct format
2. Config is readable by VS Code MCP client (validate JSON structure)

**Manual Tests** (critical for UX):
1. Run setup with OpenAI provider → config written → restart prompt works
2. Run setup with Google provider → config written → restart prompt works
3. Run setup with Ollama provider → config written → restart prompt works
4. First activation without config → prompt shows → "Run Setup" works
5. First activation without config → prompt shows → "Remind Me Later" dismisses
6. Second activation → prompt does NOT show again
7. Delete `.vscode/mcp.json` → prompt shows again
8. Try setup without workspace → clear error message

### Restart Behavior

**Why restart is required**: VS Code MCP client reads `.vscode/mcp.json` during startup. Changes to this file require a window reload to take effect.

**Alternative considered**: Automatic restart. **Rejected** because:
- User might have unsaved work
- User might want to review the config first
- VS Code UX guidelines prefer prompts over forced actions

**Compromise**: Show clear message explaining WHY restart is needed, with one-click "Restart Now" button.

## Dependencies

**MCPINIT-1001** (CRITICAL) - Must be completed first because this ticket imports and uses `MCPConfigWriter`.

## Risk Assessment

### Risk 1: Wizard Enhancement Breaks Existing Functionality
- **Impact**: Existing users lose ability to run setup
- **Mitigation**: Extensive manual testing with all three providers
- **Test**: Verify existing wizard behavior (credential collection) still works

### Risk 2: Restart Prompt is Annoying
- **Impact**: Users feel nagged
- **Mitigation**: Only show once per setup completion, clear "Later" option
- **Test**: Manual UX review - does the prompt feel helpful or annoying?

### Risk 3: First-Activation Prompt Appears Too Often
- **Impact**: Users see prompt on every activation
- **Mitigation**: Use `workspaceState` to track "has prompted" flag
- **Test**: Unit test verifies state management, manual test verifies one-time behavior

### Risk 4: MCP Config Format Changes
- **Impact**: Extension writes outdated config format
- **Mitigation**: Config format is standardized by VS Code, low risk
- **Fallback**: If format changes, update `MCPConfigWriter` (encapsulated in one place)

## Files/Packages Affected

### Files to Modify
- `packages/vscode-maproom/src/ui/setupWizard.ts` (+50 lines)
- `packages/vscode-maproom/src/extension.ts` (+20 lines)

### Files to Create
- `packages/vscode-maproom/src/ui/setupWizard.test.ts` (~100 lines) [if not exists]
- `packages/vscode-maproom/src/extension.test.ts` (~80 lines) [if not exists]

### Files to Read (for context)
- `packages/vscode-maproom/src/config/mcp-writer.ts` - Config writer from MCPINIT-1001
- `packages/vscode-maproom/src/config/secrets.ts` - See how credentials are handled
- `.agents/projects/MCPINIT_mcp-extension-initialization/tickets/MCPINIT-1001_mcp-configuration-writer.md` - Dependency ticket

### Package Context
- **Package**: `packages/vscode-maproom`
- **Test Command**: `pnpm test`
- **Build Command**: `pnpm build`
- **Target Version**: 0.2.0 (feature addition)

## Related Documentation

- [MCPINIT-1001 Ticket](./MCPINIT-1001_mcp-configuration-writer.md) - Config writer dependency
- [Planning: Architecture](../planning/architecture.md) - Wizard integration design
- [Planning: Quality Strategy](../planning/quality-strategy.md) - Manual testing checklist
- [Existing Setup Wizard](./../../../packages/vscode-maproom/src/ui/setupWizard.ts) - Code to enhance
- [VS Code Extension API: Window](https://code.visualstudio.com/api/references/vscode-api#window) - Message and restart APIs

## Definition of Done

- [ ] `setupWizard.ts` enhanced to call `MCPConfigWriter` after provider selection
- [ ] Success message with "Restart Now" button implemented
- [ ] First-activation prompt implemented in `extension.ts`
- [ ] Workspace state prevents duplicate prompts
- [ ] All 16 acceptance criteria met
- [ ] Unit tests written and passing
- [ ] Integration tests written and passing
- [ ] Manual testing completed with all 3 providers
- [ ] Error messages are user-friendly (verified manually)
- [ ] Code follows TypeScript best practices
- [ ] JSDoc comments added to new functions
- [ ] No lint violations
- [ ] Files use ESM modules (import/export)
- [ ] Ready for production release (0.2.0)

---

**Estimated Complexity**: Low
**Estimated Time**: 2-3 hours
**Phase**: 1 (Foundation)
**Dependency**: MCPINIT-1001 (must complete first)
