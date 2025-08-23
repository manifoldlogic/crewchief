#!/usr/bin/env node
/**
 * Spawn command for creating new agents in iTerm2
 */

import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import chalk from 'chalk'
import { Command } from 'commander'
import { loadConfig } from '../config/loader.js'

const __filename = fileURLToPath(import.meta.url)
const __dirname = dirname(__filename)

interface SpawnOptions {
  name?: string
  vertical?: boolean
  args?: string
  noLabel?: boolean
}

function findITermScriptsDir(): string | null {
  // Look for iterm_scripts directory in multiple locations
  const possiblePaths = [
    // Relative to CLI dist directory
    join(__dirname, '..', '..', '..', '..', 'iterm_scripts'),
    // Relative to project root
    join(process.cwd(), 'iterm_scripts'),
    // In .crewchief directory
    join(process.cwd(), '.crewchief', 'iterm_scripts'),
    // Absolute path in home directory
    join(process.env.HOME || '', '.crewchief', 'iterm_scripts'),
  ]

  for (const path of possiblePaths) {
    const scriptPath = join(path, 'spawn_agent.py')
    if (existsSync(scriptPath)) {
      return path
    }
  }

  return null
}

function detectTerminalBackend(): 'iterm' | 'tmux' | null {
  // Check if we're in iTerm2
  if (process.env.TERM_PROGRAM === 'iTerm.app') {
    return 'iterm'
  }

  // Check if tmux is available
  const tmuxCheck = spawnSync('which', ['tmux'], { encoding: 'utf-8' })
  if (tmuxCheck.status === 0) {
    return 'tmux'
  }

  return null
}

export function registerSpawnCommand(program: Command): void {
  program
    .command('spawn')
    .description('Spawn a new AI agent in a dedicated terminal pane with its own worktree')
    .argument('<agent>', 'Agent type (claude, gemini, gpt, cursor, aider, or custom command)')
    .argument('[task]', 'Optional task description to include in agent name')
    .option('-n, --name <name>', 'Custom name for the agent')
    .option('-v, --vertical', 'Split pane vertically instead of horizontally')
    .option('-a, --args <args>', 'Additional arguments to pass to the agent command')
    .option('--no-label', 'Skip labeling the pane')
    .option('--backend <backend>', 'Force specific backend (iterm or tmux)')
    .action(async (agent: string, task: string | undefined, options: SpawnOptions) => {
      try {
        const config = await loadConfig()

        // Detect or use specified backend
        const backend = options.backend || config.terminal?.backend || 'auto'
        const detectedBackend = backend === 'auto' ? detectTerminalBackend() : backend

        if (!detectedBackend) {
          console.error(chalk.red('❌ No suitable terminal backend found'))
          console.error(chalk.yellow('   Please install iTerm2 (macOS) or tmux'))
          process.exit(1)
        }

        if (detectedBackend === 'tmux') {
          console.error(chalk.yellow('⚠️  Tmux backend not yet implemented for spawn command'))
          console.error(chalk.dim('   Please use iTerm2 or implement tmux support'))
          process.exit(1)
        }

        if (detectedBackend === 'iterm') {
          // Find iterm_scripts directory
          const scriptsDir = findITermScriptsDir()
          if (!scriptsDir) {
            console.error(chalk.red('❌ Could not find iterm_scripts directory'))
            console.error(chalk.yellow('   Make sure iTerm2 scripts are installed'))
            process.exit(1)
          }

          // Use smart spawning with intelligent pane management
          const spawnScript = existsSync(join(scriptsDir, 'spawn_agent_smart.py'))
            ? join(scriptsDir, 'spawn_agent_smart.py')
            : join(scriptsDir, 'spawn_agent.py')

          // Generate agent name in format: {name}__{agent}
          let baseName: string
          if (options.name) {
            // Use provided name
            baseName = options.name
          } else if (task) {
            // Use task as the name
            baseName = task.replace(/\s+/g, '-').toLowerCase()
          } else {
            // Generate a simple name with timestamp
            const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
            baseName = `${agent}-${timestamp}`
          }

          // Create the base worktree name
          let agentName = `${baseName}__${agent}`

          // Get current working directory (project directory)
          const projectDir = process.cwd()

          // Check if worktree already exists
          const worktreePath = join(projectDir, '.crewchief', 'worktrees', agentName)
          if (existsSync(worktreePath)) {
            // Append timestamp if worktree exists
            const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
            agentName = `${baseName}__${agent}_${timestamp}`
            console.log(chalk.yellow(`⚠️  Worktree ${baseName}__${agent} exists, using ${agentName}`))
          }

          // Build spawn command arguments
          const args = [spawnScript, agent, '--name', agentName, '--project-dir', projectDir]

          if (options.vertical) {
            args.push('--vertical')
          }

          if (options.args) {
            args.push('--args', options.args)
          }

          if (options.noLabel) {
            args.push('--no-label')
          }

          console.log(chalk.cyan('🚀 Spawning agent via iTerm2...'))
          console.log(chalk.dim(`   Script: ${spawnScript}`))
          console.log(chalk.dim(`   Agent: ${agent}`))
          console.log(chalk.dim(`   Name: ${agentName}`))
          console.log(chalk.dim(`   Project: ${projectDir}`))

          // Execute the spawn script
          const result = spawnSync('python3', args, {
            stdio: 'inherit',
            encoding: 'utf-8',
          })

          if (result.status !== 0) {
            console.error(chalk.red('❌ Failed to spawn agent'))
            if (result.error) {
              console.error(chalk.dim(`   Error: ${result.error.message}`))
            }
            process.exit(1)
          }

          console.log(chalk.green('✅ Agent spawned successfully'))
          console.log(chalk.dim("   Use 'crewchief agent list' to see all agents"))
          console.log(chalk.dim(`   Use 'crewchief agent message ${agentName} <text>' to send commands`))
        }
      } catch (error) {
        console.error(chalk.red('❌ Error spawning agent:'), error)
        process.exit(1)
      }
    })
    .addHelpText(
      'after',
      `
Examples:
  $ crewchief spawn claude                    # Spawn Claude with auto-generated name
  $ crewchief spawn claude "auth-feature"     # Include task in name
  $ crewchief spawn claude --name my-agent    # Use custom name
  $ crewchief spawn gemini --vertical         # Split vertically
  $ crewchief spawn claude --args "--model claude-3-opus"  # Pass args to agent

Supported agents:
  - claude    Anthropic's Claude
  - gemini    Google's Gemini  
  - gpt       OpenAI GPT
  - cursor    Cursor AI
  - aider     Aider coding assistant
  - custom    Any custom command

The agent will be spawned in:
  1. A new iTerm2 pane (split from current)
  2. Its own git worktree (.crewchief/worktrees/<name>)
  3. With a visual badge and label for identification`,
    )
}
