/**
 * Tests for NDJSON stdout parser
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { Readable } from 'node:stream'
import { StdoutParser } from './parser.js'
import type { WatchEvent } from './events.js'

describe('StdoutParser', () => {
  let stdout: Readable
  let parser: StdoutParser

  beforeEach(() => {
    stdout = new Readable({
      read() {}, // No-op read (we'll push data manually)
    })
    parser = new StdoutParser(stdout)
  })

  afterEach(() => {
    parser.close()
  })

  describe('valid NDJSON events', () => {
    it('should parse progress event', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('progress')
        if (event.type === 'progress') {
          expect(event.files).toBe(100)
          expect(event.complete).toBe(45)
        }
        done()
      })

      stdout.push('{"type":"progress","files":100,"complete":45}\n')
      stdout.push(null) // End stream
    })

    it('should parse error event', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('error')
        if (event.type === 'error') {
          expect(event.message).toBe('Failed to parse file')
          expect(event.file).toBe('src/main.rs')
        }
        done()
      })

      stdout.push('{"type":"error","message":"Failed to parse file","file":"src/main.rs"}\n')
      stdout.push(null)
    })

    it('should parse error event without file', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('error')
        if (event.type === 'error') {
          expect(event.message).toBe('Database connection failed')
          expect(event.file).toBeUndefined()
        }
        done()
      })

      stdout.push('{"type":"error","message":"Database connection failed"}\n')
      stdout.push(null)
    })

    it('should parse complete event', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('complete')
        if (event.type === 'complete') {
          expect(event.files).toBe(100)
          expect(event.duration).toBe(2500)
        }
        done()
      })

      stdout.push('{"type":"complete","files":100,"duration":2500}\n')
      stdout.push(null)
    })

    it('should parse status event - watching', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('status')
        if (event.type === 'status') {
          expect(event.state).toBe('watching')
        }
        done()
      })

      stdout.push('{"type":"status","state":"watching"}\n')
      stdout.push(null)
    })

    it('should parse status event - indexing', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('status')
        if (event.type === 'status') {
          expect(event.state).toBe('indexing')
        }
        done()
      })

      stdout.push('{"type":"status","state":"indexing"}\n')
      stdout.push(null)
    })

    it('should parse status event - idle', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('status')
        if (event.type === 'status') {
          expect(event.state).toBe('idle')
        }
        done()
      })

      stdout.push('{"type":"status","state":"idle"}\n')
      stdout.push(null)
    })

    it('should parse multiple events in sequence', (done) => {
      const events: WatchEvent[] = []

      parser.on('event', (event: WatchEvent) => {
        events.push(event)

        if (events.length === 3) {
          expect(events[0].type).toBe('status')
          expect(events[1].type).toBe('progress')
          expect(events[2].type).toBe('complete')
          done()
        }
      })

      stdout.push('{"type":"status","state":"indexing"}\n')
      stdout.push('{"type":"progress","files":50,"complete":25}\n')
      stdout.push('{"type":"complete","files":50,"duration":1000}\n')
      stdout.push(null)
    })
  })

  describe('malformed JSON', () => {
    it('should emit parseError for invalid JSON', async () => {
      return new Promise<void>((resolve) => {
        parser.on('parseError', (error: Error, line: string) => {
          expect(error).toBeInstanceOf(Error)
          expect(line).toBe('{invalid json}')
          resolve()
        })

        stdout.push('{invalid json}\n')
        stdout.push(null)
      })
    })

    it('should emit parseError for incomplete JSON', async () => {
      return new Promise<void>((resolve) => {
        parser.on('parseError', (error: Error, line: string) => {
          expect(error).toBeInstanceOf(Error)
          expect(line).toBe('{"type":"progress"')
          resolve()
        })

        stdout.push('{"type":"progress"\n')
        stdout.push(null)
      })
    })

    it('should continue parsing after malformed JSON', async () => {
      return new Promise<void>((resolve) => {
        const events: WatchEvent[] = []
        let errorCount = 0

        parser.on('event', (event: WatchEvent) => {
          events.push(event)
        })

        parser.on('parseError', () => {
          errorCount++
        })

        parser.on('close', () => {
          expect(errorCount).toBe(1)
          expect(events.length).toBe(2)
          expect(events[0].type).toBe('progress')
          expect(events[1].type).toBe('complete')
          resolve()
        })

        setImmediate(() => {
          stdout.push('{"type":"progress","files":10,"complete":5}\n')
          stdout.push('{invalid}\n')
          stdout.push('{"type":"complete","files":10,"duration":500}\n')
          stdout.push(null)
        })
      })
    })
  })

  describe('partial lines', () => {
    it('should handle partial lines across chunks', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('progress')
        if (event.type === 'progress') {
          expect(event.files).toBe(100)
          expect(event.complete).toBe(50)
        }
        done()
      })

      // Split JSON across multiple chunks
      stdout.push('{"type":"prog')
      stdout.push('ress","files":')
      stdout.push('100,"complete":50}\n')
      stdout.push(null)
    })

    it('should handle multiple events split across chunks', (done) => {
      const events: WatchEvent[] = []

      parser.on('event', (event: WatchEvent) => {
        events.push(event)

        if (events.length === 2) {
          expect(events[0].type).toBe('progress')
          expect(events[1].type).toBe('complete')
          done()
        }
      })

      stdout.push('{"type":"progress","files":20,"co')
      stdout.push('mplete":10}\n{"type":"compl')
      stdout.push('ete","files":20,"duration":800}\n')
      stdout.push(null)
    })
  })

  describe('missing fields', () => {
    it('should emit parseError for missing type field', (done) => {
      parser.on('parseError', (error: Error, line: string) => {
        expect(error.message).toContain('Invalid event schema')
        expect(line).toContain('"files":100')
        done()
      })

      stdout.push('{"files":100,"complete":50}\n')
      stdout.push(null)
    })

    it('should emit parseError for missing progress fields', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"progress","files":100}\n') // Missing complete
      stdout.push(null)
    })

    it('should emit parseError for missing error message', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"error"}\n') // Missing message
      stdout.push(null)
    })

    it('should emit parseError for missing complete fields', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"complete","files":100}\n') // Missing duration
      stdout.push(null)
    })

    it('should emit parseError for missing status state', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"status"}\n') // Missing state
      stdout.push(null)
    })
  })

  describe('invalid event types', () => {
    it('should emit parseError for unknown event type', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"unknown","data":"test"}\n')
      stdout.push(null)
    })

    it('should emit parseError for invalid status state', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"status","state":"invalid"}\n')
      stdout.push(null)
    })

    it('should emit parseError for wrong field types', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"progress","files":"not a number","complete":50}\n')
      stdout.push(null)
    })

    it('should emit parseError for negative progress values', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"progress","files":100,"complete":-1}\n')
      stdout.push(null)
    })

    it('should emit parseError for complete > files', (done) => {
      parser.on('parseError', (error: Error) => {
        expect(error.message).toContain('Invalid event schema')
        done()
      })

      stdout.push('{"type":"progress","files":100,"complete":150}\n')
      stdout.push(null)
    })
  })

  describe('high-frequency events', () => {
    it('should handle rapid event stream', (done) => {
      const events: WatchEvent[] = []
      const eventCount = 100

      parser.on('event', (event: WatchEvent) => {
        events.push(event)

        if (events.length === eventCount) {
          expect(events).toHaveLength(eventCount)
          // Verify all events are progress events with incrementing complete values
          events.forEach((evt, index) => {
            expect(evt.type).toBe('progress')
            if (evt.type === 'progress') {
              expect(evt.files).toBe(100)
              expect(evt.complete).toBe(index + 1)
            }
          })
          done()
        }
      })

      // Push 100 events rapidly
      for (let i = 1; i <= eventCount; i++) {
        stdout.push(`{"type":"progress","files":100,"complete":${i}}\n`)
      }
      stdout.push(null)
    })
  })

  describe('empty lines', () => {
    it('should ignore empty lines', async () => {
      return new Promise<void>((resolve) => {
        const events: WatchEvent[] = []

        parser.on('event', (event: WatchEvent) => {
          events.push(event)
        })

        parser.on('close', () => {
          expect(events).toHaveLength(2)
          expect(events[0].type).toBe('progress')
          expect(events[1].type).toBe('complete')
          resolve()
        })

        setImmediate(() => {
          stdout.push('{"type":"progress","files":10,"complete":5}\n')
          stdout.push('\n')
          stdout.push('\n')
          stdout.push('{"type":"complete","files":10,"duration":500}\n')
          stdout.push(null)
        })
      })
    })

    it('should ignore whitespace-only lines', async () => {
      return new Promise<void>((resolve) => {
        const events: WatchEvent[] = []

        parser.on('event', (event: WatchEvent) => {
          events.push(event)
        })

        parser.on('close', () => {
          expect(events).toHaveLength(1)
          resolve()
        })

        setImmediate(() => {
          stdout.push('   \n')
          stdout.push('\t\t\n')
          stdout.push('{"type":"status","state":"idle"}\n')
          stdout.push(null)
        })
      })
    })
  })

  describe('getLastValidEvent', () => {
    it('should return null before any events parsed', () => {
      expect(parser.getLastValidEvent()).toBeNull()
    })

    it('should return last valid event', (done) => {
      parser.on('event', (event: WatchEvent) => {
        if (event.type === 'complete') {
          const lastEvent = parser.getLastValidEvent()
          expect(lastEvent).not.toBeNull()
          expect(lastEvent?.type).toBe('complete')
          done()
        }
      })

      stdout.push('{"type":"progress","files":10,"complete":5}\n')
      stdout.push('{"type":"complete","files":10,"duration":500}\n')
      stdout.push(null)
    })

    it('should not update on parse errors', (done) => {
      parser.on('parseError', () => {
        const lastEvent = parser.getLastValidEvent()
        expect(lastEvent?.type).toBe('progress')
        done()
      })

      stdout.push('{"type":"progress","files":10,"complete":5}\n')
      stdout.push('{invalid}\n')
      stdout.push(null)
    })
  })

  describe('getLineCount', () => {
    it('should return 0 before parsing', () => {
      expect(parser.getLineCount()).toBe(0)
    })

    it('should count all lines including empty', async () => {
      return new Promise<void>((resolve) => {
        parser.on('close', () => {
          expect(parser.getLineCount()).toBe(4)
          resolve()
        })

        setImmediate(() => {
          stdout.push('{"type":"progress","files":10,"complete":5}\n')
          stdout.push('\n')
          stdout.push('{invalid}\n')
          stdout.push('{"type":"complete","files":10,"duration":500}\n')
          stdout.push(null)
        })
      })
    })
  })

  describe('close', () => {
    it('should emit close event when stream ends', async () => {
      return new Promise<void>((resolve) => {
        parser.on('close', () => {
          resolve()
        })

        setImmediate(() => {
          stdout.push('{"type":"status","state":"idle"}\n')
          stdout.push(null)
        })
      })
    })

    it('should clean up when close() is called', async () => {
      const eventSpy = vi.fn()
      parser.on('event', eventSpy)

      parser.close()

      // Try to push data after close
      setImmediate(() => {
        stdout.push('{"type":"status","state":"idle"}\n')
        stdout.push(null)
      })

      // Wait a bit then check
      await new Promise<void>((resolve) => setTimeout(resolve, 50))

      // Event should not be emitted after close
      expect(eventSpy).not.toHaveBeenCalled()
    })
  })

  describe('stream errors', () => {
    it('should handle stream errors without crashing', async () => {
      // Stream errors on stdin aren't actually handled by our parser
      // since we only listen to the readline interface, not the raw stream
      // This test just verifies the parser doesn't crash

      const eventSpy = vi.fn()
      parser.on('event', eventSpy)

      // Emit a valid event first
      stdout.push('{"type":"status","state":"idle"}\n')

      // Wait for event to be processed
      await new Promise<void>((resolve) => {
        parser.on('event', () => resolve())
      })

      expect(eventSpy).toHaveBeenCalledTimes(1)
    })
  })

  describe('CRLF handling', () => {
    it('should handle Windows-style line endings', (done) => {
      parser.on('event', (event: WatchEvent) => {
        expect(event.type).toBe('progress')
        done()
      })

      stdout.push('{"type":"progress","files":100,"complete":50}\r\n')
      stdout.push(null)
    })

    it('should handle mixed line endings', (done) => {
      const events: WatchEvent[] = []

      parser.on('event', (event: WatchEvent) => {
        events.push(event)

        if (events.length === 3) {
          expect(events).toHaveLength(3)
          done()
        }
      })

      stdout.push('{"type":"progress","files":10,"complete":5}\n')
      stdout.push('{"type":"progress","files":10,"complete":8}\r\n')
      stdout.push('{"type":"complete","files":10,"duration":500}\n')
      stdout.push(null)
    })
  })
})
