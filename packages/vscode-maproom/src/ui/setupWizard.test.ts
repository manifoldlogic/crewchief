/**
 * Tests for Setup Wizard
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import * as http from 'http'
import * as fs from 'fs/promises'
import * as fsSync from 'fs'
import * as path from 'path'
import * as os from 'os'
import type { DatabaseConfig } from '../services/database-checker'

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
    get: (key: string) => this.workspaceStateData.get(key),
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
 * Mock QuickPick item
 */
interface MockQuickPickItem {
  label: string
  detail: string
  value: string
}

/**
 * Track showQuickPick calls
 */
let lastQuickPickOptions: MockQuickPickItem[] = []
let quickPickResult: MockQuickPickItem | undefined = undefined

/**
 * Track command registrations
 */
const registeredCommands = new Map<string, Function>()

/**
 * Track information messages
 */
let lastInfoMessage: string | undefined = undefined

/**
 * Track showInputBox calls
 */
let inputBoxResult: string | undefined = 'test-api-key'

/**
 * Track workspace folders
 */
let mockWorkspaceFolders: any[] | undefined = undefined

/**
 * Track information message action results
 */
let infoMessageAction: string | undefined = undefined

/**
 * Track command executions
 */
const executedCommands: string[] = []

/**
 * Track error messages
 */
let lastErrorMessage: string | undefined = undefined

/**
 * Track warning messages
 */
let lastWarningMessage: string | undefined = undefined
let warningMessageAction: string | undefined = undefined

/**
 * Track open dialog results
 */
let openDialogResult: { fsPath: string }[] | undefined = undefined

/**
 * Track configuration updates
 */
const configUpdates: { key: string; value: any; target: number }[] = []

/**
 * Track terminal creations
 */
const createdTerminals: { name: string; text: string }[] = []

/**
 * Track clipboard writes
 */
let lastClipboardText: string | undefined = undefined

/**
 * Mock homedir path for SQLite tests
 */
let mockHomedir: string | undefined = undefined

/**
 * Mock node:os module
 */
vi.mock('node:os', async () => {
  const actual = await vi.importActual<typeof import('node:os')>('node:os')
  return {
    ...actual,
    homedir: () => mockHomedir ?? actual.homedir(),
  }
})

/**
 * Mock vscode module
 */
vi.mock('vscode', () => ({
  window: {
    showQuickPick: async (items: MockQuickPickItem[], _options?: any) => {
      lastQuickPickOptions = items
      return quickPickResult
    },
    showInformationMessage: (message: string, ...actions: string[]) => {
      lastInfoMessage = message
      return Promise.resolve(infoMessageAction)
    },
    showInputBox: async (_options?: any) => {
      return inputBoxResult
    },
    showErrorMessage: (message: string) => {
      lastErrorMessage = message
      return Promise.resolve()
    },
    showWarningMessage: (message: string, _options?: any, ...actions: string[]) => {
      lastWarningMessage = message
      return Promise.resolve(warningMessageAction)
    },
    showOpenDialog: async (_options?: any) => {
      return openDialogResult
    },
    createTerminal: (name: string) => {
      const terminal = {
        name,
        show: () => {},
        sendText: (text: string) => {
          createdTerminals.push({ name, text })
        },
      }
      return terminal
    },
  },
  commands: {
    registerCommand: (command: string, callback: Function) => {
      registeredCommands.set(command, callback)
      return { dispose: () => registeredCommands.delete(command) }
    },
    executeCommand: (command: string) => {
      executedCommands.push(command)
      return Promise.resolve()
    },
  },
  workspace: {
    get workspaceFolders() {
      return mockWorkspaceFolders
    },
    getConfiguration: () => ({
      get: (key: string) => {
        // Default to sqlite for tests
        if (key === 'provider') return 'sqlite'
        if (key === 'sqlitePath') return ''
        return undefined
      },
      update: (key: string, value: any, target: number) => {
        configUpdates.push({ key, value, target })
        return Promise.resolve()
      },
    }),
  },
  env: {
    clipboard: {
      writeText: async (text: string) => {
        lastClipboardText = text
      },
    },
  },
  ConfigurationTarget: {
    Global: 1,
    Workspace: 2,
    WorkspaceFolder: 3,
  },
}))

// Import setupWizard after mock is set up
const {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
  detectOllama,
  runSqliteSetup,
  promptForSqlitePath,
  showNoSqliteGuidance,
  runDatabaseSetup,
} = await import('./setupWizard.js')

