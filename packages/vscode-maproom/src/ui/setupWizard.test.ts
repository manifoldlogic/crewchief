/**
 * Tests for Setup Wizard
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import * as http from 'http'

/**
 * Mock ExtensionContext
 */
class MockExtensionContext {
  private workspaceStateData = new Map<string, any>()

  workspaceState = {
    get: (key: string) => this.workspaceStateData.get(key),
    update: (key: string, value: any) => {
      this.workspaceStateData.set(key, value)
      return Promise.resolve()
    },
    keys: () => Array.from(this.workspaceStateData.keys()),
  }

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
 * Mock vscode module
 */
vi.mock('vscode', () => ({
  window: {
    showQuickPick: async (items: MockQuickPickItem[], _options?: any) => {
      lastQuickPickOptions = items
      return quickPickResult
    },
    showInformationMessage: (message: string) => {
      lastInfoMessage = message
      return Promise.resolve()
    },
  },
  commands: {
    registerCommand: (command: string, callback: Function) => {
      registeredCommands.set(command, callback)
      return { dispose: () => registeredCommands.delete(command) }
    },
  },
}))

// Import setupWizard after mock is set up
const {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
  detectOllama,
} = await import('./setupWizard.js')

describe('Setup Wizard', () => {
  let context: MockExtensionContext

  beforeEach(() => {
    context = new MockExtensionContext()
    lastQuickPickOptions = []
    quickPickResult = undefined
    registeredCommands.clear()
    lastInfoMessage = undefined
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

      // Should show success message
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
})
