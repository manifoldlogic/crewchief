import fs from 'node:fs'
import path from 'node:path'
import { Command } from 'commander'
import { listPlatforms, listAgentsForPlatform } from '../agents/platforms'
import { ITermSimpleService } from '../iterm/iterm-simple.service'
import { RunManager } from '../orchestrator/runManager'
import { Scheduler, SpawnOptions } from '../orchestrator/scheduler'
import { TerminalFactory } from '../terminal/factory'
import { logger } from '../utils/logger'

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
          // Parse platform from name (format: name__platform)
          let platform: string | undefined
          if (agentName.includes('__')) {
            const parts = agentName.split('__')
            platform = parts[parts.length - 1]
          }

          // Use iTerm2 with platform-specific Enter key (chr(13) for Claude, etc.)
          const success = iterm.sendKeys(agentName, textToSend, platform)
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
          const platform = parts[parts.length - 1]
          const taskName = parts.slice(0, -1).join('__')
          logger.info(`  ${idx + 1}. ${pane.label}`)
          logger.info(`     Platform: ${platform}, Task: ${taskName}`)
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
      const run = rm.getRunByPlatform(agentId)
      if (!run) {
        logger.warn(`Agent ${agentId} not running`)
        return
      }
      rm.updateRun(run.id, { status: 'closed' })
      logger.success(`Marked agent ${agentId} as closed [run=${run.id}]`)
      logger.info('Please manually close the terminal pane')
    })

  agent
    .command('platforms')
    .description('List available platforms and named agents')
    .addHelpText(
      'after',
      `
This command shows:
  - Built-in platforms (claude, gemini, codex, aider)
  - Named agent definitions found in your project

Use this to discover what's available before spawning.
`,
    )
    .action(async () => {
      const platforms = listPlatforms()
      const projectDir = process.cwd()

      console.log('\nAvailable Platforms:\n')

      for (const platform of platforms) {
        let agentsDisplay = 'N/A'

        if (platform.agentDir) {
          try {
            const agents = listAgentsForPlatform(platform.name, projectDir)
            if (agents.length > 0) {
              const displayAgents = agents.sort().slice(0, 5)
              agentsDisplay = displayAgents.join(', ')
              if (agents.length > 5) {
                agentsDisplay += ` ... (${agents.length - 5} more)`
              }
            } else {
              agentsDisplay = '(none found)'
            }
          } catch {
            agentsDisplay = '(error scanning)'
          }
        }

        console.log(`Platform: ${platform.name}`)
        console.log(`  Command: ${platform.command}`)
        console.log(`  Agent Dir: ${platform.agentDir || 'N/A'}`)
        console.log(`  Named Agents: ${agentsDisplay}`)
        console.log('')
      }
    })

  agent
    .command('spawn')
    .description('Spawn an AI coding agent on a task')
    .argument('<platforms>', 'Platform(s) - single or comma-separated (e.g., claude or claude,gemini)')
    .argument('[task]', 'Task description (auto-generated if omitted)')
    .option('--worktree', 'Create isolated git worktree for this task', false)
    .option('--agent <name>', 'Use named agent definition (e.g., backend-developer)')
    .option('--args <string>', 'Extra arguments to pass to platform CLI')
    .option('-v, --verbose', 'Verbose logging')
    .addHelpText(
      'after',
      `
QUICK START:
  crewchief agent spawn claude "fix the login bug"

WHAT HAPPENS:
  1. Resolves the platform (claude, gemini, aider, codex, or any command)
  2. Opens a terminal pane labeled <task>__<platform>
  3. Runs the platform command in that pane

EXAMPLES:
  # Simplest: spawn Claude on a task
  crewchief agent spawn claude "fix the login bug"

  # Use a named agent definition
  crewchief agent spawn claude --agent backend-developer "fix the login bug"

  # Spawn in an isolated git worktree
  crewchief agent spawn claude --worktree "fix the login bug"

  # Spawn multiple platforms at once
  crewchief agent spawn claude,gemini "review the code"

  # Pass extra flags to the platform CLI
  crewchief agent spawn claude --args "--dangerously-skip-permissions" "fix bug"

SEE ALSO:
  crewchief agent platforms    List available platforms and named agents
  crewchief agent list         List running agents
  crewchief agent message      Send a message to a running agent
`,
    )
    .action(
      async (
        platformsStr: string,
        task: string | undefined,
        options: { worktree: boolean; agent?: string; args?: string; verbose?: boolean },
      ) => {
        // 1. Parse platforms
        const platforms = platformsStr
          .split(',')
          .map((p) => p.trim())
          .filter((p) => p.length > 0)

        if (platforms.length === 0) {
          console.error('Error: No valid platforms specified')
          process.exit(1)
        }

        // 2. Generate task name if not provided
        const effectiveTask = task || `task-${Date.now()}`

        // 3. Create scheduler
        const terminal = TerminalFactory.autoDetect()
        const runManager = new RunManager()
        const scheduler = new Scheduler(terminal, runManager)

        // 4. Spawn each platform sequentially
        for (const platform of platforms) {
          try {
            const spawnOptions: SpawnOptions = {
              useWorktree: options.worktree,
              agentName: options.agent,
              verbose: options.verbose,
              extraArgs: options.args,
            }
            const runId = await scheduler.spawnAgent(effectiveTask, platform, spawnOptions)
            console.log(`Spawned ${platform} agent: ${runId}`)
          } catch (error: unknown) {
            const message = error instanceof Error ? error.message : String(error)
            console.error(`Failed to spawn ${platform}: ${message}`)
          }
        }
      },
    )

  program.addCommand(agent)
}
