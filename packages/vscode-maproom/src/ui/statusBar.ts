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
import type { ProcessOrchestrator } from '../process/orchestrator.js'
import type { WatchEvent } from '../process/events.js'

/**
 * Status bar icon and text for each state
 */
const STATUS_CONFIG = {
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
type StatusBarState = 'idle' | 'watching' | 'indexing' | 'error'

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
  private readonly orchestrator: ProcessOrchestrator
  private currentState: StatusBarState = 'idle'
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
   * @param orchestrator - Process orchestrator to subscribe to
   */
  constructor(context: vscode.ExtensionContext, orchestrator: ProcessOrchestrator) {
    this.context = context
    this.orchestrator = orchestrator

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

    // Subscribe to orchestrator events
    this.orchestrator.on('watchEvent', this.watchEventHandler)

    // Initialize with idle state
    this.updateStatusBar()
    this.statusBarItem.show()
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
        this.handleStatusEvent(event.state)
        break

      case 'progress':
        this.handleProgressEvent(event.complete, event.files)
        break

      case 'complete':
        this.handleCompleteEvent(event.files)
        break

      case 'error':
        this.handleErrorEvent(event.message)
        break
    }
  }

  /**
   * Handle status state change event
   *
   * @param state - New state (watching, indexing, idle)
   */
  private handleStatusEvent(state: 'watching' | 'indexing' | 'idle'): void {
    this.currentState = state
    this.lastError = undefined

    if (state === 'idle') {
      // Reset counters when idle
      this.currentFileCount = 0
      this.totalFiles = 0
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
   * Handle indexing complete event
   *
   * @param filesIndexed - Total number of files indexed
   */
  private handleCompleteEvent(filesIndexed: number): void {
    this.currentState = 'watching'
    this.currentFileCount = filesIndexed
    this.totalFiles = filesIndexed
    this.lastError = undefined

    // Store last indexed timestamp
    this.context.workspaceState.update(LAST_INDEXED_KEY, Date.now())

    this.scheduleUpdate()
  }

  /**
   * Handle error event
   *
   * @param message - Error message
   */
  private handleErrorEvent(message: string): void {
    this.currentState = 'error'
    this.lastError = message

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
      // Show file count during indexing
      text = `$(${config.icon}) ${config.text} ${this.currentFileCount}/${this.totalFiles} files...`
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
      const timeAgo = this.formatTimeAgo(lastIndexed)
      lines.push(`Last indexed: ${timeAgo}`)
    }

    // Click instruction
    lines.push('', 'Click to show output')

    return lines.join('\n')
  }

  /**
   * Format timestamp as human-friendly "time ago" string
   *
   * @param timestamp - Unix timestamp in milliseconds
   * @returns Human-friendly string (e.g., "2 minutes ago")
   */
  private formatTimeAgo(timestamp: number): string {
    const now = Date.now()
    const seconds = Math.floor((now - timestamp) / 1000)

    if (seconds < 60) {
      return 'just now'
    }

    const minutes = Math.floor(seconds / 60)
    if (minutes < 60) {
      return `${minutes} minute${minutes === 1 ? '' : 's'} ago`
    }

    const hours = Math.floor(minutes / 60)
    if (hours < 24) {
      return `${hours} hour${hours === 1 ? '' : 's'} ago`
    }

    const days = Math.floor(hours / 24)
    return `${days} day${days === 1 ? '' : 's'} ago`
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
