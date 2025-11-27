import { spawn } from 'child_process'
import { EventEmitter } from 'events'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { scanAllWorktrees, scanWorktree, waitForScanCompletion, type ScanConfig } from './scan-orchestrator'

// Mock child_process spawn at module level before imports
vi.mock('child_process', () => ({
  spawn: vi.fn(),
  spawnSync: vi.fn().mockReturnValue({ status: 0, stdout: Buffer.from('/usr/local/bin/crewchief') }),
}))

// Mock fs to make findCrewchiefCli always find the binary
vi.mock('fs', async () => {
  const actual = await vi.importActual('fs')
  return {
    ...actual,
    existsSync: vi.fn().mockReturnValue(true),
  }
})

const mockSpawn = spawn as ReturnType<typeof vi.fn>

// Helper to create mock process with stdout/stderr
function createMockProcess(exitCode: number, stdout: string = '', stderr: string = ''): Record<string, EventEmitter> {
  const proc = new EventEmitter() as unknown as Record<string, EventEmitter>
  proc.stdout = new EventEmitter()
  proc.stderr = new EventEmitter()

  // Emit data and close asynchronously
  setImmediate(() => {
    if (stdout) {
      proc.stdout.emit('data', Buffer.from(stdout))
    }
    if (stderr) {
      proc.stderr.emit('data', Buffer.from(stderr))
    }
    proc.emit('close', exitCode)
  })

  return proc
}

