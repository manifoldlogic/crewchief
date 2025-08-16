/**
 * CrewChief Configuration
 * 
 * This is a plain JavaScript configuration file that works without any TypeScript setup.
 * 
 * You can create either:
 * - crewchief.config.js (committed to repo, shared with team)
 * - crewchief.config.local.js (gitignored, for local overrides)
 * 
 * If both exist, the local version takes priority.
 */

module.exports = {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },
  
  orchestrator: {
    model: 'claude-opus-4-1',
    maxConcurrentAgents: 5,
    defaultTimeout: 30 * 60 * 1000, // 30 minutes
  },
  
  launch: {
    autoRunDefaultAgents: false,
    askToUpdateLlmGuides: false,
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
    agentPaneArrangement: 'tiled', // or 'even-horizontal', 'even-vertical'
  },
  
  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: true
  },
  
  worktree: {
    // Files to copy to new worktrees (useful for .env files, etc.)
    copyIgnoredFiles: [
      '**/.claude/**/*',
      '**/.cursor/**/*',
      '**/.gemini/**/*',
      '**/.codex/**/*',
      '**/.cursorrules',
      'crewchief.config.js',
      'crewchief.config.local.js', // Also copy local config if it exists
      '**/.env',
      '**/.env.local',
      '**/.mcp.json'
    ],
    copyFromPath: '.',
    overwriteStrategy: 'skip' // 'skip', 'overwrite', or 'backup'
  }
};