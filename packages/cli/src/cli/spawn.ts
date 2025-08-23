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
    .description('Spawn AI agent(s) in dedicated terminal pane(s) with their own worktrees')
    .argument('<agents>', 'Agent type(s) - single or comma-separated (e.g., claude or claude,gemini,codex)')
    .argument('[task]', 'Optional task description to include in agent name(s)')
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

          // Check if spawning multiple agents (comma-separated)
          const isMultiAgent = agent.includes(',')

          let spawnScript: string
          if (isMultiAgent && existsSync(join(scriptsDir, 'spawn_multi_agents.py'))) {
            // Use multi-agent spawning script
            spawnScript = join(scriptsDir, 'spawn_multi_agents.py')
          } else if (!isMultiAgent && existsSync(join(scriptsDir, 'spawn_agent_smart.py'))) {
            // Use smart spawning with intelligent pane management
            spawnScript = join(scriptsDir, 'spawn_agent_smart.py')
          } else {
            // Fallback to basic spawn
            spawnScript = join(scriptsDir, 'spawn_agent.py')
          }

          // Get current working directory (project directory)
          const projectDir = process.cwd()

          // Build spawn command arguments
          let args: string[]

          if (isMultiAgent) {
            // Multi-agent spawning
            console.log(chalk.blue('🤖 Spawning multiple agents...'))

            // Parse agent types (comma-separated)
            const agentTypes = agent
              .split(',')
              .map((a) => a.trim())
              .filter(Boolean)
            console.log(chalk.dim(`   Agents: ${agentTypes.join(', ')}`))

            // For multi-agent, pass agents and task directly
            args = [spawnScript, agent]

            if (task) {
              args.push(task)
            }

            args.push('--project-dir', projectDir)

            if (options.args) {
              args.push('--args', options.args)
            }
          } else {
            // Single agent spawning
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

            // Check if worktree already exists
            const worktreePath = join(projectDir, '.crewchief', 'worktrees', agentName)
            if (existsSync(worktreePath)) {
              // Append timestamp if worktree exists
              const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19)
              agentName = `${baseName}__${agent}_${timestamp}`
              console.log(chalk.yellow(`⚠️  Worktree ${baseName}__${agent} exists, using ${agentName}`))
            }

            args = [spawnScript, agent, '--name', agentName, '--project-dir', projectDir]

            if (options.vertical) {
              args.push('--vertical')
            }

            if (options.args) {
              args.push('--args', options.args)
            }
          }

          if (options.noLabel) {
            args.push('--no-label')
          }

          console.log(chalk.cyan('🚀 Spawning agent via iTerm2...'))
          console.log(chalk.dim(`   Script: ${spawnScript}`))
          console.log(chalk.dim(`   Agent: ${agent}`))
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

          if (isMultiAgent) {
            console.log(chalk.green('✅ Agents spawned successfully'))
            console.log(chalk.dim("   Use 'crewchief agent list' to see all agents"))
            console.log(chalk.dim("   Use 'crewchief agent message <agent-name> <text>' to send commands"))
          } else {
            console.log(chalk.green('✅ Agent spawned successfully'))
            console.log(chalk.dim("   Use 'crewchief agent list' to see all agents"))
            // Only show specific agent name for single spawn
            const agentName = args.find((a, i) => args[i - 1] === '--name')
            if (agentName) {
              console.log(chalk.dim(`   Use 'crewchief agent message ${agentName} <text>' to send commands`))
            }
          }
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
  Single agent:
  $ crewchief spawn claude                    # Spawn Claude with auto-generated name
  $ crewchief spawn claude "auth-feature"     # Include task in name
  $ crewchief spawn claude --name my-agent    # Use custom name
  
  Multiple agents:
  $ crewchief spawn claude,gemini implement-auth     # Spawn both Claude and Gemini
  $ crewchief spawn claude,gemini,codex code-review  # Spawn three agents
  $ crewchief spawn "claude, gemini" fix-bug         # With spaces (quoted)
  
  With options:
  $ crewchief spawn claude --vertical         # Split vertically (single agent only)
  $ crewchief spawn claude,gemini --args "--model gpt-4"  # Pass args to all agents

Supported agents:
  - claude    Anthropic's Claude (uses 'claude' command)
  - gemini    Google's Gemini (uses 'gemini' command)
  - codex     OpenAI Codex (uses 'codex' command)
  - cursor    Cursor AI (uses 'cursor-agent' command)
  - aider     Aider coding assistant (uses 'aider' command)
  - custom    Any custom command

Spawning behavior:
  Single agent:
    - Creates one new pane with intelligent splitting
    - One git worktree: .crewchief/worktrees/<task>__<agent>
  
  Multiple agents:
    - Creates multiple panes with hierarchical layout
    - Separate worktrees for each: <task>__<agent1>, <task>__<agent2>, etc.
    - First agent: vertical split (left/right)
    - Additional agents: horizontal splits of right pane`,
    )
}