describe('ScanOrchestrator', () => {
  let consoleLogSpy: ReturnType<typeof vi.spyOn>
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    mockSpawn.mockReset()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  describe('scanWorktree', () => {
    const testConfig: ScanConfig = {
      worktreePath: '/tmp/test',
      repo: 'crewchief',
      worktree: 'test-variant',
      commit: 'abc123',
      baseDir: '/tmp',
    }

    it('returns success when scan completes with chunk count', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Processing files...\nTotal chunks: 123\nScan complete')
      })

      const result = await scanWorktree(testConfig)

      expect(result.success).toBe(true)
      expect(result.worktree).toBe('test-variant')
      expect(result.chunkCount).toBe(123)
      expect(result.durationMs).toBeGreaterThanOrEqual(0)
      expect(result.error).toBeUndefined()
    })

    it('returns success with 0 chunks when chunk count not in output', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Processing files...\nScan complete')
      })

      const result = await scanWorktree(testConfig)

      expect(result.success).toBe(true)
      expect(result.chunkCount).toBe(0) // Default when parsing fails
    })

    it('returns failure when scan command exits with non-zero code', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(1, '', 'Error: Permission denied')
      })

      const result = await scanWorktree(testConfig)

      expect(result.success).toBe(false)
      expect(result.worktree).toBe('test-variant')
      expect(result.chunkCount).toBe(0)
      expect(result.error).toContain('Scan failed with code 1')
      expect(result.error).toContain('Permission denied')
    })

    it('captures stderr in error message on failure', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(2, '', 'Database connection failed')
      })

      const result = await scanWorktree(testConfig)

      expect(result.success).toBe(false)
      expect(result.error).toContain('Database connection failed')
    })

    it('uses spawn with args array and shell: false', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 100')
      })

      await scanWorktree(testConfig)

      // Now uses crewchief CLI with 'maproom scan' subcommand
      expect(mockSpawn).toHaveBeenCalledWith(
        expect.stringContaining('crewchief'),
        [
          'maproom',
          'scan',
          '--repo',
          'crewchief',
          '--worktree',
          'test-variant',
          '--commit',
          'abc123',
          '--path',
          '/tmp/test',
        ],
        expect.objectContaining({
          shell: false,
          stdio: 'pipe',
        }),
      )
    })

    it('logs progress messages during scan', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 456')
      })

      await scanWorktree(testConfig)

      expect(consoleLogSpy).toHaveBeenCalledWith('📊 Scanning worktree: test-variant')
      expect(consoleLogSpy).toHaveBeenCalledWith('   Path: /tmp/test')
      expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('✅ Scan complete: 456 chunks'))
    })

    it('logs error messages on failure', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(1, '', 'Test error')
      })

      await scanWorktree(testConfig)

      expect(consoleErrorSpy).toHaveBeenCalledWith(expect.stringContaining('❌ Scan failed'))
    })

    it('tracks duration correctly', async () => {
      mockSpawn.mockImplementation(() => {
        const proc = new EventEmitter() as unknown as Record<string, EventEmitter>
        proc.stdout = new EventEmitter()
        proc.stderr = new EventEmitter()

        // Delay to simulate scan time
        setTimeout(() => {
          proc.stdout.emit('data', Buffer.from('Total chunks: 789'))
          proc.emit('close', 0)
        }, 50)

        return proc
      })

      const result = await scanWorktree(testConfig)

      expect(result.durationMs).toBeGreaterThanOrEqual(50)
    })

    it('handles multiple chunk count patterns in output', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Processing: 50 chunks\nIndexed: 100 chunks\nTotal chunks: 999\nFinal count: 1000')
      })

      const result = await scanWorktree(testConfig)

      // Should match first occurrence
      expect(result.chunkCount).toBe(999)
    })
  })

  describe('scanAllWorktrees', () => {
    const configs: ScanConfig[] = [
      {
        worktreePath: '/tmp/variant-a',
        repo: 'crewchief',
        worktree: 'variant-a',
        commit: '123',
        baseDir: '/tmp',
      },
      {
        worktreePath: '/tmp/variant-b',
        repo: 'crewchief',
        worktree: 'variant-b',
        commit: '456',
        baseDir: '/tmp',
      },
      {
        worktreePath: '/tmp/variant-c',
        repo: 'crewchief',
        worktree: 'variant-c',
        commit: '789',
        baseDir: '/tmp',
      },
    ]

    it('scans all worktrees sequentially', async () => {
      let callCount = 0
      mockSpawn.mockImplementation(() => {
        callCount++
        return createMockProcess(0, `Total chunks: ${callCount * 100}`)
      })

      const results = await scanAllWorktrees(configs)

      expect(mockSpawn).toHaveBeenCalledTimes(3)
      expect(results).toHaveLength(3)
      expect(results[0].worktree).toBe('variant-a')
      expect(results[0].chunkCount).toBe(100)
      expect(results[1].worktree).toBe('variant-b')
      expect(results[1].chunkCount).toBe(200)
      expect(results[2].worktree).toBe('variant-c')
      expect(results[2].chunkCount).toBe(300)
    })

    it('all results have success: true when scans succeed', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 150')
      })

      const results = await scanAllWorktrees(configs)

      expect(results.every((r) => r.success)).toBe(true)
    })

    it('throws error immediately if any scan fails (fail-fast)', async () => {
      let callCount = 0
      mockSpawn.mockImplementation(() => {
        callCount++
        if (callCount === 2) {
          return createMockProcess(1, '', 'Scan error on variant-b')
        }
        return createMockProcess(0, 'Total chunks: 100')
      })

      await expect(scanAllWorktrees(configs)).rejects.toThrow('Scan failed for variant-b')

      // Should have stopped after second scan
      expect(mockSpawn).toHaveBeenCalledTimes(2)
    })

    it('logs summary with total duration and chunks', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 200')
      })

      await scanAllWorktrees(configs)

      expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('📊 Scanning 3 worktrees'))
      expect(consoleLogSpy).toHaveBeenCalledWith('='.repeat(60))
      expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('✅ All scans complete'))
      expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('📊 Total chunks indexed: 600'))
    })

    it('calculates total duration correctly', async () => {
      mockSpawn.mockImplementation(() => {
        const proc = new EventEmitter() as unknown as Record<string, EventEmitter>
        proc.stdout = new EventEmitter()
        proc.stderr = new EventEmitter()

        setTimeout(() => {
          proc.stdout.emit('data', Buffer.from('Total chunks: 50'))
          proc.emit('close', 0)
        }, 30)

        return proc
      })

      const results = await scanAllWorktrees(configs)

      const totalDuration = results.reduce((sum, r) => sum + r.durationMs, 0)
      expect(totalDuration).toBeGreaterThanOrEqual(90) // 3 scans * ~30ms each
    })

    it('handles empty config array', async () => {
      const results = await scanAllWorktrees([])

      expect(results).toHaveLength(0)
      expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('📊 Scanning 0 worktrees'))
    })

    it('preserves order of scan results', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 1')
      })

      const results = await scanAllWorktrees(configs)

      expect(results[0].worktree).toBe('variant-a')
      expect(results[1].worktree).toBe('variant-b')
      expect(results[2].worktree).toBe('variant-c')
    })
  })

  describe('waitForScanCompletion', () => {
    it('throws "Async scanning not implemented" error', async () => {
      await expect(waitForScanCompletion('test-scan-id')).rejects.toThrow('Async scanning not implemented')
    })

    it('throws error immediately without waiting', async () => {
      const startTime = Date.now()

      await expect(waitForScanCompletion('test-scan-id', 5000)).rejects.toThrow()

      const duration = Date.now() - startTime
      // Should fail immediately, not wait for timeout
      expect(duration).toBeLessThan(1000)
    })

    it('accepts timeout parameter', async () => {
      // Should accept timeout but still throw immediately
      await expect(waitForScanCompletion('test-scan-id', 10000)).rejects.toThrow('Async scanning not implemented')
    })
  })

  describe('Security - Command Injection Protection', () => {
    it('never uses shell option', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 1')
      })

      const maliciousConfig: ScanConfig = {
        worktreePath: '/tmp/test; rm -rf /',
        repo: 'crewchief; cat /etc/passwd',
        worktree: 'test`whoami`',
        commit: 'abc$(date)',
        baseDir: '/tmp',
      }

      await scanWorktree(maliciousConfig)

      // Verify shell: false was used (now uses crewchief CLI)
      expect(mockSpawn).toHaveBeenCalledWith(
        expect.stringContaining('crewchief'),
        expect.any(Array),
        expect.objectContaining({ shell: false }),
      )

      // Verify malicious strings passed as individual args (safe)
      const [, args] = mockSpawn.mock.calls[0]
      expect(args).toContain('/tmp/test; rm -rf /')
      expect(args).toContain('crewchief; cat /etc/passwd')
      expect(args).toContain('test`whoami`')
      expect(args).toContain('abc$(date)')
    })

    it('uses args array instead of string interpolation', async () => {
      mockSpawn.mockImplementation(() => {
        return createMockProcess(0, 'Total chunks: 1')
      })

      const config: ScanConfig = {
        worktreePath: '/tmp/test',
        repo: 'crewchief',
        worktree: 'variant-a',
        commit: 'HEAD',
        baseDir: '/tmp',
      }

      await scanWorktree(config)

      // First argument should be command (crewchief CLI path)
      const [command, args] = mockSpawn.mock.calls[0]
      expect(command).toContain('crewchief')

      // Second argument should be array of args (now uses 'maproom scan' subcommand)
      expect(Array.isArray(args)).toBe(true)
      expect(args).toEqual([
        'maproom',
        'scan',
        '--repo',
        'crewchief',
        '--worktree',
        'variant-a',
        '--commit',
        'HEAD',
        '--path',
        '/tmp/test',
      ])
    })
  })
})
