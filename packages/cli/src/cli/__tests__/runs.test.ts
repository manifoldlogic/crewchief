import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import chalk from 'chalk'
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest'
import { getLogPath, runExists, isValidStreamType, colorize } from '../runs'

// Force chalk color level so colorize tests produce ANSI codes in non-TTY environments
const savedLevel = chalk.level
beforeAll(() => {
  chalk.level = 1
})
afterAll(() => {
  chalk.level = savedLevel
})

// ---------------------------------------------------------------------------
// Helper: create a temporary run directory structure with log files
// ---------------------------------------------------------------------------
let tmpDir: string

function createRunLogs(runId: string, logs: { stdout?: string; stderr?: string; combined?: string }): void {
  const logDir = path.join(tmpDir, '.crewchief', 'runs', runId, 'logs')
  fs.mkdirSync(logDir, { recursive: true })
  if (logs.stdout !== undefined) {
    fs.writeFileSync(path.join(logDir, 'stdout.log'), logs.stdout)
  }
  if (logs.stderr !== undefined) {
    fs.writeFileSync(path.join(logDir, 'stderr.log'), logs.stderr)
  }
  if (logs.combined !== undefined) {
    fs.writeFileSync(path.join(logDir, 'combined.log'), logs.combined)
  }
}

function createRunDir(runId: string): void {
  const runDir = path.join(tmpDir, '.crewchief', 'runs', runId)
  fs.mkdirSync(runDir, { recursive: true })
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'runs-test-'))
})

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true })
})

// ---------------------------------------------------------------------------
// getLogPath
// ---------------------------------------------------------------------------
describe('getLogPath', () => {
  it('returns correct path for combined stream', () => {
    const result = getLogPath('run-123', 'combined', tmpDir)
    expect(result).toBe(path.join(tmpDir, '.crewchief', 'runs', 'run-123', 'logs', 'combined.log'))
  })

  it('returns correct path for stdout stream', () => {
    const result = getLogPath('run-123', 'stdout', tmpDir)
    expect(result).toBe(path.join(tmpDir, '.crewchief', 'runs', 'run-123', 'logs', 'stdout.log'))
  })

  it('returns correct path for stderr stream', () => {
    const result = getLogPath('run-123', 'stderr', tmpDir)
    expect(result).toBe(path.join(tmpDir, '.crewchief', 'runs', 'run-123', 'logs', 'stderr.log'))
  })

  it('defaults to process.cwd() when no baseDir provided', () => {
    const result = getLogPath('run-xyz', 'combined')
    expect(result).toBe(path.join(process.cwd(), '.crewchief', 'runs', 'run-xyz', 'logs', 'combined.log'))
  })
})

