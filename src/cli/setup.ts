import { Command } from 'commander';
import inquirer from 'inquirer';
import fs from 'node:fs';
import path from 'node:path';
import { logger } from '../utils/logger';

export function registerSetupCommand(program: Command): void {
  program
    .command('setup')
    .description('Interactive configuration wizard for CrewChief')
    .action(async () => {
      try {
        const answers = await inquirer.prompt([
          {
            name: 'repoType',
            type: 'list',
            message: 'Repository type',
            choices: ['standard', 'monorepo'],
            default: 'standard'
          },
          {
            name: 'mainBranch',
            type: 'input',
            message: 'Main branch name',
            default: 'main'
          },
          {
            name: 'maxConcurrentAgents',
            type: 'number',
            message: 'Maximum concurrent agents',
            default: 5
          },
          {
            name: 'defaultPlatform',
            type: 'list',
            message: 'Default agent platform',
            choices: ['claude', 'gemini', 'both'],
            default: 'both'
          },
          {
            name: 'enableCompetitionDefault',
            type: 'confirm',
            message: 'Enable competition mode by default?',
            default: true
          },
          {
            name: 'orchestratorPaneSize',
            type: 'number',
            message: 'Orchestrator pane size (%)',
            default: 40
          },
          {
            name: 'agentPaneArrangement',
            type: 'list',
            message: 'Agent pane arrangement',
            choices: ['tiled', 'vertical', 'horizontal'],
            default: 'tiled'
          }
        ]);

        const configPath = path.join(process.cwd(), 'crewchief.config.ts');
        const content = `export default {
  repository: {
    mainBranch: ${JSON.stringify(answers.mainBranch)},
    worktreeBasePath: '.crewchief/worktrees'
  },
  orchestrator: {
    model: 'claude-opus-4-1',
    maxConcurrentAgents: ${Number(answers.maxConcurrentAgents)},
    defaultTimeout: 30 * 60 * 1000
  },
  agents: {
    claude: {
      command: 'claude-cli',
      defaultArgs: ['--model', 'claude-3-opus'],
      agentsDir: '.claude/agents/',
      commandsDir: '.claude/commands/'
    },
    gemini: {
      command: 'gemini-cli',
      defaultArgs: ['--model', 'gemini-pro'],
      agentsDir: '.gemini/agents/'
    }
  },
  tmux: {
    sessionName: 'crewchief',
    orchestratorPaneSize: ${Number(answers.orchestratorPaneSize)},
    agentPaneArrangement: ${JSON.stringify(answers.agentPaneArrangement)}
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false
  }
};
`;
        fs.writeFileSync(configPath, content);
        logger.success(`Configuration saved to ${configPath}`);
      } catch (err) {
        logger.error('Setup failed:', err);
        process.exitCode = 1;
      }
    });
}


