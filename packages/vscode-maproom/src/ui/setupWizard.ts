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
import { SecretsManager } from '../config/secrets'

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
