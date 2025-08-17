#!/usr/bin/env node
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Command } from 'commander'
// Removed `init` per new spec: setup covers initialization
import inquirer from 'inquirer'
import { registerAgentCommands } from './agent'
import { registerBuildCommand } from './build'
import { registerCompetitionCommands } from './competition'
import { registerDoctorCommand } from './doctor'
import { registerEvalCommands } from './eval'
import { registerMaproomCommands } from './maproom'
import { registerMergeCommands } from './merge'
import { registerOpsdeckCommand } from './opsdeck'
import { registerReleaseCommand } from './release'
import { registerRunsCommands } from './runs'
import { registerSetupCommand, runSetupWizard } from './setup'
import { registerTaskCommands } from './task'
import { registerWorktreeCommands } from './worktree'
// Backwards-compat session subcommand removed; `crewchief` is the entrypoint
// registerSessionCommands(program);
import { getAgentType } from '../agents/registry'
import { messageBus } from '../bus/index'
import { LogFollower } from '../bus/logFollower'
import { loadConfig } from '../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees'
import { startOrchestratorEventBridge } from '../orchestrator/events'
import { RunManager } from '../orchestrator/runManager'
import { TmuxService } from '../tmux/tmux.service'
import { logger } from '../utils/logger'

const program = new Command()

function resolvePackageVersion(): string {
  try {
    const here = path.dirname(fileURLToPath(import.meta.url))
    // Works in both src (dev) and dist (built) layouts
    const pkgPath = path.resolve(here, '..', '..', 'package.json')
    const raw = fs.readFileSync(pkgPath, 'utf8')
    const pkg = JSON.parse(raw)
    return String(pkg.version || '0.0.0')
  } catch {
    return '0.0.0'
  }
}

program
  .name('crewchief')
  .description('CrewChief: Multi-agent orchestration for AI agents via git worktrees and tmux')
  .version(resolvePackageVersion())

// Start event bridge for orchestrator messages
startOrchestratorEventBridge()

registerWorktreeCommands(program)
registerAgentCommands(program)
registerRunsCommands(program)
registerEvalCommands(program)
registerTaskCommands(program)
registerMergeCommands(program)
registerCompetitionCommands(program)
registerOpsdeckCommand(program)
registerMaproomCommands(program)
registerSetupCommand(program)
registerDoctorCommand(program)
registerBuildCommand(program)
registerReleaseCommand(program)

