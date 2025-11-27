/**
 * VSCode extension entry point for Maproom Semantic Search
 *
 * Integrates core components with fast activation pattern:
 * - ProcessOrchestrator: Manages unified watch process (file monitoring, branch detection)
 * - StatusBarManager: Displays real-time indexing status in status bar
 * - OllamaClient: Manages embedding model availability (ollama provider only)
 *
 * Extension lifecycle:
 * 1. activate() - Called when extension loads (onStartupFinished)
 *    - Create output channel and status bar immediately (<500ms)
 *    - Register commands synchronously
 *    - Return quickly (FAST ACTIVATION)
 *    - Background: Check/pull Ollama model (ollama provider only)
 *    - Background: Run startup reconciliation
 *    - Background: Start unified watch process
 *    - Background: Update status bar to "Watching" state
 * 2. deactivate() - Called when extension unloads
 *    - Stop watch process
 *    - Cleanup resources
 *
 * Performance:
 * - activate() completes in <500ms (doesn't block VSCode startup)
 * - All heavy initialization happens in background with progress UI
 * - Status bar shows "Starting..." immediately, updates to "Watching" when ready
 */

import * as vscode from 'vscode'
import * as path from 'path'
import * as fs from 'fs'
import { ProcessOrchestrator } from './process/orchestrator'
import { StatusBarManager } from './ui/statusBar'
import { reconcileChanges } from './process/reconcile'
import {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
  showNoSqliteGuidance,
} from './ui/setupWizard'
import { SecretsManager } from './config/secrets'
import {
  resolveDatabaseConfig,
  checkDatabaseAvailable,
  getDatabaseUrl,
} from './services/database-checker'
import {
  ensureOllamaModel,
  showOllamaNotRunningError,
  showModelPullError,
  OllamaNotRunningError,
  ModelPullError,
  DEFAULT_EMBEDDING_MODEL,
} from './ollama'

/**
 * Output channel for extension logging
 */
let outputChannel: vscode.OutputChannel | undefined

/**
 * Process orchestrator for watch processes
 */
let orchestrator: ProcessOrchestrator | undefined

/**
 * Status bar manager for UI updates
 */
let statusBar: StatusBarManager | undefined

/**
 * Extension activation
 *
 * Called when extension is activated (onStartupFinished).
 * Uses fast activation pattern: completes in <500ms by deferring heavy work to background.
 *
 * Activation sequence:
 * 1. Create output channel and status bar (synchronous, fast)
 * 2. Register commands (synchronous, fast)
 * 3. Return immediately (FAST!)
 * 4. Background: Check/ensure Ollama model (ollama provider only)
 * 5. Background: Run startup reconciliation
 * 6. Background: Start watch process
 * 7. Background: Connect status bar to orchestrator
 *
 * @param context - Extension context
 */
export function activate(context: vscode.ExtensionContext): void {
  const activationStart = performance.now()
  console.log('Maproom extension activating...')

  // Step 1: Create output channel (fast, synchronous)
  outputChannel = vscode.window.createOutputChannel('Maproom')
  context.subscriptions.push(outputChannel)
  outputChannel.appendLine('Maproom extension starting...')

  // Step 2: Check for workspace folder (fast, synchronous)
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0]
  if (!workspaceFolder) {
    const message = 'Maproom requires an open workspace folder'
    outputChannel.appendLine(`ERROR: ${message}`)
    vscode.window.showErrorMessage(message)
    return
  }

  // Step 3: Create status bar manager (fast, synchronous)
  // Shows "Starting..." state immediately
  statusBar = new StatusBarManager(context)
  statusBar.setState('starting')
  context.subscriptions.push(statusBar)
  outputChannel.appendLine('Status bar created (Starting...)')

  // Step 4: Register commands (fast, synchronous)
  const showOutputCommand = vscode.commands.registerCommand('maproom.showOutput', () => {
    outputChannel?.show()
  })
  context.subscriptions.push(showOutputCommand)

  // Register restart watchers command
  const restartWatchersCommand = vscode.commands.registerCommand('maproom.restartWatchers', async () => {
    if (orchestrator) {
      try {
        await orchestrator.restartWatchers()
      } catch (error: any) {
        vscode.window.showErrorMessage(`Failed to restart watchers: ${error.message}`)
      }
    } else {
      vscode.window.showWarningMessage('Maproom watchers are not running')
    }
  })
  context.subscriptions.push(restartWatchersCommand)

  // Register show status command
  const showStatusCommand = vscode.commands.registerCommand('maproom.showStatus', () => {
    if (orchestrator) {
      const status = orchestrator.getStatus()
      const statusLines: string[] = ['Maproom Process Status:', '']

      for (const [name, state] of status) {
        const statusText = state.running ? '✓ Running' : state.crashed ? '✗ Crashed' : '○ Stopped'
        statusLines.push(`${name}: ${statusText}`)
        if (state.exitCode !== undefined) {
          statusLines.push(`  Exit code: ${state.exitCode}`)
        }
      }

      outputChannel?.show()
      outputChannel?.appendLine('\n' + statusLines.join('\n'))
    } else {
      vscode.window.showInformationMessage('Maproom orchestrator not initialized')
    }
  })
  context.subscriptions.push(showStatusCommand)

  // Register setup wizard command
  registerSetupCommand(context)
  outputChannel.appendLine('Commands registered')

  // Step 5: Check for provider configuration (fast, synchronous)
  const configuredProvider = getConfiguredProvider(context)
  if (!configuredProvider) {
    // No provider configured - show setup wizard
    outputChannel.appendLine('No provider configured, showing setup wizard...')
    void runFirstTimeSetup(context, workspaceFolder.uri.fsPath)
  } else {
    // Provider already configured - proceed with normal initialization
    outputChannel.appendLine(`Provider configured: ${configuredProvider}`)
    void initializeServices(context, workspaceFolder.uri.fsPath)
  }

  // Step 6: Check and prompt for MCP setup if needed (fast, asynchronous)
  void checkAndPromptForSetup(context)

  // Step 7: Set database config on status bar for tooltip display
  const dbConfig = resolveDatabaseConfig()
  statusBar.setDatabaseConfig(dbConfig)

  // Step 8: Log activation time and return (FAST ACTIVATION - under 500ms)
  const activationTime = performance.now() - activationStart
  outputChannel.appendLine(`Maproom: Activated in ${activationTime.toFixed(0)}ms (${dbConfig.type} mode)`)
  if (activationTime > 500) {
    outputChannel.appendLine(`Warning: Activation exceeded 500ms target`)
  }
  console.log(`Maproom extension activated in ${activationTime.toFixed(0)}ms (background initialization starting...)`)
}

