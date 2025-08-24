import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Command } from 'commander'
import { getAgentType } from '../agents/registry'
import { messageBus } from '../bus/index'
import { LogFollower } from '../bus/logFollower'
import { loadConfig } from '../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees'
import { ITermSimpleService } from '../iterm/iterm-simple.service'
import { RunManager } from '../orchestrator/runManager'
import { TmuxService } from '../tmux/tmux.service' // DEPRECATED: tmux implementation is incomplete and no longer under development
import { logger } from '../utils/logger'

export function registerAgentCommands(program: Command): void {
  const agent = new Command('agent').description('Agent lifecycle')

  agent
    .command('spawn')
    .argument('<type>')
    .argument('[task]')
    .option('--count <n>', 'Number of agents to spawn', '1')
    .option('--branch <base>', 'Base branch name to derive worktrees from')
    .option('--env <kv...>', 'Environment variables KEY=VAL to pass to the agent')
    .description('Spawn one or more agents of <type> (optionally provide a task)')
    .action(
      async (typeId: string, task: string | undefined, options: { count: string; branch?: string; env?: string[] }) => {
        const type = getAgentType(typeId)
        if (!type) {
          logger.error(`Unknown agent type: ${typeId}`)
          process.exitCode = 1
          return
        }
        const config = await loadConfig()
        const count = Math.max(1, parseInt(options.count, 10))
        const baseBranch = options.branch ?? config.repository.mainBranch
        const envVars: Record<string, string> = {}
        for (const kv of options.env ?? []) {
          const [k, ...rest] = kv.split('=')
          if (k && rest.length) envVars[k] = rest.join('=')
        }

        // DEPRECATED: tmux implementation is incomplete - iTerm2 is required
        const tmux = new TmuxService(config.tmux.sessionName)
        tmux.ensureSession()
        const rm = new RunManager()
        const wt = new WorktreeService()

        for (let i = 0; i < count; i++) {
          const branchName = buildDeterministicBranchName({ agentTypeId: typeId, taskDescription: task })
          const worktreePath = await wt.createWorktree(branchName, baseBranch, config.repository.worktreeBasePath)

          // Determine execution command (support mock-agent replacement)
          let execCmd = type.executionCommand
          if (execCmd.includes('scripts/mock-agent.js')) {
            const here = path.dirname(fileURLToPath(import.meta.url))
            const pkgDir = path.resolve(here, '..')
            const cand = path.join(pkgDir, 'scripts', 'mock-agent.js')
            const fallback = path.join(process.cwd(), 'scripts', 'mock-agent.js')
            const abs = fs.existsSync(cand) ? cand : fallback
            // Preserve uniqueness even with deterministic branch naming by using branchName in mock id
            execCmd = `MOCK_AGENT_ID=${typeId}-${branchName.slice(-8)} node ${JSON.stringify(abs)}`
          }

          // Merge in provided env vars
          const envArgs = Object.entries(envVars)
            .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
            .join(' ')
          const fullCmd = `${envArgs} ${execCmd}`.trim()

          // Launch command directly in a new tmux window; embed cd into the command
          const cmd = `cd ${JSON.stringify(worktreePath)} && ${fullCmd}`
          const paneId = tmux.createWindowWithCommand(cmd)
          const run = rm.createRun(typeId, task ?? '', paneId, worktreePath, branchName)

          const logPath = `${rm.getRunDir(run.id)}/pane.log`
          tmux.pipePaneToFile(paneId, logPath, true)
          const follower = new LogFollower(logPath)
          follower.start((env) => {
            rm.appendLog(run.id, 'events.log', JSON.stringify(env))
            messageBus.send({
              type: 'status',
              from: `${typeId}`,
              to: 'orchestrator',
              payload: env,
              timestamp: new Date(),
              worktreeContext: { branch: run.branchName ?? '', modifiedFiles: [], lastCommit: '' },
            })
          })

          logger.success(`Spawned ${typeId} in ${worktreePath} [pane=${paneId}] [run=${run.id}]`)
        }
      },
    )

  agent
    .command('message')
    .argument('<agentName>')
    .argument('[message]')
    .option('-f, --file <path>', 'Read message from a file')
    .description('Send a message to an agent by name (e.g., fix-bug__claude)')
    .action(async (agentName: string, message: string | undefined, options: { file?: string }) => {
      const config = await loadConfig()

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
        // Parse agent type from name (format: name__type)
        let agentType: string | undefined
        if (agentName.includes('__')) {
          const parts = agentName.split('__')
          agentType = parts[parts.length - 1]
        }

        // Use iTerm2 with agent-specific Enter key (chr(13) for Claude, etc.)
        const success = iterm.sendKeys(agentName, textToSend, agentType)
        if (success) {
          if (options.file) {
            logger.info(`[${agentName}] <= <contents of ${options.file}> [iTerm2]`)
          } else {
            logger.info(`[${agentName}] <= ${textToSend} [iTerm2]`)
          }
        } else {
          logger.error(`Failed to send message to ${agentName}`)
        }
      } else {
        // DEPRECATED: tmux implementation is incomplete and no longer under development
        // Users should install iTerm2 for proper agent communication
        logger.error('iTerm2 is required for agent messaging. Please install iTerm2.')
        logger.error('Visit: https://iterm2.com/downloads.html')
        logger.warn('Attempting tmux fallback (incomplete implementation)...')
        const tmux = new TmuxService(config.tmux?.sessionName || 'crewchief')
        tmux.sendKeys(run.paneId, textToSend)
        rm.appendLog(run.id, 'messages.log', `[in] ${textToSend}`)
        logger.info(`[${agentId}] <= ${textToSend} [tmux:${run.paneId}] [run=${run.id}]`)
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
    .description('Close an agent and optionally merge work (mock)')
    .action(async (agentId: string) => {
      const rm = new RunManager()
      const run = rm.getRunByAgentType(agentId)
      if (!run) {
        logger.warn(`Agent ${agentId} not running`)
        return
      }
      const config = await loadConfig()
      const tmux = new TmuxService(config.tmux.sessionName)
      tmux.closePane(run.paneId)
      rm.updateRun(run.id, { status: 'closed' })
      logger.success(`Closed agent ${agentId} [pane=${run.paneId}] [run=${run.id}]`)
    })

  program.addCommand(agent)
}
