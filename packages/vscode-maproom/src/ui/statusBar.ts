/**
 * Status bar manager for Maproom extension
 *
 * Displays indexing status in VSCode status bar with real-time updates
 * based on watch process events from ProcessOrchestrator.
 *
 * Key features:
 * - Real-time status updates (Idle, Watching, Indexing, Error)
 * - Debounced updates (max 1 update/second to prevent flicker)
 * - Detailed tooltip with last indexed time, file counts, state
 * - Click handler to open Output panel
 * - Persistent state storage (last indexed timestamp)
 * - Proper disposal and cleanup
 */

import * as vscode from 'vscode'
import type { ProcessOrchestrator } from '../process/orchestrator'
import type { WatchEvent } from '../process/events'
import { formatRelativeTime } from '../utils/time'

/**
 * Status bar icon and text for each state
 */
const STATUS_CONFIG = {
  starting: {
    icon: 'sync~spin',
    text: 'Starting...',
  },
  idle: {
    icon: 'database',
    text: 'Maproom Ready',
  },
  watching: {
    icon: 'eye',
    text: 'Watching...',
  },
  indexing: {
    icon: 'sync~spin',
    text: 'Indexing', // Suffix added dynamically with file count
  },
  error: {
    icon: 'error',
    text: 'Maproom Error',
  },
} as const

/**
 * Current state of the status bar
 */
type StatusBarState = 'starting' | 'idle' | 'watching' | 'indexing' | 'error'

/**
 * Debounce interval for status bar updates (milliseconds)
 */
const UPDATE_DEBOUNCE_MS = 1000

/**
 * Key for storing last indexed timestamp in workspace state
 */
const LAST_INDEXED_KEY = 'maproom.lastIndexed'

/**
 * Status bar manager for Maproom extension
 *
 * Manages a VSCode status bar item that shows real-time indexing status.
 * Subscribes to ProcessOrchestrator events and updates display accordingly.
 *
 * Usage:
 * ```typescript
 * const statusBar = new StatusBarManager(context, orchestrator)
 * // Status bar automatically updates based on orchestrator events
 * // Click status bar to open Output panel
 * ```
 */
export class StatusBarManager implements vscode.Disposable {
  private readonly statusBarItem: vscode.StatusBarItem
  private readonly context: vscode.ExtensionContext
  private orchestrator: ProcessOrchestrator
  private currentState: StatusBarState = 'starting'
  private currentFileCount = 0
  private totalFiles = 0
  private lastError: string | undefined
  private debounceTimer: NodeJS.Timeout | undefined
  private pendingUpdate = false
  private readonly watchEventHandler: (processName: string, event: WatchEvent) => void
  private isDisposed = false

  /**
   * Create a new status bar manager
   *
   * @param context - Extension context for state storage
   * @param orchestrator - Process orchestrator to subscribe to (optional for delayed initialization)
   */
  constructor(context: vscode.ExtensionContext, orchestrator?: ProcessOrchestrator) {
    this.context = context
    this.orchestrator = orchestrator!

    // Create status bar item
    // Right alignment, priority 100 (lower than most built-in items)
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      100
    )

    // Set command to open Output panel
    this.statusBarItem.command = 'maproom.showOutput'

    // Bind and store the event handler reference for proper cleanup
    this.watchEventHandler = this.handleWatchEvent.bind(this)

    // Subscribe to orchestrator events if provided
    if (orchestrator) {
      this.orchestrator.on('watchEvent', this.watchEventHandler)
    }

