#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Command } from 'commander'
import inquirer from 'inquirer'
import { registerAgentCommands } from './agent'
import { registerCompetitionCommands } from './competition'
import { registerDoctorCommand } from './doctor'
import { registerMaproomCommands } from './maproom'
import { registerOptimizationCommands } from './optimization'
import { registerSetupCommand, runSetupWizard } from './setup'
import { registerWorktreeCommands } from './worktree'
import { loadConfig } from '../config/loader'
import { startOrchestratorEventBridge } from '../orchestrator/events'
import { TerminalFactory } from '../terminal/factory'
import { logger } from '../utils/logger'

const program = new Command()

function resolvePackageVersion(): string {
  try {
    const here = path.dirname(fileURLToPath(import.meta.url))
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
registerAgentCommands(program)
registerCompetitionCommands(program)
registerMaproomCommands(program)
registerOptimizationCommands(program)
registerSetupCommand(program)
registerDoctorCommand(program)

// Default behavior: `crewchief` with no subcommand starts/attaches session.
program.action(async () => {
  try {
    // Try to load config; if missing, run setup automatically
    let config
    try {
      config = await loadConfig()
    } catch (e: any) {
      if (String(e?.message || '').includes('Missing configuration file')) {
        logger.info('No configuration found. Running setup...')
        await runSetupWizard()
        config = await loadConfig()
      } else {
        throw e
      }
    }

    // Initialize terminal provider
    const terminal = TerminalFactory.autoDetect()
    await terminal.initialize()

    if (terminal.id === 'headless') {
      logger.success('✅ Running in Headless Mode')
    } else {
      logger.success(`✅ Running in ${terminal.id}`)
    }

    // Auto-launch default root agents if configured
    const defaults = (config as any).defaults
    const launch = (config as any).launch

    // DEPRECATED: Auto-launching agents is no longer supported
    // Users should use `crewchief agent spawn <agent>` command instead
    const ensureAgentRunning = async (typeId: string) => {
      logger.warn(`Auto-launching agents is deprecated. Please use: crewchief agent spawn ${typeId}`)
      logger.info('Example: crewchief agent spawn claude "my-task"')
      logger.info('See: crewchief agent spawn --help')
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
    logger.info('CrewChief is ready. Use `crewchief agent spawn` to launch agents.')
  } catch (err: any) {
    logger.error('Failed to start CrewChief:', err)
    process.exitCode = 1
  }
})

program.parseAsync(process.argv)
