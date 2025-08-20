export default {
  repository: {
    mainBranch: "main",
    worktreeBasePath: '.crewchief/worktrees'
  },
  orchestrator: {
    model: 'claude-opus-4-1',
    maxConcurrentAgents: 5,
    defaultTimeout: 30 * 60 * 1000
  },
  launch: {
    autoRunDefaultAgents: false,
    askToUpdateLlmGuides: false
  },
  agents: {
    claude: {
      command: 'claude',
      defaultArgs: ['--model', 'claude-3-opus'],
      agentsDir: '.claude/agents/',
      commandsDir: '.claude/commands/'
    },
    gemini: {
      command: 'gemini',
      defaultArgs: ['--model', 'gemini-pro'],
      agentsDir: '.gemini/agents/'
    }
  },
  defaults: {
    rootAgents: []
  },
  tmux: {
    sessionName: 'crewchief',
    orchestratorPaneSize: 40,
    agentPaneArrangement: "tiled"
  },
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: true
  },
  worktree: {
    copyIgnoredFiles: [
      '**/.claude/**/*',
      '**/.cursor/**/*',
      '**/.gemini/**/*',
      '**/.codex/**/*',
      '**/.cursorrules',
      'crewchief.config.ts',
      '**/.env',
      '**/.mcp.json'
    ],
    copyFromPath: '.',
    overwriteStrategy: 'skip'
  }
};
