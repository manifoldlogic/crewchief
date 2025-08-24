import fs from 'node:fs'
import path from 'node:path'
import { Command } from 'commander'
import inquirer from 'inquirer'
import { logger } from '../utils/logger'

export async function runSetupWizard(): Promise<string> {
  const answers = await inquirer.prompt([
    {
      name: 'repoType',
      type: 'list',
      message: 'Repository type',
      choices: ['standard', 'monorepo'],
      default: 'standard',
    },
    {
      name: 'mainBranch',
      type: 'input',
      message: 'Main branch name',
      default: 'main',
    },
    {
      name: 'defaultPlatform',
      type: 'list',
      message: 'Default agent platform',
      choices: ['claude', 'gemini', 'both'],
      default: 'both',
    },
    {
      name: 'enableCompetitionDefault',
      type: 'confirm',
      message: 'Enable competition mode by default?',
      default: true,
    },
    {
      name: 'rootAgentsCsv',
      type: 'input',
      message: 'Default root agent id(s) to auto-launch on start (comma-separated, leave blank to choose on launch)',
      default: '',
    },
    {
      name: 'askToUpdateLlmGuides',
      type: 'confirm',
      message: 'Update LLM guide files (e.g., CLAUDE.md) with instructions on using crewchief to spawn agents?',
      default: true,
    },
  ])

  const rootAgents = String(answers.rootAgentsCsv || '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean)
    .map((id) => ({ id }))

  const configPath = path.join(process.cwd(), 'crewchief.config.js')
  const content = `export default {
  repository: {
    mainBranch: ${JSON.stringify(answers.mainBranch)},
    worktreeBasePath: '.crewchief/worktrees'
  },
  // orchestrator section removed (unused)
  launch: {
    autoRunDefaultAgents: ${rootAgents.length > 0 ? 'true' : 'false'},
    askToUpdateLlmGuides: ${answers.askToUpdateLlmGuides ? 'true' : 'false'}
  },
  // agents section removed (unused)
  defaults: {
    rootAgents: ${JSON.stringify(rootAgents)}
  },
  terminal: {
    backend: 'iterm',
    iterm: {
      sessionName: 'crewchief'
    }
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false
  }
};
`
  fs.writeFileSync(configPath, content)

  if (answers.askToUpdateLlmGuides) {
    const claudeGuide = path.join(process.cwd(), 'CLAUDE.md')
    const snippet =
      '\n\n> Tip: Use `crewchief` to spawn agents and coordinate work. Example: `crewchief agent spawn project-manager "Plan homepage redesign"`'
    try {
      if (fs.existsSync(claudeGuide)) {
        fs.appendFileSync(claudeGuide, snippet)
      } else {
        fs.writeFileSync(claudeGuide, `# Claude Guide${snippet}\n`)
      }
    } catch {
      // best-effort, ignore
    }
  }

  return configPath
}

export function registerSetupCommand(program: Command): void {
  program
    .command('setup')
    .description('Interactive configuration wizard for CrewChief')
    .action(async () => {
      try {
        const configPath = await runSetupWizard()
        logger.success(`Configuration saved to ${configPath}`)
      } catch (err) {
        logger.error('Setup failed:', err)
        process.exitCode = 1
      }
    })
}
