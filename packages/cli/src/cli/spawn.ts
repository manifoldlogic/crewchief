import { spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import chalk from 'chalk'
import { Command } from 'commander'
import { Scheduler } from '../orchestrator/scheduler'
import { TerminalFactory } from '../terminal/factory'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

interface SpawnOptions {
  name?: string
  vertical?: boolean
  args?: string
  noLabel?: boolean
  backend?: string
  headless?: boolean
}

// eslint-disable-next-line @typescript-eslint/no-unused-vars
function _resolveRepoRoot(startDir: string): string {
  try {
    const res = spawnSync('git', ['rev-parse', '--git-common-dir'], {
      cwd: startDir,
      encoding: 'utf-8',
    })
    if (res.status === 0) {
      const commonDir = res.stdout.trim()
      const commonAbs = resolve(startDir, commonDir)
      return dirname(commonAbs)
    }
  } catch {}
  return startDir
}

export function registerSpawnCommand(program: Command): void {
  program
    .command('spawn')
    .description('Spawn AI agent(s) in dedicated terminal pane(s) with their own worktrees')
    .argument('<agents>', 'Agent type(s) - single or comma-separated (e.g., claude or claude,gemini,codex)')
    .argument('[task]', 'Optional task description to include in agent name(s)')
    .option('-n, --name <name>', 'Custom name for the agent')
    .option('-v, --vertical', 'Split pane vertically instead of horizontally')
    .option('-a, --args <args>', 'Additional arguments to pass to the agent command')
    .option('--no-label', 'Skip labeling the pane')
    .option('--backend <backend>', 'Force specific backend (iterm only)')
    .option('--headless', 'Force headless mode (no terminal UI)')
    .action(async (agent: string, task: string | undefined, options: SpawnOptions) => {
      try {
        // Detect terminal provider
        const terminal = TerminalFactory.autoDetect()
        await terminal.initialize()

        // Use the new Scheduler architecture
        const scheduler = new Scheduler(terminal)

        if (agent.includes(',')) {
          console.error(chalk.red('❌ Multi-agent spawn not yet supported in new architecture'))
          process.exit(1)
        }

        // Determine effective task name/description
        const effectiveTask = task || options.name || `agent-${Date.now()}`

        console.log(chalk.cyan(`🚀 Spawning agent ${agent} via ${terminal.id}...`))

        const runId = await scheduler.assignSingleAgent(effectiveTask, agent)

        console.log(chalk.green(`✅ Agent spawned successfully [Run ID: ${runId}]`))

        if (terminal.id === 'headless') {
          console.log(chalk.blue('ℹ️  Running in headless mode. Press Ctrl+C to stop all agents.'))
          // Keep the process alive to stream logs and manage child processes
          await new Promise(() => {})
        } else {
          // For iTerm, we can exit, the window is independent
          process.exit(0)
        }
      } catch (error: any) {
        console.error(chalk.red('❌ Error spawning agent:'), error.message)
        process.exit(1)
      }
    })
}
