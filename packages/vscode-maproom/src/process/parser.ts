/**
 * NDJSON stdout parser for crewchief-maproom binary output
 *
 * Parses newline-delimited JSON events from stdout stream and emits
 * structured TypeScript events using the EventEmitter pattern.
 *
 * Key features:
 * - Line-by-line parsing using readline.createInterface()
 * - Automatic buffering of incomplete lines
 * - Graceful error handling (malformed JSON, invalid schemas)
 * - Defensive programming (doesn't crash on bad input)
 * - EventEmitter pattern for pub/sub
 */

import { EventEmitter } from 'node:events'
import { createInterface, type Interface } from 'node:readline'
import type { Readable } from 'node:stream'
import { isWatchEvent, type WatchEvent } from './events.js'

/**
 * Parser events emitted via EventEmitter
 */
export interface ParserEvents {
  /** Emitted when a valid event is parsed */
  event: (event: WatchEvent) => void
  /** Emitted when a parse error occurs (malformed JSON or invalid schema) */
  parseError: (error: Error, line: string) => void
  /** Emitted when readline closes */
  close: () => void
}

/**
 * NDJSON stdout parser
 *
 * Extends EventEmitter to provide pub/sub pattern for parsed events.
 *
 * Usage:
 * ```typescript
 * const parser = new StdoutParser(childProcess.stdout)
 * parser.on('event', (event) => {
 *   if (event.type === 'progress') {
 *     console.log(`Progress: ${event.complete}/${event.files}`)
 *   }
 * })
 * parser.on('parseError', (error, line) => {
 *   console.error('Parse error:', error.message, 'Line:', line)
 * })
 * ```
 */
export class StdoutParser extends EventEmitter {
  private readonly readline: Interface
  private lineCount = 0
  private lastValidEvent: WatchEvent | null = null
  private isClosed = false

  /**
   * Create a new stdout parser
   *
   * @param stdout - Readable stream from child process stdout
   */
  constructor(stdout: Readable) {
    super()

    // Create readline interface for line-by-line parsing
    // readline automatically handles buffering of partial lines
    this.readline = createInterface({
      input: stdout,
      crlfDelay: Infinity, // Treat \r\n as single line break
    })

    // Set up line event handler
    this.readline.on('line', (line: string) => {
      this.handleLine(line)
    })

    // Handle readline close
    this.readline.on('close', () => {
      this.isClosed = true
      this.emit('close')
    })

    // Handle stream errors (propagate as parseError)
    stdout.on('error', (error: Error) => {
      this.emit('parseError', error, '')
    })
  }

  /**
   * Process a single line from stdout
   *
   * @param line - Line of text (potentially NDJSON)
   */
  private handleLine(line: string): void {
    this.lineCount++

    // Ignore empty lines
    const trimmed = line.trim()
    if (!trimmed) {
      return
    }

    try {
      // Parse JSON
      const parsed = JSON.parse(trimmed)

      // Validate event structure
      if (!isWatchEvent(parsed)) {
        const error = new Error(
          `Invalid event schema at line ${this.lineCount}: ${JSON.stringify(parsed)}`
        )
        this.emit('parseError', error, trimmed)
        return
      }

      // Store last valid event for debugging
      this.lastValidEvent = parsed

      // Emit the valid event
      this.emit('event', parsed)
    } catch (error: unknown) {
      // Handle JSON parse errors
      const parseError = error instanceof Error
        ? error
        : new Error(`Unknown error parsing line ${this.lineCount}`)

      this.emit('parseError', parseError, trimmed)
    }
  }

  /**
   * Get the last valid event parsed
   *
   * Useful for debugging parser issues and understanding state.
   *
   * @returns Last valid event or null if none parsed yet
   */
  getLastValidEvent(): WatchEvent | null {
    return this.lastValidEvent
  }

  /**
   * Get the number of lines processed
   *
   * @returns Total line count (including empty and invalid lines)
   */
  getLineCount(): number {
    return this.lineCount
  }

  /**
   * Check if the parser is closed
   *
   * @returns true if readline has closed
   */
  isClosed_(): boolean {
    return this.isClosed
  }

  /**
   * Close the parser and clean up resources
   *
   * Removes all event listeners and closes readline interface.
   */
  close(): void {
    if (!this.isClosed) {
      this.readline.close()
      this.removeAllListeners()
    }
  }
}

/**
 * Typed EventEmitter interface for StdoutParser
 *
 * Use this for type-safe event handling:
 * ```typescript
 * const parser: TypedStdoutParser = new StdoutParser(stdout)
 * parser.on('event', (event) => { ... }) // event is typed as WatchEvent
 * ```
 */
export interface TypedStdoutParser {
  on<K extends keyof ParserEvents>(event: K, listener: ParserEvents[K]): this
  emit<K extends keyof ParserEvents>(event: K, ...args: Parameters<ParserEvents[K]>): boolean
  removeListener<K extends keyof ParserEvents>(event: K, listener: ParserEvents[K]): this
  removeAllListeners(event?: keyof ParserEvents): this
}