    // Initialize with idle state
    this.updateStatusBar()
    this.statusBarItem.show()
  }

  /**
   * Connect to process orchestrator for event updates
   *
   * Used when orchestrator is initialized after status bar creation.
   *
   * @param orchestrator - Process orchestrator to subscribe to
   */
  connectOrchestrator(orchestrator: ProcessOrchestrator): void {
    // Remove existing listener if already connected
    if (this.orchestrator) {
      this.orchestrator.removeListener('watchEvent', this.watchEventHandler)
    }

    // Store new orchestrator and subscribe
    this.orchestrator = orchestrator
    this.orchestrator.on('watchEvent', this.watchEventHandler)
  }

  /**
   * Manually set status bar state
   *
   * Used for pre-initialization states before orchestrator is ready.
   *
   * @param state - New state
   * @param message - Optional custom message
   */
  setState(state: StatusBarState, message?: string): void {
    this.currentState = state
    if (state === 'error' && message) {
      this.lastError = message
    } else if (state !== 'error') {
      this.lastError = undefined
    }
    this.scheduleUpdate()
  }

  /**
   * Handle watch events from ProcessOrchestrator
   *
   * @param processName - Name of the process that emitted the event
   * @param event - Watch event from stdout parser
   */
  private handleWatchEvent(processName: string, event: WatchEvent): void {
    switch (event.type) {
      case 'status':
        this.handleStatusEvent(event.state, event.files)
        break

      case 'progress':
        this.handleProgressEvent(event.complete, event.files)
        break

      case 'complete':
        this.handleCompleteEvent(event.files, event.timestamp)
        break

      case 'error':
        this.handleErrorEvent(event.message, event.file, event.error_type)
        break

      case 'file_processed':
        this.handleFileProcessedEvent(event.file_path)
        break
    }
  }

  /**
   * Handle status state change event
   *
   * @param state - New state (watching, indexing, idle)
   * @param files - Optional file count from enhanced status event
   */
  private handleStatusEvent(state: 'watching' | 'indexing' | 'idle', files?: number): void {
    this.currentState = state
    this.lastError = undefined

    if (state === 'idle') {
      // Reset counters when idle
      this.currentFileCount = 0
      this.totalFiles = 0
    } else if (files !== undefined) {
      // Update file count if provided
      this.currentFileCount = files
      this.totalFiles = files
    }

    this.scheduleUpdate()
  }

  /**
   * Handle indexing progress event
   *
   * @param complete - Number of files completed
   * @param total - Total number of files to index
   */
  private handleProgressEvent(complete: number, total: number): void {
    this.currentState = 'indexing'
    this.currentFileCount = complete
    this.totalFiles = total
    this.lastError = undefined

    this.scheduleUpdate()
  }

  /**
   * Handle file processed event
   *
   * Not currently displayed in status bar, but logged for debugging
   *
   * @param filePath - Path to processed file
   */
  private handleFileProcessedEvent(filePath: string): void {
    // Future enhancement: Could show last processed file in tooltip
    // For now, just ensure we stay in indexing state
    if (this.currentState !== 'indexing') {
      this.currentState = 'indexing'
      this.scheduleUpdate()
    }
  }

  /**
   * Handle indexing complete event
   *
   * @param filesIndexed - Total number of files indexed
   * @param timestamp - Optional ISO timestamp from enhanced complete event
   */
  private handleCompleteEvent(filesIndexed: number, timestamp?: string): void {
    this.currentState = 'watching'
    this.currentFileCount = filesIndexed
    this.totalFiles = filesIndexed
    this.lastError = undefined

    // Store last indexed timestamp
    // Use provided timestamp if available, otherwise use current time
    const completionTime = timestamp ? new Date(timestamp).getTime() : Date.now()
    this.context.workspaceState.update(LAST_INDEXED_KEY, completionTime)

    this.scheduleUpdate()
  }

  /**
   * Handle error event
   *
   * @param message - Error message
   * @param file - Optional file path where error occurred
   * @param errorType - Optional error classification
   */
  private handleErrorEvent(
    message: string,
    file?: string,
    errorType?: 'parse' | 'io' | 'embedding' | 'database'
  ): void {
    this.currentState = 'error'

    // Build detailed error message with file path and type if available
    let errorMessage = message
    if (file) {
      errorMessage = `${message} (in ${file})`
    }
    if (errorType) {
      errorMessage = `[${errorType}] ${errorMessage}`
    }

    this.lastError = errorMessage

    this.scheduleUpdate()
  }

  /**
   * Schedule a debounced status bar update
   *
   * Prevents flickering by batching rapid updates and limiting to
   * max 1 update per second.
   */
  private scheduleUpdate(): void {
    // Don't schedule updates if disposed
    if (this.isDisposed) {
      return
    }

    this.pendingUpdate = true

    // If already scheduled, wait for existing timer
    if (this.debounceTimer) {
      return
    }

    // Schedule update after debounce interval
    this.debounceTimer = setTimeout(() => {
      this.debounceTimer = undefined
      if (this.pendingUpdate && !this.isDisposed) {
        this.pendingUpdate = false
        this.updateStatusBar()
      }
    }, UPDATE_DEBOUNCE_MS)
  }

  /**
   * Update the status bar item text and tooltip
   *
   * Called after debounce timer expires.
   */
  private updateStatusBar(): void {
    const config = STATUS_CONFIG[this.currentState]

    // Build status text
    let text = `$(${config.icon}) ${config.text}`

    if (this.currentState === 'indexing' && this.totalFiles > 0) {
      // Show file count during indexing with locale formatting
      const formattedTotal = this.totalFiles.toLocaleString()
      text = `$(${config.icon}) ${config.text}: ${formattedTotal} files`
    } else if (this.currentState === 'watching' && this.currentFileCount > 0) {
      // Show indexed file count when watching
      const formattedCount = this.currentFileCount.toLocaleString()
      text = `$(${config.icon}) Indexed: ${formattedCount} files`
    }

    this.statusBarItem.text = text

    // Build detailed tooltip
    this.statusBarItem.tooltip = this.buildTooltip()
  }

  /**
   * Build tooltip with detailed status information
   *
   * @returns Tooltip markdown string
   */
  private buildTooltip(): string {
    const lines: string[] = ['Maproom Semantic Search']

    // Current state
    lines.push(`Status: ${this.currentState}`)

    // File counts for indexing state
    if (this.currentState === 'indexing' && this.totalFiles > 0) {
      lines.push(`Progress: ${this.currentFileCount}/${this.totalFiles} files`)
    }

    // Error message if present
    if (this.currentState === 'error' && this.lastError) {
      lines.push(`Error: ${this.lastError}`)
    }

    // Last indexed timestamp
    const lastIndexed = this.context.workspaceState.get<number>(LAST_INDEXED_KEY)
    if (lastIndexed) {
      const timeAgo = formatRelativeTime(new Date(lastIndexed))
      lines.push(`Last indexed: ${timeAgo}`)
    }

    // Click instruction
    lines.push('', 'Click to show output')

    return lines.join('\n')
  }

  /**
   * Dispose of resources
   *
   * Clears timers, removes event listeners, and hides status bar.
   */
  dispose(): void {
    // Mark as disposed to prevent any new updates
    this.isDisposed = true

    // Clear pending timer
    if (this.debounceTimer) {
      clearTimeout(this.debounceTimer)
      this.debounceTimer = undefined
    }

    // Remove event listeners using stored handler reference
    this.orchestrator.removeListener('watchEvent', this.watchEventHandler)

    // Dispose status bar item
    this.statusBarItem.dispose()
  }
}
