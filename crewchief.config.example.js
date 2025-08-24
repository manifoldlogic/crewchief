/**
 * Example CrewChief configuration
 * 
 * IMPORTANT: iTerm2 is required for agent orchestration.
 * The tmux implementation is incomplete and no longer under development.
 * 
 * @type {import('./packages/cli/src/config/schema.js').CrewChiefConfig}
 */
const config = {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },

  orchestrator: {
    model: 'claude-opus-4-1',
    maxConcurrentAgents: 5,
    defaultTimeout: 30 * 60 * 1000, // 30 minutes
  },

  agents: {
    claude: {
      command: 'claude',
      defaultArgs: ['--model', 'claude-3-opus'],
      agentsDir: '.claude/agents/',
      commandsDir: '.claude/commands/',
    },
    gemini: {
      command: 'gemini',
      defaultArgs: ['--model', 'gemini-pro'],
      agentsDir: '.gemini/agents/',
    },
  },

  // Terminal configuration
  terminal: {
    // Backend: 'iterm' is required, 'tmux' is deprecated
    backend: 'iterm',

    // iTerm2 settings (REQUIRED for agent orchestration)
    iterm: {
      sessionName: 'crewchief',
      bridgePort: 8765,
      gridLayout: {
        rows: 2,
        cols: 2,
      },
      agentBadges: true,
      // Optional: specify an iTerm2 profile for agent sessions
      // profile: 'CrewChief Agent',
    },

    // DEPRECATED: tmux settings are no longer supported
    // The tmux implementation is incomplete and should not be used
    // tmux: {
    //   sessionName: 'crewchief',
    //   orchestratorPaneSize: 40,
    //   agentPaneArrangement: 'tiled',
    // },
  },

  evaluation: {
    autoMergeThreshold: 0.95,
    requireTestsPass: true,
    requireReview: false,
    qualityChecks: [
      {
        type: 'tests',
        command: 'pnpm test',
      },
      {
        type: 'linting',
        command: 'pnpm lint',
      },
      {
        type: 'build',
        command: 'pnpm build',
      },
    ],
  },

  launch: {
    autoRunDefaultAgents: false,
    askToUpdateLlmGuides: true,
    autoStartOpsdeck: false,
  },

  defaults: {
    rootAgents: [
      { id: 'planner', platform: 'claude' },
      { id: 'coder', platform: 'claude' },
      { id: 'reviewer', platform: 'gemini' },
    ],
  },

  worktree: {
    copyIgnoredFiles: ['.env', '.env.local', 'config.local.js'],
    copyFromPath: '.',
    overwriteStrategy: 'skip',
  },
}

export default config