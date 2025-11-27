/**
 * Ollama model management
 *
 * Provides orchestration for ensuring required embedding models are available
 * before the watch process starts. Handles model verification, automatic
 * download with progress notifications, and error handling.
 */

import * as vscode from 'vscode'
import { OllamaClient, OllamaApiError, createOllamaClient } from './client'
import { OllamaNotRunningError, ModelPullError, ModelCheckError } from './errors'

/** Default embedding model used by the extension */
export const DEFAULT_EMBEDDING_MODEL = 'nomic-embed-text'

/** URL for Ollama installation */
export const OLLAMA_INSTALL_URL = 'https://ollama.ai'

/**
 * Result of a model check operation
 */
export interface ModelCheckResult {
  /** Whether the model exists */
  exists: boolean
  /** Error message if check failed */
  error?: string
}

/**
 * Options for ensureOllamaModel
 */
export interface EnsureModelOptions {
  /** Progress callback for download updates */
  onProgress?: (message: string) => void
  /** Skip VSCode progress notification (for testing) */
  skipNotification?: boolean
}

/**
 * Ensure that a specific Ollama model is available
 *
 * This function:
 * 1. Checks if Ollama is running
 * 2. Checks if the model exists
 * 3. If not, downloads the model with progress notification
 *
 * @param modelName - Name of the model to ensure (default: nomic-embed-text)
 * @param options - Configuration options
 * @throws OllamaNotRunningError if Ollama service is not accessible
 * @throws ModelCheckError if model existence cannot be verified
 * @throws ModelPullError if model download fails
 *
 * @example
 * ```typescript
 * try {
 *   await ensureOllamaModel('nomic-embed-text')
 * } catch (error) {
 *   if (error instanceof OllamaNotRunningError) {
 *     // Show "Install Ollama" button
 *   }
 * }
 * ```
 */
export async function ensureOllamaModel(
  modelName: string = DEFAULT_EMBEDDING_MODEL,
  options: EnsureModelOptions = {}
): Promise<void> {
  const client = createOllamaClient()

  // Check if Ollama is running
  if (!(await client.isRunning())) {
    throw new OllamaNotRunningError()
  }

  // Check if model already exists
  let hasModel: boolean
  try {
    hasModel = await client.hasModel(modelName)
  } catch (error) {
    throw new ModelCheckError(
      modelName,
      error instanceof Error ? error : undefined
    )
  }

  // If model exists, we're done
  if (hasModel) {
    return
  }

  // Pull the model with progress
  if (options.skipNotification) {
    // Direct pull without VSCode notification (for testing)
    await pullModelWithProgress(client, modelName, options.onProgress)
  } else {
    // Pull with VSCode progress notification
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: `Downloading embedding model "${modelName}"...`,
        cancellable: false,
      },
      async (progress) => {
        await pullModelWithProgress(client, modelName, (message) => {
          progress.report({ message })
          options.onProgress?.(message)
        })
      }
    )
  }
}

/**
 * Pull a model with progress tracking
 *
 * @internal
 */
async function pullModelWithProgress(
  client: OllamaClient,
  modelName: string,
  onProgress?: (message: string) => void
): Promise<void> {
  try {
    await client.pullModel(modelName, (pullProgress) => {
      // Format progress message
      let message = pullProgress.status
      if (pullProgress.total && pullProgress.completed) {
        const percent = Math.round((pullProgress.completed / pullProgress.total) * 100)
        message = `${pullProgress.status} (${percent}%)`
      }
      onProgress?.(message)
    })
  } catch (error) {
    throw new ModelPullError(
      modelName,
      error instanceof Error ? error : undefined
    )
  }
}

/**
 * Show error notification for Ollama not running
 *
 * Displays an error message with options to install or retry.
 *
 * @param onRetry - Callback to invoke when user clicks "Retry"
 * @returns The action selected by the user, or undefined if dismissed
 */
export async function showOllamaNotRunningError(
  onRetry?: () => void
): Promise<string | undefined> {
  const action = await vscode.window.showErrorMessage(
    'Ollama is not running. Please start Ollama or install it.',
    'Install Ollama',
    'Retry'
  )

  if (action === 'Install Ollama') {
    await vscode.env.openExternal(vscode.Uri.parse(OLLAMA_INSTALL_URL))
  } else if (action === 'Retry') {
    onRetry?.()
  }

  return action
}

/**
 * Show error notification for model pull failure
 *
 * @param modelName - Name of the model that failed to pull
 * @param onRetry - Callback to invoke when user clicks "Retry"
 * @returns The action selected by the user, or undefined if dismissed
 */
export async function showModelPullError(
  modelName: string,
  onRetry?: () => void
): Promise<string | undefined> {
  const action = await vscode.window.showErrorMessage(
    `Failed to download embedding model "${modelName}". Check your network connection.`,
    'Retry',
    'Dismiss'
  )

  if (action === 'Retry') {
    onRetry?.()
  }

  return action
}

/**
 * Check if a model is available without throwing errors
 *
 * Use this for non-critical checks where you want to handle
 * missing models gracefully.
 *
 * @param modelName - Name of the model to check
 * @returns Result object with exists flag and optional error
 */
export async function checkModelAvailability(
  modelName: string = DEFAULT_EMBEDDING_MODEL
): Promise<ModelCheckResult> {
  const client = createOllamaClient()

  try {
    if (!(await client.isRunning())) {
      return { exists: false, error: 'Ollama is not running' }
    }

    const exists = await client.hasModel(modelName)
    return { exists }
  } catch (error) {
    return {
      exists: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    }
  }
}
