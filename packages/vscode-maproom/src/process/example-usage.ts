/**
 * Example usage of ProcessOrchestrator
 *
 * This file demonstrates how to integrate the ProcessOrchestrator
 * with a VSCode extension to manage crewchief-maproom watch processes.
 */

import * as vscode from 'vscode'
import { ProcessOrchestrator, ProcessError } from './orchestrator'
import { DockerManager } from '../docker/manager'

/**
 * Example: Initialize and start watch processes in a VSCode extension
 */
export async function activateProcessOrchestrator(
  context: vscode.ExtensionContext
): Promise<ProcessOrchestrator | undefined> {
  // Create output channel for logs (shared with DockerManager)
  const outputChannel = vscode.window.createOutputChannel('Maproom')

  try {
    // 1. Get workspace root (required for branch-watch)
    const workspaceFolders = vscode.workspace.workspaceFolders
    if (!workspaceFolders || workspaceFolders.length === 0) {
      vscode.window.showErrorMessage('Maproom: No workspace folder is open')
      return undefined
    }

    const workspaceRoot = workspaceFolders[0].uri.fsPath

    // 2. Initialize DockerManager and ensure PostgreSQL is running
    const dockerManager = new DockerManager(outputChannel, context.extensionPath)

    outputChannel.appendLine('Starting Docker services...')
    await dockerManager.ensureServicesRunning()

    // 3. Get PostgreSQL configuration from DockerManager
    const postgresConfig = dockerManager.getPostgresConfig()

    // 4. Create ProcessOrchestrator
    const orchestrator = new ProcessOrchestrator(outputChannel, {
      extensionRoot: context.extensionPath,
      workspaceRoot,
      postgres: postgresConfig,
    })

    // 5. Set up event listeners for watch events
    orchestrator.on('watchEvent', (processName, event) => {
      // Handle different event types
      switch (event.type) {
        case 'progress':
          outputChannel.appendLine(
            `[${processName}] Progress: ${event.complete}/${event.files} files indexed`
          )
          break
        case 'error':
          outputChannel.appendLine(`[${processName}] Error: ${event.message}`)
          if (event.file) {
            outputChannel.appendLine(`  File: ${event.file}`)
          }
          break
        case 'complete':
          outputChannel.appendLine(
            `[${processName}] Complete: ${event.files} files indexed in ${event.duration}ms`
          )
          vscode.window.showInformationMessage(
            `Maproom: Indexed ${event.files} files in ${event.duration}ms`
          )
          break
        case 'status':
          outputChannel.appendLine(`[${processName}] Status: ${event.state}`)
          break
      }
    })

    // Handle parse errors (malformed NDJSON from binary)
    orchestrator.on('parseError', (processName, error, line) => {
      outputChannel.appendLine(`[${processName}] Parse error: ${error.message}`)
      outputChannel.appendLine(`  Invalid line: ${line}`)
    })

    // 6. Start watch processes
    outputChannel.appendLine('Starting watch processes...')
    await orchestrator.startWatching()

    outputChannel.show()
    vscode.window.showInformationMessage('Maproom: Watch processes started successfully')

    // 7. Register cleanup on extension deactivation
    context.subscriptions.push({
      dispose: async () => {
        outputChannel.appendLine('Stopping watch processes...')
        await orchestrator.stopWatching()
        await dockerManager.stop()
        outputChannel.dispose()
      },
    })

    // 8. Optional: Monitor process status
    const statusCheckInterval = setInterval(() => {
      if (!orchestrator.isRunning()) {
        outputChannel.appendLine('WARNING: Watch processes are not running!')
        vscode.window.showWarningMessage('Maproom: Watch processes have stopped unexpectedly')
        clearInterval(statusCheckInterval)
      }
    }, 30000) // Check every 30 seconds

    context.subscriptions.push({
      dispose: () => clearInterval(statusCheckInterval),
    })

    return orchestrator
  } catch (error) {
    if (error instanceof ProcessError) {
      outputChannel.appendLine(`ERROR: ${error.message}`)
      outputChannel.appendLine(`Code: ${error.code}`)

      // Show user-friendly error messages based on error code
      switch (error.code) {
        case 'BINARY_NOT_FOUND':
          vscode.window.showErrorMessage(
            'Maproom: Binary not found for your platform. Please reinstall the extension.'
          )
          break
        case 'BINARY_NOT_EXECUTABLE':
          vscode.window.showErrorMessage(
            'Maproom: Binary is not executable. Try running: chmod +x on the binary.'
          )
          break
        case 'PROCESS_CRASHED_IMMEDIATELY':
          vscode.window.showErrorMessage(
            'Maproom: Watch process crashed immediately. Check the output panel for details.'
          )
          break
        default:
          vscode.window.showErrorMessage(`Maproom: Failed to start watch processes: ${error.message}`)
      }
    } else {
      outputChannel.appendLine(`ERROR: ${error}`)
      vscode.window.showErrorMessage('Maproom: An unexpected error occurred')
    }

    outputChannel.show()
    return undefined
  }
}

/**
 * Example: Manually control processes via commands
 */
export function registerProcessCommands(
  context: vscode.ExtensionContext,
  orchestrator: ProcessOrchestrator
): void {
  // Command: Start watching
  context.subscriptions.push(
    vscode.commands.registerCommand('maproom.startWatching', async () => {
      try {
        await orchestrator.startWatching()
        vscode.window.showInformationMessage('Maproom: Watch processes started')
      } catch (error) {
        vscode.window.showErrorMessage(`Maproom: Failed to start: ${error}`)
      }
    })
  )

  // Command: Stop watching
  context.subscriptions.push(
    vscode.commands.registerCommand('maproom.stopWatching', async () => {
      try {
        await orchestrator.stopWatching()
        vscode.window.showInformationMessage('Maproom: Watch processes stopped')
      } catch (error) {
        vscode.window.showErrorMessage(`Maproom: Failed to stop: ${error}`)
      }
    })
  )

  // Command: Show process status
  context.subscriptions.push(
    vscode.commands.registerCommand('maproom.showProcessStatus', () => {
      const status = orchestrator.getStatus()
      const statusLines: string[] = []

      for (const [name, info] of status) {
        const state = info.running ? '✓ Running' : info.crashed ? '✗ Crashed' : '○ Stopped'
        const exitCode = info.exitCode !== undefined ? ` (exit code: ${info.exitCode})` : ''
        statusLines.push(`${name}: ${state}${exitCode}`)
      }

      vscode.window.showInformationMessage(
        `Maproom Process Status:\n${statusLines.join('\n')}`
      )
    })
  )
}
