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
 * - Re-runnable via command palette
 * - Graceful error handling for network issues
 */

import * as vscode from 'vscode'
import * as http from 'http'

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
 * Run the setup wizard to select embedding provider
 *
 * Shows a QuickPick with three provider options. If Ollama is detected
 * running on localhost:11434, it will be marked as "Recommended".
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

  // Save selection to workspace state
  await context.workspaceState.update(PROVIDER_STATE_KEY, selected.value)

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