// Default behavior: `crewchief` with no subcommand starts/attaches session.
program.action(async () => {
  try {
    // Try to load config; if missing, run setup automatically
    let config
    try {
      config = await loadConfig()
    } catch (e: any) {
      if (String(e?.message || '').includes('Missing crewchief.config.ts')) {
        logger.info('No configuration found. Running setup...')
        await runSetupWizard()
        config = await loadConfig()
      } else {
        throw e
      }
    }

    const tmux = new TmuxService(config.tmux.sessionName)
    if (!tmux.hasSession()) {
      tmux.startSession()
      logger.success(`Started tmux session '${config.tmux.sessionName}'`)
    }
    // Auto-launch default root agents if configured
    const defaults = (config as any).defaults
    const launch = (config as any).launch
    if (launch?.autoStartOpsdeck) {
      const ok = spawnSync('crewchief-opsdeck', ['--version']).status === 0
      if (ok) {
        const paneId = tmux.createWindowWithCommand('crewchief-opsdeck ui --mode opsdeck --fps 2')
        logger.info(`Ops Deck requested via config; started in pane ${paneId}`)
      } else {
        logger.warn('autoStartOpsdeck enabled but `crewchief-opsdeck` not found on PATH. Skipping.')
      }
    }
    const ensureAgentRunning = async (typeId: string) => {
      const type = getAgentType(typeId)
      if (!type) {
        logger.warn(`Agent type '${typeId}' not recognized; skipping`)
        return
      }
      // Resolve execution command, with binary presence checks and fallback
      let execCmd = type.executionCommand
      const needsClaude = execCmd.startsWith('claude')
      const needsGemini = execCmd.startsWith('gemini')
      const haveClaude = spawnSync('bash', ['-lc', 'command -v claude']).status === 0
      const haveGemini = spawnSync('bash', ['-lc', 'command -v gemini']).status === 0
      if ((needsClaude && !haveClaude) || (needsGemini && !haveGemini)) {
        const mock = getAgentType('mock-agent')
        if (mock) {
          const here = path.dirname(fileURLToPath(import.meta.url))
          const pkgDir = path.resolve(here, '..')
          const cand = path.join(pkgDir, 'scripts', 'mock-agent.js')
          const fallback = path.join(process.cwd(), 'scripts', 'mock-agent.js')
          const abs = fs.existsSync(cand) ? cand : fallback
          // Placeholder; will finalize MOCK_AGENT_ID after branchName is known
          execCmd = `node ${JSON.stringify(abs)}`
          logger.warn(`'${type.executionCommand}' not found. Falling back to mock agent for '${typeId}'.`)
        } else {
          logger.error(`Required binary for '${typeId}' not found and no mock agent available.`)
          return
        }
      }
      const wt = new WorktreeService()
      const rm = new RunManager()
      const branchName = buildDeterministicBranchName({ agentTypeId: typeId })
      const worktreePath = await wt.createWorktree(
        branchName,
        config.repository.mainBranch,
        config.repository.worktreeBasePath,
      )
      // Launch command directly in tmux window, embedding cd to the worktree (avoid -c differences)
      // If we are using mock agent, set MOCK_AGENT_ID with branch suffix for uniqueness
      const finalExec = execCmd.includes('mock-agent.js')
        ? `MOCK_AGENT_ID=${typeId}-${branchName.slice(-8)} ${execCmd}`
        : execCmd
      const cmd = `cd ${JSON.stringify(worktreePath)} && ${finalExec}`
      const paneId = tmux.createWindowWithCommand(cmd)
      const windowId = tmux.getWindowIdForPane(paneId) || ''
      const target = windowId ? windowId : `${config.tmux.sessionName}`
      logger.info(`tmux target: window=${windowId} target=${target} pane=${paneId}`)
      const run = rm.createRun(typeId, '', paneId, worktreePath, branchName)
      logger.info(`run: id=${run.id} worktree=${worktreePath}`)
      const runDir = rm.getRunDir(run.id)
      rm.appendLog(run.id, 'debug.log', `tmux target: window=${windowId} target=${target} pane=${paneId}`)
      rm.appendLog(run.id, 'debug.log', `run: id=${run.id} worktree=${worktreePath}`)
      rm.appendLog(run.id, 'debug.log', `[crewchief] exec=${execCmd}`)
      // Start piping pane output to a file ASAP so we capture early output
      const logPath = `${runDir}/pane.log`
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
      // Snapshot recent pane output for visibility
      await new Promise((r) => setTimeout(r, 600))
      const snapshot = tmux.captureOutput(paneId).split('\n').slice(-50).join('\n')
      logger.info('[tmux tail]')
      rm.appendLog(run.id, 'debug.log', `[tmux tail]\n${snapshot}`)
      for (const line of snapshot.split('\n')) logger.info(line)
      logger.success(`Launched agent ${typeId} in ${worktreePath} [pane=${paneId}] [run=${run.id}]`)
    }

    if (launch?.autoRunDefaultAgents) {
      // If no defaults configured, prompt user to pick one of known agent types
      let roots: { id: string }[] = Array.isArray(defaults?.rootAgents) ? defaults.rootAgents : []
      if (roots.length === 0) {
        try {
          const { listAgentTypes } = await import('../agents/registry')
          const types = listAgentTypes()
          if (types.length > 0) {
            roots = [{ id: types[0].id }]
            logger.info(`No default root agents configured; launching '${types[0].id}' by default.`)
          }
        } catch {
          // ignore errors
        }
      }
      for (const ra of roots) await ensureAgentRunning(ra.id)
    } else {
      // Prompt to launch an agent now for first-run ergonomics
      try {
        const { listAgentTypes } = await import('../agents/registry')
        const types = listAgentTypes()
        if (types.length > 0) {
          const choices = types.map((t) => ({ name: `${t.id} (${t.platform})`, value: t.id }))
          choices.push({ name: 'Skip (start empty session)', value: '' } as any)
          const ans = await inquirer.prompt([
            { name: 'agentId', type: 'list', message: 'Start an agent now?', choices },
          ])
          if (ans.agentId) await ensureAgentRunning(ans.agentId as string)
        }
      } catch {
        // non-interactive or inquirer missing; ignore
      }
    }
    tmux.attach()
  } catch (err) {
    logger.error('Failed to start CrewChief:', err)
    process.exitCode = 1
  }
})

program.parseAsync(process.argv)
