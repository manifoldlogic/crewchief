export default {
  repository: {
    mainBranch: "main",
    // worktreeBasePath uses default: '~/.crewchief/worktrees/<repo-name>'
  },
  worktree: {
    copyIgnoredFiles: [".env", ".env.local"],
    copyFromPath: '.',
    overwriteStrategy: 'skip'
  },
  launch: {
    askToUpdateLlmGuides: false
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
