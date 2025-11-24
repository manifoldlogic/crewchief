/**
 * Example usage of DockerManager in a VSCode extension
 *
 * This file demonstrates how to integrate the DockerManager
 * into your extension's activation and deactivation lifecycle.
 */

import * as vscode from 'vscode'
import { DockerManager, DockerError } from './manager'

/**
 * Example extension activation
 */
export async function activate(context: vscode.ExtensionContext) {
  // Create output channel for Docker logs
  const outputChannel = vscode.window.createOutputChannel('Maproom Docker')

  // Create Docker manager
  const dockerManager = new DockerManager(outputChannel, context.extensionPath)

  // Show output channel to user (optional)
  // outputChannel.show()

  // Register a command to start services manually
  const startCommand = vscode.commands.registerCommand(
    'maproom.startServices',
    async () => {
      try {
        outputChannel.show()
        await vscode.window.withProgress(
          {
            location: vscode.ProgressLocation.Notification,
            title: 'Starting Maproom services...',
            cancellable: false,
          },
          async (progress) => {
            progress.report({ message: 'Checking Docker...' })
            await dockerManager.ensureServicesRunning('ollama', { MAPROOM_EMBEDDING_PROVIDER: 'ollama' })
            progress.report({ message: 'Services ready!' })
          }
        )

        vscode.window.showInformationMessage('Maproom services started successfully')
      } catch (error) {
        if (error instanceof DockerError) {
          if (error.code === 'DOCKER_NOT_FOUND') {
            vscode.window.showErrorMessage(
              'Docker not found. Please install Docker Desktop and restart.'
            )
          } else if (error.code === 'DOCKER_DAEMON_NOT_RUNNING') {
            vscode.window.showErrorMessage(
              'Docker is not running. Please start Docker Desktop and try again.'
            )
          } else {
            vscode.window.showErrorMessage(`Failed to start services: ${error.message}`)
          }
        } else {
          vscode.window.showErrorMessage(`Unexpected error: ${error}`)
        }
        outputChannel.show()
      }
    }
  )

  // Register a command to stop services manually
  const stopCommand = vscode.commands.registerCommand(
    'maproom.stopServices',
    async () => {
      try {
        await vscode.window.withProgress(
          {
            location: vscode.ProgressLocation.Notification,
            title: 'Stopping Maproom services...',
            cancellable: false,
          },
          async () => {
            await dockerManager.stop()
          }
        )

        vscode.window.showInformationMessage('Maproom services stopped successfully')
      } catch (error) {
        vscode.window.showErrorMessage(`Failed to stop services: ${error}`)
        outputChannel.show()
      }
    }
  )

  // Auto-start services on extension activation (optional)
  // Uncomment if you want services to start automatically
  /*
  try {
    outputChannel.appendLine('Auto-starting Maproom services...')
    await dockerManager.ensureServicesRunning('ollama', { MAPROOM_EMBEDDING_PROVIDER: 'ollama' })
    outputChannel.appendLine('Services ready')
  } catch (error) {
    outputChannel.appendLine(`Auto-start failed: ${error}`)
    // Don't block extension activation on Docker errors
  }
  */

  // Register cleanup on extension deactivation
  context.subscriptions.push(
    startCommand,
    stopCommand,
    outputChannel,
    new vscode.Disposable(async () => {
      try {
        outputChannel.appendLine('Extension deactivating, stopping services...')
        await dockerManager.stop()
      } catch (error) {
        outputChannel.appendLine(`Failed to stop services on deactivation: ${error}`)
      }
    })
  )

  return {
    dockerManager,
    outputChannel,
  }
}

/**
 * Example extension deactivation
 */
export async function deactivate() {
  // Cleanup is handled by the Disposable registered in activate()
  // No additional work needed here
}

/**
 * Example: Using DockerManager in a language server or other feature
 */
export class MaproomFeature {
  private dockerManager: DockerManager
  private outputChannel: vscode.OutputChannel

  constructor(extensionPath: string) {
    this.outputChannel = vscode.window.createOutputChannel('Maproom')
    this.dockerManager = new DockerManager(this.outputChannel, extensionPath)
  }

  /**
   * Initialize feature - ensure services are running
   */
  async initialize(): Promise<void> {
    try {
      await this.dockerManager.ensureServicesRunning('ollama', { MAPROOM_EMBEDDING_PROVIDER: 'ollama' })
      this.outputChannel.appendLine('Maproom feature initialized')
    } catch (error) {
      this.outputChannel.appendLine(`Failed to initialize: ${error}`)
      throw error
    }
  }

  /**
   * Perform a search (example)
   */
  async search(query: string): Promise<any[]> {
    // Ensure services are running before searching
    await this.dockerManager.ensureServicesRunning('ollama', { MAPROOM_EMBEDDING_PROVIDER: 'ollama' })

    // TODO: Implement actual search logic
    // This would typically connect to the MCP server or database
    this.outputChannel.appendLine(`Searching for: ${query}`)

    return []
  }

  /**
   * Clean up resources
   */
  async dispose(): Promise<void> {
    try {
      await this.dockerManager.stop()
      this.outputChannel.dispose()
    } catch (error) {
      // Log but don't throw on cleanup
      console.error('Failed to dispose MaproomFeature:', error)
    }
  }
}
