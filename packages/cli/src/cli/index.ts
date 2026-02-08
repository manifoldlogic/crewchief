#!/usr/bin/env node
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { Command } from 'commander'
import { registerAgentCommands } from './agent'
import { registerCompetitionCommands } from './competition'
import { registerDoctorCommand } from './doctor'
import { registerMaproomCommands } from './maproom'
import { registerOptimizationCommands } from './optimization'
import { registerReleaseCommand } from './release'
import { registerSetupCommand } from './setup'
import { registerWorktreeCommands } from './worktree'
import { startOrchestratorEventBridge } from '../orchestrator/events'

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
registerReleaseCommand(program)

// Default behavior: show quick start hint
program.action(() => {
  console.log('Quick start: crewchief agent spawn claude "your task"')
  console.log('See: crewchief --help')
})

program.parseAsync(process.argv)