/**
 * First-time setup flow
 *
 * Runs the setup wizard to select embedding provider, then proceeds with
 * normal initialization (Ollama model check, reconciliation, watch).
 * If user cancels setup, initialization is skipped.
 *
 * @param context - Extension context
 * @param workspaceRoot - Workspace root path
 */
async function runFirstTimeSetup(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  try {
    // Run setup wizard
    const provider = await runSetupWizard(context)

    if (!provider) {
      // User cancelled setup
      outputChannel?.appendLine('Setup cancelled by user')
      vscode.window.showInformationMessage(
        'Maproom setup cancelled. Run "Maproom: Setup" to configure later.'
      )
      statusBar?.setState('idle')
      return
    }

    // Setup complete - show success message
    outputChannel?.appendLine(`Setup complete: ${provider} selected`)
    vscode.window.showInformationMessage(
      `Maproom configured to use ${provider.toUpperCase()} for embeddings`
    )

    // Proceed with normal initialization
    await initializeServices(context, workspaceRoot)
  } catch (error: any) {
    const errorMessage = `Setup failed: ${error.message}`
    outputChannel?.appendLine(`ERROR: ${errorMessage}`)
    console.error(errorMessage, error)

    // Show error notification
    vscode.window.showErrorMessage(errorMessage)

    // Set status bar to error state
    statusBar?.setState('error', error.message)
  }
}


/**
 * Background service initialization
 *
 * Runs after activate() returns. New simplified flow:
 * 1. Check/ensure Ollama model (ollama provider only)
 * 2. Run startup reconciliation
 * 3. Start unified watch process
 *
 * Handles errors gracefully without crashing the extension.
 *
 * @param context - Extension context
 * @param workspaceRoot - Workspace root path
 */