describe('Setup Wizard', () => {
  let context: MockExtensionContext
  let tempDir: string

  beforeEach(async () => {
    context = new MockExtensionContext()
    lastQuickPickOptions = []
    quickPickResult = undefined
    registeredCommands.clear()
    lastInfoMessage = undefined
    lastErrorMessage = undefined
    lastWarningMessage = undefined
    warningMessageAction = undefined
    openDialogResult = undefined
    configUpdates.length = 0
    createdTerminals.length = 0
    lastClipboardText = undefined
    mockHomedir = undefined
    inputBoxResult = 'test-api-key'
    infoMessageAction = undefined
    executedCommands.length = 0
    mockWorkspaceFolders = undefined

    // Create temp directory for MCP config tests
    tempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'setup-wizard-test-'))
  })

  afterEach(async () => {
    // Clean up temp directory
    try {
      await fs.rm(tempDir, { recursive: true, force: true })
    } catch (error) {
      // Ignore cleanup errors
    }
  })

  describe('detectOllama', () => {
    it('should return true when Ollama is running', async () => {
      // Create a minimal HTTP server on port 11434
      const server = http.createServer((_req, res) => {
        res.writeHead(200)
        res.end()
      })

      await new Promise<void>((resolve) => {
        server.listen(11434, () => resolve())
      })

      try {
        const result = await detectOllama()
        expect(result).toBe(true)
      } finally {
        // Clean up server
        await new Promise<void>((resolve) => {
          server.close(() => resolve())
        })
      }
    }, 5000) // Increase timeout for server operations

    it('should return false when Ollama is not running', async () => {
      // No server running - should timeout and return false
      const result = await detectOllama()
      expect(result).toBe(false)
    }, 5000) // Increase timeout to allow for detection timeout

    it('should complete within timeout period', async () => {
      const startTime = Date.now()
      await detectOllama()
      const duration = Date.now() - startTime

      // Should complete within ~2.5 seconds (2s timeout + overhead)
      // Note: When connection is refused, it may complete immediately
      // When truly timing out (no response), it takes the full 2s
      expect(duration).toBeLessThan(3000)
    }, 5000)
  })

  describe('runSetupWizard', () => {
    it('should show QuickPick with three provider options', async () => {
      // User selects Ollama
      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      await runSetupWizard(context)

      // Should show 3 options
      expect(lastQuickPickOptions).toHaveLength(3)

      // Check option labels
      const labels = lastQuickPickOptions.map((opt) => opt.label)
      expect(labels).toContain('$(zap) Ollama')
      expect(labels).toContain('$(cloud) OpenAI')
      expect(labels).toContain('$(cloud) Google Vertex AI')
    })

    it('should mark Ollama as recommended when detected running', async () => {
      // Create server to simulate Ollama running
      const server = http.createServer((_req, res) => {
        res.writeHead(200)
        res.end()
      })

      await new Promise<void>((resolve) => {
        server.listen(11434, () => resolve())
      })

      try {
        quickPickResult = {
          label: '$(zap) Ollama (Recommended)',
          detail: 'Running locally - fast and private',
          value: 'ollama',
        }

        await runSetupWizard(context)

        // Find Ollama option
        const ollamaOption = lastQuickPickOptions.find((opt) =>
          opt.label.includes('Ollama')
        )

        expect(ollamaOption?.label).toBe('$(zap) Ollama (Recommended)')
        expect(ollamaOption?.detail).toBe('Running locally - fast and private')
      } finally {
        await new Promise<void>((resolve) => {
          server.close(() => resolve())
        })
      }
    }, 5000)

    it('should save selected provider to workspace state', async () => {
      // Set up mock workspace folder
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(cloud) OpenAI',
        detail: 'API key required',
        value: 'openai',
      }

      const result = await runSetupWizard(context)

      // Should return selected provider
      expect(result).toBe('openai')

      // Should save to workspace state
      expect(context.workspaceState.get('maproom.provider')).toBe('openai')
    })

    it('should return undefined when user cancels', async () => {
      quickPickResult = undefined

      const result = await runSetupWizard(context)

      expect(result).toBeUndefined()
      expect(context.workspaceState.get('maproom.provider')).toBeUndefined()
    })

    it('should handle Google Vertex AI selection', async () => {
      // Set up mock workspace folder
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(cloud) Google Vertex AI',
        detail: 'API key required',
        value: 'google',
      }

      const result = await runSetupWizard(context)

      expect(result).toBe('google')
      expect(context.workspaceState.get('maproom.provider')).toBe('google')
    })
  })

  describe('getConfiguredProvider', () => {
    it('should return undefined when no provider configured', () => {
      const provider = getConfiguredProvider(context)
      expect(provider).toBeUndefined()
    })

    it('should return saved provider from workspace state', async () => {
      await context.workspaceState.update('maproom.provider', 'ollama')

      const provider = getConfiguredProvider(context)
      expect(provider).toBe('ollama')
    })

    it('should return correct provider for each type', async () => {
      // Test ollama
      await context.workspaceState.update('maproom.provider', 'ollama')
      expect(getConfiguredProvider(context)).toBe('ollama')

      // Test openai
      await context.workspaceState.update('maproom.provider', 'openai')
      expect(getConfiguredProvider(context)).toBe('openai')

      // Test google
      await context.workspaceState.update('maproom.provider', 'google')
      expect(getConfiguredProvider(context)).toBe('google')
    })
  })

  describe('registerSetupCommand', () => {
    it('should register maproom.setup command', () => {
      registerSetupCommand(context)

      expect(registeredCommands.has('maproom.setup')).toBe(true)
    })

    it('should run setup wizard when command executed', async () => {
      // Set up mock workspace folder
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      registerSetupCommand(context)

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      const command = registeredCommands.get('maproom.setup')
      await command!()

      // Should have shown QuickPick
      expect(lastQuickPickOptions.length).toBeGreaterThan(0)

      // Should save selection
      expect(context.workspaceState.get('maproom.provider')).toBe('ollama')

      // Should show success message (from registerSetupCommand wrapper)
      expect(lastInfoMessage).toBeDefined()
      expect(lastInfoMessage).toContain('OLLAMA')
    })

    it('should handle cancelled setup gracefully', async () => {
      registerSetupCommand(context)

      quickPickResult = undefined

      const command = registeredCommands.get('maproom.setup')
      await command!()

      // Should not save anything
      expect(context.workspaceState.get('maproom.provider')).toBeUndefined()

      // Should not show success message
      expect(lastInfoMessage).toBeUndefined()
    })

    it('should add command disposable to context subscriptions', () => {
      const initialLength = context.subscriptions.length

      registerSetupCommand(context)

      expect(context.subscriptions.length).toBe(initialLength + 1)
    })
  })

  describe('Provider Options Display', () => {
    it('should include codicon icons in labels', async () => {
      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      await runSetupWizard(context)

      // All options should have codicon icons
      expect(lastQuickPickOptions[0].label).toMatch(/\$\(.*\)/)
      expect(lastQuickPickOptions[1].label).toMatch(/\$\(.*\)/)
      expect(lastQuickPickOptions[2].label).toMatch(/\$\(.*\)/)
    })

    it('should provide helpful detail text for each option', async () => {
      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      await runSetupWizard(context)

      // All options should have detail text
      expect(lastQuickPickOptions[0].detail).toBeTruthy()
      expect(lastQuickPickOptions[1].detail).toBeTruthy()
      expect(lastQuickPickOptions[2].detail).toBeTruthy()

      // Cloud options should mention API key requirement
      const cloudOptions = lastQuickPickOptions.filter((opt) =>
        opt.label.includes('cloud')
      )
      cloudOptions.forEach((opt) => {
        expect(opt.detail).toContain('API key required')
      })
    })
  })

  describe('MCP Configuration Integration', () => {
    it('should write MCP configuration after provider selection', async () => {
      // Set up mock workspace folder
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      const result = await runSetupWizard(context)

      expect(result).toBe('ollama')

      // Verify MCP config was written
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')
      const configContent = await fs.readFile(configPath, 'utf-8')
      const config = JSON.parse(configContent)

      expect(config.servers.maproom).toBeDefined()
      expect(config.servers.maproom.command).toBe('npx')
    })

    it('should show restart prompt after MCP configuration', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      await runSetupWizard(context)

      // Should show restart prompt
      expect(lastInfoMessage).toContain('MCP server configured')
      expect(lastInfoMessage).toContain('Restart your editor')
    })

    it('should execute reload command when "Restart Now" clicked', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      infoMessageAction = 'Restart Now'

      await runSetupWizard(context)

      // Should execute reload command
      expect(executedCommands).toContain('workbench.action.reloadWindow')
    })

    it('should not execute reload command when "Later" clicked', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      infoMessageAction = 'Later'

      await runSetupWizard(context)

      // Should not execute reload command
      expect(executedCommands).not.toContain('workbench.action.reloadWindow')
    })

    it('should handle no workspace open gracefully', async () => {
      // No workspace folders
      mockWorkspaceFolders = undefined

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      const result = await runSetupWizard(context)

      // Should return undefined
      expect(result).toBeUndefined()

      // Should show error message
      expect(lastErrorMessage).toContain('No workspace folder open')
    })

    it('should write provider-specific environment variables', async () => {
      mockWorkspaceFolders = [{ uri: { fsPath: tempDir } }]

      quickPickResult = {
        label: '$(cloud) OpenAI',
        detail: 'API key required',
        value: 'openai',
      }

      await runSetupWizard(context)

      // Verify OpenAI env var in config
      const configPath = path.join(tempDir, '.vscode', 'mcp.json')
      const configContent = await fs.readFile(configPath, 'utf-8')
      const config = JSON.parse(configContent)

      expect(config.servers.maproom.env).toEqual({
        MAPROOM_DATABASE_URL: 'sqlite://~/.maproom/maproom.db',
        MAPROOM_EMBEDDING_PROVIDER: 'openai',
        OPENAI_API_KEY: '${env:OPENAI_API_KEY}',
      })
    })

    it('should handle MCP writer errors gracefully', async () => {
      // Set workspace to invalid path to trigger error
      mockWorkspaceFolders = [{ uri: { fsPath: '/etc/invalid-path' } }]

      quickPickResult = {
        label: '$(zap) Ollama',
        detail: 'Local inference - requires Ollama installation',
        value: 'ollama',
      }

      const result = await runSetupWizard(context)

      // Should return undefined on error
      expect(result).toBeUndefined()

      // Should show error message
      expect(lastErrorMessage).toContain('Failed to configure MCP server')
    })
  })

  describe('SQLite Setup Wizard', () => {
    let sqliteTempDir: string

    beforeEach(async () => {
      // Create temp directory for SQLite tests
      sqliteTempDir = await fs.mkdtemp(path.join(os.tmpdir(), 'sqlite-setup-test-'))
    })

    afterEach(async () => {
      try {
        await fs.rm(sqliteTempDir, { recursive: true, force: true })
      } catch {
        // Ignore cleanup errors
      }
    })

    describe('runSqliteSetup', () => {
      it('should return true when custom path is already configured and exists', async () => {
        const customPath = path.join(sqliteTempDir, 'custom.db')
        await fs.writeFile(customPath, '')

        const config: DatabaseConfig = {
          type: 'sqlite',
          url: `sqlite://${customPath}`,
          path: customPath,
        }

        const result = await runSqliteSetup(config)
        expect(result).toBe(true)
        // No dialog should be shown
        expect(lastInfoMessage).toBeUndefined()
      })

      it('should show "Found existing" dialog when default path exists', async () => {
        // Mock homedir to use temp directory
        mockHomedir = sqliteTempDir

        // Create default database file
        const maproomDir = path.join(sqliteTempDir, '.maproom')
        await fs.mkdir(maproomDir, { recursive: true })
        await fs.writeFile(path.join(maproomDir, 'maproom.db'), '')

        const config: DatabaseConfig = {
          type: 'sqlite',
          url: `sqlite://${path.join(maproomDir, 'maproom.db')}`,
          path: undefined, // No custom path configured
        }

        // User selects "Use Existing"
        infoMessageAction = 'Use Existing'

        const result = await runSqliteSetup(config)

        expect(result).toBe(true)
        expect(lastInfoMessage).toContain('Found existing Maproom index')
      })

      it('should prompt for path when user selects "Choose Different"', async () => {
        // Mock homedir to use temp directory
        mockHomedir = sqliteTempDir

        // Create default database file
        const maproomDir = path.join(sqliteTempDir, '.maproom')
        await fs.mkdir(maproomDir, { recursive: true })
        await fs.writeFile(path.join(maproomDir, 'maproom.db'), '')

        const config: DatabaseConfig = {
          type: 'sqlite',
          url: `sqlite://${path.join(maproomDir, 'maproom.db')}`,
          path: undefined,
        }

        // User selects "Choose Different" but then cancels file dialog
        infoMessageAction = 'Choose Different'
        openDialogResult = undefined

        const result = await runSqliteSetup(config)

        expect(result).toBe(false)
        expect(lastInfoMessage).toContain('Found existing')
      })

      it('should return false when user cancels', async () => {
        mockHomedir = sqliteTempDir

        // Create default database file
        const maproomDir = path.join(sqliteTempDir, '.maproom')
        await fs.mkdir(maproomDir, { recursive: true })
        await fs.writeFile(path.join(maproomDir, 'maproom.db'), '')

        const config: DatabaseConfig = {
          type: 'sqlite',
          url: `sqlite://${path.join(maproomDir, 'maproom.db')}`,
          path: undefined,
        }

        // User clicks Cancel
        infoMessageAction = 'Cancel'

        const result = await runSqliteSetup(config)

        expect(result).toBe(false)
      })

      it('should show guidance when no database exists', async () => {
        mockHomedir = sqliteTempDir

        // No .maproom directory exists
        const config: DatabaseConfig = {
          type: 'sqlite',
          url: 'sqlite:///nonexistent/path.db',
          path: undefined,
        }

        // User dismisses guidance
        warningMessageAction = undefined

        const result = await runSqliteSetup(config)

        expect(result).toBe(false)
        expect(lastWarningMessage).toContain('No Maproom index found')
      })
    })

    describe('promptForSqlitePath', () => {
      it('should update settings when file is selected', async () => {
        const selectedPath = '/custom/path/index.db'
        openDialogResult = [{ fsPath: selectedPath }]

        const result = await promptForSqlitePath()

        expect(result).toBe(true)
        expect(configUpdates).toHaveLength(1)
        expect(configUpdates[0].key).toBe('sqlitePath')
        expect(configUpdates[0].value).toBe(selectedPath)
        expect(configUpdates[0].target).toBe(1) // ConfigurationTarget.Global
        expect(lastInfoMessage).toContain(selectedPath)
      })

      it('should return false when dialog is cancelled', async () => {
        openDialogResult = undefined

        const result = await promptForSqlitePath()

        expect(result).toBe(false)
        expect(configUpdates).toHaveLength(0)
      })

      it('should return false when empty result', async () => {
        openDialogResult = []

        const result = await promptForSqlitePath()

        expect(result).toBe(false)
        expect(configUpdates).toHaveLength(0)
      })
    })

    describe('showNoSqliteGuidance', () => {
      it('should copy scan command to clipboard when selected', async () => {
        mockWorkspaceFolders = [{ uri: { fsPath: '/my/workspace' } }]
        warningMessageAction = 'Copy Scan Command'

        const result = await showNoSqliteGuidance()

        expect(result).toBe(true)
        expect(lastClipboardText).toBe('crewchief-maproom scan /my/workspace')
        expect(lastInfoMessage).toContain('copied to clipboard')
      })

      it('should use default path when no workspace open', async () => {
        mockWorkspaceFolders = undefined
        warningMessageAction = 'Copy Scan Command'

        const result = await showNoSqliteGuidance()

        expect(result).toBe(true)
        expect(lastClipboardText).toBe('crewchief-maproom scan /path/to/your/repo')
      })

      it('should open terminal when selected', async () => {
        mockWorkspaceFolders = [{ uri: { fsPath: '/my/workspace' } }]
        warningMessageAction = 'Open Terminal'

        const result = await showNoSqliteGuidance()

        expect(result).toBe(true)
        expect(createdTerminals).toHaveLength(1)
        expect(createdTerminals[0].name).toBe('Maproom Setup')
        expect(createdTerminals[0].text).toContain('crewchief-maproom scan')
      })

      it('should prompt for file when "Choose Existing File" selected', async () => {
        warningMessageAction = 'Choose Existing File'
        openDialogResult = [{ fsPath: '/some/path/db.sqlite' }]

        const result = await showNoSqliteGuidance()

        expect(result).toBe(true)
        expect(configUpdates).toHaveLength(1)
      })

      it('should return false when dialog dismissed', async () => {
        warningMessageAction = undefined

        const result = await showNoSqliteGuidance()

        expect(result).toBe(false)
      })
    })

    describe('runDatabaseSetup', () => {
      it('should call runSqliteSetup for sqlite type', async () => {
        // Mock resolveDatabaseConfig to return sqlite type
        // The function is imported from database-checker, which needs mocking
        // For this test, we verify the function exists and handles the call
        mockHomedir = sqliteTempDir

        // No database exists, so it will show guidance
        warningMessageAction = undefined

        const result = await runDatabaseSetup()

        // Should show SQLite guidance since no database exists
        expect(lastWarningMessage).toContain('No Maproom index found')
        expect(result).toBe(false)
      })
    })
  })
})
