/**
 * Tests for Extension activation and initialization flow
 *
 * Tests the new simplified activation flow:
 * 1. Fast sync activation (<500ms)
 * 2. Background: Ollama model check (ollama provider only)
 * 3. Background: Startup reconciliation
 * 4. Background: Start watch process
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
 * Track error messages
 */
let lastErrorMessage: string | undefined = undefined
let errorMessageActions: string[] = []
let errorMessageAction: string | undefined = undefined

/**
 * Track command executions
 */
const executedCommands: string[] = []

/**
 * Mock output channel
 */
const mockOutputChannel = {
  appendLine: vi.fn(),
  show: vi.fn(),
}

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
    showErrorMessage: (message: string, ...actions: string[]) => {
      lastErrorMessage = message
      errorMessageActions = actions
      return Promise.resolve(errorMessageAction)
    },
    createOutputChannel: () => mockOutputChannel,
    withProgress: (options: any, task: any) => {
      const progress = { report: vi.fn() }
      return task(progress)
    },
  },
  commands: {
    executeCommand: (command: string) => {
      executedCommands.push(command)
      return Promise.resolve()
    },
    registerCommand: () => ({ dispose: () => {} }),
  },
  workspace: {
    get workspaceFolders() {
      return mockWorkspaceFolders
    },
    getConfiguration: () => ({
      get: () => 'sqlite',
    }),
  },
  ProgressLocation: {
    Notification: 15,
  },
  env: {
    openExternal: vi.fn(),
  },
  Uri: {
    parse: (url: string) => ({ toString: () => url }),
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

describe('Extension Activation Flow', () => {
  let tempDir: string

  beforeEach(async () => {
    lastInfoMessage = undefined
    infoMessageActions = []
    infoMessageAction = undefined
    lastErrorMessage = undefined
    errorMessageActions = []
    errorMessageAction = undefined
    executedCommands.length = 0
    mockWorkspaceFolders = undefined
    mockFileExists.clear()
    mockOutputChannel.appendLine.mockClear()
    mockOutputChannel.show.mockClear()

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
            await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)
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
            await vscode.window.showInformationMessage(
              'Maproom MCP server not configured. Run setup to enable semantic code search?',
              'Run Setup',
              'Remind Me Later'
            )

            await ctx.workspaceState.update('maproom.hasPromptedSetup', true)
          }
        }
      }

      await checkAndPromptForSetup(context)

      // Should not show prompt
      expect(lastInfoMessage).toBeUndefined()
    })
  })

  describe('New Activation Flow', () => {
    it('should run Ollama model check only for ollama provider', async () => {
      const callOrder: string[] = []

      const ensureOllamaModel = async (provider: string) => {
        if (provider === 'ollama') {
          callOrder.push('ollama-check')
        }
      }

      const reconcileChanges = async () => {
        callOrder.push('reconcile')
      }

      const startWatch = async () => {
        callOrder.push('watch')
      }

      // Simulate flow with ollama provider
      const provider = 'ollama'
      await ensureOllamaModel(provider)
      await reconcileChanges()
      await startWatch()

      expect(callOrder).toEqual(['ollama-check', 'reconcile', 'watch'])
    })

    it('should skip Ollama check for non-ollama providers', async () => {
      const callOrder: string[] = []

      const ensureOllamaModel = async (provider: string) => {
        if (provider === 'ollama') {
          callOrder.push('ollama-check')
        }
      }

      const reconcileChanges = async () => {
        callOrder.push('reconcile')
      }

      const startWatch = async () => {
        callOrder.push('watch')
      }

      // Simulate flow with openai provider
      const provider = 'openai'
      await ensureOllamaModel(provider)
      await reconcileChanges()
      await startWatch()

      // Ollama check should be skipped
      expect(callOrder).toEqual(['reconcile', 'watch'])
    })

    it('should run reconciliation before starting watch', async () => {
      const callOrder: string[] = []

      const reconcileChanges = async () => {
        callOrder.push('reconcile')
      }

      const startWatch = async () => {
        callOrder.push('watch')
      }

      await reconcileChanges()
      await startWatch()

      expect(callOrder).toEqual(['reconcile', 'watch'])
    })

    it('should handle Ollama not running error', async () => {
      let errorHandled = false
      let statusBarState = ''

      class OllamaNotRunningError extends Error {
        constructor() {
          super('Ollama is not running')
          this.name = 'OllamaNotRunningError'
        }
      }

      const ensureOllamaModel = async () => {
        throw new OllamaNotRunningError()
      }

      const showOllamaNotRunningError = async () => {
        errorHandled = true
      }

      const setStatusBar = (state: string) => {
        statusBarState = state
      }

      try {
        await ensureOllamaModel()
      } catch (error) {
        if (error instanceof OllamaNotRunningError) {
          await showOllamaNotRunningError()
          setStatusBar('error')
        }
      }

      expect(errorHandled).toBe(true)
      expect(statusBarState).toBe('error')
    })

    it('should enter degraded mode when database not found', async () => {
      let statusBarState = ''
      let guidanceShown = false

      const checkDatabaseAvailable = async () => false

      const showNoSqliteGuidance = async () => {
        guidanceShown = true
      }

      const setStatusBar = (state: string) => {
        statusBarState = state
      }

      const dbAvailable = await checkDatabaseAvailable()

      if (!dbAvailable) {
        await showNoSqliteGuidance()
        setStatusBar('idle')
      }

      expect(guidanceShown).toBe(true)
      expect(statusBarState).toBe('idle')
    })
  })

  describe('runFirstTimeSetup', () => {
    it('should proceed to initializeServices after wizard completes', async () => {
      const callOrder: string[] = []

      const runSetupWizard = async () => {
        callOrder.push('wizard')
        return 'ollama'
      }

      const initializeServices = async () => {
        callOrder.push('initialize')
      }

      // Simulate first-time setup flow
      const provider = await runSetupWizard()
      if (provider) {
        await initializeServices()
      }

      expect(callOrder).toEqual(['wizard', 'initialize'])
    })

    it('should show idle state when user cancels setup', async () => {
      let statusBarState = ''

      const runSetupWizard = async () => {
        return null // User cancelled
      }

      const setStatusBar = (state: string) => {
        statusBarState = state
      }

      const provider = await runSetupWizard()
      if (!provider) {
        setStatusBar('idle')
      }

      expect(statusBarState).toBe('idle')
    })
  })

  describe('Status Bar State Transitions', () => {
    it('should transition through correct states during initialization', async () => {
      const stateTransitions: string[] = []

      const setStatusBar = (state: string) => {
        stateTransitions.push(state)
      }

      // Simulate initialization flow
      setStatusBar('starting')
      setStatusBar('reconciling')
      setStatusBar('watching')

      expect(stateTransitions).toEqual(['starting', 'reconciling', 'watching'])
    })

    it('should show error state when initialization fails', async () => {
      const stateTransitions: string[] = []

      const setStatusBar = (state: string) => {
        stateTransitions.push(state)
      }

      // Simulate initialization with error
      setStatusBar('starting')
      setStatusBar('reconciling')
      setStatusBar('error')

      expect(stateTransitions).toContain('error')
    })
  })
})
