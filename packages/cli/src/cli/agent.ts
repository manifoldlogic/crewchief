import fs from 'node:fs'
import path from 'node:path'
import chalk from 'chalk'
import { Command } from 'commander'
import { ITermSimpleService } from '../iterm/iterm-simple.service'
import { RunManager } from '../orchestrator/runManager'
import { Scheduler } from '../orchestrator/scheduler'
import { TerminalFactory } from '../terminal/factory'
import { logger } from '../utils/logger'

interface SpawnOptions {
  name?: string
  vertical?: boolean
  args?: string
  noLabel?: boolean
  backend?: string
  headless?: boolean
}

/** Known backend identifiers accepted by --backend */
export const VALID_BACKENDS = ['iterm', 'tmux', 'headless', 'auto'] as const

/**
 * Validate a backend string against known providers.
 * Returns true if valid, false otherwise.
 */
export function validateBackend(backend: string): backend is (typeof VALID_BACKENDS)[number] {
  return (VALID_BACKENDS as readonly string[]).includes(backend)
}

export function registerAgentCommands(program: Command): void {
  const agent = new Command('agent').description('Agent communication and management')

  agent
    .command('message')
    .argument('<pattern>')
    .argument('[message]')
    .option('-f, --file <path>', 'Read message from a file')
    .option('-a, --all', 'Send to all agents matching the pattern')
    .description('Send a message to agent(s) by name or pattern')
    .addHelpText(
      'after',
      `
Examples:
  Send to specific agent:
    crewchief agent message fix-bug__claude "Add OAuth support"
    
  Send to all agents with same task:
    crewchief agent message fix-bug --all "Update your approach"
    
  Send to all agents:
    crewchief agent message "*" --all "Status update please"
    
  Send file to multiple agents:
    crewchief agent message implement-auth --all --file prompt.md`,
    )
    .action(async (pattern: string, message: string | undefined, options: { file?: string; all?: boolean }) => {
      // Determine the message to send
      let textToSend: string
      if (options.file) {
        // Read message from file
        try {
          const filePath = path.resolve(options.file)
          if (!fs.existsSync(filePath)) {
            logger.error(`File not found: ${filePath}`)
            process.exit(1)
          }
          textToSend = fs.readFileSync(filePath, 'utf-8')
          logger.info(`Reading message from file: ${filePath}`)
        } catch (error) {
          logger.error(`Error reading file: ${error}`)
          process.exit(1)
        }
      } else if (message) {
        textToSend = message
      } else {
        logger.error('Either provide a message or use --file option')
        process.exit(1)
      }

      // Detect backend and use appropriate service
      const iterm = new ITermSimpleService()
      if (iterm.isAvailable()) {
        // Get list of agents to send to
        let targetAgents: string[] = []

        if (options.all) {
          // Get all agent panes
          const panes = iterm.listPanes()
          const agentPanes = panes.filter((p) => p.label && p.label.includes('__'))

          // Filter by pattern
          if (pattern === '*') {
            // Send to all agents
            targetAgents = agentPanes.map((p) => p.label)
          } else if (pattern.includes('__')) {
            // Full agent name provided with --all, just send to that one
            targetAgents = agentPanes.filter((p) => p.label === pattern).map((p) => p.label)
          } else {
            // Pattern is a task name prefix
            targetAgents = agentPanes
              .filter((p) => p.label.startsWith(pattern + '__') || p.label.includes('__' + pattern))
              .map((p) => p.label)
          }

          if (targetAgents.length === 0) {
            logger.error(`No agents found matching pattern: ${pattern}`)
            process.exit(1)
          }

          logger.info(`Sending to ${targetAgents.length} agent(s) matching '${pattern}'`)
        } else {
          // Single agent mode
          targetAgents = [pattern]
        }

        // Send to each target agent
        let successCount = 0
        for (const agentName of targetAgents) {
          // Parse agent type from name (format: name__type)
          let agentType: string | undefined
          if (agentName.includes('__')) {
            const parts = agentName.split('__')
            agentType = parts[parts.length - 1]
          }

          // Use iTerm2 with agent-specific Enter key (chr(13) for Claude, etc.)
          const success = iterm.sendKeys(agentName, textToSend, agentType)
          if (success) {
            successCount++
            if (options.file) {
              logger.info(`[${agentName}] <= <contents of ${options.file}> [iTerm2]`)
            } else {
              // For multiple agents, show abbreviated message
              const displayMsg =
                options.all && targetAgents.length > 1
                  ? textToSend.length > 50
                    ? textToSend.substring(0, 50) + '...'
                    : textToSend
                  : textToSend
              logger.info(`[${agentName}] <= ${displayMsg} [iTerm2]`)
            }
          } else {
            logger.error(`Failed to send message to ${agentName}`)
          }
        }

        if (options.all) {
          logger.info(`Successfully sent to ${successCount}/${targetAgents.length} agents`)
        }
      } else {
        logger.error('iTerm2 is required for agent messaging. Please install iTerm2.')
        logger.error('Visit: https://iterm2.com/downloads.html')
        process.exit(1)
      }
    })

  agent
    .command('list')
    .description('List running agents in iTerm2')
    .action(async () => {
      const iterm = new ITermSimpleService()
      if (iterm.isAvailable()) {
        const panes = iterm.listPanes()

        if (panes.length === 0) {
          logger.info('No panes found')
          return
        }

        // Filter for agent panes (those with __ in the label)
        const agentPanes = panes.filter((p) => p.label && p.label.includes('__'))

        if (agentPanes.length === 0) {
          logger.info('No agent panes found')
          logger.info('(Agent panes have names like: task-name__claude)')
          return
        }

        logger.info('Running agents:')
        agentPanes.forEach((pane, idx) => {
          const parts = pane.label.split('__')
          const agentType = parts[parts.length - 1]
          const taskName = parts.slice(0, -1).join('__')
          logger.info(`  ${idx + 1}. ${pane.label}`)
          logger.info(`     Type: ${agentType}, Task: ${taskName}`)
          logger.info(`     Session: ${pane.sessionId.substring(0, 8)}...`)
        })

        logger.info('')
        logger.info('Send messages with: crewchief agent message <name> <message>')
        logger.info('Example: crewchief agent message fix-login-bug5__claude "your message"')
      } else {
        logger.error('iTerm2 is required for agent listing')
      }
    })

  agent
    .command('close')
    .argument('<agentId>')
    .description('Close an agent (requires manual pane closure)')
    .action(async (agentId: string) => {
      const rm = new RunManager()
      const run = rm.getRunByAgentType(agentId)
      if (!run) {
        logger.warn(`Agent ${agentId} not running`)
        return
      }
      rm.updateRun(run.id, { status: 'closed' })
      logger.success(`Marked agent ${agentId} as closed [run=${run.id}]`)
      logger.info('Please manually close the terminal pane')
    })

  agent
    .command('spawn')
    .description('Spawn AI agent(s) in dedicated terminal pane(s)')
    .argument('<agents>', 'Agent type(s) - single or comma-separated (e.g., claude or claude,gemini)')
    .argument('[task]', 'Optional task description to include in agent name(s)')
    .option('-n, --name <name>', 'Custom name for the agent')
    .option('-v, --vertical', 'Split pane vertically instead of horizontally')
    .option('-a, --args <args>', 'Additional arguments to pass to the agent command')
    .option('--no-label', 'Skip labeling the pane')
    .option('--backend <backend>', 'Terminal backend (iterm|tmux|headless|auto, default: auto)')
    .option('--headless', 'Force headless mode (no terminal UI)')
    .addHelpText(
      'after',
      `
Examples:
  Spawn a Claude agent:
    crewchief agent spawn claude "fix login bug"

  Spawn in vertical split:
    crewchief agent spawn claude -v "add tests"

  Spawn in headless mode:
    crewchief agent spawn gemini --headless "refactor module"

  Spawn multiple agents:
    crewchief agent spawn claude,gemini "review code"

  Spawn with explicit tmux backend (Linux/server):
    crewchief agent spawn claude "fix bug" --backend tmux

  Spawn with explicit iTerm2 backend (macOS):
    crewchief agent spawn claude "add tests" --backend iterm

  Spawn with explicit headless backend:
    crewchief agent spawn claude "refactor" --backend headless

  Auto-detect backend (default):
    crewchief agent spawn claude "implement feature" --backend auto

Backend Options:
  iterm    - iTerm2 terminal (macOS only, requires iTerm.app)
  tmux     - tmux multiplexer (Linux/macOS, requires tmux >= 2.1)
  headless - Background process (no terminal UI, logs to files)
  auto     - Auto-detect based on environment (default)
`,
    )
    .action(async (agents: string, task: string | undefined, options: SpawnOptions) => {
      try {
        // Validate --backend option if provided
        if (options.backend && !validateBackend(options.backend)) {
          logger.error(`Invalid backend: ${options.backend}\n` + `Valid options: ${VALID_BACKENDS.join(', ')}`)
          process.exit(1)
        }

        // Warn if both --headless and --backend are specified (--headless takes precedence)
        if (options.headless && options.backend && options.backend !== 'headless' && options.backend !== 'auto') {
          logger.warn(`Both --headless and --backend ${options.backend} specified. ` + '--headless takes precedence.')
        }

        // Determine terminal provider: --headless > --backend > auto-detect
        let terminal
        if (options.headless) {
          terminal = TerminalFactory.getProvider('headless')
        } else if (options.backend && options.backend !== 'auto') {
          terminal = TerminalFactory.getProvider(options.backend as 'iterm' | 'tmux' | 'headless')
        } else {
          terminal = TerminalFactory.autoDetect()
        }

        // Initialize with helpful error messages on failure
        try {
          await terminal.initialize()
        } catch (initErr: unknown) {
          const initMessage = initErr instanceof Error ? initErr.message : String(initErr)
          logger.error(`Failed to initialize ${options.backend || 'auto-detected'} backend: ${initMessage}`)

          if (initMessage.includes('tmux not found') || initMessage.includes('tmux: not found')) {
            logger.info(
              'Install tmux:\n' +
                '  Ubuntu/Debian: sudo apt install tmux\n' +
                '  macOS: brew install tmux\n' +
                '  Or use --backend headless for non-terminal operation',
            )
          } else if (initMessage.includes('iTerm') || initMessage.includes('iterm')) {
            logger.info(
              'iTerm2 is macOS-only. On Linux, use:\n' +
                '  --backend tmux   (terminal multiplexer)\n' +
                '  --backend headless (background process)',
            )
          } else if (initMessage.includes('too old') || initMessage.includes('version')) {
            logger.info(
              'Upgrade tmux:\n' +
                '  Ubuntu/Debian: sudo apt update && sudo apt install tmux\n' +
                '  macOS: brew upgrade tmux',
            )
          } else {
            logger.info('Try --backend headless for non-terminal operation')
          }

          process.exit(1)
        }

        // Use the Scheduler architecture
        const scheduler = new Scheduler(terminal)

        // Parse comma-separated agent types
        const agentTypes = agents
          .split(',')
          .map((a) => a.trim())
          .filter((a) => a.length > 0)

        if (agentTypes.length === 0) {
          console.error(chalk.red('No valid agent types specified'))
          process.exit(1)
        }

        // Determine effective task name/description
        const effectiveTask = task || options.name || `agent-${Date.now()}`

        if (agentTypes.length === 1) {
          // Single agent - existing logic
          console.log(chalk.cyan(`Spawning agent ${agentTypes[0]} via ${terminal.id}...`))

          const runId = await scheduler.assignSingleAgent(effectiveTask, agentTypes[0])

          console.log(chalk.green(`Agent spawned successfully [Run ID: ${runId}]`))
        } else {
          // Multi-agent spawn
          console.log(chalk.cyan(`Spawning ${agentTypes.length} agents via ${terminal.id}...`))

          const results = await Promise.allSettled(
            agentTypes.map((type) => scheduler.assignSingleAgent(effectiveTask, type)),
          )

          // Report results for each agent
          let successCount = 0
          results.forEach((result, i) => {
            if (result.status === 'fulfilled') {
              console.log(chalk.green(`${agentTypes[i]}: spawned [Run ID: ${result.value}]`))
              successCount++
            } else {
              console.log(chalk.red(`${agentTypes[i]}: ${result.reason?.message || result.reason}`))
            }
          })

          console.log(chalk.cyan(`\nSummary: ${successCount}/${agentTypes.length} agents spawned successfully`))

          if (successCount === 0) {
            process.exit(1)
          }
        }

        if (terminal.id === 'headless') {
          console.log(chalk.blue('Running in headless mode. Press Ctrl+C to stop all agents.'))
          // Keep the process alive to stream logs and manage child processes
          await new Promise(() => {})
        } else {
          // For iTerm, we can exit, the window is independent
          process.exit(0)
        }
      } catch (error: unknown) {
        const message = error instanceof Error ? error.message : String(error)
        console.error(chalk.red('Error spawning agent:'), message)
        process.exit(1)
      }
    })

  program.addCommand(agent)
}
