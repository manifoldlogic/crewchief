import * as vscode from 'vscode'
import * as path from 'path'
import * as fs from 'fs'
import { ProcessOrchestrator } from './process/orchestrator'
import { StatusBarManager } from './ui/statusBar'
import {
  runSetupWizard,
  getConfiguredProvider,
  registerSetupCommand,
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
import { DockerManager } from './docker/manager'

// ... (keep existing imports and variable declarations)

// Helper to check if we are in SQLite mode
function isSqliteMode(): boolean {
  const config = vscode.workspace.getConfiguration('maproom')
  const provider = config.get<string>('database.provider')
  return provider === 'sqlite'
}

// ... (keep existing activate function until Step 5)

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

// ... (keep rest of activate)

/**
 * Ensure Docker services are running
 * ...
 */
async function ensureDockerRunning(
  context: vscode.ExtensionContext,
  provider: string
): Promise<void> {
  // Skip Docker check if using SQLite
  if (isSqliteMode()) {
    outputChannel?.appendLine('Using SQLite backend - skipping Docker check')
    return
  }

  const dockerManager = new DockerManager(outputChannel!)
  // ... (rest of existing ensureDockerRunning implementation)
}

/**
 * Background service initialization
 * ...
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

    // Show progress notification
    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'Maproom',
        cancellable: false,
      },
      async (progress) => {
        // Step 1: Start Docker services (skipped if sqlite)
        if (!isSqliteMode()) {
            progress.report({ message: 'Starting Docker services...' })
            await ensureDockerRunning(context, provider)

            // Step 2: Check PostgreSQL availability
            progress.report({ message: 'Checking PostgreSQL...' })
            await ensurePostgresAvailable()
        }

        // Step 3: Create process orchestrator
        progress.report({ message: 'Starting watch processes...' })
        outputChannel?.appendLine('Creating process orchestrator...')

        // Get database URL from settings
        let databaseUrl: string
        if (isSqliteMode()) {
            const dbPath = path.join(workspaceRoot, '.crewchief', 'maproom.db')
            databaseUrl = `sqlite://${dbPath}`
            // Ensure dir exists
            if (!fs.existsSync(path.dirname(dbPath))) {
                fs.mkdirSync(path.dirname(dbPath), { recursive: true })
            }
        } else {
            const config = getPostgresConfigFromSettings()
            databaseUrl = getPostgresUrl(config)
        }

        const postgresConfig = {
          host: 'maproom-postgres', // Docker network hostname
          port: 5432,
          user: 'maproom',
          password: 'maproom',
          database: 'maproom',
        }

        // Create secrets manager (provider already retrieved above)
        const secretsManager = new SecretsManager(context.secrets)

        orchestrator = new ProcessOrchestrator(outputChannel!, {
          extensionRoot: context.extensionPath,
          workspaceRoot,
          postgres: postgresConfig,
          secretsManager,
          provider,
          // Pass the calculated database URL directly to override default postgres logic
          // We'll need to update ProcessOrchestrator to accept this override
          databaseUrlOverride: databaseUrl 
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
    // ... (error handling)
  }
}

/**
 * Ensure PostgreSQL is available
 * ...
 */
async function ensurePostgresAvailable(): Promise<void> {
  if (isSqliteMode()) return

  const config = getPostgresConfigFromSettings()
  // ... (rest of implementation)
}

/**
 * Run initial workspace scan
 * ...
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

  // Get database URL
  let databaseUrl: string
  if (isSqliteMode()) {
      // Use default local sqlite file in workspace storage or global storage?
      // Ideally global storage to share across workspaces for repo caching?
      // Or workspace storage for isolation?
      // The daemon defaults to `maproom.db` in CWD.
      // VSCode extension runs daemon with CWD = workspaceRoot (usually).
      // So it will create maproom.db in the root of the repo.
      // Ideally we want it in .vscode/ or user directory.
      // Let's set it explicitly if we want control.
      // For zero-config, letting daemon decide (maproom.db) is risky if it pollutes root.
      // Let's set MAPROOM_DATABASE_URL to a file in the workspace's .crewchief directory or global storage.
      // Using workspaceRoot/.crewchief/maproom.db seems safe and standard for this project.
      const dbPath = path.join(workspaceRoot, '.crewchief', 'maproom.db')
      databaseUrl = `sqlite://${dbPath}`
      // Ensure dir exists? Daemon might do it or fail. 
      if (!fs.existsSync(path.dirname(dbPath))) {
          fs.mkdirSync(path.dirname(dbPath), { recursive: true })
      }
  } else {
      const config = getPostgresConfigFromSettings()
      databaseUrl = getPostgresUrl(config)
  }

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

// ... (rest of file)
