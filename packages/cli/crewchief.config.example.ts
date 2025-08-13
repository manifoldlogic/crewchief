export default {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },
  orchestrator: {
    model: 'claude-opus-4-1',
    maxConcurrentAgents: 5,
    defaultTimeout: 30 * 60 * 1000,
  },
  agents: {
    claude: {
      command: 'claude-cli',
      defaultArgs: ['--model', 'claude-3-opus'],
      agentsDir: '.claude/agents/',
      commandsDir: '.claude/commands/',
    },
    gemini: {
      command: 'gemini-cli',
      defaultArgs: ['--model', 'gemini-pro'],
      agentsDir: '.gemini/agents/',
    },
  },
  tmux: {
    sessionName: 'crewchief',
    orchestratorPaneSize: 40,
    agentPaneArrangement: 'tiled',
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false,
  },
}
