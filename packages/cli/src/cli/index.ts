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
import { registerSpawnCommand } from './spawn'
// Backwards-compat session subcommand removed; `crewchief` is the entrypoint
// registerSessionCommands(program);
import { getAgentType } from '../agents/registry'
import { messageBus } from '../bus/index'
import { LogFollower } from '../bus/logFollower'
import { loadConfig } from '../config/loader'
import { WorktreeService, buildDeterministicBranchName } from '../git/worktrees'
import { startOrchestratorEventBridge } from '../orchestrator/events'
import { RunManager } from '../orchestrator/runManager'
import { TmuxService } from '../tmux/tmux.service' // DEPRECATED: tmux implementation is incomplete - iTerm2 is required
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
  .description('Git worktree management, semantic code search, and multi-agent orchestration')
  .version(resolvePackageVersion())

// Start event bridge for orchestrator messages
startOrchestratorEventBridge()

registerWorktreeCommands(program)
registerSpawnCommand(program)
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

    // Check if iTerm2 is available
    if (process.env.TERM_PROGRAM !== 'iTerm.app') {
      logger.error('❌ iTerm2 is required to run CrewChief')
      logger.error('   The tmux implementation is incomplete and no longer under development.')
      logger.error('   Please install iTerm2: https://iterm2.com/downloads.html')
      logger.error('   Then run CrewChief from within iTerm2.')
      process.exit(1)
    }
    
    logger.success('✅ Running in iTerm2')
    
    // Auto-launch default root agents if configured
    const defaults = (config as any).defaults
    const launch = (config as any).launch
    
    // DEPRECATED: Opsdeck auto-launch requires tmux which is no longer supported
    if (launch?.autoStartOpsdeck) {
      logger.warn('autoStartOpsdeck is not supported with iTerm2. Use `crewchief spawn` instead.')
    }
    // DEPRECATED: Auto-launching agents requires tmux which is no longer supported
    // Users should use `crewchief spawn <agent>` command instead
    const ensureAgentRunning = async (typeId: string) => {
      logger.warn(`Auto-launching agents is deprecated. Please use: crewchief spawn ${typeId}`)
      logger.info('Example: crewchief spawn claude "my-task"')
      logger.info('See: crewchief spawn --help')
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
    // iTerm2 doesn't need explicit attach - already in the terminal
    logger.info('CrewChief is ready. Use `crewchief spawn` to launch agents.')
  } catch (err) {
    logger.error('Failed to start CrewChief:', err)
    process.exitCode = 1
  }
})

program.parseAsync(process.argv)