async function initializeServices(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  try {
    // Get configured provider
    const provider = getConfiguredProvider(context)
    if (!provider) {
      throw new Error('No embedding provider configured. Run "Maproom: Setup" to configure.')
    }

    // Resolve database configuration (SQLite only now)
    const dbConfig = resolveDatabaseConfig()
    outputChannel?.appendLine(`Database mode: ${dbConfig.type}`)

    // Step 1: Ensure Ollama model (ONLY for ollama provider)
    if (provider === 'ollama') {
      outputChannel?.appendLine('Checking Ollama embedding model...')
      try {
        await ensureOllamaModel(DEFAULT_EMBEDDING_MODEL, {
          onProgress: (msg) => outputChannel?.appendLine(`Ollama: ${msg}`),
        })
        outputChannel?.appendLine('Ollama model ready')
      } catch (error) {
        if (error instanceof OllamaNotRunningError) {
          await showOllamaNotRunningError(() => {
            // Retry callback
            void initializeServices(context, workspaceRoot)
          })
          statusBar?.setState('error', 'Ollama not running')
          return
        } else if (error instanceof ModelPullError) {
          await showModelPullError(DEFAULT_EMBEDDING_MODEL, () => {
            // Retry callback
            void initializeServices(context, workspaceRoot)
          })
          statusBar?.setState('error', 'Model pull failed')
          return
        }
        throw error
      }
    } else {
      outputChannel?.appendLine(`Using ${provider} provider - skipping Ollama model check`)
    }

    // Step 2: Check SQLite database availability
    outputChannel?.appendLine(`Checking database at ${dbConfig.path || dbConfig.url}...`)
    const dbAvailable = await checkDatabaseAvailable(dbConfig)

    if (!dbAvailable) {
      // Database not found - show guidance and enter degraded mode
      outputChannel?.appendLine('SQLite database not found - run crewchief-maproom scan to create index')
      await showNoSqliteGuidance()
      statusBar?.setState('idle')
      return
    }
    outputChannel?.appendLine('Database found')

    // Step 3: Run startup reconciliation
    statusBar?.setState('reconciling')
    outputChannel?.appendLine('Running startup reconciliation...')

    const reconcileResult = await reconcileChanges(context, {
      extensionRoot: context.extensionPath,
      databaseUrl: getDatabaseUrl(dbConfig),
      onProgress: (msg) => {
        outputChannel?.appendLine(`Reconcile: ${msg}`)
        statusBar?.setState('reconciling', msg)
      },
    })

    if (reconcileResult.performed) {
      outputChannel?.appendLine(
        `Reconciliation complete: ${reconcileResult.filesReconciled} files indexed`
      )
    } else if (reconcileResult.error) {
      outputChannel?.appendLine(`Reconciliation skipped: ${reconcileResult.error}`)
    } else {
      outputChannel?.appendLine('Reconciliation skipped: no changes since last run')
    }

    // Step 4: Create and start process orchestrator
    outputChannel?.appendLine('Creating process orchestrator...')

    // Create secrets manager for API key resolution
    const secretsManager = new SecretsManager(context.secrets)

    orchestrator = new ProcessOrchestrator(outputChannel!, {
      extensionRoot: context.extensionPath,
      workspaceRoot,
      databaseUrl: getDatabaseUrl(dbConfig),
      secretsManager,
      provider,
    })

    // Start unified watch process
    outputChannel?.appendLine('Starting watch process...')
    await orchestrator.startWatching()
    outputChannel?.appendLine('Watch process started successfully')

    // Step 5: Connect status bar to orchestrator
    outputChannel?.appendLine('Connecting status bar to orchestrator...')
    statusBar?.connectOrchestrator(orchestrator)
    statusBar?.setState('watching')
    outputChannel?.appendLine('Status bar connected (Watching)')

    // Register orchestrator for cleanup
    context.subscriptions.push({
      dispose: () => void orchestrator?.stopWatching(),
    })

    // Success!
    outputChannel?.appendLine('Maproom services initialized successfully')
    console.log('Maproom background initialization complete')
  } catch (error: any) {
    const errorMessage = `Failed to initialize Maproom services: ${error.message}`
    outputChannel?.appendLine(`ERROR: ${errorMessage}`)
    console.error(errorMessage, error)

    // Update status bar to error state
    statusBar?.setState('error', error.message)

    // Show error notification with appropriate action
    vscode.window.showErrorMessage(errorMessage)

    // Cleanup partial initialization
    await cleanup()
  }
}


/**
 * Check and prompt for MCP setup if needed
 *
 * Shows a one-time prompt to run setup if the MCP configuration file is missing.
 * Uses workspace state to ensure the prompt is only shown once per workspace.
 *
 * @param context - Extension context
 */
async function checkAndPromptForSetup(context: vscode.ExtensionContext): Promise<void> {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) {
    return // No workspace, skip prompt
  }

  const mcpConfigPath = path.join(workspaceRoot, '.vscode', 'mcp.json')
  const configExists = fs.existsSync(mcpConfigPath)

  if (!configExists) {
    const workspaceState = context.workspaceState
    const hasPrompted = workspaceState.get<boolean>('maproom.hasPromptedSetup', false)

    if (!hasPrompted) {
      const action = await vscode.window.showInformationMessage(
        'Maproom MCP server not configured. Run setup to enable semantic code search?',
        'Run Setup',
        'Remind Me Later'
      )

      await workspaceState.update('maproom.hasPromptedSetup', true)

      if (action === 'Run Setup') {
        await vscode.commands.executeCommand('maproom.setup')
      }
    }
  }
}

/**
 * Extension deactivation
 *
 * Called when extension is deactivated (e.g., VSCode shutdown, extension reload).
 * Performs graceful cleanup of all resources.
 */
export async function deactivate(): Promise<void> {
  console.log('Maproom extension deactivating...')
  outputChannel?.appendLine('Deactivating extension...')

  await cleanup()

  outputChannel?.appendLine('Maproom extension deactivated')
  console.log('Maproom extension deactivated')
}

/**
 * Cleanup resources
 *
 * Helper function to stop processes and cleanup resources.
 * Can be called from deactivate() or on background initialization failure.
 *
 * Safe to call even if services aren't fully initialized.
 */
async function cleanup(): Promise<void> {
  try {
    // Stop watch process if it was started
    if (orchestrator) {
      outputChannel?.appendLine('Stopping watch process...')
      await orchestrator.stopWatching()
      outputChannel?.appendLine('Watch process stopped')
      orchestrator = undefined
    }

    // Update status bar to idle if present
    if (statusBar) {
      statusBar.setState('idle')
    }

    // Status bar and output channel are disposed via context.subscriptions
  } catch (error: any) {
    outputChannel?.appendLine(`ERROR during cleanup: ${error.message}`)
    console.error('Error during cleanup:', error)
  }
}
