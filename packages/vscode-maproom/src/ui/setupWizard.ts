/**
 * Setup wizard for Maproom extension first-run configuration
 *
 * Provides QuickPick UI for selecting embedding provider (Ollama/OpenAI/Google).
 * Intelligently detects if Ollama is running locally and recommends it as the
 * preferred option for fast, private indexing.
 *
 * Key features:
 * - QuickPick selection UI with provider options
 * - Ollama auto-detection via HTTP ping (localhost:11434)
 * - Provider selection saved to workspace state
 * - Secure API credential collection for OpenAI/Google
 * - Password-masked input for API keys
 * - Re-runnable via command palette
 * - Graceful error handling for network issues
 */

import * as vscode from 'vscode'
import * as http from 'http'
import { existsSync } from 'fs'
import { homedir } from 'os'
import * as path from 'path'
import { SecretsManager } from '../config/secrets'
import { MCPConfigWriter } from '../config/mcp-writer'
import { resolveDatabaseConfig, type DatabaseConfig } from '../services/database-checker'

/**
 * Supported embedding providers
 */
export type EmbeddingProvider = 'ollama' | 'openai' | 'google'

/**
 * Provider configuration for QuickPick display
 */
interface ProviderOption {
  label: string
  detail: string
  value: EmbeddingProvider
}

/**
 * Timeout for Ollama detection (milliseconds)
 */
const OLLAMA_DETECTION_TIMEOUT_MS = 2000

/**
 * Key for storing provider selection in workspace state
 */
const PROVIDER_STATE_KEY = 'maproom.provider'

/**
 * Provider documentation URLs for API key help
 */
const PROVIDER_DOCS = {
  openai: 'https://platform.openai.com/api-keys',
  google: 'https://cloud.google.com/vertex-ai/docs/authentication',
} as const

/**
 * Run the setup wizard to select embedding provider
 *
 * Shows a QuickPick with three provider options. If Ollama is detected
 * running on localhost:11434, it will be marked as "Recommended".
 *
 * For OpenAI/Google providers, prompts for API credentials with password-masked
 * input and stores them securely in VSCode SecretStorage.
 *
 * The selected provider is saved to workspace state for future use.
 *
 * @param context - Extension context for state storage
 * @returns Selected provider, or undefined if user cancelled
 */
