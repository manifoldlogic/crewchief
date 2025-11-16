import * as vscode from 'vscode';
import { spawn, ChildProcess } from 'child_process';

let statusBarItem: vscode.StatusBarItem | undefined;
let childProcess: ChildProcess | undefined;

/**
 * Extension activation function.
 * Creates a status bar item showing "Hello World".
 *
 * Performance target: <500ms activation time
 */
export function activate(context: vscode.ExtensionContext) {
  // Create status bar item (right-aligned, high priority)
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );

  statusBarItem.text = "Hello World";
  statusBarItem.tooltip = "Hello World Test Extension";
  statusBarItem.show();

  // Register for proper cleanup
  context.subscriptions.push(statusBarItem);

  // Spawn echo process to test process spawning functionality
  spawnTestProcess();
}

/**
 * Spawns a test process and handles its output.
 * Updates the status bar item on completion.
 */
function spawnTestProcess() {
  try {
    // Spawn echo command (cross-platform)
    childProcess = spawn('echo', ['test']);

    // Handle stdout data
    if (childProcess.stdout) {
      childProcess.stdout.on('data', (data: Buffer) => {
        const output = data.toString().trim();
        console.log(`Process output: ${output}`);
      });
    }

    // Handle stderr (for completeness)
    if (childProcess.stderr) {
      childProcess.stderr.on('error', (error: Error) => {
        console.error('Process stderr error:', error);
      });
    }

    // Handle process errors
    childProcess.on('error', (error: Error) => {
      console.error('Failed to spawn process:', error);
      if (statusBarItem) {
        statusBarItem.text = "$(error) Process failed";
      }
    });

    // Handle process completion
    childProcess.on('close', (code: number | null) => {
      if (code === 0) {
        console.log('Process completed successfully');
        if (statusBarItem) {
          statusBarItem.text = "$(check) Process complete";
        }
      } else {
        console.error(`Process exited with code ${code}`);
        if (statusBarItem) {
          statusBarItem.text = `$(error) Process failed (code ${code})`;
        }
      }
      // Clear reference after completion
      childProcess = undefined;
    });
  } catch (error) {
    console.error('Exception spawning process:', error);
    if (statusBarItem) {
      statusBarItem.text = "$(error) Process error";
    }
  }
}

/**
 * Extension deactivation function.
 * Cleanup is handled automatically via context.subscriptions,
 * but explicit cleanup is shown here for clarity.
 */
export function deactivate() {
  // Clean up any running child process
  if (childProcess && !childProcess.killed) {
    console.log('Cleaning up child process on deactivation');
    childProcess.kill('SIGTERM');
    childProcess = undefined;
  }

  // StatusBarItem will be disposed via context.subscriptions
  // but we can explicitly clear the reference
  if (statusBarItem) {
    statusBarItem.dispose();
    statusBarItem = undefined;
  }
}
