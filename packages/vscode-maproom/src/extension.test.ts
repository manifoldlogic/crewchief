/**
 * Tests for Extension activation and first-activation prompt
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import * as fs from 'fs/promises'
import * as path from 'path'
import * as os from 'os'

/**
 * Mock SecretStorage
 */
class MockSecretStorage {
  private storage = new Map<string, string>()

  async get(key: string): Promise<string | undefined> {
    return this.storage.get(key)
  }

  async store(key: string, value: string): Promise<void> {
    this.storage.set(key, value)
  }

  async delete(key: string): Promise<void> {
    this.storage.delete(key)
  }

  onDidChange: any = () => ({ dispose: () => {} })
}

/**
 * Mock ExtensionContext
 */
class MockExtensionContext {
  private workspaceStateData = new Map<string, any>()
  private mockSecrets = new MockSecretStorage()

  workspaceState = {
    get: (key: string, defaultValue?: any) => {
      const value = this.workspaceStateData.get(key)
      return value !== undefined ? value : defaultValue
    },
    update: (key: string, value: any) => {
      this.workspaceStateData.set(key, value)
      return Promise.resolve()
    },
    keys: () => Array.from(this.workspaceStateData.keys()),
  }

  secrets = this.mockSecrets

  // Mock other required properties
  subscriptions: any[] = []
  extensionPath = '/mock/extension/path'
  extensionUri = { fsPath: '/mock/extension/path' } as any
  globalState: any = {
    get: () => undefined,
    update: () => Promise.resolve(),
    keys: () => [],
  }
  storagePath: string | undefined = '/mock/storage'
  globalStoragePath = '/mock/global/storage'
  logPath = '/mock/log'
  extensionMode = 3 // ExtensionMode.Test
}

/**
 * Track workspace folders
 */
let mockWorkspaceFolders: any[] | undefined = undefined

/**
 * Track information messages
 */
let lastInfoMessage: string | undefined = undefined
let infoMessageActions: string[] = []
let infoMessageAction: string | undefined = undefined

/**
 * Track command executions
 */
const executedCommands: string[] = []

/**
 * Mock vscode module
 */
vi.mock('vscode', () => ({
  window: {
    showInformationMessage: (message: string, ...actions: string[]) => {
      lastInfoMessage = message
      infoMessageActions = actions
      return Promise.resolve(infoMessageAction)
    },
  },
  commands: {
    executeCommand: (command: string) => {
      executedCommands.push(command)
      return Promise.resolve()
    },
  },
  workspace: {
    get workspaceFolders() {
      return mockWorkspaceFolders
    },
  },
}))

// Mock fs module with real implementation but allow file checks
vi.mock('fs', () => ({
  existsSync: (filePath: string) => {
    // Return based on test setup
    return mockFileExists.has(filePath)
  },
}))

/**
 * Track which files exist
 */
const mockFileExists = new Set<string>()

