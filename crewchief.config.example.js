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

    // Worktree base path - controls where git worktrees are created
    //
    // Default (v2.0+): Worktrees outside repo in user-specific location
    // worktreeBasePath: '~/.crewchief/worktrees/<repo-name>',
    //
    // Legacy (v1.x): Worktrees inside repo (opt-in to old behavior)
    // worktreeBasePath: '.crewchief/worktrees',
    //
    // Custom: Absolute path with repository isolation
    // worktreeBasePath: '/mnt/ssd/worktrees/<repo-name>',
    //
    // Features:
    // - Tilde expansion: ~/path → /home/user/path
    // - Repo placeholder: <repo-name> → actual repository name
    // - Absolute paths: /custom/path
    // - Relative paths: .crewchief/worktrees
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