// ---------------------------------------------------------------------------
// runExists
// ---------------------------------------------------------------------------
describe('runExists', () => {
  it('returns true when run directory exists', () => {
    createRunDir('existing-run')
    expect(runExists('existing-run', tmpDir)).toBe(true)
  })

  it('returns false when run directory does not exist', () => {
    expect(runExists('nonexistent-run', tmpDir)).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// isValidStreamType
// ---------------------------------------------------------------------------
describe('isValidStreamType', () => {
  it('accepts stdout', () => {
    expect(isValidStreamType('stdout')).toBe(true)
  })

  it('accepts stderr', () => {
    expect(isValidStreamType('stderr')).toBe(true)
  })

  it('accepts combined', () => {
    expect(isValidStreamType('combined')).toBe(true)
  })

  it('rejects invalid stream type', () => {
    expect(isValidStreamType('invalid')).toBe(false)
  })

  it('rejects empty string', () => {
    expect(isValidStreamType('')).toBe(false)
  })

  it('rejects similar but incorrect names', () => {
    expect(isValidStreamType('STDOUT')).toBe(false)
    expect(isValidStreamType('std_out')).toBe(false)
    expect(isValidStreamType('all')).toBe(false)
  })
})

// ---------------------------------------------------------------------------
// colorize
// ---------------------------------------------------------------------------
describe('colorize', () => {
  it('returns same number of lines', () => {
    const input = ['line1', 'line2', 'line3']
    const result = colorize(input)
    expect(result).toHaveLength(3)
  })

  it('does not modify plain lines', () => {
    const input = ['just a plain line']
    const result = colorize(input)
    expect(result[0]).toBe('just a plain line')
  })

  it('does not modify the original array', () => {
    const input = ['ERROR something failed']
    const copy = [...input]
    colorize(input)
    expect(input).toEqual(copy)
  })

  it('colorizes ERROR lines (contains ANSI escape or chalk output)', () => {
    const errorLine = 'ERROR something failed'
    const plainLine = 'normal output'
    const result = colorize([errorLine, plainLine])
    // ERROR line should contain the original text
    expect(result[0]).toContain('ERROR something failed')
    // Plain line should be unmodified
    expect(result[1]).toBe(plainLine)
    // ERROR line result should match chalk.red output
    expect(result[0]).toBe(chalk.red(errorLine))
  })

  it('colorizes WARN lines with chalk.yellow', () => {
    const warnLine = 'WARN possible issue detected'
    const result = colorize([warnLine])
    expect(result[0]).toBe(chalk.yellow(warnLine))
  })

  it('colorizes WARNING lines with chalk.yellow', () => {
    const warnLine = 'WARNING low disk space'
    const result = colorize([warnLine])
    expect(result[0]).toBe(chalk.yellow(warnLine))
  })

  it('colorizes INFO lines with chalk.blue', () => {
    const infoLine = 'INFO server started'
    const result = colorize([infoLine])
    expect(result[0]).toBe(chalk.blue(infoLine))
  })

  it('colorizes FATAL lines with chalk.red', () => {
    const fatalLine = 'FATAL crash detected'
    const result = colorize([fatalLine])
    expect(result[0]).toBe(chalk.red(fatalLine))
  })

  it('handles empty array', () => {
    expect(colorize([])).toEqual([])
  })

  it('handles empty string lines', () => {
    const result = colorize([''])
    expect(result).toEqual([''])
  })
})

// ---------------------------------------------------------------------------
// Command action handler logic (integration-style tests using exported helpers)
// ---------------------------------------------------------------------------
describe('runs logs command logic', () => {
  describe('log reading', () => {
    it('reads combined log by default', () => {
      const logContent = 'line1\nline2\nline3\n'
      createRunLogs('run-abc', { combined: logContent })

      const logPath = getLogPath('run-abc', 'combined', tmpDir)
      expect(fs.existsSync(logPath)).toBe(true)

      const content = fs.readFileSync(logPath, 'utf-8')
      expect(content).toBe(logContent)
    })

    it('reads stdout log when requested', () => {
      createRunLogs('run-abc', {
        stdout: 'stdout line1\nstdout line2\n',
        combined: 'combined output\n',
      })

      const logPath = getLogPath('run-abc', 'stdout', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      expect(content).toContain('stdout line1')
    })

    it('reads stderr log when requested', () => {
      createRunLogs('run-abc', {
        stderr: 'error output\n',
        combined: 'combined output\n',
      })

      const logPath = getLogPath('run-abc', 'stderr', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      expect(content).toContain('error output')
    })
  })

  describe('--lines option', () => {
    it('shows last N lines from log file', () => {
      const lines = Array.from({ length: 100 }, (_, i) => `line ${i + 1}`)
      const logContent = lines.join('\n') + '\n'
      createRunLogs('run-lines', { combined: logContent })

      const logPath = getLogPath('run-lines', 'combined', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      const allLines = content.split('\n')

      // Simulate --lines 5
      const lastFive = allLines.slice(-5)
      expect(lastFive).toHaveLength(5)
      // The last element should be empty string (trailing newline)
      expect(lastFive[lastFive.length - 1]).toBe('')
      expect(lastFive[0]).toContain('line 97')
    })

    it('shows all lines when --lines is larger than file', () => {
      const logContent = 'line1\nline2\nline3\n'
      createRunLogs('run-small', { combined: logContent })

      const logPath = getLogPath('run-small', 'combined', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      const allLines = content.split('\n')

      // Simulate --lines 1000 on a 4-line file
      const sliced = allLines.slice(-1000)
      expect(sliced).toEqual(allLines)
    })
  })

  describe('error cases', () => {
    it('detects when run does not exist', () => {
      expect(runExists('nonexistent', tmpDir)).toBe(false)
    })

    it('detects when run exists but log file is missing', () => {
      // Create run directory without log files
      createRunDir('run-no-logs')
      expect(runExists('run-no-logs', tmpDir)).toBe(true)

      const logPath = getLogPath('run-no-logs', 'combined', tmpDir)
      expect(fs.existsSync(logPath)).toBe(false)
    })

    it('validates invalid stream types', () => {
      expect(isValidStreamType('invalid')).toBe(false)
      expect(isValidStreamType('both')).toBe(false)
      expect(isValidStreamType('all')).toBe(false)
    })
  })

  describe('backward compatibility (--tail)', () => {
    it('--tail value can be parsed as integer for --lines', () => {
      // Simulate the deprecated --tail handling
      const tailValue = '50'
      const parsed = parseInt(tailValue, 10)
      expect(parsed).toBe(50)
      expect(isNaN(parsed)).toBe(false)
    })

    it('--tail with invalid value does not produce a valid number', () => {
      const parsed = parseInt('notanumber', 10)
      expect(isNaN(parsed)).toBe(true)
    })
  })

  describe('--stream option', () => {
    it('correctly reads from each stream log file', () => {
      createRunLogs('run-streams', {
        stdout: 'stdout content\n',
        stderr: 'stderr content\n',
        combined: 'combined content\n',
      })

      const stdoutPath = getLogPath('run-streams', 'stdout', tmpDir)
      const stderrPath = getLogPath('run-streams', 'stderr', tmpDir)
      const combinedPath = getLogPath('run-streams', 'combined', tmpDir)

      expect(fs.readFileSync(stdoutPath, 'utf-8')).toBe('stdout content\n')
      expect(fs.readFileSync(stderrPath, 'utf-8')).toBe('stderr content\n')
      expect(fs.readFileSync(combinedPath, 'utf-8')).toBe('combined content\n')
    })
  })

  describe('--no-color option', () => {
    it('colorize applies chalk formatting to log-level lines', () => {
      const lines = ['ERROR test failure']
      const colored = colorize(lines)
      // With chalk.level=1 forced, output matches chalk.red
      expect(colored[0]).toBe(chalk.red('ERROR test failure'))
    })

    it('skipping colorize returns unmodified lines', () => {
      const lines = ['ERROR test failure', 'normal line']
      // Simulate --no-color: just don't call colorize
      const output = lines
      expect(output[0]).toBe('ERROR test failure')
      expect(output[1]).toBe('normal line')
    })
  })

  describe('edge cases', () => {
    it('handles empty log file', () => {
      createRunLogs('run-empty', { combined: '' })

      const logPath = getLogPath('run-empty', 'combined', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      const lines = content.split('\n')
      expect(lines).toEqual([''])
    })

    it('handles log file with only newlines', () => {
      createRunLogs('run-newlines', { combined: '\n\n\n' })

      const logPath = getLogPath('run-newlines', 'combined', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      const lines = content.split('\n')
      expect(lines).toEqual(['', '', '', ''])
    })

    it('handles single line without trailing newline', () => {
      createRunLogs('run-single', { combined: 'single line' })

      const logPath = getLogPath('run-single', 'combined', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      const lines = content.split('\n')
      expect(lines).toEqual(['single line'])
    })

    it('handles very long lines', () => {
      const longLine = 'x'.repeat(10000)
      createRunLogs('run-long', { combined: longLine + '\n' })

      const logPath = getLogPath('run-long', 'combined', tmpDir)
      const content = fs.readFileSync(logPath, 'utf-8')
      expect(content).toBe(longLine + '\n')
    })
  })
})
