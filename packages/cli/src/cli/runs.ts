import { spawn } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import chalk from 'chalk'
import { Command } from 'commander'
import { RunManager } from '../orchestrator/runManager'
import { logger } from '../utils/logger'

/** Valid stream types for log retrieval */
const VALID_STREAM_TYPES = ['stdout', 'stderr', 'combined'] as const
type StreamType = (typeof VALID_STREAM_TYPES)[number]

/** UUID format regex: 36 characters of hex digits and hyphens (case-insensitive) */
const UUID_REGEX = /^[0-9a-f-]{36}$/i

/**
 * Validate that a run ID matches UUID format.
 * Defense-in-depth measure to prevent path traversal attacks.
 * @throws {Error} if runId does not match UUID format
 */
export function validateRunId(runId: string): void {
  if (!UUID_REGEX.test(runId)) {
    throw new Error('Invalid run ID format')
  }
}

/**
 * Get the log file path for a given run and stream type.
 * Log files are stored at: .crewchief/runs/<runId>/logs/<stream>.log
 */
export function getLogPath(runId: string, stream: StreamType, baseDir?: string): string {
  validateRunId(runId)
  const runsDir = path.join(baseDir ?? process.cwd(), '.crewchief', 'runs')
  return path.join(runsDir, runId, 'logs', `${stream}.log`)
}

/**
 * Check whether a run directory exists on disk.
 */
export function runExists(runId: string, baseDir?: string): boolean {
  validateRunId(runId)
  const runsDir = path.join(baseDir ?? process.cwd(), '.crewchief', 'runs')
  const runDir = path.join(runsDir, runId)
  return fs.existsSync(runDir)
}

/**
 * Check whether a stream type string is valid.
 */
export function isValidStreamType(stream: string): stream is StreamType {
  return VALID_STREAM_TYPES.includes(stream as StreamType)
}

/**
 * Apply basic colorization to log lines.
 * - Lines containing ERROR or FATAL are colored red
 * - Lines containing WARN are colored yellow
 * - Lines containing INFO are colored blue
 * - All other lines are returned as-is
 */
export function colorize(lines: string[]): string[] {
  return lines.map((line) => {
    if (/\bERROR\b|\bFATAL\b/i.test(line)) {
      return chalk.red(line)
    }
    if (/\bWARN(ING)?\b/i.test(line)) {
      return chalk.yellow(line)
    }
    if (/\bINFO\b/i.test(line)) {
      return chalk.blue(line)
    }
    return line
  })
}

interface LogsOptions {
  lines?: number
  follow?: boolean
  stream: string
  color: boolean
  tail?: string
}

export function registerRunsCommands(program: Command): void {
  const runs = new Command('runs').description('Inspect agent runs')

  runs
    .command('list')
    .description('List persisted runs')
    .action(async () => {
      const rm = new RunManager()
      const list = rm.listRuns()
      if (list.length === 0) {
        logger.info('No runs')
        return
      }
      for (const run of list) {
        logger.info(`${run.id} ${run.platform} [${run.status}] ${run.workingDirectory} pane=${run.paneId}`)
      }
    })

  runs
    .command('events')
    .argument('<runId>')
    .description('Show parsed JSONL events for a run')
    .action(async (runId: string) => {
      const rm = new RunManager()
      const run = rm.getRun(runId)
      if (!run) {
        logger.warn(`Run not found: ${runId}`)
        return
      }
      const eventsPath = path.join(rm.getRunDir(runId), 'events.log')
      if (!fs.existsSync(eventsPath)) {
        logger.warn('No events.log found yet')
        return
      }
      const content = fs.readFileSync(eventsPath, 'utf8')
      process.stdout.write(content)
    })

  runs
    .command('logs')
    .argument('<runId>', 'Run ID to view logs for')
    .option('-n, --lines <count>', 'Number of lines to show from the end', parseInt)
    .option('-f, --follow', 'Follow log output (tail -f behavior)')
    .option('--stream <type>', 'Stream to display (stdout|stderr|combined)', 'combined')
    .option('--no-color', 'Disable color output')
    .option('--tail <n>', '(deprecated) Alias for --lines')
    .description('View logs for a headless agent run')
    .action(async (runId: string, options: LogsOptions) => {
      // Handle deprecated --tail option
      if (options.tail !== undefined) {
        logger.warn('--tail is deprecated. Use -n or --lines instead.')
        if (options.lines === undefined) {
          const parsed = parseInt(options.tail, 10)
          if (!isNaN(parsed) && parsed > 0) {
            options.lines = parsed
          }
        }
      }

      // Validate stream type
      if (!isValidStreamType(options.stream)) {
        logger.error(`Invalid stream type '${options.stream}'. Valid options: ${VALID_STREAM_TYPES.join(', ')}`)
        process.exit(1)
      }

      const stream = options.stream as StreamType
      const logPath = getLogPath(runId, stream)

      if (!fs.existsSync(logPath)) {
        if (!runExists(runId)) {
          logger.error(`Run '${runId}' not found. Use 'crewchief runs list' to see available runs.`)
          process.exit(1)
        } else {
          logger.error(
            `No ${stream} logs found for run '${runId}'. ` +
              'Agent may not have been started in headless mode or logs were not enabled.',
          )
          process.exit(1)
        }
      }

      if (options.follow) {
        // Use tail -f behavior via child process spawn
        const tailArgs = ['-f', logPath]

        // If --lines is also specified alongside --follow, show last N lines then follow
        if (options.lines !== undefined && options.lines > 0) {
          tailArgs.unshift('-n', String(options.lines))
        }

        const tail = spawn('tail', tailArgs, { stdio: 'inherit' })

        // Handle SIGINT to cleanly exit
        const sigintHandler = () => {
          tail.kill()
          process.exit(0)
        }
        process.on('SIGINT', sigintHandler)

        tail.on('exit', (code) => {
          process.removeListener('SIGINT', sigintHandler)
          process.exit(code ?? 0)
        })

        tail.on('error', (err) => {
          logger.error(`Failed to follow logs: ${err.message}`)
          process.exit(1)
        })
      } else {
        // Static read mode
        const content = await fs.promises.readFile(logPath, 'utf-8')
        const lines = content.split('\n')

        // Slice to last N lines if --lines specified
        const toShow = options.lines !== undefined && options.lines > 0 ? lines.slice(-options.lines) : lines

        // Apply coloring if enabled (--color is true by default via Commander --no-color)
        const output = options.color !== false ? colorize(toShow) : toShow
        process.stdout.write(output.join('\n'))

        // Add trailing newline if content doesn't end with one
        if (output.length > 0 && output[output.length - 1] !== '') {
          process.stdout.write('\n')
        }
      }
    })

  program.addCommand(runs)
}
