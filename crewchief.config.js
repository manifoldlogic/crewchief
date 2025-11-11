/**
 * Example CrewChief configuration
 * 
 * IMPORTANT: iTerm2 is required for agent orchestration.
 * The tmux implementation is incomplete and no longer under development.
 * 
 * @type {import('./packages/cli/src/config/schema').CrewChiefConfig}
 */
const config = {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },

  // orchestrator and agents sections removed (unused)

  // Terminal configuration (iTerm2)
  terminal: {
    backend: 'iterm',
    iterm: {
      sessionName: 'crewchief',
    },
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