export async function runSetupWizard(
  context: vscode.ExtensionContext
): Promise<EmbeddingProvider | undefined> {
  // Detect if Ollama is running
  const ollamaRunning = await detectOllama()

  // Build QuickPick options
  const options = buildProviderOptions(ollamaRunning)

  // Show QuickPick to user
  const selected = await vscode.window.showQuickPick(options, {
    placeHolder: 'Select an embedding provider for semantic search',
    title: 'Maproom Setup - Choose Embedding Provider',
    ignoreFocusOut: true,
  })

  // User cancelled
  if (!selected) {
    return undefined
  }

  // Collect API credentials if needed
  if (selected.value !== 'ollama') {
    const credentialCollected = await collectApiCredential(context, selected.value)

    // User cancelled credential input
    if (!credentialCollected) {
      return undefined
    }
  }

  // Save selection to workspace state
  await context.workspaceState.update(PROVIDER_STATE_KEY, selected.value)

  // Write MCP configuration
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) {
    vscode.window.showErrorMessage(
      'No workspace folder open. Open a folder or workspace to configure Maproom.'
    )
    return undefined
  }

  try {
    const writer = new MCPConfigWriter()
    // Write config for both VS Code and Cursor
    await writer.registerMCPServerForAllEditors(workspaceRoot, selected.value)

    const action = await vscode.window.showInformationMessage(
      'Maproom MCP server configured! Restart your editor to activate the MCP server.',
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

  return selected.value
}

/**
 * Get the currently configured provider from workspace state
 *
 * @param context - Extension context
 * @returns Configured provider, or undefined if not set
 */
export function getConfiguredProvider(
  context: vscode.ExtensionContext
): EmbeddingProvider | undefined {
  return context.workspaceState.get<EmbeddingProvider>(PROVIDER_STATE_KEY)
}

/**
 * Build provider options for QuickPick
 *
 * @param ollamaRunning - Whether Ollama was detected running
 * @returns Array of QuickPick items
 */
function buildProviderOptions(ollamaRunning: boolean): ProviderOption[] {
  // Ollama option - mark as recommended if running
  const ollamaOption: ProviderOption = {
    label: ollamaRunning
      ? '$(zap) Ollama (Recommended)'
      : '$(zap) Ollama',
    detail: ollamaRunning
      ? 'Running locally - fast and private'
      : 'Local inference - requires Ollama installation',
    value: 'ollama',
  }

  // OpenAI option
  const openaiOption: ProviderOption = {
    label: '$(cloud) OpenAI',
    detail: 'API key required',
    value: 'openai',
  }

  // Google Vertex AI option
  const googleOption: ProviderOption = {
    label: '$(cloud) Google Vertex AI',
    detail: 'API key required',
    value: 'google',
  }

  // Return options with Ollama first if running, otherwise alphabetical
  return ollamaRunning
    ? [ollamaOption, openaiOption, googleOption]
    : [ollamaOption, openaiOption, googleOption]
}

/**
 * Detect if Ollama is running on localhost:11434
 *
 * Performs a simple HTTP GET request with a 2-second timeout.
 * Returns true if any response received (indicates Ollama is running).
 * Returns false if timeout or connection error.
 *
 * @returns Promise resolving to true if Ollama detected, false otherwise
 */
export async function detectOllama(): Promise<boolean> {
  return new Promise((resolve) => {
    const timeout = setTimeout(() => {
      // Timeout - Ollama not detected
      req.destroy()
      resolve(false)
    }, OLLAMA_DETECTION_TIMEOUT_MS)

    const req = http.get('http://localhost:11434', (res) => {
      // Got response - Ollama is running
      clearTimeout(timeout)
      resolve(true)
      res.resume() // Drain response
    })

    req.on('error', () => {
      // Connection error - Ollama not running
      clearTimeout(timeout)
      resolve(false)
    })

    req.end()
  })
}

/**
 * Collect API credential for a provider
 *
 * Shows password-masked InputBox to collect API key from user.
 * Provides helpful prompts with links to provider documentation.
 *
 * SECURITY: Input is password-masked and stored in encrypted SecretStorage.
 *
 * @param context - Extension context for secret storage
 * @param provider - Provider requiring credentials (openai or google)
 * @returns true if credential collected, false if user cancelled
 */
async function collectApiCredential(
  context: vscode.ExtensionContext,
  provider: 'openai' | 'google'
): Promise<boolean> {
  const secretsManager = new SecretsManager(context.secrets)

  // Check if credential already exists
  const hasExisting = await secretsManager.hasApiKey(provider)

  // Build prompt based on provider
  let prompt: string
  let placeholder: string

  if (provider === 'openai') {
    prompt = hasExisting
      ? 'OpenAI API Key (leave empty to keep existing)'
      : `OpenAI API Key (get yours at ${PROVIDER_DOCS.openai})`
    placeholder = 'sk-...'
  } else {
    // google
    prompt = hasExisting
      ? 'Google API Key (leave empty to keep existing)'
      : `Google API Key (get yours at ${PROVIDER_DOCS.google})`
    placeholder = 'Your Google API key'
  }

  // Show password-masked input box
  const apiKey = await vscode.window.showInputBox({
    prompt,
    placeHolder: placeholder,
    password: true, // Mask input for security
    ignoreFocusOut: true,
    validateInput: (value: string) => {
      // Allow empty if credential already exists (keep existing)
      if (hasExisting && value.trim() === '') {
        return null
      }

      // Require non-empty input for new credentials
      if (value.trim() === '') {
        return 'API key cannot be empty'
      }

      return null
    },
  })

  // User cancelled
  if (apiKey === undefined) {
    return false
  }

  // If empty and credential exists, keep existing (no-op)
  if (apiKey.trim() === '' && hasExisting) {
    return true
  }

  // Store new credential
  await secretsManager.storeApiKey(provider, apiKey.trim())

  return true
}

/**
 * Register setup wizard command
 *
 * Adds "Maproom: Setup" command to command palette.
 * Allows users to re-run the setup wizard to change provider.
 *
 * @param context - Extension context
 */
export function registerSetupCommand(context: vscode.ExtensionContext): void {
  const setupCommand = vscode.commands.registerCommand('maproom.setup', async () => {
    const provider = await runSetupWizard(context)

    if (provider) {
      vscode.window.showInformationMessage(
        `Maproom: Configured to use ${provider.toUpperCase()} for embeddings`
      )
    }
  })

  context.subscriptions.push(setupCommand)
}

/**
 * Run SQLite-specific setup flow
 *
 * Handles SQLite database detection and configuration:
 * - Auto-detects existing database at default path (~/.maproom/maproom.db)
 * - Offers to use existing database or choose different file
 * - Shows guidance when no database exists
 *
 * @param config - Current database configuration
 * @returns true if setup completed, false if user cancelled
 */
export async function runSqliteSetup(config: DatabaseConfig): Promise<boolean> {
  const defaultPath = path.join(homedir(), '.maproom', 'maproom.db')
  const defaultExists = existsSync(defaultPath)

  // Check if custom path is already configured and exists
  if (config.path && config.path !== defaultPath && existsSync(config.path)) {
    // Custom path already configured and exists - nothing to do
    return true
  }

  if (defaultExists) {
    // Offer to use existing database
    const action = await vscode.window.showInformationMessage(
      `Found existing Maproom index at ${defaultPath}`,
      'Use Existing',
      'Choose Different',
      'Cancel'
    )

    if (action === 'Use Existing') {
      // Default path, no settings change needed (empty sqlitePath = default)
      return true
    } else if (action === 'Choose Different') {
      return await promptForSqlitePath()
    }
    // Cancel
    return false
  } else {
    // No existing database - guide user
    return await showNoSqliteGuidance()
  }
}

/**
 * Prompt user to select a SQLite database file
 *
 * Shows file picker dialog filtered to common SQLite file extensions.
 * Updates the maproom.database.sqlitePath setting with selected path.
 *
 * @returns true if file selected, false if cancelled
 */
export async function promptForSqlitePath(): Promise<boolean> {
  const result = await vscode.window.showOpenDialog({
    canSelectFiles: true,
    canSelectFolders: false,
    canSelectMany: false,
    filters: {
      'SQLite Database': ['db', 'sqlite', 'sqlite3'],
    },
    title: 'Select Maproom SQLite Database',
  })

  if (result && result[0]) {
    const selectedPath = result[0].fsPath

    try {
      // Update settings
      const config = vscode.workspace.getConfiguration('maproom.database')
      await config.update('sqlitePath', selectedPath, vscode.ConfigurationTarget.Global)

      vscode.window.showInformationMessage(`Maproom will use: ${selectedPath}`)
      return true
    } catch (error) {
      vscode.window.showErrorMessage(
        `Failed to update settings: ${error instanceof Error ? error.message : String(error)}`
      )
      return false
    }
  }

  return false
}

/**
 * Show guidance when no SQLite database exists
 *
 * Provides options to help user create or find a database:
 * - Copy scan command to clipboard
 * - Open terminal with scan command hint
 * - Choose existing file
 *
 * @returns true if user took action, false if cancelled
 */
export async function showNoSqliteGuidance(): Promise<boolean> {
  const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '/path/to/your/repo'

  const action = await vscode.window.showWarningMessage(
    'No Maproom index found. Create one to enable code search.',
    { modal: false, detail: 'Run crewchief-maproom scan in your terminal to index a repository.' },
    'Copy Scan Command',
    'Open Terminal',
    'Choose Existing File'
  )

  if (action === 'Copy Scan Command') {
    const command = `crewchief-maproom scan ${workspacePath}`
    await vscode.env.clipboard.writeText(command)
    vscode.window.showInformationMessage('Scan command copied to clipboard')
    return true
  } else if (action === 'Open Terminal') {
    const terminal = vscode.window.createTerminal('Maproom Setup')
    terminal.show()
    terminal.sendText(`# Run: crewchief-maproom scan ${workspacePath}`, false)
    return true
  } else if (action === 'Choose Existing File') {
    return await promptForSqlitePath()
  }

  return false
}

/**
 * Run database setup based on current configuration
 *
 * Entry point for database-aware setup that routes to appropriate
 * flow based on configured database provider.
 *
 * @returns true if setup completed successfully, false if cancelled
 */
export async function runDatabaseSetup(): Promise<boolean> {
  const dbConfig = resolveDatabaseConfig()

  if (dbConfig.type === 'sqlite') {
    return await runSqliteSetup(dbConfig)
  }

  // PostgreSQL mode - no additional setup needed here
  // (Docker setup is handled in extension.ts)
  return true
}
