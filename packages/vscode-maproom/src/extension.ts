/**
 * VSCode extension entry point for Maproom Semantic Search
 *
 * Integrates core components with fast activation pattern:
 * - DockerManager: Manages PostgreSQL container lifecycle
 * - ProcessOrchestrator: Manages watch processes (file monitoring, branch monitoring)
 * - StatusBarManager: Displays real-time indexing status in status bar
 *
 * Extension lifecycle:
 * 1. activate() - Called when extension loads (onStartupFinished)
 *    - Create output channel and status bar immediately (<500ms)
 *    - Register commands synchronously
 *    - Return quickly (FAST ACTIVATION)
 *    - Background: Start Docker services asynchronously
 *    - Background: Start watch processes after Docker healthy
 *    - Background: Update status bar to "Watching" state
 * 2. deactivate() - Called when extension unloads
 *    - Stop watch processes
 *    - Optionally stop PostgreSQL container
 *    - Cleanup resources
 *
 * Performance:
 * - activate() completes in <500ms (doesn't block VSCode startup)
 * - Docker and process initialization happens in background with progress UI
 * - Status bar shows "Starting..." immediately, updates to "Watching" when ready
 */

import * as vscode from 'vscode'
import * as path from 'path'
import * as fs from 'fs'
import { ProcessOrchestrator } from './process/orchestrator'
import { StatusBarManager } from './ui/statusBar'
import {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
  showNoSqliteGuidance,
} from './ui/setupWizard'
import { SecretsManager } from './config/secrets'
import { runInitialScan } from './process/scan'
import {
  checkPostgresAvailable,
  getPostgresUnavailableMessage,
  DEFAULT_POSTGRES_CONFIG,
  getPostgresConfigFromSettings,
  getPostgresUrl,
} from './services/postgres-checker'
import {
  resolveDatabaseConfig,
  checkDatabaseAvailable,
  getDatabaseUnavailableMessage,
  getDatabaseUrl,
  type DatabaseConfig,
} from './services/database-checker'
import { DockerManager } from './docker/manager'

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
 * 4. Background: Start Docker services with progress UI
 * 5. Background: Start watch processes after Docker ready
 * 6. Background: Connect status bar to orchestrator
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
 * Runs the setup wizard to select embedding provider, then starts Docker services
 * (for PostgreSQL mode) or checks SQLite availability, and triggers initial workspace scan.
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

    // Resolve database configuration
    const dbConfig = resolveDatabaseConfig()
    outputChannel?.appendLine(`Database mode: ${dbConfig.type}`)

    // Branch based on database type
    if (dbConfig.type === 'postgresql') {
      // PostgreSQL mode: Start Docker services before checking availability
      await ensureDockerRunning(context, provider)
      await ensurePostgresAvailable()

      // Run initial scan after setup completes
      await runInitialWorkspaceScan(context, workspaceRoot)

      // After scan completes, start watch processes
      await startWatchProcesses(context, workspaceRoot)
    } else {
      // SQLite mode: Check database availability (no Docker needed)
      const sqliteAvailable = await ensureSqliteAvailable(dbConfig)

      if (sqliteAvailable) {
        // Database exists - start watch processes
        await startWatchProcesses(context, workspaceRoot)
      } else {
        // Database not found - show guidance and enter degraded mode
        outputChannel?.appendLine('SQLite database not found - run crewchief-maproom scan to create index')
        statusBar?.setState('idle')
      }
    }
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
 * Ensure Docker services are running
 *
 * Multi-workspace behavior:
 * - Multiple VSCode workspaces share the same Docker containers
 * - DockerManager.ensureServicesRunning() is idempotent (safe to call multiple times)
 * - Containers remain running until last workspace closes
 * - Each workspace registers its own cleanup handler
 *
 * @param context - Extension context
 * @param provider - Embedding provider selected by user ('ollama', 'openai', 'google')
 * @throws Error if Docker services cannot be started
 */
