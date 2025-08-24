/**
 * CrewChief Configuration
 *
 * You can create either:
 * - crewchief.config.js (committed to repo, shared with team)
 * - crewchief.config.local.js (gitignored, for local overrides)
 *
 * If both exist, the local version takes priority.
 */

export default {
  repository: {
    mainBranch: 'main',
    worktreeBasePath: '.crewchief/worktrees',
  },

  // Launch behavior for initial CLI UX
  launch: {
    autoRunDefaultAgents: false,
    askToUpdateLlmGuides: true,
  },

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
      { type: 'tests', command: 'pnpm test' },
      { type: 'linting', command: 'pnpm lint' },
      { type: 'build', command: 'pnpm build' },
    ],
  },

  worktree: {
    // Files to copy to new worktrees (useful for .env files, etc.)
    copyIgnoredFiles: [
      '!**/.crewchief/worktrees/**/*',
      '**/.claude/**/*',
      '**/.cursor/**/*',
      '**/.gemini/**/*',
      '**/.codex/**/*',
      '**/.cursorrules',
      'crewchief.config.js',
      'crewchief.config.local.js',
      '**/.env',
      '**/.env.local',
      '**/.mcp.json',
    ],
    copyFromPath: '.',
    overwriteStrategy: 'skip',
  },
}
