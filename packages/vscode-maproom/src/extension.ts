/**
 * VSCode extension entry point for Maproom Semantic Search
 *
 * This is a stub implementation showing how to integrate the core components:
 * - DockerManager: Manages PostgreSQL container lifecycle
 * - ProcessOrchestrator: Manages watch processes (file monitoring, branch monitoring)
 * - StatusBarManager: Displays real-time indexing status in status bar
 *
 * Extension lifecycle:
 * 1. activate() - Called when extension loads (onStartupFinished)
 *    - Start PostgreSQL container via DockerManager
 *    - Start watch processes via ProcessOrchestrator
 *    - Initialize StatusBarManager to display status
 *    - Register commands (maproom.showOutput)
 * 2. deactivate() - Called when extension unloads
 *    - Stop watch processes
 *    - Optionally stop PostgreSQL container
 *    - Cleanup resources
 */

import * as vscode from 'vscode'
import { DockerManager } from './docker/manager.js'
import { ProcessOrchestrator } from './process/orchestrator.js'
import { StatusBarManager } from './ui/statusBar.js'

/**
 * Output channel for extension logging
 */
let outputChannel: vscode.OutputChannel

/**
 * Docker manager for PostgreSQL container
 */
let dockerManager: DockerManager

/**
 * Process orchestrator for watch processes
 */
let orchestrator: ProcessOrchestrator

/**
 * Status bar manager for UI updates
 */
let statusBar: StatusBarManager

/**
 * Extension activation
 *
 * Called when extension is activated (onStartupFinished).
 * This is the recommended activation event for background tasks that don't
 * need to block editor startup.
 *
 * @param context - Extension context
 */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
  console.log('Maproom extension activating...')

  // Create output channel for logging
  outputChannel = vscode.window.createOutputChannel('Maproom')
  context.subscriptions.push(outputChannel)

  try {
    // Step 1: Start Docker Compose services (PostgreSQL, Ollama, MCP)
    outputChannel.appendLine('Starting Docker Compose services...')
    dockerManager = new DockerManager(outputChannel, context.extensionPath)

    await dockerManager.ensureServicesRunning()
    outputChannel.appendLine('Docker services started successfully')

    // PostgreSQL connection details (from docker-compose.yml)
    const host = 'localhost' // or 'maproom-postgres' if connecting from within Docker
    const port = 5432

    // Step 2: Start watch processes
    outputChannel.appendLine('Starting watch processes...')

    const workspaceFolder = vscode.workspace.workspaceFolders?.[0]
    if (!workspaceFolder) {
      throw new Error('No workspace folder open')
    }

    orchestrator = new ProcessOrchestrator(outputChannel, {
      extensionRoot: context.extensionPath,
      workspaceRoot: workspaceFolder.uri.fsPath,
      postgres: {
        host,
        port,
        user: 'maproom',
        password: 'maproom',
        database: 'maproom',
      },
    })

    await orchestrator.startWatching()
    outputChannel.appendLine('Watch processes started')

    // Step 3: Initialize status bar
    outputChannel.appendLine('Initializing status bar...')
    statusBar = new StatusBarManager(context, orchestrator)
    context.subscriptions.push(statusBar)
    outputChannel.appendLine('Status bar initialized')

    // Step 4: Register commands
    const showOutputCommand = vscode.commands.registerCommand('maproom.showOutput', () => {
      outputChannel.show()
    })
    context.subscriptions.push(showOutputCommand)

    outputChannel.appendLine('Maproom extension activated successfully')
    console.log('Maproom extension activated successfully')
  } catch (error: any) {
    const errorMessage = `Failed to activate Maproom extension: ${error.message}`
    outputChannel.appendLine(`ERROR: ${errorMessage}`)
    console.error(errorMessage, error)

    // Show error to user
    vscode.window.showErrorMessage(errorMessage)

    // Cleanup partial initialization
    await cleanup()

    throw error
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
 * Can be called from deactivate() or on activation failure.
 */
async function cleanup(): Promise<void> {
  try {
    // Stop watch processes
    if (orchestrator) {
      outputChannel?.appendLine('Stopping watch processes...')
      await orchestrator.stopWatching()
      outputChannel?.appendLine('Watch processes stopped')
    }

    // Note: We don't stop PostgreSQL container on deactivation
    // because it may be shared across VSCode sessions.
    // Users can manually stop it with: docker stop maproom-postgres
    // or we could add a command: "Maproom: Stop Database"

    // Status bar is disposed via context.subscriptions
  } catch (error: any) {
    outputChannel?.appendLine(`ERROR during cleanup: ${error.message}`)
    console.error('Error during cleanup:', error)
  }
}