async function ensureDockerRunning(
  context: vscode.ExtensionContext,
  provider: string
): Promise<void> {
  const dockerManager = new DockerManager(outputChannel!)

  try {
    // Build environment variables for docker compose
    const envVars: Record<string, string> = {
      MAPROOM_EMBEDDING_PROVIDER: provider,
    }

    // Get API key from secrets if needed (not for ollama)
    if (provider !== 'ollama') {
      const secretsManager = new SecretsManager(context.secrets)
      const apiKey = await secretsManager.getApiKey(provider as any)

      if (apiKey) {
        // Map to environment variable names expected by docker-compose.yml
        if (provider === 'openai') {
          envVars.OPENAI_API_KEY = apiKey
        } else if (provider === 'google') {
          envVars.GOOGLE_APPLICATION_CREDENTIALS = apiKey
        }
      }
    }

    outputChannel?.appendLine(`Starting Docker services for provider: ${provider}`)
    outputChannel?.appendLine(`Environment: MAPROOM_EMBEDDING_PROVIDER=${provider}`)

    await dockerManager.ensureServicesRunning()
    context.subscriptions.push({
      dispose: () => void dockerManager.stop()
    })
  } catch (error: any) {
    // Error message templates:
    // - "Maproom requires Docker Desktop to be running." (Docker not running)
    // - "Failed to start Docker services: [specific error]" (other errors)
    const action = await vscode.window.showErrorMessage(
      'Maproom requires Docker Desktop to be running.',
      'Open Docker Desktop',
      'Show Logs',
      'Retry'
    )

    if (action === 'Show Logs') outputChannel?.show()
    throw new Error(`Failed to start Docker services: ${error.message}`)
  }
}

/**
 * Background service initialization
 *
 * Runs after activate() returns. Shows progress notification to user.
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

    // Resolve database configuration
    const dbConfig = resolveDatabaseConfig()
    outputChannel?.appendLine(`Database mode: ${dbConfig.type}`)

    // Show progress notification
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'Maproom',
        cancellable: false,
      },
      async (progress) => {
        // Branch based on database type
        if (dbConfig.type === 'postgresql') {
          // PostgreSQL mode: Start Docker services and check availability
          progress.report({ message: 'Starting Docker services...' })
          await ensureDockerRunning(context, provider)

          progress.report({ message: 'Checking PostgreSQL...' })
          await ensurePostgresAvailable()
        } else {
          // SQLite mode: Check database file exists (no Docker needed)
          progress.report({ message: 'Checking SQLite database...' })
          const sqliteAvailable = await ensureSqliteAvailable(dbConfig)

          if (!sqliteAvailable) {
            // Database not found - update status and continue with degraded mode
            statusBar?.setState('idle')
            outputChannel?.appendLine('SQLite database not found - extension running in degraded mode')
            return // Exit progress handler but don't throw
          }
        }

        // Step 3: Create process orchestrator
        progress.report({ message: 'Starting watch processes...' })
        outputChannel?.appendLine('Creating process orchestrator...')

        // Create secrets manager
        const secretsManager = new SecretsManager(context.secrets)

        // Use databaseUrlOverride for both SQLite and PostgreSQL modes
        orchestrator = new ProcessOrchestrator(outputChannel!, {
          extensionRoot: context.extensionPath,
          workspaceRoot,
          databaseUrlOverride: getDatabaseUrl(dbConfig),
          secretsManager,
          provider,
        })

        // Step 4: Start watch processes
        outputChannel?.appendLine('Starting watch processes...')
        await orchestrator.startWatching()
        outputChannel?.appendLine('Watch processes started successfully')

        // Step 5: Connect status bar to orchestrator
        progress.report({ message: 'Initializing status bar...' })
        outputChannel?.appendLine('Connecting status bar to orchestrator...')

        statusBar?.connectOrchestrator(orchestrator)
        statusBar?.setState('watching')

        outputChannel?.appendLine('Status bar connected (Watching)')

        // Success!
        progress.report({ message: 'Ready!' })
        outputChannel?.appendLine('Maproom services initialized successfully')
        console.log('Maproom background initialization complete')
      }
    )
  } catch (error: any) {
    const errorMessage = `Failed to initialize Maproom services: ${error.message}`
    outputChannel?.appendLine(`ERROR: ${errorMessage}`)
    console.error(errorMessage, error)

    // Update status bar to error state
    statusBar?.setState('error', error.message)

    // Show error notification
    vscode.window.showErrorMessage(errorMessage)

    // Cleanup partial initialization
    await cleanup()
  }
}

/**
 * Ensure PostgreSQL is available
 *
 * Checks if PostgreSQL is listening at the configured host/port.
 * Throws error with helpful message if not available.
 *
 * @throws Error if PostgreSQL is not available
 */
async function ensurePostgresAvailable(): Promise<void> {
  const config = getPostgresConfigFromSettings()
  outputChannel?.appendLine(`Checking PostgreSQL availability at ${config.host}:${config.port}...`)

  const available = await checkPostgresAvailable(config)

  if (!available) {
    const message = getPostgresUnavailableMessage()
    outputChannel?.appendLine(`ERROR: ${message}`)
    throw new Error(message)
  }

  outputChannel?.appendLine('PostgreSQL is available and ready')
}

