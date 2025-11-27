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
    .action(async (agents: string, task: string | undefined, options: SpawnOptions) => {
      try {
        // Detect terminal provider
        const terminal = TerminalFactory.autoDetect()
        await terminal.initialize()

        // Use the new Scheduler architecture
        const scheduler = new Scheduler(terminal)

        // Parse comma-separated agent types
        const agentTypes = agents.split(',').map((a) => a.trim()).filter((a) => a.length > 0)

        if (agentTypes.length === 0) {
          console.error(chalk.red('❌ No valid agent types specified'))
          process.exit(1)
        }

        // Determine effective task name/description
        const effectiveTask = task || options.name || `agent-${Date.now()}`

        if (agentTypes.length === 1) {
          // Single agent - existing logic
          console.log(chalk.cyan(`🚀 Spawning agent ${agentTypes[0]} via ${terminal.id}...`))

          const runId = await scheduler.assignSingleAgent(effectiveTask, agentTypes[0])

          console.log(chalk.green(`✅ Agent spawned successfully [Run ID: ${runId}]`))
        } else {
          // Multi-agent spawn
          console.log(chalk.cyan(`🚀 Spawning ${agentTypes.length} agents via ${terminal.id}...`))

          const results = await Promise.allSettled(
            agentTypes.map((type) => scheduler.assignSingleAgent(effectiveTask, type))
          )

          // Report results for each agent
          let successCount = 0
          results.forEach((result, i) => {
            if (result.status === 'fulfilled') {
              console.log(chalk.green(`✅ ${agentTypes[i]}: spawned [Run ID: ${result.value}]`))
              successCount++
            } else {
              console.log(chalk.red(`❌ ${agentTypes[i]}: ${result.reason?.message || result.reason}`))
            }
          })

          console.log(chalk.cyan(`\n📊 Summary: ${successCount}/${agentTypes.length} agents spawned successfully`))

          if (successCount === 0) {
            process.exit(1)
          }
        }

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