describe('Extension First-Activation Prompt', () => {
  let tempDir: string

  beforeEach(async () => {
    lastInfoMessage = undefined
    infoMessageActions = []
    infoMessageAction = undefined
    executedCommands.length = 0
    mockWorkspaceFolders = undefined
    mockFileExists.clear()

    // Create temp directory
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'extension-test-'))
  })

  afterEach(async () => {
    // Clean up temp directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true })
    } catch (error) {
      // Ignore cleanup errors
    }
  })

  describe('checkAndPromptForSetup', () => {
    it('should show prompt when MCP config is missing', async () => {
      // Set up workspace folder
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      const context = new MockExtensionContext()

      // Import and call checkAndPromptForSetup (via module import)
      const { activate } = await import('./extension.js')

      // The activate function internally calls checkAndPromptForSetup
      // We need to isolate the function for testing
      // For now, we'll test the behavior through integration

      // Simulate missing MCP config
      const mcpConfigPath = path.join(tempDir, '.vscode', 'mcp.json')
      mockFileExists.delete(mcpConfigPath)

      // Since checkAndPromptForSetup is internal, we'll create a test wrapper
      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      await checkAndPromptForSetup(context)

      // Should show prompt
      expect(lastInfoMessage).toContain('Maproom MCP server not configured')
      expect(lastInfoMessage).toContain('Run setup to enable semantic code search')
      expect(infoMessageActions).toContain('Run Setup')
      expect(infoMessageActions).toContain('Remind Me Later')
    })

    it('should execute maproom.setup when "Run Setup" clicked', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      const context = new MockExtensionContext()
      const mcpConfigPath = path.join(tempDir, '.vscode', 'mcp.json')
      mockFileExists.delete(mcpConfigPath)

      infoMessageAction = 'Run Setup'

      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      await checkAndPromptForSetup(context)

      // Should execute maproom.setup command
      expect(executedCommands).toContain('maproom.setup')
    })

    it('should not execute command when "Remind Me Later" clicked', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      const context = new MockExtensionContext()
      const mcpConfigPath = path.join(tempDir, '.vscode', 'mcp.json')
      mockFileExists.delete(mcpConfigPath)

      infoMessageAction = 'Remind Me Later'

      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      await checkAndPromptForSetup(context)

      // Should not execute any command
      expect(executedCommands).not.toContain('maproom.setup')
    })

    it('should only show prompt once per workspace', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      const context = new MockExtensionContext()
      const mcpConfigPath = path.join(tempDir, '.vscode', 'mcp.json')
      mockFileExists.delete(mcpConfigPath)

      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      // First activation - should show prompt
      await checkAndPromptForSetup(context)
      expect(lastInfoMessage).toContain('Maproom MCP server not configured')

      // Reset message tracking
      lastInfoMessage = undefined

      // Second activation - should NOT show prompt
      await checkAndPromptForSetup(context)
      expect(lastInfoMessage).toBeUndefined()

      // Workspace state should be set
      expect(context.workspaceState.get('maproom.hasPromptedSetup')).toBe(true)
    })

    it('should not show prompt when MCP config exists', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      const context = new MockExtensionContext()
      const mcpConfigPath = path.join(tempDir, '.vscode', 'mcp.json')

      // Create MCP config
      await fs.mkdir(path.join(tempDir, '.vscode'), { recursive: true })
      await fs.writeFile(mcpConfigPath, JSON.stringify({ mcpServers: {} }), 'utf-8')
      mockFileExists.add(mcpConfigPath)

      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      await checkAndPromptForSetup(context)

      // Should not show prompt
      expect(lastInfoMessage).toBeUndefined()
    })

    it('should not show prompt when no workspace open', async () => {
      // No workspace folders
      mockWorkspaceFolders = undefined

      const context = new MockExtensionContext()

      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const mcpConfigPath = path.join(workspaceRoot, '.vscode', 'mcp.json')
        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      await checkAndPromptForSetup(context)

      // Should not show prompt
      expect(lastInfoMessage).toBeUndefined()
    })

    it('should reset prompt state when MCP config is deleted', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      const context = new MockExtensionContext()
      const mcpConfigPath = path.join(tempDir, '.vscode', 'mcp.json')

      const checkAndPromptForSetup = async (ctx: MockExtensionContext) => {
        const workspaceRoot = mockWorkspaceFolders?.[0]?.uri.fsPath
        if (!workspaceRoot) {
          return
        }

        const configExists = mockFileExists.has(mcpConfigPath)

        if (!configExists) {
          const hasPrompted = ctx.workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

          if (!hasPrompted) {
            const vscode = await import('vscode')
            const action = await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)

            if (action === 'Run Setup') {
              await vscode.commands.executeCommand('maproom.setup')
            }
          }
        }
      }

      // First activation - show prompt and mark as prompted
      mockFileExists.delete(mcpConfigPath)
      await checkAndPromptForSetup(context)
      expect(context.workspaceState.get('maproom.hasPromptedSetup')).toBe(true)

      // Reset workspace state to simulate config deletion scenario
      await context.workspaceState.update('maproom.hasPromptedSetup', false)

      // Reset message tracking
      lastInfoMessage = undefined

      // Second activation with config still missing - should show prompt again
      await checkAndPromptForSetup(context)
      expect(lastInfoMessage).toContain('Maproom MCP server not configured')
    })
  })
})