/**
 * Ensure SQLite database is available
 *
 * Checks if the SQLite database file exists at the configured path.
 * Shows error notification with guidance if not available, but allows
 * extension to activate (graceful degradation).
 *
 * @param config - Database configuration from resolveDatabaseConfig()
 * @returns true if database is available, false otherwise
 */
async function ensureSqliteAvailable(config: DatabaseConfig): Promise<boolean> {
  outputChannel?.appendLine(`Checking SQLite database at ${config.path}...`)

  const available = await checkDatabaseAvailable(config)

  if (!available) {
    const message = getDatabaseUnavailableMessage(config)
    outputChannel?.appendLine(`WARNING: ${message}`)

    // Use enhanced SQLite guidance with file picker, copy command, and terminal options
    await showNoSqliteGuidance()

    // Return false but don't throw - allow graceful degradation
    return false
  }

  outputChannel?.appendLine(`SQLite database found at ${config.path}`)
  return true
}

/**
 * Run initial workspace scan
 *
 * Triggers a one-time scan of the workspace to build the initial semantic index.
 * Shows progress notification with file counts and percentage.
 *
 * @param context - Extension context
 * @param workspaceRoot - Workspace root path
 */
async function runInitialWorkspaceScan(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  if (!statusBar) {
    throw new Error('Status bar not initialized')
  }

  outputChannel?.appendLine('Running initial workspace scan...')

  // Get configured provider for environment variables
  const provider = getConfiguredProvider(context)
  const secretsManager = new SecretsManager(context.secrets)

  // Build environment variables with credentials
  const env: NodeJS.ProcessEnv = { ...process.env }
  if (provider) {
    const credentialEnv = await secretsManager.getEnvironmentVars(provider)
    Object.assign(env, credentialEnv)
  }

  // Get database URL from database-checker (supports both SQLite and PostgreSQL)
  const dbConfig = resolveDatabaseConfig()
  const databaseUrl = getDatabaseUrl(dbConfig)

  // Run scan with progress notification
  const filesIndexed = await runInitialScan({
    extensionRoot: context.extensionPath,
    workspaceRoot,
    databaseUrl,
    outputChannel: outputChannel!,
    statusBarManager: statusBar,
    env,
  })

  outputChannel?.appendLine(`Initial scan complete: ${filesIndexed} files indexed`)
}

/**
 * Start watch processes
 *
 * Starts file and branch watch processes after initial scan completes.
 *
 * @param context - Extension context
 * @param workspaceRoot - Workspace root path
 */
async function startWatchProcesses(
  context: vscode.ExtensionContext,
  workspaceRoot: string
): Promise<void> {
  outputChannel?.appendLine('Creating process orchestrator...')

  // Resolve database configuration and use databaseUrlOverride
  const dbConfig = resolveDatabaseConfig()

  // Get configured provider and create secrets manager
  const provider = getConfiguredProvider(context)
  const secretsManager = new SecretsManager(context.secrets)

  orchestrator = new ProcessOrchestrator(outputChannel!, {
    extensionRoot: context.extensionPath,
    workspaceRoot,
    databaseUrlOverride: getDatabaseUrl(dbConfig),
    secretsManager,
    provider,
  })

  // Start watch processes
  outputChannel?.appendLine('Starting watch processes...')
  await orchestrator.startWatching()
  outputChannel?.appendLine('Watch processes started successfully')

  // Connect status bar to orchestrator
  outputChannel?.appendLine('Connecting status bar to orchestrator...')
  statusBar?.connectOrchestrator(orchestrator)
  statusBar?.setState('watching')
  outputChannel?.appendLine('Status bar connected (Watching)')
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
    // Stop watch processes if they were started
    if (orchestrator) {
      outputChannel?.appendLine('Stopping watch processes...')
      await orchestrator.stopWatching()
      outputChannel?.appendLine('Watch processes stopped')
      orchestrator = undefined
    }

    // Update status bar to idle if present
    if (statusBar) {
      statusBar.setState('idle')
    }

    // Note: We don't stop PostgreSQL container on deactivation
    // because it may be shared across VSCode sessions.
    // Users can manually stop it with: docker compose down
    // or we could add a command: "Maproom: Stop Services"

    // Status bar and output channel are disposed via context.subscriptions
  } catch (error: any) {
    outputChannel?.appendLine(`ERROR during cleanup: ${error.message}`)
    console.error('Error during cleanup:', error)
  }
}
